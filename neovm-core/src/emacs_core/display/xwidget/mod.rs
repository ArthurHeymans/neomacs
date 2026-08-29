//! GNU-shaped xwidget model and view runtime.
//!
//! GNU Emacs stores xwidgets as `PVEC_XWIDGET` pseudovectors and xwidget views
//! as `PVEC_XWIDGET_VIEW` pseudovectors.  This module owns the evaluator-side
//! lists and builtins for that object model; native frontend/embedder state is
//! intentionally kept out of the Lisp heap objects.

use super::builtins::{
    builtin_get_buffer, builtin_get_buffer_create, collect_proper_list_items, expect_wholenump,
};
use super::error::{EvalResult, Flow, signal};
use super::eval::Context;
use super::subr::SubrSpec;
use super::symbol::Obarray;
use super::value::{Value, eq_value};
use crate::emacs_core::error::LispCondition;
use crate::emacs_core::error::expect_args_range;
use crate::heap_types::LispString;
use std::collections::HashMap;
use strum::IntoStaticStr;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::many("make-xwidget", create, 4, Some(7)),
    SubrSpec::many("xwidgetp", is_xwidget, 1, Some(1)),
    SubrSpec::many("xwidget-view-p", is_view, 1, Some(1)),
    SubrSpec::many("xwidget-live-p", is_live, 1, Some(1)),
    SubrSpec::many("xwidget-info", info, 1, Some(1)),
    SubrSpec::many("xwidget-view-info", view_info, 1, Some(1)),
    SubrSpec::many("xwidget-view-model", view_model, 1, Some(1)),
    SubrSpec::many("xwidget-view-window", view_window, 1, Some(1)),
    SubrSpec::many("xwidget-view-lookup", lookup_view, 2, Some(2)),
    SubrSpec::many("delete-xwidget-view", delete_view, 1, Some(1)),
    SubrSpec::many("xwidget-plist", plist, 1, Some(1)),
    SubrSpec::many("set-xwidget-plist", set_plist, 2, Some(2)),
    SubrSpec::many("xwidget-buffer", buffer, 1, Some(1)),
    SubrSpec::many("set-xwidget-buffer", set_buffer, 2, Some(2)),
    SubrSpec::many("xwidget-query-on-exit-flag", query_on_exit, 1, Some(1)),
    SubrSpec::many(
        "set-xwidget-query-on-exit-flag",
        set_query_on_exit,
        2,
        Some(2),
    ),
    SubrSpec::many("get-buffer-xwidgets", buffer_xwidgets, 1, Some(1)),
    SubrSpec::many("kill-xwidget", kill, 1, Some(1)),
    SubrSpec::many("xwidget-resize", resize, 3, Some(3)),
    SubrSpec::many("xwidget-size-request", size_request, 1, Some(1)),
    SubrSpec::many("xwidget-webkit-uri", webkit_uri, 1, Some(1)),
    SubrSpec::many("xwidget-webkit-title", webkit_title, 1, Some(1)),
    SubrSpec::many("xwidget-webkit-goto-uri", navigate_webkit, 2, Some(2)),
    SubrSpec::many(
        "xwidget-webkit-execute-script",
        execute_script,
        2,
        Some(3),
    ),
    SubrSpec::many(
        "xwidget-webkit-estimated-load-progress",
        estimated_load_progress,
        1,
        Some(1),
    ),
];

pub(crate) fn register_subrs(ctx: &mut Context) {
    ctx.register_subrs(SUBRS);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum XwidgetType {
    Webkit,
}

impl XwidgetType {
    fn is_lisp_value(self, value: Value) -> bool {
        value == self.value()
    }

    fn value(self) -> Value {
        Value::symbol(self.name())
    }

    fn name(self) -> &'static str {
        self.into()
    }
}

#[derive(Clone, Debug, Default)]
struct WebKitRuntimeState {
    uri: String,
    title: String,
    /// Estimated load progress, 0.0..=1.0, as `xwidget-webkit-estimated-load-
    /// progress' reports it.
    ///
    /// GNU reads WebKitGTK's continuous `estimated-load-progress' property.
    /// This build cannot: the web view lives on the render thread and there
    /// is no progress signal coming back (`InputEvent::WebKitLoadFinished'
    /// exists but nothing sends or consumes it yet). So this is not measured
    /// -- it is 0.0 before any navigation and 1.0 once one has been
    /// dispatched. A stuck 0.0 would render as a permanent "[0%]" in
    /// `xwidget-webkit-mode''s header line, which is a worse lie than
    /// reporting the navigation as issued.
    load_progress: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct XwidgetState {
    internal_xwidget_list: Value,
    internal_xwidget_view_list: Value,
    webkit_state: HashMap<u32, WebKitRuntimeState>,
    counter: u32,
}

impl XwidgetState {
    pub(crate) fn new() -> Self {
        Self {
            internal_xwidget_list: Value::NIL,
            internal_xwidget_view_list: Value::NIL,
            webkit_state: HashMap::new(),
            counter: 0,
        }
    }

    pub(crate) fn trace_roots_with(&self, visit: &mut dyn FnMut(Value)) {
        visit(self.internal_xwidget_list);
        visit(self.internal_xwidget_view_list);
    }

    fn next_id(&mut self) -> u32 {
        self.counter = self.counter.wrapping_add(1);
        self.counter
    }

    fn ensure_webkit_state(&mut self, id: u32) {
        self.webkit_state.entry(id).or_default();
    }

    fn remove_webkit_state(&mut self, id: u32) {
        self.webkit_state.remove(&id);
    }

    fn webkit_uri(&self, id: u32) -> String {
        self.webkit_state
            .get(&id)
            .map(|state| state.uri.clone())
            .unwrap_or_default()
    }

    fn set_webkit_uri(&mut self, id: u32, uri: String) {
        self.webkit_state.entry(id).or_default().uri = uri;
    }

    fn webkit_load_progress(&self, id: u32) -> f64 {
        self.webkit_state
            .get(&id)
            .map(|state| state.load_progress)
            .unwrap_or(0.0)
    }

    fn set_webkit_load_progress(&mut self, id: u32, progress: f64) {
        self.webkit_state.entry(id).or_default().load_progress = progress;
    }

    fn webkit_title(&self, id: u32) -> String {
        self.webkit_state
            .get(&id)
            .map(|state| state.title.clone())
            .unwrap_or_default()
    }

    fn publish(&self, obarray: &mut Obarray) {
        obarray.set_symbol_value(
            "xwidget-list",
            shallow_copy_list(self.internal_xwidget_list),
        );
        obarray.set_symbol_value(
            "xwidget-view-list",
            shallow_copy_list(self.internal_xwidget_view_list),
        );
    }
}

pub(crate) fn init_xwidget_variables(obarray: &mut Obarray) {
    obarray.set_symbol_value("xwidget-list", Value::NIL);
    obarray.make_special("xwidget-list");
    obarray.set_symbol_value("xwidget-view-list", Value::NIL);
    obarray.make_special("xwidget-view-list");
    obarray.set_symbol_value("xwidget-webkit-disable-javascript", Value::NIL);
    obarray.make_special("xwidget-webkit-disable-javascript");
}

fn shallow_copy_list(list: Value) -> Value {
    if list.is_nil() {
        return Value::NIL;
    }
    let items = collect_proper_list_items(list).expect("xwidget internal list must be proper");
    Value::list(items)
}

fn delq_from_list(list: Value, target: Value) -> Value {
    let items = collect_proper_list_items(list).expect("xwidget internal list must be proper");
    Value::list(
        items
            .into_iter()
            .filter(|item| !eq_value(item, &target))
            .collect(),
    )
}

fn expect_xwidget(value: Value) -> Result<Value, Flow> {
    if value.is_xwidget() {
        Ok(value)
    } else {
        Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("xwidgetp"), value],
        ))
    }
}

fn expect_live_xwidget(value: Value) -> Result<Value, Flow> {
    if value
        .as_xwidget()
        .is_some_and(|xwidget| !xwidget.buffer.is_nil())
    {
        Ok(value)
    } else {
        Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("xwidget-live-p"), value],
        ))
    }
}

fn expect_live_webkit_xwidget(value: Value) -> Result<Value, Flow> {
    let value = expect_live_xwidget(value)?;
    let xwidget = value.as_xwidget().unwrap();
    if XwidgetType::Webkit.is_lisp_value(xwidget.type_) {
        Ok(value)
    } else {
        Err(signal("error", vec![Value::string("Not a WebKit widget")]))
    }
}

fn expect_xwidget_view(value: Value) -> Result<Value, Flow> {
    if value.is_xwidget_view() {
        Ok(value)
    } else {
        Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("xwidget-view-p"), value],
        ))
    }
}

fn expect_buffer(value: Value) -> Result<Value, Flow> {
    if value.is_buffer() {
        Ok(value)
    } else {
        Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("bufferp"), value],
        ))
    }
}

fn expect_symbol(value: Value) -> Result<Value, Flow> {
    if value.is_symbol() {
        Ok(value)
    } else {
        Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("symbolp"), value],
        ))
    }
}

fn expect_string(value: Value) -> Result<LispString, Flow> {
    value.as_lisp_string().cloned().ok_or_else(|| {
        signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), value],
        )
    })
}

fn expect_i32_wholenump(value: Value) -> Result<i32, Flow> {
    let n = expect_wholenump(&value)?;
    i32::try_from(n).map_err(|_| {
        signal(
            LispCondition::ArgsOutOfRange,
            vec![value, Value::fixnum(0), Value::fixnum(i32::MAX as i64)],
        )
    })
}

fn ensure_proper_list(value: Value) -> Result<(), Flow> {
    collect_proper_list_items(value).map(|_| ())
}

fn current_buffer_value(eval: &Context) -> EvalResult {
    let id = eval
        .buffers
        .current_buffer_id()
        .ok_or_else(|| signal("error", vec![Value::string("No current buffer")]))?;
    Ok(Value::make_buffer(id))
}

fn xwidget_live_p_value(value: Value) -> bool {
    value
        .as_xwidget()
        .is_some_and(|xwidget| !xwidget.buffer.is_nil())
}

fn create(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("make-xwidget", &args, 4, 7)?;
    let type_ = expect_symbol(args[0])?;
    if !XwidgetType::Webkit.is_lisp_value(type_) {
        return Err(signal("error", vec![Value::string("Bad xwidget type")]));
    }
    eval.require_value(Value::symbol("xwidget"), None, None)?;
    let title = args[1];
    let width = expect_i32_wholenump(args[2])?;
    let height = expect_i32_wholenump(args[3])?;
    let buffer_arg = args.get(5).copied().unwrap_or(Value::NIL);
    let buffer = if buffer_arg.is_nil() {
        current_buffer_value(eval)?
    } else {
        builtin_get_buffer_create(eval, vec![buffer_arg, Value::NIL])?
    };
    let id = eval.xwidgets.next_id();
    let xwidget = Value::make_xwidget(type_, title, buffer, width, height, id);
    if let Some(host) = eval.display_host.as_ref() {
        host.create_webkit_xwidget(id, width.max(1) as u32, height.max(1) as u32)
            .map_err(|err| signal("error", vec![Value::string(err)]))?;
    }
    eval.xwidgets.ensure_webkit_state(id);
    eval.xwidgets.internal_xwidget_list = Value::cons(xwidget, eval.xwidgets.internal_xwidget_list);
    eval.xwidgets.publish(&mut eval.obarray);
    Ok(xwidget)
}

fn is_xwidget(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidgetp", &args, 1, 1)?;
    Ok(Value::bool_val(args[0].is_xwidget()))
}

fn is_view(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-view-p", &args, 1, 1)?;
    Ok(Value::bool_val(args[0].is_xwidget_view()))
}

fn is_live(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-live-p", &args, 1, 1)?;
    Ok(Value::bool_val(xwidget_live_p_value(args[0])))
}

fn info(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-info", &args, 1, 1)?;
    let value = expect_live_xwidget(args[0])?;
    let xwidget = value.as_xwidget().unwrap();
    Ok(Value::vector(vec![
        xwidget.type_,
        xwidget.title,
        Value::fixnum(xwidget.width as i64),
        Value::fixnum(xwidget.height as i64),
    ]))
}

fn view_info(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-view-info", &args, 1, 1)?;
    let value = expect_xwidget_view(args[0])?;
    let view = value.as_xwidget_view().unwrap();
    Ok(Value::vector(vec![
        Value::fixnum(view.x as i64),
        Value::fixnum(view.y as i64),
        Value::fixnum(view.clip_right as i64),
        Value::fixnum(view.clip_bottom as i64),
        Value::fixnum(view.clip_top as i64),
        Value::fixnum(view.clip_left as i64),
    ]))
}

fn view_model(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-view-model", &args, 1, 1)?;
    let value = expect_xwidget_view(args[0])?;
    Ok(value.as_xwidget_view().unwrap().model)
}

fn view_window(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-view-window", &args, 1, 1)?;
    let value = expect_xwidget_view(args[0])?;
    Ok(value.as_xwidget_view().unwrap().window)
}

fn lookup_view(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-view-lookup", &args, 2, 2)?;
    let model = expect_live_xwidget(args[0])?;
    let window = args[1];
    let items = collect_proper_list_items(eval.xwidgets.internal_xwidget_view_list)?;
    for view_value in items {
        let Some(view) = view_value.as_xwidget_view() else {
            continue;
        };
        if eq_value(&view.model, &model) && eq_value(&view.window, &window) {
            return Ok(view_value);
        }
    }
    Ok(Value::NIL)
}

fn delete_view(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("delete-xwidget-view", &args, 1, 1)?;
    let value = expect_xwidget_view(args[0])?;
    eval.xwidgets.internal_xwidget_view_list =
        delq_from_list(eval.xwidgets.internal_xwidget_view_list, value);
    eval.xwidgets.publish(&mut eval.obarray);
    Ok(Value::NIL)
}

fn plist(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-plist", &args, 1, 1)?;
    let value = expect_live_xwidget(args[0])?;
    Ok(value.as_xwidget().unwrap().plist)
}

fn set_plist(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("set-xwidget-plist", &args, 2, 2)?;
    let value = expect_live_xwidget(args[0])?;
    let plist = args[1];
    ensure_proper_list(plist)?;
    value.with_xwidget_mut(|xwidget| {
        xwidget.plist = plist;
    });
    Ok(plist)
}

fn buffer(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-buffer", &args, 1, 1)?;
    let value = expect_xwidget(args[0])?;
    Ok(value.as_xwidget().unwrap().buffer)
}

fn set_buffer(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("set-xwidget-buffer", &args, 2, 2)?;
    let value = expect_live_xwidget(args[0])?;
    let buffer = expect_buffer(args[1])?;
    value.with_xwidget_mut(|xwidget| {
        xwidget.buffer = buffer;
    });
    Ok(Value::NIL)
}

fn query_on_exit(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-query-on-exit-flag", &args, 1, 1)?;
    let value = expect_live_xwidget(args[0])?;
    Ok(Value::bool_val(
        !value.as_xwidget().unwrap().kill_without_query,
    ))
}

fn set_query_on_exit(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("set-xwidget-query-on-exit-flag", &args, 2, 2)?;
    let value = expect_live_xwidget(args[0])?;
    let flag = args[1];
    value.with_xwidget_mut(|xwidget| {
        xwidget.kill_without_query = flag.is_nil();
    });
    Ok(flag)
}

fn buffer_xwidgets(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("get-buffer-xwidgets", &args, 1, 1)?;
    if args[0].is_nil() {
        return Ok(Value::NIL);
    }
    let buffer = builtin_get_buffer(eval, vec![args[0]])?;
    if buffer.is_nil() {
        return Ok(Value::NIL);
    }
    let items = collect_proper_list_items(eval.xwidgets.internal_xwidget_list)?;
    let mut result = Value::NIL;
    for value in items {
        let Some(xwidget) = value.as_xwidget() else {
            continue;
        };
        if eq_value(&xwidget.buffer, &buffer) {
            result = Value::cons(value, result);
        }
    }
    Ok(result)
}

fn kill(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("kill-xwidget", &args, 1, 1)?;
    let value = expect_live_xwidget(args[0])?;
    let id = value.as_xwidget().unwrap().xwidget_id;
    eval.xwidgets.internal_xwidget_list =
        delq_from_list(eval.xwidgets.internal_xwidget_list, value);
    eval.xwidgets.publish(&mut eval.obarray);
    if let Some(host) = eval.display_host.as_ref() {
        host.destroy_webkit_xwidget(id)
            .map_err(|err| signal("error", vec![Value::string(err)]))?;
    }
    eval.xwidgets.remove_webkit_state(id);
    value.with_xwidget_mut(|xwidget| {
        xwidget.buffer = Value::NIL;
    });
    Ok(Value::NIL)
}

fn resize(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-resize", &args, 3, 3)?;
    let value = expect_live_xwidget(args[0])?;
    let width = expect_i32_wholenump(args[1])?;
    let height = expect_i32_wholenump(args[2])?;
    let id = value.as_xwidget().unwrap().xwidget_id;
    value.with_xwidget_mut(|xwidget| {
        xwidget.width = width;
        xwidget.height = height;
    });
    if let Some(host) = eval.display_host.as_ref() {
        host.resize_webkit_xwidget(id, width.max(1) as u32, height.max(1) as u32)
            .map_err(|err| signal("error", vec![Value::string(err)]))?;
    }
    Ok(Value::NIL)
}

fn webkit_uri(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-webkit-uri", &args, 1, 1)?;
    let value = expect_live_webkit_xwidget(args[0])?;
    let id = value.as_xwidget().unwrap().xwidget_id;
    Ok(Value::string(_eval.xwidgets.webkit_uri(id)))
}

fn webkit_title(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-webkit-title", &args, 1, 1)?;
    let value = expect_live_webkit_xwidget(args[0])?;
    let id = value.as_xwidget().unwrap().xwidget_id;
    Ok(Value::string(_eval.xwidgets.webkit_title(id)))
}

fn navigate_webkit(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-webkit-goto-uri", &args, 2, 2)?;
    let value = expect_live_webkit_xwidget(args[0])?;
    let uri = expect_string(args[1])?;
    let id = value.as_xwidget().unwrap().xwidget_id;
    if let Some(host) = eval.display_host.as_ref() {
        host.load_webkit_xwidget_uri(id, uri.clone())
            .map_err(|err| signal("error", vec![Value::string(err)]))?;
    }
    eval.xwidgets
        .set_webkit_uri(id, String::from_utf8_lossy(uri.as_bytes()).into_owned());
    // Not measured -- see `WebKitRuntimeState::load_progress'. Dispatching
    // the navigation is the only event this build observes.
    eval.xwidgets.set_webkit_load_progress(id, 1.0);
    Ok(Value::NIL)
}

fn execute_script(eval: &mut Context, args: Vec<Value>) -> EvalResult {
    // GNU takes (XWIDGET SCRIPT &optional FUN) and feeds the script's return
    // value to FUN.  Delivering a result needs a channel from the render
    // thread back to the Lisp thread, which does not exist yet, so FUN is
    // accepted and reported as unsupported rather than silently ignored --
    // a caller that passes it would otherwise wait forever for a call that
    // never comes.
    expect_args_range("xwidget-webkit-execute-script", &args, 2, 3)?;
    let value = expect_live_webkit_xwidget(args[0])?;
    let script = expect_string(args[1])?;
    let id = value.as_xwidget().unwrap().xwidget_id;
    if args.get(2).is_some_and(|fun| !fun.is_nil()) {
        return Err(signal(
            "error",
            vec![Value::string(
                "xwidget-webkit-execute-script: the FUN callback is not supported \
                 in this build; the script still runs when FUN is omitted",
            )],
        ));
    }
    if let Some(host) = eval.display_host.as_ref() {
        host.execute_webkit_xwidget_script(id, script)
            .map_err(|err| signal("error", vec![Value::string(err)]))?;
    }
    Ok(Value::NIL)
}

fn estimated_load_progress(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-webkit-estimated-load-progress", &args, 1, 1)?;
    let value = expect_live_webkit_xwidget(args[0])?;
    let id = value.as_xwidget().unwrap().xwidget_id;
    Ok(Value::make_float(_eval.xwidgets.webkit_load_progress(id)))
}

fn size_request(_eval: &mut Context, args: Vec<Value>) -> EvalResult {
    expect_args_range("xwidget-size-request", &args, 1, 1)?;
    let value = expect_live_xwidget(args[0])?;
    let xwidget = value.as_xwidget().unwrap();
    Ok(Value::list(vec![
        Value::fixnum(xwidget.width as i64),
        Value::fixnum(xwidget.height as i64),
    ]))
}
