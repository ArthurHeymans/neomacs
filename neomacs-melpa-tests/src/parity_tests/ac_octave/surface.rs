use expect_test::expect;

use super::assert_ac_octave_parity;

#[test]
fn ac_octave_exact_pin_dependencies_features_variable_and_source_descriptor_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-octave
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-octave
                   auto-complete
                   octave
                   octave-inf))
                (list
                 ac-octave-complete-list
                 (get
                  'ac-octave-complete-list
                  'standard-value)
                 (get
                  'ac-octave-complete-list
                  'variable-documentation))
                ac-source-octave))"##;
    let expect = expect![[
        r#"OK (ac-octave "20180406.334" ((auto-complete (1 4 0))) (t t t nil) (nil nil nil) ((candidates . ac-octave-candidate) (document . ac-octave-documentation) (candidate-face . ac-octave-candidate-face) (selection-face . ac-octave-selection-face) (init . ac-octave-init) (requires . 0) (cache) (symbol . "f")))"#
    ]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_complete_function_surface_arities_interactivity_and_documentation_match() {
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
               '(ac-octave-init
                 ac-octave-do-complete
                 ac-octave-candidate
                 ac-octave-documentation
                 ac-octave-setup
                 ac-complete-octave))"##;
    let expect = expect![[
        r#"OK ((ac-octave-init nil nil "Start inferior-octave in background before use ac-octave." interpreted) (ac-octave-do-complete nil (interactive nil) nil interpreted) (ac-octave-candidate nil nil nil interpreted) (ac-octave-documentation (symbol) nil nil interpreted) (ac-octave-setup nil nil "Add the Octave completion source to the front of `ac-sources'." interpreted) (ac-complete-octave nil (interactive nil) nil interpreted))"#
    ]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_faces_preserve_exact_specs_documentation_and_custom_group() {
    let elisp_form = r##"(mapcar
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
                    'auto-complete
                    'custom-group))))
               '(ac-octave-candidate-face
                 ac-octave-selection-face))"##;
    let expect = expect![[
        r#"OK ((ac-octave-candidate-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit ac-candidate-face))) "face for octave candidate" nil (ac-octave-candidate-face custom-face)) (ac-octave-selection-face [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:inherit ac-selection-face))) "face for the octave selected candidate." nil (ac-octave-selection-face custom-face)))"#
    ]];

    assert_ac_octave_parity(elisp_form, expect);
}

#[test]
fn ac_octave_generated_completion_command_calls_auto_complete_with_only_its_source() {
    let elisp_form = r##"(let (calls)
               (cl-letf
                   (((symbol-function
                      'auto-complete)
                     (lambda (&optional sources)
                       (push sources calls)
                       'completion-result)))
                 (list
                  (call-interactively
                   'ac-complete-octave)
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (completion-result ((ac-source-octave)))"#]];

    assert_ac_octave_parity(elisp_form, expect);
}
