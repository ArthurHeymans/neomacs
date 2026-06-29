//! Divergence tests: subprocess, pipe, signal handling deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_call_process_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (fboundp 'call-process)
  (fboundp 'call-process-region)
  (fboundp 'process-file)
  (fboundp 'process-file-region)
  (listp (process-environment))
  (stringp (getenv \"PATH\"))) ",
        expect_test::expect![[r#""ERR (void-function process-environment)""#]],
    );
}

#[test]
fn divergence_shell_command_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (fboundp 'shell-command)
  (fboundp 'shell-command-to-string)
  (boundp 'shell-file-name)
  (stringp shell-file-name)
  (boundp 'shell-command-switch)
  (stringp shell-command-switch)) ",
        expect_test::expect![[r#""OK (t t t t t t)""#]],
    );
}

#[test]
fn divergence_process_env_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (listp process-environment)
  (fboundp 'getenv)
  (fboundp 'setenv)
  (boundp 'initial-environment)
  (listp initial-environment)) ",
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_signal_handling() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (fboundp 'signal-process)
  (fboundp 'interrupt-process)
  (fboundp 'kill-process)
  (fboundp 'quit-process)
  (fboundp 'stop-process)
  (fboundp 'continue-process)) ",
        expect_test::expect![[r#""OK (t t t t t t)""#]],
    );
}

#[test]
fn divergence_process_io() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (fboundp 'process-send-string)
  (fboundp 'process-send-region)
  (fboundp 'process-send-buffer)
  (fboundp 'process-send-eof)
  (fboundp 'accept-process-output)) ",
        expect_test::expect![[r#""OK (t t nil t t)""#]],
    );
}

#[test]
fn divergence_process_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (fboundp 'process-list)
  (fboundp 'get-process)
  (fboundp 'process-name)
  (fboundp 'process-command)
  (fboundp 'process-status)
  (fboundp 'process-exit-status)) ",
        expect_test::expect![[r#""OK (t t t t t t)""#]],
    );
}

#[test]
fn divergence_process_tty() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (fboundp 'make-serial-process)
  (fboundp 'serial-process-config)
  (fboundp 'make-network-process)
  (fboundp 'set-network-process-option)) ",
        expect_test::expect![[r#""OK (t nil t t)""#]],
    );
}

#[test]
fn divergence_process_filter() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (fboundp 'set-process-filter)
  (fboundp 'set-process-sentinel)
  (fboundp 'set-process-buffer)
  (fboundp 'set-process-window-size)
  (fboundp 'set-process-query-on-exit-flag)) ",
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_process_coding_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (fboundp 'set-process-coding-system)
  (fboundp 'process-coding-system)
  (fboundp 'set-process-plist)
  (fboundp 'process-plist)) ",
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_process_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        "(list
  (fboundp 'process-get)
  (fboundp 'process-put)
  (fboundp 'process-contact)
  (fboundp 'process-type)
  (fboundp 'process-tty-name)
  (fboundp 'process-multithreaded)) ",
        expect_test::expect![[r#""OK (t t t t t nil)""#]],
    );
}
