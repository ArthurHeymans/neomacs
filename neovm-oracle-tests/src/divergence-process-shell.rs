//! Divergence tests: process communication, pipe, shell-command stubs.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_start_process_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((proc (start-process "test-echo" "*test-echo-output*" "echo" "hello")))
  (list (processp proc)
        (process-name proc)
        (process-command proc)
        (process-status proc)))"#,
    );
}

#[test]
fn divergence_call_process_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(with-temp-buffer
  (call-process "echo" nil t nil "test-output")
  (buffer-string))"#,
    );
}

#[test]
fn divergence_process_exit_status() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((status (call-process "true" nil nil)))
  (list status (numberp status)))"#,
    );
}

#[test]
fn divergence_process_exit_failure() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((status (call-process "false" nil nil)))
  (list status (numberp status)))"#,
    );
}

#[test]
fn divergence_shell_command_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((output (shell-command-to-string "echo hello")))
  (list output (string-match "hello" output)))"#,
    );
}

#[test]
fn divergence_process_environment() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (listp process-environment)
  (not (null (member "HOME" (mapcar (lambda (e) (and (stringp e)
                                                (substring e 0 (string-match "=" e))))
                                    process-environment))))
  (> (length process-environment) 0))"#,
    );
}

#[test]
fn divergence_setenv_getenv() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (setenv "NEOVM_TEST_VAR" "testval")
  (let ((val (getenv "NEOVM_TEST_VAR")))
    (setenv "NEOVM_TEST_VAR")
    (list val
          (getenv "NEOVM_TEST_VAR"))))"#,
    );
}

#[test]
fn divergence_exec_path() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (listp exec-path)
  (> (length exec-path) 0)
  (member (expand-file-name "bin" invocation-directory) exec-path))"#,
    );
}

#[test]
fn divergence_process_send_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'process-send-string)
  (fboundp 'process-send-region)
  (fboundp 'process-send-eof)
  (fboundp 'interrupt-process)
  (fboundp 'kill-process))"#,
    );
}

#[test]
fn divergence_process_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'process-buffer)
  (fboundp 'process-get)
  (fboundp 'process-put)
  (fboundp 'set-process-buffer)
  (fboundp 'set-process-filter))"#,
    );
}
