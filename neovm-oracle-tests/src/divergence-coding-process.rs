//! Divergence tests: coding systems, encoding/decoding, process stubs.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_coding_system_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (coding-system-p 'utf-8)
  (coding-system-p 'iso-8859-1)
  (coding-system-p 'binary)
  (coding-system-p 'us-ascii)
  (coding-system-p 'no-such-coding-system))"#,
    );
}

#[test]
fn divergence_encode_decode_utf8() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let* ((str "Héllo 世界")
         (encoded (encode-coding-string str 'utf-8))
         (decoded (decode-coding-string encoded 'utf-8)))
  (list (string-equal str decoded)
        (string-bytes encoded)
        (multibyte-string-p str)
        (multibyte-string-p decoded)))"#,
    );
}

#[test]
fn divergence_encode_decode_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let* ((bytes (unibyte-string 72 101 108 108 111))
         (decoded (decode-coding-string bytes 'binary)))
  (list (string-equal decoded "Hello")
        (multibyte-string-p bytes)
        (multibyte-string-p decoded)))"#,
    );
}

#[test]
fn divergence_coding_system_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (coding-system-base 'utf-8)
  (coding-system-base 'utf-8-dos)
  (coding-system-base 'iso-latin-1)
  (coding-system-eol-type 'utf-8)
  (coding-system-eol-type 'utf-8-dos))"#,
    );
}

#[test]
fn divergence_coding_system_priority() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((cs (find-coding-systems-string "Hello World")))
  (list (consp cs)
        (member 'utf-8 cs)
        (member 'raw-text cs)))"#,
    );
}

#[test]
fn divergence_preferred_coding_system() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (coding-system-p preferred-coding-system)
  (coding-system-p default-terminal-coding-system)
  (coding-system-p default-buffer-file-coding-system))"#,
    );
}

#[test]
fn divergence_process_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (listp (process-list))
  (process-list))"#,
    );
}

#[test]
fn divergence_get_buffer_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (processp (get-buffer-process "*scratch*"))
  (null (get-buffer-process "*scratch*"))
  (null (get-process "nonexistent-process-xyz")))"#,
    );
}

#[test]
fn divergence_make_network_process_params() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (featurep 'make-network-process)
  (fboundp 'make-network-process)
  (fboundp 'make-serial-process))"#,
    );
}

#[test]
fn divergence_call_process_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'call-process)
  (fboundp 'call-process-region)
  (fboundp 'start-process)
  (fboundp 'shell-command))"#,
    );
}
