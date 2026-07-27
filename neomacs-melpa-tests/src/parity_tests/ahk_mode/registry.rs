use expect_test::expect;

use super::{assert_ahk_mode_autoload_parity, assert_ahk_mode_parity};

#[test]
fn ahk_mode_defaults_custom_metadata_keyword_catalogs_and_hash_contract_match() {
    let elisp_form = r##"(list
         (featurep 'ahk-mode)
         ahk-mode-version
         (list
          ahk-indentation
          (get 'ahk-indentation 'custom-type)
          (get 'ahk-indentation 'custom-group)
          (custom-variable-p 'ahk-indentation))
         (list ahk-debug ahk-path)
         (mapcar
          #'length
          (list
           ahk-commands
           ahk-directives
           ahk-functions
           ahk-variables
           ahk-keys
           ahk-operator-words
           ahk-operators
           ahk-all-keywords))
         (hash-table-test ahk-kwd-list)
         (hash-table-count ahk-kwd-list)
         (get 'ahk-kwd-list 'risky-local-variable)
         (mapcar
          (lambda (keyword)
            (list
             keyword
             (gethash keyword ahk-kwd-list)
             (and
              (member keyword ahk-all-keywords)
              t)))
          '("MsgBox" "#SingleInstance" "RegExReplace"
            "A_ScriptDir" "NumpadEnter" "not-a-keyword")))"##;
    let expect = expect![[
        r##"OK (t "1.5.6" (8 integer nil ((funcall #'#[nil ((or tab-width 2)) (company-tooltip-align-annotations ac-modes t)]))) (nil nil) (488 30 85 146 175 3 42 719) equal 905 t (("MsgBox" t t) ("#SingleInstance" t nil) ("RegExReplace" t t) ("A_ScriptDir" t t) ("NumpadEnter" t nil) ("not-a-keyword" nil nil)))"##
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_complete_callable_surface_arglists_and_command_status_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist symbol t)
            (commandp symbol)
            (macrop symbol)
            (autoloadp (symbol-function symbol))))
         '(ahk-run-script
           ahk-command-at-point
           ahk-lookup-web
           ahk-lookup-chm
           ahk-version
           ahk-calc-indentation
           ahk-previous-indent
           ahk-indent-message
           ahk-indent-line
           ahk-indent-region
           ahk-comment-dwim
           ahk-comment-block-dwim
           ahk-completion-at-point
           ahk-company-annotation
           ahk-font-lock-extend-region
           ahk-ltrim-blocks
           ahk-mode))"##;
    let expect = expect![
        "OK ((ahk-run-script nil t nil nil) (ahk-command-at-point nil nil nil nil) (ahk-lookup-web nil t nil nil) (ahk-lookup-chm nil t nil nil) (ahk-version nil t nil nil) (ahk-calc-indentation (str &optional offset) nil nil nil) (ahk-previous-indent nil nil nil nil) (ahk-indent-message nil t nil nil) (ahk-indent-line nil t nil nil) (ahk-indent-region (start end) t nil nil) (ahk-comment-dwim (arg) t nil nil) (ahk-comment-block-dwim (arg) t nil nil) (ahk-completion-at-point nil t nil nil) (ahk-company-annotation (candidate) nil nil nil) (ahk-font-lock-extend-region nil nil nil nil) (ahk-ltrim-blocks nil nil nil nil) (ahk-mode nil t nil nil))"
    ];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_keymap_menu_bindings_and_auto_mode_dispatch_contract_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (key)
            (list key
                  (lookup-key ahk-mode-map
                              (kbd key))))
          '("C-c C-?" "C-c C-r" "C-c M-i"
            "C-c C-c" "C-c C-b" "C-c C-k"))
         (keymapp ahk-mode-map)
         (keymapp ahk-menu)
         (car ahk-menu)
         (cdr (assoc "\\.ahk\\'" auto-mode-alist))
         (with-temp-buffer
           (set-visited-file-name
            (expand-file-name
             "automation.ahk"
             (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
           (set-auto-mode)
           (list major-mode mode-name)))"##;
    let expect = expect![[
        r#"OK ((("C-c C-?" ahk-lookup-web) ("C-c C-r" ahk-lookup-chm) ("C-c M-i" ahk-indent-message) ("C-c C-c" ahk-comment-dwim) ("C-c C-b" ahk-comment-block-dwim) ("C-c C-k" ahk-run-script)) t t keymap ahk-mode (ahk-mode "AHK"))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_syntax_table_handles_words_escapes_line_and_block_comments() {
    let elisp_form = r##"(with-temp-buffer
         (set-syntax-table ahk-mode-syntax-table)
         (insert
          "name_#@ C:\\scripts\\demo.ahk ; line\n"
          "/* block\ncomment */\n"
          "\"escaped`\"quote\"")
         (let ((classes
                (mapcar
                 (lambda (character)
                   (cons character
                         (char-syntax character)))
                 '(?# ?_ ?@ ?\\ ?\; ?/ ?* ?\n ?`))))
           (goto-char (point-min))
           (search-forward "line")
           (let ((line-state
                  (syntax-ppss
                   (match-beginning 0))))
             (search-forward "block")
             (let ((block-state
                    (syntax-ppss
                     (match-beginning 0))))
               (search-forward "escaped")
               (let ((string-state
                      (syntax-ppss
                       (match-beginning 0))))
                 (list
                  classes
                  (nth 4 line-state)
                  (nth 4 block-state)
                  (nth 3 string-state)))))))"##;
    let expect = expect![
        "OK (((35 . 119) (95 . 119) (64 . 119) (92 . 119) (59 . 60) (47 . 46) (42 . 46) (10 . 62) (96 . 92)) t t 34)"
    ];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_imenu_definition_covers_functions_labels_hotkeys_hotstrings_and_comments() {
    let elisp_form = r##"(progn
         (require 'imenu)
         (list
          ahk-imenu-generic-expression
          (mapcar
           (lambda (entry)
             (list
              (car entry)
              (nth 2 entry)))
           ahk-imenu-generic-expression)
          (with-temp-buffer
            (insert
             "BuildReport(name)\n{\n  return name\n}\n"
             "StartWorker:\n"
             "^!r::Run, report.exe\n"
             ":*:btw::by the way\n"
             ";imenu Deployment helpers\n")
            (mapcar
             (lambda (group)
               (cons
                (car group)
                (mapcar #'car (cdr group))))
             (imenu--generic-function
              ahk-imenu-generic-expression)))))"##;
    let expect = expect![[
        r#"OK ((("Functions" "^[ \11]*\\([^ ]+\\)(.*)[\n]{" 1) ("Labels" "^[ \11]*\\([^:;]+\\):\n" 1) ("Keybindings" "^[ \11]*\\([^;: \11\15\n\13\f].*?\\)::" 1) ("Hotstrings" "^[ \11]*\\(:.*?:.*?::\\)" 1) ("Comments" "^;imenu \\(.+\\)" 1)) (("Functions" 1) ("Labels" 1) ("Keybindings" 1) ("Hotstrings" 1) ("Comments" 1)) (("Comments" "Deployment helpers") ("Hotstrings" ":*:btw::") ("Keybindings" "^!r") ("Labels" "StartWorker") ("Functions" "BuildReport")))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_regexps_classify_real_language_tokens_case_insensitively() {
    let elisp_form = r##"(let ((case-fold-search t))
         (mapcar
          (lambda (case)
            (let ((regexp (symbol-value (car case)))
                  (text (cadr case)))
              (list
               (car case)
               text
               (and
                (string-match regexp text)
                (match-string 0 text)))))
          '((ahk-commands-regexp "msgbox")
            (ahk-functions-regexp "regexreplace")
            (ahk-directives-regexp "#singleinstance")
            (ahk-variables-regexp "a_scriptdir")
            (ahk-keys-regexp "numpadenter")
            (ahk-operator-words-regexp "and")
            (ahk-operators-regexp "<<=")
            (ahk-double-quote-string-re "\"a`\"b\"")
            (ahk-single-quote-string-re "'path\\name'"))))"##;
    let expect = expect![[
        r##"OK ((ahk-commands-regexp "msgbox" "msgbox") (ahk-functions-regexp "regexreplace" "regexreplace") (ahk-directives-regexp "#singleinstance" nil) (ahk-variables-regexp "a_scriptdir" "a_scriptdir") (ahk-keys-regexp "numpadenter" "numpadenter") (ahk-operator-words-regexp "and" "and") (ahk-operators-regexp "<<=" "<<=") (ahk-double-quote-string-re "\"a`\"b\"" "\"a`\"") (ahk-single-quote-string-re "'path\\name'" "'path\\name'"))"##
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_initializes_complete_practical_buffer_local_editor_state() {
    let elisp_form = r##"(let ((evil-shift-width 99))
         (with-temp-buffer
           (ahk-mode)
           (list
            major-mode
            mode-name
            (derived-mode-p 'prog-mode)
            (eq (current-local-map) ahk-mode-map)
            (eq (syntax-table) ahk-mode-syntax-table)
            imenu-generic-expression
            imenu-sort-function
            font-lock-defaults
            comment-start
            comment-end
            comment-start-skip
            (list
             block-comment-start
             block-comment-end
             block-comment-left
             block-comment-right
             block-comment-char)
            indent-line-function
            indent-region-function
            parse-sexp-ignore-comments
            parse-sexp-lookup-properties
            paragraph-start
            paragraph-separate
            paragraph-ignore-fill-prefix
            company-tooltip-align-annotations
            completion-at-point-functions
            evil-shift-width
            (local-variable-p 'evil-shift-width)
            (local-variable-p 'font-lock-defaults))))"##;
    let expect = expect![[
        r#"OK (ahk-mode "AHK" prog-mode t t (("Functions" "^[ \11]*\\([^ ]+\\)(.*)[\n]{" 1) ("Labels" "^[ \11]*\\([^:;]+\\):\n" 1) ("Keybindings" "^[ \11]*\\([^;: \11\15\n\13\f].*?\\)::" 1) ("Hotstrings" "^[ \11]*\\(:.*?:.*?::\\)" 1) ("Comments" "^;imenu \\(.+\\)" 1)) imenu--sort-by-position ((ahk-font-lock-keywords) nil t) ";" "" ";+ *" ("/*" "*/" " * " " *" 42) ahk-indent-line ahk-indent-region t t "$\\|^\f" "$\\|^\f" t t (ahk-completion-at-point t) 99 nil t)"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_runs_mode_hook_once_and_preserves_mode_specific_state_after_reentry() {
    let elisp_form = r##"(progn
         (defvar ahk-mode-test-hook-calls)
         (let ((ahk-mode-test-hook-calls 0)
               (ahk-mode-hook
                (list
                 (lambda ()
                   (setq
                    ahk-mode-test-hook-calls
                    (1+ ahk-mode-test-hook-calls))
                   (insert
                    (format
                     "<hook-%d>"
                     ahk-mode-test-hook-calls))))))
           (with-temp-buffer
             (insert "stale")
             (ahk-mode)
             (let ((first
                    (list
                     ahk-mode-test-hook-calls
                     (buffer-string)
                     major-mode
                     comment-start)))
               (ahk-mode)
               (list
                first
                ahk-mode-test-hook-calls
                (buffer-string)
                major-mode
                comment-start
                (length
                 completion-at-point-functions))))))"##;
    let expect = expect![[
        r#"OK ((2 "stale<hook-1><hook-2>" ahk-mode ";") 4 "stale<hook-1><hook-2><hook-3><hook-4>" ahk-mode ";" 2)"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_autoload_file_registers_mode_command_and_extension_without_loading_feature() {
    let elisp_form = r##"(list
         (featurep 'ahk-mode)
         (fboundp 'ahk-mode)
         (autoloadp (symbol-function 'ahk-mode))
         (help-function-arglist 'ahk-mode t)
         (commandp 'ahk-mode)
         (cdr (assoc "\\.ahk\\'" auto-mode-alist))
         (nth 1 (symbol-function 'ahk-mode)))"##;
    let expect = expect![[
        r#"OK (nil t t "[Arg list not available until function definition is loaded.]" t ahk-mode "ahk-mode")"#
    ]];
    assert_ahk_mode_autoload_parity(elisp_form, expect);
}
