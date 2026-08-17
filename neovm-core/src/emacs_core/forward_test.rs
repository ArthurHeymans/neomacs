//! GNU parity tests for the `Lisp_Fwd` family's assignment rules.
//!
//! Every expectation here was produced by running the same form under GNU
//! Emacs 31.0.90 (`emacs -Q --batch`), never derived from the C source.

use crate::emacs_core::error::format_eval_result;
use crate::emacs_core::eval::Context;

fn ev() -> Context {
    crate::test_utils::init_test_tracing();
    Context::new()
}

/// `with-temp-buffer` / `setq-local` / `setq-default` are Lisp macros that a
/// bare [`Context`] has not loaded, so the probes spell them with the special
/// forms and subrs they expand to.
fn in_fresh_buffer(body: &str) -> String {
    format!(
        "(save-current-buffer
           (set-buffer (get-buffer-create \"fwd132\"))
           (prog1 (progn {body}) (kill-buffer \"fwd132\")))"
    )
}

/// GNU `store_symval_forwarding`'s `Lisp_Fwd_Int` arm (`src/data.c:1475-1483`)
/// runs `CHECK_INTEGER` before the store, so a `DEFVAR_INT` variable can never
/// hold a string, a float, `nil` or `t`.  Measured under GNU:
///
/// ```elisp
/// (setq undo-limit "x")   ;; => (wrong-type-argument integerp "x")
/// undo-limit              ;; => 160000
/// ```
#[test]
fn defvar_int_setq_signals_wrong_type_like_gnu() {
    let mut eval = ev();

    for (form, expected) in [
        (
            r#"(condition-case e (setq undo-limit "x") (error e))"#,
            r#"OK (wrong-type-argument integerp "x")"#,
        ),
        (
            r#"(condition-case e (setq gc-cons-threshold "x") (error e))"#,
            r#"OK (wrong-type-argument integerp "x")"#,
        ),
        (
            "(condition-case e (setq gc-cons-threshold 1.5) (error e))",
            "OK (wrong-type-argument integerp 1.5)",
        ),
        (
            "(condition-case e (setq gc-cons-threshold nil) (error e))",
            "OK (wrong-type-argument integerp nil)",
        ),
        (
            "(condition-case e (setq gc-cons-threshold t) (error e))",
            "OK (wrong-type-argument integerp t)",
        ),
        (
            "(condition-case e (setq undo-strong-limit \"x\") (error e))",
            r#"OK (wrong-type-argument integerp "x")"#,
        ),
    ] {
        assert_eq!(format_eval_result(&eval.eval_str(form)), expected, "{form}");
    }

    // The refused write leaves the old value in place, exactly as GNU's
    // longjmp out of `store_symval_forwarding` does.
    assert_eq!(
        format_eval_result(&eval.eval_str("undo-limit")),
        "OK 160000"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("gc-cons-threshold")),
        "OK 800000"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("undo-strong-limit")),
        "OK 240000"
    );
}

/// Integers still go through, including a bignum inside `intmax_t` range.
/// Measured under GNU: `(setq gc-cons-threshold (* most-positive-fixnum 4))`
/// => 9223372036854775804, and reading it back returns the same bignum.
#[test]
fn defvar_int_accepts_every_integer_intmax_can_hold_like_gnu() {
    let mut eval = ev();

    assert_eq!(
        format_eval_result(&eval.eval_str("(setq gc-cons-threshold 777777)")),
        "OK 777777"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("gc-cons-threshold")),
        "OK 777777"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("(setq gc-cons-threshold -1)")),
        "OK -1"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("gc-cons-threshold")),
        "OK -1"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("(setq gc-cons-threshold most-positive-fixnum)")),
        "OK 2305843009213693951"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("(setq gc-cons-threshold (* most-positive-fixnum 4))")),
        "OK 9223372036854775804"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("gc-cons-threshold")),
        "OK 9223372036854775804"
    );
}

/// GNU's integer arm signals `overflow-error` -- not `wrong-type-argument` --
/// for an integer too large for the `intmax_t` slot (`src/data.c:1479-1480`).
/// Measured under GNU: `(setq gc-cons-threshold (expt 2 200))` =>
/// `(overflow-error 1606938044258990275541962092341162602522202993782792835301376)`
/// and `gc-cons-threshold` is left at its previous value.
#[test]
fn defvar_int_signals_overflow_error_past_intmax_like_gnu() {
    let mut eval = ev();

    assert_eq!(
        format_eval_result(
            &eval.eval_str("(condition-case e (setq gc-cons-threshold (expt 2 200)) (error e))")
        ),
        "OK (overflow-error 1606938044258990275541962092341162602522202993782792835301376)"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("gc-cons-threshold")),
        "OK 800000"
    );
}

/// GNU checks the integer arm below `set_internal`, so every assignment
/// spelling reaches it: `set`, `set-default`, `setq-default`, a `let` binding,
/// and `make-local-variable` + `set`.  All five measured under GNU as
/// `(wrong-type-argument integerp "x")`.
#[test]
fn defvar_int_check_covers_every_assignment_spelling_like_gnu() {
    let mut eval = ev();

    let local_set = in_fresh_buffer(r#"(set (make-local-variable 'undo-limit) "x")"#);
    for form in [
        r#"(condition-case e (set 'undo-limit "x") (error e))"#.to_string(),
        r#"(condition-case e (set-default 'undo-limit "x") (error e))"#.to_string(),
        r#"(condition-case e (let ((undo-limit "x")) undo-limit) (error e))"#.to_string(),
        format!("(condition-case e {local_set} (error e))"),
    ] {
        assert_eq!(
            format_eval_result(&eval.eval_str(&form)),
            r#"OK (wrong-type-argument integerp "x")"#,
            "{form}"
        );
    }

    assert_eq!(
        format_eval_result(&eval.eval_str("undo-limit")),
        "OK 160000"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("(default-value 'undo-limit)")),
        "OK 160000"
    );
}

/// A per-buffer binding of a `DEFVAR_INT` variable is still an integer slot.
/// Measured under GNU:
/// `(with-temp-buffer (setq-local undo-limit 5) (list undo-limit (default-value 'undo-limit)))`
/// => `(5 160000)`.
#[test]
fn defvar_int_buffer_local_binding_keeps_the_default_like_gnu() {
    let mut eval = ev();

    let form = in_fresh_buffer(
        "(progn (set (make-local-variable 'undo-limit) 5)
                (list undo-limit (default-value 'undo-limit)))",
    );
    assert_eq!(format_eval_result(&eval.eval_str(&form)), "OK (5 160000)");
    assert_eq!(
        format_eval_result(&eval.eval_str("undo-limit")),
        "OK 160000"
    );
}

/// A forwarded slot has no "unbound" bit pattern, so GNU refuses to create
/// one: `error ("Built-in variable may not be unbound : %s")`
/// (`src/data.c:1725-1728` and `:1805-1807`).  Measured under GNU:
///
/// ```elisp
/// (makunbound 'gc-cons-threshold)
/// ;; => (error "Built-in variable may not be unbound : gc-cons-threshold")
/// (boundp 'gc-cons-threshold)  ;; => t
/// ```
#[test]
fn forwarded_variables_refuse_makunbound_like_gnu() {
    let mut eval = ev();

    assert_eq!(
        format_eval_result(
            &eval.eval_str("(condition-case e (makunbound 'gc-cons-threshold) (error e))")
        ),
        r#"OK (error "Built-in variable may not be unbound : gc-cons-threshold")"#
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("(boundp 'gc-cons-threshold)")),
        "OK t"
    );
    assert_eq!(
        format_eval_result(
            &eval.eval_str("(condition-case e (makunbound 'inhibit-message) (error e))")
        ),
        r#"OK (error "Built-in variable may not be unbound : inhibit-message")"#
    );
}

/// GNU's `Lisp_Fwd_Bool` arm does NOT signal -- it coerces, storing
/// `!NILP (newval)` (`src/data.c:1485-1487`).  `setq` still returns the value
/// it was given; only the variable is canonical.  Measured under GNU:
///
/// ```elisp
/// (setq inhibit-message 5)   ;; => 5
/// inhibit-message            ;; => t
/// ```
#[test]
fn defvar_bool_coerces_instead_of_signalling_like_gnu() {
    let mut eval = ev();

    assert_eq!(
        format_eval_result(&eval.eval_str("(setq inhibit-message 5)")),
        "OK 5"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("inhibit-message")),
        "OK t"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str(r#"(progn (setq inhibit-message "s") inhibit-message)"#)),
        "OK t"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("(progn (setq inhibit-message nil) inhibit-message)")),
        "OK nil"
    );
}

/// The Boolean coercion is a property of the forwarder, so it survives every
/// assignment spelling too.  `(set 'inhibit-message 9)` returns 9 but leaves
/// `t` behind, and `set-default` / `setq-default` do the same -- all measured
/// under GNU.
#[test]
fn defvar_bool_coercion_covers_every_assignment_spelling_like_gnu() {
    let mut eval = ev();

    assert_eq!(
        format_eval_result(&eval.eval_str("(progn (set 'inhibit-message 9) inhibit-message)")),
        "OK t"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str(
            "(progn (setq inhibit-message nil)
                    (set-default 'inhibit-message 9)
                    (default-value 'inhibit-message))"
        )),
        "OK t"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("(let ((inhibit-message 5)) inhibit-message)")),
        "OK t"
    );
}

/// A per-buffer binding of a `DEFVAR_BOOL` variable is a Boolean slot too.
/// Measured under GNU:
/// `(with-temp-buffer (setq-local inhibit-message 3) (list inhibit-message
///  (default-value 'inhibit-message)))` => `(t nil)`.
///
/// And making one buffer's binding must not disarm the forwarder for the
/// global cell: measured under GNU, a later `(setq inhibit-message 7)` still
/// reads back `t`.
#[test]
fn defvar_bool_survives_make_local_variable_like_gnu() {
    let mut eval = ev();

    let form = in_fresh_buffer(
        "(progn (set (make-local-variable 'inhibit-message) 3)
                (list inhibit-message (default-value 'inhibit-message)))",
    );
    assert_eq!(format_eval_result(&eval.eval_str(&form)), "OK (t nil)");
    assert_eq!(
        format_eval_result(&eval.eval_str("(progn (setq inhibit-message 7) inhibit-message)")),
        "OK t"
    );
}

/// Registering a `DEFVAR_BOOL` variable also puts its symbol on
/// `byte-boolean-vars` -- GNU does it inside `defvar_bool` itself
/// (`src/lread.c:5261`).  The byte optimizer reads that list before folding a
/// `varset X; varref X` pair back into the stored value, because "what we put
/// in might not be what we get out"
/// (`lisp/emacs-lisp/byte-opt.el:2285-2300`).  Measured under GNU 31.0.90:
/// `(memq 'inhibit-message byte-boolean-vars)` is non-nil, and
/// `(special-variable-p 'byte-boolean-vars)` is t.
#[test]
fn defvar_bool_registration_lists_the_symbol_in_byte_boolean_vars_like_gnu() {
    let mut eval = ev();

    assert_eq!(
        format_eval_result(&eval.eval_str("(and (memq 'inhibit-message byte-boolean-vars) t)")),
        "OK t"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("(special-variable-p 'byte-boolean-vars)")),
        "OK t"
    );
}
