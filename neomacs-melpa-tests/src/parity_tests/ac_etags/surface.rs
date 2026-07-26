use expect_test::expect;

use super::assert_ac_etags_parity;

#[test]
fn ac_etags_exact_pin_dependencies_features_defaults_and_cache_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-etags package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-etags auto-complete etags))
                (get 'ac-etags 'group-documentation)
                (get 'ac-etags 'custom-group)
                ac-etags-requires
                (get 'ac-etags-requires 'standard-value)
                (get 'ac-etags-requires 'custom-type)
                (get 'ac-etags-requires 'custom-group)
                (hash-table-p
                 ac-etags--completion-cache)
                (hash-table-test
                 ac-etags--completion-cache)
                (hash-table-count
                 ac-etags--completion-cache)
                (boundp 'ac-source-etags)))"##;
    let expect = expect![[
        r#"OK (ac-etags "20161001.1507" ((auto-complete (1 4))) (t t t) "Auto completion with etags" ((ac-etags-requires custom-variable) (ac-etags-candidate-face custom-face) (ac-etags-selection-face custom-face)) 3 (3) integer nil t equal 0 nil)"#
    ]];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_faces_preserve_exact_specs_documentation_and_groups() {
    let elisp_form = r##"(mapcar
               (lambda (face)
                 (list
                  face
                  (facep face)
                  (get face 'face-defface-spec)
                  (face-documentation face)
                  (get face 'custom-group)))
               '(ac-etags-candidate-face
                 ac-etags-selection-face))"##;
    let expect = expect![[
        r#"OK ((ac-etags-candidate-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit ac-candidate-face :foreground "navy"))) "Face for etags candidate" nil) (ac-etags-selection-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit ac-selection-face :background "navy"))) "Face for the etags selected candidate." nil))"#
    ]];

    assert_ac_etags_parity(elisp_form, expect);
}

#[test]
fn ac_etags_function_arities_interactive_forms_and_documentation_match() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (help-function-arglist
                   function t)
                  (interactive-form function)
                  (documentation function t)))
               '(ac-etags--cache-candidates
                 ac-etags--candidates
                 ac-etags-ac-setup
                 ac-etags-clear-cache
                 ac-etags-setup))"##;
    let expect = expect![[
        r#"OK ((ac-etags--cache-candidates (prefix) nil nil) (ac-etags--candidates nil nil nil) (ac-etags-ac-setup nil (interactive nil) "Add `ac-source-etags' to `ac-sources' and enable `auto-complete' mode") (ac-etags-clear-cache nil (interactive nil) nil) (ac-etags-setup nil (interactive nil) nil))"#
    ]];

    assert_ac_etags_parity(elisp_form, expect);
}
