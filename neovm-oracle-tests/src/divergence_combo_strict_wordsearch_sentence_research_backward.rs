//! Strict combo oracle probes, batch 74: word-search-forward (whole-word
//! search), sentence motion (forward/backward across . ! ?), and
//! re-search-backward.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_o8_word_search_forward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "find foo bar here\nfoo barbaz\n")
  (goto-char 1)
  (word-search-forward "foo bar")
  (list (match-beginning 0) (match-end 0) (point)))
"##,
        expect_test::expect![[r#""OK (6 13 13)""#]],
    );
}

#[test]
fn div_o8_sentence_motion_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "First sentence.  Second one!  Third?\n")
  (goto-char 1)
  (forward-sentence)
  (let ((p1 (point)))
    (forward-sentence)
    (let ((p2 (point)))
      (backward-sentence)
      (list p1 p2 (point)
            (progn (backward-sentence) (point))))))
"##,
        expect_test::expect![[r#""OK (16 29 18 1)""#]],
    );
}

#[test]
fn div_o8_re_search_backward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "aaa bbb aaa ccc aaa")
  (goto-char (point-max))
  (re-search-backward "aaa")
  (list (match-beginning 0) (match-end 0) (point)
        (progn (re-search-backward "aaa" nil t) (point))))
"##,
        expect_test::expect![[r#""OK (17 20 17 9)""#]],
    );
}

#[test]
fn div_o8_word_search_regexp_lax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "Hello World foo bar\nhello world\n")
  (goto-char 1)
  (list (word-search-forward "hello world")
        (point)
        (word-search-regexp "foo bar")))
"##,
        expect_test::expect![[r#""OK (12 12 \"\\\\<foo\\\\W+bar\\\\>\")""#]],
    );
}
