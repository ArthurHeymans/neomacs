use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_agenda_custom_command_series_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file
                "org-agenda-custom" nil ".org"
                "#+CATEGORY: Work
* TODO Alpha :work:
SCHEDULED: <2026-05-27 Wed 09:00>
* WAIT Beta :work:
DEADLINE: <2026-05-28 Thu>
* TODO Home :home:
SCHEDULED: <2026-05-27 Wed 12:00>
"))
         (org-agenda-files (list file))
         (org-agenda-custom-commands
          '(("x" "Oracle combo"
             ((agenda "" ((org-agenda-span 2)
                          (org-agenda-start-day "2026-05-27")
                          (org-agenda-start-on-weekday nil)
                          (org-agenda-show-all-dates nil)
                          (org-agenda-use-time-grid nil)
                          (org-agenda-prefix-format "%?-12t%-8:c% s")))
              (tags-todo "+work" ((org-agenda-overriding-header "Work tasks")
                                  (org-agenda-prefix-format "%-8:c% s")))
              (todo "WAIT" ((org-agenda-overriding-header "Waiting")
                            (org-agenda-prefix-format "%-8:c% s")))))))
         (org-agenda-sorting-strategy
          '((agenda time-up priority-down category-keep)
            (tags todo-state-up priority-down)
            (todo todo-state-up priority-down))))
    (unwind-protect
        (progn
          (org-agenda nil "x")
          (with-current-buffer org-agenda-buffer-name
            (let ((text (buffer-substring-no-properties
                         (point-min) (point-max))))
              (list (not (null (string-match-p "Oracle combo" text)))
                    (not (null (string-match-p "Work tasks" text)))
                    (not (null (string-match-p "Waiting" text)))
                    (mapcar (lambda (needle)
                              (not (null (string-match-p needle text))))
                            '("TODO Alpha" "WAIT Beta" "TODO Home"))
                    text))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-file file))))"##,
    );
}

#[test]
fn org_agenda_skip_done_tags_represented_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file
                "org-agenda-skip" nil ".org"
                "#+CATEGORY: Probe
* TODO Keep :work:billable:
:PROPERTIES:
:Effort: 0:30
:END:
* DONE Skip :work:
CLOSED: [2026-05-26 Tue]
* WAIT Also keep :work:blocked:
:PROPERTIES:
:Effort: 1:15
:END:
* TODO Other :home:
"))
         (org-agenda-files (list file))
         (org-agenda-prefix-format "%-8:c%5e %s")
         (org-agenda-skip-function
          (lambda ()
            (org-agenda-skip-entry-if 'todo 'done))))
    (unwind-protect
        (progn
          (org-tags-view t "+work")
          (with-current-buffer org-agenda-buffer-name
            (let ((text (buffer-substring-no-properties
                         (point-min) (point-max))))
              (list (mapcar (lambda (needle)
                              (not (null (string-match-p needle text))))
                            '("TODO Keep" "WAIT Also keep" "DONE Skip" "TODO Other"))
                    (sort (org-agenda-get-represented-categories) #'string<)
                    (sort (org-agenda-get-represented-tags) #'string<)
                    text))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-file file))))"##,
    );
}

#[test]
fn org_agenda_log_mode_deadline_schedule_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file
                "org-agenda-log" nil ".org"
                "#+CATEGORY: Log
* DONE Finished :work:
CLOSED: [2026-05-27 Wed 10:00]
SCHEDULED: <2026-05-27 Wed 09:00>
* TODO Due soon :work:
DEADLINE: <2026-05-28 Thu>
* TODO Timed event
<2026-05-27 Wed 14:00-15:00>
"))
         (org-agenda-files (list file))
         (org-agenda-start-day "2026-05-27")
         (org-agenda-span 2)
         (org-agenda-start-on-weekday nil)
         (org-agenda-show-all-dates nil)
         (org-agenda-use-time-grid nil)
         (org-agenda-start-with-log-mode t)
         (org-agenda-log-mode-items '(closed clock state))
         (org-agenda-prefix-format "%?-12t%-8:c% s"))
    (unwind-protect
        (progn
          (org-agenda-list nil "2026-05-27" 2)
          (with-current-buffer org-agenda-buffer-name
            (let ((text (buffer-substring-no-properties
                         (point-min) (point-max))))
              (list (mapcar (lambda (needle)
                              (not (null (string-match-p needle text))))
                            '("Finished" "Closed" "Due soon" "Timed event"
                              "14:00-15:00"))
                    (org-agenda-span-name org-agenda-current-span)
                    text))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-file file))))"##,
    );
}
