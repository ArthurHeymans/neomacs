//! Strong org-mode oracle tests — multi-step editing sequences.
//!
//! These tests perform sequences of editing operations and compare
//! the final buffer content, point position, or computed values.
//! Multi-step tests are the strongest way to catch implementation
//! divergences because any difference in any intermediate step
//! propagates to the final result.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: insert heading then promote then add body
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_insert_promote_body() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1\n** H2")
      (goto-char (point-max))
      (org-insert-heading)
      (insert "New heading")
      (org-promote)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: insert then edit headline then add tags
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_edit_headline_add_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Old")
      (goto-char (point-min))
      (org-edit-headline "New")
      (org-set-tags '("tag1" "tag2"))
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: set property then get it
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_set_then_get_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H")
      (goto-char (point-min))
      (org-entry-put (point) "MYPROP" "myval")
      (list (org-entry-get (point) "MYPROP")
            (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: set deadline then schedule then get planning
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_deadline_then_schedule() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil) (org-adapt-indentation nil))
    (with-temp-buffer (org-mode)
      (insert "* H")
      (goto-char (point-min))
      (org-deadline nil "<2024-01-15 Mon>")
      (org-schedule nil "<2024-01-14 Sun>")
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: toggle checkbox then check buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_toggle_checkbox_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "- item1\n- item2\n- item3")
      (goto-char (point-min))
      (org-toggle-checkbox)
      (forward-line 1)
      (org-toggle-checkbox)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: navigate and read properties at each position
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_navigate_and_read() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* TODO [#A] H1 :tag1:\nBody1\n* DONE [#B] H2 :tag2:\nBody2")
      (goto-char (point-min))
      (let ((r1 (list (org-get-heading t t nil t)
                      (org-entry-get (point) "TODO")
                      (org-get-tags-at))))
        (org-next-visible-heading 1)
        (let ((r2 (list (org-get-heading t t nil t)
                        (org-entry-get (point) "TODO")
                        (org-get-tags-at))))
          (list r1 r2))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: clock in then clock out then get duration
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_in_out_duration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Task\nBody")
      (goto-char (point-min))
      (org-clock-in)
      (org-clock-out)
      (let* ((tree (org-element-parse-buffer))
             (clock (car (org-element-map tree 'clock #'identity))))
        (list (org-element-property :status clock)
              (org-element-property :duration clock)
              (buffer-string))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: archive then check remaining
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_archive_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-archive)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* Keep\n* Archive Me\nBody\n* Also Keep")
      (goto-char (point-min))
      (forward-line 1)
      (org-archive-subtree)
      (list (buffer-string)
            (org-element-map (org-element-parse-buffer) 'headline
              (lambda (h) (substring-no-properties (org-element-property :raw-value h))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: sparse tree then check visible
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sparse_tree_then_check() {
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
          (lambda (h) (let ((title (org-element-property :raw-value h)))
                   (when (org-element-property :begin h) (push title visible)))))
        (nreverse visible)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: fill then check buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_fill_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "|a|b|\n|c|d|")
      (goto-char (point-min))
      (org-fill-element)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: table formula then check result
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_formula_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| 10 | 20 |   |\n| 30 | 40 |   |\n|    |    |   |\n#+TBLFM: @1$3=$1+$2::@2$3=$1+$2::@3$1=vsum(@1$1..@2$1)::@3$2=vsum(@1$2..@2$2)::@3$3=vsum(@1$3..@2$3)")
      (goto-char (point-min))
      (org-table-calc-current-TBLFM)
      (buffer-substring-no-properties (point-min) (point-max)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: table transpose then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_transpose_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| a | b | c |\n| 1 | 2 | 3 |")
      (goto-char (point-min))
      (org-table-transpose-table-at-point)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: sort table then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_table_sort_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "| c |\n| a |\n| b |")
      (goto-char (point-min))
      (org-table-sort-lines ?a 'string)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: macro expansion then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_macro_expand_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-macro)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+MACRO: greet Hello\n#+MACRO: name World\n{{{greet}}} {{{name}}}!")
      (goto-char (point-min))
      (org-macro-initialize-templates)
      (org-macro-replace-all org-macro-templates)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: cycle todo then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_todo_cycle_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-todo-keywords '((sequence "TODO" "DONE"))))
    (with-temp-buffer (org-mode)
      (insert "* H")
      (goto-char (point-min))
      (org-todo 'todo)
      (let ((after-todo (buffer-string)))
        (org-todo 'done)
        (let ((after-done (buffer-string)))
          (org-todo nil)
          (list after-todo after-done (buffer-string))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: sort entries then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sort_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\n* def\n* xyz\n* abc")
      (goto-char (point-min))
      (org-sort-entries nil ?a)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: move subtree then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_move_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* A\nBody\n* B\nBody\n* C\nBody")
      (goto-char (point-min))
      (org-move-subtree 1)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: promote/demote subtree then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_promote_demote_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1\n** S1\n** S2")
      (goto-char (point-min))
      (org-demote-subtree)
      (let ((after-demote (buffer-string)))
        (org-promote-subtree)
        (list after-demote (buffer-string))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: cycle list bullet then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_cycle_bullet_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil) (org-plain-list-ordered-item-terminator t))
    (with-temp-buffer (org-mode)
      (insert "- item")
      (goto-char (point-min))
      (org-cycle-list-bullet)
      (let ((after1 (buffer-string)))
        (org-cycle-list-bullet)
        (let ((after2 (buffer-string)))
          (org-cycle-list-bullet)
          (list after1 after2 (buffer-string))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: fold/unfold then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_fold_unfold_drawer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-fold)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert ":drawer:\ncontents\n:end:")
      (goto-char (point-min))
      (org-fold-show-all)
      (org-fold-hide-drawer-toggle)
      (let ((hidden (get-char-property (line-end-position) 'invisible)))
        (org-fold-hide-drawer-toggle 'off)
        (let ((shown (get-char-property (line-end-position) 'invisible)))
          (list hidden shown))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: fold/unfold block then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_fold_unfold_block() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-fold)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_CENTER\ncontents\n#+END_CENTER")
      (goto-char (point-min))
      (org-fold-hide-block-toggle)
      (let ((hidden (get-char-property (line-end-position) 'invisible)))
        (org-fold-hide-block-toggle 'off)
        (let ((shown (get-char-property (line-end-position) 'invisible)))
          (list hidden shown))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: indent then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_indent_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\nA")
      (goto-char (point-max))
      (let ((org-adapt-indentation t))
        (org-indent-line)
        (let ((indent (org-get-indentation)))
          (org-indent-line)
          (list indent (org-get-indentation) (buffer-string)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: return key then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_return_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Para graph")
      (goto-char (+ 4 (point-min)))
      (org-return)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: kill-line then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_kill_line_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "abc\n123")
      (goto-char (point-min))
      (org-kill-line)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: footnote new then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_footnote_new_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil) (org-footnote-auto-label t) (org-footnote-section nil))
    (with-temp-buffer (org-mode)
      (insert "Text")
      (goto-char (point-max))
      (org-footnote-new)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: footnote delete then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_footnote_delete_then_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil) (org-footnote-section nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:1]\n\n[fn:1] Def")
      (goto-char (point-min))
      (search-forward "[fn:1]")
      (org-footnote-delete)
      (org-trim (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: timer operations then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_timer_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-timer)
  (list
   (org-timer-hms-to-secs (org-timer-secs-to-hms 3690))
   (org-timer-hms-to-secs (org-timer-secs-to-hms 130))
   (org-timer-hms-to-secs (org-timer-secs-to-hms 30))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: duration roundtrip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_duration_roundtrip() {
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
   (org-duration-to-minutes "")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: colview format roundtrip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_colview_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-colview)
  (list
   (org-columns-uncompile-format (org-columns-compile-format "%ITEM"))
   (org-columns-uncompile-format (org-columns-compile-format "%ITEM %TODO"))
   (org-columns-uncompile-format (org-columns-compile-format "%10ITEM"))
   (org-columns-uncompile-format (org-columns-compile-format "%ITEM{+}"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: protocol parse then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_protocol_parse_roundtrip_v3() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-protocol)
  (list
   (let ((d (org-protocol-parse-parameters '(:url "abc" :title "def") nil)))
     (list (plist-get d :url) (plist-get d :title)))
   (let ((d (org-protocol-parse-parameters "url=abc&title=def" t)))
     (list (plist-get d :url) (plist-get d :title)))
   (let ((d (org-protocol-parse-parameters "abc/def" nil '(:url :title))))
     (list (plist-get d :url) (plist-get d :title)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: capture template then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_capture_template_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-capture)
  (let ((org-store-link-plist nil))
    (list
     (org-capture-fill-template "%(concat \"success\" \"!\")")
     (org-capture-fill-template "%<%Y>")
     (org-capture-fill-template "%i" "hello"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: clock table data then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_table_roundtrip() {
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
// Multi-step: refile targets then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_refile_targets_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-refile)
  (let ((org-mode-hook nil)
        (org-refile-targets '((nil :maxlevel . 2))))
    (with-temp-buffer (org-mode)
      (insert "* A\n** B\n* C\n** D")
      (goto-char (point-min))
      (mapcar (lambda (r) (car r)) (org-refile-get-targets)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: pcomplete then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_pcomplete_roundtrip() {
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
// Multi-step: num mode overlays then check
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_num_mode_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-num)
  (let ((org-mode-hook nil) (org-num-max-level 2))
    (with-temp-buffer (org-mode)
      (insert "* H1\n** H2\n*** H3")
      (goto-char (point-min))
      (org-num-mode 1)
      (sort (mapcar (lambda (o) (overlay-get o 'after-string))
                    (overlays-in (point-min) (point-max)))
            #'string-lessp))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: cut and paste subtree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_cut_paste_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* A\nBody A\n* B\nBody B\n* C\nBody C")
      (goto-char (point-min))
      (org-cut-subtree)
      (goto-char (point-max))
      (org-paste-subtree 1)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: clone subtree with time shift
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clone_subtree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1\n<2015-06-21>")
      (goto-char (point-min))
      (org-clone-subtree-with-time-shift 1 "+2d")
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: insert-todo-heading-respect-content
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_insert_todo_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1\n Body")
      (org-insert-todo-heading-respect-content)
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: timer change times
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_timer_change_times() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-timer)
  (list
   (with-temp-buffer (org-mode)
     (insert "\n0:00:25\n2:30:05")
     (org-timer-change-times-in-region (point-min) (point-max) "1:30:50")
     (buffer-string))
   (with-temp-buffer (org-mode)
     (insert "\n0:00:25\n2:30:05")
     (org-timer-change-times-in-region (point-min) (point-max) "-1:30:50")
     (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: set then delete property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_set_delete_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H")
      (goto-char (point-min))
      (org-entry-put (point) "MYPROP" "myval")
      (let ((result1 (buffer-string)))
        (org-delete-property "MYPROP")
        (list result1 (buffer-string))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: deadline and schedule combined
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_deadline_schedule_combined() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil) (org-adapt-indentation nil))
    (with-temp-buffer (org-mode)
      (insert "* H")
      (goto-char (point-min))
      (org-deadline nil "<2024-01-15 Mon>")
      (org-schedule nil "<2024-01-14 Sun>")
      (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: todo cycle through states
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_todo_cycle_through() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-todo-keywords '((sequence "TODO" "DONE"))))
    (with-temp-buffer (org-mode)
      (insert "* H")
      (goto-char (point-min))
      (org-todo 'todo)
      (let ((s1 (buffer-string)))
        (org-todo 'done)
        (let ((s2 (buffer-string)))
          (org-todo nil)
          (list s1 s2 (buffer-string))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: sort entries various types
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_sort_entries_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "\n* def\n* xyz\n* abc")
       (goto-char (point-min)) (org-sort-entries nil ?a) (buffer-string))
     (with-temp-buffer (org-mode) (insert "\n* 10\n* 1\n* 2")
       (goto-char (point-min)) (org-sort-entries nil ?n) (buffer-string))
     (with-temp-buffer (org-mode) (insert "\n* [#C] h1\n* [#A] h2\n* [#B] h3")
       (goto-char (point-min)) (org-sort-entries nil ?p) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: move subtree up and down
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_move_subtree_up_down() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "* A\nBody\n* B\nBody\n* C\nBody")
       (goto-char (point-min)) (org-move-subtree 1) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* A\nBody\n* B\nBody\n* C\nBody")
       (goto-char (point-min)) (forward-line 2) (org-move-subtree -1)
       (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: promote/demote subtree roundtrip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_promote_demote_subtree_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1\n** S1\n** S2")
      (goto-char (point-min))
      (org-demote-subtree)
      (let ((after-demote (buffer-string)))
        (org-promote-subtree)
        (list after-demote (buffer-string))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: cycle list bullet various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_cycle_list_bullet_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil) (org-plain-list-ordered-item-terminator t))
    (with-temp-buffer (org-mode)
      (insert "- item")
      (goto-char (point-min))
      (org-cycle-list-bullet)
      (let ((s1 (buffer-string)))
        (org-cycle-list-bullet)
        (let ((s2 (buffer-string)))
          (org-cycle-list-bullet)
          (list s1 s2 (buffer-string))))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: macro replace all
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_macro_replace_all_various() {
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
       (insert "#+MACRO: m $1 $2\n{{{m(a,b)}}}")
       (goto-char (point-min)) (org-macro-initialize-templates)
       (org-macro-replace-all org-macro-templates) (buffer-string))
     (with-temp-buffer (org-mode)
       (insert "#+MACRO: in inner\n#+MACRO: out {{{in}}} outer\n{{{out}}}")
       (goto-char (point-min)) (org-macro-initialize-templates)
       (org-macro-replace-all org-macro-templates) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: footnote new and delete cycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_footnote_new_delete_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil) (org-footnote-auto-label t) (org-footnote-section nil))
    (with-temp-buffer (org-mode)
      (insert "Text")
      (goto-char (point-max))
      (org-footnote-new)
      (let ((after-new (buffer-string)))
        (goto-char (point-min))
        (search-forward "[fn:")
        (backward-char 4)
        (org-footnote-delete)
        (list after-new (org-trim (buffer-string)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: fill element various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_fill_element_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "|a|")
       (goto-char (point-min)) (org-fill-element) (buffer-string))
     (with-temp-buffer (org-mode) (insert "A\nB")
       (goto-char (point-max)) (let ((fill-column 20)) (org-fill-element)) (buffer-string))
     (with-temp-buffer (org-mode) (insert "- A\n  B")
       (goto-char (point-min)) (let ((fill-column 20)) (org-fill-element)) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: fold drawer toggle cycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_fold_drawer_toggle_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-fold)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode) (insert ":drawer:\ncontents\n:end:")
      (goto-char (point-min))
      (org-fold-show-all)
      (org-fold-hide-drawer-toggle)
      (let ((h (get-char-property (line-end-position) 'invisible)))
        (org-fold-hide-drawer-toggle 'off)
        (list h (get-char-property (line-end-position) 'invisible))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: fold block toggle cycle
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_fold_block_toggle_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-fold)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+BEGIN_CENTER\ncontents\n#+END_CENTER")
      (goto-char (point-min))
      (org-fold-hide-block-toggle)
      (let ((h (get-char-property (line-end-position) 'invisible)))
        (org-fold-hide-block-toggle 'off)
        (list h (get-char-property (line-end-position) 'invisible))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: indent line various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_indent_line_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-indent-line) (org-get-indentation))
     (with-temp-buffer (org-mode) (insert "* H\nA")
       (goto-char (point-max)) (let ((org-adapt-indentation t)) (org-indent-line)) (org-get-indentation))
     (with-temp-buffer (org-mode) (insert "* H\nA")
       (goto-char (point-max)) (let ((org-adapt-indentation nil)) (org-indent-line)) (org-get-indentation)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: return various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_return_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "Para graph")
       (goto-char (+ 4 (point-min))) (org-return) (buffer-string))
     (with-temp-buffer (org-mode) (insert "  Para graph")
       (goto-char (+ 6 (point-min))) (org-return t) (buffer-string))
     (with-temp-buffer (org-mode) (insert "| a |\n| b |")
       (goto-char (point-min)) (forward-char 2) (org-return) (looking-at "b")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: meta-return various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_meta_return_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "a")
       (goto-char (point-min)) (org-meta-return) (buffer-string))
     (with-temp-buffer (org-mode) (insert "- a")
       (goto-char (point-min)) (org-meta-return) (buffer-string))
     (with-temp-buffer (org-mode) (insert "| a |")
       (goto-char (point-min)) (forward-char 2) (org-meta-return) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: kill-line various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_kill_line_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "abc")
       (goto-char (point-min)) (org-kill-line) (buffer-string))
     (with-temp-buffer (org-mode) (insert "abc")
       (goto-char (+ 2 (point-min))) (org-kill-line) (buffer-string))
     (with-temp-buffer (org-mode) (insert "abc\n123")
       (goto-char (point-min)) (org-kill-line) (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: edit-headline various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_edit_headline_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "* A")
       (goto-char (point-min)) (org-edit-headline "B") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* TODO A")
       (goto-char (point-min)) (org-edit-headline "B") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* [#A] A")
       (goto-char (point-min)) (org-edit-headline "B") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* A :tag:")
       (goto-char (point-min)) (let ((org-tags-column 4)) (org-edit-headline "B")) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* ")
       (goto-char (point-min)) (org-edit-headline "A") (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: insert-heading various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_insert_heading_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (org-insert-heading) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-insert-heading) (buffer-string))
     (with-temp-buffer (org-mode) (insert "** H\nP")
       (goto-char (point-max)) (org-insert-heading) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H1")
       (goto-char (point-min))
       (let ((org-blank-before-new-entry '((heading . t)))) (org-insert-heading))
       (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: toggle-tag various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_toggle_tag_various() {
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

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: set-tags various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_set_tags_various() {
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
// Multi-step: entry-get various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_entry_get_various() {
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
     (with-temp-buffer (org-mode) (insert "* H\n:PROPERTIES:\n:A: 1\n:END:\n** H2")
       (goto-char (point-max)) (org-entry-get (point) "A" t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: entry-put various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_entry_put_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-entry-put (point) "A" "1") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H\n:PROPERTIES:\n:A: 1\n:END:")
       (goto-char (point-min)) (org-entry-put (point) "A" "2") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* H")
       (goto-char (point-min)) (org-entry-put (point) "TODO" "TODO") (buffer-string))
     (with-temp-buffer (org-mode) (insert "* TODO H")
       (goto-char (point-min)) (org-entry-put (point) "TODO" nil) (buffer-string))
     (with-temp-buffer (org-mode) (insert "* [#B] H")
       (goto-char (point-min)) (org-entry-put (point) "PRIORITY" "A") (buffer-string)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: delete-property various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_delete_property_various() {
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

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: set-property various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_set_property_various() {
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
// Multi-step: deadline various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_deadline_various() {
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

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: schedule various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_schedule_various() {
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
// Multi-step: get-repeat various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_get_repeat_various() {
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

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: timestamp-has-time-p various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_timestamp_has_time_p_various() {
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

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: at-timestamp-p various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_at_timestamp_p_various() {
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
// Multi-step: get-category various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_get_category_various() {
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

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: clock-get-table-data
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_clock_get_table_data() {
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
// Multi-step: refile-get-targets
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_refile_get_targets() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-refile)
  (let ((org-mode-hook nil)
        (org-refile-targets '((nil :maxlevel . 2))))
    (with-temp-buffer (org-mode)
      (insert "* A\n** B\n* C\n** D")
      (goto-char (point-min))
      (mapcar (lambda (r) (car r)) (org-refile-get-targets)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: match-sparse-tree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_match_sparse_tree() {
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
          (lambda (h) (let ((title (org-element-property :raw-value h)))
                   (when (org-element-property :begin h) (push title visible)))))
        (nreverse visible)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: map-entries various matchers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_map_entries_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "* Level 1\n** Level 2")
       (goto-char (point-min)) (org-map-entries #'point))
     (with-temp-buffer (org-mode) (insert "* Level 1\n** Level 2")
       (goto-char (point-min)) (let (org-odd-levels-only) (org-map-entries #'point "LEVEL=1")))
     (with-temp-buffer (org-mode) (insert "* H1\n* TODO H2\n* DONE H3")
       (goto-char (point-min)) (org-map-entries #'point "TODO=\"TODO\""))
     (with-temp-buffer (org-mode) (insert "* H1 :no:\n* H2 :yes:")
       (goto-char (point-min)) (org-map-entries #'point "yes"))
     (with-temp-buffer (org-mode) (insert "* [#A] H1\n* [#B] H2")
       (goto-char (point-min)) (org-map-entries #'point "PRIORITY=\"A\""))
     (with-temp-buffer (org-mode)
       (insert "* H1\n:PROPERTIES:\n:TEST: 1\n:END:\n* H2\n:PROPERTIES:\n:TEST: 2\n:END:")
       (goto-char (point-min)) (org-map-entries #'point "TEST=1")))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: entry-blocked-p various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_entry_blocked_p_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (org-enforce-todo-dependencies t)
        (org-blocker-hook '(org-block-todo-from-children-or-siblings-or-parent)))
    (list
     (with-temp-buffer (org-mode) (insert "* TODO Blocked\n** DONE one\n** TODO two")
       (goto-char (point-min)) (org-entry-blocked-p))
     (with-temp-buffer (org-mode) (insert "* TODO Blocked\n** DONE one\n** DONE two")
       (goto-char (point-min)) (org-entry-blocked-p))
     (with-temp-buffer (org-mode) (insert "* Blocked\n** TODO one")
       (goto-char (point-min)) (org-entry-blocked-p))
     (with-temp-buffer (org-mode) (insert "* DONE Blocked\n** TODO one")
       (goto-char (point-min)) (org-entry-blocked-p)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: find-olp various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_find_olp_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "\n* Headline\n** COMMENT headline2\n** TODO headline3\n*** [#A] headline4 :tags:\n** [#A]headline5\n** [0%] headline6\n** headline7 [100%]\n** headline8 [1/5] :some:more:tags:\n* Test")
      (goto-char (point-min))
      (list
       (org-find-olp '("Headline") t)
       (org-find-olp '("Headline" "headline2") t)
       (org-find-olp '("Headline" "headline3") t)
       (org-find-olp '("Headline" "headline3" "headline4") t)
       (org-find-olp '("Headline" "headline6") t)
       (org-find-olp '("Headline" "headline7") t)
       (org-find-olp '("Headline" "headline8") t)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: timer roundtrip various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_timer_roundtrip_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-timer)
  (list
   (org-timer-secs-to-hms 30)
   (org-timer-secs-to-hms 130)
   (org-timer-secs-to-hms 3690)
   (org-timer-secs-to-hms -3690)
   (org-timer-hms-to-secs (org-timer-secs-to-hms 30))
   (org-timer-hms-to-secs (org-timer-secs-to-hms 130))
   (org-timer-hms-to-secs (org-timer-secs-to-hms 3690))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: duration conversions
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_duration_conversions() {
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
   (org-duration-to-minutes "")
   (let ((org-duration-format 'h:mm)) (org-duration-from-minutes 60))
   (let ((org-duration-format 'h:mm:ss)) (org-duration-from-minutes 61.5))
   (org-duration-p "3:12")
   (org-duration-p "3d 3h 4min")
   (org-duration-p "3::12")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: colview format roundtrip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_colview_format_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-colview)
  (list
   (org-columns-uncompile-format (org-columns-compile-format "%ITEM"))
   (org-columns-uncompile-format (org-columns-compile-format "%ITEM %TODO"))
   (org-columns-uncompile-format (org-columns-compile-format "%10ITEM"))
   (org-columns-uncompile-format (org-columns-compile-format "%ITEM{+}"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: protocol parse roundtrip
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_protocol_parse_roundtrip_v2() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-protocol)
  (list
   (let ((d (org-protocol-parse-parameters '(:url "abc" :title "def") nil)))
     (list (plist-get d :url) (plist-get d :title)))
   (let ((d (org-protocol-parse-parameters "url=abc&title=def" t)))
     (list (plist-get d :url) (plist-get d :title)))
   (let ((d (org-protocol-parse-parameters "abc/def" nil '(:url :title))))
     (list (plist-get d :url) (plist-get d :title)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: capture template expansion
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_capture_template_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-capture)
  (let ((org-store-link-plist nil))
    (list
     (org-capture-fill-template "%(concat \"success\" \"!\")")
     (org-capture-fill-template "%<%Y>")
     (org-capture-fill-template "%i" "hello"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: pcomplete entity
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_pcomplete_entity() {
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
// Multi-step: num mode overlays
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_num_mode_overlays() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-num)
  (let ((org-mode-hook nil) (org-num-max-level 2))
    (with-temp-buffer (org-mode)
      (insert "* H1\n** H2\n*** H3")
      (goto-char (point-min))
      (org-num-mode 1)
      (sort (mapcar (lambda (o) (overlay-get o 'after-string))
                    (overlays-in (point-min) (point-max)))
            #'string-lessp))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: outline path various
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_outline_path_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode) (insert "* H") (goto-char (point-min)) (org-get-outline-path))
     (with-temp-buffer (org-mode) (insert "* H\n** S") (goto-char (point-max)) (org-get-outline-path))
     (with-temp-buffer (org-mode) (insert "* H\n** S\nText") (goto-char (point-max)) (org-get-outline-path))
     (with-temp-buffer (org-mode) (insert "* H") (goto-char (point-min)) (org-get-outline-path t))
     (org-format-outline-path (list "one" "two" "three"))
     (org-format-outline-path '())
     (org-format-outline-path '() nil ">>")
     (org-format-outline-path (list "one" "two" "three") nil ">>" "|")
     (org-format-outline-path (list "one" "two" "three" "four") 10))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: export headline numbers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_headline_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+OPTIONS: num:t H:3\n* Ch1\n** S1\n*** SS1\n** S2\n* Ch2\n** S3")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties tree (org-export-get-environment)))))
        (mapcar (lambda (h) (list (org-export-get-headline-number h info)
                            (org-export-get-relative-level h info)))
                (org-element-map tree 'headline #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: export footnote numbers
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_footnote_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "Text[fn:1] more[fn:2]\n\n[fn:1] Def 1\n[fn:2] Def 2")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties tree (org-export-get-environment)))))
        (list
         (mapcar (lambda (ref) (org-export-get-footnote-number ref info))
                 (org-element-map tree 'footnote-reference #'identity))
         (mapcar (lambda (ref) (org-export-footnote-first-reference-p ref info))
                 (org-element-map tree 'footnote-reference #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: export tags and categories
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_tags_categories() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+CATEGORY: work\n* H1 :tag1:\n** H2 :tag2:\n* H3")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties tree (org-export-get-environment)))))
        (list
         (mapcar (lambda (h) (org-export-get-tags h info))
                 (org-element-map tree 'headline #'identity))
         (mapcar (lambda (h) (org-export-get-category h info))
                 (org-element-map tree 'headline #'identity))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: export sibling detection
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_sibling_detection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H1\n** H2\n** H3\n** H4\n* H5")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (hls (org-element-map tree 'headline #'identity)))
        (list (mapcar #'org-export-first-sibling-p hls)
              (mapcar #'org-export-last-sibling-p hls))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: export filter chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_filter_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (list
   (org-export-filter-apply-functions
    (list (lambda (v &rest _) (concat "1" v))
          (lambda (v &rest _) (concat "2" v)))
    "0" nil)
   (org-export-filter-apply-functions
    (list #'ignore (lambda (v &rest _) (concat "2" v)))
    "0" nil)
   (org-export-filter-apply-functions (list #'ignore) "0" nil)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: export backend chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_backend_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let (org-export-registered-backends)
    (org-export-define-backend 'parent
      '((headline . (lambda (h c i) (format "P: %s" (org-element-property :raw-value h))))
        (section . (lambda (s c i) c))
        (paragraph . (lambda (p c i) c))
        (plain-text . (lambda (t i) t))))
    (org-export-define-derived-backend 'child 'parent
      :translate-alist '((headline . (lambda (h c i) (format "C: %s" (org-element-property :raw-value h))))))
    (list
     (org-export-derived-backend-p 'child 'parent)
     (org-export-derived-backend-p 'child 'child))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: export read-attribute
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_read_attribute() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (list
     (org-export-read-attribute
      :attr_html
      (with-temp-buffer (org-mode) (insert "#+ATTR_HTML: :a 1 :b 2\nP")
        (goto-char (point-min)) (org-element-at-point)))
     (org-export-read-attribute
      :attr_html
      (with-temp-buffer (org-mode) (insert "P")
        (goto-char (point-min)) (org-element-at-point)))
     (org-export-read-attribute
      :attr_html
      (with-temp-buffer (org-mode) (insert "#+ATTR_HTML: :a nil\nP")
        (goto-char (point-min)) (org-element-at-point))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: export caption
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_caption() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (list
     (with-temp-buffer (org-mode)
       (insert "#+CAPTION: My caption\n| a | b |")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (table (car (org-element-map tree 'table #'identity))))
         (org-export-get-caption table)))
     (with-temp-buffer (org-mode)
       (insert "#+CAPTION[short]: long caption\n| a | b |")
       (goto-char (point-min))
       (let* ((tree (org-element-parse-buffer))
              (table (car (org-element-map tree 'table #'identity))))
         (list (org-export-get-caption table)
               (org-export-get-caption table t)))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: export optional title
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_optional_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "#+TITLE: Doc Title\n* H\nBody")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (info (org-combine-plists
                    (org-export--get-export-attributes)
                    (org-export-get-environment)
                    (org-export--collect-tree-properties tree (org-export-get-environment))))
             (hl (car (org-element-map tree 'headline #'identity))))
        (org-export-get-optional-title hl info)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: export node property
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_export_node_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'ox)
  (let ((org-mode-hook nil))
    (with-temp-buffer (org-mode)
      (insert "* H\n:PROPERTIES:\n:CUSTOM_ID: myid\n:EFFORT: 2h\n:END:")
      (goto-char (point-min))
      (let* ((tree (org-element-parse-buffer))
             (hl (car (org-element-map tree 'headline #'identity))))
        (list (org-export-get-node-property :CUSTOM_ID hl)
              (org-export-get-node-property :EFFORT hl))))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: element type API
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_type_api() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   (org-element-type "string")
   (org-element-type nil)
   (org-element-type 1)
   (org-element-type '(dummy))
   (org-element-type '(dummy nil 'foo))
   (org-element-type '((dummy)))
   (org-element-type '((dummy)) t)
   (org-element-type '("string") t)
   (org-element-type '(1 2) t)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: element type-p API
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_type_p_api() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   (org-element-type-p '(foo) 'foo)
   (org-element-type-p '(foo) '(foo))
   (org-element-type-p '(foo) '(foo bar))
   (org-element-type-p '(foo) 'bar)
   (org-element-type-p '(foo) '(bar baz))
   (org-element-type-p "string" 'plain-text)
   (org-element-type-p '((foo)) 'anonymous)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: element class API
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_class_api() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   (org-element-class '(paragraph nil) nil)
   (org-element-class '(target nil) nil)
   (org-element-class '(org-data nil) nil)
   (org-element-class "text" nil)
   (org-element-class '("secondary " "string") nil)
   (org-element-class '(foo nil) nil)
   (org-element-class '(foo nil) '(center-block nil))
   (org-element-class '(foo nil) '(bold nil))
   (org-element-class '(foo nil) '(paragraph nil))
   (org-element-class '(foo nil) '("secondary"))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: element property inherited
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_property_inherited() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let* ((gc (org-element-create 'gc '(:shared 3 :own-gc "gc")))
         (c (org-element-create 'c '(:shared 2 :own-c "c") gc))
         (p (org-element-create 'p '(:shared 1 :own-p "p") c)))
    (list
     (org-element-property-inherited :shared gc)
     (org-element-property-inherited :shared gc 'with-self)
     (org-element-property-inherited :shared gc 'with-self 'accumulate)
     (org-element-property-inherited :own-p gc 'with-self 'accumulate)
     (org-element-property-inherited :own-c gc 'with-self 'accumulate)
     (org-element-property-inherited :own-gc gc 'with-self 'accumulate))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: element operations chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_element_operations_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (let* ((doc (org-element-create 'org-data nil))
         (h1 (org-element-create 'headline '(:level 1 :raw-value "A")
              (org-element-create 'section nil (org-element-create 'paragraph nil "P1.\n"))))
         (h2 (org-element-create 'headline '(:level 1 :raw-value "B")
              (org-element-create 'section nil (org-element-create 'paragraph nil "P2.\n")))))
    (org-element-adopt doc h1 h2)
    (let ((after-adopt (substring-no-properties (org-element-interpret-data doc))))
      (org-element-extract h2)
      (list after-adopt
            (substring-no-properties (org-element-interpret-data doc))
            (org-element-property :parent h2)))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: deferred chain
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_deferred_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org-element)
  (list
   (let ((el (org-element-create 'd
              `(:deferred ,(org-element-deferred-create t
                            (lambda (el) (org-element-put-property el :foo 'bar) nil))))))
     (list (org-element-property :foo el) (org-element-property :foo2 el)))
   (let ((el (org-element-create 'd `(:foo ,(org-element-deferred-create nil (lambda (_) 'bar))))))
     (org-element-property :foo el))
   (let ((el (org-element-create 'd `(:foo ,(org-element-deferred-create t (lambda (_) 'bar))))))
     (list (org-element-property :foo el) (org-element-property-raw :foo el)))
   (let ((el (org-element-create 'd `( :foo 1 :bar ,(org-element-deferred-create-alias :foo)))))
     (list (org-element-property :foo el) (org-element-property :bar el)))
   (let ((el (org-element-create 'd `(:foo ,(org-element-deferred-create-list
                              (list 1 2 (org-element-deferred-create nil (lambda (_) 3))))))))
     (org-element-property :foo el))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-step: parse-and-interpret round-trips
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn strong_parse_interpret_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (f (lambda (text)
             (with-temp-buffer (org-mode) (insert text)
               (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     (funcall f "*text*") (funcall f "/text/") (funcall f "~text~")
     (funcall f "=text=") (funcall f "_text_") (funcall f "+target+")
     (funcall f "a_b") (funcall f "a_{b}") (funcall f "a^b") (funcall f "a^{b}")
     (funcall f "\\alpha text") (funcall f "\\alpha{}text"))))"##,
    );
}

#[test]
fn strong_link_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (f (lambda (text)
             (with-temp-buffer (org-mode) (insert text)
               (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     (funcall f "[[https://orgmode.org]]")
     (funcall f "[[https://orgmode.org][Org mode]]")
     (funcall f "[[file:todo.org::*task]]")
     (funcall f "[[id:aaaa]]")
     (funcall f "[[#id]]")
     (funcall f "https://orgmode.org")
     (funcall f "<https://orgmode.org>"))))"##,
    );
}

#[test]
fn strong_footnote_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (f (lambda (text)
             (with-temp-buffer (org-mode) (insert text)
               (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     (funcall f "Text[fn:1]") (funcall f "Text[fn:label]")
     (funcall f "Text[fn:label:def]") (funcall f "Text[fn::def]"))))"##,
    );
}

#[test]
fn strong_block_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil) (org-src-preserve-indentation t)
        (f (lambda (text)
             (with-temp-buffer (org-mode) (insert text)
               (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     (funcall f "#+BEGIN_CENTER\nText\n#+END_CENTER")
     (funcall f "#+BEGIN_QUOTE\nText\n#+END_QUOTE")
     (funcall f "#+BEGIN_EXAMPLE\nTest\n#+END_EXAMPLE")
     (funcall f "#+BEGIN_EXPORT HTML\n<p>Text</p>\n#+END_EXPORT")
     (funcall f "#+BEGIN_VERSE\nTest\n#+END_VERSE"))))"##,
    );
}

#[test]
fn strong_inline_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (f (lambda (text)
             (with-temp-buffer (org-mode) (insert text)
               (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     (funcall f "call_test()") (funcall f "call_test(x=2)")
     (funcall f "src_emacs-lisp{(+ 1 1)}") (funcall f "@@backend:contents@@")
     (funcall f "\\command{}") (funcall f "$x$") (funcall f "$$x+y$$")
     (funcall f "\\(x+y\\)") (funcall f "\\[x+y\\]")
     (funcall f "[0/1]") (funcall f "[66%]")
     (funcall f "<<target>>") (funcall f "<<<some text>>>")
     (funcall f "{{{test}}}") (funcall f "{{{test(arg1,arg2)}}}"))))"##,
    );
}

#[test]
fn strong_table_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (f (lambda (text)
             (with-temp-buffer (org-mode) (insert text)
               (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     (funcall f "| a | b |\n| c | d |")
     (funcall f "| a | b |\n|---+---|\n| c | d |"))))"##,
    );
}

#[test]
fn strong_timestamp_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (f (lambda (text)
             (with-temp-buffer (org-mode) (insert text)
               (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     (string-match "<2012-03-29 .* 16:40>" (funcall f "<2012-03-29 thu. 16:40>"))
     (string-match "\\[2012-03-29 .* 16:40\\]" (funcall f "[2012-03-29 thu. 16:40]"))
     (string-match "<2012-03-29 .* 16:40-16:41>" (funcall f "<2012-03-29 thu. 16:40-16:41>"))
     (string-match "<2012-03-29 .* \\+1y>" (funcall f "<2012-03-29 thu. +1y>"))
     (equal "<%%(diary-float t 4 2)>\n" (funcall f "<%%(diary-float t 4 2)>"))))"##,
    );
}

#[test]
fn strong_keyword_comment_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let ((org-mode-hook nil)
        (f (lambda (text)
             (with-temp-buffer (org-mode) (insert text)
               (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     (funcall f "#+KEYWORD: value") (funcall f "# Comment")
     (funcall f "#+BEGIN_COMMENT\nTest\n#+END_COMMENT")
     (funcall f ": Test") (funcall f "-------")
     (funcall f "\\begin{equation}\n1+1=2\n\\end{equation}"))))"##,
    );
}

#[test]
fn strong_citation_roundtrips() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'oc)
  (let ((org-mode-hook nil)
        (f (lambda (text)
             (with-temp-buffer (org-mode) (insert text)
               (org-element-interpret-data (org-element-parse-buffer))))))
    (list
     (funcall f "[cite:@key]") (funcall f "[cite/style:@key]")
     (funcall f "[cite:pre @key]") (funcall f "[cite:@key post]")
     (funcall f "[cite:@a;@b;@c]"))))"##,
    );
}
