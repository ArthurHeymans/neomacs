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
