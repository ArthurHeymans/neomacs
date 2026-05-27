//! Strong org-mode oracle tests — complex multi-step editing operations.
//!
//! These tests perform sequences of editing operations and compare
//! the final buffer content, point position, or computed values.
//! If Neomacs and GNU Emacs diverge at any step, the final result
//! will differ and the test will fail.

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
fn strong_protocol_parse_roundtrip_multi_step() {
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
