use expect_test::expect;

use super::assert_ahk_mode_parity;

#[test]
fn ahk_mode_resolves_command_from_symbol_property_free_region_and_prompt_fallback() {
    let elisp_form = r##"(list
         (with-temp-buffer
           (ahk-mode)
           (insert
            "MsgBox A_ScriptDir user_value")
           (search-backward "A_ScriptDir")
           (ahk-command-at-point))
         (with-temp-buffer
           (ahk-mode)
           (insert
            (propertize
             "RegExReplace"
             'face 'bold
             'melpa-test-property t))
           (let ((transient-mark-mode t))
             (goto-char (point-min))
             (push-mark (point-max) t t)
             (let ((command
                    (ahk-command-at-point)))
               (list
                command
                (text-properties-at
                 0 command)))))
         (with-temp-buffer
           (let (prompts)
             (cl-letf
                 (((symbol-function
                    'read-string)
                   (lambda (prompt)
                     (push prompt prompts)
                     "FileAppend")))
               (list
                (ahk-command-at-point)
                (nreverse prompts))))))"##;
    let expect =
        expect![[r#"OK ("A_ScriptDir" ("RegExReplace" nil) ("FileAppend" ("Command: ")))"#]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_web_lookup_builds_command_reference_url_from_real_buffer_context() {
    let elisp_form = r##"(let (requests)
         (cl-letf
             (((symbol-function
                'browse-url)
               (lambda
                 (url &rest arguments)
                 (push
                  (cons url arguments)
                  requests)
                 'opened-browser)))
           (with-temp-buffer
             (ahk-mode)
             (insert
              "if RegExReplace(value, pattern)")
             (search-backward "RegExReplace")
             (list
              (ahk-lookup-web)
              (nreverse requests)))))"##;
    let expect = expect![[
        r#"OK (opened-browser (("http://ahkscript.org/docs/commands/RegExReplace.htm")))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_local_help_uses_custom_chm_and_exact_html_help_topic() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "ahk-mode custom help"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (manual
                 (expand-file-name
                  "AutoHotkey.chm"
                  root))
                (ahk-path root)
                calls)
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region
                "deterministic help fixture"
                nil manual nil 'silent)
               (cl-letf
                   (((symbol-function
                      'w32-shell-execute)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       'opened-help)))
                 (with-temp-buffer
                   (ahk-mode)
                   (insert "MsgBox")
                   (goto-char (point-min))
                   (list
                    (ahk-lookup-chm)
                    (current-message)
                    (nreverse calls)))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (opened-help nil ((1 "hh.exe" "ms-its:[ORACLE-SANDBOX]/ahk-mode custom help/AutoHotkey.chm::/docs/commands/MsgBox.htm")))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_local_help_prefers_x86_then_standard_install_then_custom_path() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "ahk-mode-help-priority"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (default-directory
                 (file-name-as-directory root))
                (x86
                 (expand-file-name
                  "c:/Program Files (x86)/AutoHotkey/AutoHotkey.chm"
                  root))
                (standard
                 (expand-file-name
                  "c:/Program Files/AutoHotkey/AutoHotkey.chm"
                  root))
                (custom-directory
                 (expand-file-name "custom" root))
                (custom
                 (expand-file-name
                  "AutoHotkey.chm"
                  custom-directory))
                (ahk-path custom-directory)
                calls)
         (unwind-protect
             (progn
               (dolist (file
                        (list x86 standard custom))
                 (make-directory
                  (file-name-directory file)
                  t)
                 (write-region
                  file nil file nil 'silent))
               (cl-letf
                   (((symbol-function
                      'ahk-command-at-point)
                     (lambda () "Run"))
                    ((symbol-function
                      'w32-shell-execute)
                     (lambda (&rest arguments)
                       (push arguments calls))))
                 (ahk-lookup-chm)
                 (delete-file x86)
                 (ahk-lookup-chm)
                 (delete-file standard)
                 (ahk-lookup-chm)
                 (nreverse calls)))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK ((1 "hh.exe" "ms-its:c:/Program Files (x86)/AutoHotkey/AutoHotkey.chm::/docs/commands/Run.htm") (1 "hh.exe" "ms-its:c:/Program Files/AutoHotkey/AutoHotkey.chm::/docs/commands/Run.htm") (1 "hh.exe" "ms-its:[ORACLE-SANDBOX]/ahk-mode-help-priority/custom/AutoHotkey.chm::/docs/commands/Run.htm"))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_local_help_reports_missing_manual_without_launching_process() {
    let elisp_form = r##"(let ((ahk-path
                (expand-file-name
                 "missing-ahk-install"
                 (getenv
                  "NEOMACS_TEST_SANDBOX_ROOT")))
               calls)
         (cl-letf
             (((symbol-function
                'ahk-command-at-point)
               (lambda () "FileAppend"))
              ((symbol-function
                'w32-shell-execute)
               (lambda (&rest arguments)
                 (push arguments calls))))
           (list
            (ahk-lookup-chm)
            (current-message)
            calls)))"##;
    let expect =
        expect![[r#"OK ("Help file could not be found, set ahk-path variable." nil nil)"#]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_run_script_converts_real_visited_path_and_launches_windows_shell() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "ahk-mode scripts"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (script
                 (expand-file-name
                  "daily report.ahk"
                  root))
                calls)
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region
                "MsgBox, daily report\n"
                nil script nil 'silent)
               (cl-letf
                   (((symbol-function
                      'w32-shell-execute)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       'launched-script)))
                 (with-temp-buffer
                   (set-visited-file-name
                    script)
                   (insert-file-contents script)
                   (let ((result
                          (ahk-run-script))
                         (argument
                          (cadar calls)))
                     (list
                      result
                      (string-match-p
                       "ahk-modescripts"
                       argument)
                      (string-match-p
                       "dailyreport\\.ahk\\'"
                       argument)
                      (string-match-p
                       " "
                       argument)
                      (substring
                       argument
                       (string-match
                        "ahk-modescripts"
                        argument))
                      (caar calls))))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (launched-script 248 267 nil "ahk-modescripts\\\\\\\\dailyreport.ahk" "open")"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_version_and_indent_commands_report_current_user_visible_values() {
    let elisp_form = r##"(list
         (list
          (ahk-version)
          (current-message))
         (with-temp-buffer
           (insert "      MsgBox, aligned")
           (list
            (ahk-indent-message)
            (current-message)
            (current-indentation))))"##;
    let expect = expect![[r#"OK (("ahk-mode version 1.5.6" nil) ("6" nil 6))"#]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_line_comment_command_round_trips_selected_real_script_region() {
    let elisp_form = r##"(with-temp-buffer
         (ahk-mode)
         (insert
          "Run, worker.exe\n"
          "FileAppend, ready, status.txt\n")
         (let ((transient-mark-mode t))
           (goto-char (point-min))
           (push-mark (point-max) t t)
           (ahk-comment-dwim nil)
           (let ((commented
                  (buffer-string)))
             (goto-char (point-min))
             (push-mark (point-max) t t)
             (ahk-comment-dwim nil)
             (list
              commented
              (buffer-string)
              comment-start
              comment-end))))"##;
    let expect = expect![[
        r#"OK ("; Run, worker.exe\n; FileAppend, ready, status.txt\n" "Run, worker.exe\nFileAppend, ready, status.txt\n" ";" "")"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_block_comment_command_round_trips_multiline_deployment_section() {
    let elisp_form = r##"(with-temp-buffer
         (ahk-mode)
         (insert
          "RunWait, deploy.exe\n"
          "MsgBox, deployment complete\n")
         (let ((transient-mark-mode t))
           (goto-char (point-min))
           (push-mark (point-max) t t)
           (ahk-comment-block-dwim nil)
           (let ((commented
                  (buffer-string)))
             (goto-char (point-min))
             (push-mark (point-max) t t)
             (ahk-comment-block-dwim nil)
             (list
              commented
              (buffer-string)
              block-comment-start
              block-comment-end))))"##;
    let expect = expect![[
        r#"OK ("/*\n * RunWait, deploy.exe\n * MsgBox, deployment complete\n */\n" "RunWait, deploy.exe\nMsgBox, deployment complete\n" "/*" "*/")"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}
