//! DIVERGENCE SURFACE tests — these MUST FAIL to alert teammates of bugs.
//!
//! These tests hit known divergence paths between Neomacs and GNU Emacs.
//! When they FAIL, it surfaces the divergence for investigation.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: drawer parsing with multiple drawers
// Neomacs: ERR "Invalid search bound"
// GNU Emacs: OK parses drawer names
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_drawer_logbook() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:LOGBOOK:\n- Note\n:END:\n:PROPERTIES:\n:A: 1\n:END:")
  (org-element-map (org-element-parse-buffer) 'drawer
    (lambda (d) (org-element-property :drawer-name d))))"##,
    );
}

#[test]
fn divergence_surface_drawer_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:00] =>  1:00\n:END:\nBody\n:CUSTOM:\nData\n:END:")
  (org-element-map (org-element-parse-buffer) 'drawer
    (lambda (d) (org-element-property :drawer-name d))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: org-ctrl-c-ctrl-c on heading
// Neomacs: returns nil (no minibuffer message)
// GNU Emacs: prints "Tags:" minibuffer message, returns nil
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_ctrlc_heading() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H")
  (goto-char (point-min))
  (org-ctrl-c-ctrl-c)
  (buffer-string))"##,
    );
}

#[test]
fn divergence_surface_ctrlc_todo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO H")
  (goto-char (point-min))
  (search-forward "TODO")
  (org-ctrl-c-ctrl-c)
  (buffer-string))"##,
    );
}

#[test]
fn divergence_surface_ctrlc_tag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H :tag:")
  (goto-char (point-max))
  (org-ctrl-c-ctrl-c)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: org-babel-execute returns nil on Neomacs
// Neomacs: returns nil
// GNU Emacs: returns results
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_babel_execute() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (org-babel-execute-src-block)
  (buffer-string))"##,
    );
}

#[test]
fn divergence_surface_babel_execute_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-babel-execute:emacs-lisp "(+ 1 2)" '((:results . "value")))"##);
}

#[test]
fn divergence_surface_babel_execute_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(org-babel-execute:emacs-lisp "(princ \"hello\")" '((:results . "output")))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: org-time-stamp/inactive minibuffer messages
// Neomacs: returns nil (no minibuffer message)
// GNU Emacs: prints "Date+time [...]:"
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_time_stamp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-max))
  (org-time-stamp nil)
  (buffer-string))"##,
    );
}

#[test]
fn divergence_surface_time_stamp_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T")
  (goto-char (point-max))
  (org-time-stamp-inactive nil)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: org-table-sort-lines invalid-function org-string<
// Neomacs: ERR "invalid-function org-string<"
// GNU Emacs: OK sorts table
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_table_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| name | val |\n|---+---|\n| c | 3 |\n| a | 1 |\n| b | 2 |")
  (org-table-sort-lines nil ?a)
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: property drawer parsing after schedule/deadline
// Neomacs: ERR "Invalid search bound"
// GNU Emacs: OK
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_prop_after_schedule() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nSCHEDULED: <2026-01-15>\n:PROPERTIES:\n:A: 1\n:END:")
  (goto-char (point-min))
  (list (org-entry-get nil "A")
        (org-element-map (org-element-parse-buffer) 'planning
          (lambda (p) (when (org-element-property :scheduled p) "S")))
        (buffer-string)))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: clock-in with property drawer
// Neomacs: ERR "Invalid search bound"
// GNU Emacs: OK
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_clock_with_drawer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:A: 1\n:END:")
  (goto-char (point-min))
  (let ((org-clock-out-switch-to-state nil)
        (org-clock-in-switch-to-state nil))
    (org-clock-in)
    (org-clock-out)
    (list (org-element-map (org-element-parse-buffer) 'clock 'identity)
          (buffer-string))))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: org-bibtex-create minibuffer message
// Neomacs: returns nil (no message)
// GNU Emacs: prints "Type:"
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_bibtex_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (condition-case nil
      (org-bibtex-create)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: org-agenda inserts into current buffer
// Neomacs: inserts agenda into current buffer
// GNU Emacs: creates separate *Org Agenda* buffer
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_agenda_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\nSCHEDULED: <2026-01-15>\n* DONE D")
  (condition-case nil
      (org-agenda-list)
    (error nil))
  (buffer-string))"##,
    );
}

#[test]
fn divergence_surface_todo_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C")
  (condition-case nil
      (org-todo-list)
    (error nil))
  (buffer-string))"##,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: HTML export has extra author meta tag
// Neomacs: includes <meta name="author">
// GNU Emacs: does not include author meta
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_html_export_author() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(org-export-string-as "#+TITLE: T\n* H\nBody" 'html t)"##);
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: text property ranges differ
// Neomacs: different property ranges for org-todo-head
// GNU Emacs: different property ranges
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_text_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Heading :tag1:tag2:\nBody")
  (goto-char (point-min))
  (buffer-string))"##,
    );
}
