use super::{EvalError, Flow, PrintShorthandSymbol, format_flow_with_eval, quote_payload, signal};
use crate::emacs_core::error::LispCondition;
use crate::emacs_core::{Context, Value, print_value_bytes_with_eval, print_value_with_eval};

#[test]
fn list_prints_buffers_with_names_in_eval_context() -> Result<(), EvalError> {
    crate::test_utils::init_test_tracing();
    let mut eval = Context::new();
    let stale = Value::make_buffer(eval.buffers.create_buffer("stale-win-buf"));
    eval.set_variable("vm-stale-win-buf", stale);
    let value = eval.eval_str(
        "(let ((b vm-stale-win-buf)
           (w (selected-window)))
  (set-window-buffer nil b)
  (kill-buffer b)
  (list (window-buffer) (window-start) (window-point)))",
    )?;

    assert_eq!(
        print_value_with_eval(&eval, &value),
        "(#<buffer *scratch*> 1 1)"
    );
    assert_eq!(
        String::from_utf8(print_value_bytes_with_eval(&eval, &value)).unwrap(),
        "(#<buffer *scratch*> 1 1)"
    );

    Ok(())
}

#[test]
fn eval_context_printer_renders_killed_buffer_handles() -> Result<(), EvalError> {
    crate::test_utils::init_test_tracing();
    let mut eval = crate::test_utils::runtime_startup_context();
    let value = eval.eval_str(
        "(with-temp-buffer
           (condition-case err
               (key-binding 1 nil nil 0)
             (error err)))",
    )?;

    assert_eq!(
        print_value_with_eval(&eval, &value),
        "(args-out-of-range #<killed buffer> 0)"
    );
    assert_eq!(
        String::from_utf8(print_value_bytes_with_eval(&eval, &value)).unwrap(),
        "(args-out-of-range #<killed buffer> 0)"
    );

    Ok(())
}

#[test]
fn diagnostic_flow_formatter_renders_signal_strings() {
    crate::test_utils::init_test_tracing();
    let eval = Context::new();
    let flow = signal(
        LispCondition::FileMissing,
        vec![
            Value::string("Cannot open load file"),
            Value::string("No such file or directory"),
            Value::string("popweb"),
        ],
    );

    assert_eq!(
        format_flow_with_eval(&eval, &flow),
        r#"(file-missing ("Cannot open load file" "No such file or directory" "popweb"))"#
    );
}

#[test]
fn eval_context_printer_renders_mutex_handles_consistently() -> Result<(), EvalError> {
    crate::test_utils::init_test_tracing();
    let mut eval = Context::new();
    let value = eval.eval_str(r#"(make-mutex "error-printer-mutex")"#)?;
    let printed = print_value_with_eval(&eval, &value);

    assert!(printed.starts_with("#<mutex "));
    assert_eq!(
        String::from_utf8(print_value_bytes_with_eval(&eval, &value)).unwrap(),
        printed
    );

    Ok(())
}

#[test]
fn eval_context_printer_renders_condvar_handles_consistently() -> Result<(), EvalError> {
    crate::test_utils::init_test_tracing();
    let mut eval = Context::new();
    let value = eval.eval_str(
        r#"(let ((m (make-mutex "error-printer-mutex")))
           (make-condition-variable m "error-printer-condvar"))"#,
    )?;
    let printed = print_value_with_eval(&eval, &value);

    assert!(printed.starts_with("#<condvar "));
    assert_eq!(
        String::from_utf8(print_value_bytes_with_eval(&eval, &value)).unwrap(),
        printed
    );

    Ok(())
}

#[test]
fn eval_context_printer_renders_frame_window_handles_consistently() -> Result<(), EvalError> {
    crate::test_utils::init_test_tracing();
    let mut eval = Context::new();
    let value = eval.eval_str("(list (selected-frame) (selected-window))")?;
    let printed = print_value_with_eval(&eval, &value);

    assert!(printed.starts_with("(#<frame"));
    assert!(printed.contains("#<window"));
    assert_eq!(
        String::from_utf8(print_value_bytes_with_eval(&eval, &value)).unwrap(),
        printed
    );

    Ok(())
}

#[test]
fn eval_context_printer_renders_window_handles_with_buffer_names() -> Result<(), EvalError> {
    crate::test_utils::init_test_tracing();
    let mut eval = Context::new();
    let value = eval.eval_str(
        "(list (selected-window)
               (condition-case err (frame-terminal (selected-window)) (error err))
               (condition-case err (tty-type (selected-window)) (error err))
               (condition-case err (terminal-name (selected-window)) (error err)))",
    )?;
    let printed = print_value_with_eval(&eval, &value);

    assert!(printed.contains("on *scratch*>"));
    assert_eq!(
        String::from_utf8(print_value_bytes_with_eval(&eval, &value)).unwrap(),
        printed
    );

    Ok(())
}

#[test]
fn eval_context_printer_renders_terminal_thread_handles_consistently() -> Result<(), EvalError> {
    crate::test_utils::init_test_tracing();
    let mut eval = Context::new();
    let value = eval.eval_str("(list (car (terminal-list)) (current-thread))")?;
    let printed = print_value_with_eval(&eval, &value);

    assert!(printed.starts_with("(#<terminal"));
    assert!(printed.contains("#<thread"));
    assert_eq!(
        String::from_utf8(print_value_bytes_with_eval(&eval, &value)).unwrap(),
        printed
    );

    Ok(())
}

#[test]
fn print_shorthand_symbol_domain_matches_gnu_printer_symbols() {
    crate::test_utils::init_test_tracing();
    for (symbol, name) in [
        (PrintShorthandSymbol::Quote, "quote"),
        (PrintShorthandSymbol::Function, "function"),
        (PrintShorthandSymbol::Backquote, "`"),
        (PrintShorthandSymbol::Comma, ","),
        (PrintShorthandSymbol::CommaAt, ",@"),
    ] {
        let value = Value::symbol(name);
        assert_eq!(symbol.name(), name);
        assert_eq!(PrintShorthandSymbol::from_lisp_value(&value), Some(symbol));
    }

    let quoted = Value::list(vec![Value::symbol("quote"), Value::symbol("foo")]);
    let function_quoted = Value::list(vec![Value::symbol("function"), Value::symbol("foo")]);
    assert_eq!(quote_payload(&quoted), Some(Value::symbol("foo")));
    assert_eq!(quote_payload(&function_quoted), None);
}

#[test]
fn eval_context_printer_matches_gnu_backquote_shorthand_rules() {
    crate::test_utils::init_test_tracing();
    // GNU verified via:
    //   (prin1-to-string (list '\` (list 'a (list '\, 'x))))
    //   => "`(a ,x)"
    // The reader-shorthand form is the canonical print of the
    // (` (a (, x))) form, *not* the verbatim escaped one.
    let eval = Context::new();
    let raw_unquote = Value::list(vec![Value::symbol(","), Value::symbol("x")]);
    let nested = Value::list(vec![
        Value::symbol("`"),
        Value::list(vec![Value::symbol("a"), raw_unquote]),
    ]);
    assert_eq!(print_value_with_eval(&eval, &nested), "`(a ,x)");
    assert_eq!(
        String::from_utf8(print_value_bytes_with_eval(&eval, &nested)).unwrap(),
        "`(a ,x)"
    );
}

#[test]
fn eval_context_printer_handles_default_circular_vector_backreference() {
    crate::test_utils::init_test_tracing();
    let eval = Context::new();
    let vector = Value::vector(vec![Value::NIL]);
    assert!(vector.set_vector_slot(0, vector));

    assert_eq!(print_value_with_eval(&eval, &vector), "[#0]");
    assert_eq!(print_value_bytes_with_eval(&eval, &vector), b"[#0]");
}

#[test]
fn eval_context_printer_handles_default_circular_cons_backreference() {
    crate::test_utils::init_test_tracing();
    let eval = Context::new();
    let cell = Value::cons(Value::NIL, Value::NIL);
    cell.set_cdr(cell);

    assert_eq!(print_value_with_eval(&eval, &cell), "(nil . #0)");
    assert_eq!(print_value_bytes_with_eval(&eval, &cell), b"(nil . #0)");
}

#[test]
fn minibuffer_quit_does_not_take_down_a_noninteractive_session() {
    crate::test_utils::init_test_tracing();
    // GNU's `command-error-default-function` guards its
    // stderr-then-kill-emacs branch with `!is_minibuffer_quit`
    // (keyboard.c:1064).  A plain `quit' still takes that branch.
    let mut eval = Context::new();
    eval.set_variable("noninteractive", Value::T);

    let minibuffer_quit = Value::list(vec![Value::symbol("minibuffer-quit")]);
    let reported = eval.command_error_default_report(minibuffer_quit, Value::string(""));
    assert!(
        reported.is_ok(),
        "aborting a minibuffer must not unwind the session: {reported:?}"
    );
    assert!(
        eval.shutdown_request().is_none(),
        "aborting a minibuffer must not request shutdown"
    );

    let plain_quit = Value::list(vec![Value::symbol("quit")]);
    let reported = eval.command_error_default_report(plain_quit, Value::string(""));
    assert!(
        matches!(reported, Err(Flow::Shutdown(_))),
        "a plain quit keeps GNU's stderr-then-exit behavior: {reported:?}"
    );
}

// ---------------------------------------------------------------------------
// In-flight signal payload rooting (DIVERGENCES.md 161)
// ---------------------------------------------------------------------------

/// A signal that is unwinding lives only in a Rust `Flow::Signal`. GNU gets
/// this for free — `signal_or_quit` carries the payload on the C stack and
/// `mark_stack` scans it conservatively — but this collector is precise, so
/// the payload has to be an explicit root or a collection reached from an
/// `unwind-protect` cleanup reclaims it and `condition-case` binds a dangling
/// cons.
///
/// This is the cheap, direct pin for that class: build a signal whose payload
/// is heap-allocated, drop every OTHER reference to it, collect, and require
/// the payload to still be a live list. Before the fix the cons was on the
/// free list, so its car read back as [`Value::DEAD`] — GNU's `dead_object`.
#[test]
fn in_flight_signal_payload_survives_a_collection() {
    crate::test_utils::init_test_tracing();
    let mut eval = Context::new();

    let flow = crate::emacs_core::error::signal_with_data(
        LispCondition::Error,
        Value::list(vec![Value::string("Malformed argument list")]),
    );

    // Nothing else references the payload: it is reachable ONLY through the
    // in-flight signal, which is what the root set has to cover.
    eval.gc_collect();

    let Flow::Signal(sig) = &flow else {
        panic!("signal_with_data builds a signal flow");
    };
    let raw = sig.raw_data.expect("signal_with_data records raw data");
    assert!(raw.is_cons(), "payload stays a cons: {raw:?}");
    assert!(
        !raw.cons_car().is_dead(),
        "the in-flight signal payload was collected while the signal was still \
         unwinding (its cons is on the free list)"
    );
    assert_eq!(
        print_value_with_eval(&eval, &super::make_signal_binding_value(sig)),
        "(error \"Malformed argument list\")"
    );
}

/// The same guarantee one level up: what `condition-case` binds must be a live
/// datum after a collection, not a resurrected free-list cell. This is the
/// shape the oracle probe `div_v8_cl_defun_key_aux_rest_optional` hit — the
/// error datum printed as `(error . <garbage symbol>)` and the printer panicked
/// in `resolve_sym_lisp_string`.
#[test]
fn condition_case_binding_value_survives_a_collection() {
    crate::test_utils::init_test_tracing();
    let mut eval = Context::new();

    let flow = crate::emacs_core::error::signal_with_data(
        LispCondition::Error,
        Value::list(vec![Value::string("Malformed argument list ends with")]),
    );
    eval.gc_collect();

    let Flow::Signal(sig) = &flow else {
        panic!("signal flow");
    };
    let bound = super::make_signal_binding_value(sig);
    assert_eq!(
        print_value_with_eval(&eval, &bound),
        "(error \"Malformed argument list ends with\")"
    );
}
