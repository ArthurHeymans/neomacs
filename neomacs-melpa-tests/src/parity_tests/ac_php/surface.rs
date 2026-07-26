use expect_test::expect;

use super::assert_ac_php_parity;

#[test]
fn ac_php_exact_pin_dependencies_features_defaults_and_source_descriptors_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-php
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-php
                   ac-php-core
                   auto-complete
                   yasnippet))
                ac-php-template-start-point
                ac-php-template-candidates
                ac-source-php
                ac-source-php-template))"##;
    let expect = expect![[
        r#"OK (ac-php "20240328.1036" ((ac-php-core (2 0)) (auto-complete (1 4 0)) (yasnippet (0 8 0))) (t t t t) nil ("ok" "no" "yes:)") ((candidates . ac-php-candidate-ac) (prefix . ac-php-prefix) (requires . 0) (document . ac-php-document) (action . ac-php-action) (cache) (symbol . "p")) ((candidates . ac-php-template-candidate) (prefix . ac-php-template-prefix) (requires . 0) (action . ac-php-template-action) (document . ac-php-template-document) (cache) (symbol . "t")))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_complete_callable_surface_arities_interactivity_and_documentation_match() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (help-function-arglist
                   function t)
                  (interactive-form
                   function)
                  (documentation
                   function t)
                  (let ((definition
                         (symbol-function
                          function)))
                    (cond
                     ((symbolp definition)
                      definition)
                     ((byte-code-function-p
                       definition)
                      'byte-code)
                     (t 'interpreted)))))
               '(ac-php-prefix
                 ac-php-document
                 ac-php-action
                 ac-php-template-candidate
                 ac-php-template-action
                 ac-php-template-prefix
                 ac-php-template-document
                 ac-php-candidate-ac
                 ac-complete-php
                 ac-complete-php-template))"##;
    let expect = expect![[
        r#"OK ((ac-php-prefix nil nil "D." interpreted) (ac-php-document (item) nil "D ITEM." interpreted) (ac-php-action nil (interactive nil) "D." interpreted) (ac-php-template-candidate nil nil "D." interpreted) (ac-php-template-action nil (interactive nil) "D." interpreted) (ac-php-template-prefix nil nil "D." interpreted) (ac-php-template-document (item) nil "D ITEM." interpreted) (ac-php-candidate-ac nil nil "D." interpreted) (ac-complete-php nil (interactive nil) nil interpreted) (ac-complete-php-template nil (interactive nil) nil interpreted))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_package_variables_and_faces_preserve_exact_metadata() {
    let elisp_form = r##"(list
               (mapcar
                (lambda (variable)
                  (list
                   variable
                   (boundp variable)
                   (default-boundp variable)
                   (symbol-value variable)
                   (get variable
                        'standard-value)
                   (get variable
                        'variable-documentation)
                   (get variable
                        'risky-local-variable)))
                '(ac-php-template-start-point
                  ac-php-template-candidates
                  ac-source-php
                  ac-source-php-template))
               (mapcar
                (lambda (face)
                  (list
                   face
                   (facep face)
                   (get face
                        'face-defface-spec)
                   (face-documentation
                    face)
                   (get face
                        'custom-group)
                   (assq
                    face
                    (get
                     'ac-php
                     'custom-group))))
                '(ac-php-candidate-face
                  ac-php-selection-face)))"##;
    let expect = expect![[
        r#"OK (((ac-php-template-start-point t t nil nil nil nil) (ac-php-template-candidates t t ("ok" "no" "yes:)") nil nil nil) (ac-source-php t t ((candidates . ac-php-candidate-ac) (prefix . ac-php-prefix) (requires . 0) (document . ac-php-document) (action . ac-php-action) (cache) (symbol . "p")) nil nil nil) (ac-source-php-template t t ((candidates . ac-php-template-candidate) (prefix . ac-php-template-prefix) (requires . 0) (action . ac-php-template-action) (document . ac-php-template-document) (cache) (symbol . "t")) nil nil nil)) ((ac-php-candidate-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:background "lightgray" :foreground "navy"))) "Face for php candidate." nil (ac-php-candidate-face custom-face)) (ac-php-selection-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:background "navy" :foreground "white"))) "Face for the php selected candidate." nil (ac-php-selection-face custom-face))))"#
    ]];

    assert_ac_php_parity(elisp_form, expect);
}

#[test]
fn ac_php_generated_completion_commands_pass_only_their_declared_sources() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'auto-complete)
                     (lambda (&optional sources)
                       (push sources calls)
                       'completed)))
                 (list
                  (call-interactively
                   'ac-complete-php)
                  (call-interactively
                   'ac-complete-php-template)
                  (nreverse calls))))"##;
    let expect = expect!["OK (completed completed ((ac-source-php) (ac-source-php-template)))"];

    assert_ac_php_parity(elisp_form, expect);
}
