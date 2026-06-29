//! Oracle parity tests for `windowp` and `framep` type predicates.
//!
//! GNU implements both in `src/window.c` and `src/frame.c` respectively.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_windowp_returns_t_for_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(windowp (selected-window))",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_windowp_nil_for_non_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(list (windowp nil) (windowp 42) (windowp "hello") (windowp 'sym))"#,
        expect_test::expect![[r#""OK (nil nil nil nil)""#]],
    );
    assert_ok_eq("(nil nil nil nil)", &oracle, &neovm);
}

#[test]
fn oracle_framep_returns_t_for_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(framep (selected-frame))",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_framep_nil_for_non_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(list (framep nil) (framep 42) (framep "hello") (framep 'sym))"#,
        expect_test::expect![[r#""OK (nil nil nil nil)""#]],
    );
    assert_ok_eq("(nil nil nil nil)", &oracle, &neovm);
}
