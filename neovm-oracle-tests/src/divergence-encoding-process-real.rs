//! Divergence tests: real encoding/decoding behavioral differences.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_encode_decode_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(progn
  (insert \"Hello World\")
  (encode-coding-region 1 12 'utf-8)
  (list (buffer-string)
        (length (buffer-string)))) ",
    );
}

#[test]
fn divergence_coding_system_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (coding-system-base 'utf-8)
  (coding-system-base 'utf-8-dos)
  (coding-system-base 'latin-1)
  (coding-system-base 'no-conversion)
  (coding-system-p 'utf-8)
  (coding-system-p 'nonexistent-cs-xxx)) ",
    );
}

#[test]
fn divergence_coding_system_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((cs (coding-system-priority-list)))
  (list (car cs)
        (>= (length cs) 1)
        (eq (car cs) 'utf-8))) ",
    );
}

#[test]
fn divergence_charset_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (charsetp 'ascii)
  (charsetp 'unicode)
  (charsetp 'nonexistent-xxx)
  (encode-char ?A 'ascii)
  (decode-char 'ascii 65)) ",
    );
}

#[test]
fn divergence_string_encode_decode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((s \"caf\\u00e9\"))
  (list (encode-coding-string s 'utf-8)
        (decode-coding-string (encode-coding-string s 'utf-8) 'utf-8)
        (string= s (decode-coding-string (encode-coding-string s 'utf-8) 'utf-8))
        (length (encode-coding-string s 'utf-8)))) ",
    );
}

#[test]
fn divergence_process_environment() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (stringp (getenv \"HOME\"))
  (stringp (getenv \"PATH\"))
  (stringp (getenv \"SHELL\"))
  (> (length (getenv \"PATH\")) 10)
  (> (length process-environment) 5)) ",
    );
}

#[test]
fn divergence_shell_command_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((out (shell-command-to-string \"echo hello\")))
  (list (string-trim out)
        (string= (string-trim out) \"hello\"))) ",
    );
}

#[test]
fn divergence_call_process_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(let ((result (with-temp-buffer
                 (insert \"hello\")
                 (call-process-region (point-min) (point-max)
                                      \"cat\" t t)
                 (buffer-string))))
  (list (string-trim result)
        (string= (string-trim result) \"hello\"))) ",
    );
}

#[test]
fn divergence_process_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (listp (process-list))
  (<= (length (process-list)) 0)
  (processp nil)) ",
    );
}

#[test]
fn divergence_system_info() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list
  (stringp system-name)
  (stringp system-configuration)
  (memq system-type '(gnu/linux gnu darwin windows-nt))
  (integerp emacs-pid)
  (> emacs-pid 0)
  (stringp emacs-version)
  (>= (length emacs-version) 5)
  (integerp emacs-major-version)) ",
    );
}
