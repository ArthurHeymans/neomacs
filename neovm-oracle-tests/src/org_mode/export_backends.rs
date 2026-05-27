use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_markdown_export_toc_footnote_list_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-md)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: Markdown Combo\n")
    (insert "#+OPTIONS: toc:2 num:nil tags:t\n")
    (insert "#+TOC: headlines 2\n")
    (insert "* Alpha :tag:\n")
    (insert "Paragraph with *bold*, /italic/, =code=, [fn:one], and [[https://example.org][site]].\n")
    (insert "- [X] done item\n")
    (insert "- [ ] open item\n")
    (insert "  1. nested number\n")
    (insert "  2. nested second\n\n")
    (insert "#+begin_quote\nquoted *text*\n#+end_quote\n")
    (insert "#+begin_src emacs-lisp\n(+ 1 2)\n#+end_src\n")
    (insert "** Beta\n")
    (insert "| Name | Qty |\n|-\n| apple | 2 |\n")
    (insert "[fn:one] Footnote with [[https://gnu.org][GNU]].\n")
    (let* ((org-md-headline-style 'mixed)
           (org-export-with-broken-links t)
           (md (replace-regexp-in-string
                "org[[:alnum:]]+"
                "org-id"
                (org-export-as 'md nil nil t nil))))
      (list (not (null (string-match-p "Alpha" md)))
            (not (null (string-match-p "done item" md)))
            (not (null (string-match-p "<table" md)))
            (not (null (string-match-p "<sup>" md)))
            md))))"##,
    );
}

#[test]
fn org_ascii_export_drawer_table_clock_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-ascii)
  (with-temp-buffer
    (org-mode)
    (insert "#+TITLE: ASCII Combo\n")
    (insert "#+SUBTITLE: Export Details\n")
    (insert "* TODO Alpha\n")
    (insert "SCHEDULED: <2026-05-27 Wed 09:00-10:00>\n")
    (insert ":LOGBOOK:\n")
    (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:15] =>  1:15\n")
    (insert ":END:\n")
    (insert "Text with \\alpha, H_2O, x^2, and [[https://example.org][Example]].\n")
    (insert "| Item | Count |\n|-\n| apples | 12 |\n| pears | 3 |\n")
    (insert "#+begin_center\nCentered line\n#+end_center\n")
    (insert "#+begin_verse\nRoses are red\n  Indented verse\n#+end_verse\n")
    (let* ((org-ascii-text-width 44)
           (org-ascii-charset 'utf-8)
           (org-ascii-links-to-notes t)
           (org-export-with-drawers '("LOGBOOK"))
           (org-export-with-toc nil)
           (text (org-export-as 'ascii nil nil t nil)))
      (list (not (null (string-match-p "ASCII Combo" text)))
            (not (null (string-match-p "SCHEDULED" text)))
            (not (null (string-match-p "CLOCK" text)))
            (not (null (string-match-p "α" text)))
            (not (null (string-match-p "apples" text)))
            text))))"##,
    );
}

#[test]
fn org_icalendar_export_deadline_schedule_repeater_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-icalendar)
  (let* ((root (make-temp-file "org-ical" t))
         (file (expand-file-name "cal.org" root))
         (org-icalendar-store-UID t)
         (org-icalendar-use-deadline '(event-if-todo todo-due event-if-not-todo))
         (org-icalendar-use-scheduled '(todo-start event-if-todo))
         (org-icalendar-include-todo 'all)
         (org-icalendar-categories '(category local-tags todo-state))
         (org-icalendar-alarm-time 15)
         (org-icalendar-force-alarm t)
         (org-icalendar-timezone "UTC"))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "#+TITLE: Calendar\n")
            (insert "#+CATEGORY: Work\n")
            (insert "* TODO Timed meeting :meet:\n")
            (insert "SCHEDULED: <2026-05-27 Wed 09:00-10:30 +1w>\n")
            (insert "DEADLINE: <2026-05-28 Thu 17:00>\n")
            (insert "Body text with comma, semicolon; and newline marker.\n")
            (insert "* DONE Finished :done:\n")
            (insert "CLOSED: [2026-05-26 Tue 18:00]\n")
            (insert "DEADLINE: <2026-05-29 Fri>\n")
            (insert "* Event only :event:\n")
            (insert "<2026-06-01 Mon 13:00-14:00>\n"))
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (let* ((ics-file (org-icalendar-export-to-ics nil nil nil))
                   (ics (with-temp-buffer
                          (insert-file-contents ics-file)
                          (buffer-string)))
                   (normalized
                    (replace-regexp-in-string
                     "PRODID:-//[^/\n]+//"
                     "PRODID:-//user//"
                     (replace-regexp-in-string
                      "DTSTAMP:[0-9TZ]+"
                      "DTSTAMP:<stamp>"
                      (replace-regexp-in-string
                       "UID:[^\n]+"
                       "UID:<uid>"
                       ics)))))
              (list (not (null (string-match-p "BEGIN:VCALENDAR" ics)))
                    (not (null (string-match-p "BEGIN:VEVENT" ics)))
                    (not (null (string-match-p "BEGIN:VTODO" ics)))
                    (not (null (string-match-p "RRULE" ics)))
                    (not (null (string-match-p "VALARM" ics)))
                    normalized))))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_icalendar_combine_agenda_files_filter_hook_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'ox-icalendar)
  (let* ((root (make-temp-file "org-ical-combine" t))
         (one (expand-file-name "one.org" root))
         (two (expand-file-name "two.org" root))
         (combined (expand-file-name "combined.ics" root))
         (org-agenda-files (list one two))
         (org-icalendar-combined-agenda-file combined)
         (org-icalendar-combined-name "Combined Name")
         (org-icalendar-combined-description "Combined Description")
         (org-icalendar-ttl "PT2H")
         (org-icalendar-timezone "UTC")
         (org-icalendar-include-todo 'all)
         (org-icalendar-use-scheduled '(todo-start event-if-todo))
         (org-icalendar-use-deadline '(todo-due event-if-not-todo))
         (org-icalendar-with-timestamps t)
         (org-icalendar-exclude-tags '("noexport"))
         (saved nil)
         (org-icalendar-after-save-hook
          (list (lambda (file)
                  (push (file-relative-name file root) saved)))))
    (unwind-protect
        (progn
          (with-temp-file one
            (insert "#+TITLE: One\n#+CATEGORY: Alpha\n")
            (insert "* TODO Task one :work:\n")
            (insert "SCHEDULED: <2026-05-27 Wed 10:00>\n")
            (insert "DEADLINE: <2026-05-28 Thu>\n")
            (insert "* Hidden :noexport:\n<2026-05-30 Sat 12:00>\n"))
          (with-temp-file two
            (insert "#+TITLE: Two\n#+CATEGORY: Beta\n")
            (insert "* Event two :event:\n")
            (insert "[2026-06-01 Mon 13:00-14:00]\n")
            (insert "* TODO Task two\n")
            (insert "DEADLINE: <2026-06-02 Tue 09:00>\n"))
          (org-icalendar-combine-agenda-files nil)
          (let* ((ics (with-temp-buffer
                        (insert-file-contents combined)
                        (buffer-string)))
                 (normalized
                  (replace-regexp-in-string
                   "PRODID:-//[^/\n]+//"
                   "PRODID:-//user//"
                   (replace-regexp-in-string
                    "DTSTAMP:[0-9TZ]+"
                    "DTSTAMP:<stamp>"
                    (replace-regexp-in-string
                     "UID:[^\n]+"
                     "UID:<uid>"
                     ics)))))
            (list (sort saved #'string<)
                  (not (null (string-match-p "X-WR-CALNAME:Combined Name" ics)))
                  (not (null (string-match-p "X-WR-CALDESC:Combined Description" ics)))
                  (not (null (string-match-p "X-PUBLISHED-TTL:PT2H" ics)))
                  (not (null (string-match-p "Task one" ics)))
                  (not (null (string-match-p "Task two" ics)))
                  (not (null (string-match-p "Event two" ics)))
                  (null (string-match-p "Hidden" ics))
                  normalized)))
      (dolist (file (list one two))
        (when (get-file-buffer file) (kill-buffer (get-file-buffer file))))
      (delete-directory root t))))"##,
    );
}
