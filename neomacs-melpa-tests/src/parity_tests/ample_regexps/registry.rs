use expect_test::expect;

use super::{assert_ample_regexps_autoload_parity, assert_ample_regexps_parity};

#[test]
fn ample_regexps_loads_exact_dependency_free_package_metadata() {
    let elisp_form = r##"(let* ((description
        (cadr (assq 'ample-regexps package-alist)))
       (directory
        (file-name-as-directory (package-desc-dir description))))
  (list
   (featurep 'ample-regexps)
   (package-installed-p 'ample-regexps)
   (package-version-join (package-desc-version description))
   (package-desc-reqs description)
   (package-desc-kind description)
   (package-desc-archive description)
   (file-readable-p
    (expand-file-name "ample-regexps.el" directory))
   (file-readable-p
    (expand-file-name "init-tryout.el" directory))))"##;
    let expect = expect![[r#"OK (t t "20200508.1021" nil nil nil t t)"#]];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn ample_regexps_exposes_complete_callable_command_and_minor_mode_surface() {
    let elisp_form = r##"(mapcar
 (lambda (symbol)
   (list
    symbol
    (fboundp symbol)
    (macrop symbol)
    (commandp symbol)
    (help-function-arglist symbol t)))
 '(define-arx
   arx-and
   arx-or
   arx-builder
   arx-minor-mode
   arx-documentation-function
   arx--bound-interval
   arx--function-arity
   arx--apply-func-post-27
   arx--form-to-rx-binding
   arx--form-make-docstring
   arx--fnsym-in-current-sexp
   arx--name-and-depth
   arx--get-form-func
   arx--get-args-string
   arx--make-macro-bindings-docstring
   arx--make-macro-to-string-docstring
   arx--make-macro-docstring
   define-arx--fn-post-27
   define-arx--fn))"##;
    let expect = expect![
        "OK ((define-arx t t nil (macro form-defs)) (arx-and t nil nil (forms)) (arx-or t nil nil (forms)) (arx-builder t nil t (&optional arx-name)) (arx-minor-mode t nil t (&optional arg)) (arx-documentation-function t nil nil nil) (arx--bound-interval t nil nil (interval lower upper)) (arx--function-arity t nil nil (func)) (arx--apply-func-post-27 t nil nil (arity predicate func form-name args)) (arx--form-to-rx-binding t nil nil (arx-form)) (arx--form-make-docstring t nil nil (arx-form)) (arx--fnsym-in-current-sexp t nil nil nil) (arx--name-and-depth t nil nil nil) (arx--get-form-func t nil nil (arx-name sym)) (arx--get-args-string t nil nil (func sym index)) (arx--make-macro-bindings-docstring t nil nil (macro-name)) (arx--make-macro-to-string-docstring t nil nil (macro-name)) (arx--make-macro-docstring t nil nil (macro-name form-docstrings)) (define-arx--fn-post-27 t nil nil #1=(macro form-defs)) (define-arx--fn t nil nil #1#))"
    ];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn generated_autoloads_defer_library_loading_and_preserve_interactive_contracts() {
    let elisp_form = r##"(list
 (featurep 'ample-regexps)
 (mapcar
  (lambda (symbol)
    (let ((definition (symbol-function symbol)))
      (list
       symbol
       (autoloadp definition)
       (and (autoloadp definition) (nth 1 definition))
       (and (autoloadp definition) (nth 4 definition))
       (macrop symbol)
       (commandp symbol))))
  '(define-arx arx-and arx-or arx-builder))
 (boundp 'arx--new-rx)
 (boundp 'arx-minor-mode))"##;
    let expect = expect![[
        r#"OK (nil ((define-arx t "ample-regexps" t (t) nil) (arx-and t "ample-regexps" nil nil nil) (arx-or t "ample-regexps" nil nil nil) (arx-builder t "ample-regexps" nil nil t)) nil nil)"#
    ]];
    assert_ample_regexps_autoload_parity(elisp_form, expect);
}

#[test]
fn modern_rx_branch_selects_bindings_implementation_and_omits_legacy_helpers() {
    let elisp_form = r##"(list
 arx--new-rx
 (fboundp 'rx-form)
 (eq (indirect-function 'define-arx--fn)
     (indirect-function 'define-arx--fn-post-27))
 (mapcar
  (lambda (symbol)
    (list symbol (fboundp symbol)))
  '(define-arx--fn-post-27
    arx--apply-func-post-27
    arx--form-to-rx-binding
    define-arx--fn-pre-27
    arx--ensure-regexp
    arx--quoted-literal
    arx--apply-form-func
    arx--alias-rx-form
    arx--form-to-rx-constituent)))"##;
    let expect = expect![
        "OK (t nil t ((define-arx--fn-post-27 t) (arx--apply-func-post-27 t) (arx--form-to-rx-binding t) (define-arx--fn-pre-27 nil) (arx--ensure-regexp nil) (arx--quoted-literal nil) (arx--apply-form-func nil) (arx--alias-rx-form nil) (arx--form-to-rx-constituent nil)))"
    ];
    assert_ample_regexps_parity(elisp_form, expect);
}

#[test]
fn define_arx_registers_generated_macro_runtime_function_bindings_and_properties() {
    let elisp_form = r##"(progn
  (define-arx deployment-rx
    '((service (regexp "[[:alpha:]][[:alnum:]-]*"))
      (separator (or ":" "/"))
      (environment "prod")))
  (list
   (macrop 'deployment-rx)
   (fboundp 'deployment-rx-to-string)
   (boundp 'deployment-rx-bindings)
   deployment-rx-bindings
   (get 'deployment-rx 'arx-name)
   (get 'deployment-rx-to-string 'arx-name)
   (get 'deployment-rx 'arx-form-defs)
   (get 'deployment-rx-bindings 'variable-documentation)))"##;
    let expect = expect![[
        r#"OK (t t t ((service #1=(regexp "[[:alpha:]][[:alnum:]-]*")) (separator #2=(or ":" "/")) (environment "prod")) "deployment-rx" "deployment-rx" ((service #1#) (separator #2#) (environment "prod")) "List of bindings for `deployment-rx' and `deployment-rx-to-string' functions.\n\nSee `deployment-rx' for a human readable list of defined forms.\n\nSee parameter BINDINGS for function `rx-let' for more information\nabout format of elements of this list.")"#
    ]];
    assert_ample_regexps_parity(elisp_form, expect);
}
