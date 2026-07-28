use expect_test::expect;

use super::assert_ahk_mode_parity;

#[test]
fn ahk_mode_looks_up_runs_and_block_comments_commands_from_a_real_script() {
    let elisp_form = r####"(let* ((root
                                  (file-name-as-directory
                                   (expand-file-name
                                    "ahk-command-workflow"
                                    (getenv
                                     "NEOMACS_TEST_SANDBOX_ROOT"))))
                                 (script
                                  (expand-file-name
                                   "automation/release.ahk"
                                   root))
                                 (manual-directory
                                  (expand-file-name
                                   "manual"
                                   root))
                                 (manual
                                  (expand-file-name
                                   "AutoHotkey.chm"
                                   manual-directory))
                                 (ahk-path
                                  manual-directory)
                                 (transient-mark-mode
                                  t)
                                 browser-calls
                                 shell-calls
                                 buffer
                                 result)
                             (unwind-protect
                                 (progn
                                   (neomacs-ahk-test-cleanup
                                    root)
                                   (neomacs-ahk-test-write-file
                                    script
                                    (concat
                                     "#SingleInstance Force\n"
                                     "MsgBox, 64, Release, Ready to deploy\n"
                                     "FileAppend, deployment started, release.log\n"
                                     "RunWait, deploy.exe\n"))
                                   (neomacs-ahk-test-write-file
                                    manual
                                    "deterministic help boundary")
                                   (setq
                                    buffer
                                    (find-file-noselect
                                     script))
                                   (with-current-buffer
                                       buffer
                                     (cl-letf
                                         (((symbol-function
                                            'browse-url)
                                           (lambda
                                             (url
                                              &rest
                                              arguments)
                                             (push
                                              (cons
                                               url
                                               arguments)
                                              browser-calls)
                                             'opened-browser))
                                          ((symbol-function
                                            'w32-shell-execute)
                                           (lambda
                                             (&rest
                                              arguments)
                                             (push
                                              (if
                                                  (equal
                                                   (car
                                                    arguments)
                                                   "open")
                                                  (let* ((windows-path
                                                          (cadr
                                                           arguments))
                                                         (unix-path
                                                          (subst-char-in-string
                                                           ?\\
                                                           ?/
                                                           windows-path))
                                                         (clean-path
                                                          (replace-regexp-in-string
                                                           "/+"
                                                           "/"
                                                           unix-path)))
                                                    (list
                                                     'run
                                                     (car
                                                      arguments)
                                                     (file-relative-name
                                                      clean-path
                                                      root)))
                                                (cons
                                                 'help
                                                 arguments))
                                              shell-calls)
                                             'opened-shell)))
                                       (goto-char
                                        (point-min))
                                       (search-forward
                                        "MsgBox")
                                       (let ((web-result
                                              (ahk-lookup-web)))
                                         (goto-char
                                          (point-min))
                                         (search-forward
                                          "FileAppend")
                                         (let ((help-result
                                                (ahk-lookup-chm)))
                                           (let ((run-result
                                                  (ahk-run-script))
                                                 (version-result
                                                  (ahk-version)))
                                             (goto-char
                                              (point-min))
                                             (forward-line
                                              1)
                                             (let ((begin
                                                    (point))
                                                   (end
                                                    (progn
                                                      (forward-line
                                                       2)
                                                      (point))))
                                               (goto-char
                                                begin)
                                               (push-mark
                                                end
                                                t
                                                t)
                                               (ahk-comment-block-dwim
                                                nil)
                                               (let ((commented
                                                      (substring-no-properties
                                                       (buffer-string))))
                                                 (goto-char
                                                  (point-min))
                                                 (forward-line
                                                  1)
                                                 (let ((comment-begin
                                                        (point))
                                                       (comment-end
                                                        (progn
                                                          (forward-line
                                                           4)
                                                          (point))))
                                                   (goto-char
                                                    comment-begin)
                                                   (push-mark
                                                    comment-end
                                                    t
                                                    t)
                                                   (ahk-comment-block-dwim
                                                    nil)
                                                   (save-buffer)
                                                   (setq
                                                    result
                                                    (list
                                                     major-mode
                                                     web-result
                                                     help-result
                                                     run-result
                                                     version-result
                                                     commented
                                                     (substring-no-properties
                                                      (buffer-string))
                                                     (neomacs-ahk-test-file-string
                                                      script)
                                                     (nreverse
                                                      browser-calls)
                                                     (nreverse
                                                      shell-calls)
                                                     (buffer-modified-p))))))))))))
                               (neomacs-ahk-test-cleanup
                                root)))"####;
    let expect = expect![[
        r##"OK (ahk-mode opened-browser opened-shell opened-shell "ahk-mode version 1.5.6" "#SingleInstance Force\n/*\n * MsgBox, 64, Release, Ready to deploy\n * FileAppend, deployment started, release.log\n */\nRunWait, deploy.exe\n" "#SingleInstance Force\nMsgBox, 64, Release, Ready to deploy\nFileAppend, deployment started, release.log\nRunWait, deploy.exe\n" "#SingleInstance Force\nMsgBox, 64, Release, Ready to deploy\nFileAppend, deployment started, release.log\nRunWait, deploy.exe\n" (("http://ahkscript.org/docs/commands/MsgBox.htm")) ((help 1 "hh.exe" "ms-its:[ORACLE-SANDBOX]/ahk-command-workflow/manual/AutoHotkey.chm::/docs/commands/FileAppend.htm") (run "open" "automation/release.ahk")) nil)"##
    ]];

    assert_ahk_mode_parity(elisp_form, expect);
}
