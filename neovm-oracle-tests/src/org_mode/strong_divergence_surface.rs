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
    let expect = expect_test::expect![[r#""OK (\"LOGBOOK\" \"PROPERTIES\")""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:LOGBOOK:\n- Note\n:END:\n:PROPERTIES:\n:A: 1\n:END:")
  (org-element-map (org-element-parse-buffer) 'drawer
    (lambda (d) (org-element-property :drawer-name d))))"##,
        expect,
    );
}

#[test]
fn divergence_surface_drawer_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK (\"LOGBOOK\" \"CUSTOM\")""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:LOGBOOK:\nCLOCK: [2026-01-10 10:00]--[2026-01-10 11:00] =>  1:00\n:END:\nBody\n:CUSTOM:\nData\n:END:")
  (org-element-map (org-element-parse-buffer) 'drawer
    (lambda (d) (org-element-property :drawer-name d))))"##,
        expect,
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
    crate::common::assert_oracle_parity(
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
    crate::common::assert_oracle_parity(
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
    crate::common::assert_oracle_parity(
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
    let expect = expect_test::expect![[r##""OK \"#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC\"""##]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN_SRC emacs-lisp\n(+ 1 2)\n#+END_SRC")
  (goto-char (point-min))
  (org-babel-execute-src-block)
  (buffer-string))"##,
        expect,
    );
}

#[test]
fn divergence_surface_babel_execute_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK 3""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(org-babel-execute:emacs-lisp "(+ 1 2)" '((:results . "value")))"##,
        expect,
    );
}

#[test]
fn divergence_surface_babel_execute_output() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""helloOK \"hello\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(org-babel-execute:emacs-lisp "(princ \"hello\")" '((:results . "output")))"##,
        expect,
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
    crate::common::assert_oracle_parity(
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
    crate::common::assert_oracle_parity(
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
    let expect = expect_test::expect![[r#""ERR (user-error \"Not in table data field\")""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| name | val |\n|---+---|\n| c | 3 |\n| a | 1 |\n| b | 2 |")
  (org-table-sort-lines nil ?a)
  (buffer-string))"##,
        expect,
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
    let expect = expect_test::expect![[
        r#""OK (\"1\" (\"S\") \"* T\nSCHEDULED: <2026-01-15>\n:PROPERTIES:\n:A: 1\n:END:\")""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\nSCHEDULED: <2026-01-15>\n:PROPERTIES:\n:A: 1\n:END:")
  (goto-char (point-min))
  (list (org-entry-get nil "A")
        (org-element-map (org-element-parse-buffer) 'planning
          (lambda (p) (when (org-element-property :scheduled p) "S")))
        (buffer-string)))"##,
        expect,
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
    let expect = expect_test::expect![[
        r#""OK (((clock (:standard-properties [40 40 nil nil 103 0 nil nil nil nil nil nil nil nil #<killed buffer> nil nil (drawer (:standard-properties [30 30 40 103 109 0 nil nil nil nil nil 41 103 nil #<killed buffer> nil nil (section (:standard-properties [5 5 5 109 109 0 nil section nil nil nil 5 109 nil #<killed buffer> nil nil (headline (:standard-properties [1 1 5 109 109 0 (:title) first-section nil nil nil 29 107 1 #<killed buffer> nil nil (org-data (:standard-properties [1 1 1 109 109 0 nil org-data nil nil nil 3 109 nil #<killed buffer> nil nil nil] :pre-blank 0 :path nil :CATEGORY nil) #11)] :pre-blank 0 :raw-value \"T\" :title (#(\"T\" 0 1 (:parent (headline (:standard-properties [1 1 5 109 109 0 (:title) first-section nil nil nil 29 107 1 #<killed buffer> nil nil (org-data (:standard-properties [1 1 1 109 109 0 nil org-data nil nil nil 3 109 nil #<killed buffer> nil nil nil] :pre-blank 0 :path nil :CATEGORY nil) #16)] :pre-blank 0 :raw-value \"T\" :title (#(\"T\" 0 1 (:parent #16))) :level 1 :priority nil :tags nil :todo-keyword nil :todo-type nil :footnote-section-p nil :archivedp nil :commentedp nil :A \"1\") (section (:standard-properties [5 5 5 109 109 0 nil section nil nil nil 5 109 nil #<killed buffer> nil nil #16]) (property-drawer (:standard-properties [5 5 18 24 30 0 nil planning nil nil nil nil nil nil #<killed buffer> nil nil #17]) (node-property (:standard-properties [18 18 nil nil 24 0 nil node-property nil nil nil nil nil nil #<killed buffer> nil nil #18] :key \"A\" :value \"1\"))) (drawer (:standard-properties [30 30 40 103 109 0 nil nil nil nil nil 41 103 nil #<killed buffer> nil nil #17] :pre-blank 0 :drawer-name \"LOGBOOK\") (clock (:standard-properties [40 40 nil nil 103 0 nil nil nil nil nil nil nil nil #<killed buffer> nil nil #18] :status closed :value (timestamp (:standard-properties [47 nil nil nil 94 1 nil nil nil nil nil nil nil nil nil nil nil nil] :type inactive-range :range-type daterange :raw-value \"[2026-06-29 Mon 06:32]--[2026-06-29 Mon 06:32]\" :year-start 2026 :month-start 6 :day-start 29 :hour-start 6 :minute-start 32 :year-end 2026 :month-end 6 :day-end 29 :hour-end 6 :minute-end 32)) :duration \"0:00\")))))))) :level 1 :priority nil :tags nil :todo-keyword nil :todo-type nil :footnote-section-p nil :archivedp nil :commentedp nil :A \"1\") #8)]) (property-drawer (:standard-properties [5 5 18 24 30 0 nil planning nil nil nil nil nil nil #<killed buffer> nil nil #8]) (node-property (:standard-properties [18 18 nil nil 24 0 nil node-property nil nil nil nil nil nil #<killed buffer> nil nil #9] :key \"A\" :value \"1\"))) #5)] :pre-blank 0 :drawer-name \"LOGBOOK\") #2)] :status closed :value (timestamp (:standard-properties [47 nil nil nil 94 1 nil nil nil nil nil nil nil nil nil nil nil nil] :type inactive-range :range-type daterange :raw-value \"[2026-06-29 Mon 06:32]--[2026-06-29 Mon 06:32]\" :year-start 2026 :month-start 6 :day-start 29 :hour-start 6 :minute-start 32 :year-end 2026 :month-end 6 :day-end 29 :hour-end 6 :minute-end 32)) :duration \"0:00\"))) \"* T\n:PROPERTIES:\n:A: 1\n:END:\n:LOGBOOK:\nCLOCK: [2026-06-29 Mon 06:32]--[2026-06-29 Mon 06:32] =>  0:00\n:END:\n\")""#
    ]];
    crate::common::assert_oracle_parity_expect(
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
        expect,
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
    crate::common::assert_oracle_parity(
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
    let expect = expect_test::expect![[
        r#""OK #(\"Week-agenda (W27):\nMonday     29 June 2026 W27\nTuesday    30 June 2026\nWednesday   1 July 2026\nThursday    2 July 2026\nFriday      3 July 2026\nSaturday    4 July 2026\nSunday      5 July 2026\n\" 0 18 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda org-agenda-structural-header t org-date-line t face org-agenda-structure) 18 19 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 19 46 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739796 org-today t org-day-cnt 1 org-agenda-date-header t org-date-line t face org-agenda-date-today) 46 47 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 47 70 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739797 org-day-cnt 2 org-agenda-date-header t org-date-line t face org-agenda-date) 70 71 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 71 94 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739798 org-day-cnt 3 org-agenda-date-header t org-date-line t face org-agenda-date) 94 95 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 95 118 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739799 org-day-cnt 4 org-agenda-date-header t org-date-line t face org-agenda-date) 118 119 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 119 142 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739800 org-day-cnt 5 org-agenda-date-header t org-date-line t face org-agenda-date) 142 143 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 143 166 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739801 org-day-cnt 6 org-agenda-date-header t org-date-line t face org-agenda-date-weekend) 166 167 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda) 167 190 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda day 739802 org-day-cnt 7 org-agenda-date-header t org-date-line t face org-agenda-date-weekend) 190 191 (org-series-cmd nil org-redo-cmd (org-agenda-list 'nil nil 'week nil) org-last-args (nil nil week) org-agenda-type agenda))""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO T\nSCHEDULED: <2026-01-15>\n* DONE D")
  (condition-case nil
      (org-agenda-list)
    (error nil))
  (buffer-string))"##,
        expect,
    );
}

#[test]
fn divergence_surface_todo_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[
        r#""OK #(\"Global list of TODO items of type: ALL\nPress ‘N r’ (e.g. ‘0 r’) to search again: (0)[ALL]\n\" 0 34 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo org-agenda-structural-header t short-heading \"ToDo: ALL\" face org-agenda-structure) 34 35 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo org-agenda-structural-header t) 35 38 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo org-agenda-structural-header t face org-agenda-structure-filter) 38 39 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo) 39 48 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 48 49 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo font-lock-face help-key-binding face org-agenda-structure-secondary) 49 50 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 50 57 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 57 58 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 58 60 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 60 61 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo font-lock-face help-key-binding face org-agenda-structure-secondary) 61 62 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 62 89 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo face org-agenda-structure-secondary) 89 90 (org-series-cmd nil org-redo-cmd (org-todo-list (or (and (numberp current-prefix-arg) current-prefix-arg) nil current-prefix-arg nil)) org-last-args nil org-agenda-type todo))""#
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO A\n* DONE B\n* TODO C")
  (condition-case nil
      (org-todo-list)
    (error nil))
  (buffer-string))"##,
        expect,
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
    let expect = expect_test::expect![[
        r##""OK \"<div id=\\\"table-of-contents\\\" role=\\\"doc-toc\\\">\n<h2>Table of Contents</h2>\n<div id=\\\"text-table-of-contents\\\" role=\\\"doc-toc\\\">\n<ul>\n<li><a href=\\\"#org155d2ba\\\">1. H</a></li>\n</ul>\n</div>\n</div>\n<div id=\\\"outline-container-org155d2ba\\\" class=\\\"outline-2\\\">\n<h2 id=\\\"org155d2ba\\\"><span class=\\\"section-number-2\\\">1.</span> H</h2>\n<div class=\\\"outline-text-2\\\" id=\\\"text-1\\\">\n<p>\nBody</p>\n</div>\n</div>\n\"""##
    ]];
    crate::common::assert_oracle_parity_expect(
        r##"(org-export-string-as "#+TITLE: T\n* H\nBody" 'html t)"##,
        expect,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// DIVERGENCE: text property ranges differ
// Neomacs: different property ranges for org-todo-head
// GNU Emacs: different property ranges
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn divergence_surface_text_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""OK \"* TODO [#A] Heading :tag1:tag2:\nBody\"""#]];
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO [#A] Heading :tag1:tag2:\nBody")
  (goto-char (point-min))
  (buffer-string))"##,
        expect,
    );
}
