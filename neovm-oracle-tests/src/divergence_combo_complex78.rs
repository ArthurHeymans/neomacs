//! Complex combo batch 78 — abbrev / completion / fill / indent / case
//! region operations: abbrev expansion, completion-styles, `complete-symbol`,
//! `fill-region`, `indent-region`, `upcase-region`/`downcase-region`/`capitalize-region`.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx78_upcase_downcase_capitalize_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (with-temp-buffer (insert "hello world") (upcase-region 1 11) (buffer-string))
 (with-temp-buffer (insert "HELLO WORLD") (downcase-region 1 11) (buffer-string))
 (with-temp-buffer (insert "hello world") (capitalize-region 1 11) (buffer-string))
 (upcase "hello world")
 (downcase "HELLO WORLD")
 (capitalize "hello world")
 (upcase-initials "hello world foo bar"))
"##,
    );
}

#[test]
fn div_cx78_upcase_word_downcase_word_capitalize_word_motion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list
 (with-temp-buffer
   (insert "hello world foo bar")
   (goto-char 1)
   (upcase-word 1)
   (buffer-string))
 (with-temp-buffer
   (insert "HELLO WORLD FOO BAR")
   (goto-char 1)
   (downcase-word 1)
   (buffer-string))
 (with-temp-buffer
   (insert "hello world foo bar")
   (goto-char 1)
   (capitalize-word 2)
   (buffer-string)))
"##,
    );
}

#[test]
fn div_cx78_fill_region_with_fill_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "This is a long line of text that should be wrapped at the fill column boundary for testing purposes.")
  (let ((fill-column 30))
    (fill-region (point-min) (point-max))
    (buffer-string)))
"##,
    );
}

#[test]
fn div_cx78_fill_paragraph_with_fill_prefix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "    This is a paragraph that has a fill prefix applied to it that should also be wrapped properly at the column boundary.")
  (goto-char 1)
  (let ((fill-column 40))
    (fill-paragraph))
  (buffer-string))
"##,
    );
}

#[test]
fn div_cx78_indent_region_with_tab_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-temp-buffer
      (insert "line1\nline2\nline3\n")
      (let ((indent-tabs-mode nil))
        (indent-rigidly (point-min) (point-max) 4)
        (buffer-string)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx78_abbrev_define_and_expand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((table (make-abbrev-table)))
      (define-abbrev table "foo" "forward" nil)
      (define-abbrev table "bar" "backward" nil)
      (list (abbrev-table-p table)
            (abbrev-symbol "foo" table)
            (abbrev-expansion "foo" table)
            (abbrev-expansion "bar" table)
            (abbrev-expansion "missing" table)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx78_completion_styles_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coll '("alpha" "alphabet" "alpine" "beta" "gamma" "delta")))
  (list
   (try-completion "al" coll)
   (try-completion "alph" coll)
   (try-completion "alphabet" coll)
   (try-completion "z" coll)
   (all-completions "al" coll)
   (all-completions "b" coll)
   (test-completion "alpha" coll)
   (test-completion "alp" coll)))
"##,
    );
}

#[test]
fn div_cx78_completion_with_predicates() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coll '("apple1" "apple2" "banana1" "banana2" "cherry1" "cherry2")))
  (list
   (all-completions "a" coll (lambda (s) (string-match-p "1$" s)))
   (all-completions "b" coll (lambda (s) (string-match-p "2$" s)))
   (try-completion "app" coll (lambda (s) (string-match-p "1$" s)))
   (length (all-completions "" coll))))
"##,
    );
}

#[test]
fn div_cx78_completion_with_alist_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coll '(("alpha" . 1) ("alphabet" . 2) ("beta" . 3))))
  (list
   (try-completion "al" coll)
   (all-completions "al" coll)
   (assoc "alpha" coll)
   (assoc "alphabet" coll)))
"##,
    );
}

#[test]
fn div_cx78_indent_to_column_with_tabs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((indent-tabs-mode t)
        (tab-width 4))
    (insert "x")
    (indent-to 12)
    (let ((with-tabs (buffer-string)))
      (erase-buffer)
      (let ((indent-tabs-mode nil))
        (insert "x")
        (indent-to 12))
      (list with-tabs (buffer-string) (current-column)))))
"##,
    );
}

#[test]
fn div_cx78_move_to_column_with_tabs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (let ((tab-width 4))
    (insert "abc\tdef\tghi")
    (goto-char 1)
    (move-to-column 8)
    (let ((p1 (point)))
      (move-to-column 4)
      (let ((p2 (point)))
        (list p1 p2 (current-column) (point))))))
"##,
    );
}

#[test]
fn div_cx78_completion_table_case_insensitive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((coll '("Alpha" "ALPHA" "alpha" "Beta"))
      (completion-ignore-case t))
  (list
   (try-completion "a" coll)
   (try-completion "A" coll)
   (length (all-completions "a" coll))
   (length (all-completions "A" coll))))
"##,
    );
}

#[test]
fn div_cx78_case_region_indent_fill_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (buffer-enable-undo)
  (insert "this is line one\nthis is line two\nthis is line three")
  (put-text-property 1 5 'face 'bold)
  (let ((m (set-marker (make-marker) 20))
        (ov (make-overlay 5 35)))
    (overlay-put ov 'face 'italic)
    (overlay-put ov 'evaporate t)
    (narrow-to-region 3 50)
    (let ((fill-column 15))
      (upcase-region 5 25)
      (fill-region (point-min) (point-max))
      (let ((state (list (buffer-string)
                         (marker-position m)
                         (overlay-start ov) (overlay-end ov)
                         (point-min) (point-max)
                         (text-properties-at 1)))))
        (undo) (undo)
        (widen)
        (list state
              (buffer-string) (marker-position m)
              (overlay-start ov) (overlay-end ov)
              (text-properties-at 1))))))
"##,
    );
}
