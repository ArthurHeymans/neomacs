/// Batch 533: thread-join, thread-signal, all-threads, thread-last-errors.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx533_thread_join() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((t1 (make-thread (lambda () "hello") "cx533-t1")))
      (thread-join t1))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_thread_join_result_num() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((t1 (make-thread (lambda () (+ 1 2)) "cx533-num")))
      (thread-join t1))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_thread_join_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((t1 (make-thread (lambda () nil) "cx533-nil")))
      (thread-join t1))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_all_threads_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (length (all-threads))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_current_thread() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (thread-name (current-thread))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_thread_yield() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (thread-yield)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_thread_last_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (thread-last-errors)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_make_mutex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (mutex-name (make-mutex "cx533-mtx"))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_make_condvar() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (condition-name (make-condvar "cx533-cv"))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_mutex_lock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((m (make-mutex "cx533-lock")))
      (mutex-lock m)
      (mutex-unlock m)
      'ok)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_with_mutex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((m (make-mutex "cx533-wm")))
      (with-mutex m (+ 1 2)))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_condition_notify() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((cv (make-condvar "cx533-cn")))
      (condition-notify cv)
      'ok)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_threadp_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (threadp (current-thread))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_mutexp_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (mutexp (make-mutex "cx533-mp"))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx533_condvarp_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (condvarp (make-condvar "cx533-cp"))
  (error (car e)))
"##,
    );
}
