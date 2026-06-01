//! Divergence tests: coding systems, encoding/decoding, process stubs.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_oracle_parity, assert_oracle_parity_with_env};

#[test]
fn divergence_coding_system_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
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

    assert_oracle_parity(
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

    assert_oracle_parity(
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

    assert_oracle_parity(
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

    assert_oracle_parity(
        r#"(let ((cs (find-coding-systems-string "Hello World")))
  (list (consp cs)
        (member 'utf-8 cs)
        (member 'raw-text cs)))"#,
    );
}

#[test]
fn divergence_preferred_coding_system() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (coding-system-p preferred-coding-system)
  (coding-system-p default-terminal-coding-system)
  (coding-system-p default-buffer-file-coding-system))"#,
    );
}

#[test]
fn divergence_process_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (listp (process-list))
  (process-list))"#,
    );
}

#[test]
fn divergence_get_buffer_process() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (processp (get-buffer-process "*scratch*"))
  (null (get-buffer-process "*scratch*"))
  (null (get-process "nonexistent-process-xyz")))"#,
    );
}

#[test]
fn divergence_make_network_process_params() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (featurep 'make-network-process)
  (fboundp 'make-network-process)
  (fboundp 'make-serial-process))"#,
    );
}

#[test]
fn divergence_make_network_process_invalid_keyword_domains() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((text-quoting-style 'grave))
  (list
   (condition-case err
       (make-network-process :name "np-nowait" :server t :nowait t :service 0)
     (error err))
   (condition-case err
       (make-network-process :name "np-type" :server t :service 0 :type 'bogus)
     (error err))
   (condition-case err
       (make-network-process :name "np-family" :server t :service 0 :family 'bogus)
     (error err))))"#,
    );
}

#[test]
fn divergence_num_processors_openmp_environment() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (integerp (num-processors))
  (> (num-processors) 0)
  (integerp (num-processors 'current))
  (> (num-processors 'current) 0)
  (integerp (num-processors 'all))
  (> (num-processors 'all) 0)
  (equal (num-processors 'bogus) (num-processors t)))"#,
    );
    assert_oracle_parity_with_env(
        r#"(num-processors)"#,
        &[("OMP_NUM_THREADS", "3"), ("OMP_THREAD_LIMIT", "0")],
    );
    assert_oracle_parity_with_env(
        r#"(num-processors)"#,
        &[("OMP_NUM_THREADS", "3"), ("OMP_THREAD_LIMIT", "2")],
    );
    assert_oracle_parity_with_env(
        r#"(num-processors)"#,
        &[("OMP_NUM_THREADS", " 4,8"), ("OMP_THREAD_LIMIT", "0")],
    );
    assert_oracle_parity_with_env(
        r#"(num-processors)"#,
        &[("OMP_NUM_THREADS", "0"), ("OMP_THREAD_LIMIT", "1")],
    );
}

#[test]
fn divergence_call_process_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'call-process)
  (fboundp 'call-process-region)
  (fboundp 'start-process)
  (fboundp 'shell-command))"#,
    );
}
