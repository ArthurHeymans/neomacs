use crate::common::assert_oracle_parity;
use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn org_timer_insert_items_pause_continue_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-timer)
  (with-temp-buffer
    (let ((org-timer-format "[%s] ")
          (org-timer-display nil)
          (events nil))
      (add-hook 'org-timer-start-hook
                (lambda () (push 'start events)) nil t)
      (add-hook 'org-timer-pause-hook
                (lambda () (push 'pause events)) nil t)
      (add-hook 'org-timer-continue-hook
                (lambda () (push 'continue events)) nil t)
      (add-hook 'org-timer-stop-hook
                (lambda () (push 'stop events)) nil t)
      (org-mode)
      (cl-letf (((symbol-function 'current-time)
                 (lambda () (seconds-to-time 1000))))
        (org-timer-start "0:01:05"))
      (cl-letf (((symbol-function 'current-time)
                 (lambda () (seconds-to-time 1070))))
        (org-timer-item nil)
        (org-timer-pause-or-continue nil))
      (let ((paused (org-timer-value-string)))
        (cl-letf (((symbol-function 'current-time)
                   (lambda () (seconds-to-time 1120))))
          (org-timer-pause-or-continue nil))
        (cl-letf (((symbol-function 'current-time)
                   (lambda () (seconds-to-time 1130))))
          (goto-char (point-max))
          (org-timer-item nil)
          (org-timer-stop))
        (list paused
              (nreverse events)
              org-timer-start-time
              org-timer-pause-time
              org-timer-countdown-timer
              (buffer-substring-no-properties
               (point-min) (point-max))))))"##,
    );
}

#[test]
fn org_timer_region_shift_negative_default_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-timer)
  (with-temp-buffer
    (insert "Intro 0:00:10\n")
    (insert "- 0:01:00:: item\n")
    (insert "- -0:00:05:: negative\n")
    (insert "Outro 1:02:03\n")
    (let ((before (buffer-substring-no-properties
                   (point-min) (point-max))))
      (org-timer-change-times-in-region (point-min) (point-max) "-0:00:10")
      (let ((after-explicit
             (buffer-substring-no-properties (point-min) (point-max))))
        (org-timer-change-times-in-region (point-min) (point-max) "")
        (list before
              after-explicit
              (buffer-substring-no-properties
               (point-min) (point-max))
              (mapcar #'org-timer-hms-to-secs
                      '("-0:00:15" "0:00:00" "1:01:43"))
              (mapcar #'org-timer-secs-to-hms
                      '(-15 0 3703))))))"##,
    );
}

#[test]
fn org_timer_countdown_effort_title_mode_line_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-timer)
  (with-temp-buffer
    (let ((org-timer-display 'mode-line)
          (global-mode-string nil)
          (frame-title-format nil)
          (org-timer-default-timer "0")
          (org-effort-property "Effort"))
      (org-mode)
      (insert "* TODO Timed task\n:PROPERTIES:\n:Effort: 0:02\n:END:\n")
      (goto-char (point-min))
      (let ((title (org-timer--get-timer-title)))
        (cl-letf (((symbol-function 'current-time)
                   (lambda () (seconds-to-time 2000))))
          (org-timer-set-timer '(4)))
        (let ((after-set
               (list (timerp org-timer-countdown-timer)
                     org-timer-countdown-timer-title
                     (org-timer-value-string)
                     global-mode-string
                     org-timer-mode-line-string)))
          (org-timer-pause-or-continue nil)
          (let ((after-pause
                 (list org-timer-countdown-timer
                       (not (null org-timer-pause-time))
                       org-timer-mode-line-timer)))
            (org-timer-stop)
            (list title
                  after-set
                  after-pause
                  org-timer-start-time
                  org-timer-countdown-timer
                  global-mode-string))))))"##,
    );
}

#[test]
fn org_timer_restart_offset_parse_item_error_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r##"(progn
  (require 'org)
  (require 'org-timer)
  (with-temp-buffer
    (let ((org-timer-format "<%s>")
          (org-timer-display nil)
          (events nil))
      (add-hook 'org-timer-start-hook
                (lambda () (push (list 'start org-timer-start-time)
                                 events))
                nil t)
      (add-hook 'org-timer-stop-hook
                (lambda () (push 'stop events)) nil t)
      (org-mode)
      (insert "* Timer\n")
      (insert "Existing stamp 0:02:03 here.\n")
      (insert "- plain item\n")
      (goto-char (point-min))
      (search-forward "0:02:03")
      (cl-letf (((symbol-function 'current-time)
                 (lambda () (seconds-to-time 5000)))
                ((symbol-function 'read-string)
                 (lambda (&rest _) "")))
        (org-timer-start '(4)))
      (let ((after-start (list org-timer-start-time
                               org-timer-pause-time
                               (org-timer-value-string))))
        (cl-letf (((symbol-function 'current-time)
                   (lambda () (seconds-to-time 5010))))
          (let ((no-insert (org-timer nil t)))
            (goto-char (point-max))
            (insert "\nInserted: ")
            (org-timer nil nil)
            (let ((after-insert
                   (buffer-substring-no-properties
                    (point-min) (point-max)))
                  (plain-item-error
                   (progn
                     (goto-char (point-min))
                     (search-forward "- plain item")
                     (condition-case err
                         (progn (org-timer-item nil) 'no-error)
                       (error (cons (car err) (cdr err)))))))
              (org-timer-stop)
              (list after-start
                    no-insert
                    after-insert
                    plain-item-error
                    (mapcar (lambda (s)
                              (condition-case err
                                  (list s
                                        (org-timer-fix-incomplete s)
                                        (org-timer-hms-to-secs
                                         (org-timer-fix-incomplete s)))
                                (error (list s (cons (car err) (cdr err))))))
                            '("7" "2:03" "1:02:03" "bad"))
                    (mapcar #'org-timer-secs-to-hms
                            '(-3723 -1 0 61 3661))
                    (mapcar (lambda (event)
                              (if (consp event) (car event) event))
                            (nreverse events))
                    org-timer-start-time
                    org-timer-pause-time)))))))"##,
    );
}
