use expect_test::expect;

use super::assert_abs_mode_parity;

#[test]
fn abs_mode_check_installation_reports_resolved_programs_and_runs_exact_version_commands() {
    let elisp_form = r##"(let ((abs-compiler-program "absc")
                    events)
               (cl-letf
                   (((symbol-function 'executable-find)
                     (lambda (program)
                       (push (list 'find program) events)
                       (cdr
                        (assoc
                         program
                         '(("absc" . "/tools/absc")
                           ("java" . "/tools/java")
                           ("erl" . "/tools/erl")
                           ("erlc" . nil))))))
                    ((symbol-function 'shell-command)
                     (lambda (command destination)
                       (push
                        (list
                         'shell
                         command
                         destination)
                        events)
                       (insert
                        (format
                         "<output:%s>"
                         command))
                       0)))
                 (prog1
                     (progn
                       (when
                           (get-buffer
                            "*ABS installation status check*")
                         (kill-buffer
                          "*ABS installation status check*"))
                       (abs-check-installation)
                       (with-current-buffer
                           "*ABS installation status check*"
                         (list
                          major-mode
                          (buffer-string)
                          (nreverse events))))
                   (when
                       (get-buffer
                        "*ABS installation status check*")
                     (kill-buffer
                      "*ABS installation status check*")))))"##;
    let expect = expect![[
        r#"OK (help-mode "abs-compiler-program: /tools/absc (found in path)\n\njava: /tools/java\nerlc: (not found)\nerl:  /tools/erl\n\nabsc -V says:\n<output:absc -V>\n/tools/java -version says:\n<output:/tools/java -version>\n/tools/erl -eval '{ok, Version} = file:read_file(filename:join([code:root_dir(), \"releases\", erlang:system_info(otp_release), \"OTP_VERSION\"])), io:fwrite(Version), halt().' -noshell says:\n<output:/tools/erl -eval '{ok, Version} = file:read_file(filename:join([code:root_dir(), \"releases\", erlang:system_info(otp_release), \"OTP_VERSION\"])), io:fwrite(Version), halt().' -noshell>\n" ((find "java") (find "erl") (find "erlc") (find "absc") (find "absc") (shell "absc -V" 4) (shell "/tools/java -version" 4) (shell "/tools/erl -eval '{ok, Version} = file:read_file(filename:join([code:root_dir(), \"releases\", erlang:system_info(otp_release), \"OTP_VERSION\"])), io:fwrite(Version), halt().' -noshell" 4)))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}

#[test]
fn abs_mode_download_compiler_selects_named_release_asset_and_persists_exact_command() {
    let elisp_form = r##"(let* ((root
                      (expand-file-name
                       "abs-mode-download"
                       (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                     (abs-directory root)
                     (release-buffer
                      (generate-new-buffer
                       " *abs-release-json*"))
                     events)
                (unwind-protect
                    (cl-letf
                        (((symbol-function
                           'url-retrieve-synchronously)
                          (lambda (url)
                            (push
                             (list 'retrieve url)
                             events)
                            release-buffer))
                         ((symbol-function 'json-read)
                          (lambda ()
                            '((assets
                               . [((name . "sources.zip")
                                   (browser_download_url
                                    . "https://example/sources"))
                                  ((name . "absfrontend.jar")
                                   (browser_download_url
                                    . "https://example/compiler"))]))))
                         ((symbol-function 'make-directory)
                          (lambda (directory parents)
                            (push
                             (list
                              'mkdir directory parents)
                             events)))
                         ((symbol-function 'url-copy-file)
                          (lambda
                              (url destination overwrite)
                            (push
                             (list
                              'copy
                              url destination overwrite)
                             events)))
                         ((symbol-function
                           'customize-save-variable)
                          (lambda
                              (variable value comment)
                            (push
                             (list
                              'save variable value comment)
                             events)
                            'saved)))
                      (list
                       (abs-download-compiler)
                       (nreverse events)))
                  (kill-buffer release-buffer)))"##;
    let expect = expect![[
        r#"OK (saved ((retrieve "https://api.github.com/repos/abstools/abstools/releases/latest") (mkdir "[ORACLE-SANDBOX]/abs-mode-download/" t) (copy "https://example/compiler" "[ORACLE-SANDBOX]/abs-mode-download/absfrontend.jar" t) (save abs-compiler-program "java -jar [ORACLE-SANDBOX]/abs-mode-download/absfrontend.jar" "Set via `abs-download-compiler'")))"#
    ]];

    assert_abs_mode_parity(elisp_form, expect);
}
