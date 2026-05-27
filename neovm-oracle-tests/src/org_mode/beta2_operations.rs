//! Beta-2 strict combo tests for org-mode extreme edge cases.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex table formulas (all types)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_table_formula_sum() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| 2 |\n| 4 |\n| 8 |\n|   |\n#+TBLFM: @>$1=vsum(@<..@>>)")
      (goto-char (point-min))
      (org-table-calc-current-TBLFM)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_formula_multiply() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| 3 | 4 |   |\n#+TBLFM: $3=$1*$2")
      (goto-char (point-min))
      (org-table-calc-current-TBLFM)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_formula_column_sum() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| 1 | 2 |\n| 3 | 4 |\n|   |   |\n#+TBLFM: @3$1=vsum(@1$1..@2$1)::@3$2=vsum(@1$2..@2$2)")
      (goto-char (point-min))
      (org-table-calc-current-TBLFM)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_formula_with_title_row() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| foo |\n|-----|\n|   2 |\n|   4 |\n|   8 |\n|     |\n#+TBLFM: @>$1=vsum(@I..@>>)")
      (goto-char (point-min))
      (org-table-calc-current-TBLFM)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_formula_remote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+NAME: mytable\n| 1 | 2 |\n| 3 | 4 |\n\n|   |   |\n#+TBLFM: $1=remote(mytable,@2$1)::$2=remote(mytable,@2$2)")
      (goto-char (point-min))
      (org-table-calc-current-TBLFM)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex table operations (all types)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_table_align() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "|a|b|\n|c|d|")
      (goto-char (point-min)) (org-table-align)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_insert_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a | b |\n| c | d |")
      (goto-char (point-min)) (org-table-insert-column)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_delete_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a | b | c |\n| d | e | f |")
      (goto-char (point-min)) (forward-char 4) (org-table-delete-column)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_insert_row() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a | b |\n| c | d |")
      (goto-char (point-min)) (org-table-insert-row)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_kill_row() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a | b |\n| c | d |\n| e | f |")
      (goto-char (point-min)) (forward-line 1) (org-table-kill-row)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_move_column_right() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a | b | c |")
      (goto-char (point-min)) (forward-char 2) (org-table-move-column-right)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_move_column_left() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a | b | c |")
      (goto-char (point-min)) (forward-char 6) (org-table-move-column-left)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_move_row_down() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a |\n| b |\n| c |")
      (goto-char (point-min)) (org-table-move-row-down)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_move_row_up() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a |\n| b |\n| c |")
      (goto-char (point-min)) (forward-line 2) (org-table-move-row-up)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| c |\n| a |\n| b |")
      (goto-char (point-min)) (org-table-sort-lines ?a 'string)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_transpose() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a | b |\n| c | d |\n| e | f |")
      (goto-char (point-min)) (org-table-transpose-table-at-point)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_convert_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "a\tb\tc\nd\te\tf")
      (goto-char (point-min))
      (org-table-convert-region (point-min) (point-max))
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (org-table-create "3x2")
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

#[test]
fn beta2_table_get_field() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| a | b |\n| c | d |")
      (goto-char (point-min))
      (list (org-table-get 1 1) (org-table-get 1 2)
            (org-table-get 2 1) (org-table-get 2 2)))))"##,
    );
}

#[test]
fn beta2_table_blank_field() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert "| value |")
      (goto-char (point-min)) (org-table-blank-field)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex table references
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_table_convert_refs_to_an() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-table)
  (list
   (org-table-convert-refs-to-an "@2$1")
   (org-table-convert-refs-to-an "@1$1 = $0")
   (org-table-convert-refs-to-an "$3 = remote(FOO, @@#$2)")))"##,
    );
}

#[test]
fn beta2_table_convert_refs_to_rc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-table)
  (list
   (org-table-convert-refs-to-rc "A2")
   (org-table-convert-refs-to-rc "A1 = $0")
   (org-table-convert-refs-to-rc "C& = remote(FOO, @@#B&)")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex list operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_list_struct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "- item1\n- item2\n  - sub1\n  - sub2\n- item3")
      (goto-char (point-min))
      (length (org-list-struct)))))"##,
    );
}

#[test]
fn beta2_toggle_checkbox() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "- item")
       (goto-char (point-min)) (org-toggle-checkbox) (buffer-string))
     (with-temp-buffer (org-mode) (insert "- [X] item")
       (goto-char (point-min)) (org-toggle-checkbox) (buffer-string)))))"##,
    );
}

#[test]
fn beta2_cycle_list_bullet() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-plain-list-ordered-item-terminator t))
    (list
     (with-temp-buffer (org-mode) (insert "  - item")
       (goto-char (point-min)) (org-cycle-list-bullet) (buffer-string))
     (with-temp-buffer (org-mode) (insert "- item")
       (goto-char (point-min)) (org-cycle-list-bullet "1.") (buffer-string))
     (with-temp-buffer (org-mode) (insert "+ item")
       (goto-char (point-min)) (org-cycle-list-bullet 'previous) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex timer operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_timer_secs_to_hms() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-timer)
  (list
   (org-timer-secs-to-hms 30)
   (org-timer-secs-to-hms 130)
   (org-timer-secs-to-hms 3690)
   (org-timer-secs-to-hms -3690)))"##,
    );
}

#[test]
fn beta2_timer_hms_to_secs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-timer)
  (list
   (org-timer-hms-to-secs "0:00:30")
   (org-timer-hms-to-secs "0:02:10")
   (org-timer-hms-to-secs "1:01:30")))"##,
    );
}

#[test]
fn beta2_timer_fix_incomplete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-timer)
  (list
   (org-timer-fix-incomplete "1:02:03")
   (org-timer-fix-incomplete "02:03")
   (org-timer-fix-incomplete "03")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex duration operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_duration_to_minutes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-duration)
  (list
   (org-duration-to-minutes "1:01")
   (org-duration-to-minutes "1:20:30")
   (org-duration-to-minutes "2h 10min")
   (org-duration-to-minutes "1d 1:02")
   (org-duration-to-minutes "2.5h")
   (org-duration-to-minutes "2")
   (org-duration-to-minutes "")))"##,
    );
}

#[test]
fn beta2_duration_from_minutes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-duration)
  (list
   (let ((org-duration-format 'h:mm)) (org-duration-from-minutes 60))
   (let ((org-duration-format 'h:mm:ss)) (org-duration-from-minutes 61.5))
   (let ((org-duration-format 'h:mm)) (org-duration-from-minutes 61.5))
   (let ((org-duration-format '(("h" . nil) ("min" . nil)))) (org-duration-from-minutes 60))
   (let ((org-duration-format '(("h" . nil) ("min" . t)))) (org-duration-from-minutes 60))
   (let ((org-duration-format '(("h" . t) ("min" . t)))) (org-duration-from-minutes 50))))"##,
    );
}

#[test]
fn beta2_duration_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-duration)
  (list
   (org-duration-p "3:12")
   (org-duration-p "123:12")
   (org-duration-p "1:23:45")
   (org-duration-p "3d 3h 4min")
   (org-duration-p "3d3h4min")
   (org-duration-p "3d 13:35")
   (org-duration-p "2.35h")
   (org-duration-p "2 h")
   (org-duration-p "3::12")
   (org-duration-p "3:2")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex column view
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_columns_compile_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-colview)
  (list
   (org-columns-compile-format "%ITEM")
   (org-columns-compile-format "%ITEM %TODO")
   (org-columns-compile-format "%10ITEM")
   (org-columns-compile-format "%ITEM(some title)")
   (org-columns-compile-format "%ITEM{+}")
   (org-columns-compile-format "%ITEM{+;%.1f}")))"##,
    );
}

#[test]
fn beta2_columns_uncompile_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-colview)
  (list
   (org-columns-uncompile-format '(("ITEM" "ITEM" nil nil nil)))
   (org-columns-uncompile-format '(("ITEM" "ITEM" nil nil nil) ("TODO" "TODO" nil nil nil)))
   (org-columns-uncompile-format '(("ITEM" "ITEM" 10 nil nil)))
   (org-columns-uncompile-format '(("ITEM" "some title" nil nil nil)))
   (org-columns-uncompile-format '(("ITEM" "ITEM" nil "+" nil)))
   (org-columns-uncompile-format '(("ITEM" "ITEM" nil "+" "%.1f")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex macro operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_macro_replace_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-macro)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode)
       (insert "#+MACRO: A B\n1 {{{A}}} 3")
       (goto-char (point-min)) (org-macro-initialize-templates)
       (org-macro-replace-all org-macro-templates) (buffer-string))
     (with-temp-buffer (org-mode)
       (insert "#+MACRO: macro $1 $2\n{{{macro(some,text)}}}")
       (goto-char (point-min)) (org-macro-initialize-templates)
       (org-macro-replace-all org-macro-templates) (buffer-string))
     (with-temp-buffer (org-mode)
       (insert "#+MACRO: in inner\n#+MACRO: out {{{in}}} outer\n{{{out}}}")
       (goto-char (point-min)) (org-macro-initialize-templates)
       (org-macro-replace-all org-macro-templates) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex footnote operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_footnote_new() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-footnote-auto-label t)
        (org-footnote-section nil))
    (list
     (with-temp-buffer (org-mode) (insert "Text")
       (goto-char (point-max)) (org-footnote-new) (buffer-string))
     (with-temp-buffer (org-mode) (insert "Text")
       (goto-char (point-max))
       (let ((org-footnote-auto-label 'anonymous))
         (org-footnote-new)) (buffer-string)))))"##,
    );
}

#[test]
fn beta2_footnote_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-footnote-section nil))
    (list
     (with-temp-buffer (org-mode)
       (insert "Text[fn:1]\n\n[fn:1] Def")
       (goto-char (point-min)) (search-forward "[fn:1]")
       (org-footnote-delete) (org-trim (buffer-string)))
     (with-temp-buffer (org-mode)
       (insert "Para[fn::def]")
       (goto-char (point-min)) (search-forward "[fn::")
       (org-footnote-delete) (org-trim (buffer-string))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex archive operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_archive_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-archive)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Top\n** DONE One\n** TODO Two")
      (goto-char (point-min)) (forward-line 1) (org-archive-subtree)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex datetree operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_datetree_find_date_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-datetree)
  (let ((org-mode-hook nil)
        (org-datetree-add-timestamp nil)
        (org-blank-before-new-entry '((heading . t))))
    (list
     (with-temp-buffer (org-mode)
       (org-datetree-find-date-create '(3 29 2012))
       (org-trim (buffer-string)))
     (with-temp-buffer (org-mode) (insert "* 2012\n")
       (org-datetree-find-date-create '(3 29 2012))
       (org-trim (buffer-string))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex protocol operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_protocol_parse_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-protocol)
  (list
   (let ((data (org-protocol-parse-parameters '(:url "abc" :title "def") nil)))
     (list (plist-get data :url) (plist-get data :title)))
   (let ((data (org-protocol-parse-parameters "url=abc&title=def" t)))
     (list (plist-get data :url) (plist-get data :title)))
   (let ((data (org-protocol-parse-parameters "abc/def" nil '(:url :title))))
     (list (plist-get data :url) (plist-get data :title)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex pcomplete operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_pcomplete_entity() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-pcomplete)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "\\alp")
       (goto-char (point-max)) (pcomplete) (buffer-string))
     (with-temp-buffer (org-mode) (insert "\\frac1")
       (goto-char (point-max)) (pcomplete) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex fold operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_fold_hide_drawer_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-fold)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert ":drawer:\ncontents\n:end:")
       (goto-char (point-min)) (org-fold-show-all)
       (org-fold-hide-drawer-toggle)
       (get-char-property (line-end-position) 'invisible))
     (with-temp-buffer (org-mode) (insert ":drawer:\ncontents\n:end:")
       (goto-char (point-min))
       (org-fold-hide-drawer-toggle)
       (org-fold-hide-drawer-toggle 'off)
       (get-char-property (line-end-position) 'invisible)))))"##,
    );
}

#[test]
fn beta2_fold_hide_block_toggle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-fold)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode)
       (insert "#+BEGIN_CENTER\ncontents\n#+END_CENTER")
       (goto-char (point-min))
       (org-fold-hide-block-toggle)
       (get-char-property (line-end-position) 'invisible))
     (with-temp-buffer (org-mode)
       (insert "#+BEGIN_CENTER\ncontents\n#+END_CENTER")
       (goto-char (point-min))
       (org-fold-hide-block-toggle)
       (org-fold-hide-block-toggle 'off)
       (get-char-property (line-end-position) 'invisible)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex num operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_num_max_level() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-num)
  (let ((org-mode-hook nil)
        (org-num-max-level 2))
    (with-temp-buffer (org-mode) (insert "* H1\n** H2\n*** H3")
      (goto-char (point-min))
      (org-num-mode 1)
      (sort (mapcar (lambda (o) (overlay-get o 'after-string))
                    (overlays-in (point-min) (point-max)))
            #'string-lessp))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex capture operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_capture_fill_template() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-capture)
  (let ((org-store-link-plist nil))
    (list
     (org-capture-fill-template "%(concat \"success\" \"!\")")
     (org-capture-fill-template "%<%Y>")
     (org-capture-fill-template "%t")
     (org-capture-fill-template "%u")
     (org-capture-fill-template "%i" "success!")
     (org-capture-fill-template "\\%i" "success!"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex clock operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_clock_table_data() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Task\n:LOGBOOK:\nCLOCK: [2023-10-13 Fri 10:00]--[2023-10-13 Fri 11:30] =>  1:30\n:END:")
      (goto-char (point-min))
      (car (org-clock-get-table-data (current-buffer) '(:maxlevel 2))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex refile operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_refile_get_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-refile)
  (let ((org-mode-hook nil)
        (org-refile-targets '((nil :maxlevel . 3))))
    (with-temp-buffer (org-mode)
      (insert "* A\n** B\n*** C\n* D\n** E")
      (goto-char (point-min))
      (mapcar (lambda (r) (car r)) (org-refile-get-targets)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex sparse tree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_match_sparse_tree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* TODO A\n* DONE B\n* TODO C\n* DONE D")
      (goto-char (point-min))
      (org-match-sparse-tree nil "TODO")
      (let ((visible nil))
        (org-element-map (org-element-parse-buffer) 'headline
          (lambda (h)
            (let ((title (org-element-property :raw-value h)))
              (when (org-element-property :begin h) (push title visible)))))
        (nreverse visible)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex tag operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_toggle_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-toggle-tag "test") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H :test:")
       (goto-char (point-min)) (org-toggle-tag "test") (buffer-string)))))"##,
    );
}

#[test]
fn beta2_set_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-set-tags '("tag1")) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H :old:")
       (goto-char (point-min)) (org-set-tags '("new")) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-set-tags '("a" "b")) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H :tag:")
       (goto-char (point-min)) (org-set-tags nil) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex todo operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_todo_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-todo-keywords '((sequence "TODO" "DONE"))))
    (list
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-todo 'todo) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* TODO H")
       (goto-char (point-min)) (org-todo 'done) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* DONE H")
       (goto-char (point-min)) (org-todo nil) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex property operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_entry_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert ":PROPERTIES:\n:A: 1\n:END:")
       (goto-char (point-min)) (org-entry-get (point) "A"))
     (with-temp-buffer (org-mode) (insert "* H\n:PROPERTIES:\n:A: 1\n:END:")
       (goto-char (point-min)) (org-entry-get (point) "a"))
     (with-temp-buffer (org-mode) (insert "* H\n:PROPERTIES:\n:A+: 2\n:A: 1\n:END:")
       (goto-char (point-min)) (org-entry-get (point) "A"))
     (with-temp-buffer (org-mode) (insert "* H\n:PROPERTIES:\n:A: nil\n:END:")
       (goto-char (point-min)) (org-entry-get (point) "A"))
     (with-temp-buffer (org-mode) (insert "* H\n:PROPERTIES:\n:A: 1\n:END:\n** H2")
       (goto-char (point-max)) (org-entry-get (point) "A" t)))))"##,
    );
}

#[test]
fn beta2_entry_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-entry-put (point) "TODO" "TODO") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* TODO H")
       (goto-char (point-min)) (org-entry-put (point) "TODO" nil) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* [#B] H")
       (goto-char (point-min)) (org-entry-put (point) "PRIORITY" "A") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H\n:PROPERTIES:\n:A: 1\n:END:")
       (goto-char (point-min)) (org-entry-put (point) "A" "2") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-entry-put (point) "A" "1") (buffer-string)))))"##,
    );
}

#[test]
fn beta2_delete_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert ":PROPERTIES:\n:TEST: t\n:END:")
       (goto-char (point-min)) (org-delete-property "TEST") (buffer-string))
     (with-temp-buffer (org-mode) (insert ":PROPERTIES:\n:T1: t\n:T2: t\n:END:")
       (goto-char (point-min)) (org-delete-property "T2") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H\n:PROPERTIES:\n:TEST: t\n:END:")
       (goto-char (point-min)) (org-delete-property "TEST") (buffer-string)))))"##,
    );
}

#[test]
fn beta2_set_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode)
       (let ((org-property-format "%s %s")) (org-set-property "TEST" "t"))
       (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min))
       (let ((org-adapt-indentation nil) (org-property-format "%s %s"))
         (org-set-property "TEST" "t"))
       (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex planning operations
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_deadline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil) (org-adapt-indentation nil))
    (list
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-deadline nil "<2012-03-29>") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H\nDEADLINE: <2012-03-29>")
       (goto-char (point-min)) (org-deadline nil "<2014-03-04>") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-deadline nil "<2012-03-29 +2y>") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H\nDEADLINE: <2012-03-29>")
       (goto-char (point-min)) (org-deadline '(4)) (buffer-string)))))"##,
    );
}

#[test]
fn beta2_schedule() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil) (org-adapt-indentation nil))
    (list
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-schedule nil "<2012-03-29>") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H\nSCHEDULED: <2012-03-29>")
       (goto-char (point-min)) (org-schedule nil "<2014-03-04>") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-schedule nil "<2012-03-29 +2y>") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H\nSCHEDULED: <2012-03-29>")
       (goto-char (point-min)) (org-schedule '(4)) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex repeat/timestamp
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_get_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "* H\nSCHEDULED: <2023-10-13 Fri +1w>")
       (goto-char (point-min)) (forward-line 1) (org-get-repeat))
     (with-temp-buffer (org-mode) (insert "* H\nSCHEDULED: <2023-10-13 Fri>")
       (goto-char (point-min)) (forward-line 1) (org-get-repeat)))))"##,
    );
}

#[test]
fn beta2_timestamp_has_time_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "<2023-10-13 Fri 14:30>")
       (goto-char (point-min)) (org-at-timestamp-p 'lax) (org-timestamp-has-time-p))
     (with-temp-buffer (org-mode) (insert "<2023-10-13 Fri>")
       (goto-char (point-min)) (org-at-timestamp-p 'lax) (org-timestamp-has-time-p)))))"##,
    );
}

#[test]
fn beta2_at_timestamp_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "<2023-10-13 Fri>")
       (goto-char (point-min)) (org-at-timestamp-p 'lax))
     (with-temp-buffer (org-mode) (insert "[2023-10-13 Fri]")
       (goto-char (point-min)) (org-at-timestamp-p 'lax))
     (with-temp-buffer (org-mode) (insert "Not a timestamp")
       (goto-char (point-min)) (org-at-timestamp-p 'lax)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Beta-2: org-element with complex category
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn beta2_get_category() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "#+CATEGORY: Work\n* H")
       (goto-char (point-min)) (org-get-category))
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-get-category)))))"##,
    );
}
