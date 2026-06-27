//! Strict combo oracle probes, batch 103: regex MATCHING semantics on edge
//! cases — \\w on CJK, \\s-/\\s. shorthands, \\ca char-class shorthand,
//! dot-not-newline, word-boundary on multibyte.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_r7_regex_matching_semantics_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r####"
(list (string-match-p "a*" "text")
      (string-match-p "^$" "")
      (string-match-p "\\w+" "hello")
      (string-match-p "\\w+" "日本語")
      (string-match-p "\\W+" "!!!")
      (string-match-p "\\s-" "  ")
      (string-match-p "\\s." "...")
      (string-match-p "\\ca" "abc")
      (string-match-p "\\bword\\b" "a word here"))
"####,
    );
}

#[test]
fn div_r7_regex_syntax_classes_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r####"
(list (string-match-p "\\sw" "café")
      (string-match-p "\\sw" "日本")
      (string-match-p "\\s_" "foo_bar")
      (string-match-p "\\s(" "(")
      (string-match-p "\\s)" ")"))
"####,
    );
}

#[test]
fn div_r7_regex_greedy_vs_lazy_matching() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r####"
(let ((s "<a><b><c>"))
  (list (and (string-match "<.+>" s) (match-string 0 s))
        (and (string-match "<.+?>" s) (match-string 0 s))
        (and (string-match "<[^>]+>" s) (match-string 0 s))
        (and (string-match "<\\(.+?\\)>" s) (match-string 1 s))))
"####,
    );
}
