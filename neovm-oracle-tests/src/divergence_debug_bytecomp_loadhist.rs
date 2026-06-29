//! Divergence tests: Edebug stubs, bytecomp, compiler macros, load-history.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_edebug_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'edebug)
  (featurep 'edebug)
  (fboundp 'debug)
  (fboundp 'debug-on-entry))"#, expect_test::expect![[r#""OK (nil nil t t)""#]]);
}

#[test]
fn divergence_debugger_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (booleanp debug-on-error)
  (booleanp debug-on-quit)
  (listp debug-ignored-errors)
  (booleanp stack-trace-on-error))"#, expect_test::expect![[r#""ERR (void-variable stack-trace-on-error)""#]]);
}

#[test]
fn divergence_bytecomp_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'byte-compile)
  (fboundp 'byte-compile-file)
  (fboundp 'batch-byte-compile)
  (fboundp 'symbol-file))"#, expect_test::expect![[r#""OK (t t t t)""#]]);
}

#[test]
fn divergence_compiler_macros() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'define-inline)
  (fboundp 'comp--function-type)
  (fboundp 'cl-define-compiler-macro)
  (fboundp 'compiler-macroexpand))"#, expect_test::expect![[r#""OK (t nil t nil)""#]]);
}

#[test]
fn divergence_load_history() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (consp load-history)
  (listp (car load-history))
  (stringp (caar load-history)))"#, expect_test::expect![[r#""OK (t t t)""#]]);
}

#[test]
fn divergence_after_load_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'eval-after-load)
  (fboundp 'with-eval-after-load)
  (fboundp 'after-load-functions))"#, expect_test::expect![[r#""OK (t t nil)""#]]);
}

#[test]
fn divergence_find_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (stringp (symbol-file 'car))
  (stringp (symbol-file 'find-file))
  (stringp (symbol-file 'nonexistent-fn-xyz)))"#, expect_test::expect![[r#""OK (nil t nil)""#]]);
}

#[test]
fn divergence_finder_known() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'finder-known)
  (fboundp 'package-initialize)
  (boundp 'package-activated-list)
  (listp package-activated-list))"#, expect_test::expect![[r#""OK (nil t t t)""#]]);
}

#[test]
fn divergence_native_comp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (featurep 'native-compile)
  (featurep 'comp)
  (fboundp 'native-compile))"#, expect_test::expect![[r#""OK (nil nil t)""#]]);
}
