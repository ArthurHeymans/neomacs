//! Oracle parity for make-mutex, mutex-name, mutex-lock, lock-file.
//! GNU src/thread.c, src/filelock.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- make-mutex ---

#[test]
fn oracle_make_mutex_returns_mutex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(mutexp (make-mutex "test"))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_make_mutex_no_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(mutexp (make-mutex))"#,
        expect_test::expect![[r#""OK t""#]],
    );
    assert_ok_eq("t", &o, &n);
}

// --- mutex-name ---

#[test]
fn oracle_mutex_name_returns_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(mutex-name (make-mutex "my-mutex"))"#,
        expect_test::expect![[r#""OK \"my-mutex\"""#]],
    );
    assert_ok_eq("\"my-mutex\"", &o, &n);
}

#[test]
fn oracle_mutex_name_unnamed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(mutex-name (make-mutex))"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    // Default name: nil
    assert_ok_eq("nil", &o, &n);
}

// --- mutex-lock ---

#[test]
fn oracle_mutex_lock_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(progn (setq m (make-mutex "lock-test")) (mutex-lock m))"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}

// --- lock-file ---

#[test]
fn oracle_lock_file_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = crate::common::eval_oracle_and_neovm_expect(
        r#"(lock-file "/tmp/neomacs-oracle-test-lock-file")"#,
        expect_test::expect![[r#""OK nil""#]],
    );
    assert_ok_eq("nil", &o, &n);
}
