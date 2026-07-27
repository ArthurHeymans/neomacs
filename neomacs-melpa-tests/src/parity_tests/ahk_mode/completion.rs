use expect_test::expect;

use super::assert_ahk_mode_parity;

#[test]
fn ahk_mode_completion_returns_real_case_insensitive_command_function_and_variable_candidates() {
    let elisp_form = r##"(mapcar
         (lambda (prefix)
           (with-temp-buffer
             (ahk-mode)
             (insert prefix)
             (let ((completion
                    (ahk-completion-at-point)))
               (list
                prefix
                (and completion
                     (buffer-substring-no-properties
                      (nth 0 completion)
                      (nth 1 completion)))
                (nth 2 completion)
                (and completion
                     (plist-get
                      (nthcdr 3 completion)
                      :exclusive))
                (and completion
                     (plist-get
                      (nthcdr 3 completion)
                      :annotation-function))))))
         '("msgb" "regexr" "a_scriptf"
           "numpad" "#sing"))"##;
    let expect = expect![[
        r##"OK (("msgb" "msgb" ("MsgBox") no ahk-company-annotation) ("regexr" "regexr" ("RegExReplace") no ahk-company-annotation) ("a_scriptf" "a_scriptf" ("A_ScriptFullPath") no ahk-company-annotation) ("numpad" "numpad" nil no ahk-company-annotation) ("#sing" "#sing" nil no ahk-company-annotation))"##
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_completion_respects_word_boundaries_point_and_nonexclusive_fallback() {
    let elisp_form = r##"(mapcar
         (lambda (case)
           (with-temp-buffer
             (ahk-mode)
             (insert (car case))
             (goto-char
              (if (eq (cadr case) 'end)
                  (point-max)
                (cadr case)))
             (condition-case error-data
                 (let ((completion
                        (ahk-completion-at-point)))
                   (list
                    (car case)
                    (point)
                    (and completion
                         (list
                          (nth 0 completion)
                          (nth 1 completion)
                          (nth 2 completion)
                          (plist-get
                           (nthcdr 3 completion)
                           :exclusive)))))
               (error
                (list
                 (car case)
                 (point)
                 error-data)))))
         '(("RunWa" end)
           ("prefix RunWa" end)
           ("RunWait suffix" 4)
           ("RunWait " end)
           (":=" end)
           ("" end)))"##;
    let expect = expect![[
        r#"OK (("RunWa" 6 (1 6 ("RunWait") no)) ("prefix RunWa" 13 (8 13 ("RunWait") no)) ("RunWait suffix" 4 (1 4 ("Run" "RunAs" "RunWait") no)) ("RunWait " 9 nil) (":=" 3 (search-failed "\\<\\w+")) ("" 1 (search-failed "\\<\\w+")))"#
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_company_annotations_cover_each_catalog_and_unknown_candidates() {
    let elisp_form = r##"(mapcar
         (lambda (candidate)
           (list
            candidate
            (ahk-company-annotation candidate)))
         '("MsgBox"
           "RegExReplace"
           "A_ScriptDir"
           "#SingleInstance"
           "NumpadEnter"
           "msgbox"
           "UserDefinedFunction"))"##;
    let expect = expect![[
        r##"OK (("MsgBox" "c") ("RegExReplace" "f") ("A_ScriptDir" "v") ("#SingleInstance" "d") ("NumpadEnter" "k") ("msgbox" "") ("UserDefinedFunction" ""))"##
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_keyword_hash_and_completion_catalogs_remain_internally_coherent() {
    let elisp_form = r##"(let (missing-from-hash
               hash-only
               duplicate-completions)
         (dolist (candidate ahk-all-keywords)
           (unless (gethash candidate ahk-kwd-list)
             (push candidate missing-from-hash))
           (when (> (cl-count
                     candidate
                     ahk-all-keywords
                     :test #'equal)
                    1)
             (cl-pushnew
              candidate
              duplicate-completions
              :test #'equal)))
         (maphash
          (lambda (candidate _)
            (unless (member candidate
                            ahk-all-keywords)
              (push candidate hash-only)))
          ahk-kwd-list)
         (list
          (length ahk-all-keywords)
          (hash-table-count ahk-kwd-list)
          (sort missing-from-hash #'string-lessp)
          (sort duplicate-completions
                #'string-lessp)
          (length hash-only)
          (cl-subseq
           (sort hash-only #'string-lessp)
           0
           (min 12 (length hash-only)))))"##;
    let expect = expect![[
        r##"OK (719 905 nil ("Asc" "Ceil" "Chr" "Exp" "Floor" "GetKeyState" "Ln" "Log" "Mod" "Round" "Sin" "Sqrt" "Tan") 199 ("#ClipboardTimeout" "#CommentFlag" "#ErrorStdOut" "#EscapeChar" "#HotkeyInterval" "#HotkeyModifierTimeout" "#Hotstring" "#If" "#IfTimeout" "#IfWinActive" "#IfWinExist" "#Include"))"##
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_auto_complete_sources_generate_role_specific_practical_candidates() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (source)
            (list
             (cdr
              (assq 'limit
                    (symbol-value source)))
             (cdr
              (assq 'symbol
                    (symbol-value source)))))
          '(ac-source-ahk
            ac-source-keys-ahk
            ac-source-directives-ahk))
         (eval
          (cdr (assq 'candidates
                     ac-source-ahk))
          '((ac-prefix . "RegExR")))
         (eval
          (cdr (assq 'candidates
                     ac-source-keys-ahk))
          '((ac-prefix . "NumpadE")))
         (eval
          (cdr (assq 'candidates
                     ac-source-directives-ahk))
          '((ac-prefix . "#Single"))))"##;
    let expect = expect![[
        r##"OK (((nil "f") (nil "k") (nil "d")) ("RegExReplace") ("NumpadEnd" "NumpadEnter") ("#SingleInstance"))"##
    ]];
    assert_ahk_mode_parity(elisp_form, expect);
}

#[test]
fn ahk_mode_integrates_with_late_loaded_auto_complete_using_real_load_cycle() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "ahk-mode-auto-complete"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
                (config
                 (expand-file-name
                  "auto-complete-config.el"
                  root))
                (library
                 (expand-file-name
                  "auto-complete.el"
                  root)))
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region
                "(defvar ac-modes '(fundamental-mode))\n(provide 'auto-complete-config)\n"
                nil config nil 'silent)
               (write-region
                "(require 'auto-complete-config)\n(defvar ac-sources '(base-source))\n(provide 'auto-complete)\n"
                nil library nil 'silent)
               (let ((load-path
                      (cons root load-path)))
                 (load "auto-complete"
                       nil t)
                 (with-temp-buffer
                   (ahk-mode)
                   (list
                    ac-modes
                    ac-sources
                    (local-variable-p
                     'ac-sources)
                    (featurep
                     'auto-complete)
                    (featurep
                     'auto-complete-config)))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect!["OK ((ahk-mode fundamental-mode) (base-source) nil t t)"];
    assert_ahk_mode_parity(elisp_form, expect);
}
