use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_timestamp_change_toggle_repeater_delay_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Task\n")
    (insert "SCHEDULED: <2026-05-27 Wed 09:30-10:45 +1w -2d>\n")
    (goto-char (point-min))
    (search-forward "09:30")
    (org-timestamp-change 45 'minute nil t)
    (let ((after-minute
           (buffer-substring-no-properties (point-min) (point-max))))
      (goto-char (point-min))
      (search-forward "2026")
      (org-timestamp-change 1 'month nil t)
      (let ((after-month
             (buffer-substring-no-properties (point-min) (point-max))))
        (goto-char (point-min))
        (search-forward "<")
        (org-toggle-timestamp-type)
        (list after-minute
              after-month
              (buffer-substring-no-properties
               (point-min) (point-max)))))))"##,
    );
}

#[test]
fn org_read_date_relative_default_time_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (let* ((base (encode-time 0 15 8 27 5 2026))
         (org-read-date-popup-calendar nil)
         (org-overriding-default-time base))
    (list
     (org-read-date nil nil "++2w" nil base)
     (org-read-date t nil "++3d 14:45" nil base)
     (format-time-string
      "%Y-%m-%d %H:%M"
      (org-read-date t t "+1m" nil base))
     (mapcar
      (lambda (s)
        (let ((ts (org-timestamp-from-string s)))
          (list s
                (format-time-string
                 "%Y-%m-%d %H:%M"
                 (org-timestamp-to-time ts))
                (org-timestamp-has-time-p ts))))
      '("<2026-05-27 Wed>"
        "[2026-05-27 Wed 09:30]"
        "<2026-05-27 Wed 09:30-10:45>"))))"##,
    );
}

#[test]
fn org_planning_repeater_warning_element_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (org-mode)
    (insert "* TODO Habit\n")
    (insert "SCHEDULED: <2026-05-27 Wed .+2d/4d> ")
    (insert "DEADLINE: <2026-06-01 Mon +1w -3d>\n")
    (insert ":PROPERTIES:\n:STYLE: habit\n:END:\n")
    (let ((tree (org-element-parse-buffer)))
      (org-element-map tree 'planning
        (lambda (planning)
          (let ((scheduled (org-element-property :scheduled planning))
                (deadline (org-element-property :deadline planning)))
            (list
             (mapcar
              (lambda (ts)
                (list (org-element-property :raw-value ts)
                      (org-element-property :repeater-type ts)
                      (org-element-property :repeater-value ts)
                      (org-element-property :repeater-unit ts)
                      (org-element-property :warning-type ts)
                      (org-element-property :warning-value ts)
                      (org-element-property :warning-unit ts)))
              (list scheduled deadline))
             (org-deadline-close-p
              (org-element-property :raw-value deadline)
              7)
             (format-time-string
              "%Y-%m-%d"
              (org-timestamp-to-time scheduled))
             (format-time-string
              "%Y-%m-%d"
              (org-timestamp-to-time deadline))))))))"##,
    );
}

#[test]
fn org_schedule_deadline_timestamp_range_shift_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (with-temp-buffer
    (let ((org-log-reschedule nil)
          (org-log-redeadline nil))
      (org-mode)
      (insert "* TODO Task\n")
      (insert "Body with <2026-05-27 Wed 09:30-10:45> and ")
      (insert "[2026-05-28 Thu].\n")
      (goto-char (point-min))
      (org-schedule nil "2026-06-03 08:15")
      (org-deadline nil "2026-06-10 +1w -2d")
      (let ((after-planning
             (buffer-substring-no-properties (point-min) (point-max))))
        (search-forward "09:30")
        (let* ((range-ts (org-timestamp-at-point))
               (split-start
                (mapcar (lambda (ts)
                          (and ts (org-element-property :raw-value ts)))
                        (org-timestamp-split-range range-ts)))
               (translated-start
                (org-timestamp-translate range-ts 'start))
               (translated-end
                (org-timestamp-translate range-ts 'end)))
          (org-timestamp-up-day 2)
          (search-forward "[2026")
          (org-timestamp-down-day 1)
          (goto-char (point-max))
          (insert "\n")
          (org-timestamp nil nil)
          (insert "\n")
          (org-timestamp-inactive nil)
          (let ((parsed
                 (mapcar
                  (lambda (s)
                    (let ((ts (org-timestamp-from-string s)))
                      (list s
                            (org-element-property :raw-value ts)
                            (org-timestamp-has-time-p ts)
                            (format-time-string
                             "%Y-%m-%d %H:%M"
                             (org-timestamp-to-time ts))
                            (format-time-string
                             "%Y-%m-%d %H:%M"
                             (org-timestamp-to-time ts 'end)))))
                  '("<2026-05-27 Wed 09:30-10:45>"
                    "<2026-06-03 Wed 08:15>"
                    "[2026-05-28 Thu]"))))
            (list after-planning
                  split-start
                  translated-start
                  translated-end
                  parsed
                  (replace-regexp-in-string
                   "\\[[0-9][^]\n]+\\]\\|<[0-9][^>\n]+>"
                   "[stamp]"
                   (buffer-substring-no-properties
                    (point-min) (point-max))))))))"##,
    );
}
