//! Strict combo oracle probes, batch 14: POSIX character classes and regex
//! backrefs/shy-groups/repetition, event and key coding, match-data
//! save/set/restore, replace-with-case-function, time encode/decode
//! extremes, assoc/rassoc/delete variants, and split-string defaults.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_e9_regex_posix_classes_backrefs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-match-p "[[:alpha:]]+" "abc123")
      (string-match-p "[[:digit:]]+" "abc123")
      (string-match-p "[[:space:]]+" "ab cd")
      (string-match-p "\\`\\(?:foo\\|bar\\)" "barbaz")
      (progn (string-match "\\(ab\\)\\1" "abab") (match-end 0))
      (progn (string-match "a\\{2,3\\}" "aaaa") (match-end 0))
      (progn (string-match "\\(?:\\(.\\)\\1\\)" "aa") (match-data t))
      (regexp-opt '("foo" "x\\_<word\\_>")))
"##,
    );
}

#[test]
fn div_e9_event_and_key_coding() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (event-modifiers 'C-S-a)
      (event-basic-type 'C-S-a)
      (event-modifiers ?\M-x)
      (event-basic-type ?\M-x)
      (event-convert-list '(control ?a))
      (event-convert-list '(meta 120))
      (key-description [C-S-a])
      (key-description [M-f5])
      (single-key-description 'f5)
      (event-modifiers 'S-down-mouse-1)
      (event-basic-type 'C-M-return))
"##,
    );
}

#[test]
fn div_e9_match_data_save_set_restore() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "hello world")
  (goto-char 1)
  (re-search-forward "o")
  (let ((md (match-data t)))
    (list md
          (save-match-data
            (string-match "x" "axb")
            (match-data t))
          (progn (set-match-data md) (match-data t)))))
"##,
    );
}

#[test]
fn div_e9_replace_with_case_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (replace-regexp-in-string "\\b\\w"
                                (lambda (_w) (upcase (match-string 0)))
                                "hello world")
      (replace-regexp-in-string "[a-z]+" #'upcase "abc def")
      (with-temp-buffer
        (insert "hello")
        (goto-char 1)
        (re-search-forward "l")
        (replace-match (upcase (match-string 0)) t)))
"##,
    );
}

#[test]
fn div_e9_encode_decode_time_extremes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (encode-time 0 0 12 1 1 1 0)
      (encode-time 0 0 12 1 1 10000 0)
      (decode-time (encode-time 0 0 0 1 1 1970 0) 0)
      (decode-time (encode-time 0 0 0 1 1 2038 0) 0)
      (format-time-string "%Y" (encode-time 0 0 0 1 1 1900 0) 0))
"##,
    );
}

#[test]
fn div_e9_assoc_rassoc_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((al '(("a" . 1) (b . 2) ("a" . 3))))
  (list (assoc "a" al)
        (assq 'b al)
        (rassoc 2 al)
        (rassq 2 al)
        (assoc-default "a" al)
        (assq-delete-all 'b al)))
"##,
    );
}

#[test]
fn div_e9_split_string_defaults() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (split-string "  a  b  c  ")
      (split-string "a,b,,c" ",")
      (split-string "a,b,,c" "," t)
      (split-string-and-unquote "a 'b c' d"))
"##,
    );
}
