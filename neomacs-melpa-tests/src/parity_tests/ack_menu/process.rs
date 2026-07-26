use super::assert_ack_menu_parity;
use expect_test::expect;

#[test]
fn ack_menu_version_helpers_parse_versions_and_enforce_minimum() {
    let elisp_form = r##"(let (calls)
         (cl-labels
             ((with-version
               (text function)
               (cl-letf
                   (((symbol-function
                      'call-process)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       (insert text)
                       0)))
                 (funcall function))))
           (list
            (with-version
             "ack 3.7.0\n"
             #'ack-version-string)
            (with-version
             "ack 1.94\n"
             #'ack-uses-line-color)
            (with-version
             "ack-grep 1.93\n"
             #'ack-uses-line-color)
            (with-version
             "ack 2.0\n"
             (lambda ()
               (ack-check-version)
               'accepted))
            (condition-case error
                (with-version
                 "ack 1.93\n"
                 #'ack-check-version)
              (error
               (list
                (car error)
                (cadr error))))
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK ("3.7.0" t nil accepted (error "Ack-menu only supports ack version 1.94 or later. Yours is 1.93.") ((nil nil t nil "--version") (nil nil t nil "--version") (nil nil t nil "--version") (nil nil t nil "--version") (nil nil t nil "--version")))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_list_files_returns_reverse_nul_records_and_nil_on_failure() {
    let elisp_form = r##"(let (calls)
         (cl-labels
             ((scenario
               (status payload)
               (cl-letf
                   (((symbol-function
                      'call-process)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       (insert payload)
                       status)))
                 (ack-list-files
                  "/fixture/root/"
                  "--type"
                  "elisp"))))
           (list
            (scenario
             0
             "one.el\0two.el\0nested/three.el\0")
            (scenario
             1
             "ignored.el\0")
            (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("nested/three.el" "two.el" "one.el") nil ((nil nil t nil "-f" "--print0" "--type" "elisp") (nil nil t nil "-f" "--print0" "--type" "elisp")))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_abort_deletes_only_live_process_values() {
    let elisp_form = r##"(let (calls)
         (cl-letf
             (((symbol-function
                'processp)
               (lambda (value)
                 (eq value
                     'fixture-process)))
              ((symbol-function
                'delete-process)
               (lambda (value)
                 (push value calls)
                 'deleted)))
           (list
            (let ((ack-process
                   'fixture-process))
              (ack-abort))
            (let ((ack-process
                   'not-a-process))
              (ack-abort))
            (nreverse calls))))"##;
    let expect = expect!["OK (deleted nil (fixture-process))"];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_run_impl_builds_buffer_process_and_rerun_contract() {
    let elisp_form = r##"(let ((ack-buffer-name
                "*ack-menu-process-fixture*")
               (ack-display-buffer t)
               calls)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'ack-abort)
                   (lambda ()
                     (push 'abort calls)))
                  ((symbol-function
                    'ack-mode)
                   (lambda ()
                     (setq major-mode
                           'ack-mode)
                     (push 'mode calls)))
                  ((symbol-function
                    'font-lock-mode)
                   (lambda (&rest arguments)
                     (push
                      (cons 'font-lock arguments)
                      calls)))
                  ((symbol-function
                    'display-buffer)
                   (lambda (buffer)
                     (push
                      (list
                       'display
                       (buffer-name buffer))
                      calls)
                     buffer))
                  ((symbol-function
                    'start-process)
                   (lambda (&rest arguments)
                     (push
                      (cons
                       'start-process
                       (mapcar
                        (lambda (value)
                          (if (bufferp value)
                              (buffer-name value)
                            value))
                        arguments))
                      calls)
                     'fixture-process))
                  ((symbol-function
                    'set-process-sentinel)
                   (lambda (&rest arguments)
                     (push
                      (cons
                       'sentinel
                       arguments)
                      calls)))
                  ((symbol-function
                    'set-process-query-on-exit-flag)
                   (lambda (&rest arguments)
                     (push
                      (cons
                       'query
                       arguments)
                      calls)))
                  ((symbol-function
                    'set-process-filter)
                   (lambda (&rest arguments)
                     (push
                      (cons
                       'filter
                       arguments)
                      calls))))
               (ack-run-impl
                "/fixture/root"
                "--color"
                "--match=x")
               (with-current-buffer
                   ack-buffer-name
                 (list
                  (buffer-name
                   next-error-last-buffer)
                  ack-buffer--rerun-args
                  (local-variable-p
                   'ack-buffer--rerun-args)
                  buffer-read-only
                  default-directory
                  major-mode
                  ack-process
                  (nreverse calls))))
           (when
               (get-buffer
                ack-buffer-name)
             (kill-buffer
              ack-buffer-name))))"##;
    let expect = expect![[
        r#"OK ("*ack-menu-process-fixture*" ("/fixture/root/" "--color" "--match=x") t t "/fixture/root/" ack-mode fixture-process (abort mode (font-lock) (display "*ack-menu-process-fixture*") (start-process "ack" "*ack-menu-process-fixture*" nil "--color" "--match=x") (sentinel fixture-process ack-sentinel) (query fixture-process nil) (filter fixture-process ack-filter)))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_sentinel_handles_running_empty_singular_diagnostic_and_plural_results() {
    let elisp_form = r##"(let ((buffer
                (generate-new-buffer
                 " *ack-sentinel-fixture*"))
               calls
               status
               finish
               count)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'process-status)
                   (lambda (process)
                     process
                     status))
                  ((symbol-function
                    'process-buffer)
                   (lambda (process)
                     process
                     buffer))
                  ((symbol-function
                    'ack-parse-sgr-sequences-finish)
                   (lambda (function)
                     function
                     finish))
                  ((symbol-function
                    'ack-count-matches)
                   (lambda ()
                     count))
                  ((symbol-function
                    'display-buffer)
                   (lambda (value)
                     (push
                      (list
                       'display
                       (buffer-name value))
                      calls)))
                  ((symbol-function
                    'kill-buffer)
                   (lambda (value)
                     (push
                      (list
                       'kill
                       (buffer-name value))
                      calls)
                     t))
                  ((symbol-function
                    'message)
                   (lambda (&rest arguments)
                     (push
                      (cons
                       'message
                       arguments)
                      calls))))
               (setq status
                     'run
                     finish
                     "ignored"
                     count
                     9)
               (ack-sentinel
                'fixture-process
                "running")
               (setq status
                     'exit
                     finish
                     ""
                     count
                     0)
               (with-current-buffer buffer
                 (erase-buffer))
               (ack-sentinel
                'fixture-process
                "empty")
               (setq finish
                     "one"
                     count
                     1)
               (with-current-buffer buffer
                 (erase-buffer))
               (ack-sentinel
                'fixture-process
                "singular")
               (setq finish
                     ""
                     count
                     0)
               (with-current-buffer buffer
                 (erase-buffer)
                 (insert
                  "diagnostic"))
               (let ((ack-display-buffer
                      'after))
                 (ack-sentinel
                  'fixture-process
                  "diagnostic"))
               (setq finish
                     "tail"
                     count
                     2)
               (with-current-buffer buffer
                 (erase-buffer))
               (let ((ack-display-buffer
                      'after))
                 (ack-sentinel
                  'fixture-process
                  "success"))
               (list
                (with-current-buffer buffer
                  (buffer-string))
                (nreverse calls)))
           (when
               (buffer-live-p buffer)
             (kill-buffer buffer))))"##;
    let expect = expect![[
        r#"OK ("tail" ((kill " *ack-sentinel-fixture*") (message "Ack finished with %d match%s" 0 "es") (message "Ack finished with %d match%s" 1 "") (display " *ack-sentinel-fixture*") (message "Ack finished with %d match%s" 0 "es") (display " *ack-sentinel-fixture*") (message "Ack finished with %d match%s" 2 "es")))"#
    ]];
    assert_ack_menu_parity(elisp_form, expect);
}

#[test]
fn ack_menu_filter_appends_decoded_output_or_aborts_for_dead_buffers() {
    let elisp_form = r##"(let ((live
                (generate-new-buffer
                 " *ack-filter-live*"))
               (dead
                (generate-new-buffer
                 " *ack-filter-dead*"))
               calls
               selected)
         (kill-buffer dead)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'process-buffer)
                   (lambda (process)
                     process
                     selected))
                  ((symbol-function
                    'ack-parse-sgr-sequences)
                   (lambda (output function)
                     (push
                      (list
                       'parse
                       output
                       function)
                      calls)
                     (concat
                      "<"
                      output
                      ">")))
                  ((symbol-function
                    'ack-abort)
                   (lambda ()
                     (push
                      'abort
                      calls))))
               (setq selected live)
               (with-current-buffer live
                 (insert
                  "prefix"))
               (ack-filter
                'fixture-process
                "output")
               (setq selected dead)
               (ack-filter
                'fixture-process
                "ignored")
               (list
                (with-current-buffer live
                  (buffer-string))
                (nreverse calls)))
           (when
               (buffer-live-p live)
             (kill-buffer live))))"##;
    let expect = expect![[r#"OK ("prefix<output>" ((parse "output" ack-apply-faces) abort))"#]];
    assert_ack_menu_parity(elisp_form, expect);
}
