use expect_test::expect;

use super::assert_aas_parity;

#[test]
fn aas_public_surface_defaults_hooks_alias_and_custom_metadata_match_the_pin() {
    let elisp_form = r##"(list
               (featurep 'aas)
               (featurep 'cl-lib)
               (mapcar
                #'fboundp
                '(aas--key-is-fully-typed?
                  aas-expand-snippet-maybe
                  aas-define-prefix-map-snippet
                  aas-set-snippets
                  aas-post-self-insert-hook
                  aas-activate-keymap
                  aas-deactivate-keymap
                  aas--modes-to-activate
                  aas-mode
                  aas-global-mode
                  aas-activate-for-major-mode
                  ass-activate-for-major-mode
                  aas-embark-menu
                  aas--format-doc-to-org
                  aas--format-snippet-array))
               (mapcar
                #'commandp
                '(aas-mode
                  aas-global-mode
                  aas-activate-for-major-mode
                  ass-activate-for-major-mode
                  aas-embark-menu))
               aas-pre-snippet-expand-hook
               aas-post-snippet-expand-hook
               aas-global-condition-hook
               (hash-table-test aas-keymaps)
               (with-temp-buffer
                 (list
                  aas-transient-snippet-key
                  aas-transient-snippet-expansion
                  aas-transient-snippet-condition-result
                  aas-active-keymaps
                  aas--prefix-map
                  aas--current-prefix-maps
                  aas-mode))
               (mapcar
                (lambda (variable)
                  (list
                   (get variable 'custom-group)
                   (get variable 'custom-type)
                   (eval
                    (car
                     (get variable 'standard-value)))))
                '(aas-pre-snippet-expand-hook
                  aas-post-snippet-expand-hook
                  aas-global-condition-hook))
               (eq
                (indirect-function
                 'ass-activate-for-major-mode)
                (indirect-function
                 'aas-activate-for-major-mode))
               (get
                'ass-activate-for-major-mode
                'byte-obsolete-info)
               (documentation 'aas-global-mode))"##;
    let expect = expect![[
        r#"OK (t t (t t t t t t t t t t t t t t t) (t t nil nil t) nil nil (aas--key-is-fully-typed?) eq (nil nil nil nil nil (nil) nil) ((nil hook nil) (nil hook nil) (nil hook nil)) t (aas-activate-for-major-mode nil "1.1") "Global ‘aas-mode’. The activated keymap is ‘global’: set global snippets with\n(aas-set-snippets ’global ...)")"#
    ]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_key_fully_typed_checks_exact_forward_range_and_preserves_point() {
    let elisp_form = r##"(with-temp-buffer
               (insert "αabc!")
               (list
                (progn
                  (goto-char 2)
                  (let ((aas-transient-snippet-key "abc"))
                    (list
                     (aas--key-is-fully-typed?)
                     (point))))
                (progn
                  (goto-char 3)
                  (let ((aas-transient-snippet-key "bc"))
                    (list
                     (aas--key-is-fully-typed?)
                     (point))))
                (progn
                  (goto-char 2)
                  (let ((aas-transient-snippet-key "abd"))
                    (list
                     (aas--key-is-fully-typed?)
                     (point))))
                (progn
                  (goto-char 2)
                  (let ((aas-transient-snippet-key "ab"))
                    (list
                     (aas--key-is-fully-typed?)
                     (point))))))"##;
    let expect = expect!["OK ((5 2) (5 3) (nil 2) (4 2))"];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_transient_snippet_variables_are_buffer_local_when_assigned() {
    let elisp_form = r##"(let ((outside
                    (list
                     aas-transient-snippet-key
                     aas-transient-snippet-expansion
                     aas-transient-snippet-condition-result)))
               (list
                (with-temp-buffer
                  (setq aas-transient-snippet-key "key"
                        aas-transient-snippet-expansion "value"
                        aas-transient-snippet-condition-result 'accepted)
                  (list
                   (local-variable-p
                    'aas-transient-snippet-key)
                   (local-variable-p
                    'aas-transient-snippet-expansion)
                   (local-variable-p
                    'aas-transient-snippet-condition-result)
                   aas-transient-snippet-key
                   aas-transient-snippet-expansion
                   aas-transient-snippet-condition-result))
                outside
                (list
                 aas-transient-snippet-key
                 aas-transient-snippet-expansion
                 aas-transient-snippet-condition-result)))"##;
    let expect = expect![[r#"OK ((t t t "key" "value" accepted) (nil nil nil) (nil nil nil))"#]];

    assert_aas_parity(elisp_form, expect);
}
