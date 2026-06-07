use super::*;

// ===========================================================================
// Misc
// ===========================================================================

pub(crate) fn builtin_identity(args: Vec<Value>) -> EvalResult {
    expect_args("identity", &args, 1)?;
    Ok(args[0])
}

pub(crate) fn builtin_prefix_numeric_value(args: Vec<Value>) -> EvalResult {
    expect_args("prefix-numeric-value", &args, 1)?;
    let numeric = crate::emacs_core::prefix::prefix_numeric_value(&args[0]);
    Ok(Value::fixnum(numeric))
}

pub(crate) fn builtin_ignore(_args: Vec<Value>) -> EvalResult {
    Ok(Value::NIL)
}

/// Log a message to the *Messages* buffer, matching GNU Emacs message_dolog
/// in xdisp.c.  Creates the buffer if it doesn't exist.
fn message_dolog(ctx: &mut super::eval::Context, msg: &crate::heap_types::LispString) {
    // GNU: check message-log-max; if nil, don't log
    let log_max = ctx.visible_variable_value_or_nil("message-log-max");
    if log_max.is_nil() {
        return;
    }

    // GNU xdisp.c defaults `messages-buffer-name` to "*Messages*" and lets
    // Lisp rebind it to redirect message logging.
    let messages_name = ctx
        .visible_variable_value_or_nil("messages-buffer-name")
        .as_str_owned()
        .unwrap_or_else(|| "*Messages*".to_string());
    let buf_id = if let Some(id) = ctx.buffers.find_buffer_by_name(&messages_name) {
        id
    } else {
        ctx.buffers.create_buffer(&messages_name)
    };

    // Insert the message text at the end, followed by newline.
    // Save and restore current buffer like GNU does.
    let old_buf = ctx.buffers.current_buffer().map(|b| b.id);
    let _ = ctx.set_current_buffer_unrecorded(buf_id);
    let Some((old_pt_byte, old_accessible, old_full_end, point_at_end, zv_at_end)) =
        ctx.buffers.get(buf_id).map(|buf| {
            let old_pt_byte = buf.point_emacs_byte_pos();
            let old_accessible = buf.accessible_region_snapshot();
            let old_full_end = buf.full_emacs_byte_range().end();
            let point_at_end = old_pt_byte == old_full_end;
            let zv_at_end = old_accessible.end_emacs_byte() == old_full_end;
            (
                old_pt_byte,
                old_accessible,
                old_full_end,
                point_at_end,
                zv_at_end,
            )
        })
    else {
        if let Some(old) = old_buf {
            ctx.restore_current_buffer_if_live(old);
        }
        return;
    };
    if let Some(full_range) = ctx.buffers.full_buffer_emacs_byte_range(buf_id) {
        let _ = ctx
            .buffers
            .restore_buffer_emacs_byte_restriction(buf_id, full_range);
    }
    if ctx
        .buffers
        .goto_buffer_emacs_byte_pos(buf_id, old_full_end)
        .is_some()
    {
        let same_multibyte = ctx
            .buffers
            .get(buf_id)
            .map(|buf| buf.get_multibyte() == msg.is_multibyte())
            .unwrap_or(false);
        if same_multibyte {
            let _ = ctx.buffers.insert_lisp_string_into_buffer(buf_id, msg);
        } else {
            let text = super::runtime_string_from_lisp_string(msg);
            let _ = ctx.buffers.insert_into_buffer(buf_id, &text);
        }
        let _ = ctx.buffers.insert_into_buffer(buf_id, "\n");
    }

    if let Some(new_full_end) = ctx
        .buffers
        .get(buf_id)
        .map(|buf| buf.full_emacs_byte_range().end())
    {
        if zv_at_end {
            let _ = ctx
                .buffers
                .restore_buffer_accessible_region_with_current_full_end(buf_id, old_accessible);
        } else {
            let _ = ctx
                .buffers
                .restore_buffer_accessible_region(buf_id, old_accessible);
        }
        let restored_point = if point_at_end {
            new_full_end
        } else {
            old_pt_byte
        };
        let _ = ctx
            .buffers
            .goto_buffer_emacs_byte_pos(buf_id, restored_point);
    }
    if let Some(old) = old_buf {
        ctx.restore_current_buffer_if_live(old);
    }
}

fn message_echo_result(
    ctx: &mut super::eval::Context,
    msg: &crate::heap_types::LispString,
) -> Result<Option<crate::heap_types::LispString>, crate::emacs_core::error::Flow> {
    if ctx
        .visible_variable_value_or_nil("inhibit-message")
        .is_truthy()
    {
        return Ok(None);
    }

    let set_message_function = ctx.visible_variable_value_or_nil("set-message-function");
    if set_message_function.is_nil()
        || ctx.gc_inhibit_depth > 0
        || builtin_functionp_1(ctx, set_message_function)?.is_nil()
    {
        return Ok(Some(msg.clone()));
    }

    // GNU xdisp.c `set_message` calls `set-message-function` through
    // `dsafe_call1`: redisplay is inhibited, quit is inhibited, and hook
    // errors are demoted instead of escaping from `message`.
    let specpdl_count = ctx.specpdl.len();
    ctx.specbind(intern("inhibit-redisplay"), Value::T);
    ctx.specbind(intern("inhibit-quit"), Value::T);
    let result = ctx.funcall_general(set_message_function, vec![Value::heap_string(msg.clone())]);
    ctx.unbind_to(specpdl_count);

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            tracing::warn!(
                "set-message-function signaled while setting echo message: {:?}",
                err
            );
            return Ok(Some(msg.clone()));
        }
    };
    if result.is_nil() {
        return Ok(Some(msg.clone()));
    }
    if let Some(string) = result.as_lisp_string() {
        return Ok(Some(string.clone()));
    }
    Ok(None)
}

pub(crate) fn builtin_message(ctx: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("message", &args, 1)?;
    // GNU Emacs: nil or empty string clears the echo area and returns as-is.
    if args[0].is_nil() {
        ctx.clear_current_message();
        return Ok(Value::NIL);
    }
    if args[0].is_string() {
        if args[0]
            .as_lisp_string()
            .expect("string")
            .as_bytes()
            .is_empty()
        {
            ctx.clear_current_message();
            return Ok(args[0]);
        }
    }
    // GNU Emacs's `message` ALWAYS calls `format-message` on the args,
    // even for a single string argument.  This converts %% -> % and
    // applies text-quoting (curly quotes).
    let formatted = super::strings::builtin_format_message(ctx, args.clone())?;
    // GNU Fmessage returns the formatted Lisp object after display/logging
    // side effects. Keep that object rooted while those side effects allocate.
    let root_scope = ctx.save_vm_roots();
    ctx.push_vm_frame_root(formatted);
    let msg = match formatted.as_lisp_string() {
        Some(string) => string.clone(),
        None => crate::heap_types::LispString::from_emacs_bytes(Vec::new()),
    };
    let side_effects = (|| {
        let displayed_message = message_echo_result(ctx, &msg)?;
        match &displayed_message {
            Some(displayed) => ctx.set_current_message(Some(displayed.clone())),
            None => ctx.clear_current_message(),
        }
        // GNU Emacs message_dolog: log to *Messages* buffer
        message_dolog(ctx, &msg);
        if displayed_message.is_some() && !ctx.noninteractive() {
            ctx.ensure_echo_area_buffers();
        }
        tracing::info!(msg = %super::runtime_string_from_lisp_string(&msg));
        // GNU Emacs editfns.c: in batch mode, message prints to stderr with newline.
        if ctx.noninteractive() {
            use std::io::Write;
            let text = super::runtime_string_from_lisp_string(&msg);
            let _ = std::io::stderr().write_all(text.as_bytes());
            let _ = std::io::stderr().write_all(b"\n");
            let _ = std::io::stderr().flush();
        }
        Ok(())
    })();
    ctx.restore_vm_roots(root_scope);
    side_effects?;
    Ok(formatted)
}

pub(crate) fn builtin_message_box(ctx: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("message-box", &args, 1)?;
    if args[0].is_nil() {
        return Ok(Value::NIL);
    }
    // GNU Emacs: always calls format-message, even for single-arg.
    let formatted = super::strings::builtin_format_message(ctx, args.clone())?;
    let msg = match formatted.kind() {
        ValueKind::String => formatted
            .as_runtime_string_owned()
            .expect("ValueKind::String must carry LispString payload"),
        _ => String::new(),
    };
    tracing::info!(msg = %msg);
    Ok(formatted)
}

pub(crate) fn builtin_message_or_box(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("message-or-box", &args, 1)?;
    if args[0].is_nil() {
        return Ok(Value::NIL);
    }
    // GNU Emacs: always calls format-message, even for single-arg.
    let formatted = super::strings::builtin_format_message(ctx, args.clone())?;
    let msg = match formatted.kind() {
        ValueKind::String => formatted
            .as_runtime_string_owned()
            .expect("ValueKind::String must carry LispString payload"),
        _ => String::new(),
    };
    tracing::info!(msg = %msg);
    Ok(formatted)
}

pub(crate) fn builtin_current_message(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("current-message", &args, 0)?;
    Ok(ctx.current_message_value().unwrap_or(Value::NIL))
}

pub(crate) fn builtin_daemonp(args: Vec<Value>) -> EvalResult {
    expect_args("daemonp", &args, 0)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_daemon_initialized(args: Vec<Value>) -> EvalResult {
    expect_args("daemon-initialized", &args, 0)?;
    Err(signal(
        "error",
        vec![Value::string(
            "This function can only be called if emacs is run as a daemon",
        )],
    ))
}

pub(crate) fn builtin_documentation_stringp(args: Vec<Value>) -> EvalResult {
    expect_args("documentation-stringp", &args, 1)?;
    let is_compiled_ref = match args[0].kind() {
        ValueKind::Cons => {
            let pair_car = args[0].cons_car();
            let pair_cdr = args[0].cons_cdr();
            pair_car.is_string() && pair_cdr.as_int().is_some()
        }
        _ => false,
    };
    Ok(Value::bool_val(
        (args[0].is_string() || args[0].is_fixnum()) || is_compiled_ref,
    ))
}

pub(crate) fn builtin_flush_standard_output(args: Vec<Value>) -> EvalResult {
    expect_args("flush-standard-output", &args, 0)?;
    Ok(Value::NIL)
}

pub(crate) fn builtin_force_mode_line_update(
    ctx: &mut super::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_max_args("force-mode-line-update", &args, 1)?;
    ctx.invalidate_redisplay();
    Ok(args.first().cloned().unwrap_or(Value::NIL))
}

pub(crate) fn builtin_get_internal_run_time(args: Vec<Value>) -> EvalResult {
    expect_args("get-internal-run-time", &args, 0)?;
    use crate::emacs_core::value::ValueKind;
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs() as i64;
    let usecs = dur.subsec_micros() as i64;
    Ok(Value::list(vec![
        Value::fixnum(secs >> 16),
        Value::fixnum(secs & 0xFFFF),
        Value::fixnum(usecs),
        Value::fixnum(0),
    ]))
}

pub(crate) fn builtin_invocation_directory(args: Vec<Value>) -> EvalResult {
    expect_args("invocation-directory", &args, 0)?;
    let mut dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|parent| parent.to_path_buf()))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/".to_string());
    if !dir.ends_with('/') {
        dir.push('/');
    }
    Ok(Value::string(dir))
}

pub(crate) fn builtin_invocation_name(args: Vec<Value>) -> EvalResult {
    expect_args("invocation-name", &args, 0)?;
    let name = std::env::current_exe()
        .ok()
        .and_then(|p| {
            p.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "emacs".to_string());
    Ok(Value::string(name))
}

pub(crate) fn builtin_error(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("error", &args, 1)?;
    let formatted = builtin_format_message(eval, args)?;
    Err(signal(
        "error",
        vec![if formatted.is_string() {
            formatted
        } else {
            Value::string("error")
        }],
    ))
}

pub(crate) fn builtin_user_error(eval: &mut super::eval::Context, args: Vec<Value>) -> EvalResult {
    expect_min_args("user-error", &args, 1)?;
    let formatted = builtin_format_message(eval, args)?;
    Err(signal(
        "user-error",
        vec![if formatted.is_string() {
            formatted
        } else {
            Value::string("user-error")
        }],
    ))
}

pub(crate) fn builtin_secure_hash_algorithms(args: Vec<Value>) -> EvalResult {
    expect_args("secure-hash-algorithms", &args, 0)?;
    Ok(Value::list(vec![
        Value::symbol("md5"),
        Value::symbol("sha1"),
        Value::symbol("sha224"),
        Value::symbol("sha256"),
        Value::symbol("sha384"),
        Value::symbol("sha512"),
    ]))
}

pub(crate) fn builtin_symbol_name(args: Vec<Value>) -> EvalResult {
    expect_args("symbol-name", &args, 1)?;
    builtin_symbol_name_value(args[0], false)
}

pub(crate) fn builtin_symbol_name_1(eval: &mut super::eval::Context, symbol: Value) -> EvalResult {
    builtin_symbol_name_value(symbol, eval.symbols_with_pos_enabled)
}

fn builtin_symbol_name_value(symbol: Value, symbols_with_pos_enabled: bool) -> EvalResult {
    match super::symbols::symbol_id_checked(&symbol, symbols_with_pos_enabled) {
        Some(id) => Ok(
            crate::emacs_core::intern::resolve_sym_name_value(id).unwrap_or_else(|| {
                Value::heap_string(crate::emacs_core::intern::resolve_sym_lisp_string(id).clone())
            }),
        ),
        None => Err(signal(
            "wrong-type-argument",
            vec![Value::symbol("symbolp"), symbol],
        )),
    }
}

pub(crate) fn builtin_make_symbol(args: Vec<Value>) -> EvalResult {
    expect_args("make-symbol", &args, 1)?;
    make_symbol_value(args[0])
}

pub(crate) fn builtin_make_symbol_1(_eval: &mut super::eval::Context, arg: Value) -> EvalResult {
    make_symbol_value(arg)
}

fn make_symbol_value(arg: Value) -> EvalResult {
    expect_lisp_string(&arg)?;
    Ok(Value::from_sym_id(
        crate::emacs_core::intern::make_uninterned_symbol_with_name_value(arg),
    ))
}
