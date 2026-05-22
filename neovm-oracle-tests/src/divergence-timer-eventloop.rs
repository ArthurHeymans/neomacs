//! Divergence tests: timer, idle-timer, and event loop stubs.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_timer_functions_exist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'run-at-time)
  (fboundp 'run-with-timer)
  (fboundp 'run-with-idle-timer)
  (fboundp 'cancel-timer)
  (fboundp 'cancel-function-timers))"#,
    );
}

#[test]
fn divergence_current_idle_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'current-idle-time)
  (fboundp 'current-time)
  (fboundp 'float-time)
  (timep (current-time))
  (float-time (current-time)))"#,
    );
}

#[test]
fn divergence_time_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (stringp (format-time-string "%Y-%m-%d"))
  (stringp (format-time-string "%H:%M:%S" nil t))
  (stringp (format-time-string "%s"))
  (> (length (format-time-string "%Y-%m-%d %T")) 5))"#,
    );
}

#[test]
fn divergence_time_arithmetic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let* ((t1 (current-time))
        (t2 (time-add t1 60)))
  (list (time-less-p t1 t2)
        (>= (float-time (time-subtract t2 t1)) 59)
        (time-equal-p t1 t1)))"#,
    );
}

#[test]
fn divergence_time_parse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (consp (parse-time-string "2024-01-15 10:30:00"))
  (decoded-time-year (parse-time-string "2024-01-15"))
  (decoded-time-month (parse-time-string "March 15, 2024")))"#,
    );
}

#[test]
fn divergence_encode_decode_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((encoded (encode-time 30 45 12 15 1 2024 t)))
  (list (consp encoded)
        (float-time encoded)
        (>= (float-time encoded) 0)))"#,
    );
}

#[test]
fn divergence_sleep_for_exists() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'sleep-for)
  (fboundp 'sit-for)
  (subrp (symbol-function 'sit-for)))"#,
    );
}

#[test]
fn divergence_accept_process_output_exists() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'accept-process-output)
  (fboundp 'waiting-for-user-input-p)
  (fboundp 'input-pending-p))"#,
    );
}
