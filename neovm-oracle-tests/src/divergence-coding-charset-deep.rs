//! Divergence tests: coding system conversion, charset mapping deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_coding_system_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((cs-list (coding-system-list)))
  (list (listp cs-list)
        (member 'utf-8 cs-list)
        (member 'latin-1 cs-list)
        (member 'binary cs-list))) "#,
    );
}

#[test]
fn divergence_coding_system_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (coding-system-base 'utf-8)
  (coding-system-base 'utf-8-dos)
  (coding-system-base 'latin-1)
  (coding-system-base 'binary)) "#,
    );
}

#[test]
fn divergence_coding_system_eol() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (coding-system-eol-type 'utf-8)
  (coding-system-eol-type 'utf-8-dos)
  (coding-system-eol-type 'utf-8-unix)) "#,
    );
}

#[test]
fn divergence_coding_system_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (coding-system-p 'utf-8)
  (coding-system-p 'latin-1)
  (coding-system-p 'binary)
  (coding-system-p 'nonexistent-cs)) "#,
    );
}

#[test]
fn divergence_encode_decode_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'encode-coding-region)
  (fboundp 'decode-coding-region)
  (fboundp 'encode-coding-string)
  (fboundp 'decode-coding-string))"#,
    );
}

#[test]
fn divergence_charset_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((cs-list (charset-list)))
  (list (listp cs-list)
        (member 'ascii cs-list)
        (member 'unicode cs-list))) "#,
    );
}

#[test]
fn divergence_charset_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (charsetp 'ascii)
  (charsetp 'unicode)
  (charsetp 'latin)
  (charsetp 'nonexistent)) "#,
    );
}

#[test]
fn divergence_decode_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'decode-char)
  (fboundp 'encode-char)
  (characterp ?A)
  (characterp 128)
  (characterp #x4e2d)) "#,
    );
}

#[test]
fn divergence_prefer_coding_system() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'prefer-coding-system)
  (boundp 'buffer-file-coding-system)
  (boundp 'default-buffer-file-coding-system)
  (boundp 'file-name-coding-system))"#,
    );
}

#[test]
fn divergence_detection_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'detect-coding-region)
  (fboundp 'detect-coding-with-priority)
  (fboundp 'find-operation-coding-system))"#,
    );
}
