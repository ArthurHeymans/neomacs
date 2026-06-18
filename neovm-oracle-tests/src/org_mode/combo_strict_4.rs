//! Org-mode combo runtime parity: tables/formulas, checkboxes, TODO
//! cycling, element parse/interpret, timestamps, properties, export,
//! duration, clock-sum. Real Org APIs to surface engine divergences.

use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_combo_org_table_recalc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode)
    (insert "| a | b | c |\n|---+---+---|\n| 1 | 2 |   |\n| 3 | 4 |   |\n")
    (insert "#+TBLFM: $3=$1+$2\n")
    (goto-char (point-min)) (forward-line 2)
    (org-table-recalculate t) (org-table-align) (buffer-string)))"##,
    );
}

#[test]
fn org_combo_org_checkbox_statistics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode)
    (insert "* Task [/]\n- [ ] one\n- [X] two\n- [ ] three\n")
    (goto-char (point-min)) (org-update-statistics-cookies t) (buffer-string)))"##,
    );
}

#[test]
fn org_combo_org_sort_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode)
    (insert "- banana\n- apple\n- cherry\n")
    (goto-char (point-min)) (org-sort-list nil ?a) (buffer-string)))"##,
    );
}

#[test]
fn org_combo_org_todo_cycle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "* Heading\n") (goto-char (point-min))
    (let ((states nil))
      (org-todo) (push (org-get-todo-state) states)
      (org-todo) (push (org-get-todo-state) states)
      (org-todo) (push (org-get-todo-state) states)
      (nreverse states))))"##,
    );
}

#[test]
fn org_combo_org_element_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode)
    (insert "* H1\nSome para.\n\n#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n\n- item1\n- item2\n")
    (org-element-map (org-element-parse-buffer) t (lambda (el) (org-element-type el)))))"##,
    );
}

#[test]
fn org_combo_org_timestamp_format() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode)
    (let* ((ts (org-timestamp-from-string "<2024-03-15 Fri 10:30>")))
      (list (org-element-property :year-start ts) (org-element-property :month-start ts)
            (org-element-property :day-start ts) (org-element-property :hour-start ts)
            (org-element-property :minute-start ts) (org-timestamp-format ts "%Y-%m-%d %H:%M")))))"##,
    );
}

#[test]
fn org_combo_org_promote_demote() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "* one\n** two\n*** three\n")
    (goto-char (point-min)) (org-demote-subtree)
    (goto-char (point-max)) (org-back-to-heading) (org-promote) (buffer-string)))"##,
    );
}

#[test]
fn org_combo_org_property_set_get() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "* Heading\n") (goto-char (point-min))
    (org-entry-put (point) "Foo" "bar") (org-entry-put (point) "Count" "3")
    (list (org-entry-get (point) "Foo") (org-entry-get (point) "Count")
          (org-entry-get (point) "Missing"))))"##,
    );
}

#[test]
fn org_combo_org_ascii_export_body() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org) (require 'ox-ascii)
  (with-temp-buffer (org-mode)
    (insert "* Title\n\nA paragraph with *bold* and /italic/ text.\n")
    (let ((org-export-show-temporary-export-buffer nil))
      (org-export-as 'ascii nil nil t))))"##,
    );
}

#[test]
fn org_combo_org_toggle_checkbox() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "- [ ] a\n- [ ] b\n- [ ] c\n")
    (goto-char (point-min)) (org-toggle-checkbox)
    (forward-line 2) (org-toggle-checkbox) (buffer-string)))"##,
    );
}

#[test]
fn org_combo_org_heading_navigation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "* A\n** A1\n* B\n** B1\n** B2\n* C\n")
    (goto-char (point-min))
    (let ((acc nil))
      (while (re-search-forward org-heading-regexp nil t)
        (push (org-get-heading t t t t) acc))
      (nreverse acc))))"##,
    );
}

#[test]
fn org_combo_org_entities() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org) (require 'org-entities)
  (list (org-entity-get "alpha") (length (org-entity-get "Rightarrow"))))"##,
    );
}

#[test]
fn org_combo_org_map_entries_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode)
    (insert "* A :work:\n* B :home:\n* C :work:urgent:\n")
    (org-map-entries (lambda () (org-get-tags)) nil nil)))"##,
    );
}

#[test]
fn org_combo_org_babel_elisp() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org) (require 'ob-emacs-lisp)
  (with-temp-buffer (org-mode)
    (let ((org-confirm-babel-evaluate nil))
      (insert "#+begin_src emacs-lisp :results value\n(+ 1 2 3)\n#+end_src\n")
      (goto-char (point-min)) (org-babel-execute-src-block) (buffer-string))))"##,
    );
}

#[test]
fn org_combo_org_duration_convert() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org) (require 'org-duration)
  (list (org-duration-to-minutes "1:30") (org-duration-from-minutes 90)
        (org-duration-to-minutes "2d 3h") (org-duration-p "1:23")))"##,
    );
}

#[test]
fn org_combo_org_element_interpret() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode)
    (insert "* Title\n\nA *bold* and /italic/ and =code= text.\n")
    (let ((tree (org-element-parse-buffer)))
      (substring-no-properties (org-element-interpret-data tree)))))"##,
    );
}

#[test]
fn org_combo_org_headline_components() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "* TODO [#A] Big task :work:urgent:\n")
    (goto-char (point-min))
    (list (org-get-todo-state) (org-get-priority (thing-at-point 'line)) (org-get-tags))))"##,
    );
}

#[test]
fn org_combo_org_link_escape() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (list (org-link-escape "a b/c[d]") (org-link-unescape (org-link-escape "a b/c[d]"))))"##,
    );
}

#[test]
fn org_combo_org_list_struct() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "- a\n  - a1\n  - a2\n- b\n")
    (goto-char (point-min)) (let ((struct (org-list-struct))) (length struct))))"##,
    );
}

#[test]
fn org_combo_org_paragraph_fill() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "* H\nword1 word2 word3 word4 word5\n")
    (goto-char (point-min)) (forward-line 1)
    (list (count-words (line-beginning-position) (line-end-position))
          (current-word) (org-at-heading-p))))"##,
    );
}

#[test]
fn org_combo_org_property_multivalued() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "* H\n") (goto-char (point-min))
    (org-entry-put-multivalued-property (point) "Items" "x" "y" "z")
    (list (org-entry-get (point) "Items")
          (org-entry-get-multivalued-property (point) "Items"))))"##,
    );
}

#[test]
fn org_combo_org_table_create() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "a,b,c\n1,2,3\n4,5,6\n")
    (goto-char (point-min))
    (org-table-convert-region (point-min) (point-max) '(4)) (buffer-string)))"##,
    );
}

#[test]
fn org_combo_org_table_xref() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "| 5 | 10 |\n| 7 |    |\n")
    (insert "#+TBLFM: @2$2=@1$1+@1$2+@2$1\n")
    (goto-char (point-min)) (org-table-recalculate t) (org-table-align) (buffer-string)))"##,
    );
}

#[test]
fn org_combo_org_timestamp_change() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "<2024-01-31 Wed>\n") (goto-char (point-min))
    (let* ((ts (org-timestamp-from-string "<2024-01-31 Wed>"))
           (later (org-timestamp-from-string "<2024-02-01 Thu>")))
      (list (org-element-property :day-start ts) (org-element-property :month-start ts)
            (org-element-property :day-start later) (org-element-property :month-start later)))))"##,
    );
}

#[test]
fn org_combo_org_table_multi_tblfm() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode) (insert "| 1 | 2 |   |   |\n| 3 | 4 |   |   |\n")
    (insert "#+TBLFM: $3=$1+$2::$4=$1*$2\n")
    (goto-char (point-min)) (org-table-recalculate t) (org-table-align) (buffer-string)))"##,
    );
}

#[test]
fn org_combo_org_sort_entries_todo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org)
  (with-temp-buffer (org-mode)
    (insert "* Parent\n** DONE zebra\n** TODO apple\n** TODO mango\n")
    (goto-char (point-min)) (org-sort-entries nil ?a) (buffer-string)))"##,
    );
}

#[test]
fn org_combo_org_clock_sum_fixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn (require 'org) (require 'org-clock)
  (with-temp-buffer (org-mode) (insert "* Task\n")
    (insert ":LOGBOOK:\n")
    (insert "CLOCK: [2024-01-01 Mon 10:00]--[2024-01-01 Mon 11:30] =>  1:30\n")
    (insert "CLOCK: [2024-01-02 Tue 09:00]--[2024-01-02 Tue 09:45] =>  0:45\n")
    (insert ":END:\n")
    (goto-char (point-min)) (org-clock-sum) (org-back-to-heading)
    (get-text-property (point) :org-clock-minutes)))"##,
    );
}
