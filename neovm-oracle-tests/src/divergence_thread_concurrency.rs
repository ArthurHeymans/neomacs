//! Divergence tests: thread, concurrency, mutex, async deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_thread_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'make-thread)
  (fboundp 'thread-name)
  (fboundp 'thread-signal)
  (fboundp 'thread-live-p)
  (fboundp 'all-threads)
  (fboundp 'current-thread)
  (fboundp 'main-thread)
  (featurep 'threads)) "#,
    );
}

#[test]
fn divergence_mutex_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'make-mutex)
  (fboundp 'mutex-lock)
  (fboundp 'mutex-unlock)
  (fboundp 'mutex-owner)) "#,
    );
}

#[test]
fn divergence_condition_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'make-condition-variable)
  (fboundp 'condition-wait)
  (fboundp 'condition-notify)
  (fboundp 'condition-broadcast)) "#,
    );
}

#[test]
fn divergence_async_processes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'make-process)
  (fboundp 'async-shell-command)
  (boundp 'async-shell-command-buffer)
  (stringp async-shell-command-buffer)) "#,
    );
}

#[test]
fn divergence_idle_timers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'run-with-idle-timer)
  (fboundp 'run-at-time)
  (fboundp 'cancel-timer)
  (fboundp 'cancel-function-timers)
  (fboundp 'timerp)
  (boundp 'timer-idle-list)
  (listp timer-idle-list)
  (boundp 'timer-list)
  (listp timer-list)) "#,
    );
}

#[test]
fn divergence_process_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'processp)
  (fboundp 'process-type)
  (fboundp 'process-status)
  (fboundp 'process-exit-status)
  (fboundp 'process-id)) "#,
    );
}

#[test]
fn divergence_process_connection() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'process-send-string)
  (fboundp 'process-send-region)
  (fboundp 'process-send-eof)
  (fboundp 'process-send-signal)) "#,
    );
}

#[test]
fn divergence_network_options() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'network-security-level)
  (stringp network-security-level)
  (boundp 'network-security-protocol-checks)
  (listp network-security-protocol-checks)) "#,
    );
}

#[test]
fn divergence_connection_local() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'connection-local-set-profile-variables)
  (fboundp 'connection-local-set-profiles)
  (fboundp 'connection-local-value)
  (featurep 'connection)) "#,
    );
}

#[test]
fn divergence_concurrency_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'inhibit-changing-match-data)
  (booleanp inhibit-changing-match-data)
  (boundp 'inhibit-message)
  (boundp 'message-log-max)
  (integerp message-log-max)
  (boundp 'messages-buffer-name)
  (stringp messages-buffer-name)) "#,
    );
}
