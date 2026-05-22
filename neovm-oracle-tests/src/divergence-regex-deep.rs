//! Divergence tests: regexp engine deep - backreferences, lookahead, boundaries.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_regex_backreference() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match "\\(ab\\)\\1" "abab")
  (string-match "\\(ab\\)\\1" "abba")
  (match-string 0 "abab"))"#,
    );
}

#[test]
fn divergence_regex_word_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match "\\<hello\\>" "say hello world")
  (string-match "\\<hello\\>" "say helloworld")
  (string-match "\\<hello\\>" "sayhello world"))"#,
    );
}

#[test]
fn divergence_regex_non_greedy() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match "<.*?>" "<a><b><c>")
  (match-string 0 "<a><b><c>")
  (progn
    (string-match "<.*>" "<a><b><c>")
    (match-string 0 "<a><b><c>")))"#,
    );
}

#[test]
fn divergence_regex_char_class_alternation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match "[aeiou]" "xyz")
  (string-match "[aeiou]" "abc")
  (string-match "[^aeiou]" "aei")
  (string-match "[a-z&&[^aeiou]]" "b"))"#,
    );
}

#[test]
fn divergence_regex_multiline() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((s "line1\nline2\nline3"))
  (list
    (string-match "^line2" s)
    (string-match "^line2" s t)
    (string-match "line2$" s)))"#,
    );
}

#[test]
fn divergence_regex_shy_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (string-match "\\(?:ab\\)\\(cd\\)" "abcd")
  (list (match-string 0 "abcd")
        (match-string 1 "abcd")
        (match-string 2 "abcd")))"#,
    );
}

#[test]
fn divergence_regex_named_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (string-match "(?2:ab)(cd)" "abcd")
  (list (match-beginning 0)
        (match-end 0)
        (match-beginning 1)
        (match-end 1)))"#,
    );
}

#[test]
fn divergence_regex_replace_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (insert "aaa bbb aaa bbb aaa")
  (goto-char 1)
  (let ((count 0))
    (while (re-search-forward "aaa" nil t)
      (setq count (1+ count))
      (replace-match "XXX"))
    (list count (buffer-string))))"#,
    );
}

#[test]
fn divergence_regex_unicode_classes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match "[[:alpha:]]+" "hello123")
  (match-string 0 "hello123")
  (string-match "[[:digit:]]+" "hello123")
  (match-string 0 "hello123")
  (string-match "[[:space:]]" "hello world"))"#,
    );
}

#[test]
fn divergence_regex_case_fold_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((case-fold-search t))
  (list (string-match "hello" "HELLO")
        (string-match "hello" "HELLO")))"#,
    );
}

#[test]
fn divergence_regex_syntax_classes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match "\\sw+" "hello world")
  (match-string 0 "hello world")
  (string-match "\\s(" "(foo)")
  (string-match "\\s)" "(foo)"))"#,
    );
}
