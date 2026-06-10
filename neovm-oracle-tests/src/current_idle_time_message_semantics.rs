//! Oracle parity tests for `current-idle-time` and `current-message`.
//!
//! GNU implements both as variables/accessors in `src/keyboard.c` and
//! `src/xdisp.c`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_current_idle_time_is_nil_or_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(or (null (current-idle-time))
       (numberp (current-idle-time))
       (and (listp (current-idle-time))
            (numberp (car (current-idle-time)))))"#,
    );
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_current_message_is_nil_or_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) =
        eval_oracle_and_neovm(r#"(or (null (current-message)) (stringp (current-message)) t)"#);
    assert_ok_eq("t", &oracle, &neovm);
}

#[test]
fn oracle_current_message_after_message_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (oracle, neovm) = eval_oracle_and_neovm(
        r#"(progn
  (message "neovm--test-msg-123")
  (or (null (current-message))
      (stringp (current-message))
      t))"#,
    );
    assert_ok_eq("t", &oracle, &neovm);
}
