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

/// `defvar_bool` conses the symbol onto `byte-boolean-vars`
/// (`src/lread.c:5261`), but `syms_of_lread` then writes
/// `Vbyte_boolean_vars = Qnil` (`src/lread.c:5774`), which throws away every
/// cons `main` had made before it got there.  Measured under GNU 31.0.90,
/// `emacs -Q --batch`:
///
/// ```elisp
/// (length byte-boolean-vars)                          ;; => 117
/// (and (memq 'visible-bell byte-boolean-vars) t)      ;; => t   (dispnew.c, after)
/// (and (memq 'use-short-answers byte-boolean-vars) t) ;; => nil (fns.c, before)
/// ```
#[test]
fn byte_boolean_vars_holds_gnus_117_and_not_the_31_erased_ones() {
    let mut eval = ev();

    assert_eq!(
        format_eval_result(&eval.eval_str("(length byte-boolean-vars)")),
        "OK 117"
    );
    for name in [
        "visible-bell",
        "inhibit-message",
        "indent-tabs-mode",
        "print-quoted",
        "noninteractive",
        "font-use-system-font",
        "load-dangerous-libraries",
    ] {
        assert_eq!(
            format_eval_result(
                &eval.eval_str(&format!("(and (memq '{name} byte-boolean-vars) t)"))
            ),
            "OK t",
            "{name} should be on byte-boolean-vars"
        );
    }
    for name in [
        "use-short-answers",
        "use-dialog-box",
        "garbage-collection-messages",
        "symbols-with-pos-enabled",
        "load-in-progress",
        "load-force-doc-strings",
        "write-region-inhibit-fsync",
        "inhibit-eol-conversion",
    ] {
        assert_eq!(
            format_eval_result(
                &eval.eval_str(&format!("(and (memq '{name} byte-boolean-vars) t)"))
            ),
            "OK nil",
            "{name} is erased by syms_of_lread's own initializer"
        );
    }
}

/// The list is in reverse declaration order because `defvar_bool` prepends.
/// Measured under GNU 31.0.90: `(car byte-boolean-vars)` is the last
/// `DEFVAR_BOOL` `main` reaches (`xsettings.c`) and `(nth 116 ...)` the first
/// one after `syms_of_lread` cleared the list (`lread.c`, immediately below
/// the `DEFVAR_LISP` for the list itself).
#[test]
fn byte_boolean_vars_is_in_gnus_reverse_declaration_order() {
    let mut eval = ev();

    assert_eq!(
        format_eval_result(&eval.eval_str("(car byte-boolean-vars)")),
        "OK font-use-system-font"
    );
    assert_eq!(
        format_eval_result(&eval.eval_str("(nth 116 byte-boolean-vars)")),
        "OK load-dangerous-libraries"
    );
}

/// `store_symval_forwarding`'s `Lisp_Fwd_Bool` arm never signals -- it is
/// `*XBOOLVAR (valcontents) = !NILP (newval);` (`src/data.c:1485-1487`), so
/// `setq` returns what it was handed and the next read is `t` or `nil`.  The
/// coercion is a property of the declaration, not of the list: it applies to
/// the 31 variables `byte-boolean-vars` does not mention too.  Measured under
/// GNU 31.0.90:
///
/// ```elisp
/// (list (setq visible-bell 5) visible-bell)           ;; => (5 t)
/// (list (setq use-short-answers 5) use-short-answers) ;; => (5 t)
/// (list (setq create-lockfiles nil) create-lockfiles) ;; => (nil nil)
/// (list (setq print-quoted (list 1)) print-quoted)    ;; => ((1) t)
/// ```
#[test]
fn defvar_bool_coerces_every_variable_in_the_table_like_gnu() {
    let mut eval = ev();

    for (form, expected) in [
        ("(list (setq visible-bell 5) visible-bell)", "OK (5 t)"),
        (
            "(list (setq use-short-answers 5) use-short-answers)",
            "OK (5 t)",
        ),
        (
            "(list (setq create-lockfiles nil) create-lockfiles)",
            "OK (nil nil)",
        ),
        (
            "(list (setq print-quoted (list 1)) print-quoted)",
            "OK ((1) t)",
        ),
    ] {
        assert_eq!(format_eval_result(&eval.eval_str(form)), expected, "{form}");
    }
}

/// Every row of the table is registered, is `special`, and still holds the
/// value the table gives it once the rest of the bootstrap has run -- which is
/// what stops a leftover plain-cell seed elsewhere from quietly deciding a
/// `DEFVAR_BOOL` variable's default.
#[test]
fn every_gnu_defvar_bool_variable_is_bound_and_reads_back_canonically() {
    use crate::emacs_core::defvar_bool::GNU_BOOL_VARIABLES;
    let mut eval = ev();

    for var in GNU_BOOL_VARIABLES {
        let name = var.name;
        assert_eq!(
            format_eval_result(&eval.eval_str(&format!("(boundp '{name})"))),
            "OK t",
            "{name} should be bound"
        );
        assert_eq!(
            format_eval_result(&eval.eval_str(&format!("(special-variable-p '{name})"))),
            "OK t",
            "{name} should be special"
        );
        assert_eq!(
            format_eval_result(&eval.eval_str(&format!("(default-value '{name})"))),
            if var.initial { "OK t" } else { "OK nil" },
            "{name} default"
        );
    }
}

/// The coercion has to survive `let`, `set-default` and a buffer-local
/// binding, because `do_specbind` and `set_default_internal` both route a
/// forwarded symbol through `store_symval_forwarding` (`src/eval.c:3594-3622`,
/// `src/data.c:2077`).  Measured under GNU 31.0.90:
///
/// ```elisp
/// (let ((inverse-video 3)) inverse-video)                   ;; => t
/// (progn (set-default 'inverse-video 9)
///        (default-value 'inverse-video))                    ;; => t
/// (with-temp-buffer (setq-local indent-tabs-mode 4)
///                   indent-tabs-mode)                       ;; => t
/// ```
#[test]
fn defvar_bool_coercion_survives_let_set_default_and_buffer_local_like_gnu() {
    let mut eval = ev();

    assert_eq!(
        format_eval_result(&eval.eval_str("(let ((inverse-video 3)) inverse-video)")),
        "OK t"
    );
    assert_eq!(
        format_eval_result(
            &eval.eval_str("(progn (set-default 'inverse-video 9) (default-value 'inverse-video))")
        ),
        "OK t"
    );
    let form =
        in_fresh_buffer("(progn (set (make-local-variable 'indent-tabs-mode) 4) indent-tabs-mode)");
    assert_eq!(format_eval_result(&eval.eval_str(&form)), "OK t");
}
