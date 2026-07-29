use expect_test::expect;

use super::assert_alan_mode_parity;

#[test]
fn alan_mode_exact_pin_feature_defaults_environment_and_checker_registration_match() {
    let elisp_form = r##"(list
                      (featurep 'alan-mode)
                      (list alan-xref-limit-to-project-scope
                            alan-compiler
                            alan-log-level
                            alan-compiler-project-root
                            alan-script
                            alan-language-definition
                            alan-lsp-capture-path)
                      (list (getenv "ALAN_COMPILER_FORMAT")
                            (getenv "ALAN_COMPILER_LOG"))
                      (and (memq 'alan flycheck-checkers) t)
                      (flycheck-checker-get 'alan 'modes)
                      (flycheck-checker-get 'alan 'error-filter)
                      (seq-filter
                       (lambda (entry)
                         (and
                          (consp entry)
                          (symbolp (cdr entry))
                          (string-prefix-p
                           "alan-" (symbol-name (cdr entry)))))
                       auto-mode-alist)
                      (cdr (assoc "\\.alan\\'" auto-mode-alist)))"##;
    let expect = expect![[
        r#"OK (t (t "compiler-project" "warning" "." "alan" nil nil) ("emacs" "warning") t (alan-phrases-mode alan-translations-mode alan-interface-mode alan-control-mode alan-settings-mode alan-migration-mode alan-mapping-mode alan-deployment-mode alan-wiring-mode alan-views-mode alan-widget-mode alan-application-mode alan-template-mode alan-grammar-mode alan-schema-mode alan-mode) alan-flycheck-error-filter (("phrases\\.alan\\'" . alan-phrases-mode) ("translations/.*\\.alan\\'" . alan-translations-mode) ("interface\\.alan\\'" . alan-interface-mode) ("control\\.alan\\'" . alan-control-mode) ("settings\\.alan\\'" . alan-settings-mode) ("migration\\.alan\\'" . alan-migration-mode) ("mapping\\.alan\\'" . alan-mapping-mode) ("deployment\\.alan\\'" . alan-deployment-mode) ("wiring\\.alan\\'" . alan-wiring-mode) ("views/.*\\.alan\\'" . alan-views-mode) ("widgets/.*\\.alan\\'" . alan-widget-mode) ("application\\.alan\\'" . alan-application-mode) ("templates/.*\\.alan\\'" . alan-template-mode) ("grammar\\.alan\\'" . alan-grammar-mode) ("schema\\.alan\\'" . alan-schema-mode) ("\\.alan\\'" . alan-mode)) alan-mode)"#
    ]];
    assert_alan_mode_parity(elisp_form, expect);
}

#[test]
fn alan_mode_complete_package_callable_surface_arglists_macros_and_commands_match() {
    let elisp_form = r##"(let ((source (file-truename
                                      (getenv "NEOMACS_PACKAGE_SOURCE")))
                          rows)
                      (mapatoms
                       (lambda (symbol)
                         (when (and
                                (string-prefix-p "alan" (symbol-name symbol))
                                (fboundp symbol)
                                (when-let ((file (symbol-file symbol 'defun)))
                                  (string=
                                   source
                                   (file-truename file))))
                           (push
                            (list symbol
                                  (condition-case nil
                                      (copy-tree
                                       (help-function-arglist symbol t))
                                    (error :unavailable))
                                  (macrop symbol)
                                  (commandp symbol))
                            rows))))
                      (sort rows
                            (lambda (left right)
                              (string-lessp
                               (symbol-name (car left))
                               (symbol-name (car right))))))"##;
    let expect = expect![
        "OK ((alan--documentation-p nil nil nil) (alan--file-exists (name) nil nil) (alan--file-path-to-relative-project-path (file) nil nil) (alan--has-parent nil nil nil) (alan--projectile-project-root nil nil nil) (alan--save-hs-overlays (&rest body) t nil) (alan--single-block (line) nil nil) (alan--xref-backend nil nil nil) (alan--xref-find-definitions (symbol) nil nil) (alan--xref-make-xref (symbol type buffer symbol-position path) nil nil) (alan-add-to-phrases nil nil t) (alan-application-mode nil nil t) (alan-application-mode--setup-tree-sitter nil nil nil) (alan-boundry-of-identifier-at-point nil nil nil) (alan-control-mode nil nil t) (alan-copy-path-to-clipboard nil nil t) (alan-define-mode (name &optional docstring &rest body) t nil) (alan-deployment-mode nil nil t) (alan-documentation-abort nil nil t) (alan-documentation-exit nil nil t) (alan-documentation-follow-include-link-at-point nil nil t) (alan-documentation-include-link-p nil nil nil) (alan-documentation-mode (&optional arg) nil nil) (alan-documentation-sync-buffer nil nil t) (alan-edit-documentation nil nil t) (alan-eglot--server-command (&optional is-interactive) nil nil) (alan-file-executable (file) nil nil) (alan-find-alan-script nil nil nil) (alan-flycheck-error-filter (error-list) nil nil) (alan-font-lock-syntactic-face-function (state) nil nil) (alan-goto-parent nil nil t) (alan-grammar-mode nil nil t) (alan-grammar-update-keyword nil nil t) (alan-guess-type nil nil nil) (alan-identifier-at-point nil nil nil) (alan-interface-mode nil nil t) (alan-list-nummerical-types nil nil nil) (alan-lsp--find-command nil nil nil) (alan-lsp--server-command nil nil nil) (alan-lsp-activate-alan-mode (_file-name _mode) nil nil) (alan-mapping-mode nil nil t) (alan-mark-documentation nil nil t) (alan-migration-mode nil nil t) (alan-mode nil nil t) (alan-mode--setup-font-lock nil nil nil) (alan-mode--treesit-feature-list (rules) nil nil) (alan-mode--treesit-setup (language &optional extra-rules folds) nil nil) (alan-mode-indent-line nil nil t) (alan-path nil nil nil) (alan-phrases-mode nil nil t) (alan-project-root nil nil nil) (alan-remove-from-phrases nil nil t) (alan-schema-mode nil nil t) (alan-schema-mode--setup-tree-sitter nil nil nil) (alan-settings-mode nil nil t) (alan-setup-build-system nil nil nil) (alan-setup-eglot nil nil nil) (alan-setup-lsp nil nil nil) (alan-template-mode nil nil t) (alan-template-yank nil nil t) (alan-translations-mode nil nil t) (alan-views-mode nil nil t) (alan-widget-mode nil nil t) (alan-wiring-mode nil nil t))"
    ];
    assert_alan_mode_parity(elisp_form, expect);
}

#[test]
fn alan_derived_modes_activate_real_language_build_pair_and_pretty_print_contracts() {
    let elisp_form = r##"(let ((modes
                           '(alan-mode alan-schema-mode alan-grammar-mode
                             alan-template-mode alan-application-mode
                             alan-widget-mode alan-views-mode alan-wiring-mode
                             alan-deployment-mode alan-mapping-mode
                             alan-migration-mode alan-settings-mode
                             alan-control-mode alan-interface-mode
                             alan-translations-mode alan-phrases-mode))
                          rows)
                      (dolist (mode modes)
                        (with-temp-buffer
                          (cl-letf (((symbol-function
                                     'alan-setup-build-system)
                                    (lambda () nil)))
                            (funcall mode))
                          (push
                           (list mode
                                 major-mode
                                 mode-name
                                 (derived-mode-p 'alan-mode)
                                 alan-language-definition
                                 alan-compiler-project-root
                                 alan-pretty-print
                                 (eq indent-line-function
                                     'alan-mode-indent-line)
                                 comment-start
                                 (key-binding "\C-c'"))
                           rows)))
                      (nreverse rows))"##;
    let expect = expect![[
        r#"OK ((alan-mode alan-mode "Alan" alan-mode nil "." nil t "//" alan-edit-documentation) (alan-schema-mode alan-schema-mode "schema" alan-mode "dependencies/dev/internals/alan/language" "../.." t t "//" alan-edit-documentation) (alan-grammar-mode alan-grammar-mode "grammar" alan-mode "dependencies/dev/internals/alan/language" "../.." t t "//" alan-edit-documentation) (alan-template-mode alan-template-mode "template" alan-mode "dependencies/dev/internals/alan-to-text-transformation/language" "../../../" t t "//" alan-edit-documentation) (alan-application-mode alan-application-mode "application" alan-mode ".alan/devenv/platform/if-types/model/language" "." t t "//" alan-edit-documentation) (alan-widget-mode alan-widget-mode "widget" alan-mode ".alan/devenv/system-types/webclient/language" "../" t t "//" alan-edit-documentation) (alan-views-mode alan-views-mode "views" alan-mode ".alan/devenv/system-types/webclient/language" "../" t t "//" alan-edit-documentation) (alan-wiring-mode alan-wiring-mode "wiring" alan-mode nil "." nil t "//" alan-edit-documentation) (alan-deployment-mode alan-deployment-mode "deployment" alan-mode ".alan/devenv/platform/project-build-environment/language" "." nil t "//" alan-edit-documentation) (alan-mapping-mode alan-mapping-mode "mapping" alan-mode nil "." nil t "//" alan-edit-documentation) (alan-migration-mode alan-migration-mode "migration" alan-mode nil "." t t "//" alan-edit-documentation) (alan-settings-mode alan-settings-mode "settings" alan-mode ".alan/devenv/system-types/auto-webclient/language" "." t t "//" alan-edit-documentation) (alan-control-mode alan-control-mode "control" alan-mode nil "." t t "//" alan-edit-documentation) (alan-interface-mode alan-interface-mode "interface" alan-mode nil "." t t "//" alan-edit-documentation) (alan-translations-mode alan-translations-mode "translations" alan-mode nil "." nil t "//" alan-edit-documentation) (alan-phrases-mode alan-phrases-mode "phrases" alan-mode nil "." nil t "//" alan-edit-documentation))"#
    ]];
    assert_alan_mode_parity(elisp_form, expect);
}

#[test]
fn alan_mode_tree_sitter_feature_buckets_preserve_rule_order_and_empty_levels() {
    let elisp_form = r##"(list
                      (alan-mode--treesit-feature-list
                       '(:language alan :feature comment (a)
                         :language alan :feature string (b)
                         :language alan :feature identifier (c)
                         :language alan :feature number (d)
                         :language alan :feature keyword (e)
                         :language alan :feature comment (f)))
                      (alan-mode--treesit-feature-list
                       '(:feature identifier (a) :feature string (b)))
                      (alan-mode--treesit-feature-list nil)
                      alan--treesit-fold-rules)"##;
    let expect = expect![
        "OK (((comment comment) (string identifier) (number keyword)) (nil (identifier string)) (nil nil) ((brace_block . treesit-fold-range-seq) (paren_block . treesit-fold-range-seq) (alan__comment . treesit-fold-range-c-like-comment)))"
    ];
    assert_alan_mode_parity(elisp_form, expect);
}
