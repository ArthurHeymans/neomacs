use expect_test::expect;

use super::assert_ahk_mode_parity;

#[test]
fn ahk_mode_fontifies_line_block_comments_and_quoted_strings_in_real_script() {
    let elisp_form = r##"(progn
         (require 'cl-lib)
         (with-temp-buffer
           (ahk-mode)
           (insert
            "; explain the workflow\n"
            "title := \"Quarterly `\"Report`\"\"\n"
            "path := 'C:\\reports\\today.csv'\n"
            "/* disable while deploying\n"
           "Run, staging.exe\n"
            "*/\n")
           (font-lock-ensure)
           (mapcar
            (lambda (text)
              (goto-char (point-min))
              (search-forward text)
              (list
               text
               (get-text-property
                (match-beginning 0)
                'face)
               (get-text-property
                (match-beginning 0)
                'font-lock-face)
               (nth
                4
                (syntax-ppss
                 (match-beginning 0)))
               (nth
                3
                (syntax-ppss
                 (match-beginning 0)))))
            '("explain" "Quarterly" "C:\\reports"
              "disable" "staging.exe"))))"##;
    let expect = expect![[
        r#"OK (("explain" font-lock-comment-face nil t nil) ("Quarterly" font-lock-string-face nil nil 34) ("C:\\reports" font-lock-string-face nil nil nil) ("disable" font-lock-comment-face nil t nil) ("staging.exe" font-lock-comment-face nil t nil))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_fontifies_complete_practical_language_vocabulary_by_role() {
    let elisp_form = r##"(progn
         (require 'cl-lib)
         (with-temp-buffer
           (ahk-mode)
           (insert
            "#SingleInstance Force\n"
            "BuildReport(name) {\n"
            "  report := A_ScriptDir . \"\\\\\" . name\n"
            "  if FileExist(report) AND GetKeyState(\"NumpadEnter\")\n"
            "    MsgBox, 64, Report, %report%\n"
            "  return report\n"
            "}\n")
           (font-lock-ensure)
           (cl-labels
               ((role
                 (text occurrence)
                 (goto-char (point-min))
                 (let ((count 0))
                   (while (< count occurrence)
                     (search-forward text)
                     (setq count (1+ count))))
                 (list
                  text
                  (get-text-property
                   (match-beginning 0)
                   'face)
                  (get-text-property
                   (match-beginning 0)
                   'font-lock-face))))
             (list
              (role "#SingleInstance" 1)
              (role "BuildReport" 1)
              (role ":=" 1)
              (role "A_ScriptDir" 1)
              (role "FileExist" 1)
              (role "AND" 1)
              (role "GetKeyState" 1)
              (role "NumpadEnter" 1)
              (role "MsgBox" 1)
              (role "%report%" 1)
              (role "return" 1)))))"##;
    let expect = expect![[
        r##"OK (("#SingleInstance" font-lock-preprocessor-face nil) ("BuildReport" font-lock-function-name-face nil) (":=" font-lock-builtin-face nil) ("A_ScriptDir" font-lock-variable-name-face nil) ("FileExist" font-lock-function-name-face nil) ("AND" font-lock-keyword-face nil) ("GetKeyState" font-lock-keyword-face nil) ("NumpadEnter" font-lock-string-face nil) ("MsgBox" font-lock-keyword-face nil) ("%report%" font-lock-variable-name-face nil) ("return" font-lock-warning-face nil))"##
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_distinguishes_hotkeys_labels_hotstrings_functions_and_return() {
    let elisp_form = r##"(progn
         (require 'cl-lib)
         (with-temp-buffer
           (ahk-mode)
           (insert
            "^!r::Run, report.exe\n"
            "WorkerReady:\n"
            ":*:addr::123 Main Street\n"
            "Deploy(environment)\n"
            "{\n"
            "  return environment\n"
            "}\n")
           (font-lock-ensure)
           (cl-labels
               ((face-for
                 (text)
                 (goto-char (point-min))
                 (search-forward text)
                 (list
                  text
                  (get-text-property
                   (match-beginning 0)
                   'face)
                  (get-text-property
                   (match-beginning 0)
                   'font-lock-face))))
             (mapcar
              #'face-for
              '("^!r" "WorkerReady" ":*:addr::"
                "Deploy" "return")))))"##;
    let expect = expect![[
        r#"OK (("^!r" font-lock-constant-face nil) ("WorkerReady" font-lock-doc-face nil) (":*:addr::" nil nil) ("Deploy" font-lock-function-name-face nil) ("return" font-lock-warning-face nil))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_ltrim_matcher_captures_complete_multiline_continuation_section() {
    let elisp_form = r##"(with-temp-buffer
         (ahk-mode)
         (insert
          "message =\n"
          "(LTrim0 Join`n\n"
          "First report line\n"
          "  Second report line\n"
          ")\n"
          "MsgBox, %message%\n")
         (goto-char (point-max))
         (search-backward "Second")
         (let ((matched
                (ahk-ltrim-blocks)))
           (list
            matched
            (and matched
                 (match-string-no-properties 0))
            (and matched
                 (line-number-at-pos
                  (match-beginning 0)))
            (and matched
                 (line-number-at-pos
                  (match-end 0)))
            (point))))"##;
    let expect =
        expect![[r#"OK (t "(LTrim0 Join`n\nFirst report line\n  Second report line\n)" 2 5 66)"#]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_font_lock_extension_expands_dirty_region_across_ltrim_block() {
    let elisp_form = r##"(progn
         (defvar font-lock-beg)
         (defvar font-lock-end)
         (with-temp-buffer
           (insert
            "before := 1\n"
            "text =\n"
            "(LTrim0\n"
            "alpha\n"
            "beta\n"
            ")\n"
            "after := 2\n")
           (let ((font-lock-beg
                  (progn
                    (goto-char (point-min))
                    (search-forward "alpha")
                    (match-beginning 0)))
                 (font-lock-end
                  (progn
                    (goto-char (point-min))
                    (search-forward "beta")
                    (match-end 0))))
             (let ((result
                    (ahk-font-lock-extend-region)))
               (list
                result
                (line-number-at-pos font-lock-beg)
                (line-number-at-pos font-lock-end)
                (buffer-substring-no-properties
                 font-lock-beg
                 font-lock-end))))))"##;
    let expect = expect![[r#"OK (20 3 6 "(LTrim0\nalpha\nbeta\n")"#]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_refontifies_edited_command_function_and_variable_roles() {
    let elisp_form = r##"(with-temp-buffer
         (ahk-mode)
         (insert "MsgBox, %A_Index%\n")
         (font-lock-ensure)
         (let ((before
                (mapcar
                 (lambda (text)
                   (goto-char (point-min))
                   (search-forward text)
                   (list
                    text
                    (get-text-property
                     (match-beginning 0)
                     'face)))
                 '("MsgBox" "A_Index"))))
           (let ((inhibit-modification-hooks nil))
             (goto-char (point-min))
             (delete-region
              (point)
              (line-end-position))
             (insert
              "result := RegExReplace(A_ScriptName, \"\\\\.ahk$\")"))
           (font-lock-flush (point-min) (point-max))
           (font-lock-ensure)
           (list
            before
            (mapcar
             (lambda (text)
               (goto-char (point-min))
               (search-forward text)
               (list
                text
                (get-text-property
                 (match-beginning 0)
                 'face)))
             '(":=" "RegExReplace" "A_ScriptName")))))"##;
    let expect = expect![[
        r#"OK ((("MsgBox" font-lock-keyword-face) ("A_Index" font-lock-variable-name-face)) ((":=" font-lock-builtin-face) ("RegExReplace" font-lock-function-name-face) ("A_ScriptName" font-lock-variable-name-face)))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}
