//! Complex combo batch 322 — `regexp`/`search`/`match-data`/`replace`
//! ultimate: regexp-quote special chars, regexp-opt variants, shy groups,
//! backreferences, search-forward/backward, re-search, match-data save/restore,
//! replace-match with backref.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx322_regexp_quote_all_special_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(mapcar (lambda (s)
          (let ((q (regexp-quote s)))
            (list s q (string-match q s) (match-string 0 s))))
        '("." "+" "*" "?" "(" ")" "[" "]" "{" "}" "|"
          "^" "$" "\\" "a.b*c?" "12+34"
          "café" "世界" "😀"))
"##,
    )
}

#[test]
fn div_cx322_regexp_opt_with_words_symbols() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((words '("apple" "apricot" "banana" "berry" "cherry")))
  (let ((opt-none (regexp-opt words nil))
        (opt-words (regexp-opt words 'words))
        (opt-symbols (regexp-opt words 'symbols)))
    (list opt-none opt-words opt-symbols
          (string-match opt-none "I love apple pie")
          (match-string 0 "I love apple pie"))))
"##,
    )
}

#[test]
fn div_cx322_search_forward_backward_with_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "aaa 111 bbb 222 ccc 333 ddd")
  (goto-char 1)
  (search-forward "111" nil t)
  (let ((p1 (point)))
    (search-forward "333" nil t)
    (let ((p2 (point)))
      (search-backward "bbb" nil t)
      (list p1 p2 (point) (match-string 0)))))
"##,
    )
}

#[test]
fn div_cx322_re_search_forward_with_groups() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "name:alpha age:42 city:Tokyo name:beta age:30")
  (goto-char 1)
  (re-search-forward "\\(\\w+\\):\\(\\w+\\)")
  (list (match-string 0) (match-string 1) (match-string 2)
        (match-beginning 0) (match-end 0)
        (match-beginning 1) (match-end 1)
        (match-beginning 2) (match-end 2)
        (match-data)))
"##,
    )
}

#[test]
fn div_cx322_match_data_save_restore_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (saved)
  (with-temp-buffer
    (insert "alpha beta gamma")
    (string-match "\\(\\w+\\) \\(\\w+\\) \\(\\w+\\)" (buffer-string))
    (setq saved (match-data))
    (string-match "no-match" "different")
    (set-match-data saved)
    (list (match-data)
          (match-string 1)
          (match-string 2)
          (match-string 3))))
"##,
    )
}

#[test]
fn div_cx322_replace_match_with_backref_in_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "name:alpha age:42 city:Tokyo")
  (goto-char 1)
  (re-search-forward "\\(\\w+\\):\\(\\w+\\)")
  (replace-match "\\2:\\1")
  (buffer-string))
"##,
    )
}

#[test]
fn div_cx322_replace_regexp_in_string_with_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (replace-regexp-in-string "[0-9]+" "#" "abc 123 def 456 ghi 789")
      (replace-regexp-in-string "\\b\\w+\\b" (lambda (m) (upcase m)) "hello world foo")
      (replace-regexp-in-string "\\(\\w+\\) \\(\\w+\\)" "\\2 \\1" "alpha beta gamma delta")
      (replace-regexp-in-string " +" "_" "a  b   c    d"))
"##,
    )
}

#[test]
fn div_cx322_looking_at_chain_then_search() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "First Second Third Fourth")
  (goto-char 1)
  (list (looking-at "[A-Z][a-z]+")
        (match-string 0)
        (re-search-forward "[a-z]+" nil t)
        (match-string 0)
        (re-search-forward "[a-z]+" nil t)
        (match-string 0)))
"##,
    )
}

#[test]
fn div_cx322_word_search_forward_lax_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "the quick brown fox the theory theocracy")
  (goto-char 1)
  (word-search-forward "the" t)
  (let ((first (point)))
    (word-search-forward "the" t)
    (let ((second (point)))
      (list first second))))
"##,
    )
}

#[test]
fn div_cx322_regexp_search_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "name:alpha age:42 city:Tokyo name:beta age:30 end")
  (put-text-property 1 5 'face 'bold)
  (let ((m (set-marker (make-marker) 12))
        (ov (make-overlay 5 20)))
    (overlay-put ov 'face 'region)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 3 45)
    (undo-boundary)
    (goto-char 1)
    (let ((matches
           (cl-loop for i from 0 below 3
                    while (re-search-forward "\\(\\w+\\):\\(\\w+\\)" nil t)
                    collect (list (match-string 1) (match-string 2)))))
      (let ((state (list matches
                         (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (point-min) (point-max)
                         (text-properties-at 1))))
        (undo) (undo)
        (widen)
        (list state (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1)))))
"##,
    )
}
