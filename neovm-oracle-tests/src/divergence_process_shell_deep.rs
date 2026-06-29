//! Divergence tests: sub-process, shell, call-process deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_call_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'call-process)
  (fboundp 'call-process-region)
  (fboundp 'process-file)
  (fboundp 'process-file-region))"#,
        expect_test::expect![[r#""OK (t t t nil)""#]],
    );
}

#[test]
fn divergence_shell_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'shell-command)
  (fboundp 'shell-command-to-string)
  (fboundp 'async-shell-command)
  (boundp 'shell-file-name)
  (boundp 'shell-command-switch))"#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_make_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'make-process)
  (fboundp 'make-pipe-process)
  (fboundp 'process-contact)
  (fboundp 'process-type))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_process_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'process-list)
  (fboundp 'get-process)
  (fboundp 'delete-process)
  (fboundp 'process-status))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_process_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'accept-process-output)
  (fboundp 'process-buffer)
  (fboundp 'process-mark)
  (fboundp 'set-process-buffer))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_process_filter_sentinel() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'set-process-filter)
  (fboundp 'set-process-sentinel)
  (fboundp 'process-filter)
  (fboundp 'process-sentinel))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_process_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'set-process-coding-system)
  (fboundp 'process-coding-system)
  (fboundp 'set-process-query-on-exit-flag)
  (fboundp 'process-query-on-exit-flag))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_process_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'process-get)
  (fboundp 'process-put)
  (fboundp 'process-plist)
  (fboundp 'set-process-plist))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_signal_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'signal-process)
  (fboundp 'interrupt-process)
  (fboundp 'kill-process)
  (fboundp 'quit-process)
  (fboundp 'stop-process)
  (fboundp 'continue-process))"#,
        expect_test::expect![[r#""OK (t t t t t t)""#]],
    );
}

#[test]
fn divergence_network_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'make-network-process)
  (fboundp 'make-serial-process)
  (fboundp 'process-datagram-address))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}
