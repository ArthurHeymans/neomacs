use expect_test::expect;

use super::{assert_alt_codes_autoload_parity, assert_alt_codes_parity};

#[test]
fn alt_codes_registry_exposes_complete_callable_surface_and_group() {
    let elisp_form = r##"(list
         (featurep 'alt-codes)
         (mapcar
          (lambda (symbol)
            (list symbol
                  (fboundp symbol)
                  (help-function-arglist symbol t)
                  (commandp symbol)))
          '(alt-codes--pre-command-hook
            alt-codes--get-symbol
            alt-codes-insert
            alt-codes--enable
            alt-codes--disable
            alt-codes-mode
            alt-codes-turn-on-alt-codes-mode
            global-alt-codes-mode))
         (get 'alt-codes 'group-documentation)
         (get 'alt-codes 'custom-prefix)
         (get 'alt-codes 'custom-group)
         (get 'alt-codes 'custom-links))"##;
    let expect = expect![[
        r#"OK (t ((alt-codes--pre-command-hook t nil nil) (alt-codes--get-symbol t (code) nil) (alt-codes-insert t nil t) (alt-codes--enable t nil nil) (alt-codes--disable t nil nil) (alt-codes-mode t (&optional arg) t) (alt-codes-turn-on-alt-codes-mode t nil nil) (global-alt-codes-mode t (&optional arg) t)) "Insert alt codes using meta key." "alt-codes-" ((global-alt-codes-mode custom-variable)) ((url-link :tag "Repository" "https://github.com/jcs-elpa/alt-codes")))"#
    ]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_state_is_automatically_buffer_local_with_empty_default() {
    let elisp_form = r##"(let ((first (generate-new-buffer " *alt-first*"))
               (second (generate-new-buffer " *alt-second*")))
         (unwind-protect
             (progn
               (with-current-buffer first
                 (setq alt-codes--code "123"))
               (list
                (default-value 'alt-codes--code)
                (local-variable-if-set-p
                 'alt-codes--code)
                (with-current-buffer first
                  (list alt-codes--code
                        (local-variable-p
                         'alt-codes--code)))
                (with-current-buffer second
                  (list alt-codes--code
                        (local-variable-p
                         'alt-codes--code)))))
           (kill-buffer first)
           (kill-buffer second)))"##;
    let expect = expect![[r#"OK ("" t ("123" t) ("" nil))"#]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_minor_mode_metadata_has_exact_lighter_and_global_contract() {
    let elisp_form = r##"(list
         (assq 'alt-codes-mode minor-mode-alist)
         (assq 'alt-codes-mode minor-mode-map-alist)
         (get 'alt-codes-mode 'custom-type)
         (get 'alt-codes-mode 'custom-group)
         (get 'global-alt-codes-mode 'custom-type)
         (get 'global-alt-codes-mode 'custom-group)
         (get 'global-alt-codes-mode 'globalized-minor-mode)
         (get 'global-alt-codes-mode
              'function-documentation))"##;
    let expect = expect![[r#"OK ((alt-codes-mode " alt-codes") nil nil nil boolean nil t nil)"#]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_package_descriptor_records_exact_pin_and_emacs_requirement() {
    let elisp_form = r##"(let ((description
                        (cadr
                         (assq 'alt-codes package-alist))))
         (list
          (package-version-join
           (package-desc-version description))
          (package-desc-reqs description)
          (package-desc-summary description)
          (file-name-nondirectory
           (directory-file-name
            (package-desc-dir description)))))"##;
    let expect = expect![[
        r#"OK ("20260101.557" ((emacs (26 1))) "Insert alt codes using meta key." "alt-codes-20260101.557")"#
    ]];
    assert_alt_codes_parity(elisp_form, expect);
}

#[test]
fn alt_codes_autoloads_publish_interactive_commands_without_loading_runtime() {
    let elisp_form = r##"(list
         (featurep 'alt-codes)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp symbol)
             (autoloadp
              (and (fboundp symbol)
                   (symbol-function symbol)))
             (commandp symbol)
             (help-function-arglist symbol t)))
          '(alt-codes-insert
            alt-codes-mode
            global-alt-codes-mode))
         (file-name-nondirectory
          (getenv "NEOMACS_PACKAGE_SOURCE")))"##;
    let expect = expect![[
        r#"OK (nil ((alt-codes-insert t t t "[Arg list not available until function definition is loaded.]") (alt-codes-mode t t t "[Arg list not available until function definition is loaded.]") (global-alt-codes-mode t t t "[Arg list not available until function definition is loaded.]")) "alt-codes-autoloads.el")"#
    ]];
    assert_alt_codes_autoload_parity(elisp_form, expect);
}
