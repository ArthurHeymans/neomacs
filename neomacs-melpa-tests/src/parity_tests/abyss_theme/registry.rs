use expect_test::expect;

use super::assert_abyss_theme_parity;

#[test]
fn abyss_theme_exact_pin_registers_theme_feature_documentation_and_command_surface() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'abyss-theme package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (package-desc-kind descriptor)
                (package-desc-archive descriptor)
                (featurep 'abyss-theme)
                (and (custom-theme-p 'abyss) t)
                (custom-theme-name-valid-p 'abyss)
                (custom-theme-enabled-p 'abyss)
                (get 'abyss 'theme-feature)
                (get 'abyss 'theme-documentation)
                (fboundp 'abyss-theme)
                (commandp 'abyss-theme)
                (interactive-form 'abyss-theme)
                (documentation 'abyss-theme)))"##;
    let expect = expect![[
        r#"OK (abyss-theme "20260125.1959" ((emacs (24))) nil nil t t t nil abyss-theme "Dark background and contrasting colours." t t (interactive nil) "Load abyss-theme.")"#
    ]];

    assert_abyss_theme_parity(elisp_form, expect);
}

#[test]
fn abyss_theme_setting_inventory_has_exact_source_order_shape_theme_and_uniqueness() {
    let elisp_form = r##"(let* ((settings
                     (get 'abyss 'theme-settings))
                    (faces
                     (mapcar #'cadr settings)))
               (list
                (length settings)
                faces
                (mapcar #'car settings)
                (mapcar #'caddr settings)
                (length
                 (delete-dups
                  (copy-sequence faces)))))"##;
    let expect = expect![
        "OK (52 (magit-item-highlight whitespace-line whitespace-tab toolbar left-margin italic region text-cursor compilation-mode-line-run compilation-mode-line-fail compilation-mode-line-exit compilation-info compilation-warning compilation-error flycheck-fringe-info flycheck-fringe-warning flycheck-fringe-error flycheck-info flycheck-warning flycheck-error envrc-mode-line-none-face envrc-mode-line-error-face envrc-mode-line-on-face error warning success mode-line-inactive mode-line-buffer-id mode-line-emphasis mode-line-highlight mode-line gui-element font-lock-warning-face font-lock-negation-char-face font-lock-variable-name-face font-lock-type-face font-lock-preprocessor-face font-lock-keyword-face font-lock-function-name-face font-lock-string-face font-lock-doc-string-face font-lock-doc-face font-lock-constant-face font-lock-comment-face font-lock-comment-delimiter-face font-lock-builtin-face buffers-tab fringe default border-glyph bold-italic bold) (theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face theme-face) (abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss abyss) 52)"
    ];

    assert_abyss_theme_parity(elisp_form, expect);
}

#[test]
fn abyss_theme_source_reloads_accumulate_settings_but_register_one_load_path_entry() {
    let elisp_form = r##"(let* ((source
                     (getenv "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory source)))
                    (custom-theme-load-path
                     '(sentinel))
                    observations)
               (dolist (_ '(first second))
                 (load source nil t t)
                 (push
                  (list
                   (equal
                    (car custom-theme-load-path)
                    directory)
                   (length custom-theme-load-path)
                   (length
                    (get 'abyss 'theme-settings)))
                  observations))
               (list
                (nreverse observations)
                (let ((count 0))
                  (dolist
                      (entry custom-theme-load-path count)
                    (when
                        (equal entry directory)
                      (setq count (1+ count)))))))"##;
    let expect = expect!["OK (((t 2 104) (t 2 156)) 1)"];

    assert_abyss_theme_parity(elisp_form, expect);
}

#[test]
fn abyss_theme_command_forwards_the_exact_noninteractive_theme_load_contract() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function 'load-theme)
                     (lambda (&rest arguments)
                       (push arguments calls)
                       'loaded)))
                 (list
                  (abyss-theme)
                  (nreverse calls))))"##;
    let expect = expect!["OK (loaded ((abyss t)))"];

    assert_abyss_theme_parity(elisp_form, expect);
}
