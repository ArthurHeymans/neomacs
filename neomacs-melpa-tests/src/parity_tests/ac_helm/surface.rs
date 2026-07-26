use expect_test::expect;

use super::assert_ac_helm_parity;

#[test]
fn ac_helm_exact_pin_dependencies_features_private_api_and_source_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-helm package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-helm
                   helm
                   helm-multi-match
                   helm-elisp
                   auto-complete
                   cl-lib
                   popup))
                (mapcar
                 (lambda (symbol)
                   (list
                    symbol
                    (functionp symbol)
                    (macrop symbol)))
                 '(with-helm-show-completion
                   helm
                   helm-attr
                   helm-attrset
                   helm-aif
                   popup-preferred-width
                   popup-item-show-help))
                helm-source-auto-complete-candidates))"##;
    let expect = expect![[
        r#"OK (ac-helm "20160319.233" ((helm (1 6 3)) (auto-complete (1 4 0)) (popup (0 5 0)) (cl-lib (0 5))) (t t t t t t t) ((with-helm-show-completion nil t) (helm t nil) (helm-attr t nil) (helm-attrset t nil) (helm-aif nil t) (popup-preferred-width t nil) (popup-item-show-help t nil)) ((name . "Auto Complete") (init . helm-auto-complete-init) (candidates . helm-auto-complete-candidates) (action . helm-auto-complete-action) (persistent-action . popup-item-show-help) (ac-candidates) (menu-width)))"#
    ]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_function_arities_interactive_forms_and_documentation_match() {
    let elisp_form = r##"(mapcar
               (lambda (function)
                 (list
                  function
                  (help-function-arglist
                   function t)
                  (interactive-form function)
                  (documentation function t)))
               '(ac-complete-with-helm
                 helm-auto-complete-init
                 helm-auto-complete-action
                 helm-auto-complete-candidates))"##;
    let expect = expect![[
        r#"OK ((ac-complete-with-helm nil (interactive nil) "Select `auto-complete' candidates by `helm'.\nIt is useful to narrow candidates.") (helm-auto-complete-init nil nil nil) (helm-auto-complete-action (string) nil nil) (helm-auto-complete-candidates nil nil nil))"#
    ]];

    assert_ac_helm_parity(elisp_form, expect);
}

#[test]
fn ac_helm_source_callbacks_marker_and_placeholder_attributes_match() {
    let elisp_form = r##"(mapcar
               (lambda (entry)
                 (list
                  entry
                  (cond
                   ((null
                     (cdr entry))
                    'placeholder)
                   ((functionp
                     (cdr entry))
                    'function)
                   (t 'value))))
               helm-source-auto-complete-candidates)"##;
    let expect = expect![[
        r#"OK (((name . "Auto Complete") value) ((init . helm-auto-complete-init) function) ((candidates . helm-auto-complete-candidates) function) ((action . helm-auto-complete-action) function) ((persistent-action . popup-item-show-help) function) ((ac-candidates) placeholder) ((menu-width) placeholder))"#
    ]];

    assert_ac_helm_parity(elisp_form, expect);
}
