//! Oracle parity tests for `macrop`, `obarrayp`, and `daemonp`.
//!
//! GNU implements `macrop` in `src/eval.c`, `obarrayp` in `src/lread.c`,
//! and `daemonp` in `src/emacs.c`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_macrop_nil_for_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(macrop nil)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_macrop_nil_for_lambda() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(macrop (lambda () 42))",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_obarrayp_t_for_obarray() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(obarrayp obarray)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_obarrayp_nil_for_non_obarray() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(list (obarrayp nil) (obarrayp 42) (obarrayp "hello"))"#,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
    assert_ok_eq("(nil nil nil)", &oracle, &neovm);
}

#[test]
fn oracle_daemonp_returns_nil_or_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(or (null (daemonp)) (daemonp) t)",
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &oracle, &neovm);
}
