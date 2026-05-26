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

#[test]
fn org_agenda_file_schedule_deadline_and_tags_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file "org-agenda-probe" nil ".org"
                               "#+CATEGORY: Probe
* TODO Write report :work:
SCHEDULED: <2026-05-27 Wed 09:00>
:PROPERTIES:
:Effort: 1:30
:END:
* WAIT Blocked :home:
DEADLINE: <2026-05-28 Thu>
* DONE Finished :work:
CLOSED: [2026-05-26 Tue]
"))
         (org-agenda-files (list file))
         (org-agenda-span 3)
         (org-agenda-start-day "2026-05-27")
         (org-agenda-start-on-weekday nil)
         (org-agenda-use-time-grid nil)
         (org-agenda-show-all-dates nil)
         (org-agenda-prefix-format "%-8:c%?-12t% s")
         (org-agenda-sorting-strategy '((agenda time-up priority-down category-keep))))
    (unwind-protect
        (progn
          (org-agenda-list nil "2026-05-27" 3)
          (with-current-buffer org-agenda-buffer-name
            (list (not (null (string-match-p "Write report" (buffer-string))))
                  (not (null (string-match-p "Blocked" (buffer-string))))
                  (not (null (string-match-p "Probe" (buffer-string))))
                  (buffer-substring-no-properties (point-min) (point-max)))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-file file))))"##,
    );
}

#[test]
fn org_clock_table_data_from_logbook_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (org-mode)
    (insert "* Project\n")
    (insert "** Task A\n")
    (insert ":LOGBOOK:\n")
    (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:30] =>  1:30\n")
    (insert ":END:\n")
    (insert "** Task B\n")
    (insert ":LOGBOOK:\n")
    (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 11:45] =>  0:45\n")
    (insert ":END:\n")
    (let* ((data (org-clock-get-table-data
                  nil (list :maxlevel 3 :scope 'buffer :block nil)))
           (total (nth 1 data))
           (rows (mapcar (lambda (row)
                           (list (nth 0 row)
                                 (substring-no-properties (nth 1 row))
                                 (nth 4 row)))
                         (nth 2 data))))
      (list total rows))))"#,
    );
}

#[test]
fn org_babel_tangle_multiple_emacs_lisp_blocks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'ob-tangle)
  (require 'ob-emacs-lisp)
  (let* ((src (make-temp-file "org-tangle-src" nil ".org"))
         (out (make-temp-file "org-tangle" nil ".el"))
         (org-confirm-babel-evaluate nil))
    (unwind-protect
        (with-current-buffer (find-file-noselect src)
          (erase-buffer)
          (org-mode)
          (insert "#+PROPERTY: header-args:emacs-lisp :comments no\n")
          (insert "#+begin_src emacs-lisp :tangle " out "\n")
          (insert "(defun alpha () 1)\n")
          (insert "#+end_src\n")
          (insert "#+begin_src emacs-lisp :tangle " out "\n")
          (insert "(defun beta () (+ (alpha) 2))\n")
          (insert "#+end_src\n")
          (save-buffer)
          (let ((files (org-babel-tangle)))
            (list (mapcar #'file-name-extension files)
                  (with-temp-buffer
                    (insert-file-contents out)
                    (buffer-string)))))
      (when (get-file-buffer src) (kill-buffer (get-file-buffer src)))
      (when (file-exists-p src) (delete-file src))
      (when (file-exists-p out) (delete-file out)))))"##,
    );
}

#[test]
fn org_footnote_normalize_and_sort_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-footnote)
  (with-temp-buffer
    (org-mode)
    (insert "* Notes\n")
    (insert "First ref[fn:alpha] and second[fn:beta].\n\n")
    (insert "[fn:beta] Beta text\n")
    (insert "[fn:alpha] Alpha text\n")
    (org-footnote-normalize)
    (org-footnote-sort)
    (list (org-footnote-all-labels)
          (buffer-substring-no-properties (point-min) (point-max)))))"#,
    );
}

#[test]
fn org_archive_to_sibling_normalized_timestamp_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-archive)
  (with-temp-buffer
    (org-mode)
    (insert "* Active\n")
    (insert "** DONE Finished\n")
    (insert "Body\n")
    (insert "** TODO Keep\n")
    (goto-char (point-min))
    (search-forward "Finished")
    (beginning-of-line)
    (org-archive-to-archive-sibling)
    (replace-regexp-in-string
     ":ARCHIVE_TIME: .*"
     ":ARCHIVE_TIME: <time>"
     (buffer-substring-no-properties (point-min) (point-max)))))"#,
    );
}

#[test]
fn org_refile_file_backed_subtree_to_target_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-refile)
  (let ((file (make-temp-file "org-refile" nil ".org"
                              "* Inbox\n** TODO Task\n* Projects\n** Target\n")))
    (unwind-protect
        (with-current-buffer (find-file-noselect file)
          (org-mode)
          (goto-char (point-min))
          (search-forward "Task")
          (beginning-of-line)
          (let ((target-pos (save-excursion
                              (goto-char (point-min))
                              (search-forward "Target")
                              (line-beginning-position))))
            (org-refile nil nil (list "Target" file nil target-pos)))
          (save-buffer)
          (buffer-substring-no-properties (point-min) (point-max)))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"#,
    );
}
