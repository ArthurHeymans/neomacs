use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_sort_entries_property_schedule_custom_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-todo-keywords '((sequence "TODO" "WAIT" "|" "DONE")))
          (events nil)
          (org-after-sorting-entries-or-items-hook
           (list (lambda ()
                   (push (list (line-number-at-pos)
                               (and (org-at-heading-p)
                                    (org-get-heading t t t t)))
                         events)))))
      (org-mode)
      (insert "* Parent\n")
      (insert "** WAIT Zebra [#C]\n")
      (insert "SCHEDULED: <2026-05-29 Fri>\n")
      (insert ":PROPERTIES:\n:Rank: 20\n:Owner: zoe\n:END:\n")
      (insert "See [[https://example.org/z][Zed]].\n")
      (insert "** TODO alpha [#A]\n")
      (insert "SCHEDULED: <2026-05-27 Wed>\n")
      (insert ":PROPERTIES:\n:Rank: 3\n:Owner: ada\n:END:\n")
      (insert "** DONE Middle [#B]\n")
      (insert "SCHEDULED: <2026-05-28 Thu>\n")
      (insert ":PROPERTIES:\n:Rank: 11\n:Owner: bob\n:END:\n")
      (goto-char (point-min))
      (org-sort-entries nil ?r nil nil "Rank")
      (let ((by-rank (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (org-sort-entries nil ?s)
        (let ((by-scheduled (buffer-substring-no-properties (point-min) (point-max))))
          (goto-char (point-min))
          (org-sort-entries
           nil ?f
           (lambda ()
             (concat (or (org-entry-get nil "Owner") "")
                     ":"
                     (org-get-heading t t t t)))
           #'string>)
          (list by-rank
                by-scheduled
                (buffer-substring-no-properties (point-min) (point-max))
                (nreverse events)
                (org-element-map (org-element-parse-buffer) 'headline
                  (lambda (h)
                    (list (org-element-property :level h)
                          (org-element-property :todo-keyword h)
                          (org-element-property :priority h)
                          (org-element-property :raw-value h))))))))"##,
    );
}

#[test]
fn org_sort_list_checkbox_time_custom_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-list)
  (with-temp-buffer
    (let ((events nil)
          (org-after-sorting-entries-or-items-hook
           (list (lambda ()
                   (push (list (line-number-at-pos)
                               (thing-at-point 'line t))
                         events)))))
      (org-mode)
      (insert "- [ ] task beta <2026-05-29 Fri>\n")
      (insert "  - nested z\n")
      (insert "- [X] task alpha <2026-05-27 Wed>\n")
      (insert "- [-] task gamma <2026-05-28 Thu>\n")
      (goto-char (point-min))
      (org-sort-list nil ?x)
      (let ((by-check (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (org-sort-list nil ?t)
        (let ((by-time (buffer-substring-no-properties (point-min) (point-max))))
          (goto-char (point-min))
          (org-sort-list
           t ?f
           (lambda ()
             (let ((line (thing-at-point 'line t)))
               (list (length line) line)))
           (lambda (a b)
             (if (= (car a) (car b))
                 (string< (cadr a) (cadr b))
               (< (car a) (car b)))))
          (list by-check
                by-time
                (buffer-substring-no-properties (point-min) (point-max))
                (nreverse events)
                (org-list-to-lisp))))))"##,
    );
}

#[test]
fn org_table_sort_region_time_numeric_function_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-table)
  (with-temp-buffer
    (org-mode)
    (insert "| Task | Time | Score | Owner |\n")
    (insert "|------+-------+-------+-------|\n")
    (insert "| C | 11:30 | 8 | bob |\n")
    (insert "| A | 09:15 | 13 | ada |\n")
    (insert "| B | 10:00 | 5 | zoe |\n")
    (insert "|------+-------+-------+-------|\n")
    (insert "| Z | 12:00 | 1 | tail |\n")
    (goto-char (point-min))
    (search-forward "Time")
    (org-table-sort-lines nil ?t)
    (let ((by-time (buffer-substring-no-properties (point-min) (point-max))))
      (goto-char (point-min))
      (search-forward "Score")
      (org-table-sort-lines nil ?N)
      (let ((by-score-desc (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "Owner")
        (org-table-sort-lines
         t ?f
         (lambda ()
           (let ((fields (org-split-string (org-table-get-field) "[ \t]*|[ \t]*")))
             (downcase (car fields))))
         #'string<)
        (list by-time
              by-score-desc
              (buffer-substring-no-properties (point-min) (point-max))
              (org-table-to-lisp))))))"##,
    );
}
