use expect_test::expect;

use super::{assert_add_hooks_autoload_parity, assert_add_hooks_parity};

#[test]
fn add_hooks_exact_pin_metadata_header_feature_and_complete_prefix_surface_match() {
    let elisp_form = r##"(progn
         (require
          'lisp-mnt)
         (let ((descriptor
                (cadr
                 (assq
                  'add-hooks
                  package-alist)))
               callables)
           (mapatoms
            (lambda (symbol)
              (when
                  (and
                   (string-prefix-p
                    "add-hooks"
                    (symbol-name
                     symbol))
                   (fboundp
                    symbol))
                (push
                 symbol
                 callables))))
           (list
            (package-desc-name
             descriptor)
            (package-version-join
             (package-desc-version
              descriptor))
            (package-desc-summary
             descriptor)
            (package-desc-kind
             descriptor)
            (package-desc-reqs
             descriptor)
            (package-desc-extras
             descriptor)
            (featurep
             'add-hooks)
            (with-temp-buffer
              (insert-file-contents
               (getenv
                "NEOMACS_PACKAGE_SOURCE"))
              (lm-header
               "version"))
            (sort
             callables
             (lambda (left right)
               (string-lessp
                (symbol-name
                 left)
                (symbol-name
                 right)))))))"##;
    let expect = expect![[
        r#"OK (add-hooks "20171217.123" "Functions for setting multiple hooks." nil nil ((:maintainers ("Nick McCurdy" . "nick@nickmccurdy.com")) (:authors ("Nick McCurdy" . "nick@nickmccurdy.com")) (:keywords "lisp") (:revdesc . "184513770346") (:commit . "1845137703461fc44bd77cf24014ba58f19c369d") (:url . "https://github.com/nickmccurdy/add-hooks")) t nil (add-hooks add-hooks-listify add-hooks-normalize-hook add-hooks-pair))"#
    ]];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_complete_callable_argument_command_documentation_and_source_surface_matches() {
    let elisp_form = r##"(mapcar
         (lambda (symbol)
           (list
            symbol
            (help-function-arglist
             symbol
             t)
            (commandp
             symbol)
            (interactive-form
             symbol)
            (documentation
             symbol
             t)
            (let ((file
                   (symbol-file
                    symbol
                    'defun)))
              (and
               file
               (file-name-nondirectory
                file)))))
         '(add-hooks-listify
           add-hooks-normalize-hook
           add-hooks-pair
           add-hooks))"##;
    let expect = expect![[
        r#"OK ((add-hooks-listify (object) nil nil "If OBJECT is a list and not a function, return it, else wrap it in a list." "add-hooks.el") (add-hooks-normalize-hook (hook) nil nil "If HOOK is a symbol, ensure `-hook' is appended, else return HOOK itself." "add-hooks.el") (add-hooks-pair (hooks functions) nil nil "Call `add-hook' for each combined pair of items in HOOKS and FUNCTIONS.\n\nHOOKS can be a symbol or a list of symbols representing hook\nvariables (the `-hook' suffix is implied).  FUNCTIONS can be a\nsymbol, a lambda, or a list of either representing hook\nfunctions.  If lists are used, a function can be added to\nmultiple hooks and/or multiple functions can be added to a hook.\n\nExample:\n\n  ELISP> (add-hooks-pair '(css-mode sgml-mode) 'emmet-mode)\n  nil\n  ELISP> css-mode-hook\n  (emmet-mode)\n  ELISP> sgml-mode-hook\n  (emmet-mode)" "add-hooks.el") (add-hooks (pairs) nil nil "Call `add-hooks-pair' on each cons pair in PAIRS.\n\nEach pair has a `car' for setting hooks and a `cdr' for setting\nfunctions to add to those hooks.  Pair values are passed to the\nHOOKS and FUNCTIONS arguments of `add-hooks-pair', respectively.\n\nUsage:\n\n  (add-hooks ((HOOKS . FUNCTIONS)...))\n\nExample:\n\n  ELISP> (add-hooks '(((css-mode sgml-mode) . emmet-mode)))\n  nil\n  ELISP> css-mode-hook\n  (emmet-mode)\n  ELISP> sgml-mode-hook\n  (emmet-mode)" "add-hooks.el"))"#
    ]];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_installed_package_inventory_sizes_and_sha256_match_exactly() {
    let elisp_form = r##"(let* ((descriptor
                  (cadr
                   (assq
                    'add-hooks
                    package-alist)))
                 (directory
                  (package-desc-dir
                   descriptor)))
         (mapcar
          (lambda (file)
            (let ((path
                   (expand-file-name
                    file
                    directory)))
              (list
               file
               (file-attribute-size
                (file-attributes
                 path))
               (with-temp-buffer
                 (set-buffer-multibyte nil)
                 (insert-file-contents-literally path)
                 (secure-hash
                  'sha256
                  (current-buffer))))))
          (sort
           (seq-filter
            (lambda (file)
              (file-regular-p
               (expand-file-name
                file
                directory)))
            (directory-files
             directory
             nil
             "\\`[^.]"))
           #'string-lessp)))"##;
    let expect = expect![[
        r#"OK (("README-elpa" 550 "cb98e99a8524f03a6575c643b10239c8a1690c700a679319dc1bb6f77572c68c") ("add-hooks-autoloads.el" 1776 "ae6583b17dd7a593dbf1628eab243474b6b6b7e1b2008831fda44b5320d2897e") ("add-hooks-pkg.el" 410 "794fbaf363e26e685e88828eda459dad4270717f7131db8f0214a7a8affe8afd") ("add-hooks.el" 3510 "1c0674fafbeacb94ae3b27be92685d29664a5a70b7e636cc0767dffd322d3fef") ("add-hooks.elc" 1923 "7aa0d0df48921c45c752181d9b6441c2c1289a396144be4f60ecb5dae3e55303"))"#
    ]];
    assert_add_hooks_parity(elisp_form, expect);
}

#[test]
fn add_hooks_generated_autoload_surface_exposes_only_public_pair_and_batch_functions() {
    let elisp_form = r##"(list
         (featurep
          'add-hooks-autoloads)
         (featurep
          'add-hooks)
         (mapcar
          (lambda (symbol)
            (list
             symbol
             (fboundp
              symbol)
             (and
              (fboundp
               symbol)
              (autoloadp
               (symbol-function
                symbol)))
             (commandp
              symbol)
             (and
              (fboundp
               symbol)
              (help-function-arglist
               symbol
               t))))
          '(add-hooks-listify
            add-hooks-normalize-hook
            add-hooks-pair
            add-hooks)))"##;
    let expect = expect![[
        r#"OK (t nil ((add-hooks-listify nil nil nil nil) (add-hooks-normalize-hook nil nil nil nil) (add-hooks-pair t t nil "[Arg list not available until function definition is loaded.]") (add-hooks t t nil "[Arg list not available until function definition is loaded.]")))"#
    ]];
    assert_add_hooks_autoload_parity(elisp_form, expect);
}
