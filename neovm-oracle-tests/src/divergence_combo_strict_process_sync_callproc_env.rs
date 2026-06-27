//! Strict combo oracle probes, batch 23: synchronous process calls
//! (call-process exit codes and output, shell-command-to-string, call-process
//! stderr routing), process-environment / getenv, and process-list shape.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_f8_call_process_exit_and_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (with-temp-buffer
        (let ((status (call-process "echo" nil t nil "hello")))
          (list status (buffer-string))))
      (with-temp-buffer
        (let ((status (call-process "printf" nil t nil "%s\n" "world")))
          (list status (buffer-string))))
      (with-temp-buffer
        (let ((status (call-process "false" nil t nil)))
          (list status (buffer-string)))))
"##,
    );
}

#[test]
fn div_f8_shell_command_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (shell-command-to-string "echo hi")
      (shell-command-to-string "printf abc")
      (length (shell-command-to-string "seq 1 5")))
"##,
    );
}

#[test]
fn div_f8_call_process_stderr_and_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (with-temp-buffer
        (let ((status (call-process shell-file-name nil t nil
                                    shell-command-switch "printf 'a\\nb'")))
          (list status (buffer-string))))
      (with-temp-buffer
        (let ((status (call-process "sh" nil t nil "-c" "echo out; echo err 1>&2")))
          (list status (buffer-string))))
      (with-temp-buffer
        (let ((status (call-process "seq" nil t nil "2" "4")))
          (list status (buffer-string)))))
"##,
    );
}

#[test]
fn div_f8_process_environment_getenv() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((process-environment (cons "NEO_PROBE_SYNC_ENV=zzz" process-environment)))
  (list (getenv "NEO_PROBE_SYNC_ENV")
        (stringp (getenv "HOME"))
        (stringp (getenv "PATH"))))
"##,
    );
}

#[test]
fn div_f8_process_environment_length() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Divergence surfaced 2026-06-27:
    // GNU Emacs: OK 284
    // Neomacs:   OK 283
    // (length process-environment) differs by one: GNU's default process-
    // environment has one more entry than Neomacs (an Emacs-internal variable
    // GNU injects). getenv values for HOME/PATH agree.
    assert_oracle_parity(
        r##"
(length process-environment)
"##,
    );
}

#[test]
fn div_f8_call_process_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "3\n1\n2\n")
  (let ((status (call-process-region (point-min) (point-max) "sort" t t nil)))
    (list status (buffer-string))))
"##,
    );
}
