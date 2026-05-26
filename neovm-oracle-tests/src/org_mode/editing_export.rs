use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_schedule_deadline_priority_property_mutation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-log-reschedule nil)
          (org-log-redeadline nil))
      (org-mode)
      (insert "* TODO Task\n")
      (goto-char (point-min))
      (org-schedule nil "2026-05-27 Wed 09:30")
      (org-deadline nil "2026-05-28 Thu")
      (org-set-property "Effort" "1:15")
      (org-priority ?A)
      (list (org-entry-get nil "SCHEDULED")
            (org-entry-get nil "DEADLINE")
            (org-entry-get nil "Effort")
            (org-get-priority (thing-at-point 'line t))
            (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_clock_in_out_drawer_logbook_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Task\n")
    (goto-char (point-min))
    (let ((org-clock-into-drawer t)
          (org-clock-out-remove-zero-time-clocks nil))
      (org-clock-in nil (encode-time 0 0 9 27 5 2026))
      (org-clock-out nil t (encode-time 0 30 10 27 5 2026))
      (list org-clock-total-time
            (buffer-substring-no-properties (point-min) (point-max))))))"#,
    );
}

#[test]
fn org_promote_demote_subtree_startup_odd_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "#+STARTUP: odd\n")
    (insert "* A\n** B\n*** C\n")
    (goto-char (point-min))
    (search-forward "B")
    (beginning-of-line)
    (org-promote-subtree)
    (let ((after-promote
           (buffer-substring-no-properties (point-min) (point-max))))
      (org-demote-subtree)
      (list after-promote
            (buffer-substring-no-properties (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_list_indent_outdent_repair_lisp_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (require 'org)
  (require 'org-list)
  (with-temp-buffer
    (org-mode)
    (insert "- one\n- two\n  - child\n- three\n")
    (goto-char (point-min))
    (search-forward "two")
    (beginning-of-line)
    (org-indent-item)
    (let ((after-indent
           (buffer-substring-no-properties (point-min) (point-max))))
      (org-outdent-item)
      (org-list-repair)
      (list after-indent
            (buffer-substring-no-properties (point-min) (point-max))
            (org-list-to-lisp)))))"#,
    );
}

#[test]
fn org_texinfo_export_markup_list_table_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-texinfo)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Manual\n")
    (insert "#+AUTHOR: Ada\n")
    (insert "* Intro\n")
    (insert "Text with *bold*, =code=, and [[https://example.org][link]].\n")
    (insert "- item one\n- item two\n")
    (insert "| A | B |\n| 1 | 2 |\n")
    (let* ((org-export-with-toc nil)
           (texi (org-export-as 'texinfo nil nil t nil)))
      (list (not (null (string-match-p "@node Intro" texi)))
            (not (null (string-match-p "@chapter Intro" texi)))
            (not (null (string-match-p "@strong{bold}" texi)))
            (not (null (string-match-p "@samp{code}" texi)))
            (not (null (string-match-p "@uref{https://example.org, link}" texi)))
            (not (null (string-match-p "@itemize" texi)))
            (not (null (string-match-p "@multitable" texi)))
            texi))))"##,
    );
}

#[test]
fn org_beamer_export_frame_list_alert_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-beamer)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Slides\n")
    (insert "#+OPTIONS: H:2 toc:nil\n")
    (insert "* Section\n")
    (insert "** Frame\n")
    (insert "- item one\n- item two\n")
    (insert "#+ATTR_BEAMER: :overlay <2->\n")
    (insert "A paragraph with *bold*.\n")
    (let* ((org-export-with-toc nil)
           (latex (org-export-as 'beamer nil nil t nil))
           (normalized
            (replace-regexp-in-string
             "sec:org[[:alnum:]]+"
             "sec:org-id"
             latex)))
      (list (not (null (string-match-p "\\\\section" latex)))
            (not (null (string-match-p "\\\\begin{frame}" latex)))
            (not (null (string-match-p "{Frame}" latex)))
            (not (null (string-match-p "\\\\begin{itemize}" latex)))
            (not (null (string-match-p "\\\\alert{bold}" latex)))
            normalized))))"##,
    );
}

#[test]
fn org_icalendar_export_todo_schedule_deadline_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-icalendar)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Cal Probe\n")
    (insert "* TODO Event\n")
    (insert "SCHEDULED: <2026-05-27 Wed 09:00-10:00>\n")
    (insert "DEADLINE: <2026-05-28 Thu>\n")
    (let* ((org-icalendar-include-todo t)
           (ical (org-export-as 'icalendar nil nil t nil))
           (normalized
            (replace-regexp-in-string
             "DTSTAMP:[0-9TZ]+"
             "DTSTAMP:<stamp>"
             (replace-regexp-in-string
              "UID:\\(TS1\\|TODO\\)-[^\n]+"
              "UID:\\1-<uid>"
              ical))))
      (list (not (null (string-match-p "BEGIN:VEVENT" ical)))
            (not (null (string-match-p "BEGIN:VTODO" ical)))
            (not (null (string-match-p "SUMMARY:Event" ical)))
            (not (null (string-match-p "DTSTART:20260527T090000" ical)))
            (not (null (string-match-p "DTSTART;VALUE=DATE:20260528" ical)))
            (not (null (string-match-p "STATUS:NEEDS-ACTION" ical)))
            normalized))))"##,
    );
}
