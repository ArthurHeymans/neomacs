//! Divergence tests: Edebug stubs, bytecomp, compiler macros, load-history.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_edebug_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'edebug)
  (featurep 'edebug)
  (fboundp 'debug)
  (fboundp 'debug-on-entry))"#,
    );
}

#[test]
fn divergence_debugger_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (booleanp debug-on-error)
  (booleanp debug-on-quit)
  (listp debug-ignored-errors)
  (booleanp stack-trace-on-error))"#,
    );
}

#[test]
fn divergence_bytecomp_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'byte-compile)
  (fboundp 'byte-compile-file)
  (fboundp 'batch-byte-compile)
  (fboundp 'symbol-file))"#,
    );
}

#[test]
fn divergence_compiler_macros() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'define-inline)
  (fboundp 'comp--function-type)
  (fboundp 'cl-define-compiler-macro)
  (fboundp 'compiler-macroexpand))"#,
    );
}

#[test]
fn divergence_load_history() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (consp load-history)
  (listp (car load-history))
  (stringp (caar load-history)))"#,
    );
}

#[test]
fn divergence_after_load_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'eval-after-load)
  (fboundp 'with-eval-after-load)
  (fboundp 'after-load-functions))"#,
    );
}

#[test]
fn divergence_find_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (stringp (symbol-file 'car))
  (stringp (symbol-file 'find-file))
  (stringp (symbol-file 'nonexistent-fn-xyz)))"#,
    );
}

#[test]
fn divergence_finder_known() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'finder-known)
  (fboundp 'package-initialize)
  (boundp 'package-activated-list)
  (listp package-activated-list))"#,
    );
}

#[test]
fn divergence_native_comp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (featurep 'native-compile)
  (featurep 'comp)
  (fboundp 'native-compile))"#,
    );
}
