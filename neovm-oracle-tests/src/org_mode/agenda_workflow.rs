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
fn org_agenda_modes_filter_mutate_source_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (require 'org-clock)
  (let* ((file (make-temp-file
                "org-agenda-modes" nil ".org"
                "#+CATEGORY: Modes
* TODO Alpha :work:billable:
SCHEDULED: <2026-05-27 Wed 09:00>
:PROPERTIES:
:Effort: 0:45
:END:
Body alpha line one.
Body alpha line two.
CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 09:30] =>  0:30
* WAIT Beta :work:internal:
DEADLINE: <2026-05-27 Wed>
:PROPERTIES:
:Effort: 2:00
:END:
Body beta.
CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 12:15] =>  1:15
* DONE Gamma :home:
CLOSED: [2026-05-27 Wed 17:00]
"))
         (org-agenda-files (list file))
         (org-agenda-start-day "2026-05-27")
         (org-agenda-span 1)
         (org-agenda-start-on-weekday nil)
         (org-agenda-show-all-dates nil)
         (org-agenda-use-time-grid nil)
         (org-agenda-entry-text-maxlines 2)
         (org-agenda-clockreport-parameter-plist
          '(:link nil :maxlevel 2 :fileskip0 t))
         (org-agenda-prefix-format "%?-12t%-8:c%5e %s"))
    (unwind-protect
        (progn
          (org-agenda-list nil "2026-05-27" 1)
          (with-current-buffer org-agenda-buffer-name
            (let ((initial (buffer-substring-no-properties
                            (point-min) (point-max))))
              (org-agenda-entry-text-mode)
              (let ((entry-text (buffer-substring-no-properties
                                 (point-min) (point-max)))
                    (entry-mode org-agenda-entry-text-mode))
                (org-agenda-clockreport-mode)
                (let ((clockreport (buffer-substring-no-properties
                                    (point-min) (point-max)))
                      (clock-mode org-agenda-clockreport-mode))
                  (org-agenda-filter-apply '("+work" "-internal") 'tag t)
                  (let ((filtered (buffer-substring-no-properties
                                   (point-min) (point-max)))
                        (tag-filter org-agenda-tag-filter))
                    (org-agenda-filter-remove-all)
                    (goto-char (point-min))
                    (search-forward "Alpha")
                    (beginning-of-line)
                    (let ((org-log-reschedule nil)
                          (org-log-redeadline nil)
                          (org-log-done nil))
                      (org-agenda-priority ?A)
                      (org-agenda-schedule nil "2026-05-28 10:15")
                      (org-agenda-deadline nil "2026-05-29")
                      (org-agenda-set-tags "review" 'on)
                      (org-agenda-todo "DONE"))
                    (let ((after-mutate
                           (buffer-substring-no-properties
                            (point-min) (point-max))))
                      (list (mapcar (lambda (needle)
                                      (not (null
                                            (string-match-p needle initial))))
                                    '("Alpha" "Beta" "Gamma"))
                            entry-mode
                            (mapcar (lambda (needle)
                                      (not (null
                                            (string-match-p needle entry-text))))
                                    '("Body alpha line one"
                                      "Body alpha line two"
                                      "Body beta"))
                            clock-mode
                            (mapcar (lambda (needle)
                                      (not (null
                                            (string-match-p needle
                                                            clockreport))))
                                    '("Clock summary" "Alpha" "Beta"
                                      "0:30" "1:15"))
                            tag-filter
                            (mapcar (lambda (needle)
                                      (not (null
                                            (string-match-p needle
                                                            filtered))))
                                    '("Alpha" "Beta" "Gamma"))
                            (replace-regexp-in-string
                             "org-agenda-modes[^ \n|]+\\.org"
                             "org-agenda-modes<tmp>.org"
                             after-mutate)
                            (with-current-buffer (find-file-noselect file)
                              (buffer-substring-no-properties
                               (point-min) (point-max)))))))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
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

#[test]
fn org_agenda_filter_apply_remove_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file
                "org-agenda-filter" nil ".org"
                "#+CATEGORY: Filter
* TODO Alpha :work:billable:
:PROPERTIES:
:Effort: 0:30
:END:
* TODO Beta :work:internal:
:PROPERTIES:
:Effort: 2:00
:END:
* TODO Home :home:
:PROPERTIES:
:Effort: 0:15
:END:
"))
         (org-agenda-files (list file))
         (org-agenda-prefix-format "%-8:c%5e %s")
         (org-agenda-show-all-dates nil))
    (unwind-protect
        (progn
          (org-tags-view t "+TODO")
          (with-current-buffer org-agenda-buffer-name
            (let ((all (buffer-substring-no-properties
                        (point-min) (point-max))))
              (org-agenda-filter-apply '("+work") 'tag)
              (let ((work (buffer-substring-no-properties
                           (point-min) (point-max)))
                    (filter-tag org-agenda-tag-filter))
                (org-agenda-filter-apply '("<1:00") 'effort)
                (let ((effort (buffer-substring-no-properties
                               (point-min) (point-max)))
                      (filter-effort org-agenda-effort-filter))
                  (org-agenda-filter-remove-all)
                  (list (mapcar (lambda (needle)
                                  (not (null (string-match-p needle all))))
                                '("Alpha" "Beta" "Home"))
                        (mapcar (lambda (needle)
                                  (not (null (string-match-p needle work))))
                                '("Alpha" "Beta" "Home"))
                        (mapcar (lambda (needle)
                                  (not (null (string-match-p needle effort))))
                                '("Alpha" "Beta" "Home"))
                        filter-tag
                        filter-effort
                        org-agenda-tag-filter
                        org-agenda-effort-filter)))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-file file))))"##,
    );
}

#[test]
fn org_agenda_priority_effort_source_mutation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file
                "org-agenda-edit" nil ".org"
                "#+CATEGORY: Edit
* TODO Alpha
:PROPERTIES:
:Effort: 0:30
:END:
* TODO Beta
:PROPERTIES:
:Effort: 1:00
:END:
"))
         (org-agenda-files (list file))
         (org-agenda-prefix-format "%-8:c%5e %s")
         (org-priority-enable-commands t))
    (unwind-protect
        (progn
          (org-todo-list)
          (with-current-buffer org-agenda-buffer-name
            (goto-char (point-min))
            (search-forward "Alpha")
            (beginning-of-line)
            (org-agenda-priority ?A)
            (cl-letf (((symbol-function 'completing-read)
                       (lambda (&rest _) "2:30")))
              (org-agenda-set-effort))
            (let ((agenda-after-alpha
                   (buffer-substring-no-properties
                    (point-min) (point-max))))
              (search-forward "Beta")
              (beginning-of-line)
              (org-agenda-priority 'down)
              (list agenda-after-alpha
                    (buffer-substring-no-properties
                     (point-min) (point-max))
                    (with-current-buffer (find-file-noselect file)
                      (buffer-substring-no-properties
                       (point-min) (point-max)))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-file file))))"##,
    );
}

#[test]
fn org_agenda_bulk_mark_toggle_regexp_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file
                "org-agenda-bulk" nil ".org"
                "#+CATEGORY: Bulk
* TODO Alpha :work:
* TODO Beta :home:
* WAIT Gamma :work:
"))
         (org-agenda-files (list file))
         (org-agenda-prefix-format "%-8:c% s"))
    (unwind-protect
        (progn
          (org-todo-list)
          (with-current-buffer org-agenda-buffer-name
            (goto-char (point-min))
            (search-forward "Alpha")
            (beginning-of-line)
            (org-agenda-bulk-mark 2)
            (let ((after-two
                   (list (length org-agenda-bulk-marked-entries)
                         (org-agenda-bulk-marked-p)
                         (mapcar (lambda (m)
                                   (with-current-buffer (marker-buffer m)
                                     (save-excursion
                                       (goto-char m)
                                       (org-get-heading t t t t))))
                                 org-agenda-bulk-marked-entries))))
              (org-agenda-bulk-unmark-all)
              (org-agenda-bulk-mark-regexp "Gamma")
              (let ((after-regexp
                     (list (length org-agenda-bulk-marked-entries)
                           (mapcar (lambda (m)
                                     (with-current-buffer (marker-buffer m)
                                       (save-excursion
                                         (goto-char m)
                                         (org-get-heading t t t t))))
                                   org-agenda-bulk-marked-entries))))
                (org-agenda-bulk-toggle-all)
                (list after-two
                      after-regexp
                      (length org-agenda-bulk-marked-entries)
                      (buffer-substring-no-properties
                       (point-min) (point-max)))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_agenda_filter_matcher_visibility_matrix_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file
                "org-agenda-filter-matrix" nil ".org"
                "#+CATEGORY: Matrix
* TODO Alpha :work:billable:
:PROPERTIES:
:Effort: 0:30
:Owner: Ada
:END:
* WAIT Beta :work:internal:
:PROPERTIES:
:Effort: 2:00
:Owner: Bea
:END:
* TODO Home :home:
:PROPERTIES:
:Effort: 0:15
:Owner: Cy
:END:
"))
         (org-agenda-files (list file))
         (org-agenda-prefix-format "%-8:c%5e %s")
         (org-agenda-show-all-dates nil)
         (org-agenda-hide-tags-regexp nil))
    (unwind-protect
        (progn
          (org-tags-view t "+TODO")
          (with-current-buffer org-agenda-buffer-name
            (let ((all (buffer-substring-no-properties
                        (point-min) (point-max)))
                  (tag-matcher
                   (org-agenda-filter-make-matcher-tag-exp
                    '("+work" "-internal") 'and))
                  (effort-form
                   (org-agenda-filter-effort-form "<1:00")))
              (org-agenda-filter-by-regexp nil)
              (let ((after-regexp-filter org-agenda-regexp-filter))
                (org-agenda-filter-apply '("+work" "-internal") 'tag t)
                (let ((work-billable
                       (buffer-substring-no-properties
                        (point-min) (point-max)))
                      (tag-filter org-agenda-tag-filter))
                  (org-agenda-filter-apply '("<1:00") 'effort)
                  (let ((effort-filtered
                         (buffer-substring-no-properties
                          (point-min) (point-max)))
                        (effort-filter org-agenda-effort-filter)
                        (line-states nil))
                    (goto-char (point-min))
                    (while (re-search-forward "^[ \t]*Matrix" nil t)
                      (push (list
                             (buffer-substring-no-properties
                              (line-beginning-position)
                              (line-end-position))
                             (get-text-property (line-beginning-position)
                                                'invisible))
                            line-states))
                    (org-agenda-filter-remove-all)
                    (list (mapcar (lambda (needle)
                                    (not (null (string-match-p needle all))))
                                  '("Alpha" "Beta" "Home"))
                          tag-matcher
                          effort-form
                          after-regexp-filter
                          tag-filter
                          effort-filter
                          (mapcar (lambda (needle)
                                    (not (null (string-match-p
                                                needle work-billable))))
                                  '("Alpha" "Beta" "Home"))
                          (mapcar (lambda (needle)
                                    (not (null (string-match-p
                                                needle effort-filtered))))
                                  '("Alpha" "Beta" "Home"))
                          (nreverse line-states)
                          org-agenda-tag-filter
                          org-agenda-effort-filter))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_agenda_clockreport_archives_mode_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (require 'org-clock)
  (let* ((file (make-temp-file
                "org-agenda-clockreport" nil ".org"
                "#+CATEGORY: Report
* TODO Alpha :work:
SCHEDULED: <2026-05-27 Wed>
CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:00] =>  1:00
* TODO Beta :ARCHIVE:
SCHEDULED: <2026-05-27 Wed>
CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 12:30] =>  1:30
* TODO Gamma :work:
SCHEDULED: <2026-05-28 Thu>
CLOCK: [2026-05-28 Thu 08:00]--[2026-05-28 Thu 08:45] =>  0:45
"))
         (org-agenda-files (list file))
         (org-agenda-start-day "2026-05-27")
         (org-agenda-span 2)
         (org-agenda-start-on-weekday nil)
         (org-agenda-show-all-dates nil)
         (org-agenda-use-time-grid nil)
         (org-agenda-clockreport-parameter-plist
          '(:link nil :maxlevel 3 :fileskip0 t))
         (org-agenda-prefix-format "%-8:c%?-12t% s"))
    (unwind-protect
        (progn
          (org-agenda-list nil "2026-05-27" 2)
          (with-current-buffer org-agenda-buffer-name
            (let ((initial (buffer-substring-no-properties
                            (point-min) (point-max))))
              (org-agenda-clockreport-mode)
              (let ((clockreport (buffer-substring-no-properties
                                  (point-min) (point-max)))
                    (clock-mode org-agenda-clockreport-mode))
                (org-agenda-archives-mode)
                (let ((archives (buffer-substring-no-properties
                                 (point-min) (point-max)))
                      (archive-mode org-agenda-archives-mode))
                  (list (mapcar (lambda (needle)
                                  (not (null (string-match-p needle initial))))
                                '("Alpha" "Beta" "Gamma"))
                        (mapcar (lambda (needle)
                                  (not (null (string-match-p needle clockreport))))
                                '("Clock summary" "Alpha" "Gamma" "1:00" "0:45"))
                        clock-mode
                        (mapcar (lambda (needle)
                                  (not (null (string-match-p needle archives))))
                                '("Alpha" "Beta" "Gamma" "1:30"))
                        archive-mode
                        clockreport
                        archives)))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-file file))))"##,
    );
}

#[test]
fn org_agenda_entry_text_switch_context_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file
                "org-agenda-entry-text" nil ".org"
                "#+CATEGORY: Text
* TODO Alpha :work:
First line.
Second line.
Third line.
* TODO Beta :home:
Beta body.
"))
         (org-agenda-files (list file))
         (org-agenda-prefix-format "%-8:c% s")
         (org-agenda-entry-text-maxlines 2))
    (unwind-protect
        (progn
          (org-todo-list)
          (with-current-buffer org-agenda-buffer-name
            (goto-char (point-min))
            (search-forward "Alpha")
            (beginning-of-line)
            (let ((before (buffer-substring-no-properties
                           (point-min) (point-max))))
              (org-agenda-entry-text-mode 2)
              (let ((with-text (buffer-substring-no-properties
                                (point-min) (point-max)))
                    (mode-on org-agenda-entry-text-mode))
                (org-agenda-switch-to)
                (let ((source (with-current-buffer (find-file-noselect file)
                                (list (org-get-heading t t t t)
                                      (buffer-substring-no-properties
                                       (line-beginning-position)
                                       (line-end-position))))))
                  (with-current-buffer org-agenda-buffer-name
                    (org-agenda-entry-text-mode)
                    (list (not (null (string-match-p "First line" before)))
                          (mapcar (lambda (needle)
                                    (not (null
                                          (string-match-p needle with-text))))
                                  '("First line" "Second line" "Third line"
                                    "Beta body"))
                          mode-on
                          source
                          org-agenda-entry-text-mode
                          (buffer-substring-no-properties
                           (point-min) (point-max)))))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-file file))))"##,
    );
}

#[test]
fn org_agenda_archive_sibling_source_mutation_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (require 'org-archive)
  (let* ((file (make-temp-file
                "org-agenda-archive-sibling" nil ".org"
                "#+CATEGORY: Archive
* TODO Keep
* DONE Finished
:PROPERTIES:
:Effort: 0:30
:END:
Body.
* TODO Later
"))
         (org-agenda-files (list file))
         (org-agenda-prefix-format "%-8:c% s")
         (org-archive-location "::* Archive"))
    (unwind-protect
        (progn
          (org-todo-list)
          (with-current-buffer org-agenda-buffer-name
            (goto-char (point-min))
            (search-forward "Finished")
            (beginning-of-line)
            (org-agenda-archive-to-archive-sibling)
            (let ((agenda-after (buffer-substring-no-properties
                                 (point-min) (point-max))))
              (with-current-buffer (find-file-noselect file)
                (let ((text (buffer-substring-no-properties
                             (point-min) (point-max))))
                  (list (mapcar (lambda (needle)
                                  (not (null
                                        (string-match-p needle agenda-after))))
                                '("Keep" "Finished" "Later"))
                        (mapcar (lambda (needle)
                                  (not (null (string-match-p needle text))))
                                '("* Archive" "** DONE Finished" ":Effort:"
                                  "Body." "* TODO Later"))
                        agenda-after
                        text))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-file file))))"##,
    );
}

#[test]
fn org_agenda_day_entries_properties_timestamp_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file
                "org-agenda-day-entries" nil ".org"
                "#+CATEGORY: Day
* TODO Scheduled [#A] :work:
SCHEDULED: <2026-05-27 Wed 09:00>
:PROPERTIES:
:Effort: 0:45
:END:
* TODO Deadline [#C] :work:
DEADLINE: <2026-05-27 Wed -1d>
:PROPERTIES:
:Effort: 1:30
:END:
* DONE Closed :done:
CLOSED: [2026-05-27 Wed 17:00]
* Event heading :event:
<2026-05-27 Wed 14:00-15:30>
* Repeating deadline :repeat:
DEADLINE: <2026-05-20 Wed +1w>
"))
         (org-agenda-files (list file))
         (org-agenda-prefix-format "%?-12t%-8:c%5e % s")
         (org-agenda-show-inherited-tags t)
         (org-agenda-use-tag-inheritance t)
         (org-agenda-sorting-strategy-selected
          '(time-up priority-down deadline-up scheduled-up)))
    (unwind-protect
        (let* ((date '(5 27 2026))
               (summary
                (lambda (items)
                  (mapcar
                   (lambda (item)
                     (let* ((marker (or (get-text-property 0 'org-hd-marker item)
                                        (get-text-property 0 'org-marker item)))
                            (heading
                             (and (markerp marker)
                                  (marker-buffer marker)
                                  (with-current-buffer (marker-buffer marker)
                                    (save-excursion
                                      (goto-char marker)
                                      (org-get-heading t t t t))))))
                       (list (substring-no-properties item)
                             (get-text-property 0 'type item)
                             (get-text-property 0 'todo-state item)
                             (get-text-property 0 'priority item)
                             (get-text-property 0 'effort item)
                             (get-text-property 0 'effort-minutes item)
                             (get-text-property 0 'org-category item)
                             (get-text-property 0 'ts-date item)
                             heading)))
                   items)))
               (all (org-agenda-get-day-entries
                     file date
                     :deadline :scheduled :timestamp :closed))
               (deadline-only (org-agenda-get-day-entries
                               file date :deadline*))
               (scheduled-only (org-agenda-get-day-entries
                                file date :scheduled*))
               timestamp-sorts)
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            (dolist (strategy '((deadline-up)
                                (scheduled-up)
                                (ts-up)
                                (timestamp-up)))
              (let ((org-agenda-sorting-strategy-selected strategy))
                (goto-char (point-min))
                (search-forward "Scheduled")
                (beginning-of-line)
                (push (list strategy
                            (org-agenda-entry-get-agenda-timestamp
                             (point)))
                      timestamp-sorts))))
          (list (funcall summary all)
                (funcall summary deadline-only)
                (funcall summary scheduled-only)
                (nreverse timestamp-sorts)))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-file file))))"##,
    );
}

#[test]
fn org_agenda_date_shift_redo_marker_source_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file
                "org-agenda-date-shift" nil ".org"
                "#+CATEGORY: Shift
* TODO Window :work:ship:
SCHEDULED: <2026-05-27 Wed 09:00>
DEADLINE: <2026-05-29 Fri>
:PROPERTIES:
:Effort: 1:00
:END:
* TODO Range :work:call:
<2026-05-27 Wed 13:00-14:00>
* WAIT Future :home:
SCHEDULED: <2026-05-28 Thu 08:30>
"))
         (org-agenda-files (list file))
         (org-agenda-start-day "2026-05-27")
         (org-agenda-span 3)
         (org-agenda-start-on-weekday nil)
         (org-agenda-show-all-dates nil)
         (org-agenda-use-time-grid nil)
         (org-agenda-prefix-format "%?-12t%-8:c%5e %s")
         (org-timestamp-rounding-minutes '(0 15))
         (org-log-reschedule nil)
         (org-log-redeadline nil))
    (unwind-protect
        (progn
          (org-agenda-list nil "2026-05-27" 3)
          (with-current-buffer org-agenda-buffer-name
            (let ((line-summary
                   (lambda ()
                     (let (rows)
                       (save-excursion
                         (goto-char (point-min))
                         (while (re-search-forward
                                 "^[ \t]*Shift:.*\\(Window\\|Range\\|Future\\)"
                                 nil t)
                           (let* ((pos (line-beginning-position))
                                  (marker (or (get-text-property pos
                                                                 'org-hd-marker)
                                              (get-text-property pos
                                                                 'org-marker)))
                                  (heading
                                   (and (markerp marker)
                                        (marker-buffer marker)
                                        (with-current-buffer
                                            (marker-buffer marker)
                                          (save-excursion
                                            (goto-char marker)
                                            (org-get-heading t t t t))))))
                             (push (list
                                    (buffer-substring-no-properties
                                     pos (line-end-position))
                                    (get-text-property pos 'type)
                                    (get-text-property pos 'todo-state)
                                    (get-text-property pos 'time-of-day)
                                    (get-text-property pos 'duration)
                                    (get-text-property pos 'effort-minutes)
                                    heading)
                                   rows))))
                       (nreverse rows)))))
              (let ((initial (buffer-substring-no-properties
                              (point-min) (point-max)))
                    (initial-summary (funcall line-summary)))
                (org-agenda-filter-apply '("+work") 'tag t)
                (let ((filtered (buffer-substring-no-properties
                                 (point-min) (point-max)))
                      (tag-filter org-agenda-tag-filter))
                  (org-agenda-filter-remove-all)
                  (goto-char (point-min))
                  (search-forward "Window")
                  (beginning-of-line)
                  (org-agenda-date-later 2)
                  (let ((after-window-display
                         (buffer-substring-no-properties
                          (line-beginning-position) (line-end-position))))
                    (goto-char (point-min))
                    (search-forward "Range")
                    (beginning-of-line)
                    (org-agenda-date-later-hours 2)
                    (let ((after-range-display
                           (buffer-substring-no-properties
                            (line-beginning-position) (line-end-position)))
                          (source-after-edits
                           (with-current-buffer (find-file-noselect file)
                             (buffer-substring-no-properties
                              (point-min) (point-max)))))
                      (org-agenda-redo)
                      (let ((after-redo
                             (buffer-substring-no-properties
                              (point-min) (point-max)))
                            (after-redo-summary (funcall line-summary)))
                        (list
                         (mapcar (lambda (needle)
                                   (not (null
                                         (string-match-p needle initial))))
                                 '("Window" "Range" "Future"
                                   "09:00" "13:00-14:00" "08:30"))
                         initial-summary
                         tag-filter
                         (mapcar (lambda (needle)
                                   (not (null
                                         (string-match-p needle filtered))))
                                 '("Window" "Range" "Future"))
                         after-window-display
                         after-range-display
                         (mapcar (lambda (needle)
                                   (not (null
                                         (string-match-p needle
                                                         source-after-edits))))
                                 '("SCHEDULED: <2026-05-29 Fri 09:00>"
                                   "<2026-05-27 Wed 15:00-16:00>"
                                   "DEADLINE: <2026-05-29 Fri>"
                                   "SCHEDULED: <2026-05-28 Thu 08:30>"))
                         (mapcar (lambda (needle)
                                   (not (null
                                         (string-match-p needle after-redo))))
                                 '("Window" "Range" "Future"
                                   "15:00-16:00" "08:30"))
                          after-redo-summary
                          source-after-edits)))))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-file file))))"##,
    );
}

#[test]
fn org_agenda_clockreport_mode_habit_consistency_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (require 'org-habit)
  (require 'org-clock)
  (let* ((root (make-temp-file "org-agenda-cr" t))
         (file (expand-file-name "tasks.org" root))
         (org-agenda-files (list file))
         (org-agenda-span 'week)
         (org-agenda-start-day "2026-05-25")
         (org-agenda-start-on-weekday 1)
         (org-agenda-clockreport-mode t)
         (org-agenda-show-log t)
         (org-agenda-log-mode-items '(closed clock state))
         (org-habit-show-habits t)
         (org-habit-show-all-today t)
         (org-habit-following-days 7)
         (org-habit-preceding-days 14)
         (org-agenda-use-time-grid nil))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "#+CATEGORY: Work\n")
            (insert "* TODO Write report :work:\n")
            (insert "SCHEDULED: <2026-05-27 Wed .+2d/4d>\n")
            (insert ":PROPERTIES:\n:STYLE: habit\n:Effort: 2:00\n:END:\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-25 Sun 10:00]--[2026-05-25 Sun 11:30] =>  1:30\n")
            (insert "CLOCK: [2026-05-26 Mon 09:00]--[2026-05-26 Mon 10:00] =>  1:00\n")
            (insert ":END:\n")
            (insert "* TODO Review code :work:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-26 Mon 14:00]--[2026-05-26 Mon 15:30] =>  1:30\n")
            (insert ":END:\n")
            (insert "* DONE Deploy :ops:\n")
            (insert "CLOSED: [2026-05-26 Mon 16:00]\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-26 Mon 15:30]--[2026-05-26 Mon 16:00] =>  0:30\n")
            (insert ":END:\n"))
          (org-agenda-list nil "2026-05-25" 7)
          (with-current-buffer org-agenda-buffer-name
            (let ((agenda-text
                   (buffer-substring-no-properties (point-min) (point-max)))
                  (has-habit nil)
                  (has-clockreport nil)
                  (has-clocked nil))
              (goto-char (point-min))
              (setq has-habit
                    (not (null (re-search-forward "habit" nil t))))
              (goto-char (point-min))
              (setq has-clockreport
                    (not (null (re-search-forward "Clock report" nil t))))
              (goto-char (point-min))
              (setq has-clocked
                    (not (null (re-search-forward "Clocked" nil t))))
              (list has-habit
                    has-clockreport
                    has-clocked
                    (replace-regexp-in-string
                     (regexp-quote root) "<root>"
                     agenda-text)))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_agenda_log_mode_clock_state_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((root (make-temp-file "org-agenda-log" t))
         (file (expand-file-name "tasks.org" root))
         (org-agenda-files (list file))
         (org-agenda-span 'day)
         (org-agenda-start-day "2026-05-27")
         (org-agenda-start-on-weekday nil)
         (org-agenda-show-log t)
         (org-agenda-log-mode-items '(closed clock state))
         (org-agenda-use-time-grid nil))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "#+CATEGORY: Work\n")
            (insert "* TODO Write report\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:30] =>  1:30\n")
            (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 11:45] =>  0:45\n")
            (insert ":END:\n")
            (insert "* DONE Deploy\n")
            (insert "CLOSED: [2026-05-27 Wed 12:00]\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 12:00]--[2026-05-27 Wed 12:30] =>  0:30\n")
            (insert "- State \"DONE\"  from \"TODO\"  [2026-05-27 Wed 12:00]\n")
            (insert ":END:\n")
            (insert "* Review code\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 14:00]--[2026-05-27 Wed 15:00] =>  1:00\n")
            (insert ":END:\n"))
          (org-agenda-list nil "2026-05-27" 1)
          (with-current-buffer org-agenda-buffer-name
            (let ((agenda-text
                   (buffer-substring-no-properties (point-min) (point-max)))
                  ;; Count specific patterns in agenda
                  (clocked-count
                   (let ((c 0) (s 0))
                     (while (string-match "Clocked" agenda-text s)
                       (setq s (match-end 0) c (1+ c)))
                     c))
                  (closed-count
                   (let ((c 0) (s 0))
                     (while (string-match "Closed" agenda-text s)
                       (setq s (match-end 0) c (1+ c)))
                     c))
                  (state-count
                   (let ((c 0) (s 0))
                     (while (string-match "State" agenda-text s)
                       (setq s (match-end 0) c (1+ c)))
                     c))
                  ;; Extract time entries
                  (time-entries
                   (let ((entries nil) (s 0))
                     (while (string-match
                             "\\([0-9]+:[0-9]+\\)\\s-+.*Clocked" agenda-text s)
                       (push (match-string 1 agenda-text) entries)
                       (setq s (match-end 0)))
                     (nreverse entries))))
              (list clocked-count
                    closed-count
                    state-count
                    time-entries
                    (replace-regexp-in-string
                     (regexp-quote root) "<root>" agenda-text)))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_agenda_filter_tag_todo_match_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((root (make-temp-file "org-agenda-filter" t))
         (file (expand-file-name "tasks.org" root))
         (org-agenda-files (list file))
         (org-agenda-span 'day)
         (org-agenda-start-day "2026-05-27")
         (org-agenda-start-on-weekday nil))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* TODO Write report :work:urgent:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert "* DONE Review code :work:\n")
            (insert "CLOSED: [2026-05-27 Wed]\n")
            (insert "* TODO Buy groceries :home:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert "* WAIT Fix bug :work:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert "* DONE Deploy :ops:\n")
            (insert "CLOSED: [2026-05-27 Wed]\n"))
          (org-agenda-list nil "2026-05-27" 1)
          (with-current-buffer org-agenda-buffer-name
            (let ((full-text (buffer-substring-no-properties
                              (point-min) (point-max))))
              (org-agenda-filter-apply '("+work") 'tag)
              (let ((work-text (buffer-substring-no-properties
                                (point-min) (point-max))))
                (org-agenda-filter-remove-all)
                (org-agenda-filter-apply '("+TODO") 'todo)
                (let ((todo-text (buffer-substring-no-properties
                                  (point-min) (point-max))))
                  (org-agenda-filter-remove-all)
                  (org-agenda-filter-apply '("+DONE") 'todo)
                  (let ((done-text (buffer-substring-no-properties
                                    (point-min) (point-max))))
                    (list (replace-regexp-in-string
                           (regexp-quote root) "<root>" full-text)
                          (replace-regexp-in-string
                           (regexp-quote root) "<root>" work-text)
                          (replace-regexp-in-string
                           (regexp-quote root) "<root>" todo-text)
                          (replace-regexp-in-string
                           (regexp-quote root) "<root>" done-text))))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_agenda_date_shift_redo_source_mutation_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((root (make-temp-file "org-agenda-shift" t))
         (file (expand-file-name "tasks.org" root))
         (org-agenda-files (list file))
         (org-agenda-span 'week)
         (org-agenda-start-day "2026-05-25")
         (org-agenda-start-on-weekday 1)
         (org-agenda-use-time-grid nil))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* TODO Alpha\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert "* TODO Beta\n")
            (insert "SCHEDULED: <2026-05-28 Thu>\n")
            (insert "* TODO Gamma\n")
            (insert "DEADLINE: <2026-05-29 Fri>\n"))
          (org-agenda-list nil "2026-05-25" 7)
          (with-current-buffer org-agenda-buffer-name
            (let ((initial (replace-regexp-in-string
                            (regexp-quote root) "<root>"
                            (buffer-substring-no-properties
                             (point-min) (point-max)))))
              (goto-char (point-min))
              (search-forward "Alpha")
              (beginning-of-line)
              (org-agenda-date-later 1)
              (let ((after-shift (replace-regexp-in-string
                                  (regexp-quote root) "<root>"
                                  (buffer-substring-no-properties
                                   (point-min) (point-max)))))
                (org-agenda-redo)
                (let ((after-redo (replace-regexp-in-string
                                   (regexp-quote root) "<root>"
                                   (buffer-substring-no-properties
                                    (point-min) (point-max)))))
                  (let ((source-content
                         (with-current-buffer (find-file-noselect file)
                           (prog1 (buffer-substring-no-properties
                                   (point-min) (point-max))
                             (kill-buffer)))))
                    (list initial
                          after-shift
                          after-redo
                          (replace-regexp-in-string
                           (regexp-quote root) "<root>"
                           source-content))))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_agenda_bulk_mark_tag_filter_effort_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((root (make-temp-file "org-agenda-bulk" t))
         (file (expand-file-name "tasks.org" root))
         (org-agenda-files (list file))
         (org-agenda-span 'day)
         (org-agenda-start-day "2026-05-27")
         (org-agenda-start-on-weekday nil))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* TODO Alpha :work:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":PROPERTIES:\n:Effort: 2:00\n:END:\n")
            (insert "* TODO Beta :home:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":PROPERTIES:\n:Effort: 0:30\n:END:\n")
            (insert "* TODO Gamma :work:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":PROPERTIES:\n:Effort: 1:00\n:END:\n"))
          (org-agenda-list nil "2026-05-27" 1)
          (with-current-buffer org-agenda-buffer-name
            (let ((initial (replace-regexp-in-string
                            (regexp-quote root) "<root>"
                            (buffer-substring-no-properties
                             (point-min) (point-max)))))
              ;; Tag filter +work
              (org-agenda-filter-apply '("+work") 'tag)
              (let ((work-filter (replace-regexp-in-string
                                  (regexp-quote root) "<root>"
                                  (buffer-substring-no-properties
                                   (point-min) (point-max)))))
                ;; Clear and apply effort filter
                (org-agenda-filter-remove-all)
                (org-agenda-filter-apply '("1:00") 'effort)
                (let ((effort-filter (replace-regexp-in-string
                                      (regexp-quote root) "<root>"
                                      (buffer-substring-no-properties
                                       (point-min) (point-max)))))
                  (list initial
                        work-filter
                        effort-filter)))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_agenda_entry_text_switch_context_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((root (make-temp-file "org-agenda-entry" t))
         (file (expand-file-name "tasks.org" root))
         (org-agenda-files (list file))
         (org-agenda-span 'day)
         (org-agenda-start-day "2026-05-27")
         (org-agenda-start-on-weekday nil))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* TODO Alpha :work:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":PROPERTIES:\n:Effort: 2:00\n:END:\n")
            (insert "Alpha body paragraph.\n")
            (insert "* WAIT Beta :home:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":PROPERTIES:\n:Effort: 0:30\n:END:\n")
            (insert "Beta body paragraph.\n"))
          (org-agenda-list nil "2026-05-27" 1)
          (with-current-buffer org-agenda-buffer-name
            (let ((agenda-text
                   (replace-regexp-in-string
                    (regexp-quote root) "<root>"
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))
              (goto-char (point-min))
              (search-forward "Alpha")
              (beginning-of-line)
              (let ((entry-text
                     (org-agenda-get-some-entry-text
                      (point) 100)))
                (let ((cat (org-entry-get (point) "CATEGORY"))
                      (effort (org-entry-get (point) "Effort")))
                  (list agenda-text entry-text cat effort)))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_agenda_day_entries_properties_timestamp_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((root (make-temp-file "org-agenda-day" t))
         (file (expand-file-name "tasks.org" root))
         (org-agenda-files (list file))
         (org-agenda-span 'week)
         (org-agenda-start-day "2026-05-25")
         (org-agenda-start-on-weekday 1)
         (org-agenda-use-time-grid nil))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* TODO Morning :work:\n")
            (insert "SCHEDULED: <2026-05-26 Mon 09:00-10:00>\n")
            (insert ":PROPERTIES:\n:Effort: 1:00\n:END:\n")
            (insert "* TODO Afternoon :home:\n")
            (insert "SCHEDULED: <2026-05-27 Wed 14:00-15:30>\n")
            (insert ":PROPERTIES:\n:Effort: 1:30\n:END:\n")
            (insert "* DONE Completed\n")
            (insert "CLOSED: [2026-05-26 Mon 16:00]\n")
            (insert "* WAIT Pending\n")
            (insert "DEADLINE: <2026-05-28 Thu>\n")
            (insert "* TODO Weekend\n")
            (insert "SCHEDULED: <2026-05-31 Sat>\n"))
          (org-agenda-list nil "2026-05-25" 7)
          (with-current-buffer org-agenda-buffer-name
            (let ((agenda-text
                   (replace-regexp-in-string
                    (regexp-quote root) "<root>"
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))
              (let ((mon-count (let ((c 0) (s 0))
                                 (while (string-match "Monday" agenda-text s)
                                   (setq s (match-end 0) c (1+ c)))
                                 c))
                    (wed-count (let ((c 0) (s 0))
                                 (while (string-match "Wednesday" agenda-text s)
                                   (setq s (match-end 0) c (1+ c)))
                                 c)))
                (let ((has-0900 (string-match-p "09:00" agenda-text))
                      (has-1400 (string-match-p "14:00" agenda-text)))
                  (list agenda-text mon-count wed-count has-0900 has-1400)))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_agenda_clockreport_filter_effort_todo_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((root (make-temp-file "org-agenda-cr" t))
         (file (expand-file-name "tasks.org" root))
         (org-agenda-files (list file))
         (org-agenda-span 'day)
         (org-agenda-start-day "2026-05-27")
         (org-agenda-start-on-weekday nil)
         (org-agenda-clockreport-mode t)
         (org-agenda-clock-reporting-file file))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* TODO Write report :work:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":PROPERTIES:\n:Effort: 2:00\n:END:\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:30] =>  1:30\n")
            (insert "CLOCK: [2026-05-27 Wed 14:00]--[2026-05-27 Wed 15:00] =>  1:00\n")
            (insert ":END:\n")
            (insert "* TODO Review code :work:\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":PROPERTIES:\n:Effort: 1:00\n:END:\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 12:00] =>  1:00\n")
            (insert ":END:\n")
            (insert "* DONE Deploy :ops:\n")
            (insert "CLOSED: [2026-05-27 Wed]\n")
            (insert ":PROPERTIES:\n:Effort: 0:30\n:END:\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 16:00]--[2026-05-27 Wed 16:30] =>  0:30\n")
            (insert ":END:\n"))
          (org-agenda-list nil "2026-05-27" 1)
          (with-current-buffer org-agenda-buffer-name
            (let ((full-text (replace-regexp-in-string
                              (regexp-quote root) "<root>"
                              (buffer-substring-no-properties
                               (point-min) (point-max)))))
              (org-agenda-filter-apply '("+work") 'tag)
              (let ((work-text (replace-regexp-in-string
                                (regexp-quote root) "<root>"
                                (buffer-substring-no-properties
                                 (point-min) (point-max)))))
                (org-agenda-filter-remove-all)
                (org-agenda-filter-apply '("1:00") 'effort)
                (let ((effort-text (replace-regexp-in-string
                                    (regexp-quote root) "<root>"
                                    (buffer-substring-no-properties
                                     (point-min) (point-max)))))
                  (org-agenda-filter-remove-all)
                  (org-agenda-filter-apply '("+TODO") 'todo)
                  (let ((todo-text (replace-regexp-in-string
                                    (regexp-quote root) "<root>"
                                    (buffer-substring-no-properties
                                     (point-min) (point-max)))))
                    (list full-text work-text effort-text todo-text))))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_agenda_list_edit_todo_reagenda_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((root (make-temp-file "org-agenda-edit-" t))
         (org-agenda-files (list (expand-file-name "a.org" root)))
         (org-agenda-buffer-name "*TestAgendaEdit*")
         (file (expand-file-name "a.org" root)))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* TODO Alpha\nSCHEDULED: <2026-05-28 Wed>\n")
            (insert "* DONE Beta\nSCHEDULED: <2026-05-28 Wed>\n")
            (insert "* TODO Gamma :work:\nSCHEDULED: <2026-05-28 Wed>\n")
            (insert "* NEXT Delta\nSCHEDULED: <2026-05-28 Wed>\n"))
          ;; First agenda
          (org-agenda-list nil "2026-05-28" 1)
          (with-current-buffer org-agenda-buffer-name
            (let ((agenda1 (replace-regexp-in-string
                            (regexp-quote root) "<root>"
                            (buffer-substring-no-properties
                             (point-min) (point-max)))))
              (org-agenda-quit)
              ;; Edit: change Alpha to DONE
              (with-current-buffer (find-file-noselect file)
                (goto-char (point-min))
                (search-forward "TODO Alpha")
                (replace-match "DONE Alpha"))
              ;; Second agenda
              (org-agenda-list nil "2026-05-28" 1)
              (with-current-buffer org-agenda-buffer-name
                (let ((agenda2 (replace-regexp-in-string
                                (regexp-quote root) "<root>"
                                (buffer-substring-no-properties
                                 (point-min) (point-max)))))
                  (org-agenda-quit)
                  (list agenda1 agenda2))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-directory root t))))"##,
    );
}
