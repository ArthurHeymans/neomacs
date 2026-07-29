use expect_test::expect;

use super::assert_alan_mode_parity;

#[test]
fn alan_flycheck_filter_keeps_only_current_real_file_and_preserves_diagnostics() {
    let elisp_form = r##"(with-temp-buffer
                      (setq buffer-file-name "/workspace/models/main.alan")
                      (let* ((errors
                              (list
                               (flycheck-error-new-at
                                3 7 'error "broken account"
                                :checker 'alan
                                :filename "/workspace/models/main.alan")
                               (flycheck-error-new-at
                                5 2 'warning "other model"
                                :checker 'alan
                                :filename "/workspace/models/other.alan")
                               (flycheck-error-new-at
                                1 1 'error "compiler context"
                                :checker 'alan
                                :filename "/dev/null")))
                             (filtered
                              (alan-flycheck-error-filter errors)))
                        (mapcar
                         (lambda (error)
                           (list
                            (flycheck-error-line error)
                            (flycheck-error-column error)
                            (flycheck-error-level error)
                            (flycheck-error-message error)
                            (flycheck-error-filename error)
                            (flycheck-error-checker error)))
                         filtered)))"##;
    let expect =
        expect![[r#"OK ((3 7 error "broken account" "/workspace/models/main.alan" alan))"#]];
    assert_alan_mode_parity(elisp_form, expect);
}

#[test]
fn alan_flycheck_checker_metadata_covers_every_generated_mode_and_command_branch() {
    let elisp_form = r##"(let ((modes
                           '(alan-mode alan-schema-mode alan-grammar-mode
                             alan-template-mode alan-application-mode
                             alan-widget-mode alan-views-mode alan-wiring-mode
                             alan-deployment-mode alan-mapping-mode
                             alan-migration-mode alan-settings-mode
                             alan-control-mode alan-interface-mode
                             alan-translations-mode alan-phrases-mode)))
                      (list
                       (flycheck-checker-get 'alan 'command)
                       (flycheck-checker-get 'alan 'error-patterns)
                       (flycheck-checker-get 'alan 'error-filter)
                       (flycheck-checker-get 'alan 'modes)
                       (mapcar
                        (lambda (mode)
                          (cons
                           mode
                           (and
                            (memq mode
                                  (flycheck-checker-get 'alan 'modes))
                            t)))
                        modes)))"##;
    let expect = expect![[
        r#"OK (("alan" (eval (if (null alan--flycheck-language-definition) '("build") `(,alan--flycheck-language-definition "-C" ,alan-compiler-project-root "/dev/null")))) (("^\\(?1:.+?\\):\\(?2:[[:digit:]]+\\):\\(?3:[[:digit:]]+\\): error:\\(?: [[:digit:]]+:[[:digit:]]+\\)?\\(?4:.*\\(?:\n .*\\)*\\)$" . error) ("^\\(?1:.+?\\):\\(?2:[[:digit:]]+\\):\\(?3:[[:digit:]]+\\): warning:\\(?: [[:digit:]]+:[[:digit:]]+\\)?\\(?4:.*\\(?:\n .*\\)*\\)$" . warning)) alan-flycheck-error-filter (alan-phrases-mode alan-translations-mode alan-interface-mode alan-control-mode alan-settings-mode alan-migration-mode alan-mapping-mode alan-deployment-mode alan-wiring-mode alan-views-mode alan-widget-mode alan-application-mode alan-template-mode alan-grammar-mode alan-schema-mode alan-mode) ((alan-mode . t) (alan-schema-mode . t) (alan-grammar-mode . t) (alan-template-mode . t) (alan-application-mode . t) (alan-widget-mode . t) (alan-views-mode . t) (alan-wiring-mode . t) (alan-deployment-mode . t) (alan-mapping-mode . t) (alan-migration-mode . t) (alan-settings-mode . t) (alan-control-mode . t) (alan-interface-mode . t) (alan-translations-mode . t) (alan-phrases-mode . t)))"#
    ]];
    assert_alan_mode_parity(elisp_form, expect);
}

#[test]
fn alan_flycheck_command_evaluation_switches_between_script_and_language_compiler_usage() {
    let elisp_form = r##"(with-temp-buffer
                      (setq buffer-file-name "/workspace/main.alan"
                            flycheck-alan-executable "/tools/alan"
                            alan-compiler-project-root "../project")
                      (let ((alan--flycheck-language-definition nil))
                        (list
                         (flycheck-checker-substituted-arguments
                          'alan)
                         (let ((alan--flycheck-language-definition
                                "/languages/schema"))
                           (flycheck-checker-substituted-arguments
                            'alan)))))"##;
    let expect = expect![[r#"OK (("build") ("/languages/schema" "-C" "../project" "/dev/null"))"#]];
    assert_alan_mode_parity(elisp_form, expect);
}
