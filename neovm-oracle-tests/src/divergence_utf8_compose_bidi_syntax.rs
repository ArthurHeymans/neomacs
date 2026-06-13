//! UTF-8 / multibyte *composition, bidi direction & syntax modification* probes.
//!
//! Follows up on the composition divergence (#33): `compose-string`,
//! `decompose-region`, and `find-composition` against a string. Also probes
//! `current-bidi-paragraph-direction` for LTR/RTL text, `read` of multibyte
//! symbols, and `modify-syntax-entry` for non-ASCII characters.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// --- compose-string / decompose-region --------------------------------------

#[test]
fn div_utf8_compose_string_find_composition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((s (copy-sequence "café")))
  (compose-string s 0 (length s) "")
  (list s
        (text-properties-at 0 s)
        (find-composition 0 nil s t)))
"#,
    );
}

#[test]
fn div_utf8_decompose_region_after_compose() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(condition-case err
    (with-temp-buffer
      (insert "abc")
      (compose-region 1 3 "")
      (decompose-region 1 3)
      (list (find-composition 1 nil nil t) (buffer-string)))
  (error (cons (car err) 'errored)))
"#,
    );
}

// --- bidi paragraph direction ----------------------------------------------

#[test]
fn div_utf8_current_bidi_paragraph_direction_ltr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "café 世界 hello")
  (current-bidi-paragraph-direction))
"#,
    );
}

#[test]
fn div_utf8_current_bidi_paragraph_direction_rtl() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (insert "שלום עולם")
  (current-bidi-paragraph-direction))
"#,
    );
}

// --- read of multibyte symbols ----------------------------------------------

#[test]
fn div_utf8_read_multibyte_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (read "café")
      (symbolp (read "café"))
      (symbol-name (read "λ"))
      (eq (read "café") (intern "café"))
      (read " 世界"))
"#,
    );
}

// --- modify-syntax-entry for multibyte --------------------------------------

#[test]
fn div_utf8_modify_syntax_entry_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((st (copy-syntax-table (standard-syntax-table))))
  (modify-syntax-entry ?é "_" st)
  (modify-syntax-entry ?\x3042 "w" st)
  (with-syntax-table st
    (list (char-syntax ?é)
          (char-syntax ?\x3042)
          (char-syntax ?a))))
"#,
    );
}

// --- prin1 of structures containing multibyte strings -----------------------

#[test]
fn div_utf8_prin1_vector_alist_with_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (prin1-to-string ["café" "世界" ?é])
      (prin1-to-string '(("café" . "世界") ("λ" . "λλ")))
      (length (prin1-to-string ["café" "世界"])))
"#,
    );
}
