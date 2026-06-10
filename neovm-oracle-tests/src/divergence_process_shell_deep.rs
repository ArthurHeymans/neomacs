//! Divergence tests: sub-process, shell, call-process deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_call_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'call-process)
  (fboundp 'call-process-region)
  (fboundp 'process-file)
  (fboundp 'process-file-region))"#,
    );
}

#[test]
fn divergence_shell_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'shell-command)
  (fboundp 'shell-command-to-string)
  (fboundp 'async-shell-command)
  (boundp 'shell-file-name)
  (boundp 'shell-command-switch))"#,
    );
}

#[test]
fn divergence_make_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'make-process)
  (fboundp 'make-pipe-process)
  (fboundp 'process-contact)
  (fboundp 'process-type))"#,
    );
}

#[test]
fn divergence_process_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'process-list)
  (fboundp 'get-process)
  (fboundp 'delete-process)
  (fboundp 'process-status))"#,
    );
}

#[test]
fn divergence_process_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'accept-process-output)
  (fboundp 'process-buffer)
  (fboundp 'process-mark)
  (fboundp 'set-process-buffer))"#,
    );
}

#[test]
fn divergence_process_filter_sentinel() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'set-process-filter)
  (fboundp 'set-process-sentinel)
  (fboundp 'process-filter)
  (fboundp 'process-sentinel))"#,
    );
}

#[test]
fn divergence_process_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'set-process-coding-system)
  (fboundp 'process-coding-system)
  (fboundp 'set-process-query-on-exit-flag)
  (fboundp 'process-query-on-exit-flag))"#,
    );
}

#[test]
fn divergence_process_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'process-get)
  (fboundp 'process-put)
  (fboundp 'process-plist)
  (fboundp 'set-process-plist))"#,
    );
}

#[test]
fn divergence_signal_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'signal-process)
  (fboundp 'interrupt-process)
  (fboundp 'kill-process)
  (fboundp 'quit-process)
  (fboundp 'stop-process)
  (fboundp 'continue-process))"#,
    );
}

#[test]
fn divergence_network_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'make-network-process)
  (fboundp 'make-serial-process)
  (fboundp 'process-datagram-address))"#,
    );
}
