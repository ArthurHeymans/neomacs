use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_datetree_property_subtree_timestamp_cleanup_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-datetree)
  (with-temp-buffer
    (let ((org-datetree-add-timestamp 'inactive))
      (org-mode)
      (insert "* Inbox\n")
      (insert "** Timeline\n")
      (insert ":PROPERTIES:\n:DATE_TREE: t\n:END:\n")
      (insert "** Other\n")
      (goto-char (point-min))
      (search-forward "Timeline")
      (org-datetree-file-entry-under "* Late\nBody\n" '(5 27 2026))
      (goto-char (point-min))
      (search-forward "Timeline")
      (org-datetree-file-entry-under "* Early\n<2026-05-25 Mon>\n" '(5 25 2026))
      (goto-char (point-min))
      (search-forward "* Late")
      (insert "Moved stamp <2026-05-26 Tue>\n")
      (goto-char (point-min))
      (org-datetree-cleanup)
      (list
       (buffer-substring-no-properties (point-min) (point-max))
       (save-excursion
         (goto-char (point-min))
         (search-forward "2026-05-26")
         (org-outline-level))
       (save-excursion
         (goto-char (point-min))
         (search-forward "* Other")
         (org-outline-level))))))"#,
    );
}

#[test]
fn org_datetree_iso_week_property_ordering_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-datetree)
  (with-temp-buffer
    (org-mode)
    (insert "* Weekly\n")
    (insert ":PROPERTIES:\n:WEEK_TREE: t\n:END:\n")
    (insert "* Notes\n")
    (goto-char (point-min))
    (search-forward "Weekly")
    (beginning-of-line)
    (org-datetree-find-iso-week-create '(12 31 2026) 'subtree-at-point)
    (insert "\n**** Thu entry\n")
    (goto-char (point-min))
    (search-forward "Weekly")
    (beginning-of-line)
    (org-datetree-find-iso-week-create '(1 1 2027) 'subtree-at-point)
    (insert "\n**** Fri entry\n")
    (goto-char (point-min))
    (search-forward "Weekly")
    (beginning-of-line)
    (org-datetree-find-iso-week-create '(1 5 2026) 'subtree-at-point)
    (insert "\n**** Earlier entry\n")
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
fn org_datetree_month_and_day_find_existing_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-datetree)
  (with-temp-buffer
    (org-mode)
    (insert "* 2026\n")
    (insert "** 2026-05 May\n")
    (insert "*** 2026-05-27 Wednesday\n")
    (insert "**** Existing\n")
    (goto-char (point-min))
    (org-datetree-find-month-create '(5 1 2026))
    (insert "\n*** Month note\n")
    (goto-char (point-min))
    (org-datetree-find-date-create '(5 27 2026))
    (org-end-of-subtree t t)
    (insert "\n**** Day note\n")
    (goto-char (point-min))
    (org-datetree-find-date-create '(6 2 2026))
    (insert "\n**** New month day\n")
    (list
     (buffer-substring-no-properties (point-min) (point-max))
     (save-excursion
       (goto-char (point-min))
       (search-forward "2026-05 May")
       (org-outline-level))
     (save-excursion
       (goto-char (point-min))
       (search-forward "2026-06-02 Tuesday")
       (org-outline-level)))))"#,
    );
}

#[test]
fn org_datetree_dual_tree_cleanup_level_matrix_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-datetree)
  (with-temp-buffer
    (let ((org-datetree-add-timestamp 'active))
      (org-mode)
      (insert "* Daily\n:PROPERTIES:\n:DATE_TREE: t\n:END:\n")
      (insert "* Weekly\n:PROPERTIES:\n:WEEK_TREE: t\n:END:\n")
      (insert "* Loose\n")
      (goto-char (point-min))
      (search-forward "Daily")
      (beginning-of-line)
      (org-datetree-file-entry-under "* Day A\nBody A\n" '(5 27 2026))
      (goto-char (point-min))
      (search-forward "Daily")
      (beginning-of-line)
      (org-datetree-file-entry-under "* Day B\n<2026-05-29 Fri>\n" '(5 29 2026))
      (goto-char (point-min))
      (search-forward "Weekly")
      (beginning-of-line)
      (org-datetree-find-iso-week-create '(5 27 2026) 'subtree-at-point)
      (insert "\n**** Week entry\n")
      (goto-char (point-min))
      (search-forward "Day A")
      (insert "\nMove marker <2026-05-28 Thu>\n")
      (org-datetree-cleanup)
      (let (heads)
        (org-element-map (org-element-parse-buffer) 'headline
          (lambda (headline)
            (push (list (org-element-property :level headline)
                        (org-element-property :raw-value headline))
                  heads)))
        (list (nreverse heads)
              (mapcar (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (search-forward needle)
                          (list needle (org-outline-level))))
                      '("Daily" "2026" "2026-05 May"
                        "2026-05-28 Thursday" "Day A"
                        "Weekly" "2026-W22" "Week entry" "Loose"))
              (buffer-substring-no-properties
               (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_datetree_narrow_cleanup_sort_timestamp_shift_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-datetree)
  (with-temp-buffer
    (let ((org-datetree-add-timestamp 'inactive)
          states)
      (org-mode)
      (insert "* Journal\n:PROPERTIES:\n:DATE_TREE: t\n:END:\n")
      (insert "* Archive\n")
      (goto-char (point-min))
      (search-forward "Journal")
      (beginning-of-line)
      (org-datetree-file-entry-under "* Morning\nBody <2026-05-27 Wed 08:00>\n" '(5 27 2026))
      (goto-char (point-min))
      (search-forward "Journal")
      (beginning-of-line)
      (org-datetree-file-entry-under "* Evening\nBody <2026-05-27 Wed 20:00>\n" '(5 27 2026))
      (goto-char (point-min))
      (search-forward "Journal")
      (beginning-of-line)
      (org-datetree-file-entry-under "* Tomorrow\nBody <2026-05-28 Thu 09:00>\n" '(5 28 2026))
      (let ((snapshot
             (lambda (label)
               (let (heads)
                 (org-element-map (org-element-parse-buffer) 'headline
                   (lambda (headline)
                     (push (list (org-element-property :level headline)
                                 (org-element-property :raw-value headline)
                                 (org-element-property :begin headline)
                                 (org-element-property :end headline))
                           heads)))
                 (list label
                       (nreverse heads)
                       (mapcar (lambda (needle)
                                 (save-excursion
                                   (goto-char (point-min))
                                   (search-forward needle)
                                   (list needle
                                         (org-outline-level)
                                         (line-number-at-pos))))
                               '("Journal" "2026" "2026-05 May"
                                 "2026-05-27 Wednesday"
                                 "Morning" "Evening"
                                 "2026-05-28 Thursday" "Tomorrow"
                                 "Archive"))
                       (buffer-substring-no-properties
                        (point-min) (point-max)))))))
        (push (funcall snapshot 'initial) states)
        (goto-char (point-min))
        (search-forward "Journal")
        (org-narrow-to-subtree)
        (goto-char (point-min))
        (search-forward "Evening")
        (beginning-of-line)
        (org-cut-subtree)
        (goto-char (point-max))
        (org-paste-subtree 4)
        (search-backward "2026-05-27 Wed 20:00")
        (org-timestamp-down-day 1)
        (widen)
        (push (funcall snapshot 'after-shift-hidden-place) states)
        (org-datetree-cleanup)
        (push (funcall snapshot 'after-cleanup) states)
        (goto-char (point-min))
        (search-forward "2026-05-27 Wednesday")
        (beginning-of-line)
        (org-sort-entries nil ?a)
        (push (funcall snapshot 'after-sort-day) states)
        (goto-char (point-min))
        (search-forward "Morning")
        (beginning-of-line)
        (org-copy-subtree)
        (goto-char (point-min))
        (search-forward "Archive")
        (beginning-of-line)
        (org-paste-subtree 2)
        (push (funcall snapshot 'after-copy-archive) states)
        (list (nreverse states)
              (count-matches "^\\*+ " (point-min) (point-max))
              (mapcar (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (search-forward needle)
                          (list needle
                                (org-outline-level)
                                (buffer-substring-no-properties
                                 (line-beginning-position)
                                 (line-end-position)))))
                      '("2026-05-26 Tuesday"
                        "2026-05-27 Wednesday"
                        "2026-05-28 Thursday"
                        "** Morning"))
              (buffer-substring-no-properties
               (point-min) (point-max))))))"#,
    );
}
