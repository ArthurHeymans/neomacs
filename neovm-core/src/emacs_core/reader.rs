//! Reader/printer builtins: read-from-string, read, prin1-to-string (enhanced),
//! format-spec, and various interactive-input stubs.

use super::error::{EvalResult, Flow, signal};
use super::intern::{SymId, intern, resolve_sym};
use crate::emacs_core::error::LispCondition;
// storage imports removed — now using emacs_char directly
use super::symbol::Obarray;
use super::value::*;
use crate::buffer::{EmacsBytePos, EmacsByteRange, LispCharPos1};
use std::io::Write;
use std::time::Duration;
use strum::{EnumString, IntoStaticStr};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn expect_args(name: &str, args: &[Value], n: usize) -> Result<(), Flow> {
    if args.len() != n {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_min_args(name: &str, args: &[Value], min: usize) -> Result<(), Flow> {
    if args.len() < min {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn expect_max_args(name: &str, args: &[Value], max: usize) -> Result<(), Flow> {
    if args.len() > max {
        Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol(name), Value::fixnum(args.len() as i64)],
        ))
    } else {
        Ok(())
    }
}

fn reader_initial_input_lisp_string(value: &Value) -> Option<crate::heap_types::LispString> {
    match value.kind() {
        ValueKind::String => value.as_lisp_string().cloned(),
        ValueKind::Cons => value.cons_car().as_lisp_string().cloned(),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum RequireMatchSymbol {
    #[strum(to_string = "t")]
    T,
    Confirm,
    ConfirmAfterCompletion,
}

impl RequireMatchSymbol {
    fn from_lisp_value(value: Value) -> Option<Self> {
        value.as_symbol_name().and_then(|name| name.parse().ok())
    }
}

fn empty_runtime_lisp_string(multibyte: bool) -> crate::heap_types::LispString {
    crate::heap_types::LispString::new(String::new(), multibyte)
}

fn minibuffer_result_lisp_string(
    buffers: &crate::buffer::BufferManager,
    minibuf_id: crate::buffer::BufferId,
    prompt_byte_pos: EmacsBytePos,
) -> crate::heap_types::LispString {
    if let Some(buf) = buffers.get(minibuf_id) {
        let full_end = buf.full_emacs_byte_range().end();
        if full_end > prompt_byte_pos {
            return buf.buffer_substring_lisp_string_range(EmacsByteRange::new(
                prompt_byte_pos,
                full_end,
            ));
        }
        return empty_runtime_lisp_string(buf.get_multibyte());
    }

    empty_runtime_lisp_string(true)
}

#[derive(Clone, Copy, Debug)]
struct MinibufferHistorySpec {
    variable_value: Value,
    history_name: Option<SymId>,
    position: Value,
}

fn default_minibuffer_history_spec() -> MinibufferHistorySpec {
    let default_history = intern("minibuffer-history");
    MinibufferHistorySpec {
        variable_value: Value::from_sym_id(default_history),
        history_name: Some(default_history),
        position: Value::fixnum(0),
    }
}

fn normalize_minibuffer_history_position(position: Value) -> Value {
    if position.is_nil() {
        Value::fixnum(0)
    } else {
        position
    }
}

fn minibuffer_history_spec(hist_arg: Option<&Value>) -> MinibufferHistorySpec {
    let Some(hist) = hist_arg.copied() else {
        return default_minibuffer_history_spec();
    };

    match hist.kind() {
        ValueKind::Nil => default_minibuffer_history_spec(),
        ValueKind::Symbol(_id) if hist == Value::T => MinibufferHistorySpec {
            variable_value: Value::T,
            history_name: None,
            position: Value::fixnum(0),
        },
        ValueKind::Symbol(id) => MinibufferHistorySpec {
            variable_value: Value::from_sym_id(id),
            history_name: Some(id),
            position: Value::fixnum(0),
        },
        ValueKind::Cons => {
            let history_var = hist.cons_car();
            let position = normalize_minibuffer_history_position(hist.cons_cdr());
            match history_var.kind() {
                ValueKind::Nil => MinibufferHistorySpec {
                    position,
                    ..default_minibuffer_history_spec()
                },
                ValueKind::Symbol(_id) if history_var == Value::T => MinibufferHistorySpec {
                    variable_value: Value::T,
                    history_name: None,
                    position,
                },
                ValueKind::Symbol(id) => MinibufferHistorySpec {
                    variable_value: Value::from_sym_id(id),
                    history_name: Some(id),
                    position,
                },
                _ => default_minibuffer_history_spec(),
            }
        }
        _ => default_minibuffer_history_spec(),
    }
}

fn minibuffer_history_limit(obarray: &Obarray, history_name: SymId) -> Option<usize> {
    let configured = obarray
        .get_property_id(history_name, intern("history-length"))
        .or_else(|| obarray.symbol_value("history-length").copied());

    match configured {
        Some(value) if value == Value::T => None,
        Some(value) if value.is_fixnum() => {
            let limit = value.xfixnum();
            if limit <= 0 {
                Some(0)
            } else {
                Some(limit as usize)
            }
        }
        Some(_) => None,
        None => Some(100),
    }
}

fn add_to_minibuffer_history_variable(
    obarray: &mut Obarray,
    history_name: SymId,
    value: &crate::heap_types::LispString,
) {
    if value.as_bytes().is_empty() {
        return;
    }

    let new_value = Value::heap_string(value.clone());
    let current = obarray.symbol_value_id_or_nil(history_name);
    let mut history_items = if current.is_nil() {
        Vec::new()
    } else if let Some(items) = list_to_vec(&current) {
        items
    } else {
        return;
    };

    if history_items.first().copied() == Some(new_value) {
        return;
    }

    if obarray
        .symbol_value("history-delete-duplicates")
        .is_some_and(|value| value.is_truthy())
    {
        history_items.retain(|entry| *entry != new_value);
    }

    history_items.insert(0, new_value);

    match minibuffer_history_limit(obarray, history_name) {
        Some(0) => history_items.clear(),
        Some(max) if history_items.len() > max => history_items.truncate(max),
        _ => {}
    }

    obarray.set_symbol_value_id(history_name, Value::list(history_items));
}

fn history_add_new_input_enabled(obarray: &Obarray) -> bool {
    obarray
        .symbol_value("history-add-new-input")
        .is_none_or(|value| value.is_truthy())
}

fn expect_lisp_string(value: &Value) -> Result<crate::heap_types::LispString, Flow> {
    match value.kind() {
        ValueKind::String => Ok(value
            .as_lisp_string()
            .expect("ValueKind::String must carry LispString payload")
            .clone()),
        _other => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), *value],
        )),
    }
}

fn expect_number(value: &Value) -> Result<(), Flow> {
    if value.is_number() {
        return Ok(());
    }
    Err(signal(
        LispCondition::WrongTypeArgument,
        vec![Value::symbol("numberp"), *value],
    ))
}

pub(crate) fn parse_optional_read_seconds_arg(
    value: Option<&Value>,
) -> Result<Option<Duration>, Flow> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_nil() {
        return Ok(None);
    }

    let seconds = value.as_number_f64().ok_or_else(|| {
        signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("numberp"), *value],
        )
    })?;
    if seconds <= 0.0 {
        return Ok(Some(Duration::ZERO));
    }

    Ok(Some(Duration::from_secs_f64(seconds)))
}

fn expect_initial_input_stringish(value: &Value) -> Result<(), Flow> {
    match value.kind() {
        ValueKind::Nil | ValueKind::String => Ok(()),
        ValueKind::Cons => {
            let pair_car = value.cons_car();
            let _pair_cdr = value.cons_cdr();
            if !pair_car.is_string() {
                return Err(signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("stringp"), pair_car],
                ));
            }
            Ok(())
        }
        _other => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), *value],
        )),
    }
}

fn expect_completing_read_initial_input(value: &Value) -> Result<(), Flow> {
    match value.kind() {
        ValueKind::Nil | ValueKind::String => Ok(()),
        ValueKind::Cons => {
            let pair_car = value.cons_car();
            let pair_cdr = value.cons_cdr();
            if !pair_car.is_string() {
                return Err(signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("stringp"), pair_car],
                ));
            }
            if !(pair_cdr.is_fixnum() || pair_cdr.as_char().is_some()) {
                return Err(signal(
                    LispCondition::WrongTypeArgument,
                    vec![Value::symbol("number-or-marker-p"), pair_cdr],
                ));
            }
            Ok(())
        }
        _other => Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), *value],
        )),
    }
}

#[derive(Clone, Copy, Debug)]
struct ActiveMinibufferWindowState {
    frame_id: crate::window::FrameId,
    minibuffer_window_id: crate::window::WindowId,
    calling_frame: crate::window::FrameId,
    previous_selected_window: crate::window::WindowId,
    previous_minibuffer_buffer: Option<crate::buffer::BufferId>,
    previous_minibuffer_window_start: LispCharPos1,
    previous_minibuffer_point: LispCharPos1,
    previous_minibuffer_selected_window: Option<crate::window::WindowId>,
    previous_active_minibuffer_window: Option<crate::window::WindowId>,
}

fn activate_minibuffer_window_in_state(
    frames: &mut crate::window::FrameManager,
    buffers: &mut crate::buffer::BufferManager,
    minibuffer_selected_window: &mut Option<crate::window::WindowId>,
    active_minibuffer_window: &mut Option<crate::window::WindowId>,
    minibuf_id: crate::buffer::BufferId,
) -> Option<ActiveMinibufferWindowState> {
    let frame_id = super::window_cmds::ensure_selected_frame_id_in_state(frames, buffers);
    let frame = frames.get(frame_id)?;
    let minibuffer_window_id = frame.minibuffer_window?;
    let previous_selected_window = frame.selected_window;
    let mut previous_minibuffer_buffer = None;
    let mut previous_minibuffer_window_start = LispCharPos1::ONE;
    let mut previous_minibuffer_point = LispCharPos1::ONE;
    if let Some(crate::window::Window::Leaf {
        buffer_id,
        window_start,
        point,
        ..
    }) = frame.find_window(minibuffer_window_id)
    {
        previous_minibuffer_buffer = Some(*buffer_id);
        previous_minibuffer_window_start = *window_start;
        previous_minibuffer_point = *point;
    }

    let saved = ActiveMinibufferWindowState {
        frame_id,
        minibuffer_window_id,
        calling_frame: frame_id,
        previous_selected_window,
        previous_minibuffer_buffer,
        previous_minibuffer_window_start,
        previous_minibuffer_point,
        previous_minibuffer_selected_window: *minibuffer_selected_window,
        previous_active_minibuffer_window: *active_minibuffer_window,
    };

    if let Some(frame) = frames.get_mut(frame_id) {
        if let Some(window) = frame.find_window_mut(minibuffer_window_id) {
            window.set_buffer(minibuf_id);
            crate::window::window_markers::create_window_markers(buffers, window, minibuf_id);
        }
        let _ = frame.select_window(minibuffer_window_id);
    }
    buffers.switch_current(minibuf_id);
    *minibuffer_selected_window = Some(previous_selected_window);
    *active_minibuffer_window = Some(minibuffer_window_id);
    Some(saved)
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn activate_minibuffer_window(
    eval: &mut super::eval::Context,
    minibuf_id: crate::buffer::BufferId,
) -> Option<ActiveMinibufferWindowState> {
    activate_minibuffer_window_in_state(
        &mut eval.frames,
        &mut eval.buffers,
        &mut eval.minibuffer_selected_window,
        &mut eval.active_minibuffer_window,
        minibuf_id,
    )
}

fn restore_minibuffer_window_in_state(
    frames: &mut crate::window::FrameManager,
    buffers: &mut crate::buffer::BufferManager,
    minibuffer_selected_window: &mut Option<crate::window::WindowId>,
    active_minibuffer_window: &mut Option<crate::window::WindowId>,
    saved: ActiveMinibufferWindowState,
) {
    if let Some(frame) = frames.get_mut(saved.frame_id) {
        if let Some(window) = frame.find_window_mut(saved.minibuffer_window_id)
            && let Some(prev_buffer_id) = saved.previous_minibuffer_buffer
        {
            window.set_buffer(prev_buffer_id);
            crate::window::window_markers::create_window_markers(buffers, window, prev_buffer_id);
            crate::window::window_markers::set_window_start_with_marker(
                buffers,
                window,
                saved.previous_minibuffer_window_start,
            );
            crate::window::window_markers::set_window_point_with_marker(
                buffers,
                window,
                saved.previous_minibuffer_point,
            );
        }
        let _ = frame.select_window(saved.previous_selected_window);
    }
    if frames.get(saved.calling_frame).is_some()
        && frames
            .selected_frame()
            .is_none_or(|frame| frame.id != saved.calling_frame)
    {
        let _ = frames.select_frame(saved.calling_frame);
    }
    *minibuffer_selected_window = saved.previous_minibuffer_selected_window;
    *active_minibuffer_window = saved.previous_active_minibuffer_window;
}

fn erase_expired_minibuffer_buffer_in_state(
    buffers: &mut crate::buffer::BufferManager,
    minibuf_id: crate::buffer::BufferId,
) {
    // GNU `read_minibuf_unwind` (minibuf.c:1181) erases the expired buffer's
    // text, and its companion `get_minibuffer` reuse path (minibuf.c:1062-1063)
    // drops the buffer's overlays. neomacs previously erased text only, so a
    // vertico candidate `after-string` overlay anchored on ` *Minibuf-N*`
    // survived teardown and kept the mini-window measuring as multi-line. Delete
    // the overlays here so the expired buffer is fully reset (text + overlays).
    let _ = buffers.delete_all_buffer_overlays(minibuf_id);
    let _ = buffers.replace_buffer_contents(minibuf_id, "");
}

/// Tear down one minibuffer level, mirroring GNU's two-responsibility unwind
/// (`read_minibuf_unwind` + `minibuffer_unwind`, minibuf.c) as a single unit.
///
/// This is the ONLY path through which minibuffer exit and abort flow, so the
/// two are provably identical (GNU runs the same unwind on both — both leave via
/// `(throw 'exit …)`; there is no abort-specific teardown). The steps, in GNU
/// order, are:
///
/// - **R1** Reset the expired ` *Minibuf-N*` completely — delete its overlays
///   *and* erase its text (the vertico candidate `after-string` overlay is the
///   actual carrier of the multi-line content, so text-erase alone is not
///   enough), then run `minibuffer-inactive-mode`.
/// - **R2** Restore the mini-window's buffer to ` *Minibuf-0*` (the saved
///   `previous_minibuffer_buffer`), the analogue of `minibuffer_unwind`.
/// - **R3** At the OUTERMOST level only (`minibuffers.depth() == 0` after the
///   pop, matching GNU's `minibuf_level == 0` guard at minibuf.c:1188), force
///   the mini-window back to exactly one line, content-independent.
/// - **R4** Invalidate the mini-window's cached glyph-matrix row count (folded
///   into `force_resize_mini_window_to_one_line`) so the layout engine cannot
///   reuse the stale 35-row matrix on the next redisplay.
///
/// `depth_after_pop` is `minibuffers.depth()` taken AFTER the exit/abort pop has
/// already run. `run_inactive_mode` runs `minibuffer-inactive-mode` and its
/// result is returned for the caller to `?`-propagate, exactly as before.
#[allow(clippy::too_many_arguments)]
fn teardown_minibuffer_level_in_state(
    frames: &mut crate::window::FrameManager,
    buffers: &mut crate::buffer::BufferManager,
    minibuffer_selected_window: &mut Option<crate::window::WindowId>,
    active_minibuffer_window: &mut Option<crate::window::WindowId>,
    minibuf_id: crate::buffer::BufferId,
    depth_after_pop: usize,
    saved: ActiveMinibufferWindowState,
    run_inactive_mode: impl FnOnce() -> EvalResult,
) -> EvalResult {
    let teardown_frame_id = saved.frame_id;

    // (R1) Completely reset the expired *Minibuf-N* (overlays + text), then run
    // minibuffer-inactive-mode.
    erase_expired_minibuffer_buffer_in_state(buffers, minibuf_id);
    let inactive_mode_result = run_inactive_mode();

    // (R2) Restore the mini-window's buffer to *Minibuf-0* / the prev buffer.
    restore_minibuffer_window_in_state(
        frames,
        buffers,
        minibuffer_selected_window,
        active_minibuffer_window,
        saved,
    );

    // (R3 + R4) At the outermost level, force the mini-window back to one line
    // and invalidate its cached matrix so the engine cannot reuse the stale
    // row count. Guarded by depth==0 so a nested minibuffer popping back to an
    // outer (still active) one does not collapse the outer minibuffer's window.
    if depth_after_pop == 0 {
        frames.force_resize_mini_window_to_one_line(teardown_frame_id);
    }

    inactive_mode_result
}

fn find_or_create_minibuffer_buffer_in_state(
    buffers: &mut crate::buffer::BufferManager,
    depth: usize,
) -> crate::buffer::BufferId {
    let minibuf_name = format!(" *Minibuf-{depth}*");
    let minibuf_id = match buffers.find_buffer_by_name(&minibuf_name) {
        Some(existing) => {
            // GNU `get_minibuffer` (minibuf.c:1062-1063) resets every reused
            // minibuffer pool buffer with `delete_all_overlays + reset_buffer`
            // so a new activation never inherits stale overlays or text from a
            // prior (possibly aborted) session. Mirror that defense-in-depth
            // here on the reuse branch: even if a teardown was skipped, the
            // buffer starts clean.
            let _ = buffers.delete_all_buffer_overlays(existing);
            let _ = buffers.replace_buffer_contents(existing, "");
            existing
        }
        None => buffers.create_buffer(&minibuf_name),
    };
    let _ = buffers.configure_buffer_undo_list(minibuf_id, Value::NIL);
    let _ = buffers.set_buffer_local_property(minibuf_id, "truncate-lines", Value::NIL);
    minibuf_id
}

/// Capture the directory that a newly activated minibuffer should inherit.
///
/// GNU `read_minibuf` snapshots `BVAR (current_buffer, directory)` before it
/// switches to `*Minibuf-N*`, then installs that value in the minibuffer after
/// `set_minibuffer_mode`.  The fallback scan is GNU's minibuffer-only-frame
/// defense for callers whose current buffer has no string directory.
fn minibuffer_ambient_directory_in_state(buffers: &crate::buffer::BufferManager) -> Option<Value> {
    let current = buffers.current_buffer_id();
    current
        .and_then(|id| buffers.get(id))
        .and_then(|buffer| buffer.buffer_local_value("default-directory"))
        .filter(|value| value.is_string())
        .or_else(|| {
            buffers.buffer_list().into_iter().find_map(|id| {
                buffers
                    .get(id)
                    .and_then(|buffer| buffer.buffer_local_value("default-directory"))
                    .filter(|value| value.is_string())
            })
        })
}

fn install_minibuffer_ambient_directory_in_state(
    buffers: &mut crate::buffer::BufferManager,
    minibuf_id: crate::buffer::BufferId,
    ambient_directory: Option<Value>,
) {
    if let Some(directory) = ambient_directory {
        let _ = buffers.set_buffer_local_property(minibuf_id, "default-directory", directory);
    }
}

fn run_minibuffer_mode_if_bound(eval: &mut super::eval::Context, mode: &str) -> EvalResult {
    if eval.obarray().symbol_function(mode).is_some() {
        eval.apply0(Value::symbol(mode))
    } else {
        Ok(Value::NIL)
    }
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn restore_minibuffer_window(eval: &mut super::eval::Context, saved: ActiveMinibufferWindowState) {
    restore_minibuffer_window_in_state(
        &mut eval.frames,
        &mut eval.buffers,
        &mut eval.minibuffer_selected_window,
        &mut eval.active_minibuffer_window,
        saved,
    )
}

#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
fn signal_invalid_read_syntax_in_lisp_string(
    buffer_text: &crate::heap_types::LispString,
    absolute_error_pos: usize,
    message: String,
) -> Flow {
    let clamped_pos = absolute_error_pos.min(buffer_text.sbytes());
    let prefix = &buffer_text.as_bytes()[..clamped_pos];
    let line = prefix.iter().filter(|&&byte| byte == b'\n').count() as i64 + 1;
    let line_start = prefix
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);
    let column = if buffer_text.is_multibyte() {
        crate::emacs_core::emacs_char::chars_in_multibyte(&prefix[line_start..]) as i64
    } else {
        (prefix.len() - line_start) as i64
    };
    signal(
        LispCondition::InvalidReadSyntax,
        vec![
            Value::string(message),
            Value::fixnum(line),
            Value::fixnum(column),
        ],
    )
}

fn signal_invalid_read_syntax_in_buffer_object(
    buffer: &crate::buffer::Buffer,
    absolute_error_pos: usize,
    message: String,
) -> Flow {
    let accessible = buffer.accessible_emacs_byte_region();
    let end = accessible.clamp(EmacsBytePos::new(absolute_error_pos));
    let range = EmacsByteRange::new(accessible.start(), end);
    let mut prefix = Vec::with_capacity(range.len().get());
    buffer.copy_emacs_byte_range_to(range, &mut prefix);
    let line = prefix.iter().filter(|&&byte| byte == b'\n').count() as i64 + 1;
    let line_start = prefix
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);
    let column = if buffer.get_multibyte() {
        crate::emacs_core::emacs_char::chars_in_multibyte(&prefix[line_start..]) as i64
    } else {
        (prefix.len() - line_start) as i64
    };
    signal(
        LispCondition::InvalidReadSyntax,
        vec![
            Value::string(message),
            Value::fixnum(line),
            Value::fixnum(column),
        ],
    )
}

fn end_of_file_during_parsing_error() -> Flow {
    signal(
        LispCondition::EndOfFile,
        vec![Value::string("End of file during parsing")],
    )
}

/// Read the next top-level form from the active load-read cursor — the stream
/// `standard-input` is bound to during a `load`/`eval-buffer` readevalloop
/// (see [`crate::emacs_core::eval::LOAD_READ_STREAM_SYMBOL`]).  Advancing the
/// shared byte cursor makes the enclosing loop resume *after* this form,
/// exactly like GNU's shared `readcharfun` (lread.c `readevalloop`): a file
/// that calls `(read)` mid-load consumes its next top-level form.
fn read_from_active_load_cursor(
    ctx: &mut crate::emacs_core::eval::Context,
    locate_syms: bool,
) -> EvalResult {
    let Some(cursor) = ctx.load_read_cursors.last() else {
        // `standard-input` names the load stream but no load is active: treat
        // as a spent stream (EOF) rather than crashing.
        return Err(end_of_file_during_parsing_error());
    };
    let source = cursor.source;
    let pos = cursor.pos;
    let shorthands = cursor.shorthands.clone();

    let lisp_str = source.as_lisp_string().ok_or_else(|| {
        signal(
            "error",
            vec![Value::string("load-read stream source is not a string")],
        )
    })?;
    let read_source = super::value_reader::LispReadSource::new(lisp_str);
    let end = read_source.logical_len();
    if pos >= end {
        return Err(end_of_file_during_parsing_error());
    }
    let read_result = read_source
        .read_one_range_with_locate_syms(pos, end, locate_syms, &ctx.obarray, shorthands.as_ref())
        .map_err(signal_reader_error_from_string)?;
    let Some((value, next_pos)) = read_result else {
        return Err(end_of_file_during_parsing_error());
    };
    // Advance the shared cursor so the readevalloop resumes after this form.
    if let Some(cursor) = ctx.load_read_cursors.last_mut() {
        cursor.pos = next_pos;
    }
    ctx.obarray_mut().materialize_read_symbols(value);
    Ok(value)
}

fn signal_reader_error_from_string(e: super::value_reader::ReadError) -> Flow {
    match e.kind {
        super::value_reader::ReadErrorKind::EndOfFile => signal(LispCondition::EndOfFile, vec![]),
        super::value_reader::ReadErrorKind::Error => {
            signal("error", vec![Value::string(e.message)])
        }
        super::value_reader::ReadErrorKind::InvalidReadSyntax => signal(
            LispCondition::InvalidReadSyntax,
            vec![Value::string(e.message)],
        ),
        super::value_reader::ReadErrorKind::Signal => {
            signal(e.signal_symbol.as_deref().unwrap_or("error"), e.signal_data)
        }
    }
}

fn signal_reader_error_from_buffer(
    buffer: &crate::buffer::Buffer,
    e: super::value_reader::ReadError,
) -> Flow {
    match e.kind {
        super::value_reader::ReadErrorKind::EndOfFile => signal(LispCondition::EndOfFile, vec![]),
        super::value_reader::ReadErrorKind::Error => {
            signal("error", vec![Value::string(e.message)])
        }
        super::value_reader::ReadErrorKind::InvalidReadSyntax => {
            signal_invalid_read_syntax_in_buffer_object(buffer, e.position, e.message)
        }
        super::value_reader::ReadErrorKind::Signal => {
            signal(e.signal_symbol.as_deref().unwrap_or("error"), e.signal_data)
        }
    }
}

fn stdin_end_of_file_error() -> Flow {
    signal(
        LispCondition::EndOfFile,
        vec![Value::string("Error reading from stdin")],
    )
}

// ---------------------------------------------------------------------------
// 1. read-from-string
// ---------------------------------------------------------------------------

/// `(read-from-string STRING &optional START END)`
///
/// Parse a single Lisp object from STRING starting at position START (default 0).
/// Returns `(OBJECT . END-POSITION)` where END-POSITION is the character index
/// after the parsed object.
/// Fetch the active `read-symbol-shorthands` value and build the reader's
/// shorthand table.  GNU's `read`/`read-from-string` consult this variable
/// (set **buffer-local** by `hack-local-variables` during
/// `byte-compile-file`), so reading source that declares
/// `read-symbol-shorthands` in its local variables rewrites `prefix:name`
/// symbols.  The value must be resolved with buffer-local visibility — the
/// global binding is normally nil — hence we go through
/// `visible_runtime_variable_value_by_id` rather than the raw obarray slot.
/// Returns `None` when unset/nil.
fn current_read_symbol_shorthands(
    eval: &super::eval::Context,
) -> Option<super::value_reader::ReadSymbolShorthands> {
    let sym = crate::emacs_core::intern::intern("read-symbol-shorthands");
    let value = eval
        .visible_runtime_variable_value_by_id(sym)
        .ok()
        .flatten()?;
    if value.is_nil() {
        return None;
    }
    super::value_reader::ReadSymbolShorthands::from_lisp_value(value)
}

pub(crate) fn builtin_read_from_string(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    let shorthands = current_read_symbol_shorthands(ctx);
    let result = read_from_string_impl_inner(&ctx.obarray, args, false, shorthands.as_ref())?;
    if result.is_cons() {
        ctx.obarray_mut()
            .materialize_read_symbols(result.cons_car());
    }
    Ok(result)
}

pub(crate) fn read_from_string_impl(
    obarray: &crate::emacs_core::symbol::Obarray,
    args: Vec<Value>,
) -> EvalResult {
    read_from_string_impl_inner(obarray, args, false, None)
}

fn read_from_string_impl_inner(
    obarray: &crate::emacs_core::symbol::Obarray,
    args: Vec<Value>,
    locate_syms: bool,
    shorthands: Option<&super::value_reader::ReadSymbolShorthands>,
) -> EvalResult {
    expect_min_args("read-from-string", &args, 1)?;
    if args.len() > 3 {
        return Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![
                Value::symbol("read-from-string"),
                Value::fixnum(args.len() as i64),
            ],
        ));
    }

    let full_string = expect_lisp_string(&args[0])?;
    let read_source = super::value_reader::LispReadSource::new(&full_string);

    // GNU Emacs `Fread_from_string` (`src/lread.c:2514`) treats START and
    // END as character indices into STRING (validated via
    // `validate_subarray` against `SCHARS (string)`), translates them to
    // byte offsets through `string_char_to_byte`, and reports
    // FINAL-STRING-INDEX as a *character* index too. Indexing by raw
    // UTF-8 byte length here was a long-standing bug (audit §11.6) that
    // would either panic on multibyte input (slicing mid-codepoint) or
    // return a byte offset where elisp expected a character count.
    let full_string_bytes = full_string.as_bytes();
    let char_count = full_string.schars();

    let start_arg = args.get(1).cloned().unwrap_or(Value::NIL);
    let end_arg = args.get(2).cloned().unwrap_or(Value::NIL);
    let to_char_index = |value: &Value| -> Result<usize, Flow> {
        match value.kind() {
            ValueKind::Nil => Ok(0),
            ValueKind::Fixnum(n) => {
                let idx = if n < 0 { (char_count as i64) + n } else { n };
                if idx < 0 || idx > char_count as i64 {
                    return Err(signal(
                        LispCondition::ArgsOutOfRange,
                        vec![args[0], start_arg, end_arg],
                    ));
                }
                Ok(idx as usize)
            }
            _ => Err(signal(
                LispCondition::WrongTypeArgument,
                vec![Value::symbol("integerp"), *value],
            )),
        }
    };
    let start_char = if args.len() > 1 {
        to_char_index(&start_arg)?
    } else {
        0
    };
    let end_char = if args.len() > 2 {
        to_char_index(&end_arg)?
    } else {
        char_count
    };

    if start_char > end_char {
        return Err(signal(
            LispCondition::ArgsOutOfRange,
            vec![args[0], start_arg, end_arg],
        ));
    }

    let start_byte = if full_string.is_multibyte() {
        crate::emacs_core::emacs_char::char_to_byte_pos(full_string_bytes, start_char)
    } else {
        start_char
    };
    let end_byte = if full_string.is_multibyte() {
        crate::emacs_core::emacs_char::char_to_byte_pos(full_string_bytes, end_char)
    } else {
        end_char
    };

    let read_result = read_source.read_one_range_with_locate_syms(
        start_byte,
        end_byte,
        locate_syms,
        obarray,
        shorthands,
    );

    let (value, absolute_end_byte) = read_result
        .map_err(signal_reader_error_from_string)?
        .ok_or_else(|| signal(LispCondition::EndOfFile, vec![]))?;

    let absolute_end_char = if full_string.is_multibyte() {
        crate::emacs_core::emacs_char::byte_to_char_pos(full_string_bytes, absolute_end_byte)
    } else {
        absolute_end_byte
    };

    Ok(Value::cons(value, Value::fixnum(absolute_end_char as i64)))
}

// ---------------------------------------------------------------------------
// 2. read
// ---------------------------------------------------------------------------

/// `(read &optional STREAM)`
///
/// Read one Lisp expression from STREAM.
/// - If STREAM is a string, read from that string (equivalent to car of read-from-string).
/// - If STREAM is nil, read from `standard-input`.
/// - If STREAM is a buffer, read from buffer at point.
pub fn builtin_read(ctx: &mut crate::emacs_core::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_read_impl(ctx, args, false)
}

/// Shared implementation for `read` and `read-positioning-symbols`.
/// When `locate_syms` is true, every interned symbol (except nil) is
/// wrapped in a `symbol-with-pos` object carrying its source byte offset.
pub fn builtin_read_impl(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
    locate_syms: bool,
) -> EvalResult {
    expect_max_args("read", &args, 1)?;

    let stream = if args.is_empty() || args[0].is_nil() {
        ctx.obarray
            .symbol_value("standard-input")
            .copied()
            .unwrap_or(Value::NIL)
    } else {
        args[0]
    };

    if stream.is_nil() {
        // In batch/non-interactive runs, stdin-backed read signals EOF.
        return Err(signal(
            LispCondition::EndOfFile,
            vec![Value::string("End of file during parsing")],
        ));
    }

    let shorthands = current_read_symbol_shorthands(ctx);
    match stream.kind() {
        ValueKind::String => {
            // Read from string
            let result = read_from_string_impl_inner(
                &ctx.obarray,
                vec![stream],
                locate_syms,
                shorthands.as_ref(),
            )?;
            // Return just the car (the parsed object)
            match result.kind() {
                ValueKind::Cons => {
                    let pair_car = result.cons_car();
                    ctx.obarray_mut().materialize_read_symbols(pair_car);
                    Ok(pair_car)
                }
                _ => Ok(result),
            }
        }
        ValueKind::Veclike(VecLikeType::Buffer) => {
            let buf_id = stream.as_buffer_id().unwrap();
            let (maybe_value, new_pt) = {
                let buf = ctx
                    .buffers
                    .get(buf_id)
                    .ok_or_else(|| signal("error", vec![Value::string("Buffer does not exist")]))?;

                let start = buf.point_emacs_byte_pos();
                let end = buf.accessible_emacs_byte_region().end();
                if start >= end {
                    return Err(end_of_file_during_parsing_error());
                }

                match super::value_reader::read_one_from_buffer_with_locate_syms(
                    buf,
                    EmacsByteRange::new(start, end),
                    locate_syms,
                    &ctx.obarray,
                    shorthands.as_ref(),
                ) {
                    Ok(result) => result,
                    Err(e) => return Err(signal_reader_error_from_buffer(buf, e)),
                }
            };

            let _ = &mut ctx.buffers.goto_buffer_emacs_byte_pos(buf_id, new_pt);
            let value = maybe_value.ok_or_else(end_of_file_during_parsing_error)?;
            ctx.obarray_mut().materialize_read_symbols(value);
            Ok(value)
        }
        ValueKind::Symbol(id)
            if resolve_sym(id) == crate::emacs_core::eval::LOAD_READ_STREAM_SYMBOL =>
        {
            read_from_active_load_cursor(ctx, locate_syms)
        }
        ValueKind::Symbol(id) => Err(signal(
            LispCondition::VoidFunction,
            vec![Value::symbol(resolve_sym(id))],
        )),
        ValueKind::T => {
            // GNU `Fread` (lread.c): a `t` stream -- including the batch default
            // `standard-input` = t reached by `(read)` with no argument -- maps
            // to `(read-minibuffer "Lisp expression: ")`: read one line (from the
            // minibuffer interactively, or from stdin in `--batch`, prompt and
            // all) and parse it as a single Lisp expression. neomacs previously
            // signaled `end-of-file` outright, so a piped `echo '(+ 1 2)' |
            // neomacs --batch --eval '(print (read))'` couldn't read its input.
            let prompt = Value::string("Lisp expression: ");
            let input = builtin_read_from_minibuffer(ctx, vec![prompt])?;
            let result = read_from_string_impl_inner(
                &ctx.obarray,
                vec![input],
                locate_syms,
                shorthands.as_ref(),
            )?;
            match result.kind() {
                ValueKind::Cons => {
                    let pair_car = result.cons_car();
                    ctx.obarray_mut().materialize_read_symbols(pair_car);
                    Ok(pair_car)
                }
                _ => Ok(result),
            }
        }
        _ => {
            // Unsupported stream source type for read-char function protocol.
            Err(signal(LispCondition::InvalidFunction, vec![stream]))
        }
    }
}

// ---------------------------------------------------------------------------
// 5. read-from-minibuffer
// ---------------------------------------------------------------------------

/// `(read-from-minibuffer PROMPT &optional INITIAL KEYMAP READ HIST DEFAULT INHERIT-INPUT-METHOD)`
///
/// Read a string from the minibuffer.
/// In interactive mode, sets up the minibuffer buffer, enters recursive-edit,
/// and returns the user's input when they press RET (exit-minibuffer).
/// In batch mode, signals `end-of-file`.
pub(crate) fn builtin_read_from_minibuffer(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if let Some(result) = builtin_read_from_minibuffer_in_runtime(eval, &args)? {
        return Ok(result);
    }
    finish_read_from_minibuffer_in_eval(eval, &args)
}

fn read_from_stdin_noninteractive(prompt: &str) -> EvalResult {
    print!("{prompt}");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) => Err(stdin_end_of_file_error()),
        Ok(_) => {
            let input = line.trim_end_matches(['\n', '\r']);
            Ok(Value::string(input))
        }
        Err(_) => Err(stdin_end_of_file_error()),
    }
}

pub(crate) fn finish_read_from_minibuffer_in_eval(
    eval: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    finish_read_from_minibuffer_in_eval_with_setup(eval, args, |_| Ok(Value::NIL))
}

fn finish_read_from_minibuffer_in_eval_with_setup(
    eval: &mut super::eval::Context,
    args: &[Value],
    mut run_before_setup_hook: impl FnMut(&mut super::eval::Context) -> EvalResult,
) -> EvalResult {
    // GNU `read_minibuf` saves the OUTER command's `this-command-keys` on
    // `minibuf_save_list` (minibuf.c:738-739, `Fthis_command_keys_vector ()`)
    // and `read_minibuf_unwind` restores it on EVERY teardown path
    // (minibuf.c:1144-1146, `this_command_keys = key_vec`). The minibuffer's
    // own recursive-edit command loop reads and commits its own key sequences
    // (the closing RET ends up as `this-command-keys` == [13]); without
    // restoring the outer keys, the command that invoked the minibuffer (e.g.
    // `query-replace`/`perform-replace`) would see that stale [13] in its next
    // `read-key`, whose idle timer (subr.el:3648-3665) then throws immediately
    // and leaks the user's real keystroke into the buffer. Snapshot here and
    // restore unconditionally after the recursive edit so the outer
    // `this-command-keys` survives the minibuffer recursion, exactly like GNU.
    let saved_command_keys = eval.read_command_keys().to_vec();
    let saved_raw_command_keys = eval.read_raw_command_keys().to_vec();

    let eval_ptr = std::ptr::NonNull::from(&mut *eval);
    let command_loop_depth = eval.recursive_command_loop_depth();
    let result = finish_read_from_minibuffer_in_state_with_recursive_edit(
        &mut eval.obarray,
        &mut eval.buffers,
        &mut eval.frames,
        &mut eval.minibuffers,
        &mut eval.minibuffer_selected_window,
        &mut eval.active_minibuffer_window,
        command_loop_depth,
        args,
        move || unsafe {
            run_minibuffer_mode_if_bound(eval_ptr.as_ptr().as_mut().unwrap(), "minibuffer-mode")
        },
        move || unsafe {
            let eval = eval_ptr.as_ptr().as_mut().unwrap();
            run_before_setup_hook(eval)?;
            eval.run_hook_if_bound("minibuffer-setup-hook")
        },
        move || unsafe {
            match eval_ptr
                .as_ptr()
                .as_mut()
                .unwrap()
                .run_hook_if_bound("minibuffer-exit-hook")
            {
                Ok(value) => Ok(value),
                Err(Flow::Signal(_)) => Ok(Value::NIL),
                Err(flow) => Err(flow),
            }
        },
        move || unsafe {
            run_minibuffer_mode_if_bound(
                eval_ptr.as_ptr().as_mut().unwrap(),
                "minibuffer-inactive-mode",
            )
        },
        move || unsafe {
            eval_ptr
                .as_ptr()
                .as_mut()
                .unwrap()
                .minibuffer_command_loop_inner()
        },
    );
    // Restore the outer command's `this-command-keys` (GNU
    // `read_minibuf_unwind`, minibuf.c:1144-1146). Done on every path — normal
    // return, `'exit` throw, and error — so the invoking command's key context
    // is never clobbered by the minibuffer's own command-loop reads.
    eval.set_command_key_sequences(saved_command_keys, saved_raw_command_keys);
    if result.is_ok() {
        eval.note_interactive_minibuffer_read();
    }
    result
}

pub(crate) fn builtin_read_from_minibuffer_in_runtime(
    runtime: &impl KeyboardInputRuntime,
    args: &[Value],
) -> Result<Option<Value>, Flow> {
    expect_min_args("read-from-minibuffer", args, 1)?;
    expect_max_args("read-from-minibuffer", args, 7)?;
    let prompt = expect_lisp_string(&args[0])?;
    if let Some(initial) = args.get(1) {
        expect_initial_input_stringish(initial)?;
    }

    match runtime.minibuffer_input_source() {
        MinibufferInputSource::CommandLoop => Ok(None),
        MinibufferInputSource::StandardInput => read_from_stdin_noninteractive(
            &crate::emacs_core::emacs_char::to_utf8_lossy(prompt.as_bytes()),
        )
        .map(Some),
    }
}

/// Shared runtime setup/teardown for `read-from-minibuffer`.
///
/// GNU's `read_minibuf` is a C/runtime path that only enters the command
/// loop for the actual recursive edit. This helper mirrors that shape: it
/// performs buffer/window setup and final result handling in shared runtime
/// state, and delegates only the recursive edit itself to the callback.
#[allow(clippy::too_many_arguments)] // mirrors GNU read_minibuf's split runtime state
pub(crate) fn finish_read_from_minibuffer_in_state_with_recursive_edit(
    obarray: &mut super::symbol::Obarray,
    buffers: &mut crate::buffer::BufferManager,
    frames: &mut crate::window::FrameManager,
    minibuffers: &mut crate::emacs_core::minibuffer::MinibufferManager,
    minibuffer_selected_window: &mut Option<crate::window::WindowId>,
    active_minibuffer_window: &mut Option<crate::window::WindowId>,
    recursive_depth: usize,
    args: &[Value],
    mut run_active_mode: impl FnMut() -> EvalResult,
    mut run_setup_hook: impl FnMut() -> EvalResult,
    mut run_exit_hook: impl FnMut() -> EvalResult,
    run_inactive_mode: impl FnOnce() -> EvalResult,
    mut run_recursive_edit: impl FnMut() -> EvalResult,
) -> EvalResult {
    // Check inhibit-interaction — GNU Emacs signals an error when any
    // interactive read is attempted while this variable is non-nil.
    if obarray
        .symbol_value("inhibit-interaction")
        .is_some_and(|v| v.is_truthy())
    {
        return Err(signal(
            "inhibited-interaction",
            vec![Value::string(
                "Attempt to interact with user while inhibit-interaction is non-nil",
            )],
        ));
    }

    let prompt = expect_lisp_string(&args[0])?;
    let prompt_display = crate::emacs_core::emacs_char::to_utf8_lossy(prompt.as_bytes());
    // Extract optional arguments
    let initial_input = args.get(1).and_then(reader_initial_input_lisp_string);
    let keymap_arg = args.get(2).copied().unwrap_or(Value::NIL);
    let read_arg = args.get(3).copied().unwrap_or(Value::NIL);
    let history_spec = minibuffer_history_spec(args.get(4));
    let default_val = args.get(5).copied().unwrap_or(Value::NIL);

    // Save state.  GNU read_minibuf saves Vcurrent_prefix_arg in
    // minibuf_save_list and restores it during read_minibuf_unwind;
    // minibuffer commands may clobber it while reading input.
    let saved_buffer_id = buffers.current_buffer().map(|b| b.id);
    let saved_current_prefix_arg = obarray
        .symbol_value("current-prefix-arg")
        .copied()
        .unwrap_or(Value::NIL);
    let saved_minibuffer_history_variable = obarray
        .symbol_value("minibuffer-history-variable")
        .copied()
        .unwrap_or(Value::from_sym_id(intern("minibuffer-history")));
    let saved_minibuffer_history_position = obarray
        .symbol_value("minibuffer-history-position")
        .copied()
        .unwrap_or(Value::NIL);

    // GNU `read_minibuf` captures the caller's directory before switching to
    // *Minibuf-N*, then installs it after minibuffer-mode has reset locals.
    let ambient_directory = minibuffer_ambient_directory_in_state(buffers);

    // Find or create *Minibuf-N* buffer
    let minibuf_depth = minibuffers.depth() + 1;
    let minibuf_id = find_or_create_minibuffer_buffer_in_state(buffers, minibuf_depth);

    let active_window_state = activate_minibuffer_window_in_state(
        frames,
        buffers,
        minibuffer_selected_window,
        active_minibuffer_window,
        minibuf_id,
    );
    if active_window_state.is_none() {
        // Batch/no-frame fallback: still switch current buffer so tests without
        // a realized GUI frame can exercise the minibuffer logic.
        buffers.switch_current(minibuf_id);
    }
    run_active_mode()?;
    install_minibuffer_ambient_directory_in_state(buffers, minibuf_id, ambient_directory);

    // Clear the minibuffer buffer and insert prompt + initial input
    let prompt_properties = obarray
        .symbol_value("minibuffer-prompt-properties")
        .copied()
        .unwrap_or(Value::NIL);
    let prompt_byte_pos = super::minibuffer::install_minibuffer_buffer_text(
        buffers,
        minibuf_id,
        &prompt,
        initial_input.as_ref(),
        prompt_properties,
    );
    tracing::debug!(
        "read-from-minibuffer: prompt={:?} minibuf_id={:?} current_buffer={:?} active_window={:?} selected_window={:?}",
        prompt_display,
        minibuf_id,
        buffers.current_buffer_id(),
        *active_minibuffer_window,
        frames.selected_frame().map(|frame| frame.selected_window)
    );

    let enable_recursive = obarray
        .symbol_value("enable-recursive-minibuffers")
        .copied()
        .unwrap_or(Value::NIL)
        .is_truthy();
    minibuffers.set_enable_recursive(enable_recursive);
    let state = minibuffers.read_from_minibuffer_lisp(
        minibuf_id,
        &prompt,
        initial_input.as_ref(),
        history_spec.history_name,
    )?;
    state.command_loop_depth = recursive_depth;

    // Set local keymap: use KEYMAP arg if provided, otherwise minibuffer-local-map
    let minibuf_keymap = if !keymap_arg.is_nil() {
        keymap_arg
    } else {
        obarray
            .symbol_value("minibuffer-local-map")
            .copied()
            .unwrap_or(Value::NIL)
    };
    let _ = buffers.set_current_local_map(minibuf_keymap);

    // Set minibuffer-related variables
    obarray.set_symbol_value("minibuffer-prompt", Value::heap_string(prompt.clone()));
    obarray.set_symbol_value("minibuffer-depth", Value::fixnum(minibuf_depth as i64));
    obarray.set_symbol_value("minibuffer-history-variable", history_spec.variable_value);
    obarray.set_symbol_value("minibuffer-history-position", history_spec.position);

    run_setup_hook()?;

    // Enter recursive edit — the command loop runs until exit-minibuffer throws 'exit.
    let edit_result = run_recursive_edit();

    // Read the minibuffer contents (everything after the prompt)
    let result_text = minibuffer_result_lisp_string(buffers, minibuf_id, prompt_byte_pos);

    let _ = buffers.switch_current_unrecorded(minibuf_id);
    let exit_hook_result = match run_exit_hook() {
        Err(Flow::Signal(_)) => Ok(Value::NIL),
        other => other,
    };

    let exited_normally = match &edit_result {
        Ok(_) => true,
        Err(Flow::Throw { tag, value }) if tag.is_symbol_named("exit") => !value.is_truthy(),
        _ => false,
    };

    match &edit_result {
        Ok(_) => {
            let _ = minibuffers.exit_minibuffer();
        }
        Err(Flow::Throw { tag, value }) if tag.is_symbol_named("exit") => {
            if value.is_truthy() {
                minibuffers.abort_minibuffer();
            } else {
                let _ = minibuffers.exit_minibuffer();
            }
        }
        Err(_) => {
            minibuffers.abort_minibuffer();
        }
    }

    // Restore state. Route the full teardown (reset expired buffer + overlays,
    // inactive-mode, restore window buffer, force-resize at the outermost level)
    // through the single `teardown_minibuffer_level_in_state` boundary so exit
    // and abort tear down identically (GNU runs the same unwind for both).
    let depth_after_pop = minibuffers.depth();
    let inactive_mode_result = if let Some(saved) = active_window_state {
        let _ = buffers.switch_current_unrecorded(minibuf_id);
        teardown_minibuffer_level_in_state(
            frames,
            buffers,
            minibuffer_selected_window,
            active_minibuffer_window,
            minibuf_id,
            depth_after_pop,
            saved,
            run_inactive_mode,
        )
    } else {
        Ok(Value::NIL)
    };
    if let Some(buf_id) = saved_buffer_id {
        buffers.switch_current(buf_id);
    }
    tracing::debug!(
        "read-from-minibuffer: restored current_buffer={:?} active_window={:?} selected_window={:?}",
        buffers.current_buffer_id(),
        *active_minibuffer_window,
        frames.selected_frame().map(|frame| frame.selected_window)
    );
    obarray.set_symbol_value(
        "minibuffer-depth",
        Value::fixnum(minibuffers.depth() as i64),
    );
    obarray.set_symbol_value("current-prefix-arg", saved_current_prefix_arg);
    obarray.set_symbol_value(
        "minibuffer-history-variable",
        saved_minibuffer_history_variable,
    );
    obarray.set_symbol_value(
        "minibuffer-history-position",
        saved_minibuffer_history_position,
    );
    exit_hook_result?;
    inactive_mode_result?;

    if exited_normally
        && history_add_new_input_enabled(obarray)
        && let Some(history_name) = history_spec.history_name
    {
        add_to_minibuffer_history_variable(obarray, history_name, &result_text);
        let max_length = minibuffer_history_limit(obarray, history_name).unwrap_or(usize::MAX);
        minibuffers.add_to_history_lisp(history_name, result_text.clone(), max_length);
    }

    // Handle the recursive edit result
    match edit_result {
        Ok(_) | Err(Flow::Throw { .. }) => {
            // Normal exit (throw 'exit from exit-minibuffer)
            // If READ arg is non-nil, evaluate the result as a Lisp expression
            if !read_arg.is_nil() && !result_text.as_bytes().is_empty() {
                // READ is non-nil: parse the result string as a Lisp expression
                // (like calling (read STRING)) and return the parsed object.
                let read_result =
                    read_from_string_impl(obarray, vec![Value::heap_string(result_text.clone())])?;
                // read-from-string returns (OBJECT . END-POS), extract OBJECT
                if read_result.is_cons() {
                    return Ok(read_result.cons_car());
                }
                return Ok(read_result);
            }

            // If result is empty and DEFAULT is provided, use it
            if result_text.as_bytes().is_empty() && !default_val.is_nil() {
                return Ok(default_val);
            }

            Ok(Value::heap_string(result_text))
        }
        Err(flow) => Err(flow),
    }
}

// ---------------------------------------------------------------------------
// 6. read-string
// ---------------------------------------------------------------------------

/// `(read-string PROMPT &optional INITIAL HISTORY DEFAULT INHERIT-INPUT-METHOD)`
///
/// Read a string from the minibuffer.  Delegates to `read-from-minibuffer`.
pub(crate) fn builtin_read_string(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    if let Some(result) = builtin_read_string_in_runtime(eval, &args)? {
        return Ok(result);
    }
    finish_read_string_in_eval(eval, &args)
}

pub(crate) fn finish_read_string_in_eval(
    eval: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    finish_read_string_with_minibuffer(args, |minibuffer_args| {
        finish_read_from_minibuffer_in_eval(eval, minibuffer_args)
    })
}

pub(crate) fn builtin_read_string_in_runtime(
    runtime: &impl KeyboardInputRuntime,
    args: &[Value],
) -> Result<Option<Value>, Flow> {
    expect_min_args("read-string", args, 1)?;
    expect_max_args("read-string", args, 5)?;
    let prompt = args[0];
    if let Some(initial) = args.get(1) {
        expect_initial_input_stringish(initial)?;
    }

    match runtime.minibuffer_input_source() {
        MinibufferInputSource::CommandLoop => Ok(None),
        MinibufferInputSource::StandardInput => {
            let prompt_str = expect_lisp_string(&prompt)?;
            read_from_stdin_noninteractive(&crate::emacs_core::emacs_char::to_utf8_lossy(
                prompt_str.as_bytes(),
            ))
            .map(Some)
        }
    }
}

pub(crate) fn finish_read_string_with_minibuffer(
    args: &[Value],
    mut read_from_minibuffer: impl FnMut(&[Value]) -> EvalResult,
) -> EvalResult {
    let prompt = args[0];

    // (read-from-minibuffer PROMPT INITIAL nil nil HIST DEFAULT INHERIT-INPUT-METHOD)
    let initial = args.get(1).copied().unwrap_or(Value::NIL);
    let history = args.get(2).copied().unwrap_or(Value::NIL);
    let default = args.get(3).copied().unwrap_or(Value::NIL);
    let inherit = args.get(4).copied().unwrap_or(Value::NIL);

    let minibuffer_args = [
        prompt,
        initial,
        Value::NIL,
        Value::NIL,
        history,
        default,
        inherit,
    ];
    read_from_minibuffer(&minibuffer_args)
}

// ---------------------------------------------------------------------------
// 7. read-number
// ---------------------------------------------------------------------------

/// `(read-number PROMPT &optional DEFAULT)`
///
/// Read a numeric value from the minibuffer.
/// Delegates to read-from-minibuffer with READ=t, then validates the result
/// is a number.
pub(crate) fn builtin_read_number(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    builtin_read_number_in_runtime(eval, &args)?;
    finish_read_number_in_eval(eval, &args)
}

pub(crate) fn builtin_read_number_in_runtime(
    runtime: &impl KeyboardInputRuntime,
    args: &[Value],
) -> Result<(), Flow> {
    expect_min_args("read-number", args, 1)?;
    expect_max_args("read-number", args, 3)?;
    let prompt = args[0];
    expect_lisp_string(&prompt)?;
    if let Some(default) = args.get(1)
        && !default.is_nil()
    {
        expect_number(default)?;
    }
    match runtime.minibuffer_input_source() {
        MinibufferInputSource::CommandLoop => Ok(()),
        MinibufferInputSource::StandardInput => Err(stdin_end_of_file_error()),
    }
}

fn read_number_minibuffer_args(args: &[Value]) -> [Value; 6] {
    let prompt = args[0];
    let default_val = args.get(1).copied().unwrap_or(Value::NIL);
    [
        prompt,
        Value::NIL,
        Value::NIL,
        Value::T,
        Value::NIL,
        default_val,
    ]
}

fn validate_read_number_result(result: Value) -> EvalResult {
    if result.is_number() {
        return Ok(result);
    }
    Err(signal("error", vec![Value::string("Not a number")]))
}

pub(crate) fn finish_read_number_with_minibuffer(
    args: &[Value],
    mut read_from_minibuffer: impl FnMut(&[Value]) -> EvalResult,
) -> EvalResult {
    let minibuffer_args = read_number_minibuffer_args(args);
    validate_read_number_result(read_from_minibuffer(&minibuffer_args)?)
}

pub(crate) fn finish_read_number_in_eval(
    eval: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    finish_read_number_with_minibuffer(args, |minibuffer_args| {
        finish_read_from_minibuffer_in_eval(eval, minibuffer_args)
    })
}

pub(crate) fn finish_read_number_in_vm_runtime(
    shared: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    builtin_read_number_in_runtime(shared, args)?;
    finish_read_number_with_minibuffer(args, |minibuffer_args| {
        finish_read_from_minibuffer_in_vm_runtime(shared, minibuffer_args)
    })
}

// ---------------------------------------------------------------------------
// 8. completing-read
// ---------------------------------------------------------------------------

/// `(completing-read PROMPT COLLECTION &optional PREDICATE REQUIRE-MATCH
///                    INITIAL-INPUT HIST DEF INHERIT-INPUT-METHOD)`
///
/// Read a string from the minibuffer with completion.
/// In interactive mode, delegates to read-from-minibuffer with
/// minibuffer-local-completion-map (or minibuffer-local-must-match-map
/// if REQUIRE-MATCH is non-nil).
pub(crate) fn builtin_completing_read(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    validate_completing_read_arity(&args)?;
    if let Some(function) = completing_read_function_value(eval) {
        return eval.apply(function, args);
    }

    if let Some(result) = builtin_completing_read_in_runtime(eval, &args)? {
        return Ok(result);
    }
    finish_completing_read_in_eval(eval, &args)
}

pub(crate) fn finish_completing_read_in_eval(
    eval: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    let minibuffer_args = completing_read_minibuffer_args(eval.obarray(), args);
    let collection = args[1];
    let predicate = args.get(2).copied().unwrap_or(Value::NIL);
    let require_match = args.get(3).copied().unwrap_or(Value::NIL);
    let original_buffer = eval
        .buffers
        .current_buffer_id()
        .map(Value::make_buffer)
        .unwrap_or(Value::NIL);
    let completion_ignore_case = eval
        .eval_symbol("completion-ignore-case")
        .unwrap_or(Value::NIL);

    finish_read_from_minibuffer_in_eval_with_setup(eval, &minibuffer_args, move |eval| {
        install_completing_read_minibuffer_locals(
            eval,
            collection,
            predicate,
            require_match,
            original_buffer,
            completion_ignore_case,
        );
        Ok(Value::NIL)
    })
}

pub(crate) fn builtin_completing_read_in_runtime(
    runtime: &impl KeyboardInputRuntime,
    args: &[Value],
) -> Result<Option<Value>, Flow> {
    validate_completing_read_arity(args)?;
    let prompt = expect_lisp_string(&args[0])?;
    if let Some(initial) = args.get(4) {
        expect_completing_read_initial_input(initial)?;
    }

    match runtime.minibuffer_input_source() {
        MinibufferInputSource::CommandLoop => Ok(None),
        MinibufferInputSource::StandardInput => {
            // Batch/noninteractive: GNU's `Fcompleting_read` routes through
            // `read_minibuf` -> `read_minibuf_noninteractive` (minibuf.c), which
            // writes the prompt to stdout and reads the answer from stdin, exactly
            // like `read-from-minibuffer`.  Mirror that so the prompt is emitted
            // before the (likely) end-of-file signal on empty stdin.
            read_from_stdin_noninteractive(&crate::emacs_core::emacs_char::to_utf8_lossy(
                prompt.as_bytes(),
            ))
            .map(Some)
        }
    }
}

pub(crate) fn validate_completing_read_arity(args: &[Value]) -> Result<(), Flow> {
    expect_min_args("completing-read", args, 2)?;
    expect_max_args("completing-read", args, 8)?;
    Ok(())
}

pub(crate) fn completing_read_function_value(eval: &super::eval::Context) -> Option<Value> {
    eval.eval_symbol("completing-read-function")
        .ok()
        .filter(|function| !function.is_nil())
}

pub(crate) fn finish_read_from_minibuffer_in_vm_runtime(
    shared: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    finish_read_from_minibuffer_in_vm_runtime_with_setup(shared, args, |_| Ok(Value::NIL))
}

fn finish_read_from_minibuffer_in_vm_runtime_with_setup(
    shared: &mut super::eval::Context,
    args: &[Value],
    mut run_before_setup_hook: impl FnMut(&mut super::eval::Context) -> EvalResult,
) -> EvalResult {
    if let Some(result) = builtin_read_from_minibuffer_in_runtime(shared, args)? {
        return Ok(result);
    }

    // Check inhibit-interaction — GNU Emacs signals an error when any
    // interactive read is attempted while this variable is non-nil.
    if shared
        .obarray
        .symbol_value("inhibit-interaction")
        .is_some_and(|v| v.is_truthy())
    {
        return Err(signal(
            "inhibited-interaction",
            vec![Value::string(
                "Attempt to interact with user while inhibit-interaction is non-nil",
            )],
        ));
    }

    let prompt = expect_lisp_string(&args[0])?;
    let prompt_display = crate::emacs_core::emacs_char::to_utf8_lossy(prompt.as_bytes());
    let initial_input = args.get(1).and_then(reader_initial_input_lisp_string);
    let keymap_arg = args.get(2).copied().unwrap_or(Value::NIL);
    let read_arg = args.get(3).copied().unwrap_or(Value::NIL);
    let history_spec = minibuffer_history_spec(args.get(4));
    let default_val = args.get(5).copied().unwrap_or(Value::NIL);

    // Save state.  GNU read_minibuf saves Vcurrent_prefix_arg in
    // minibuf_save_list and restores it during read_minibuf_unwind;
    // minibuffer commands may clobber it while reading input.
    let saved_buffer_id = shared.buffers.current_buffer().map(|b| b.id);
    let saved_current_prefix_arg = shared
        .obarray
        .symbol_value("current-prefix-arg")
        .copied()
        .unwrap_or(Value::NIL);
    // GNU `read_minibuf` also saves `(this-command-keys-vector)` (minibuf.c:
    // 738-739) and `read_minibuf_unwind` restores it (minibuf.c:1144-1146) so
    // the invoking command's `this-command-keys` survives the minibuffer's own
    // command-loop reads. Byte-compiled callers (`query-replace-read-to`,
    // `register-read-with-preview`, …) reach the minibuffer through THIS VM
    // runtime path, so the save/restore must live here too — otherwise their
    // following `read-key` sees the minibuffer's terminating RET and fires its
    // idle-timer probe early.
    let saved_command_keys = shared.read_command_keys().to_vec();
    let saved_raw_command_keys = shared.read_raw_command_keys().to_vec();
    let saved_minibuffer_history_variable = shared
        .obarray
        .symbol_value("minibuffer-history-variable")
        .copied()
        .unwrap_or(Value::from_sym_id(intern("minibuffer-history")));
    let saved_minibuffer_history_position = shared
        .obarray
        .symbol_value("minibuffer-history-position")
        .copied()
        .unwrap_or(Value::NIL);
    let recursive_depth = shared.recursive_command_loop_depth();

    // GNU `read_minibuf` captures the caller's directory before switching to
    // *Minibuf-N*, then installs it after minibuffer-mode has reset locals.
    let ambient_directory = minibuffer_ambient_directory_in_state(&shared.buffers);

    let minibuf_depth = shared.minibuffers.depth() + 1;
    let minibuf_id = find_or_create_minibuffer_buffer_in_state(&mut shared.buffers, minibuf_depth);

    let active_window_state = activate_minibuffer_window_in_state(
        &mut shared.frames,
        &mut shared.buffers,
        &mut shared.minibuffer_selected_window,
        &mut shared.active_minibuffer_window,
        minibuf_id,
    );
    if active_window_state.is_none() {
        shared.buffers.switch_current(minibuf_id);
    }
    run_minibuffer_mode_if_bound(shared, "minibuffer-mode")?;
    install_minibuffer_ambient_directory_in_state(
        &mut shared.buffers,
        minibuf_id,
        ambient_directory,
    );

    let prompt_properties = shared
        .obarray
        .symbol_value("minibuffer-prompt-properties")
        .copied()
        .unwrap_or(Value::NIL);
    let prompt_byte_pos = super::minibuffer::install_minibuffer_buffer_text(
        &mut shared.buffers,
        minibuf_id,
        &prompt,
        initial_input.as_ref(),
        prompt_properties,
    );
    tracing::debug!(
        "read-from-minibuffer: prompt={:?} minibuf_id={:?} current_buffer={:?} active_window={:?} selected_window={:?}",
        prompt_display,
        minibuf_id,
        shared.buffers.current_buffer_id(),
        shared.active_minibuffer_window,
        shared
            .frames
            .selected_frame()
            .map(|frame| frame.selected_window)
    );

    let enable_recursive = shared
        .obarray
        .symbol_value("enable-recursive-minibuffers")
        .copied()
        .unwrap_or(Value::NIL)
        .is_truthy();
    shared.minibuffers.set_enable_recursive(enable_recursive);
    {
        let state = shared.minibuffers.read_from_minibuffer_lisp(
            minibuf_id,
            &prompt,
            initial_input.as_ref(),
            history_spec.history_name,
        )?;
        state.command_loop_depth = recursive_depth;
    }

    let minibuf_keymap = if !keymap_arg.is_nil() {
        keymap_arg
    } else {
        shared
            .obarray
            .symbol_value("minibuffer-local-map")
            .copied()
            .unwrap_or(Value::NIL)
    };
    let _ = shared.buffers.set_current_local_map(minibuf_keymap);
    shared
        .obarray
        .set_symbol_value("minibuffer-prompt", Value::heap_string(prompt.clone()));
    shared
        .obarray
        .set_symbol_value("minibuffer-depth", Value::fixnum(minibuf_depth as i64));
    shared
        .obarray
        .set_symbol_value("minibuffer-history-variable", history_spec.variable_value);
    shared
        .obarray
        .set_symbol_value("minibuffer-history-position", history_spec.position);
    run_before_setup_hook(shared)?;
    shared.run_hook_if_bound("minibuffer-setup-hook")?;

    let gc_roots = shared.save_specpdl_roots();
    for root in args {
        shared.push_specpdl_root(*root);
    }
    let edit_result = shared.minibuffer_command_loop_inner();
    shared.restore_specpdl_roots(gc_roots);

    let result_text = minibuffer_result_lisp_string(&shared.buffers, minibuf_id, prompt_byte_pos);

    let _ = shared.buffers.switch_current_unrecorded(minibuf_id);
    let exit_hook_result = match shared.run_hook_if_bound("minibuffer-exit-hook") {
        Ok(value) => Ok(value),
        Err(Flow::Signal(_)) => Ok(Value::NIL),
        Err(flow) => Err(flow),
    };

    let exited_normally = match &edit_result {
        Ok(_) => true,
        Err(Flow::Throw { tag, value }) if tag.is_symbol_named("exit") => !value.is_truthy(),
        _ => false,
    };

    match &edit_result {
        Ok(_) => {
            let _ = shared.minibuffers.exit_minibuffer();
        }
        Err(Flow::Throw { tag, value }) if tag.is_symbol_named("exit") => {
            if value.is_truthy() {
                shared.minibuffers.abort_minibuffer();
            } else {
                let _ = shared.minibuffers.exit_minibuffer();
            }
        }
        Err(_) => {
            shared.minibuffers.abort_minibuffer();
        }
    }

    // Route the full teardown through the single
    // `teardown_minibuffer_level_in_state` boundary (the same one the eval-side
    // `finish_read_from_minibuffer_in_state_with_recursive_edit` uses) so exit
    // and abort are provably identical here too. The `minibuffer-inactive-mode`
    // hook needs `&mut shared`, while the boundary borrows individual `shared`
    // fields; mirror this file's established pattern and run the hook through a
    // raw pointer so the two borrows do not alias at the type level.
    let depth_after_pop = shared.minibuffers.depth();
    let inactive_mode_result = if let Some(saved) = active_window_state {
        let _ = shared.buffers.switch_current_unrecorded(minibuf_id);
        let shared_ptr = std::ptr::NonNull::from(&mut *shared);
        teardown_minibuffer_level_in_state(
            &mut shared.frames,
            &mut shared.buffers,
            &mut shared.minibuffer_selected_window,
            &mut shared.active_minibuffer_window,
            minibuf_id,
            depth_after_pop,
            saved,
            move || unsafe {
                run_minibuffer_mode_if_bound(
                    shared_ptr.as_ptr().as_mut().unwrap(),
                    "minibuffer-inactive-mode",
                )
            },
        )
    } else {
        Ok(Value::NIL)
    };
    if let Some(buf_id) = saved_buffer_id {
        shared.buffers.switch_current(buf_id);
    }
    tracing::debug!(
        "read-from-minibuffer: restored current_buffer={:?} active_window={:?} selected_window={:?}",
        shared.buffers.current_buffer_id(),
        shared.active_minibuffer_window,
        shared
            .frames
            .selected_frame()
            .map(|frame| frame.selected_window)
    );
    shared.obarray.set_symbol_value(
        "minibuffer-depth",
        Value::fixnum(shared.minibuffers.depth() as i64),
    );
    shared
        .obarray
        .set_symbol_value("current-prefix-arg", saved_current_prefix_arg);
    // Restore the invoking command's `this-command-keys` (GNU
    // `read_minibuf_unwind`, minibuf.c:1144-1146). Placed before the `?`
    // propagations below and after the (non-`?`) `edit_result` teardown so it
    // runs on every exit path: normal return, `'exit` throw, and error.
    shared.set_command_key_sequences(saved_command_keys, saved_raw_command_keys);
    shared.obarray.set_symbol_value(
        "minibuffer-history-variable",
        saved_minibuffer_history_variable,
    );
    shared.obarray.set_symbol_value(
        "minibuffer-history-position",
        saved_minibuffer_history_position,
    );
    exit_hook_result?;
    inactive_mode_result?;

    if exited_normally
        && history_add_new_input_enabled(&shared.obarray)
        && let Some(history_name) = history_spec.history_name
    {
        add_to_minibuffer_history_variable(&mut shared.obarray, history_name, &result_text);
        let max_length =
            minibuffer_history_limit(&shared.obarray, history_name).unwrap_or(usize::MAX);
        shared
            .minibuffers
            .add_to_history_lisp(history_name, result_text.clone(), max_length);
    }

    let result = match edit_result {
        Ok(_) | Err(Flow::Throw { .. }) => {
            if !read_arg.is_nil() && !result_text.as_bytes().is_empty() {
                let read_result = read_from_string_impl(
                    &shared.obarray,
                    vec![Value::heap_string(result_text.clone())],
                )?;
                if read_result.is_cons() {
                    return Ok(read_result.cons_car());
                }
                return Ok(read_result);
            }

            if result_text.as_bytes().is_empty() && !default_val.is_nil() {
                return Ok(default_val);
            }

            Ok(Value::heap_string(result_text))
        }
        Err(flow) => Err(flow),
    };
    if result.is_ok() {
        shared.note_interactive_minibuffer_read();
    }
    result
}

pub(crate) fn finish_completing_read_in_vm_runtime(
    shared: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    if let Some(result) = builtin_completing_read_in_runtime(shared, args)? {
        return Ok(result);
    }
    let minibuffer_args = completing_read_minibuffer_args(&shared.obarray, args);
    let collection = args[1];
    let predicate = args.get(2).copied().unwrap_or(Value::NIL);
    let require_match = args.get(3).copied().unwrap_or(Value::NIL);
    let original_buffer = shared
        .buffers
        .current_buffer_id()
        .map(Value::make_buffer)
        .unwrap_or(Value::NIL);
    let completion_ignore_case = shared
        .eval_symbol("completion-ignore-case")
        .unwrap_or(Value::NIL);

    finish_read_from_minibuffer_in_vm_runtime_with_setup(shared, &minibuffer_args, move |shared| {
        install_completing_read_minibuffer_locals(
            shared,
            collection,
            predicate,
            require_match,
            original_buffer,
            completion_ignore_case,
        );
        Ok(Value::NIL)
    })
}

/// Map the `REQUIRE-MATCH` argument of `completing-read` to the value
/// stored in `minibuffer-completion-confirm`.
///
/// GNU semantics:
///   nil        → nil
///   t          → nil
///   confirm    → confirm
///   confirm-after-completion → confirm-after-completion
///   function / other non-t, non-nil → unchanged
fn completion_confirm_from_require_match(require_match: Value) -> Value {
    match RequireMatchSymbol::from_lisp_value(require_match) {
        Some(RequireMatchSymbol::T) => Value::NIL,
        Some(RequireMatchSymbol::Confirm | RequireMatchSymbol::ConfirmAfterCompletion) => {
            require_match
        }
        None if require_match.is_nil() => Value::NIL,
        None => require_match,
    }
}

fn install_completing_read_minibuffer_locals(
    eval: &mut super::eval::Context,
    collection: Value,
    predicate: Value,
    require_match: Value,
    original_buffer: Value,
    completion_ignore_case: Value,
) {
    let Some(current_id) = eval.buffers.current_buffer_id() else {
        return;
    };
    for (name, value) in [
        ("minibuffer-completion-table", collection),
        ("minibuffer-completion-predicate", predicate),
        (
            "minibuffer-completion-confirm",
            completion_confirm_from_require_match(require_match),
        ),
        ("minibuffer--require-match", require_match),
        ("minibuffer--original-buffer", original_buffer),
        ("completion-ignore-case", completion_ignore_case),
    ] {
        let _ = eval.set_buffer_local_binding_by_id(current_id, intern(name), value);
    }
}

pub(crate) fn completing_read_minibuffer_args(obarray: &Obarray, args: &[Value]) -> [Value; 7] {
    let prompt = args[0];
    let require_match = args.get(3).copied().unwrap_or(Value::NIL);
    let initial_input = args.get(4).copied().unwrap_or(Value::NIL);
    let hist = args.get(5).copied().unwrap_or(Value::NIL);
    let default_val = args.get(6).copied().unwrap_or(Value::NIL);
    let inherit = args.get(7).copied().unwrap_or(Value::NIL);

    let keymap = if !require_match.is_nil() {
        obarray
            .symbol_value("minibuffer-local-must-match-map")
            .copied()
            .unwrap_or(Value::NIL)
    } else {
        obarray
            .symbol_value("minibuffer-local-completion-map")
            .copied()
            .unwrap_or(Value::NIL)
    };

    [
        prompt,
        initial_input,
        keymap,
        Value::NIL,
        hist,
        default_val,
        inherit,
    ]
}

fn event_to_int(event: &Value) -> Option<i64> {
    match event.kind() {
        ValueKind::Fixnum(n) => Some(n),
        _ => None,
    }
}

fn event_to_char(event: &Value) -> Option<char> {
    match event.kind() {
        ValueKind::Fixnum(c) => char::from_u32(c as u32),
        _ => None,
    }
}

fn expect_optional_prompt_string(args: &[Value]) -> Result<(), Flow> {
    if args.is_empty() || args[0].is_nil() || args[0].is_string() {
        return Ok(());
    }
    Err(signal(
        LispCondition::WrongTypeArgument,
        vec![Value::symbol("stringp"), args[0]],
    ))
}

fn non_character_input_event_error() -> Flow {
    signal("error", vec![Value::string("Non-character input-event")])
}

/// Where a minibuffer read obtains its input.
///
/// GNU Emacs does not use `noninteractive` alone to select stdin:
/// `read_minibuf` enters the command loop while a keyboard macro is executing,
/// even in batch mode.  Keeping that semantic decision separate from the
/// presence of a live terminal receiver prevents individual reader builtins
/// from accidentally disagreeing about batch keyboard macros.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MinibufferInputSource {
    CommandLoop,
    StandardInput,
}

/// Whether a character/event reader can obtain input from the command runtime.
///
/// Unlike minibuffer readers, `read-char`, `read-event`, and
/// `read-char-exclusive` never fall back to standard input.  They can still
/// read without a live frontend when a keyboard macro is executing or a
/// low-level event is queued.  Centralizing that distinction prevents each
/// builtin from recognizing a different subset of GNU `read_char`'s sources.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandEventInputSource {
    Runtime,
    Unavailable,
}

pub(crate) trait KeyboardInputRuntime {
    fn pop_unread_command_event(&mut self) -> Option<Value>;
    fn peek_unread_command_event(&self) -> Option<Value>;
    fn replace_unread_command_event_with_singleton(&mut self, event: Value);
    fn record_input_event(&mut self, event: Value);
    #[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
    fn record_nonmenu_input_event(&mut self, event: Value);
    fn set_read_command_keys(&mut self, keys: Vec<Value>);
    #[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
    fn clear_read_command_keys(&mut self);
    fn read_command_keys(&self) -> &[Value];
    fn has_input_receiver(&self) -> bool;
    fn is_executing_keyboard_macro(&self) -> bool;
    fn minibuffer_input_source(&self) -> MinibufferInputSource {
        if self.has_input_receiver() || self.is_executing_keyboard_macro() {
            MinibufferInputSource::CommandLoop
        } else {
            MinibufferInputSource::StandardInput
        }
    }
    fn has_pending_low_level_events(&self) -> bool {
        false
    }
    fn command_event_input_source(&self) -> CommandEventInputSource {
        if self.has_input_receiver()
            || self.is_executing_keyboard_macro()
            || self.has_pending_low_level_events()
        {
            CommandEventInputSource::Runtime
        } else {
            CommandEventInputSource::Unavailable
        }
    }
    #[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
    fn read_char_blocking(&mut self) -> Result<Value, Flow>;
    fn read_char_with_timeout(&mut self, timeout: Option<Duration>) -> Result<Option<Value>, Flow>;
    fn read_key_sequence_blocking(
        &mut self,
        options: crate::keyboard::ReadKeySequenceOptions,
    ) -> Result<(Vec<Value>, Value), Flow>;
    fn symbol_value_or_nil(&self, name: &str) -> Value;
}

impl KeyboardInputRuntime for super::eval::Context {
    fn pop_unread_command_event(&mut self) -> Option<Value> {
        super::eval::Context::pop_unread_command_event(self)
    }

    fn peek_unread_command_event(&self) -> Option<Value> {
        super::eval::Context::peek_unread_command_event(self)
    }

    fn replace_unread_command_event_with_singleton(&mut self, event: Value) {
        super::eval::Context::replace_unread_command_event_with_singleton(self, event);
    }

    fn record_input_event(&mut self, event: Value) {
        super::eval::Context::record_input_event(self, event);
    }

    fn record_nonmenu_input_event(&mut self, event: Value) {
        super::eval::Context::record_nonmenu_input_event(self, event);
    }

    fn set_read_command_keys(&mut self, keys: Vec<Value>) {
        super::eval::Context::set_read_command_keys(self, keys);
    }

    fn clear_read_command_keys(&mut self) {
        super::eval::Context::clear_read_command_keys(self);
    }

    fn read_command_keys(&self) -> &[Value] {
        super::eval::Context::read_command_keys(self)
    }

    fn has_input_receiver(&self) -> bool {
        super::eval::Context::has_input_receiver(self)
    }

    fn is_executing_keyboard_macro(&self) -> bool {
        self.command_loop.is_executing_kbd_macro()
    }

    fn has_pending_low_level_events(&self) -> bool {
        super::eval::Context::has_pending_low_level_events(self)
    }

    fn read_char_blocking(&mut self) -> Result<Value, Flow> {
        super::eval::Context::read_char(self)
    }

    fn read_char_with_timeout(&mut self, timeout: Option<Duration>) -> Result<Option<Value>, Flow> {
        super::eval::Context::read_char_with_timeout(self, timeout)
    }

    fn read_key_sequence_blocking(
        &mut self,
        options: crate::keyboard::ReadKeySequenceOptions,
    ) -> Result<(Vec<Value>, Value), Flow> {
        super::eval::Context::read_key_sequence_with_options(self, options)
    }

    fn symbol_value_or_nil(&self, name: &str) -> Value {
        self.obarray
            .symbol_value(name)
            .copied()
            .unwrap_or(Value::NIL)
    }
}

pub(crate) fn read_key_sequence_options_from_args(
    args: &[Value],
) -> crate::keyboard::ReadKeySequenceOptions {
    // GNU `Fread_key_sequence`/`Fread_key_sequence_vector` signature
    // (keyboard.c:11935) is
    //   (PROMPT CONTINUE-ECHO DONT-DOWNCASE-LAST CAN-RETURN-SWITCH-FRAME ...).
    // Arg 1 (CONTINUE-ECHO) governs whether the previous command's
    // `this-command-keys` is preserved (non-nil) or cleared for a fresh
    // sequence (nil); `read-key` (subr.el) passes nil here and relies on the
    // clear so its idle-timer `(this-command-keys-vector)` probe is empty.
    crate::keyboard::ReadKeySequenceOptions::new(
        args.first().copied().unwrap_or(Value::NIL),
        args.get(1).is_some_and(|v| v.is_truthy()),
        args.get(2).is_some_and(|v| v.is_truthy()),
        args.get(3).is_some_and(|v| v.is_truthy()),
    )
}

fn read_key_sequence_string_result(keys: &[Value]) -> Value {
    let mut chars_only = true;
    let mut s = String::new();
    for key in keys {
        if let Some(c) = event_to_char(key) {
            s.push(c);
        } else {
            chars_only = false;
            break;
        }
    }
    if chars_only {
        Value::string(s)
    } else {
        read_key_sequence_vector_result(keys)
    }
}

fn read_key_sequence_vector_result(keys: &[Value]) -> Value {
    Value::vector(
        keys.iter()
            .map(|key| event_to_int(key).map(Value::fixnum).unwrap_or(*key))
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// 10. input-pending-p
// ---------------------------------------------------------------------------

/// `(input-pending-p &optional CHECK-TIMERS)`
///
/// Return non-nil when unread input, staged host input, or `quit-flag` is pending.
/// `CHECK-TIMERS` is accepted and fires due timers before checking.
fn input_pending_now(ctx: &crate::emacs_core::eval::Context, filter_events: bool) -> bool {
    if peek_unread_command_event_in_state(&ctx.obarray, &[]).is_some() {
        return true;
    }

    if ctx.command_loop.keyboard.has_pending_kboard_input() {
        return true;
    }

    if !ctx.quit_flag_value().is_nil() {
        return true;
    }

    ctx.has_pending_frontend_input(filter_events)
}

pub(crate) fn builtin_input_pending_p(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_max_args("input-pending-p", &args, 1)?;
    ctx.sync_keyboard_terminal_owner();
    let filter_events = ctx.input_pending_p_filters_events();
    ctx.service_input_pending_without_timers()?;

    if input_pending_now(ctx, filter_events) {
        return Ok(Value::T);
    }

    if args.first().is_some_and(|v| v.is_truthy()) {
        // GNU `input-pending-p' can run due timers here, but it does not
        // force a redisplay the way `detect_input_pending_run_timers' does.
        ctx.service_input_pending_with_timers()?;
    }

    Ok(Value::bool_val(input_pending_now(ctx, filter_events)))
}

// ---------------------------------------------------------------------------
// 11. discard-input
// ---------------------------------------------------------------------------

/// `(discard-input)`
///
/// Discard pending unread command events for the current scope.
pub(crate) fn builtin_discard_input(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("discard-input", &args, 0)?;
    super::eval::set_runtime_binding(
        &mut ctx.obarray,
        &mut ctx.buffers,
        &ctx.custom,
        ctx.specpdl.as_slice(),
        intern("unread-command-events"),
        Value::NIL,
    );
    Ok(Value::NIL)
}

// ---------------------------------------------------------------------------
// 11b. insert-special-event
// ---------------------------------------------------------------------------

/// `(insert-special-event EVENT)` -> nil
///
/// Insert EVENT into the low-level special-event queue, so that the next
/// key-reading operation handles it through `special-event-map` instead of
/// returning it as ordinary user input.
///
/// Mirrors GNU `Finsert_special_event` at
/// `src/keyboard.c:12060`:
///
///   DEFUN ("insert-special-event", Finsert_special_event, ...)
///     (Lisp_Object event)
///   {
///     CHECK_CONS (event);
///     if (NILP (access_keymap (... Vspecial_event_map ..., event, ...)))
///       signal_error ("Invalid event kind", XCAR (event));
///     kbd_buffer_store_event (&ie);
///     return Qnil;
///   }
///
/// GNU pushes into the kernel kbd_buffer (which is a ring of
/// `struct input_event` records) so the event is delivered via the
/// same special-event path as hardware input. Neomacs keeps this queue in the
/// keyboard runtime (`unread_events`), not in `unread-command-events`: callers
/// like file notification rely on `read-event` consuming the event internally
/// and running the `special-event-map` handler.
///
/// Keyboard audit Finding 16 in
/// `drafts/keyboard-command-loop-audit.md`.
pub(crate) fn builtin_insert_special_event(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("insert-special-event", &args, 1)?;
    let event = args[0];
    if !event.is_cons() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("consp"), event],
        ));
    }
    if ctx.special_event_binding(&event).is_none() {
        return Err(signal(
            "error",
            vec![Value::string("Invalid event kind"), event.cons_car()],
        ));
    }
    ctx.queue_special_event(event);
    Ok(Value::NIL)
}

// ---------------------------------------------------------------------------
// 12. current-input-mode / set-input-mode
// ---------------------------------------------------------------------------

/// `(current-input-mode)` -> `(INTERRUPT FLOW META QUIT)`
pub(crate) fn builtin_current_input_mode(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("current-input-mode", &args, 0)?;
    let (interrupt, flow, meta, quit) = ctx.current_input_mode_tuple();
    Ok(Value::list(vec![
        Value::bool_val(interrupt),
        Value::bool_val(flow),
        Value::bool_val(meta),
        Value::fixnum(quit),
    ]))
}

/// `(set-input-mode INTERRUPT FLOW META QUIT)`
///
/// Batch-compatible behavior currently tracks INTERRUPT plus Lisp-visible
/// QUIT while leaving FLOW/META fixed.
pub(crate) fn builtin_set_input_mode(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("set-input-mode", &args, 3)?;
    expect_max_args("set-input-mode", &args, 4)?;
    eval.set_input_mode_interrupt(args[0].is_truthy());
    if let Some(quit) = args.get(3).copied()
        && !quit.is_nil()
    {
        set_quit_char_in_context(eval, quit)?;
    }
    Ok(Value::NIL)
}

// ---------------------------------------------------------------------------
// 13. input mode helper setters
// ---------------------------------------------------------------------------

/// `(set-input-interrupt-mode INTERRUPT)`
pub(crate) fn builtin_set_input_interrupt_mode(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-input-interrupt-mode", &args, 1)?;
    eval.set_input_mode_interrupt(args[0].is_truthy());
    Ok(Value::NIL)
}

fn peek_unread_command_event_in_state(
    obarray: &Obarray,
    dynamic: &[OrderedRuntimeBindingMap],
) -> Option<Value> {
    let name_id = intern("unread-command-events");
    let unread = dynamic
        .iter()
        .rev()
        .find_map(|frame| frame.get(&name_id).copied())
        .or_else(|| obarray.symbol_value("unread-command-events").copied());
    match unread {
        Some(v) if v.is_cons() => Some(v.cons_car()),
        _ => None,
    }
}

pub(crate) fn builtin_read_char_in_runtime(
    runtime: &mut impl KeyboardInputRuntime,
    args: &[Value],
) -> Result<Option<Value>, Flow> {
    if args.len() > 3 {
        return Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol("read-char"), Value::fixnum(args.len() as i64)],
        ));
    }
    expect_optional_prompt_string(args)?;
    let seconds_is_nil_or_omitted = args.get(2).is_none_or(|v| v.is_nil());

    if let Some(event) = runtime.peek_unread_command_event() {
        if let Some(n) = event_to_int(&event) {
            let event = runtime
                .pop_unread_command_event()
                .expect("peeked unread event should still be present");
            if runtime.read_command_keys().is_empty() && seconds_is_nil_or_omitted {
                runtime.set_read_command_keys(vec![event]);
            }
            return Ok(Some(Value::fixnum(n)));
        }
        runtime.replace_unread_command_event_with_singleton(event);
        runtime.record_input_event(event);
        return Err(non_character_input_event_error());
    }

    match runtime.command_event_input_source() {
        CommandEventInputSource::Runtime => Ok(None),
        CommandEventInputSource::Unavailable => Ok(Some(Value::NIL)),
    }
}

pub(crate) fn builtin_read_key_sequence_in_runtime(
    runtime: &mut impl KeyboardInputRuntime,
    args: &[Value],
) -> Result<Option<Value>, Flow> {
    expect_min_args("read-key-sequence", args, 1)?;
    expect_max_args("read-key-sequence", args, 6)?;
    expect_optional_prompt_string(args)?;

    if runtime.peek_unread_command_event().is_some() {
        let (keys, _binding) =
            runtime.read_key_sequence_blocking(read_key_sequence_options_from_args(args))?;
        return Ok(Some(read_key_sequence_string_result(&keys)));
    }

    Ok(None)
}

pub(crate) fn builtin_read_key_sequence_vector_in_runtime(
    runtime: &mut impl KeyboardInputRuntime,
    args: &[Value],
) -> Result<Option<Value>, Flow> {
    expect_min_args("read-key-sequence-vector", args, 1)?;
    expect_max_args("read-key-sequence-vector", args, 6)?;
    expect_optional_prompt_string(args)?;

    if runtime.peek_unread_command_event().is_some() {
        let (keys, _binding) =
            runtime.read_key_sequence_blocking(read_key_sequence_options_from_args(args))?;
        return Ok(Some(read_key_sequence_vector_result(&keys)));
    }

    Ok(None)
}

/// `(set-input-meta-mode META)`
///
/// Batch-compatible behavior: accepts GNU-compatible optional TERMINAL and returns nil.
pub(crate) fn builtin_set_input_meta_mode(args: Vec<Value>) -> EvalResult {
    expect_min_args("set-input-meta-mode", &args, 1)?;
    expect_max_args("set-input-meta-mode", &args, 2)?;
    Ok(Value::NIL)
}

/// `(set-output-flow-control FLOW)`
///
/// Batch-compatible behavior: accepts one argument and returns nil.
pub(crate) fn builtin_set_output_flow_control(args: Vec<Value>) -> EvalResult {
    expect_min_args("set-output-flow-control", &args, 1)?;
    expect_max_args("set-output-flow-control", &args, 2)?;
    Ok(Value::NIL)
}

/// `(set-quit-char CHAR)`
///
fn set_quit_char_in_context(eval: &mut super::eval::Context, quit: Value) -> EvalResult {
    let Some(quit) = quit.as_fixnum() else {
        return Err(signal(
            "error",
            vec![Value::string("QUIT must be an ASCII character")],
        ));
    };
    if !(0..=0o400).contains(&quit) {
        return Err(signal(
            "error",
            vec![Value::string("QUIT must be an ASCII character")],
        ));
    }

    eval.set_quit_char(quit);
    Ok(Value::NIL)
}

/// GNU-compatible quit-char setter for the current evaluator.
pub(crate) fn builtin_set_quit_char(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("set-quit-char", &args, 1)?;
    set_quit_char_in_context(eval, args[0])
}

// ---------------------------------------------------------------------------
// 14. waiting-for-user-input-p
// ---------------------------------------------------------------------------

/// `(waiting-for-user-input-p)`
///
/// Batch-mode compatibility: always returns nil.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_waiting_for_user_input_p(args: Vec<Value>) -> EvalResult {
    expect_args("waiting-for-user-input-p", &args, 0)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_waiting_for_user_input_p_ctx(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("waiting-for-user-input-p", &args, 0)?;
    Ok(Value::bool_val(eval.waiting_for_user_input()))
}

// ---------------------------------------------------------------------------
// 15. yes-or-no-p
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------

/// `(yes-or-no-p PROMPT)`
///
/// Ask user a yes-or-no question requiring "yes" or "no" typed in full.
/// In interactive mode, uses read-from-minibuffer.
/// In batch mode, signals end-of-file.
pub(crate) fn builtin_yes_or_no_p(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    validate_yes_or_no_p_args(&args)?;
    if let Some(result) = yes_or_no_p_dialog_result(eval, &args)? {
        return Ok(result);
    }
    if yes_or_no_p_use_short_answers(eval) {
        return eval.apply(Value::symbol("y-or-n-p"), args);
    }
    if let Some(result) = builtin_yes_or_no_p_in_runtime(eval, &args)? {
        return Ok(result);
    }
    finish_yes_or_no_p_in_eval(eval, &args)
}

pub(crate) fn finish_yes_or_no_p_in_eval(
    eval: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    finish_yes_or_no_p_with_minibuffer(args, |minibuffer_args| {
        finish_read_from_minibuffer_in_eval(eval, minibuffer_args)
    })
}

pub(crate) fn finish_yes_or_no_p_with_minibuffer(
    args: &[Value],
    mut read_from_minibuffer: impl FnMut(&[Value]) -> EvalResult,
) -> EvalResult {
    let prompt_ls = if args[0].is_string() {
        args[0].as_lisp_string().expect("checked string").clone()
    } else {
        crate::heap_types::LispString::from_unibyte(Vec::new())
    };
    // Build the prompt exactly like GNU `Fyes_or_no_p` (fns.c): append
    // `yes-or-no-prompt` ("(yes or no) "), preceded by a single space only when
    // the prompt does not already end in whitespace.
    let ends_in_blank = prompt_ls
        .as_bytes()
        .last()
        .is_some_and(|&b| b == b' ' || b == b'\t');
    let suffix: &[u8] = if ends_in_blank {
        b"(yes or no) "
    } else {
        b" (yes or no) "
    };
    let full_prompt = prompt_ls.concat(&crate::heap_types::LispString::from_unibyte(
        suffix.to_vec(),
    ));
    loop {
        let result = read_from_minibuffer(&[Value::heap_string(full_prompt.clone())])?;
        if result.is_string() {
            let answer = result.as_lisp_string().expect("checked string");
            // The valid answers are ASCII ("yes"/"no"); decode lossily to compare.
            match crate::emacs_core::emacs_char::to_utf8_lossy(answer.as_bytes()).trim() {
                "yes" => return Ok(Value::T),
                "no" => return Ok(Value::NIL),
                _ => continue,
            }
        }
    }
}

pub(crate) fn builtin_yes_or_no_p_in_runtime(
    runtime: &impl KeyboardInputRuntime,
    args: &[Value],
) -> Result<Option<Value>, Flow> {
    validate_yes_or_no_p_args(args)?;

    match runtime.minibuffer_input_source() {
        MinibufferInputSource::CommandLoop => Ok(None),
        MinibufferInputSource::StandardInput => {
            // Batch/noninteractive: GNU's `read_minibuf_noninteractive` (minibuf.c)
            // writes the prompt to stdout and reads the answer from stdin. Mirror
            // that — including the yes/no re-prompt loop — instead of failing before
            // the prompt is ever shown, so batch `yes-or-no-p` emits the prompt
            // exactly like GNU (and still signals end-of-file on empty stdin).
            finish_yes_or_no_p_with_minibuffer(args, |minibuffer_args| {
                let prompt = minibuffer_args[0]
                    .as_lisp_string()
                    .expect("yes-or-no-p minibuffer prompt is a string");
                read_from_stdin_noninteractive(&crate::emacs_core::emacs_char::to_utf8_lossy(
                    prompt.as_bytes(),
                ))
            })
            .map(Some)
        }
    }
}

fn validate_yes_or_no_p_args(args: &[Value]) -> Result<(), Flow> {
    expect_args("yes-or-no-p", args, 1)?;
    if !args[0].is_string() {
        return Err(signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("stringp"), args[0]],
        ));
    }
    Ok(())
}

fn yes_or_no_p_dialog_result(
    eval: &mut super::eval::Context,
    args: &[Value],
) -> Result<Option<Value>, Flow> {
    if !yes_or_no_p_should_use_dialog(eval) {
        return Ok(None);
    }
    let menu = Value::cons(
        args[0],
        Value::list(vec![
            Value::cons(Value::string("Yes"), Value::T),
            Value::cons(Value::string("No"), Value::NIL),
        ]),
    );
    super::display::builtin_x_popup_dialog(eval, vec![Value::T, menu, Value::NIL]).map(Some)
}

fn yes_or_no_p_use_short_answers(eval: &super::eval::Context) -> bool {
    eval.obarray
        .symbol_value("use-short-answers")
        .is_some_and(|v| v.is_truthy())
}

fn yes_or_no_p_should_use_dialog(runtime: &impl KeyboardInputRuntime) -> bool {
    let last_input_event = runtime.symbol_value_or_nil("last-input-event");
    if last_input_event.is_nil() || !runtime.symbol_value_or_nil("use-dialog-box").is_truthy() {
        return false;
    }

    let last_nonmenu_event = runtime.symbol_value_or_nil("last-nonmenu-event");
    let from_tty_menu = runtime.symbol_value_or_nil("from--tty-menu-p");
    last_nonmenu_event.is_cons()
        || (last_nonmenu_event.is_nil() && last_input_event.is_cons())
        || (from_tty_menu.is_truthy() && from_tty_menu.as_symbol_name() != Some("unbound"))
}

// ---------------------------------------------------------------------------
// 17. read-char
// ---------------------------------------------------------------------------

/// `(read-char &optional PROMPT INHERIT-INPUT-METHOD SECONDS)`
///
/// Read a character from the command input (keyboard or macro).
/// In batch mode, checks `unread-command-events` and returns nil if empty.
/// In interactive mode, blocks on the input channel via `read_char()`.
/// GNU `read_char` (keyboard.c) displays a non-nil string PROMPT in the echo
/// area for the duration of the read. Mirror that so prompts such as
/// `perform-replace`'s `(read-key "Query replacing ...: (? for help) ")` are
/// visible while waiting for the key. A nil/omitted or empty prompt shows
/// nothing (GNU only echoes a non-empty string prompt).
pub(crate) fn display_read_prompt(eval: &mut super::eval::Context, args: &[Value]) {
    if let Some(prompt) = args.first().and_then(|v| v.as_lisp_string())
        && !prompt.is_empty()
    {
        eval.set_current_message(Some(prompt.clone()));
    }
}

pub(crate) fn builtin_read_char(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    display_read_prompt(eval, &args);
    if let Some(value) = builtin_read_char_in_runtime(eval, &args)? {
        return Ok(value);
    }

    finish_read_char_in_eval(eval, &args)
}

pub(crate) fn finish_read_char_in_eval(
    eval: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    finish_read_char_interactive_in_runtime(eval, args)
}

pub(crate) fn finish_read_char_interactive_in_runtime(
    runtime: &mut impl KeyboardInputRuntime,
    args: &[Value],
) -> EvalResult {
    match runtime.command_event_input_source() {
        CommandEventInputSource::Runtime => {
            let timeout = parse_optional_read_seconds_arg(args.get(2))?;
            let Some(event) = runtime.read_char_with_timeout(timeout)? else {
                return Ok(Value::NIL);
            };
            let seconds_is_nil_or_omitted = args.get(2).is_none_or(|v| v.is_nil());
            if let Some(n) = event_to_int(&event) {
                if runtime.read_command_keys().is_empty() && seconds_is_nil_or_omitted {
                    runtime.set_read_command_keys(vec![event]);
                }
                return Ok(Value::fixnum(n));
            }
            runtime.replace_unread_command_event_with_singleton(event);
            runtime.record_input_event(event);
            Err(non_character_input_event_error())
        }
        CommandEventInputSource::Unavailable => Ok(Value::NIL),
    }
}

/// `(read-key &optional PROMPT)`
///
/// Read a key from the command input.
/// In batch mode, returns next `unread-command-events` event, else nil.
/// In interactive mode, blocks on the input channel via `read_char()`.
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn builtin_read_key(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    if args.len() > 2 {
        return Err(signal(
            LispCondition::WrongNumberOfArguments,
            vec![Value::symbol("read-key"), Value::fixnum(args.len() as i64)],
        ));
    }
    expect_optional_prompt_string(&args)?;

    // 1. Check unread-command-events first
    if let Some(event) = eval.pop_unread_command_event() {
        eval.record_nonmenu_input_event(event);
        eval.set_read_command_keys(vec![event]);
        if let Some(n) = event_to_int(&event) {
            return Ok(Value::fixnum(n));
        }
        return Ok(event);
    }

    // 2. Interactive mode: block on input channel
    if eval.input_rx.is_some() {
        let event = eval.read_char()?;
        eval.record_nonmenu_input_event(event);
        eval.set_read_command_keys(vec![event]);
        if let Some(n) = event_to_int(&event) {
            return Ok(Value::fixnum(n));
        }
        return Ok(event);
    }

    // 3. Batch mode: no input
    eval.clear_read_command_keys();
    Ok(Value::NIL)
}

// ---------------------------------------------------------------------------
// 18. read-key-sequence
// ---------------------------------------------------------------------------

/// `(read-key-sequence PROMPT &optional ...)`
///
/// Read a sequence of keystrokes that forms a complete key binding.
/// In batch mode, consumes one queued event. In interactive mode, uses the
/// evaluator's `read_key_sequence()` to accumulate keys through prefix keymaps.
pub(crate) fn builtin_read_key_sequence(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if let Some(value) = builtin_read_key_sequence_in_runtime(eval, &args)? {
        return Ok(value);
    }

    finish_read_key_sequence_in_eval(eval, &args)
}

pub(crate) fn finish_read_key_sequence_in_eval(
    eval: &mut super::eval::Context,
    args: &[Value],
) -> EvalResult {
    finish_read_key_sequence_interactive_in_runtime(eval, read_key_sequence_options_from_args(args))
}

pub(crate) fn finish_read_key_sequence_interactive_in_runtime(
    runtime: &mut impl KeyboardInputRuntime,
    options: crate::keyboard::ReadKeySequenceOptions,
) -> EvalResult {
    let (keys, _binding) = runtime.read_key_sequence_blocking(options)?;
    let mut chars_only = true;
    let mut s = String::new();
    for k in &keys {
        if let Some(c) = event_to_char(k) {
            s.push(c);
        } else {
            chars_only = false;
            break;
        }
    }
    if chars_only {
        return Ok(Value::string(s));
    }
    Ok(Value::vector(keys))
}

/// `(read-key-sequence-vector PROMPT)`
///
/// Batch mode: returns next `unread-command-events` event as a single-element
/// vector when present, otherwise an empty vector.
pub(crate) fn builtin_read_key_sequence_vector(
    eval: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    if let Some(value) = builtin_read_key_sequence_vector_in_runtime(eval, &args)? {
        return Ok(value);
    }
    finish_read_key_sequence_vector_interactive_in_runtime(
        eval,
        read_key_sequence_options_from_args(&args),
    )
}

pub(crate) fn finish_read_key_sequence_vector_interactive_in_runtime(
    runtime: &mut impl KeyboardInputRuntime,
    options: crate::keyboard::ReadKeySequenceOptions,
) -> EvalResult {
    let (keys, _binding) = runtime.read_key_sequence_blocking(options)?;
    Ok(Value::vector(keys))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "reader_minibuffer_teardown_test.rs"]
mod minibuffer_teardown_tests;
#[cfg(test)]
#[path = "reader_raw_bytes_test.rs"]
mod raw_bytes_tests;
#[cfg(test)]
#[path = "reader_test.rs"]
mod tests;
