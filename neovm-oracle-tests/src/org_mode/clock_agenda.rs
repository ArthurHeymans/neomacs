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

#[test]
fn org_clocktable_shift_special_range_regenerate_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (let ((org-extend-today-until 0)
          (org-clock-clocktable-default-properties nil))
      (org-mode)
      (insert "#+BEGIN: clocktable :scope file :block 2026-05-27 :maxlevel 4 :link nil :compact t :tags t\n")
      (insert "#+END:\n\n")
      (insert "* Client :billable:\n")
      (insert "** TODO Alpha :dev:\n")
      (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:30] =>  1:30\n")
      (insert "** TODO Beta :qa:\n")
      (insert "CLOCK: [2026-05-28 Thu 11:00]--[2026-05-28 Thu 12:15] =>  1:15\n")
      (insert "** TODO Gamma :ops:\n")
      (insert "CLOCK: [2026-05-29 Fri 13:00]--[2026-05-29 Fri 14:45] =>  1:45\n")
      (insert "* Internal :admin:\n")
      (insert "** TODO Planning\n")
      (insert "CLOCK: [2026-05-27 Wed 16:00]--[2026-05-27 Wed 16:20] =>  0:20\n")
      (let ((snapshot
             (lambda (label)
               (save-excursion
                 (goto-char (point-min))
                 (let ((begin-line
                        (buffer-substring-no-properties
                         (line-beginning-position) (line-end-position)))
                       (table
                        (buffer-substring-no-properties
                         (point-min)
                         (save-excursion
                           (search-forward "#+END:")
                           (line-end-position)))))
                   (list label
                         begin-line
                         (mapcar (lambda (needle)
                                   (not (null (string-match-p needle table))))
                                 '("Alpha" "Beta" "Gamma" "Planning"
                                   "1:30" "1:15" "1:45" "0:20"))
                         table)))))
            states)
        (goto-char (point-min))
        (org-update-dblock)
        (push (funcall snapshot 'day-27) states)
        (goto-char (point-min))
        (org-clocktable-shift 'right 1)
        (push (funcall snapshot 'day-28) states)
        (goto-char (point-min))
        (org-clocktable-shift 'right 1)
        (push (funcall snapshot 'day-29) states)
        (goto-char (point-min))
        (org-clocktable-shift 'left 2)
        (push (funcall snapshot 'back-27) states)
        (goto-char (point-min))
        (search-forward ":block")
        (delete-region (point) (progn (forward-word 1) (point)))
        (insert " 2026-05")
        (goto-char (point-min))
        (org-update-dblock)
        (push (funcall snapshot 'month) states)
        (list (mapcar (lambda (key)
                        (org-clock-special-range
                         key (encode-time 0 0 12 27 5 2026) t 1 1))
                      '(today yesterday "2026-05-27" "2026-05"
                        "2026-W22" "2026-Q2"))
              (nreverse states)
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_clock_shift_display_overlay_cleanup_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (let ((org-clock-into-drawer "LOGBOOK")
          (org-clock-display-default-range 'today)
          (org-clock-out-remove-zero-time-clocks t))
      (org-mode)
      (insert "* TODO Alpha\n")
      (insert ":LOGBOOK:\n")
      (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:00] =>  1:00\n")
      (insert ":END:\n")
      (insert "* TODO Beta\n")
      (insert ":LOGBOOK:\n")
      (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 11:00] =>  0:00\n")
      (insert ":END:\n")
      (let ((snapshot
             (lambda (label)
               (org-clock-sum "2026-05-27" "2026-05-28" nil
                              :clock-sum-probe)
               (list label
                     (mapcar
                      (lambda (needle)
                        (save-excursion
                          (goto-char (point-min))
                          (search-forward needle)
                          (list needle
                                (org-get-heading t t t t)
                                (get-text-property
                                 (line-beginning-position)
                                 :clock-sum-probe)
                                (mapcar
                                 (lambda (ov)
                                   (list (overlay-start ov)
                                         (overlay-end ov)
                                         (overlay-get ov 'face)
                                         (overlay-get ov 'display)))
                                 (overlays-at (line-end-position)))))))
                      '("Alpha" "Beta"))
                     (buffer-substring-no-properties
                      (point-min) (point-max))))))
        (goto-char (point-min))
        (search-forward "09:00")
        (org-clock-timestamps-up 15)
        (search-forward "10:00")
        (org-clock-timestamps-down 10)
        (let ((after-shift (funcall snapshot 'after-shift)))
          (org-clock-display '(4))
          (let ((after-display (funcall snapshot 'after-display)))
            (org-clock-remove-overlays)
            (goto-char (point-min))
            (search-forward "Beta")
            (beginning-of-line)
            (org-clock-in nil (encode-time 0 0 12 27 5 2026))
            (org-clock-out nil t (encode-time 0 0 12 27 5 2026))
            (org-clock-remove-empty-clock-drawer)
            (list after-shift
                  after-display
                  (funcall snapshot 'after-cleanup)))))))"##,
    );
}

#[test]
fn org_clock_interrupt_resume_state_history_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (with-temp-buffer
    (let ((org-todo-keywords '((sequence "TODO" "NEXT" "WAIT" "|" "DONE")))
          (org-clock-into-drawer "LOGBOOK")
          (org-clock-history-length 4)
          (org-clock-persist nil)
          (org-clock-continuously nil)
          (org-clock-clocked-in-display 'both)
          (org-clock-frame-title-format '(org-mode-line-string))
          (org-clock-string-limit 40)
          (org-clock-in-switch-to-state
           (lambda (state)
             (cond ((member state '("TODO" "WAIT")) "NEXT")
                   (t nil))))
          (org-clock-out-switch-to-state
           (lambda (state)
             (cond ((string= state "NEXT") "WAIT")
                   (t nil))))
          (org-log-done nil)
          (global-mode-string nil)
          (frame-title-format '("base"))
          (fake-timers nil)
          (cancelled nil)
          (events nil))
      (cl-labels
          ((clock-time (hour minute)
             (encode-time 0 minute hour 27 5 2026))
           (marker-heading (marker)
             (and (markerp marker)
                  (marker-buffer marker)
                  (with-current-buffer (marker-buffer marker)
                    (save-excursion
                      (goto-char marker)
                      (org-get-heading t t t t)))))
           (snapshot
            (label)
            (list label
                  (org-clocking-p)
                  org-clock-current-task
                  (and org-mode-line-string
                       (substring-no-properties org-mode-line-string))
                  global-mode-string
                  frame-title-format
                  (marker-heading org-clock-marker)
                  (marker-heading org-clock-hd-marker)
                  (marker-heading org-clock-interrupted-task)
                  (mapcar #'marker-heading org-clock-history)
                  (mapcar
                   (lambda (title)
                     (save-excursion
                       (goto-char (point-min))
                       (search-forward title)
                       (beginning-of-line)
                       (list title
                             (org-get-todo-state)
                             (org-entry-get nil "Effort")
                             (org-clock-sum-current-item "2026-05-27")
                             (buffer-substring-no-properties
                              (point)
                              (save-excursion
                                (org-end-of-subtree t t)
                                (point)))))))
                   '("Alpha" "Beta" "Gamma"))
                  (nreverse (copy-sequence events))
                  (nreverse (copy-sequence cancelled))
                  (nreverse (copy-sequence fake-timers)))))
        (cl-letf (((symbol-function 'run-with-timer)
                   (lambda (secs repeat function &rest args)
                     (let ((timer (list :timer secs repeat function args)))
                       (push timer fake-timers)
                       timer)))
                  ((symbol-function 'timerp)
                   (lambda (object)
                     (and (consp object) (eq (car object) :timer))))
                  ((symbol-function 'cancel-timer)
                   (lambda (timer) (push timer cancelled) nil))
                  ((symbol-function 'force-mode-line-update)
                   (lambda (&rest _) (push 'force-mode-line events)))
                  ((symbol-function 'org-current-time)
                   (lambda (&rest _) (clock-time 9 20))))
          (org-mode)
          (insert "* TODO Alpha\n")
          (insert ":PROPERTIES:\n:Effort: 0:45\n:END:\n")
          (insert "* TODO Beta\n")
          (insert ":PROPERTIES:\n:Effort: 1:00\n:END:\n")
          (insert "* WAIT Gamma\n")
          (insert ":PROPERTIES:\n:Effort: 0:15\n:END:\n")
          (add-hook 'org-clock-in-hook
                    (lambda () (push (list 'in org-clock-current-task)
                                     events))
                    nil t)
          (add-hook 'org-clock-out-hook
                    (lambda () (push (list 'out
                                           org-clock-current-task
                                           org-clock-out-removed-last-clock)
                                     events))
                    nil t)
          (add-hook 'org-clock-cancel-hook
                    (lambda () (push 'cancel events)) nil t)
          (let (states)
            (goto-char (point-min))
            (search-forward "Alpha")
            (beginning-of-line)
            (org-clock-in nil (clock-time 9 0))
            (push (snapshot 'alpha-in) states)
            (goto-char (point-min))
            (search-forward "Beta")
            (beginning-of-line)
            (org-clock-in nil (clock-time 9 20))
            (push (snapshot 'beta-interrupts-alpha) states)
            (org-clock-out nil t (clock-time 10 0))
            (push (snapshot 'beta-out) states)
            (org-clock-in-last '(16))
            (push (snapshot 'clock-in-last-from-out-time) states)
            (org-clock-modify-effort-estimate "+0:30")
            (push (snapshot 'effort-modified) states)
            (org-clock-out nil t (clock-time 10 15))
            (push (snapshot 'resumed-out) states)
            (goto-char (point-min))
            (search-forward "Gamma")
            (beginning-of-line)
            (org-clock-in nil (clock-time 11 0))
            (push (snapshot 'gamma-in) states)
            (org-clock-cancel)
            (org-clock-remove-empty-clock-drawer)
            (push (snapshot 'gamma-cancel-cleanup) states)
            (list (nreverse states)
                  (buffer-substring-no-properties
                   (point-min) (point-max))
                  org-clock-out-time
                  org-clock-leftover-time
                  org-clock-has-been-used))))))"##,
    );
}

#[test]
fn org_clocktable_agenda_custom_formatter_regenerate_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (require 'org-agenda)
  (let* ((root (make-temp-file "org-clocktable-agenda" t))
         (file-a (expand-file-name "alpha.org" root))
         (file-b (expand-file-name "beta.org" root))
         (org-agenda-files (list file-a file-b))
         (formatter-calls nil))
    (unwind-protect
        (progn
          (with-temp-file file-a
            (insert "#+CATEGORY: AlphaFile\n")
            (insert "* Client :billable:\n")
            (insert ":PROPERTIES:\n:Owner: Ada\n:END:\n")
            (insert "** TODO Build :dev:\n")
            (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:30] =>  1:30\n")
            (insert "** TODO Review :review:\n")
            (insert "CLOCK: [2026-05-27 Wed 10:45]--[2026-05-27 Wed 11:15] =>  0:30\n"))
          (with-temp-file file-b
            (insert "#+CATEGORY: BetaFile\n")
            (insert "* Internal :internal:\n")
            (insert "** TODO Planning :admin:\n")
            (insert "CLOCK: [2026-05-27 Wed 08:00]--[2026-05-27 Wed 08:20] =>  0:20\n")
            (insert "* Client :billable:\n")
            (insert ":PROPERTIES:\n:Owner: Bea\n:END:\n")
            (insert "** TODO Test :qa:\n")
            (insert "CLOCK: [2026-05-27 Wed 12:00]--[2026-05-27 Wed 12:40] =>  0:40\n"))
          (with-temp-buffer
            (org-mode)
            (insert "#+TITLE: Clock Rollup\n")
            (insert "#+BEGIN: clocktable :scope agenda :block 2026-05-27 :maxlevel 4 :link nil :tags t :timestamp t :match \"+billable\" :formatter probe-clock-formatter\n")
            (insert "#+END:\n")
            (cl-letf (((symbol-function 'probe-clock-formatter)
                       (lambda (ipos tables params)
                         (push
                          (list ipos
                                (plist-get params :scope)
                                (plist-get params :block)
                                (plist-get params :maxlevel)
                                (plist-get params :match)
                                (plist-get params :tags)
                                (mapcar
                                 (lambda (table)
                                   (list (nth 0 table)
                                         (nth 1 table)
                                         (length (nth 2 table))
                                         (mapcar
                                          (lambda (row)
                                            (list (nth 0 row)
                                                  (substring-no-properties
                                                   (nth 1 row))
                                                  (mapcar
                                                   #'substring-no-properties
                                                   (nth 2 row))
                                                  (nth 3 row)
                                                  (nth 4 row)
                                                  (nth 5 row)))
                                          (nth 2 table))))
                                 tables))
                          formatter-calls)
                         (org-clocktable-write-default ipos tables params))))
              (goto-char (point-min))
              (search-forward "#+BEGIN")
              (beginning-of-line)
              (org-update-dblock)
              (let ((first
                     (replace-regexp-in-string
                      (regexp-quote root)
                      "<root>"
                      (buffer-substring-no-properties
                       (point-min) (point-max))))
                    (first-calls (copy-tree formatter-calls)))
                (with-current-buffer (find-file-noselect file-b)
                  (goto-char (point-min))
                  (search-forward "12:40")
                  (replace-match "13:10" t t)
                  (save-buffer))
                (setq formatter-calls nil)
                (goto-char (point-min))
                (search-forward "#+BEGIN")
                (beginning-of-line)
                (org-update-dblock)
                    (let ((second
                           (replace-regexp-in-string
                            (regexp-quote root)
                            "<root>"
                            (buffer-substring-no-properties
                             (point-min) (point-max))))
                          (direct
                           (let ((table
                                  (org-clock-get-table-data
                                   file-a
                                   '(:block "2026-05-27" :match "+billable"
                                     :maxlevel 4 :tags t :timestamp t))))
                             (list (nth 0 table)
                                   (nth 1 table)
                                   (length (nth 2 table))))))
                      (cl-labels
                          ((clean
                            (value)
                            (cond
                             ((stringp value)
                              (replace-regexp-in-string
                               (regexp-quote root) "<root>" value))
                             ((consp value)
                              (cons (clean (car value))
                                    (clean (cdr value))))
                             ((vectorp value)
                              (apply #'vector
                                     (mapcar #'clean
                                             (append value nil))))
                             (t value))))
                        (clean
                         (list (nreverse first-calls)
                               (nreverse formatter-calls)
                               first
                               second
                               direct))))))))))
      (dolist (file (list file-a file-b))
        (when (get-file-buffer file) (kill-buffer (get-file-buffer file))))
      (when (file-directory-p root) (delete-directory root t)))))"##,
    );
}

#[test]
fn org_clock_get_table_data_multi_file_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (let* ((root (make-temp-file "org-clock-deep" t))
         (file-a (expand-file-name "a.org" root))
         (file-b (expand-file-name "b.org" root)))
    (unwind-protect
        (progn
          (with-temp-file file-a
            (insert "* Project A :proj:\n")
            (insert "** Task A1\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:30] =>  1:30\n")
            (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 11:45] =>  0:45\n")
            (insert ":END:\n")
            (insert "** Task A2 :proj:\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 13:00]--[2026-05-27 Wed 14:00] =>  1:00\n")
            (insert ":END:\n"))
          (with-temp-file file-b
            (insert "* Project B\n")
            (insert "** Task B1\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 15:00]--[2026-05-27 Wed 16:00] =>  1:00\n")
            (insert ":END:\n"))
          (let* ((scope (list file-a file-b))
                 (table-a (org-clock-get-table-data
                           file-a '(:maxlevel 2 :scope file)))
                 (table-b (org-clock-get-table-data
                           file-b '(:maxlevel 2 :scope file)))
                 (rows-a (mapcar (lambda (row)
                                   (list (nth 0 row)
                                         (substring-no-properties (nth 1 row))
                                         (nth 4 row)))
                                 (nth 2 table-a)))
                 (rows-b (mapcar (lambda (row)
                                   (list (nth 0 row)
                                         (substring-no-properties (nth 1 row))
                                         (nth 4 row)))
                                 (nth 2 table-b)))
                 ;; Block clocktable
                 (block-tbl
                  (with-temp-buffer
                    (org-mode)
                    (insert "#+BEGIN: clocktable :maxlevel 3 :scope "
                            (if (> (length scope) 1) "agenda" "file")
                            "\n#+END:\n")
                    (goto-char (point-min))
                    (org-dblock-update)
                    (buffer-substring-no-properties
                     (point-min) (point-max)))))
            (list (nth 1 table-a)
                  rows-a
                  (nth 1 table-b)
                  rows-b
                  (replace-regexp-in-string
                   (regexp-quote root) "<root>" block-tbl))))
      (dolist (f (list file-a file-b))
        (when (get-file-buffer f) (kill-buffer (get-file-buffer f))))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_clock_log_agenda_timestamp_filter_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-agenda)
  (require 'org-clock)
  (let* ((root (make-temp-file "org-clock-filter" t))
         (file (expand-file-name "tasks.org" root))
         (org-agenda-files (list file))
         (org-agenda-span 'day)
         (org-agenda-start-day "2026-05-27")
         (org-agenda-start-on-weekday nil)
         (org-agenda-show-log t)
         (org-agenda-log-mode-items '(clock)))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* TODO Alpha\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:30] =>  1:30\n")
            (insert "CLOCK: [2026-05-27 Wed 14:00]--[2026-05-27 Wed 15:00] =>  1:00\n")
            (insert ":END:\n")
            (insert "* TODO Beta\n")
            (insert "SCHEDULED: <2026-05-27 Wed>\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 12:30] =>  1:30\n")
            (insert ":END:\n"))
          ;; Clock report mode
          (let ((org-agenda-clockreport-mode t)
                (org-agenda-clock-reporting-file file))
            (org-agenda-list nil "2026-05-27" 1)
            (with-current-buffer org-agenda-buffer-name
              (let ((agenda-text (replace-regexp-in-string
                                  (regexp-quote root) "<root>"
                                  (buffer-substring-no-properties
                                   (point-min) (point-max))))
                    ;; Count clock entries
                    (clock-count
                     (let ((c 0) (s 0))
                       (while (string-match "Clocked" agenda-text s)
                         (setq s (match-end 0) c (1+ c)))
                       c))
                    ;; Extract time entries
                    (times nil)
                    (_ (let ((s 0))
                         (while (string-match
                                 "\\([0-9]+:[0-9]+\\)\\s-+.*Clocked" agenda-text s)
                           (push (match-string 1 agenda-text) times)
                           (setq s (match-end 0))))))
                (list clock-count
                      (nreverse times)
                      agenda-text))))))
      (when (get-buffer org-agenda-buffer-name)
        (kill-buffer org-agenda-buffer-name))
      (delete-directory root t))))"##,
    );
}

#[test]
fn org_clock_report_custom_columns_deep_state_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-clock)
  (let* ((root (make-temp-file "org-clock-col" t))
         (file (expand-file-name "tasks.org" root)))
    (unwind-protect
        (progn
          (with-temp-file file
            (insert "* Project\n")
            (insert "** Task A\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 09:00]--[2026-05-27 Wed 10:30] =>  1:30\n")
            (insert "CLOCK: [2026-05-27 Wed 14:00]--[2026-05-27 Wed 15:00] =>  1:00\n")
            (insert ":END:\n")
            (insert "** Task B\n")
            (insert ":LOGBOOK:\n")
            (insert "CLOCK: [2026-05-27 Wed 11:00]--[2026-05-27 Wed 12:30] =>  1:30\n")
            (insert ":END:\n"))
          (with-current-buffer (find-file-noselect file)
            (org-mode)
            ;; Get table data
            (let* ((table (org-clock-get-table-data
                           nil '(:maxlevel 2 :scope buffer)))
                 (total (nth 1 table))
                 (rows (mapcar (lambda (row)
                                 (list (nth 0 row)
                                       (substring-no-properties (nth 1 row))
                                       (nth 4 row)))
                               (nth 2 table))))
            ;; Generate clocktable
            (with-temp-buffer
              (org-mode)
              (insert "#+BEGIN: clocktable :maxlevel 2\n#+END:\n")
              (goto-char (point-min))
              (org-dblock-update)
              (let ((clocktable (replace-regexp-in-string
                                 (regexp-quote root) "<root>"
                                 (buffer-substring-no-properties
                                  (point-min) (point-max)))))
                (kill-buffer)
                (list total
                      rows
                      clocktable))))))
      (delete-directory root t))))"##,
    );
}
