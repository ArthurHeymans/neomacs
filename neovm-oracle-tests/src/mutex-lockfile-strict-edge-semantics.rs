//! Oracle parity for make-mutex, mutex-name, mutex-lock, lock-file.
//! GNU src/thread.c, src/filelock.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

// --- make-mutex ---

#[test]
fn oracle_make_mutex_returns_mutex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mutexp (make-mutex "test"))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_make_mutex_no_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mutexp (make-mutex))"#);
    assert_ok_eq("t", &o, &n);
}

// --- mutex-name ---

#[test]
fn oracle_mutex_name_returns_name() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mutex-name (make-mutex "my-mutex"))"#);
    assert_ok_eq("\"my-mutex\"", &o, &n);
}

#[test]
fn oracle_mutex_name_unnamed() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(mutex-name (make-mutex))"#);
    // Default name: nil
    assert_ok_eq("nil", &o, &n);
}

// --- mutex-lock ---

#[test]
fn oracle_mutex_lock_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) =
        eval_oracle_and_neovm(r#"(progn (setq m (make-mutex "lock-test")) (mutex-lock m))"#);
    assert_ok_eq("nil", &o, &n);
}

// --- lock-file ---

#[test]
fn oracle_lock_file_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(lock-file "/tmp/neomacs-oracle-test-lock-file")"#);
    assert_ok_eq("nil", &o, &n);
}
