//! Regex edge-case divergence probes (regex-emacs.c vs neovm-core regex_emacs.rs).
//!
//! Classic regex-engine divergence points: backreferences, shy/explicit-numbered
//! groups, interval operators, word boundaries, case-fold over special casing
//! (ß, Σ/σ), non-greedy quantifiers, match-data subexpressions, and replace-match
//! backrefs with case directives (\u \l \U \L \E) and \,(lisp) eval.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_ar_backreference_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (string-match "\(a\)\1" "xaa") (string-match "\(a\)\1" "xab"))"##,
    );
}

#[test]
fn div_ar_shy_group_numbering() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Shy group (?:ab) does NOT capture; group 1 is "c".
    assert_oracle_parity(
        r##"
(progn (string-match "\(?:ab\)\\(c\\)" "abc")
       (list (match-beginning 1) (match-end 1) (match-beginning 2)))
"##,
    );
}

#[test]
fn div_ar_shy_then_backref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (string-match "\(?:a\)\\(b\\)\1" "abb")
       (match-beginning 1)))
"##,
    );
}

#[test]
fn div_ar_interval_operator() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (string-match "a\\{2,3\\}" "aaaaa")
       (list (match-beginning 0) (match-end 0))))
"##,
    );
}

#[test]
fn div_ar_interval_brace_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (string-match "a\\{2\\}" "aaaa")
       (match-end 0)))
"##,
    );
}

#[test]
fn div_ar_non_greedy_plus() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (string-match "a+?" "aaa")
       (list (match-beginning 0) (match-end 0))))
"##,
    );
}

#[test]
fn div_ar_word_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(string-match "\\bword\\b" "a word here")"##);
}

#[test]
fn div_ar_angle_word_boundaries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-match "\\<foo\\>" "foo bar")
      (string-match "\\<foo\\>" "foobar")
      (progn (string-match "\\<foo\\>" "x foo y") (match-beginning 0)))
"##,
    );
}

#[test]
fn div_ar_case_fold_sigma() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // σ (U+03C3) should match Σ (U+03A3) under case-fold-search.
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (list (string-match "σ" "abcΣdef")
        (string-match "Σ" "abcσdef")))
"##,
    );
}

#[test]
fn div_ar_case_fold_sharp_s() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((case-fold-search t))
  (list (string-match "ß" "STRASSE")
        (string-match "ß" "straße")))
"##,
    );
}

#[test]
fn div_ar_match_data_subexpressions() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (string-match "\\(a\\)\\(b\\)\\(c\\)" "xabc")
       (match-data))
"##,
    );
}

#[test]
fn div_ar_replace_match_backref_swap() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(replace-regexp-in-string "\\(a\\)\\(b\\)" "\\2\\1" "ab cd ab")
"##,
    );
}

#[test]
fn div_ar_replace_match_amp_and_n() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(replace-regexp-in-string "a+" "[\\&]" "aaa bb aaa")
"##,
    );
}

#[test]
fn div_ar_replace_match_upper_directive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(replace-regexp-in-string "\\(a\\)" "\\u\\1" "abc"))
"##,
    );
}

#[test]
fn div_ar_replace_match_upper_all_directive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (replace-regexp-in-string "\\(ab\\)" "\\U\\1" "ab cd")
      (replace-regexp-in-string "\\(AB\\)" "\\L\\1" "AB CD"))
"##,
    );
}

#[test]
fn div_ar_replace_match_lisp_eval() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(replace-regexp-in-string "[0-9]+"
  (lambda (m) (number-to-string (1+ (string-to-number m))))
  "a1b22c333")
"##,
    );
}

#[test]
fn div_ar_looking_back() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "foo bar")
  (goto-char 7)
  (list (looking-back "bar") (looking-back "foo" 3)))
"##,
    );
}

#[test]
fn div_ar_alternation_first_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Emacs regex returns FIRST alternative match (a), not longest (ab).
    assert_oracle_parity(
        r##"
(progn (string-match "a\\|ab" "ab") (match-end 0))
"##,
    );
}

#[test]
fn div_ar_regexp_quote_special() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(regexp-quote ".*+?[](){}^$\\|")"##);
}
