//! Oracle parity tests for `default-boundp`.
//!
//! GNU implements `default-boundp` in `src/data.c` via `Fdefault_boundp`,
//! which calls `default_value(symbol)` and checks whether it's `Qunbound`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_default_boundp_nil_for_unbound_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(default-boundp 'neovm--test-void-unbound-xyz789)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_default_boundp_t_for_global_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (defvar neovm--test-def-boundp-var t)
  (default-boundp 'neovm--test-def-boundp-var))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_default_boundp_t_for_buffer_local_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn
  (defvar neovm--test-per-buffer-var "default-val")
  (make-variable-buffer-local 'neovm--test-per-buffer-var)
  (list
   (default-boundp 'neovm--test-per-buffer-var)
   (boundp 'neovm--test-per-buffer-var)))"#,
        expect_test::expect![[r#""OK (t t)""#]],
    );
    assert_ok_eq("(t t)", &oracle, &neovm);
}

#[test]
fn oracle_default_boundp_with_symbol_arg_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(default-boundp 'emacs-version)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_default_boundp_wrong_type_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(default-boundp 123)",
        expect_test::expect![[r#""ERR (wrong-type-argument symbolp 123)""#]],
    );
    assert_err_kind(&oracle, &neovm, "wrong-type-argument");
}

#[test]
fn oracle_default_boundp_wrong_number_of_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(default-boundp)",
        expect_test::expect![[r#""ERR (wrong-number-of-arguments default-boundp 0)""#]],
    );
    assert_err_kind(&oracle, &neovm, "wrong-number-of-arguments");

    let (oracle2, neovm2) = crate::common::eval_oracle_and_neovm_expect(
        "(default-boundp 'a 'b)",
        expect_test::expect![[r#""ERR (wrong-number-of-arguments default-boundp 2)""#]],
    );
    assert_err_kind(&oracle2, &neovm2, "wrong-number-of-arguments");
}

#[test]
fn oracle_default_boundp_constants_are_bound() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(list
  (default-boundp 't)
  (default-boundp 'nil)
  (default-boundp 'emacs-version))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
    assert_ok_eq("(t t t)", &oracle, &neovm);
}
