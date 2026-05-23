//! Divergence tests: char-folding, isearch, and occur stubs.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_char_folding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'char-fold-to-regexp)
  (boundp 'char-fold-symmetric)
  (booleanp char-fold-symmetric))"#,
    );
}

#[test]
fn divergence_char_fold_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((regexp (char-fold-to-regexp "e")))
  (list (stringp regexp)
        (string-match regexp "é")
        (string-match regexp "e")
        (string-match regexp "ë")))"#,
    );
}

#[test]
fn divergence_isearch_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'isearch-forward)
  (fboundp 'isearch-backward)
  (fboundp 'isearch-forward-regexp)
  (fboundp 'isearch-backward-regexp))"#,
    );
}

#[test]
fn divergence_isearch_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (booleanp case-fold-search)
  (booleanp search-highlight)
  (booleanp search-invisible)
  (booleanp isearch-lazy-highlight))"#,
    );
}

#[test]
fn divergence_occur_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'occur)
  (fboundp 'multi-occur)
  (fboundp 'how-many)
  (fboundp 'flush-lines)
  (fboundp 'keep-lines))"#,
    );
}

#[test]
fn divergence_keep_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "apple\nbanana\ncherry\napricot\nblueberry")
  (keep-lines "ap")
  (buffer-string))"#,
    );
}

#[test]
fn divergence_flush_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "apple\nbanana\ncherry\napricot\nblueberry")
  (flush-lines "an")
  (buffer-string))"#,
    );
}

#[test]
fn divergence_how_many() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "aaa bbb aaa ccc aaa")
  (how-many "aaa"))"#,
    );
}

#[test]
fn divergence_replace_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (insert "foo bar foo baz foo")
  (goto-char 1)
  (replace-string "foo" "XXX")
  (buffer-string))"#,
    );
}

#[test]
fn divergence_query_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'query-replace)
  (fboundp 'query-replace-regexp)
  (fboundp 'map-query-replace-regexp))"#,
    );
}
