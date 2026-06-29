//! Oracle parity tests for type predicates: `processp`, `threadp`,
//! `mutexp`, `overlayp`.
//!
//! GNU implements these in `src/process.c`, `src/thread.c`,
//! `src/thread.c`, and `src/buffer.c` respectively.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_processp_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(processp nil)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_processp_non_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(list (processp 42) (processp "hello") (processp 'sym))"#,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
    assert_ok_eq("(nil nil nil)", &oracle, &neovm);
}

#[test]
fn oracle_threadp_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(threadp nil)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_threadp_non_thread() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(list (threadp 42) (threadp "hello") (threadp 'sym))"#,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
    assert_ok_eq("(nil nil nil)", &oracle, &neovm);
}

#[test]
fn oracle_mutexp_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(mutexp nil)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_mutexp_non_mutex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(list (mutexp 42) (mutexp "hello") (mutexp 'sym))"#,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
    assert_ok_eq("(nil nil nil)", &oracle, &neovm);
}

#[test]
fn oracle_overlayp_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        "(overlayp nil)",
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &oracle, &neovm);
}

#[test]
fn oracle_overlayp_non_overlay() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = crate::common::eval_oracle_and_neovm_expect(
        r#"(list (overlayp 42) (overlayp "hello") (overlayp 'sym))"#,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
    assert_ok_eq("(nil nil nil)", &oracle, &neovm);
}
