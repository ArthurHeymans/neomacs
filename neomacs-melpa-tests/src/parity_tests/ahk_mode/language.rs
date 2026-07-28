use expect_test::expect;

use super::assert_ahk_mode_parity;

#[test]
fn ahk_mode_fontifies_and_refontifies_a_real_automation_script_with_real_syntax_state() {
    let elisp_form = r####"(let* ((root
                                  (file-name-as-directory
                                   (expand-file-name
                                    "ahk-language-workflow"
                                    (getenv
                                     "NEOMACS_TEST_SANDBOX_ROOT"))))
                                 (script
                                  (expand-file-name
                                   "automation/report.ahk"
                                   root))
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
                                     "; release keyboard shortcut\n"
                                     "BuildReport(name) {\n"
                                     "  output := A_ScriptDir . \"\\\\\" . name\n"
                                     "  if FileExist(output) AND GetKeyState(\"NumpadEnter\")\n"
                                     "    MsgBox, 64, Report, %output%\n"
                                     "  return output\n"
                                     "}\n"
                                     "\n"
                                     "/* disabled staging path\n"
                                     "Run, staging.exe\n"
                                     "*/\n"
                                     "\n"
                                     "^!r::Run, report.exe\n"
                                     "WorkerReady:\n"
                                     ":*:btw::by the way\n"
                                     "message =\n"
                                     "(LTrim0 Join`n\n"
                                     "First report line\n"
                                     "  Second report line\n"
                                     ")\n"))
                                   (setq
                                    buffer
                                    (find-file-noselect
                                     script))
                                   (with-current-buffer
                                       buffer
                                     (font-lock-ensure)
                                     (cl-labels
                                         ((describe
                                           (text
                                            &optional
                                            occurrence)
                                              (goto-char
                                               (point-min))
                                              (dotimes
                                                  (_
                                                   (or
                                                    occurrence
                                                    1))
                                                (search-forward
                                                 text))
                                              (let ((position
                                                     (match-beginning
                                                      0)))
                                                (list
                                                 text
                                                 (get-text-property
                                                  position
                                                  'face)
                                                 (nth
                                                  3
                                                  (syntax-ppss
                                                   position))
                                                 (nth
                                                  4
                                                  (syntax-ppss
                                                   position))
                                                 (line-number-at-pos
                                                  position)
                                                 (save-excursion
                                                   (goto-char
                                                    position)
                                                   (current-column))))))
                                       (let ((before
                                              (mapcar
                                               #'describe
                                               '("#SingleInstance"
                                                 "release keyboard"
                                                 "BuildReport"
                                                 ":="
                                                 "A_ScriptDir"
                                                 "FileExist"
                                                 "AND"
                                                 "GetKeyState"
                                                 "NumpadEnter"
                                                 "MsgBox"
                                                 "%output%"
                                                 "return"
                                                 "disabled staging"
                                                 "staging.exe"
                                                 "^!r"
                                                 "WorkerReady"
                                                 ":*:btw::"
                                                 "First report line"
                                                 "Second report line"))))
                                         (goto-char
                                          (point-min))
                                         (search-forward
                                          "MsgBox, 64, Report, %output%")
                                         (replace-match
                                          "result := RegExReplace(A_ScriptName, \"\\\\.ahk$\")"
                                          t
                                          t)
                                         (font-lock-flush
                                          (line-beginning-position)
                                          (line-beginning-position
                                           2))
                                         (font-lock-ensure)
                                         (save-buffer)
                                         (setq
                                          result
                                          (list
                                           major-mode
                                           (file-relative-name
                                            buffer-file-name
                                            root)
                                           before
                                           (list
                                            (describe
                                             "result")
                                            (describe
                                             ":="
                                             2)
                                            (describe
                                             "RegExReplace")
                                            (describe
                                             "A_ScriptName")
                                            (describe
                                             "\\.ahk$"))
                                           (substring-no-properties
                                            (buffer-string))
                                           (neomacs-ahk-test-file-string
                                            script)
                                           (buffer-modified-p)))))))
                               (neomacs-ahk-test-cleanup
                                root))
                             result)"####;
    let expect = expect![[
        r##"OK (ahk-mode "automation/report.ahk" (("#SingleInstance" font-lock-preprocessor-face nil nil 1 0) ("release keyboard" font-lock-comment-face nil t 2 2) ("BuildReport" font-lock-function-name-face nil nil 3 0) (":=" font-lock-builtin-face nil nil 4 9) ("A_ScriptDir" font-lock-variable-name-face nil nil 4 12) ("FileExist" font-lock-function-name-face nil nil 5 5) ("AND" font-lock-keyword-face nil nil 5 23) ("GetKeyState" font-lock-keyword-face nil nil 5 27) ("NumpadEnter" font-lock-string-face 34 nil 5 40) ("MsgBox" font-lock-keyword-face nil nil 6 4) ("%output%" font-lock-variable-name-face nil nil 6 24) ("return" font-lock-warning-face nil nil 7 2) ("disabled staging" font-lock-comment-face nil t 10 3) ("staging.exe" font-lock-comment-face nil t 11 5) ("^!r" font-lock-constant-face nil nil 14 0) ("WorkerReady" font-lock-doc-face nil nil 15 0) (":*:btw::" nil nil nil 16 0) ("First report line" font-lock-string-face nil nil 19 0) ("Second report line" font-lock-string-face nil nil 20 2)) (("result" nil nil nil 6 4) (":=" font-lock-builtin-face nil nil 6 11) ("RegExReplace" font-lock-function-name-face nil nil 6 14) ("A_ScriptName" font-lock-variable-name-face nil nil 6 27) ("\\.ahk$" font-lock-string-face 34 nil 6 43)) "#SingleInstance Force\n; release keyboard shortcut\nBuildReport(name) {\n  output := A_ScriptDir . \"\\\\\" . name\n  if FileExist(output) AND GetKeyState(\"NumpadEnter\")\n    result := RegExReplace(A_ScriptName, \"\\\\.ahk$\")\n  return output\n}\n\n/* disabled staging path\nRun, staging.exe\n*/\n\n^!r::Run, report.exe\nWorkerReady:\n:*:btw::by the way\nmessage =\n(LTrim0 Join`n\nFirst report line\n  Second report line\n)\n" "#SingleInstance Force\n; release keyboard shortcut\nBuildReport(name) {\n  output := A_ScriptDir . \"\\\\\" . name\n  if FileExist(output) AND GetKeyState(\"NumpadEnter\")\n    result := RegExReplace(A_ScriptName, \"\\\\.ahk$\")\n  return output\n}\n\n/* disabled staging path\nRun, staging.exe\n*/\n\n^!r::Run, report.exe\nWorkerReady:\n:*:btw::by the way\nmessage =\n(LTrim0 Join`n\nFirst report line\n  Second report line\n)\n" nil)"##
    ]];

    assert_ahk_mode_parity(elisp_form, expect);
}
