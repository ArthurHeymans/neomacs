//! GNU-compatible Lisp adapters for native file-notification backends.
//!
//! Parsing and platform-specific descriptor encoding live here; native
//! adapters receive typed requests and never inspect arbitrary Lisp values.

use super::*;
use crate::emacs_core::error::expect_args;

pub(crate) fn file_notify_error(
    message: &str,
    detail: Option<String>,
    object: Option<Value>,
) -> Flow {
    let mut tail = match object {
        Some(object) if object.is_cons() => object,
        Some(object) if !object.is_nil() => Value::list(vec![object]),
        _ => Value::NIL,
    };
    if let Some(detail) = detail {
        tail = Value::cons(Value::string(&detail), tail);
    }
    let raw_data = Value::cons(Value::string(message), tail);
    crate::emacs_core::error::signal_with_data("file-notify-error", raw_data)
}

#[cfg(target_os = "linux")]
fn inotify_unknown_aspect_error(aspect: Value) -> Flow {
    file_notify_error(
        "Unknown aspect",
        Some("Invalid argument".to_string()),
        Some(aspect),
    )
}

#[cfg(target_os = "linux")]
fn inotify_invalid_descriptor_error(descriptor: Value, detail: &str) -> Flow {
    file_notify_error(
        "Invalid descriptor ",
        Some(detail.to_string()),
        Some(descriptor),
    )
}

#[cfg(target_os = "linux")]
fn inotify_aspect_symbol_valid(name: &str) -> bool {
    matches!(
        name,
        "access"
            | "attrib"
            | "close-write"
            | "close-nowrite"
            | "create"
            | "delete"
            | "delete-self"
            | "modify"
            | "move-self"
            | "moved-from"
            | "moved-to"
            | "open"
            | "move"
            | "close"
            | "dont-follow"
            | "onlydir"
            | "ignored"
            | "unmount"
            | "all-events"
            | "t"
    )
}

#[cfg(target_os = "linux")]
fn parse_inotify_aspects(aspect: Value) -> Result<Vec<String>, Flow> {
    if aspect.is_nil() {
        return Ok(Vec::new());
    }
    if let Some(name) = aspect.as_symbol_name() {
        return if inotify_aspect_symbol_valid(name) {
            Ok(vec![name.to_owned()])
        } else {
            Err(inotify_unknown_aspect_error(aspect))
        };
    }
    if !aspect.is_cons() {
        return Err(inotify_unknown_aspect_error(aspect));
    }

    let mut names = Vec::new();
    let mut rest = aspect;
    while rest.is_cons() {
        let item = rest.cons_car();
        let Some(name) = item.as_symbol_name() else {
            return Err(inotify_unknown_aspect_error(item));
        };
        if !inotify_aspect_symbol_valid(name) {
            return Err(inotify_unknown_aspect_error(item));
        }
        names.push(name.to_owned());
        rest = rest.cons_cdr();
    }
    if !rest.is_nil() {
        return Err(inotify_unknown_aspect_error(rest));
    }
    Ok(names)
}

#[cfg(target_os = "linux")]
fn extract_inotify_watch_id(value: Value) -> Option<WatchId> {
    if !value.is_cons() {
        return None;
    }
    let slot = value.cons_car().as_int()?;
    let generation = value.cons_cdr().as_int()?;
    (slot >= 0 && generation >= 0).then(|| WatchId::new(slot, generation))
}

#[cfg(target_os = "linux")]
pub(crate) fn inotify_valid_p(args: Vec<Value>) -> EvalResult {
    expect_args("inotify-valid-p", &args, 1)?;
    let Some(watch_id) = extract_inotify_watch_id(args[0]) else {
        return Ok(Value::NIL);
    };
    FILE_NOTIFY_STATE.with(|slot| {
        let state = slot.borrow();
        Ok(Value::bool_val(state.backend.valid_p(&watch_id)))
    })
}

#[cfg(target_os = "linux")]
pub(crate) fn inotify_add_watch(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("inotify-add-watch", &args, 3)?;
    let path =
        crate::emacs_core::fileio::lisp_file_name_to_path_buf(ctx.expect_lisp_string(args[0])?);
    let aspects = parse_inotify_aspects(args[1])?;
    let callback = args[2];
    let notifier = ctx.wait_notifier();

    FILE_NOTIFY_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        let watch_id = state
            .backend
            .add_watch(&path, platform::Request::new(aspects), notifier)?;
        state.registry.register(watch_id.clone(), callback);
        Ok(watch_id.to_inotify_lisp())
    })
}

#[cfg(target_os = "linux")]
pub(crate) fn inotify_rm_watch(args: Vec<Value>) -> EvalResult {
    expect_args("inotify-rm-watch", &args, 1)?;
    let detail = if args[0].is_cons() {
        "Invalid argument"
    } else {
        "No such file or directory"
    };
    let Some(watch_id) = extract_inotify_watch_id(args[0]) else {
        return Err(inotify_invalid_descriptor_error(args[0], detail));
    };

    FILE_NOTIFY_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        if state.backend.remove_watch(&watch_id)? {
            state.registry.unregister(&watch_id);
        }
        Ok(Value::T)
    })
}

/// GNU's w32 backend exposes an opaque pointer integer.  Neomacs keeps the
/// platform handle private and uses this fixnum only as a compatibility token.
#[cfg(target_os = "windows")]
fn extract_w32_watch_id(value: Value) -> Option<WatchId> {
    let slot = value.as_fixnum()?;
    (slot >= 0).then(|| WatchId::new(slot, 0))
}

#[cfg(target_os = "windows")]
pub(crate) fn w32notify_add_watch(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("w32notify-add-watch", &args, 3)?;
    let filters = list_to_vec(&args[1]).ok_or_else(|| {
        crate::emacs_core::error::signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("listp"), args[1]],
        )
    })?;
    ctx.expect_lisp_string(args[0])?;
    let expanded =
        crate::emacs_core::fileio::builtin_expand_file_name(ctx, vec![args[0], Value::NIL])?;
    let normalized = crate::emacs_core::fileio::builtin_directory_file_name(ctx, vec![expanded])?;
    let path =
        crate::emacs_core::fileio::lisp_file_name_to_path_buf(ctx.expect_lisp_string(normalized)?);
    let filters = filters
        .iter()
        .filter_map(|filter| filter.as_symbol_name())
        .filter_map(platform::windows::W32Filter::from_lisp_name)
        .fold(enumflags2::BitFlags::empty(), |filters, filter| {
            filters | filter
        });
    let callback = args[2];
    let notifier = ctx.wait_notifier();

    FILE_NOTIFY_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        let watch_id = state
            .backend
            .add_watch(&path, platform::Request::new(filters), notifier)?;
        state.registry.register(watch_id.clone(), callback);
        Ok(Value::fixnum(watch_id.slot()))
    })
}

#[cfg(target_os = "windows")]
pub(crate) fn w32notify_rm_watch(args: Vec<Value>) -> EvalResult {
    expect_args("w32notify-rm-watch", &args, 1)?;
    let invalid = || file_notify_error("Invalid watch descriptor", None, Some(args[0]));
    let Some(watch_id) = extract_w32_watch_id(args[0]) else {
        return Err(invalid());
    };
    FILE_NOTIFY_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        if state.backend.remove_watch(&watch_id)? {
            state.registry.unregister(&watch_id);
            Ok(Value::NIL)
        } else {
            Err(invalid())
        }
    })
}

#[cfg(target_os = "windows")]
pub(crate) fn w32notify_valid_p(args: Vec<Value>) -> EvalResult {
    expect_args("w32notify-valid-p", &args, 1)?;
    let Some(watch_id) = extract_w32_watch_id(args[0]) else {
        return Ok(Value::NIL);
    };
    FILE_NOTIFY_STATE.with(|slot| Ok(Value::bool_val(slot.borrow().backend.valid_p(&watch_id))))
}

/// GNU kqueue descriptors are bare fixnums -- the open fd -- unlike inotify's
/// conses.  The generation remains internal because the Lisp representation
/// has no generation component.
#[cfg(target_os = "macos")]
fn extract_kqueue_watch_id(value: Value) -> Option<WatchId> {
    let slot = value.as_fixnum()?;
    (slot >= 0).then(|| WatchId::new(slot, 0))
}

/// GNU `Fkqueue_add_watch` (`src/kqueue.c`) validates and normalizes FILE,
/// silently ignores unknown flag symbols, and returns the owned vnode fd.
#[cfg(target_os = "macos")]
pub(crate) fn kqueue_add_watch(
    ctx: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("kqueue-add-watch", &args, 3)?;
    ctx.expect_lisp_string(args[0])?;
    let expanded =
        crate::emacs_core::fileio::builtin_expand_file_name(ctx, vec![args[0], Value::NIL])?;
    let normalized = crate::emacs_core::fileio::builtin_directory_file_name(ctx, vec![expanded])?;
    if crate::emacs_core::fileio::builtin_file_exists_p(ctx, vec![normalized])?.is_nil() {
        return Err(crate::emacs_core::error::signal(
            "file-missing",
            vec![
                Value::string("File does not exist"),
                Value::string("No such file or directory"),
                normalized,
            ],
        ));
    }
    let path =
        crate::emacs_core::fileio::lisp_file_name_to_path_buf(ctx.expect_lisp_string(normalized)?);
    let flags = list_to_vec(&args[1]).ok_or_else(|| {
        crate::emacs_core::error::signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("listp"), args[1]],
        )
    })?;
    if !crate::emacs_core::builtins::value_is_function(ctx, args[2]) {
        return Err(crate::emacs_core::error::signal(
            LispCondition::WrongTypeArgument,
            vec![Value::symbol("invalid-function"), args[2]],
        ));
    }
    let actions = flags
        .iter()
        .filter_map(|flag| flag.as_symbol_name())
        .filter_map(platform::macos::action_from_lisp_name)
        .fold(enumflags2::BitFlags::empty(), |actions, action| {
            actions | action
        });
    let callback = args[2];
    let notifier = ctx.wait_notifier();

    FILE_NOTIFY_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        let watch_id = state
            .backend
            .add_watch(&path, platform::Request::new(actions), notifier)?;
        state.registry.register(watch_id.clone(), callback);
        Ok(Value::fixnum(watch_id.slot()))
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn kqueue_rm_watch(args: Vec<Value>) -> EvalResult {
    expect_args("kqueue-rm-watch", &args, 1)?;
    let not_a_watch_descriptor =
        || file_notify_error("Not a watch descriptor", None, Some(args[0]));
    let Some(watch_id) = extract_kqueue_watch_id(args[0]) else {
        return Err(not_a_watch_descriptor());
    };
    FILE_NOTIFY_STATE.with(|slot| {
        let mut state = slot.borrow_mut();
        if state.backend.remove_watch(&watch_id)? {
            state.registry.unregister(&watch_id);
            Ok(Value::T)
        } else {
            Err(not_a_watch_descriptor())
        }
    })
}

#[cfg(target_os = "macos")]
pub(crate) fn kqueue_valid_p(args: Vec<Value>) -> EvalResult {
    expect_args("kqueue-valid-p", &args, 1)?;
    let Some(watch_id) = extract_kqueue_watch_id(args[0]) else {
        return Ok(Value::NIL);
    };
    FILE_NOTIFY_STATE.with(|slot| {
        let state = slot.borrow();
        Ok(Value::bool_val(state.backend.valid_p(&watch_id)))
    })
}
