//! Strong edge-combos-4 oracle tests — edge case combinations.
//!
//! Every test returns concrete structured data to surface divergences.

use crate::common::{assert_oracle_parity, return_if_neovm_enable_oracle_proptest_not_set};

#[test]
fn ec4_empty_buffer_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert-heading)
  (insert "First")
  (list (org-get-heading t t t t) (org-current-level)))"##,
    );
}

#[test]
fn ec4_empty_buffer_meta_return() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (org-meta-return)
  (insert "Item")
  (list (buffer-string) (org-element-type (org-element-at-point))))"##,
    );
}

#[test]
fn ec4_deep_nesting_promote_demote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* H1\n** H2\n*** H3\n**** H4")
  (goto-char (point-min))
  (search-forward "H3")
  (let ((before (list (org-current-level) (org-get-heading t t t t))))
    (org-promote)
    (let ((after-p (list (org-current-level) (org-get-heading t t t t))))
      (org-demote)
      (org-demote)
      (list before after-p (list (org-current-level) (org-get-heading t t t t))))))"##,
    );
}

#[test]
fn ec4_unicode_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Test\n:PROPERTIES:\n:VAR: 値\n:NAME: 名前\n:END:")
  (goto-char (point-min))
  (list (org-entry-get nil "VAR") (org-entry-get nil "NAME")
        (progn (org-entry-put nil "VAR" "新しい値") (org-entry-get nil "VAR"))))"##,
    );
}

#[test]
fn ec4_table_empty_rows() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b |\n|   |   |\n| c | d |")
  (goto-char (point-min))
  (let ((d1 (org-table-to-lisp)))
    (org-table-next-row)
    (org-table-put 2 1 "X")
    (list d1 (org-table-to-lisp))))"##,
    );
}

#[test]
fn ec4_list_mixed_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "- plain\n1. ordered\n+ bullet\n- another")
  (org-element-map (org-element-parse-buffer) 'plain-list
    (lambda (pl)
      (list (org-element-property :type pl)
            (length (org-element-contents pl))))))"##,
    );
}

#[test]
fn ec4_footnote_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "Text[fn:1: inline def] more")
  (let* ((tree (org-element-parse-buffer))
         (fn (car (org-element-map tree 'footnote-reference (lambda (f) f)))))
    (list (org-element-property :label fn) (org-element-property :type fn))))"##,
    );
}

#[test]
fn ec4_timestamp_active_inactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<2026-01-15> [2026-01-20]")
  (let* ((tree (org-element-parse-buffer))
         (ts (org-element-map tree 'timestamp
               (lambda (t) (list (org-element-property :type t)
                                 (org-element-property :year-start t)
                                 (org-element-property :day-start t))))))
    ts)"##,
    );
}

#[test]
fn ec4_drawer_empty_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:END:")
  (let* ((tree (org-element-parse-buffer))
         (drawer (car (org-element-map tree 'drawer (lambda (d) d)))))
    (org-element-property :drawer-name drawer))"##,
    );
}

#[test]
fn ec4_link_no_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "See [[https://example.com]]")
  (let* ((tree (org-element-parse-buffer))
         (link (car (org-element-map tree 'link (lambda (l) l)))))
    (list (org-element-property :type link)
          (org-element-property :path link)
          (org-element-property :raw-link link))))"##,
    );
}

#[test]
fn ec4_planning_deadline_only() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task\nDEADLINE: <2026-01-20>")
  (goto-char (point-min))
  (list (org-entry-get nil "DEADLINE") (org-entry-get nil "SCHEDULED") (org-entry-get nil "CLOSED")))"##,
    );
}

#[test]
fn ec4_tag_no_tags() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Simple heading")
  (goto-char (point-min))
  (org-get-tags nil t))"##,
    );
}

#[test]
fn ec4_priority_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO No priority")
  (goto-char (point-min))
  (org-get-priority (char-after)))"##,
    );
}

#[test]
fn ec4_todo_no_keyword() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Just a heading")
  (goto-char (point-min))
  (org-get-todo-state))"##,
    );
}

#[test]
fn ec4_visibility_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Only heading")
  (goto-char (point-min))
  (let ((hidden (get-char-property (point) 'invisible)))
    (org-cycle)
    (list hidden (get-char-property (point) 'invisible))))"##,
    );
}

#[test]
fn ec4_property_empty_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* T\n:PROPERTIES:\n:EMPTY: \n:END:")
  (goto-char (point-min))
  (org-entry-get nil "EMPTY"))"##,
    );
}

#[test]
fn ec4_macro_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+MACRO: greeting Hello!\n{{{greeting}}}")
  (let ((raw (buffer-string)))
    (org-macro-replace-all org-macro-templates)
    (list raw (buffer-string))))"##,
    );
}

#[test]
fn ec4_dynamic_block_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "#+BEGIN: clocktable\n#+END:")
  (goto-char (point-min))
  (org-dblock-update)
  (buffer-string))"##,
    );
}

#[test]
fn ec4_structure_template_various() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "<s")
  (org-try-structure-completion)
  (let ((s1 (buffer-string)))
    (erase-buffer)
    (insert "<e")
    (org-try-structure-completion)
    (list s1 (buffer-string))))"##,
    );
}

#[test]
fn ec4_comment_single_line() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "# Comment line")
  (org-element-map (org-element-parse-buffer) 'comment
    (lambda (c) (org-element-property :value c))))"##,
    );
}

#[test]
fn ec4_fixed_width_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert ": Fixed width")
  (let* ((tree (org-element-parse-buffer))
         (fw (car (org-element-map tree 'fixed-width (lambda (f) f)))))
    (org-element-property :value fw))"##,
    );
}

#[test]
fn ec4_export_empty_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (let* ((info (org-export-get-environment nil)))
    (plist-get info :title)))"##,
    );
}

#[test]
fn ec4_element_buffer_start() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\nBody")
  (goto-char (point-min))
  (let* ((el (org-element-at-point)))
    (list (org-element-type el) (org-element-property :begin el))))"##,
    );
}

#[test]
fn ec4_element_buffer_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Heading\nBody")
  (goto-char (point-max))
  (org-element-type (org-element-at-point)))"##,
    );
}

#[test]
fn ec4_clock_no_clock() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task with no clock")
  (org-clocking-p))"##,
    );
}

#[test]
fn ec4_statistics_no_cookies() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task\n- [ ] item 1\n- [ ] item 2")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
    );
}

#[test]
fn ec4_statistics_all_checked() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* Task [0/0]\n- [X] item 1\n- [X] item 2\n- [X] item 3")
  (goto-char (point-min))
  (org-update-statistics-cookies t)
  (buffer-substring-no-properties (line-beginning-position) (line-end-position)))"##,
    );
}

#[test]
fn ec4_sparse_tree_no_match() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "* TODO Task 1\n* TODO Task 2")
  (goto-char (point-min))
  (org-match-sparse-tree nil "DONE")
  (let ((v '()) (h '()))
    (goto-char (point-min))
    (while (not (eobp))
      (let ((hd (org-get-heading t t t t)))
        (when hd
          (if (get-char-property (point) 'invisible) (push hd h) (push hd v))))
      (forward-line))
    (list (nreverse v) (nreverse h))))"##,
    );
}

#[test]
fn ec4_table_single_row() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a | b | c |")
  (org-table-to-lisp))"##,
    );
}

#[test]
fn ec4_table_single_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (org-mode)
  (insert "| a |\n| b |\n| c |")
  (org-table-to-lisp))"##,
    );
}
