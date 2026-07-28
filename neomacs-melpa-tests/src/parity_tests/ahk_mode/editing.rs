use expect_test::expect;

use super::assert_ahk_mode_parity;

#[test]
fn ahk_mode_opens_indents_comments_edits_and_saves_a_real_deployment_script() {
    let elisp_form = r####"(let* ((root
                                  (file-name-as-directory
                                   (expand-file-name
                                    "ahk-editing-workflow"
                                    (getenv
                                     "NEOMACS_TEST_SANDBOX_ROOT"))))
                                 (script
                                  (expand-file-name
                                   "automation/deploy.ahk"
                                   root))
                                 (ahk-indentation
                                  2)
                                 (transient-mark-mode
                                  t)
                                 buffer
                                 result)
                             (unwind-protect
                                 (progn
                                   (neomacs-ahk-test-cleanup
                                    root)
                                   (neomacs-ahk-test-write-file
                                    script
                                    (concat
                                     "Deploy(environment) {\n"
                                     "output := A_ScriptDir . \"\\\\build\\\\\" . environment\n"
                                     "if FileExist(output) {\n"
                                     "MsgBox, 64, Deploy, % \"Shipping \" . output\n"
                                     "} else {\n"
                                     "FileAppend, missing, %output%\\\\missing.log\n"
                                     "}\n"
                                     "return output\n"
                                     "}\n"
                                     "\n"
                                     "^!d::\n"
                                     "Deploy(\"production\")\n"
                                     "return\n"))
                                   (setq
                                    buffer
                                    (find-file-noselect
                                     script))
                                   (with-current-buffer
                                       buffer
                                     (indent-region
                                      (point-min)
                                      (point-max))
                                     (goto-char
                                      (point-min))
                                     (search-forward
                                      "Shipping")
                                     (replace-match
                                      "Publishing"
                                      t
                                      t)
                                     (goto-char
                                      (point-min))
                                     (search-forward
                                      "FileAppend")
                                     (let ((begin
                                            (line-beginning-position))
                                           (end
                                            (line-beginning-position
                                             2)))
                                       (goto-char
                                        begin)
                                       (push-mark
                                        end
                                        t
                                        t)
                                       (ahk-comment-dwim
                                        nil)
                                       (let ((commented
                                              (substring-no-properties
                                               (buffer-string))))
                                         (goto-char
                                          begin)
                                         (push-mark
                                          (line-beginning-position
                                           2)
                                          t
                                          t)
                                         (ahk-comment-dwim
                                          nil)
                                         (goto-char
                                          (point-min))
                                         (search-forward
                                          "return output")
                                         (beginning-of-line)
                                         (insert
                                          "if ErrorLevel\n"
                                          "MsgBox, 16, Deploy, deployment failed\n")
                                         (indent-region
                                          (line-beginning-position
                                           -1)
                                          (line-beginning-position
                                           1))
                                         (save-buffer)
                                         (setq
                                          result
                                          (list
                                           major-mode
                                           mode-name
                                           (file-relative-name
                                            buffer-file-name
                                            root)
                                           commented
                                           (substring-no-properties
                                            (buffer-string))
                                           (neomacs-ahk-test-file-string
                                            script)
                                           (buffer-modified-p)
                                           (save-excursion
                                             (goto-char
                                              (point-min))
                                             (let (levels)
                                               (while
                                                   (not
                                                    (eobp))
                                                 (push
                                                  (current-indentation)
                                                  levels)
                                                 (forward-line
                                                  1))
                                               (nreverse
                                                levels)))))))))
                               (neomacs-ahk-test-cleanup
                                root))
                             result)"####;
    let expect = expect![[
        r#"OK (ahk-mode "AHK" "automation/deploy.ahk" "  Deploy(environment) {\n    output := A_ScriptDir . \"\\\\build\\\\\" . environment\n    if FileExist(output) {\n      MsgBox, 64, Deploy, % \"Publishing \" . output\n    } else {\n      ; FileAppend, missing, %output%\\\\missing.log\n    }\n    return output\n  }\n\n^!d::\nDeploy(\"production\")\nreturn\n" "  Deploy(environment) {\n    output := A_ScriptDir . \"\\\\build\\\\\" . environment\n    if FileExist(output) {\n      MsgBox, 64, Deploy, % \"Publishing \" . output\n    } else {\n      FileAppend, missing, %output%\\\\missing.log\n    }\n    if ErrorLevel\n      MsgBox, 16, Deploy, deployment failed\n    return output\n  }\n\n^!d::\nDeploy(\"production\")\nreturn\n" "  Deploy(environment) {\n    output := A_ScriptDir . \"\\\\build\\\\\" . environment\n    if FileExist(output) {\n      MsgBox, 64, Deploy, % \"Publishing \" . output\n    } else {\n      FileAppend, missing, %output%\\\\missing.log\n    }\n    if ErrorLevel\n      MsgBox, 16, Deploy, deployment failed\n    return output\n  }\n\n^!d::\nDeploy(\"production\")\nreturn\n" nil (2 4 4 6 4 6 4 4 6 4 2 0 0 0 0))"#
    ]];

    assert_ahk_mode_parity(elisp_form, expect);
}
