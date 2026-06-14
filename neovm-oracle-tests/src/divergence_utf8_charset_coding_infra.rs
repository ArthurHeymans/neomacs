//! UTF-8 / multibyte *charset & coding-system infrastructure* divergence probes.
//!
//! Probes construction APIs (`define-charset`, `make-coding-system`) and
//! metadata accessors (`charset-plist`, `charset-code-space`,
//! `coding-system-aliases`, `coding-system-type`, `charset-chars`, the `block`
//! property). A UTF-8-internal reimpl often lacks the full charset/coding
//! registry machinery, so these are likely divergence points.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_utf8_define_charset_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(condition-case err
    (progn
      (define-charset 'neo-test-charset-1
        "Test charset"
        :dimension 1
        :code-space [0 127]
        :superset 'ascii)
      (list (charset-p 'neo-test-charset-1)
            (charset-dimension 'neo-test-charset-1)))
  (error (list 'errored (car err))))
"#,
    );
}

#[test]
fn div_utf8_make_coding_system_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(condition-case err
    (progn
      (make-coding-system 'neo-cs-1 0 ?T "Test coding system")
      (coding-system-p 'neo-cs-1))
  (error (list 'errored (car err))))
"#,
    );
}

#[test]
fn div_utf8_charset_plist_builtins() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (charset-plist 'ascii)
      (charset-plist 'unicode)
      (charset-plist 'eight-bit))
"#,
    );
}

#[test]
fn div_utf8_charset_code_space_builtins() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (charset-code-space 'ascii)
      (charset-code-space 'unicode)
      (charset-code-space 'japanese-jisx0208))
"#,
    );
}

#[test]
fn div_utf8_coding_system_aliases() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (coding-system-aliases 'utf-8)
      (coding-system-aliases 'latin-1)
      (coding-system-aliases 'iso-8859-1)
      (coding-system-aliases 'emacs-mule))
"#,
    );
}

#[test]
fn div_utf8_charset_chars_counts() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (charset-chars 'ascii)
      (charset-chars 'unicode)
      (charset-chars 'eight-bit))
"#,
    );
}

#[test]
fn div_utf8_block_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (get-char-code-property ?a 'block)
      (get-char-code-property ?\x3042 'block)
      (get-char-code-property ?\x1f600 'block)
      (get-char-code-property ?\x5d0 'block)
      (get-char-code-property ?é 'block))
"#,
    );
}

#[test]
fn div_utf8_coding_system_type_and_mnemonic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (coding-system-type 'utf-8)
      (coding-system-type 'utf-16)
      (coding-system-type 'latin-1)
      (coding-system-mnemonic 'utf-8)
      (coding-system-mnemonic 'latin-1))
"#,
    );
}
