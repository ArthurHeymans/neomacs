use expect_test::expect;

use super::{assert_ado_mode_autoload_parity, assert_ado_mode_parity};

#[test]
fn ado_mode_exact_pin_metadata_features_and_complete_callable_surface_match() {
    let elisp_form = r##"(progn
         (require 'lisp-mnt)
         (let ((descriptor (cadr (assq 'ado-mode package-alist)))
               callables)
           (mapatoms
            (lambda (symbol)
              (when (and (string-prefix-p "ado-" (symbol-name symbol))
                         (fboundp symbol)
                         (let ((file (symbol-file symbol 'defun)))
                           (and file
                                (string-prefix-p
                                 (file-truename
                                  (package-desc-dir descriptor))
                                 (file-truename file)))))
                (push symbol callables))))
           (list
            (package-desc-name descriptor)
            (package-version-join (package-desc-version descriptor))
            (package-desc-summary descriptor)
            (package-desc-kind descriptor)
            (package-desc-reqs descriptor)
            (package-desc-extras descriptor)
            (mapcar #'featurep
                    '(ado-mode ado-cus ado-cons ado-font ado-clip
                      ado-to-stata ado-font-lock ado-stata-info))
            (with-temp-buffer
              (insert-file-contents (getenv "NEOMACS_PACKAGE_SOURCE"))
              (list (lm-header "version") (lm-header "url")))
            (sort callables
                  (lambda (left right)
                    (string-lessp (symbol-name left)
                                  (symbol-name right)))))))"##;
    let expect = expect![[
        r#"OK (ado-mode "20260210.1431" "Major mode for editing Stata-related files." nil ((emacs (25 1))) ((:maintainers ("Bill Rising" . "brising@alum.mit.edu")) (:authors ("Bill Rising" . "brising@alum.mit.edu")) (:keywords "tools" "languages" "files" "convenience" "stata" "mata" "ado") (:revdesc . "371441d27027") (:commit . "371441d27027fd4783a8c828458a5af098babca4") (:url . "https://github.com/louabill/ado-mode")) (t t t t t t t t) (nil "https://github.com/louabill/ado-mode") (ado-add-font-lock-keywords ado-add-oldplace ado-add-personal ado-add-plus ado-add-site ado-add-sysdir-all ado-add-sysdir-font-lock-keywords ado-ask-filename ado-balance-brace ado-before-save-file ado-beginning-of-command ado-change-number ado-check-a-directory ado-clean-buffer ado-command-to-clip ado-comment-indent ado-continuation-indent ado-continued-statement-indent-spaces-change ado-convert-semicolons ado-copy-command ado-delimit-is-semi-p ado-electric-brace ado-electric-closing-brace ado-electric-semi ado-end-of-command ado-find-ado-dirs ado-find-depth ado-find-extension ado-find-help-name-start ado-find-help-name-start-pre12 ado-find-local-name ado-find-stata ado-font-lock-refresh ado-foreach-loop ado-forvalues-loop ado-get-filename-from-stata ado-get-one-result ado-get-stata-version ado-grab-block ado-grab-something ado-help-at-point ado-help-at-point-to-clip ado-help-command ado-help-command-to-clip ado-help-insert-option-in-body ado-indent-buffer ado-indent-line ado-indent-region ado-input-to-stata ado-insert-boilerplate ado-insert-file-and-indent ado-insert-new-program ado-insert-nice-current-date ado-insert-with-lfd ado-line-starts-with-end-comment ado-macify-selection-or-word ado-make-ado-name ado-make-help-name ado-mode ado-modify-font-lock-keywords ado-new-ado ado-new-class ado-new-cscript ado-new-do ado-new-generic ado-new-help ado-new-label ado-new-mata ado-new-program ado-new-testado ado-newline ado-next-error ado-nice-current-date ado-one-eol ado-open-any-file ado-open-command ado-open-file-on-adopath ado-other-to-clip ado-out-of-nested-comment ado-parse-loop ado-prev-error ado-remove-font-lock-keywords ado-remove-oldplace ado-remove-personal ado-remove-plus ado-remove-site ado-remove-sysdir-all ado-reset-adopath ado-reset-oldplace-dir ado-reset-personal-dir ado-reset-plus-dir ado-reset-site-dir ado-reset-sysdir ado-reset-tcc ado-reset-version-command ado-return-toggle ado-save-program ado-send-block-to-stata ado-send-buffer-to-stata ado-send-clip-to-stata ado-send-command-to-command ado-send-command-to-dofile ado-send-command-to-include ado-send-command-to-menu ado-send-command-to-stata ado-send2stata-name ado-set-ado-extension ado-set-ado-signature-file ado-set-font-lock-keywords ado-set-imenu-items ado-set-return ado-set-window-width ado-sho<w-extension ado-show-ado-name ado-show-delimiter ado-show-depth ado-show-local-name ado-show-stata ado-show-stata-version ado-show-tmp-dir ado-skip-header-lines ado-skip-special-comments ado-split-line ado-start-of-nested-comment ado-stata-help ado-statacorp-defaults ado-string-trim ado-string-trim-left ado-string-trim-right ado-stringify-selection ado-strip-after-newline ado-strip-comments ado-strmacify-selection-or-word ado-system-tmp-dir ado-tab-width-change ado-toggle-flag ado-toggle-help-extension ado-update-sysdir-all ado-update-timestamp ado-write-file-as-buffer-name))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_complete_callable_contracts_match() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr (assq 'ado-mode package-alist)))
                 (directory
                  (file-truename (package-desc-dir descriptor)))
                 callables)
         (mapatoms
          (lambda (symbol)
            (when (and (string-prefix-p "ado-" (symbol-name symbol))
                       (fboundp symbol)
                       (let ((file (symbol-file symbol 'defun)))
                         (and file
                              (string-prefix-p
                               directory
                               (file-truename file)))))
              (push symbol callables))))
         (let ((contracts
                (mapcar
                 (lambda (symbol)
                   (list symbol
                         (help-function-arglist symbol t)
                         (commandp symbol)
                         (interactive-form symbol)))
                 (sort callables
                       (lambda (left right)
                         (string-lessp (symbol-name left)
                                       (symbol-name right)))))))
           (list
            (length contracts)
            (secure-hash
             'sha256
             (let ((print-circle nil))
               (prin1-to-string contracts)))
            (car contracts)
            (nth 64 contracts)
            (car (last contracts)))))"##;
    let expect = expect![[
        r#"OK (140 "b7c960d6bb003360102cd3bfd468321c869ab969b84acad89676c30c09308c32" (ado-add-font-lock-keywords (name dir face &optional update refresh baddir subdir extension) nil nil) (ado-new-generic (type exten &optional stayput name purpose cusblp) nil nil) (ado-write-file-as-buffer-name nil t (interactive nil)))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_custom_variable_registry_matches_exact_defaults_types_and_groups() {
    let elisp_form = r##"(let* ((variables
                 '(ado-mode-hook ado-new-dir ado-personal-dir ado-plus-dir
                   ado-site-dir ado-oldplace-dir ado-confirm-overwrite-flag
                   ado-add-sysdir-font-lock ado-site-template-dir
                   ado-script-dir ado-mode-home ado-label-dir
                   ado-open-read-only-flag ado-signature-file
                   ado-signature-prompt-flag ado-help-author-flag
                   ado-claim-name ado-help-extension ado-smart-indent-flag
                   ado-return-also-indents-flag ado-do-indent-flag
                   ado-use-modern-split-flag ado-close-under-line-flag
                   ado-auto-newline-flag ado-closing-brace-alone-flag
                   ado-fontify-new-flag ado-tab-width
                   ado-continued-statement-indent-spaces ado-comment-column
                   ado-continuation-column ado-line-up-continuations
                   ado-debugging-indent-flag ado-debugging-indent-column
                   ado-delimit-indent-flag ado-delimit-indent-column
                   ado-comment-indent-flag ado-comment-start ado-comment-end
                   ado-comment-indent-column ado-update-timestamp-flag
                   ado-date-format ado-lowercase-date-flag ado-initials-flag
                   ado-initials ado-submit-default ado-comeback-flag
                   ado-stata-home ado-version-command ado-temp-dofile
                   ado-stata-instance ado-stata-version ado-stata-flavor
                   ado-strict-match-flag ado-send-to-all-flag
                   ado-before-save-file-hook))
                (contracts
                 (mapcar
                  (lambda (symbol)
                    (list symbol
                          (default-value symbol)
                          (eval
                           (car (get symbol 'standard-value)))
                          (get symbol 'custom-type)
                          (get symbol 'custom-group)
                          (get symbol 'safe-local-variable)))
                  variables)))
         (list
          variables
          (length contracts)
          (secure-hash
           'sha256
           (let ((print-circle nil))
             (prin1-to-string contracts)))
          (car contracts)
          (nth 27 contracts)
          (car (last contracts))))"##;
    let expect = expect![[
        r#"OK ((ado-mode-hook ado-new-dir ado-personal-dir ado-plus-dir ado-site-dir ado-oldplace-dir ado-confirm-overwrite-flag ado-add-sysdir-font-lock ado-site-template-dir ado-script-dir ado-mode-home ado-label-dir ado-open-read-only-flag ado-signature-file ado-signature-prompt-flag ado-help-author-flag ado-claim-name ado-help-extension ado-smart-indent-flag ado-return-also-indents-flag ado-do-indent-flag ado-use-modern-split-flag ado-close-under-line-flag ado-auto-newline-flag ado-closing-brace-alone-flag ado-fontify-new-flag ado-tab-width ado-continued-statement-indent-spaces ado-comment-column ado-continuation-column ado-line-up-continuations ado-debugging-indent-flag ado-debugging-indent-column ado-delimit-indent-flag ado-delimit-indent-column ado-comment-indent-flag ado-comment-start ado-comment-end ado-comment-indent-column ado-update-timestamp-flag ado-date-format ado-lowercase-date-flag ado-initials-flag ado-initials ado-submit-default ado-comeback-flag ado-stata-home ado-version-command ado-temp-dofile ado-stata-instance ado-stata-version ado-stata-flavor ado-strict-match-flag ado-send-to-all-flag ado-before-save-file-hook) 55 "124b3168d669a13e720ec7bcc3d98acbde8008b5f6fd4cc7291b48e1d6e4dbbc" (ado-mode-hook nil nil (hook) nil nil) (ado-continued-statement-indent-spaces 2 2 integer nil nil) (ado-before-save-file-hook ado-before-save-file ado-before-save-file hook nil nil))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_face_registry_matches_exact_specs_and_inheritance() {
    let elisp_form = r##"(let* ((faces
                 '(ado-builtin-harmful-face ado-builtin-harmless-face
                   ado-oldplace-harmful-face ado-oldplace-harmless-face
                   ado-plus-harmful-face ado-plus-harmless-face
                   ado-personal-harmful-face ado-personal-harmless-face
                   ado-site-harmful-face ado-site-harmless-face
                   ado-constant-face ado-platform-specific-face
                   ado-string-face ado-comment-face ado-variable-name-face
                   ado-matrix-name-face ado-function-name-face
                   ado-needs-subcommand-face ado-subcommand-face
                   ado-obsolete-face ado-mata-keyword-face
                   ado-mata-future-keyword-face
                   ado-mata-function-name-face))
                (contracts
                 (mapcar
                  (lambda (face)
                    (list face
                          (facep face)
                          (get face 'face-defface-spec)
                          (get face 'face-alias)
                          (face-documentation face)))
                  faces)))
         (list
          faces
          (length contracts)
          (secure-hash
           'sha256
           (let ((print-circle nil))
             (prin1-to-string contracts)))
          (car contracts)
          (nth 11 contracts)
          (car (last contracts))))"##;
    let expect = expect![[
        r#"OK ((ado-builtin-harmful-face ado-builtin-harmless-face ado-oldplace-harmful-face ado-oldplace-harmless-face ado-plus-harmful-face ado-plus-harmless-face ado-personal-harmful-face ado-personal-harmless-face ado-site-harmful-face ado-site-harmless-face ado-constant-face ado-platform-specific-face ado-string-face ado-comment-face ado-variable-name-face ado-matrix-name-face ado-function-name-face ado-needs-subcommand-face ado-subcommand-face ado-obsolete-face ado-mata-keyword-face ado-mata-future-keyword-face ado-mata-function-name-face) 23 "a1862318c987c9e1401fd062f44b19b7bad77556d424ebc93568b29f11f4c401" (ado-builtin-harmful-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-keyword-face)) nil "Ado mode face used to highlight builtin commands which change\ndata or the environment.") (ado-platform-specific-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit font-lock-constant-face :weight bold)) nil "Ado mode face used to highlight builtin contants which exist\nonly on particular platforms") (ado-mata-function-name-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t :inherit ado-function-name-face)) nil "Ado mode face used to highlight mata functions, of all things."))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_constants_extensions_syntax_keymap_and_abbrevs_match() {
    let elisp_form = r##"(list
         ado-extensions
         (mapcar
          (lambda (symbol) (list symbol (symbol-value symbol)))
          '(ado-capture-noisily-regexp ado-prefix-any-regexp
            ado-start-cmd-regexp ado-start-cmd-no-prefix-regexp
            ado-start-cmd-must-start-line-regexp ado-start-cmd-null-regexp
            ado-end-cmd-regexp ado-stata-name-wipe-bad-chars
            ado-stata-name-regexp ado-stata-name-bound-regexp
            ado-stata-varname-regexp ado-stata-local-name-regexp
            ado-stata-local-name-bound-regexp ado-man-abbrevs))
         (mapcar
          (lambda (character)
            (list character
                  (with-syntax-table ado-mode-syntax-table
                    (char-syntax character))))
          '(?\\ ?$ ?` ?' ?/ ?* ?+ ?- ?= ?% ?< ?> ?& ?| ?~ ?_))
         (mapcar
          (lambda (key)
            (list key (lookup-key ado-mode-map (kbd key))))
          '("TAB" "M-RET" "C-M-RET" "C-c C-b" "C-c M-b"
            "C-c C-e" "C-c C-v" "M-a" "M-e" "{" "}" ";"))
         (abbrev-table-p ado-mode-abbrev-table)
         (seq-every-p
          (lambda (extension)
            (equal (cdr (assoc (concat "\\." extension "\\'")
                               auto-mode-alist))
                   'ado-mode))
          ado-extensions))"##;
    let expect = expect![[
        r#"OK (("LBL" "CLASS" "SMCL" "DLG" "STHLP" "IHLP" "HLP" "MATA" "DO" "ADO" "lbl" "class" "smcl" "dlg" "sthlp" "ihlp" "hlp" "mata" "do" "ado") ((ado-capture-noisily-regexp "\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\)[ /t]+\\(?:n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\)") (ado-prefix-any-regexp "\\(?:[ \11]*\\(?:\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\|mata\\|n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\|python\\|qui\\(?:e\\(?:t\\(?:ly?\\)?\\)?\\)?\\)\\|\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\)[ /t]+\\(?:n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\)\\)\\(?:[ \11]*:\\)?\\)?") (ado-start-cmd-regexp "^\\(?:\\(?:.*:\\)*\\|\\(?:[ \11]*\\(?:\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\|mata\\|n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\|python\\|qui\\(?:e\\(?:t\\(?:ly?\\)?\\)?\\)?\\)\\|\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\)[ /t]+\\(?:n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\)\\)\\(?:[ \11]*:\\)?\\)?\\)[ \11]*") (ado-start-cmd-no-prefix-regexp "^\\(?:[ \11]*\\(?:\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\|mata\\|n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\|python\\|qui\\(?:e\\(?:t\\(?:ly?\\)?\\)?\\)?\\)\\|\\(?:cap\\(?:t\\(?:u\\(?:re?\\)?\\)?\\)?\\)[ /t]+\\(?:n\\(?:o\\(?:i\\(?:s\\(?:i\\(?:ly?\\)?\\)?\\)?\\)?\\)?\\)\\)\\(?:[ \11]*:\\)?\\)?[ \11]*") (ado-start-cmd-must-start-line-regexp "^[ \11]*") (ado-start-cmd-null-regexp "") (ado-end-cmd-regexp "\\([ \11]+\\|,\\|;\\|:\\|$\\)") (ado-stata-name-wipe-bad-chars "[^[:space:][:cntrl:][-^`!-/:-@{-~]") (ado-stata-name-regexp "[[:alpha:]_][^[:space:][:cntrl:][-^`!-/:-@{-~]*") (ado-stata-name-bound-regexp "\\([[:alpha:]_][^[:space:][:cntrl:][-^`!-/:-@{-~]*\\)") (ado-stata-varname-regexp "[[:alpha:]_*][^[:space:][:cntrl:][-^`!-/:-@{-~]*") (ado-stata-local-name-regexp "\\(?:`\\|[^[:space:][:cntrl:][-^`!-/:-@{-~]\\)[^[:space:][:cntrl:][-^`!-/:-@{-~]*") (ado-stata-local-name-bound-regexp "\\(\\(?:`\\|[^[:space:][:cntrl:][-^`!-/:-@{-~]\\)[^[:space:][:cntrl:][-^`!-/:-@{-~]*\\)") (ado-man-abbrevs ("ADAPT" "BAYES" "BMA" "CAUSAL" "CM" "D" "DSGE" "ERM" "FMM" "FN" "G" "GSM" "GSU" "GSW" "IG" "IRT" "LASSO" "M" "ME" "META" "MI" "MV" "P" "PSS" "R" "RPT" "SEM" "SP" "ST" "SVY" "TABLES" "TS" "U" "XT"))) ((92 92) (36 46) (96 46) (39 46) (47 46) (42 46) (43 95) (45 95) (61 95) (37 46) (60 46) (62 46) (38 46) (124 46) (126 46) (95 119)) (("TAB" ado-indent-line) ("M-RET" nil) ("C-M-RET" nil) ("C-c C-b" ado-grab-block) ("C-c M-b" ado-send-block-to-stata) ("C-c C-e" ado-foreach-loop) ("C-c C-v" ado-forvalues-loop) ("M-a" ado-beginning-of-command) ("M-e" ado-end-of-command) ("{" ado-electric-brace) ("}" ado-electric-closing-brace) (";" ado-electric-semi)) t t)"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_ordinary_variable_defaults_and_documentation_match() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list symbol
                 (boundp symbol)
                 (default-value symbol)
                 (documentation-property
                  symbol 'variable-documentation t)
                 (get symbol 'permanent-local)
                 (custom-variable-p symbol)))
         '(ado-font-lock-syntactic-keywords ado-added-names))"##;
    let expect = expect![
        "OK ((ado-font-lock-syntactic-keywords t nil nil nil nil) (ado-added-names t nil nil nil nil))"
    ];
    assert_ado_mode_parity(elisp_form, expect);
}

#[test]
fn ado_mode_generated_autoload_surface_matches() {
    let elisp_form = r##"(let (symbols)
         (mapatoms
          (lambda (symbol)
            (when (string-prefix-p "ado-" (symbol-name symbol))
              (when (or (boundp symbol) (fboundp symbol))
                (push symbol symbols)))))
         (list
          (featurep 'ado-mode-autoloads)
          (featurep 'ado-mode)
          (mapcar
           (lambda (symbol)
             (list symbol
                   (boundp symbol)
                   (and (boundp symbol) (custom-variable-p symbol))
                   (fboundp symbol)
                   (and (fboundp symbol)
                        (autoloadp (symbol-function symbol)))
                   (commandp symbol)))
           (sort symbols
                 (lambda (left right)
                   (string-lessp (symbol-name left)
                                 (symbol-name right)))))))"##;
    let expect = expect!["OK (t nil nil)"];
    assert_ado_mode_autoload_parity(elisp_form, expect);
}

#[test]
fn ado_mode_installed_inventory_and_literal_source_hashes_match() {
    let elisp_form = r##"(let* ((descriptor (cadr (assq 'ado-mode package-alist)))
                 (directory (package-desc-dir descriptor)))
         (mapcar
          (lambda (path)
            (let ((file (file-relative-name path directory)))
              (if (string-suffix-p ".elc" file)
                  (list file 'generated-bytecode)
                (with-temp-buffer
                  (set-buffer-multibyte nil)
                  (insert-file-contents-literally path)
                  (list file
                        (buffer-size)
                        (secure-hash 'sha256 (current-buffer)))))))
          (sort
           (seq-filter
            #'file-regular-p
            (directory-files-recursively directory "." nil nil))
           #'string-lessp)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" 179 "74de667206a3f3ffd024bda061cb90dd7ba60554d4fba2a0049d224163bc0592") ("ado-clip.el" 8748 "757cbd2ac5f939862cd1c988aa3373040112caf7cf2238baa5cdc7e036b63faf") ("ado-clip.elc" generated-bytecode) ("ado-cons.el" 4728 "c6a379fc02eecc074dad7b8bd79428a6247c1704bce85be6b9b243151652fba9") ("ado-cons.elc" generated-bytecode) ("ado-cus.el" 19398 "3647d039a947f5030fa4775dacd86cb9f07ef0c3f19cceb4f0b6fc6128b72a94") ("ado-cus.elc" generated-bytecode) ("ado-font-lock.el" 267789 "78377c3e9fb878c5180d308dbf08c152ad32fa319dd795efd16c9dc6231042bf") ("ado-font-lock.elc" generated-bytecode) ("ado-font.el" 7633 "38624bca71ed3a6f79a78c18414529ae78ff7baa416a145ae3e1490c25e82264") ("ado-font.elc" generated-bytecode) ("ado-mode-autoloads.el" 1385 "fd1da8762c40c10dfc667ccd3f14d772b1c86f6185db33dda2594d3f109b36d2") ("ado-mode-pkg.el" 481 "b87554601f14adfe5182c06e3c6a5f418925ab0f0e0c2924e9c637233a36b168") ("ado-mode.el" 79252 "195b80ae4f03d2248f89908ad71f3883482098775e26fe8942c92c2fe6619c5d") ("ado-mode.elc" generated-bytecode) ("ado-stata-info.el" 11009 "ef7f9bf6bf79f0e58832f3ed222fbeba6111002d8b800f38cfe5edebfe487d78") ("ado-stata-info.elc" generated-bytecode) ("ado-to-stata.el" 10708 "948625f2973284957b67cd18cba74c4cb9a0d4eb6ec510e90f76be9e6e175e19") ("ado-to-stata.elc" generated-bytecode) ("scripts/send2stata.au3" 9215 "e07d5ff8682141ddf926b150e266c004616d4c54f82ab2b8e29e2515719280aa") ("scripts/send2stata.exe" 897024 "4b07f71429a826851033e5604ff0fac65a49faf898f406d7db928c774d63270c") ("scripts/send2stata.scpt" 50248 "cf080ab10f807e896bc388ec20d3e1d3c71f2efdac53d38bffb7e1d87ceef1ca") ("scripts/send2ztata.sh" 3179 "6f634185b97f637e20fea370e4a832efa3c25213998ed3570b0fa2699b84cf82") ("templates/ado.blp" 80 "cf5de62556c73802daf15282bbf5aff6d09678796b6a29de50972749a5d233b6") ("templates/class.blp" 73 "a3de8cf684d558b427e66ff59f8e546c7db8cc10deb7a85e315075d73adc54b9") ("templates/dialog.blp" 1007 "a210df5bd8d3b70516942759af264221f0f961f8d0f830554d13bd0f4d2afa97") ("templates/do.blp" 173 "af2f9b3a753932bc4ae474a3cee90c96d446aac56001a503e2541d222b6e899e") ("templates/help.blp" 3691 "1d1e702ddf97a2d63b775fde4252e43f1e4ab2d3c9a9fd1cb79efd9513aec224") ("templates/help10.blp" 1033 "21afd1c2b52510590e3ae62e260d50e17a2077793c5de4d16092d8e1347534a8") ("templates/help11.blp" 2199 "18b50b2d35702b2667105f9af17cc158d494fcf77e9779547d1f9dc8153295f2") ("templates/help6.blp" 409 "7c78c586a20d7bb5c954feabc625b9395fbc1aedd55e8651874907aa34ef52c8") ("templates/help7.blp" 596 "bf951c23921bd2b55bc99d7fb6cfb866cac074a8fbc815001cf740d3426cea99") ("templates/lbl.blp" 25 "92327928e1fa4e6f66a080196ab23f04a5547826ca4fad2c0dedf4f316d781a9") ("templates/mata.blp" 60 "fefbdba196f04e3d4a90917b009c08c991fdd20dd2a468634fe5d5b29af8a2fa") ("templates/pkg.blp" 808 "4ec39f8a69869aa043de7637d85b53f3ce9cb7c708170ae87db260fc988b69bb") ("templates/readme" 911 "7bf43f88c6339cbb69193f18befa8cdfabc3c6e88cba9565fd1733712d91ad41") ("templates/smallado.blp" 26 "2f7168e45b028d8490684cba7051cf849791e19262e4666a6206535f2b42b890") ("templates/sparse.blp" 788 "4a0924a1ec158bc8f2752c9183db0847add6e890e2d5ebf68f2160127d0a4965") ("templates/testado.blp" 280 "57d25e5ee9342c46508c1d4b5d35f618c5ef76e1bb08e9ce7da7cb4e718f66a9") ("templates/toc.blp" 370 "e957d51720ed51731184668d74a440afd87a59f4fe6d6b7d76930f4f933c6f80"))"#
    ]];
    assert_ado_mode_parity(elisp_form, expect);
}
