use expect_test::expect;

use super::{assert_use_package_parity, assert_use_package_signal_parity};

#[test]
fn use_package_public_version_keywords_and_defaults_match_the_pinned_release() {
    let elisp_form = r##"(list
               use-package-version
               use-package-always-defer
               use-package-always-demand
               use-package-hook-name-suffix
               (and (memq :preface use-package-keywords) t)
               (and (memq :config use-package-keywords) t)
               (and (memq :vc use-package-keywords) t)
               (length use-package-keywords))"##;
    let expect = expect![[r#"OK ("2.4.5" nil nil "-hook" t t t 35)"#]];

    assert_use_package_parity(elisp_form, expect);
}

#[test]
fn use_package_preface_init_and_config_execute_in_declaration_order() {
    let elisp_form = r##"(progn
               (defvar neomacs-use-package-events nil)
               (setq neomacs-use-package-events nil)
               (use-package neomacs-use-package-order
                 :no-require t
                 :preface
                 (push 'preface neomacs-use-package-events)
                 :init
                 (push 'init neomacs-use-package-events)
                 :config
                 (push 'config neomacs-use-package-events))
               (nreverse neomacs-use-package-events))"##;
    let expect = expect![[r#"OK (preface init config)"#]];

    assert_use_package_parity(elisp_form, expect);
}

#[test]
fn use_package_disabled_if_unless_and_requires_gate_all_runtime_forms() {
    let elisp_form = r##"(let (events)
               (provide 'neomacs-use-package-required)
               (use-package neomacs-use-package-disabled
                 :disabled t
                 :no-require t
                 :init (push 'disabled events))
               (use-package neomacs-use-package-if
                 :if (= 2 (+ 1 1))
                 :no-require t
                 :init (push 'if events))
               (use-package neomacs-use-package-unless
                 :unless t
                 :no-require t
                 :init (push 'unless events))
               (use-package neomacs-use-package-requires
                 :requires neomacs-use-package-required
                 :no-require t
                 :init (push 'requires events))
               (use-package neomacs-use-package-missing
                 :requires neomacs-use-package-absent
                 :no-require t
                 :init (push 'missing events))
               (nreverse events))"##;
    let expect = expect![[r#"OK (if requires)"#]];

    assert_use_package_parity(elisp_form, expect);
}

#[test]
fn use_package_catch_converts_a_missing_required_library_into_an_exact_warning() {
    let elisp_form = r##"(let (warnings)
               (cl-letf (((symbol-function 'display-warning)
                          (lambda (type message &optional level _buffer)
                            (push (list type message level) warnings))))
                 (let ((result
                        (use-package
                            neomacs-use-package-definitely-missing
                          :catch t)))
                   (list result (nreverse warnings)))))"##;
    let expect = expect![[
        r#"OK (#1=((use-package "Cannot load neomacs-use-package-definitely-missing" :error)) #1#)"#
    ]];

    assert_use_package_parity(elisp_form, expect);
}

#[test]
fn use_package_function_normalization_matches_upstream_function_forms() {
    let elisp_form = r##"(list
               (mapcar
                #'use-package-normalize-function
                '(nil t symbol
                  (function symbol)
                  (lambda () value)
                  (quote (lambda () quoted))
                  1 "text" (nil)))
               (mapcar
                (lambda (value)
                  (use-package-recognize-function value t))
                '(nil t symbol
                  (lambda () value)
                  1 "text" (nil))))"##;
    let expect = expect![[
        r#"OK ((nil t symbol symbol (lambda nil value) (lambda nil quoted) 1 "text" (nil)) (t t t nil nil t nil))"#
    ]];

    assert_use_package_parity(elisp_form, expect);
}

#[test]
fn use_package_hook_normalization_handles_default_functions_groups_and_pairs() {
    let elisp_form = r##"(mapcar
               (lambda (args)
                 (use-package-normalize/:hook
                  'neomacs-package :hook args))
               '((mode)
                 ((mode . explicit-function))
                 (mode-a mode-b)
                 (((mode-a mode-b) . shared-function))
                 ((mode-a . one) (mode-b . two))))"##;
    let expect = expect![[
        r#"OK (((mode . neomacs-package-mode)) ((mode . explicit-function)) (((mode-a mode-b) . neomacs-package-mode)) (((mode-a mode-b) . shared-function)) ((mode-a . one) (mode-b . two)))"#
    ]];

    assert_use_package_parity(elisp_form, expect);
}

#[test]
fn use_package_empty_hook_spec_signals_the_exact_normalization_error() {
    let elisp_form = r##"(use-package-normalize/:hook
               'neomacs-use-package-invalid :hook nil)"##;
    let expect = expect![[r#"ERR (error "use-package: :hook wants a non-empty list")"#]];

    assert_use_package_signal_parity(elisp_form, expect);
}
