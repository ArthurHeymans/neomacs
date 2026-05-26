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
