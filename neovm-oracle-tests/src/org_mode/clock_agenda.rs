use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_agenda_mutate_todo_schedule_deadline_tags_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (let* ((file (make-temp-file "org-agenda-mutate" nil ".org"
                               "#+TODO: TODO WAIT | DONE
#+CATEGORY: Mutate
* TODO Alpha :old:
SCHEDULED: <2026-05-27 Wed>
* WAIT Beta
DEADLINE: <2026-05-28 Thu>
"))
         (org-agenda-files (list file))
         (org-agenda-show-all-dates nil)
         (org-agenda-use-time-grid nil)
         (org-agenda-prefix-format "%-8:c%?-12t% s")
         (org-agenda-start-day "2026-05-27")
         (org-agenda-span 5)
         (org-agenda-start-on-weekday nil))
    (unwind-protect
        (progn
          (org-agenda-list nil "2026-05-27" 5)
          (with-current-buffer org-agenda-buffer-name
            (goto-char (point-min))
            (search-forward "Alpha")
            (beginning-of-line)
            (let ((org-log-done nil)
                  (org-log-reschedule nil)
                  (org-log-redeadline nil))
              (org-agenda-schedule nil "2026-06-01")
              (org-agenda-deadline nil "2026-06-03")
              (org-agenda-set-tags "new" 'on)
              (org-agenda-set-tags "old" 'off)
              (org-agenda-todo "DONE"))
            (let ((agenda-after
                   (buffer-substring-no-properties
                    (point-min) (point-max))))
              (with-current-buffer (find-file-noselect file)
                (list agenda-after
                      (buffer-substring-no-properties
                       (point-min) (point-max)))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (when (get-file-buffer file) (kill-buffer (get-file-buffer file)))
      (when (file-exists-p file) (delete-file file)))))"##,
    );
}

#[test]
fn org_clock_report_dynamic_block_update_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (org-mode)
    (insert "* Project\n")
    (insert "** TODO Alpha\n")
    (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:15] =>  1:15\n")
    (insert "** TODO Beta\n")
    (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 11:45] =>  0:45\n")
    (goto-char (point-min))
    (let ((org-clock-clocktable-default-properties
           '(:maxlevel 3 :scope file :block "2026-05-27" :link nil)))
      (org-clock-report)
      (let ((first (buffer-substring-no-properties
                    (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "Beta")
        (forward-line 1)
        (delete-region (line-beginning-position) (line-end-position))
        (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 12:30] =>  1:30")
        (goto-char (point-min))
        (org-clock-report '(4))
        (list first
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_clock_sum_filtered_properties_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (org-mode)
    (insert "* Project :work:\n")
    (insert "** TODO Alpha :billable:\n")
    (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:15] =>  1:15\n")
    (insert "** TODO Beta :internal:\n")
    (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 11:45] =>  0:45\n")
    (goto-char (point-min))
    (org-clock-sum
     "2026-05-27"
     "2026-05-28"
     (lambda () (member "billable" (org-get-tags nil t)))
     :probe-clock-minutes)
    (let (out)
      (goto-char (point-min))
      (while (re-search-forward "^\\*+ " nil t)
        (push (list (org-get-heading t t t t)
                    (get-text-property
                     (line-beginning-position)
                     :probe-clock-minutes)
                    (get-text-property
                     (line-beginning-position)
                     :org-clock-force-headline-inclusion))
              out))
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (list (nreverse out)
            (org-clock-sum-current-item "2026-05-27")))))"##,
    );
}

#[test]
fn org_clocktable_match_properties_inherit_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (org-mode)
    (insert "#+CATEGORY: ClockProbe\n")
    (insert "* Project :work:\n")
    (insert ":PROPERTIES:\n:Client: Acme\n:END:\n")
    (insert "** TODO Alpha :billable:\n")
    (insert "SCHEDULED: <2026-05-27 Wed>\n")
    (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
    (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:20] =>  1:20\n")
    (insert "** TODO Beta :internal:\n")
    (insert "DEADLINE: <2026-05-27 Wed>\n")
    (insert ":PROPERTIES:\n:Owner: Bea\n:END:\n")
    (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 12:00] =>  1:00\n")
    (insert "** DONE Gamma :billable:\n")
    (insert "CLOCK: [2026-05-28 Thu 09:00]--[2026-05-28 Thu 09:30] =>  0:30\n")
    (let* ((data
            (org-clock-get-table-data
             "clock.org"
             '(:maxlevel 3 :block "2026-05-27" :match "+billable"
               :tags t :timestamp t :link t
               :properties ("Owner" "Client") :inherit-props t)))
           (rows
            (mapcar (lambda (row)
                      (list (nth 0 row)
                            (substring-no-properties (nth 1 row))
                            (mapcar #'substring-no-properties (nth 2 row))
                            (nth 3 row)
                            (nth 4 row)
                            (nth 5 row)))
                    (nth 2 data))))
      (list (list (nth 0 data) (nth 1 data) rows)
            (buffer-substring-no-properties
             (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_clock_cancel_history_goto_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Alpha\n")
    (insert "* TODO Beta\n")
    (let ((org-clock-into-drawer "LOGBOOK")
          (org-clock-history-length 5)
          (org-clock-out-remove-zero-time-clocks t)
          (org-clock-persist nil)
          (org-clock-goto-may-find-recent-task t))
      (goto-char (point-min))
      (search-forward "Alpha")
      (beginning-of-line)
      (let ((start-a (encode-time 0 0 9 27 5 2026)))
        (org-clock-in nil start-a)
        (org-clock-out nil t (encode-time 0 30 9 27 5 2026)))
      (goto-char (point-min))
      (search-forward "Beta")
      (beginning-of-line)
      (org-clock-in nil (encode-time 0 0 10 27 5 2026))
      (let ((during (list (org-clocking-p)
                          org-clock-current-task
                          (length org-clock-history)
                          (markerp org-clock-marker))))
        (org-clock-cancel)
        (let ((after-cancel (list (org-clocking-p)
                                  org-clock-current-task
                                  (length org-clock-history))))
          (org-clock-goto)
          (list during
                after-cancel
                (org-get-heading t t t t)
                (mapcar (lambda (m)
                          (and (markerp m)
                               (marker-buffer m)
                               (with-current-buffer (marker-buffer m)
                                 (save-excursion
                                   (goto-char m)
                                   (org-get-heading t t t t)))))
                        org-clock-history)
                (buffer-substring-no-properties
                 (point-min) (point-max))))))))"##,
    );
}

#[test]
fn org_clock_resolve_open_clock_ranges_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Open\n")
    (insert "CLOCK: [2026-05-27 Wed 09:00]\n")
    (insert "* TODO Closed\n")
    (insert "CLOCK: [2026-05-27 Wed 08:00]--[2026-05-27 Wed 08:45] =>  0:45\n")
    (goto-char (point-min))
    (search-forward "CLOCK: [2026-05-27 Wed 09:00]")
    (let* ((marker (copy-marker (line-beginning-position)))
           (clock (cons marker (encode-time 0 0 9 27 5 2026)))
           (resolve-to (encode-time 0 20 9 27 5 2026)))
      (org-clock-resolve-clock clock resolve-to nil t nil nil)
      (goto-char (point-min))
      (org-clock-sum "2026-05-27" "2026-05-28" nil :clock-mins)
      (let (props)
        (goto-char (point-min))
        (while (re-search-forward "^\\*+ " nil t)
          (push (list (org-get-heading t t t t)
                      (get-text-property
                       (line-beginning-position) :clock-mins))
                props))
        (list (nreverse props)
              (org-clock-sum-current-item "2026-05-27")
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}
