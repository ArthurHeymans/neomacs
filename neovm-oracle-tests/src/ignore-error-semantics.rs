//! Oracle parity tests for GNU `ignore-error` macro semantics.
//!
//! GNU defines `ignore-error` in `lisp/subr.el`.  The condition argument is
//! syntax, not an evaluated value; list conditions are used directly, quoted
//! singleton conditions are accepted through GNU's warning macro path, and an
//! empty body macroexpands to nil.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_ignore_error_macroexpansion_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (macroexpand '(ignore-error (end-of-file) (read "")))
 (macroexpand '(ignore-error 'error (read "")))
 (macroexpand '(ignore-error arith-error)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_ignore_error_runtime_condition_matching() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (ignore-error (end-of-file) (read ""))
 (ignore-error end-of-file (read ""))
 (ignore-error (arith-error wrong-type-argument) (/ 1 0))
 (ignore-error (arith-error wrong-type-argument) (car 42))
 (condition-case err
     (ignore-error end-of-file (/ 1 0))
   (error (list (car err) (cdr err))))
 (ignore-error error (+ 1 2)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
