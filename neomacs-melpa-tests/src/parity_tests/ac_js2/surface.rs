use expect_test::expect;

use super::assert_ac_js2_parity;

#[test]
fn ac_js2_exact_pin_dependencies_features_defaults_constants_and_data_root_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq
                      'ac-js2
                      package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-js2
                   js2-mode
                   skewer-mode
                   cl-lib
                   etags))
                (get
                 'ac-js2
                 'group-documentation)
                (assq
                 'ac-js2
                 (get
                  'completion
                  'custom-group))
                (mapcar
                 (lambda (variable)
                   (list
                    variable
                    (symbol-value variable)
                    (get variable
                         'standard-value)
                    (get variable
                         'variable-documentation)))
                 '(ac-js2-add-ecma-262-externs
                   ac-js2-add-browser-externs
                   ac-js2-add-keywords
                   ac-js2-add-prototype-completions
                   ac-js2-external-libraries
                   ac-js2-evaluate-calls
                   ac-js2-force-reparse))
                ac-js2-method-eval
                ac-js2-method-global
                (file-name-nondirectory
                 (directory-file-name
                  ac-js2-data-root))
                (file-exists-p
                 (expand-file-name
                  "skewer-addon.js"
                  ac-js2-data-root))
                ac-js2-keywords
                ac-js2-candidates
                ac-js2-skewer-candidates))"##;
    let expect = expect![[
        r#"OK (ac-js2 "20190101.933" ((js2-mode (20090723)) (skewer-mode (1 4))) (t t t t t) "Auto-completion for js2-mode." (ac-js2 custom-group) ((ac-js2-add-ecma-262-externs t (t) "If non-nil add `js2-ecma-262-externs' to completion candidates.") (ac-js2-add-browser-externs t (t) "If non-nil add `js2-browser-externs' to completion candidates.") (ac-js2-add-keywords t (t) "If non-nil add `js2-keywords' to completion candidates.") (ac-js2-add-prototype-completions t (t) "When non-nil traverse the prototype chain adding to completion candidates.") (ac-js2-external-libraries nil ('nil) "List of absolute paths to external Javascript libraries.") (ac-js2-evaluate-calls nil (nil) "Warning. When true function calls will be evaluated in the browser.\nThis may cause undesired side effects however it will\n  provide better completions. Use at your own risk.") (ac-js2-force-reparse t (t) "Force Js2-mode to reparse buffer before fetching completion candidates.")) 0 1 "ac-js2-20190101.933" t nil nil nil)"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_custom_types_groups_and_callable_arities_match() {
    let elisp_form = r##"(list
               (mapcar
                (lambda (variable)
                  (list
                   variable
                   (get variable
                        'custom-type)
                   (get variable
                        'custom-group)))
                '(ac-js2-add-ecma-262-externs
                  ac-js2-add-browser-externs
                  ac-js2-add-keywords
                  ac-js2-add-prototype-completions
                  ac-js2-external-libraries
                  ac-js2-evaluate-calls
                  ac-js2-force-reparse))
               (mapcar
                (lambda (function)
                  (list
                   function
                   (help-function-arglist
                    function t)
                   (interactive-form
                    function)))
                '(ac-js2-on-skewer-load
                  ac-js2-skewer-completion-candidates
                  ac-js2-skewer-document-candidates
                  ac-js2-get-object-properties
                  ac-js2-skewer-result-callback
                  ac-js2-skewer-eval-wrapper
                  ac-js2-candidates
                  ac-js2-document
                  ac-js2-ac-candidates
                  ac-js2-ac-document
                  ac-js2-ac-prefix
                  ac-js2-save
                  ac-js2-expand-function
                  ac-js2-setup-auto-complete-mode
                  ac-js2-completion-function
                  ac-js2-company
                  ac-js2-build-prop-name-list
                  ac-js2-prop-names-left
                  ac-js2-has-function-calls
                  ac-js2-add-extra-completions
                  ac-js2-root-or-node
                  ac-js2-get-names-in-scope
                  ac-js2-initialized-node
                  ac-js2-name-declaration
                  ac-js2-format-node
                  ac-js2-format-object-node-doc
                  ac-js2-format-node-doc
                  ac-js2-format-js2-object-prop-doc
                  ac-js2-format-function
                  ac-js2-format-comment
                  ac-js2-find-property
                  ac-js2-get-function-node
                  ac-js2-jump-to-definition
                  ac-js2-get-function-name)))"##;
    let expect = expect![[
        r#"OK (((ac-js2-add-ecma-262-externs nil nil) (ac-js2-add-browser-externs nil nil) (ac-js2-add-keywords nil nil) (ac-js2-add-prototype-completions nil nil) (ac-js2-external-libraries nil nil) (ac-js2-evaluate-calls nil nil) (ac-js2-force-reparse nil nil)) ((ac-js2-on-skewer-load nil nil) (ac-js2-skewer-completion-candidates nil nil) (ac-js2-skewer-document-candidates (name) nil) (ac-js2-get-object-properties (name) nil) (ac-js2-skewer-result-callback (result) nil) (ac-js2-skewer-eval-wrapper (str &optional extras) nil) (ac-js2-candidates nil nil) (ac-js2-document (name) nil) (ac-js2-ac-candidates nil nil) (ac-js2-ac-document (name) nil) (ac-js2-ac-prefix nil nil) (ac-js2-save nil (interactive nil)) (ac-js2-expand-function nil (interactive nil)) (ac-js2-setup-auto-complete-mode nil nil) (ac-js2-completion-function nil nil) (ac-js2-company (command &optional arg &rest ignored) (interactive (list 'interactive))) (ac-js2-build-prop-name-list (prop-node) nil) (ac-js2-prop-names-left (name-node) nil) (ac-js2-has-function-calls (string) nil) (ac-js2-add-extra-completions (completions) nil) (ac-js2-root-or-node nil nil) (ac-js2-get-names-in-scope nil nil) (ac-js2-initialized-node (name) nil) (ac-js2-name-declaration (name) nil) (ac-js2-format-node (name node) nil) (ac-js2-format-object-node-doc (obj-node) nil) (ac-js2-format-node-doc (node) nil) (ac-js2-format-js2-object-prop-doc (obj-prop) nil) (ac-js2-format-function (func) nil) (ac-js2-format-comment (comment) nil) (ac-js2-find-property (list-names) nil) (ac-js2-get-function-node (name scope) nil) (ac-js2-jump-to-definition nil (interactive nil)) (ac-js2-get-function-name (fn-node) nil)))"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}

#[test]
fn ac_js2_mode_map_and_minor_mode_metadata_match() {
    let elisp_form = r##"(list
               (keymapp
                ac-js2-mode-map)
               (mapcar
                (lambda (key)
                  (lookup-key
                   ac-js2-mode-map
                   (kbd key)))
                '("M-."
                  "M-,"
                  "C-c C-c"))
               (get
                'ac-js2-mode
                'variable-documentation)
               (get
                'ac-js2-mode
                'custom-type)
               (get
                'ac-js2-mode
                'custom-group)
               (help-function-arglist
                'ac-js2-mode t)
               (interactive-form
                'ac-js2-mode)
               (documentation
                'ac-js2-mode t))"##;
    let expect = expect![[
        r#"OK (t (ac-js2-jump-to-definition pop-tag-mark ac-js2-expand-function) "Non-nil if Ac-Js2 mode is enabled.\nUse the command `ac-js2-mode' to change this variable." nil nil (&optional arg) (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "A minor mode that provides auto-completion and navigation for Js2-mode.\n\nThis is a minor mode.  If called interactively, toggle the `Ac-Js2 mode'\nmode.  If the prefix argument is positive, enable the mode, and if it is\nzero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable `ac-js2-mode'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled.\n\n\\{ac-js2-mode-map}")"#
    ]];

    assert_ac_js2_parity(elisp_form, expect);
}
