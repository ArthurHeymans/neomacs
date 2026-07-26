use expect_test::expect;

use super::assert_ac_math_parity;

#[test]
fn ac_math_exact_pin_dependencies_features_customs_and_defaults_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-math
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-math
                   auto-complete
                   math-symbol-lists))
                (get
                 'ac-math
                 'group-documentation)
                (assq
                 'ac-math
                 (get
                  'auto-complete
                  'custom-group))
                (get
                 'ac-math
                 'custom-prefix)
                (mapcar
                 (lambda (variable)
                   (list
                    variable
                    (symbol-value variable)
                    (get variable
                         'standard-value)
                    (get variable
                         'variable-documentation)
                    (get variable
                         'custom-type)
                    (get variable
                         'custom-group)))
                 '(ac-math-unicode-in-math-p
                   ac-math-prefix-regexp))
                ac-math--dummy))"##;
    let expect = expect![[
        r#"OK (ac-math "20141116.2127" ((auto-complete (1 4)) (math-symbol-lists (1 0))) (t t t) "Auto completion." (ac-math custom-group) "ac-math" ((ac-math-unicode-in-math-p nil (nil) "Set this to t if unicode in math latex environments is needed." nil nil) (ac-math-prefix-regexp "\\\\\\(.*\\)" ("\\\\\\(.*\\)") "Regexp matching the prefix of the ac-math symbol." nil nil)) " ")"#
    ]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_callable_surface_and_source_descriptors_match() {
    let elisp_form = r##"(list
               (mapcar
                (lambda (function)
                  (list
                   function
                   (help-function-arglist
                    function t)
                   (interactive-form
                    function)
                   (documentation
                    function t)))
                '(ac-math--make-candidates
                  ac-math-action-latex
                  ac-math-action-unicode
                  ac-math-latex-math-face-p
                  ac-math-candidates-latex
                  ac-math-candidates-unicode
                  ac-math-prefix))
               ac-source-latex-commands
               ac-source-math-latex
               ac-source-math-unicode)"##;
    let expect = expect![[
        r#"OK (((ac-math--make-candidates (alist &optional unicode) nil "Build a list of math symbols ready to be used in ac source.\nEach element is a cons cell (SYMB . VALUE) where SYMB is the\nstring to be displayed during the completion and the VALUE is the\nactually value inserted on RET completion.  If UNICODE is non-nil\nthe value of VALUE is the unicode character else it's the latex\ncommand.") (ac-math-action-latex (&optional del-backward) nil "Function to be used in ac action property.\nDeletes the unicode symbol from the end of the completed\nstring. If DEL-BACKWARD is non-nil, delete the name of the symbol\ninstead.") (ac-math-action-unicode nil nil nil) (ac-math-latex-math-face-p nil nil nil) (ac-math-candidates-latex nil nil nil) (ac-math-candidates-unicode nil nil nil) (ac-math-prefix nil nil "Return the location of the start of the current symbol.\nUses `ac-math-prefix-regexp'.")) ((candidates . math-symbol-list-latex-commands) (symbol . "c") (prefix . ac-math-prefix)) ((candidates . ac-math-candidates-latex) (symbol . "l") (prefix . ac-math-prefix) (action . ac-math-action-latex)) ((candidates . ac-math-candidates-unicode) (symbol . "u") (prefix . ac-math-prefix) (action . ac-math-action-unicode)))"#
    ]];

    assert_ac_math_parity(elisp_form, expect);
}

#[test]
fn ac_math_all_package_variables_have_exact_metadata_and_defaults() {
    let elisp_form = r##"(mapcar
               (lambda (variable)
                 (list
                  variable
                  (boundp variable)
                  (default-boundp variable)
                  (get variable
                       'standard-value)
                  (get variable
                       'variable-documentation)
                  (get variable
                       'custom-type)
                  (get variable
                       'custom-group)
                  (get variable
                       'risky-local-variable)))
               '(ac-math--dummy
                 ac-math-symbols-latex
                 ac-math-symbols-unicode
                 ac-source-latex-commands
                 ac-source-math-latex
                 ac-source-math-unicode))"##;
    let expect = expect![[
        r#"OK ((ac-math--dummy t t nil nil nil nil nil) (ac-math-symbols-latex t t nil "List of math completion candidates." nil nil t) (ac-math-symbols-unicode t t nil "List of math completion candidates." nil nil t) (ac-source-latex-commands t t nil nil nil nil nil) (ac-source-math-latex t t nil nil nil nil nil) (ac-source-math-unicode t t nil nil nil nil nil))"#
    ]];

    assert_ac_math_parity(elisp_form, expect);
}
