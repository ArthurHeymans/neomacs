//! Divergence tests: character folding, unicode normalization, and bidi deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_char_fold_to_regexp_ascii() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((rx (char-fold-to-regexp "a")))
  (list (stringp rx)
        (> (length rx) 1)
        (string-match rx "a")
        (string-match rx "á")))"#,
    );
}

#[test]
fn divergence_char_fold_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((rx (char-fold-to-regexp "ss")))
  (list (stringp rx)
        (string-match rx "ss")
        (string-match rx "ß")))"#,
    );
}

#[test]
fn divergence_unicode_collation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-collate-equalp "hello" "HELLO" nil t)
  (string-collate-equalp "hello" "hello")
  (string-collate-lessp "a" "b"))"#,
    );
}

#[test]
fn divergence_get_unicode_property_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (get-char-code-property ?A 'name)
  (get-char-code-property ?中 'name)
  (get-char-code-property ?a 'old-name)
  (get-char-code-property ?\n 'name))"#,
    );
}

#[test]
fn divergence_unicode_general_categories() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (get-char-code-property ?A 'general-category)
  (get-char-code-property ?a 'general-category)
  (get-char-code-property ?0 'general-category)
  (get-char-code-property ?  'general-category)
  (get-char-code-property ?\n 'general-category)
  (get-char-code-property ?! 'general-category)
  (get-char-code-property ?$ 'general-category))"#,
    );
}

#[test]
fn divergence_decode_big5() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (coding-system-p 'big5)
  (coding-system-p 'euc-jp)
  (coding-system-p 'shift_jis)
  (coding-system-p 'koi8-r))"#,
    );
}

#[test]
fn divergence_coding_system_priority_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((cs (find-coding-systems-string "Hello")))
  (list (consp cs)
        (memq 'utf-8 cs)
        (memq 'raw-text cs)
        (memq 'emacs-mule cs)))"#,
    );
}

#[test]
fn divergence_string_width_cjk() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-width "ABC")
  (string-width "中文")
  (string-width "ABC中文")
  (= (string-width "ABC中文") (+ (string-width "ABC") (string-width "中文"))))"#,
    );
}

#[test]
fn divergence_char_width_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (aref char-width-table ?A)
  (aref char-width-table ?中)
  (aref char-width-table ?a)
  (aref char-width-table ?\n))"#,
    );
}

#[test]
fn divergence_composition_function_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (char-table-p composition-function-table)
  (aref composition-function-table ?a)
  (aref composition-function-table ?é))"#,
    );
}
