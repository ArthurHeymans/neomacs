use expect_test::expect;

use super::assert_ac_ispell_parity;

#[test]
fn ac_ispell_exact_pin_dependencies_features_defaults_custom_metadata_and_cache_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-ispell
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-ispell
                   auto-complete
                   ispell
                   ring))
                (get
                 'ac-ispell
                 'group-documentation)
                (get
                 'ac-ispell
                 'custom-group)
                (mapcar
                 (lambda (variable)
                   (list
                    variable
                    (symbol-value variable)
                    (get variable
                         'standard-value)
                    (get variable
                         'custom-type)
                    (get variable
                         'custom-group)))
                 '(ac-ispell-requires
                   ac-ispell-fuzzy-limit
                   ac-ispell-cache-size))
                (ring-p
                 ac-ispell--cache)
                (ring-size
                 ac-ispell--cache)
                (ring-length
                 ac-ispell--cache)))"##;
    let expect = expect![[
        r#"OK (ac-ispell "20151101.226" ((auto-complete (1 4)) (cl-lib (0 5))) (t t t t) "Auto completion with ispell." ((ac-ispell-requires custom-variable) (ac-ispell-fuzzy-limit custom-variable) (ac-ispell-cache-size custom-variable) (ac-ispell-fuzzy-candidate-face custom-face)) ((ac-ispell-requires 3 (3) integer nil) (ac-ispell-fuzzy-limit 2 (2) integer nil) (ac-ispell-cache-size 20 (20) integer nil)) t 20 0)"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_custom_documentation_and_parent_group_relationship_match() {
    let elisp_form = r##"(list
               (mapcar
                (lambda (variable)
                  (list
                   variable
                   (get variable
                        'variable-documentation)))
                '(ac-ispell-requires
                  ac-ispell-fuzzy-limit
                  ac-ispell-cache-size))
               (assq
                'ac-ispell
                (get
                 'auto-complete
                 'custom-group)))"##;
    let expect = expect![[
        r#"OK (((ac-ispell-requires "Minimum input for starting completion.") (ac-ispell-fuzzy-limit "Limit number of candidates for fuzzy source.") (ac-ispell-cache-size "Size of candidates cache.")) (ac-ispell custom-group))"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_fuzzy_face_preserves_exact_spec_documentation_and_group() {
    let elisp_form = r##"(list
               (facep
                'ac-ispell-fuzzy-candidate-face)
               (get
                'ac-ispell-fuzzy-candidate-face
                'face-defface-spec)
               (face-documentation
                'ac-ispell-fuzzy-candidate-face)
               (get
                'ac-ispell-fuzzy-candidate-face
                'custom-group))"##;
    let expect = expect![[
        r#"OK ([face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit ac-candidate-face :foreground "red"))) "Face for fuzzy candidate." nil)"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}

#[test]
fn ac_ispell_function_surface_arities_interactivity_and_documentation_match() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (help-function-arglist
                   function t)
                  (interactive-form function)
                  (documentation function t)
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
               '(ac-ispell--case-function
                 ac-ispell--lookup-candidates
                 ac-ispell--lookup-cache
                 ac-ispell--candidates
                 ac-ispell--correct-word
                 ac-ispell--fuzzy-candidates
                 ac-ispell-ac-setup
                 ac-ispell-setup))"##;
    let expect = expect![[
        r#"OK ((ac-ispell--case-function (input) nil nil interpreted) (ac-ispell--lookup-candidates (lookup-func input) nil nil interpreted) (ac-ispell--lookup-cache (input) nil nil interpreted) (ac-ispell--candidates nil nil nil interpreted) (ac-ispell--correct-word (word) nil nil interpreted) (ac-ispell--fuzzy-candidates nil nil nil interpreted) (ac-ispell-ac-setup nil (interactive nil) "Add `ac-source-ispell' to `ac-sources' and enable `auto-complete' mode" interpreted) (ac-ispell-setup nil (interactive nil) "Declare auto-complete source based on `ac-ispell-requires'" interpreted))"#
    ]];

    assert_ac_ispell_parity(elisp_form, expect);
}
