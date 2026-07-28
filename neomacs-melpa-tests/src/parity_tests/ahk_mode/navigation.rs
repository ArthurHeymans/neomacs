use expect_test::expect;

use super::assert_ahk_mode_parity;

#[test]
fn ahk_mode_completes_real_code_then_navigates_functions_labels_hotkeys_and_hotstrings() {
    let elisp_form = r####"(progn
                             (require
                              'imenu)
                             (let* ((root
                                  (file-name-as-directory
                                   (expand-file-name
                                    "ahk-navigation-workflow"
                                    (getenv
                                     "NEOMACS_TEST_SANDBOX_ROOT"))))
                                 (script
                                  (expand-file-name
                                   "automation/navigation.ahk"
                                   root))
                                 (choice-target
                                  nil)
                                 (last-nonmenu-event
                                  ?i)
                                 (menus
                                  nil)
                                 buffer
                                 result)
                             (unwind-protect
                                 (progn
                                   (neomacs-ahk-test-cleanup
                                    root)
                                   (neomacs-ahk-test-write-file
                                    script
                                    (concat
                                     ";imenu Deployment entry points\n"
                                     "BuildReport(name)\n"
                                     "{\n"
                                     "  sanitized := RegExR\n"
                                     "  script := A_ScriptF\n"
                                     "  MsgB, % sanitized . script\n"
                                     "  return name\n"
                                     "}\n"
                                     "\n"
                                     "WorkerReady:\n"
                                     "MsgBox, worker ready\n"
                                     "return\n"
                                     "\n"
                                     "^!r::Run, report.exe\n"
                                     ":*:btw::by the way\n"))
                                   (setq
                                    buffer
                                    (find-file-noselect
                                     script))
                                   (with-current-buffer
                                       buffer
                                     (dolist
                                         (prefix
                                          '("RegExR"
                                            "A_ScriptF"
                                            "MsgB"))
                                       (goto-char
                                        (point-min))
                                       (search-forward
                                        prefix)
                                       (completion-at-point))
                                     (set-buffer
                                      buffer)
                                     (save-buffer)
                                     (cl-labels
                                         ((navigate
                                           (target)
                                           (setq
                                            choice-target
                                            target)
                                           (call-interactively
                                            #'imenu)
                                           (list
                                            target
                                            (file-relative-name
                                             buffer-file-name
                                             root)
                                            (line-number-at-pos)
                                            (current-column)
                                            (buffer-substring-no-properties
                                             (line-beginning-position)
                                             (line-end-position)))))
                                       (cl-letf
                                           (((symbol-function
                                              'completing-read)
                                             (lambda
                                               (prompt
                                                collection
                                                &rest
                                                _arguments)
                                               (let* ((candidates
                                                       (all-completions
                                                        ""
                                                        collection))
                                                      (choice
                                                       (or
                                                        (cl-find
                                                         choice-target
                                                         candidates
                                                         :test
                                                         #'string=)
                                                        (cdr
                                                         (assoc
                                                          choice-target
                                                          '(("BuildReport"
                                                             .
                                                             "Functions")
                                                            ("WorkerReady"
                                                             .
                                                             "Labels")
                                                            ("^!r"
                                                             .
                                                             "Keybindings")
                                                            (":*:btw::"
                                                             .
                                                             "Hotstrings")))))))
                                                 (push
                                                  (list
                                                   prompt
                                                   candidates
                                                   choice-target
                                                   choice)
                                                  menus)
                                                 choice))))
                                         (let ((locations
                                                (mapcar
                                                 #'navigate
                                                 '("BuildReport"
                                                   "WorkerReady"
                                                   "^!r"
                                                   ":*:btw::"))))
                                           (setq
                                            result
                                            (list
                                             major-mode
                                             (substring-no-properties
                                              (buffer-string))
                                             (neomacs-ahk-test-file-string
                                              script)
                                             locations
                                             (nreverse
                                              menus)
                                             (buffer-modified-p))))))))
                               (neomacs-ahk-test-cleanup
                                root))
                             result))"####;
    let expect = expect![[
        r#"OK (ahk-mode ";imenu Deployment entry points\nBuildReport(name)\n{\n  sanitized := RegExReplace\n  script := A_ScriptFullPath\n  MsgBox, % sanitized . script\n  return name\n}\n\nWorkerReady:\nMsgBox, worker ready\nreturn\n\n^!r::Run, report.exe\n:*:btw::by the way\n" ";imenu Deployment entry points\nBuildReport(name)\n{\n  sanitized := RegExReplace\n  script := A_ScriptFullPath\n  MsgBox, % sanitized . script\n  return name\n}\n\nWorkerReady:\nMsgBox, worker ready\nreturn\n\n^!r::Run, report.exe\n:*:btw::by the way\n" (("BuildReport" "automation/navigation.ahk" 2 0 "BuildReport(name)") ("WorkerReady" "automation/navigation.ahk" 10 0 "WorkerReady:") ("^!r" "automation/navigation.ahk" 14 0 "^!r::Run, report.exe") (":*:btw::" "automation/navigation.ahk" 15 0 ":*:btw::by the way")) (("Index item: " ("*Rescan*" "Comments" "Hotstrings" "Keybindings" "Labels" "Functions") "BuildReport" "Functions") ("Index item: " ("BuildReport") "BuildReport" "BuildReport") ("Index item (default BuildReport): " ("*Rescan*" "Comments" "Hotstrings" "Keybindings" "Labels" "Functions") "WorkerReady" "Labels") ("Index item (default BuildReport): " ("WorkerReady") "WorkerReady" "WorkerReady") ("Index item (default WorkerReady): " ("*Rescan*" "Comments" "Hotstrings" "Keybindings" "Labels" "Functions") "^!r" "Keybindings") ("Index item (default WorkerReady): " ("^!r") "^!r" "^!r") ("Index item: " ("*Rescan*" "Comments" "Hotstrings" "Keybindings" "Labels" "Functions") ":*:btw::" "Hotstrings") ("Index item: " (":*:btw::") ":*:btw::" ":*:btw::")) nil)"#
    ]];

    assert_ahk_mode_parity(elisp_form, expect);
}
