use expect_test::expect;

use super::assert_ac_html_csswatcher_parity;

#[test]
fn ac_html_csswatcher_async_launch_preserves_process_name_buffer_command_and_arguments() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                buffer-file-name
                "/workspace/site/index page.html")
               (let ((ac-html-csswatcher-command
                      "csswatcher-bin")
                     (ac-html-csswatcher-command-args
                      '("--debug"
                        "--outputdir"
                        "build dir"))
                     events)
                 (cl-letf
                     (((symbol-function
                        'start-process)
                       (lambda (name buffer program
                                     &rest arguments)
                         (push
                          (list
                           'start
                           name
                           buffer
                           program
                           arguments)
                          events)
                         'fixture-process))
                      ((symbol-function
                        'set-process-sentinel)
                       (lambda (process sentinel)
                         (push
                          (list
                           'sentinel
                           process
                           (functionp sentinel))
                          events)
                         'sentinel-installed)))
                   (list
                    (ac-html-csswatcher-setup-html-stuff-async)
                    (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (sentinel-installed ((start "csswatcher-cd41b171baa8b3323ae5c6211991387e" "*csswatcher-output*" "csswatcher-bin" ("--debug" "--outputdir" "build dir" "/workspace/site/index page.html")) (sentinel fixture-process t)))"#
    ]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_success_sentinel_extracts_paths_messages_and_cleans_output() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                buffer-file-name
                "/workspace/site/index.html")
               (let ((ac-html-csswatcher-command
                      "fixture-csswatcher")
                     output-name
                     messages)
                 (cl-letf
                     (((symbol-function
                        'start-process)
                       (lambda (_name buffer _program
                                      &rest _arguments)
                         (setq output-name buffer)
                         (with-current-buffer
                             (get-buffer-create buffer)
                           (insert
                            "PROJECT: /workspace/project root\n"
                            "ACSOURCE: /workspace/generated data\n"))
                         'fixture-process))
                      ((symbol-function
                        'set-process-sentinel)
                       (lambda (process sentinel)
                         (funcall
                          sentinel process "finished\n")
                         'sentinel-installed))
                      ((symbol-function
                        'process-exit-status)
                       (lambda (process)
                         (unless
                             (eq process
                                 'fixture-process)
                           (error
                            "unexpected process"))
                         0))
                      ((symbol-function
                        'message)
                       (lambda (format-string
                                &rest arguments)
                         (let ((message
                                (apply
                                 #'format
                                 format-string
                                 arguments)))
                           (push message messages)
                           message))))
                   (list
                    (ac-html-csswatcher-setup-html-stuff-async)
                    ac-html-csswatcher-source-dir
                    (get-buffer output-name)
                    (nreverse messages)))))"##;
    let expect = expect![[
        r#"OK (sentinel-installed "/workspace/generated data" nil ("[csswatcher] parsed /workspace/project root"))"#
    ]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_sentinel_requires_both_exact_finished_event_and_zero_status() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                buffer-file-name
                "/workspace/site/index.html"
                ac-html-csswatcher-source-dir
                'initial-source)
               (let (sentinel
                     output-name
                     (status 7))
                 (cl-letf
                     (((symbol-function
                        'start-process)
                       (lambda (_name buffer _program
                                      &rest _arguments)
                         (setq output-name buffer)
                         (get-buffer-create buffer)
                         'fixture-process))
                      ((symbol-function
                        'set-process-sentinel)
                       (lambda (_process callback)
                         (setq sentinel callback)
                         'installed))
                      ((symbol-function
                        'process-exit-status)
                       (lambda (_process)
                         status)))
                   (ac-html-csswatcher-setup-html-stuff-async)
                   (let ((wrong-status
                          (progn
                            (funcall
                             sentinel
                             'fixture-process
                             "finished\n")
                            (list
                             ac-html-csswatcher-source-dir
                             (buffer-live-p
                              (get-buffer output-name))))))
                     (setq status 0)
                     (let ((wrong-event
                            (progn
                              (funcall
                               sentinel
                               'fixture-process
                               "exited abnormally\n")
                              (list
                               ac-html-csswatcher-source-dir
                               (buffer-live-p
                                (get-buffer output-name))))))
                       (prog1
                           (list
                            wrong-status
                            wrong-event)
                         (kill-buffer output-name)))))))"##;
    let expect = expect![[r#"OK ((initial-source t) (initial-source t))"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_success_without_both_markers_clears_source_and_cleans_output() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                buffer-file-name
                "/workspace/site/index.html"
                ac-html-csswatcher-source-dir
                'initial-source)
               (let (sentinel
                     output-name
                     messages)
                 (cl-letf
                     (((symbol-function
                        'start-process)
                       (lambda (_name buffer _program
                                      &rest _arguments)
                         (setq output-name buffer)
                         (with-current-buffer
                             (get-buffer-create buffer)
                           (insert
                            "ACSOURCE: /source-without-project\n"))
                         'fixture-process))
                      ((symbol-function
                        'set-process-sentinel)
                       (lambda (_process callback)
                         (setq sentinel callback)
                         'installed))
                      ((symbol-function
                        'process-exit-status)
                       (lambda (_process) 0))
                      ((symbol-function
                        'message)
                       (lambda (&rest arguments)
                         (push arguments messages))))
                   (ac-html-csswatcher-setup-html-stuff-async)
                   (funcall
                    sentinel
                    'fixture-process
                    "finished\n")
                   (list
                    ac-html-csswatcher-source-dir
                    (get-buffer output-name)
                    messages))))"##;
    let expect = expect![[r#"OK (nil nil nil)"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_success_with_project_only_reports_project_and_clears_source() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                buffer-file-name
                "/workspace/site/index.html"
                ac-html-csswatcher-source-dir
                'initial-source)
               (let (sentinel
                     output-name
                     messages)
                 (cl-letf
                     (((symbol-function
                        'start-process)
                       (lambda (_name buffer _program
                                      &rest _arguments)
                         (setq output-name buffer)
                         (with-current-buffer
                             (get-buffer-create buffer)
                           (insert
                            "PROJECT: /project-only\n"))
                         'fixture-process))
                      ((symbol-function
                        'set-process-sentinel)
                       (lambda (_process callback)
                         (setq sentinel callback)
                         'installed))
                      ((symbol-function
                        'process-exit-status)
                       (lambda (_process) 0))
                      ((symbol-function
                        'message)
                       (lambda (format-string
                                &rest arguments)
                         (push
                          (apply
                           #'format
                           format-string
                           arguments)
                          messages))))
                   (ac-html-csswatcher-setup-html-stuff-async)
                   (funcall
                    sentinel
                    'fixture-process
                    "finished\n")
                   (list
                    ac-html-csswatcher-source-dir
                    (get-buffer output-name)
                    (nreverse messages)))))"##;
    let expect = expect![[r#"OK (nil nil ("[csswatcher] parsed /project-only"))"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}

#[test]
fn ac_html_csswatcher_process_output_buffer_name_avoids_existing_buffer_collision() {
    let elisp_form = r##"(with-temp-buffer
               (setq
                buffer-file-name
                "/workspace/site/index.html")
               (let ((collision
                      (get-buffer-create
                       "*csswatcher-output*"))
                     observed)
                 (unwind-protect
                     (cl-letf
                         (((symbol-function
                            'start-process)
                           (lambda (_name buffer _program
                                          &rest _arguments)
                             (setq observed buffer)
                             'fixture-process))
                          ((symbol-function
                            'set-process-sentinel)
                           (lambda (&rest _arguments)
                             'installed)))
                       (list
                        (ac-html-csswatcher-setup-html-stuff-async)
                        observed
                        (not
                         (equal
                          observed
                          (buffer-name collision)))))
                   (kill-buffer collision))))"##;
    let expect = expect![[r#"OK (installed "*csswatcher-output*<2>" t)"#]];

    assert_ac_html_csswatcher_parity(elisp_form, expect);
}
