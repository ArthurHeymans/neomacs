//! Oracle parity tests for GNU `condition-case-unless-debug` macro semantics.
//!
//! GNU defines this macro in `lisp/subr.el`: it rewrites every non-`:success`
//! handler condition to include `debug`, while leaving `:success` clauses
//! structurally unchanged.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_condition_case_unless_debug_macroexpansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(macroexpand
 '(condition-case-unless-debug err
      (error "x")
    (file-error :file)
    ((arith-error error) :num)
    (:success value (list :ok value))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_condition_case_unless_debug_runtime_handler_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (condition-case-unless-debug err
     (signal 'file-error '("open" "missing"))
   (file-error (list (car err) (cdr err))))
 (condition-case-unless-debug err
     (signal 'arith-error '(bad-number))
   ((arith-error error) (list (car err) (cdr err))))
 (condition-case-unless-debug err
     (+ 1 2)
   (error :error)
   (:success value (list :success value))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_condition_case_unless_debug_respects_debug_on_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (let ((debug-on-error nil)
       (debugger (lambda (&rest args)
                   (list :unexpected-debugger args))))
   (condition-case-unless-debug err
       (error "normal")
     (error (list :caught (car err) (cdr err)))))
 (let ((debug-on-error t)
       (debugger (lambda (&rest args)
                   (throw 'neomacs-debugger
                          (list :debugger args)))))
   (catch 'neomacs-debugger
     (condition-case-unless-debug err
         (error "debug")
       (error (list :caught err)))))
 (let ((debug-on-error t)
       (debugger-called nil)
       (debugger (lambda (&rest args)
                   (setq debugger-called args))))
   (list (with-demoted-errors "demoted: %S"
           (error "demoted"))
         debugger-called)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
