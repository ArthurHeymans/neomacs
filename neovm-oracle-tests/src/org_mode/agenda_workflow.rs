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
