//! Org-mode oracle parity tests.
//!
//! These tests intentionally exercise real Org APIs rather than
//! Org-looking regexes, so divergences in parser, editing, table, link,
//! timestamp, and Babel behavior surface at the Elisp boundary.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_element_headline_properties() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Task :work:\n")
    (insert "SCHEDULED: <2026-05-26 Tue>\n")
    (insert "Body\n")
    (insert "** DONE Child\n")
    (let ((out nil))
      (org-element-map (org-element-parse-buffer) 'headline
        (lambda (headline)
          (push (list (org-element-property :level headline)
                      (org-element-property :todo-keyword headline)
                      (org-element-property :raw-value headline)
                      (org-element-property :tags headline))
                out)))
      (nreverse out))))"#,
    );
}

#[test]
fn org_todo_keyword_edit_preserves_plain_buffer_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-log-done nil)
          (org-todo-keywords '((sequence "TODO" "WAIT" "|" "DONE" "CANCELED"))))
      (org-mode)
      (insert "* TODO Task\n")
      (goto-char (point-min))
      (org-todo "DONE")
      (list (substring-no-properties (org-get-todo-state))
            (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_table_align_formats_columns_and_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "| Name | Qty |\n")
    (insert "| apple | 2 |\n")
    (insert "| banana | 10 |\n")
    (goto-char (point-min))
    (org-table-align)
    (buffer-substring-no-properties (point-min) (point-max))))"#,
    );
}

#[test]
fn org_element_link_components() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "See [[https://example.org/path][Example]] and [[file:notes.org::*Target][Target]].\n")
    (let ((out nil))
      (org-element-map (org-element-parse-buffer) 'link
        (lambda (link)
          (push (list (org-element-property :type link)
                      (org-element-property :path link)
                      (org-element-property :raw-link link)
                      (org-element-property :search-option link)
                      (buffer-substring-no-properties
                       (org-element-property :contents-begin link)
                       (org-element-property :contents-end link)))
                out)))
      (nreverse out))))"#,
    );
}

#[test]
fn org_element_timestamp_components() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "Deadline <2026-05-26 Tue 12:34-13:45> and inactive [2026-06-01 Mon]\n")
    (let ((out nil))
      (org-element-map (org-element-parse-buffer) 'timestamp
        (lambda (timestamp)
          (push (list (org-element-property :type timestamp)
                      (org-element-property :year-start timestamp)
                      (org-element-property :month-start timestamp)
                      (org-element-property :day-start timestamp)
                      (org-element-property :hour-start timestamp)
                      (org-element-property :minute-start timestamp)
                      (org-element-property :hour-end timestamp)
                      (org-element-property :minute-end timestamp))
                out)))
      (nreverse out))))"#,
    );
}

#[test]
fn org_babel_emacs_lisp_result_insertion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-emacs-lisp)
  (with-temp-buffer
    (org-mode)
    (insert "#+begin_src emacs-lisp :results value replace\n")
    (insert "(+ 2 3)\n")
    (insert "#+end_src\n")
    (goto-char (point-min))
    (let ((org-confirm-babel-evaluate nil))
      (org-babel-execute-src-block))
    (buffer-substring-no-properties (point-min) (point-max))))"##,
    );
}

#[test]
fn org_subtree_cut_paste_preserves_hierarchy() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* Alpha\n")
    (insert "** A1\nbody A1\n")
    (insert "** A2\n")
    (insert "* Beta\n")
    (insert "** B1\n")
    (insert "* Gamma\n")
    (goto-char (point-min))
    (search-forward "* Beta")
    (beginning-of-line)
    (org-cut-subtree)
    (goto-char (point-max))
    (org-paste-subtree 1)
    (let ((headlines nil))
      (org-element-map (org-element-parse-buffer) 'headline
        (lambda (headline)
          (push (list (org-element-property :level headline)
                      (org-element-property :raw-value headline))
                headlines)))
      (list (nreverse headlines)
            (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_properties_tags_and_todo_mutation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-log-done nil)
          (org-use-property-inheritance t))
      (org-mode)
      (insert "* Project :root:\n")
      (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
      (insert "** TODO Design :work:\n")
      (insert "SCHEDULED: <2026-05-26 Tue>\n")
      (insert "** WAIT Review\n")
      (goto-char (point-min))
      (search-forward "Design")
      (beginning-of-line)
      (org-todo "DONE")
      (org-set-property "Effort" "2:00")
      (org-set-tags '("work" "urgent"))
      (list (org-entry-get nil "Owner" t)
            (org-entry-get nil "Effort")
            (org-get-tags nil t)
            (substring-no-properties (org-get-todo-state))
            (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_nested_checkbox_counts_after_toggles() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* Tasks [0/2]\n")
    (insert "- [ ] first\n")
    (insert "  - [ ] child\n")
    (insert "- [ ] second\n")
    (goto-char (point-min))
    (search-forward "first")
    (beginning-of-line)
    (org-toggle-checkbox)
    (search-forward "child")
    (beginning-of-line)
    (org-toggle-checkbox)
    (goto-char (point-min))
    (org-update-checkbox-count)
    (list (buffer-substring-no-properties (point-min) (point-max))
          (org-element-map (org-element-parse-buffer) 'item
            (lambda (item)
              (org-element-property :checkbox item))))))"#,
    );
}

#[test]
fn org_table_multi_formula_recalculation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "| item | value | tax |\n")
    (insert "|------+-------+-----|\n")
    (insert "| a | 2 | 1 |\n")
    (insert "| b | 3 | 2 |\n")
    (insert "|------+-------+-----|\n")
    (insert "| total |  |  |\n")
    (insert "#+TBLFM: @>$2=vsum(@2..@-1)::@>$3=vsum(@2..@-1)\n")
    (goto-char (point-min))
    (org-table-recalculate-buffer-tables)
    (buffer-substring-no-properties (point-min) (point-max))))"##,
    );
}

#[test]
fn org_document_element_mix_with_properties_blocks_and_footnotes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Demo\n")
    (insert "#+AUTHOR: Ada\n")
    (insert "* TODO Build :tag:\n")
    (insert "DEADLINE: <2026-05-27 Wed>\n")
    (insert ":PROPERTIES:\n:ID: build-1\n:END:\n")
    (insert "Paragraph with [fn:1].\n")
    (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n")
    (insert "| a | b |\n| 1 | 2 |\n")
    (insert "[fn:1] Footnote text\n")
    (let ((tree (org-element-parse-buffer))
          (out nil))
      (dolist (type '(keyword headline planning property-drawer node-property
                      paragraph src-block table footnote-definition))
        (org-element-map tree type
          (lambda (element)
            (push (list type
                        (org-element-property :key element)
                        (org-element-property :raw-value element)
                        (org-element-property :language element)
                        (org-element-property :value element))
                  out))))
      (nreverse out))))"##,
    );
}

#[test]
fn org_html_export_markup_and_link_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-html)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Demo\n")
    (insert "* Head\n")
    (insert "Paragraph with *bold* and [[https://example.org][link]].\n")
    (let* ((org-export-with-toc nil)
           (org-export-show-temporary-export-buffer nil)
           (html (org-export-as 'html nil nil t nil)))
      (list (not (null (string-match-p "<h2" html)))
            (not (null (string-match-p "Head</h2>" html)))
            (not (null (string-match-p "<b>bold</b>" html)))
            (not (null (string-match-p "<a href=\"https://example.org\">link</a>" html)))
            (length html)))))"##,
    );
}
