use expect_test::expect;

use super::assert_apheleia_parity;

#[test]
fn apheleia_log_buffer_names_toggle_hidden_prefix_for_real_formatter_symbols() {
    let elisp_form = r##"(mapcar
         (lambda (hidden)
           (let ((apheleia-hide-log-buffers
                  hidden))
             (mapcar
              #'apheleia-log--buffer-name
              '(black
                prettier
                apheleia-test-formatter))))
         '(nil t))"##;
    let expect = expect![[
        r#"OK (("*apheleia-black-log*" "*apheleia-prettier-log*" "*apheleia-apheleia-test-formatter-log*") (" *apheleia-black-log*" " *apheleia-prettier-log*" " *apheleia-apheleia-test-formatter-log*"))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_debug_log_is_lazy_when_disabled_and_records_exact_success_and_format_errors() {
    let elisp_form = r##"(let ((buffer-name
                "*apheleia-test-debug*")
               (apheleia-test-hook-events 0))
         (unwind-protect
             (progn
               (let ((apheleia-log-debug-info
                      nil)
                     (apheleia-debug-info-buffer
                      buffer-name))
                 (apheleia--log
                  'disabled
                  "value=%s"
                  (list
                   'lambda
                   nil
                   '(progn
                      (setq apheleia-test-hook-events
                            (1+ apheleia-test-hook-events))
                      "expensive"))))
               (let ((apheleia-log-debug-info
                      t)
                     (apheleia-debug-info-buffer
                      buffer-name))
                 (cl-letf
                     (((symbol-function
                        'format-time-string)
                       (lambda (&rest _)
                         "2000-01-02 03:04:05.006")))
                   (apheleia--log
                    'process
                    "formatter=%s count=%d"
                    (list
                     'lambda
                     nil
                     '(progn
                        (setq apheleia-test-hook-events
                              (1+ apheleia-test-hook-events))
                        "demo"))
                    2)
                   (apheleia--log
                    'broken
                    "number=%d"
                    "not-a-number")))
               (with-current-buffer buffer-name
                 (list
                  apheleia-test-hook-events
                  major-mode
                  buffer-read-only
                  (buffer-string))))
           (when
               (get-buffer buffer-name)
             (kill-buffer buffer-name))))"##;
    let expect = expect![[
        r#"OK (1 special-mode t "2000-01-02 03:04:05.006 <process>: formatter=demo count=2\n2000-01-02 03:04:05.006 <broken>: Got error formatting log line \"number=%d\": Format specifier doesn’t match argument type\n")"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_formatter_result_logs_success_and_failure_transcripts_and_marks_first_error() {
    let elisp_form = r##"(let ((success-buffer
                "*apheleia-success-log*")
               (failure-buffer
                "*apheleia-failure-log*")
               (apheleia--last-error-marker
                nil))
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'current-time-string)
                   (lambda ()
                     "Sun Jan  2 03:04:05 2000")))
               (let ((success
                      (apheleia-formatter--context))
                     (failure
                      (apheleia-formatter--context)))
                 (setf
                  (apheleia-formatter--arg1 success)
                  "/usr/bin/demo formatter"
                  (apheleia-formatter--argv success)
                  '("--check" "two words")
                  (apheleia-formatter--exit-status success)
                  0
                  (apheleia-formatter--arg1 failure)
                  "broken"
                  (apheleia-formatter--argv failure)
                  '("--file" "source name.el")
                  (apheleia-formatter--exit-status failure)
                  7)
                 (apheleia-log--formatter-result
                  success
                  success-buffer
                  t
                  "/workspace/project/"
                  "")
                 (apheleia-log--formatter-result
                  failure
                  failure-buffer
                  nil
                  "/workspace/project/"
                  "syntax error\nsecond diagnostic")
                 (let ((first-error-position
                        (marker-position
                         apheleia--last-error-marker)))
                   (setf
                    (apheleia-formatter--exit-status failure)
                    9)
                   (apheleia-log--formatter-result
                    failure
                    failure-buffer
                    nil
                    "/workspace/project/"
                    "later error")
                   (list
                    (with-current-buffer success-buffer
                      (list
                       major-mode
                       buffer-read-only
                       (buffer-string)))
                    (with-current-buffer failure-buffer
                      (list
                       major-mode
                       buffer-read-only
                       (buffer-string)))
                    first-error-position
                    (marker-position
                     apheleia--last-error-marker)
                    (buffer-name
                     (marker-buffer
                      apheleia--last-error-marker))))))
           (setq apheleia--last-error-marker
                 nil)
           (dolist (buffer
                    (list success-buffer
                          failure-buffer))
             (when
                 (get-buffer buffer)
               (kill-buffer buffer)))))"##;
    let expect = expect![[
        r#"OK ((special-mode t "Sun Jan  2 03:04:05 2000 :: /workspace/project/\n$ /usr/bin/demo\\ formatter --check two\\ words\n\n(no output on stderr)\n\nCommand succeeded with exit code 0.\n") (special-mode t "Sun Jan  2 03:04:05 2000 :: /workspace/project/\n$ broken --file source\\ name.el\n\nsyntax error\nsecond diagnostic\n\nCommand failed with exit code 7.\n\n\f\nSun Jan  2 03:04:05 2000 :: /workspace/project/\n$ broken --file source\\ name.el\n\nlater error\n\nCommand failed with exit code 9.\n") 1 150 "*apheleia-failure-log*")"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}

#[test]
fn apheleia_goto_error_rejects_empty_state_then_visits_the_exact_marked_diagnostic() {
    let elisp_form = r##"(save-window-excursion
         (let ((buffer
                (generate-new-buffer
                 "*apheleia-goto-error*"))
               (apheleia--last-error-marker
                nil))
           (unwind-protect
               (list
                (condition-case error
                    (apheleia-goto-error)
                  (error
                   (list
                    (car error)
                    (cadr error))))
                (with-current-buffer buffer
                  (insert
                   "header\n"
                   "formatter failed here\n"
                   "tail\n")
                  (goto-char
                   (point-min))
                  (search-forward
                   "formatter")
                  (setq apheleia--last-error-marker
                        (copy-marker
                         (match-beginning 0)))
                  (save-current-buffer
                    (apheleia-goto-error)
                    (list
                     (buffer-name)
                     (point)
                     (line-number-at-pos)
                     (current-column)
                     (buffer-substring-no-properties
                      (line-beginning-position)
                      (line-end-position))))))
             (setq apheleia--last-error-marker
                   nil)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ((user-error "No error has happened yet") ("*apheleia-goto-error*" 8 2 0 "formatter failed here"))"#
    ]];

    assert_apheleia_parity(elisp_form, expect);
}
