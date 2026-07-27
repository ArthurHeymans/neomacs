use expect_test::expect;

use super::assert_ahk_mode_parity;

#[test]
fn ahk_mode_calculates_mixed_tab_space_indentation_with_offsets() {
    let elisp_form = r##"(let ((ahk-indentation 3)
               (tab-width 8))
         (mapcar
          (lambda (case)
            (list
             (car case)
             (cadr case)
             (ahk-calc-indentation
              (car case)
              (cadr case))))
          '(("" nil)
            ("   " nil)
            ("\t" nil)
            ("\t  " nil)
            (" \t \t" nil)
            ("  " 2)
            ("\t " -1))))"##;
    let expect = expect![[
        r#"OK (("" nil 0) ("   " nil 3) ("\11" nil 8) ("\11  " nil 10) (" \11 \11" nil 18) ("  " 2 8) ("\11 " -1 6))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_finds_previous_nonblank_indentation_without_moving_point() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "root\n"
          "    child\n"
          "\n"
          " \t \n"
          "current")
         (goto-char (point-max))
         (let ((before (point)))
           (list
            (ahk-previous-indent)
            (= before (point))
            (line-number-at-pos)
            (current-column))))"##;
    let expect = expect!["OK (4 t 5 7)"];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_indents_real_function_and_nested_conditional_script() {
    let elisp_form = r##"(let ((ahk-indentation 2))
         (with-temp-buffer
           (ahk-mode)
           (insert
            "BuildReport(name) {\n"
            "path := A_ScriptDir . \"\\\\reports\\\\\" . name\n"
            "if FileExist(path) {\n"
            "MsgBox, 64, Report, % \"Found \" . path\n"
            "} else {\n"
            "FileAppend, %name%, %path%\n"
            "}\n"
            "return path\n"
            "}\n")
           (ahk-indent-region (point-min) (point-max))
           (list
            (buffer-string)
            (save-excursion
              (goto-char (point-min))
              (let (levels)
                (while (not (eobp))
                  (push
                   (current-indentation)
                   levels)
                  (forward-line 1))
                (nreverse levels))))))"##;
    let expect = expect![[
        r#"OK ("  BuildReport(name) {\n    path := A_ScriptDir . \"\\\\reports\\\\\" . name\n    if FileExist(path) {\n      MsgBox, 64, Report, % \"Found \" . path\n    } else {\n      FileAppend, %name%, %path%\n}\nreturn path\n}\n" (2 4 4 6 4 6 0 0 0))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_indents_labels_hotkeys_loops_and_returns_as_executable_ahk() {
    let elisp_form = r##"(let ((ahk-indentation 4))
         (with-temp-buffer
           (ahk-mode)
           (insert
            "StartWorker:\n"
            "Run, worker.exe\n"
            "return\n"
            "\n"
            "^!r::\n"
            "Loop, 2\n"
            "{\n"
            "MsgBox, %A_Index%\n"
            "}\n"
            "return\n"
            "\n"
            "Esc::ExitApp\n")
           (ahk-indent-region (point-min) (point-max))
           (buffer-string)))"##;
    let expect = expect![[
        r#"OK "StartWorker:\n    Run, worker.exe\n    return\n\n^!r::\nLoop, 2\n{\n    MsgBox, %A_Index%\n}\nreturn\n\nEsc::ExitApp\n""#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_indents_multiline_calls_object_literals_and_following_statements() {
    let elisp_form = r##"(let ((ahk-indentation 2))
         (with-temp-buffer
           (ahk-mode)
           (insert
            "result := DllCall(\"SetWindowPos\"\n"
            ", \"uint\", windowId\n"
            ", \"int\", 0)\n"
            "options := { retry: 3\n"
            ", timeout: 5000}\n"
            "MsgBox, % result . options.timeout\n")
           (ahk-indent-region (point-min) (point-max))
           (buffer-string)))"##;
    let expect = expect![[
        r#"OK "  result := DllCall(\"SetWindowPos\"\n    , \"uint\", windowId\n    , \"int\", 0)\n    options := { retry: 3\n      , timeout: 5000}\n    MsgBox, % result . options.timeout\n""#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_uses_matching_openers_for_nested_closing_braces_and_parentheses() {
    let elisp_form = r##"(let ((ahk-indentation 2))
         (with-temp-buffer
           (ahk-mode)
           (insert
            "Main() {\n"
            "if (ready) {\n"
            "value := (\n"
            "first + second\n"
            ")\n"
            "}\n"
            "}\n")
           (ahk-indent-region (point-min) (point-max))
           (list
            (buffer-string)
            (save-excursion
              (goto-char (point-min))
              (forward-line 4)
              (list
               (current-indentation)
               (progn
                 (forward-line 1)
                 (current-indentation))
               (progn
                 (forward-line 1)
                 (current-indentation)))))))"##;
    let expect = expect![[
        r#"OK ("  Main() {\n    if (ready) {\n      value := (\n\11first + second\n)\n}\n}\n" (0 0 0))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_region_indentation_honors_requested_end_and_restores_point() {
    let elisp_form = r##"(let ((ahk-indentation 2))
         (with-temp-buffer
           (ahk-mode)
           (insert
            "if enabled\n"
            "MsgBox, enabled\n"
            "untouched\n")
           (goto-char (point-max))
           (let ((before (point))
                 (end
                  (save-excursion
                    (goto-char (point-min))
                    (forward-line 2)
                    (point))))
             (ahk-indent-region (point-min) end)
             (list
              (= before (point))
              (point)
              (buffer-string)))))"##;
    let expect = expect![[r#"OK (nil 43 "    if enabled\n\11MsgBox, enabled\nuntouched\n")"#]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_indent_line_preserves_text_point_and_reports_debug_decisions() {
    let elisp_form = r##"(let ((ahk-indentation 2)
               (ahk-debug t))
         (with-temp-buffer
           (ahk-mode)
           (insert "if ready {\nMsgBox, ready\n}\n")
           (goto-char (point-min))
           (forward-line 1)
           (search-forward "ready")
           (let ((offset
                  (- (point)
                     (line-beginning-position))))
             (ahk-indent-line)
             (list
              (buffer-string)
              (current-indentation)
              (current-column)
              offset
              (current-message)
              (list
               opening-paren
               if-else
               keybinding
               blank
               prev)))))"##;
    let expect =
        expect![[r#"OK ("if ready {\n  MsgBox, ready\n}\n" 2 15 13 nil (nil nil nil nil 0))"#]];
    assert_ahk_mode_parity(elisp_form, expect);
}
