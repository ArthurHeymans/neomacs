use expect_test::expect;

use super::assert_ac_dcd_parity;

#[test]
fn ac_dcd_exact_pin_dependencies_features_defaults_and_sources_match() {
    let elisp_form = r##"(let ((descriptor
                    (cadr
                     (assq 'ac-dcd package-alist))))
               (list
                (package-desc-name descriptor)
                (package-version-join
                 (package-desc-version descriptor))
                (package-desc-reqs descriptor)
                (mapcar
                 #'featurep
                 '(ac-dcd auto-complete flycheck-dmd-dub json rx))
                ac-dcd-executable
                ac-dcd-flags
                ac-dcd-server-executable
                ac-dcd-server-port
                ac-dcd-delay-after-kill-process
                ac-dcd-version
                ac-dcd-ignore-template-argument
                ac-source-dcd
                dcd-calltips
                dcd-calltips-for-struct-constructor))"##;
    let expect = expect![[
        r#"OK (ac-dcd "20250925.946" ((auto-complete (1 3 1)) (flycheck-dmd-dub (0 7))) (t t t t t) "dcd-client" nil "dcd-server" 9166 200 nil nil ((candidates . ac-dcd-get-candidates) (prefix . ac-dcd-prefix) (requires . 0) (document . ac-dcd-document) (action . ac-dcd-action) (cache) (symbol . "D")) ((candidates . ac-dcd-get-calltip-candidates) (prefix . ac-dcd-calltip-prefix) (action . ac-dcd-calltip-action) (cache)) ((candidates . ac-dcd-calltip-candidate-for-struct-constructor) (prefix . ac-dcd-calltip-prefix) (action . ac-dcd-calltip-action) (cache)))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_public_buffer_names_patterns_and_marker_ring_shape_match() {
    let elisp_form = r##"(list
               ac-dcd-error-buffer-name
               ac-dcd-output-buffer-name
               ac-dcd-document-buffer-name
               ac-dcd-search-symbol-buffer-name
               ac-dcd-goto-definition-marker-ring-length
               (ring-p ac-dcd-goto-definition-marker-ring)
               (ring-size ac-dcd-goto-definition-marker-ring)
               (mapcar
                (lambda (pattern)
                  (list
                   (stringp pattern)
                   (condition-case nil
                       (progn
                         (string-match-p pattern "")
                         t)
                     (invalid-regexp nil))))
                (list
                 ac-dcd-completion-pattern
                 ac-dcd-error-message-regexp
                 ac-dcd-normal-calltip-pattern
                 ac-dcd-template-pattern
                 ac-dcd-calltip-pattern)))"##;
    let expect = expect![[
        r#"OK ("*dcd-error*" "*dcd-output*" "*dcd-document*" "*dcd-search-symbol*" 16 t 16 ((t t) (t t) (t t) (t t) (t t)))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_document_maps_every_candidate_kind_and_unknown_values() {
    let elisp_form = r##"(mapcar
               (lambda (kind)
                 (let ((candidate
                        (propertize
                         "item"
                         'ac-dcd-help kind)))
                   (list
                    kind
                    (ac-dcd-document candidate))))
               '("c" "i" "s" "u" "v" "m" "k" "f" "g"
                 "e" "P" "M" "a" "A" "l" "t" "T" "x"))"##;
    let expect = expect![[
        r#"OK (("c" "class name") ("i" "interface name") ("s" "struct name") ("u" "union name") ("v" "variable name") ("m" "member variable name") ("k" "keyword, built-in version, scope statement") ("f" "function or method") ("g" "enum name") ("e" "enum member") ("P" "package name") ("M" "module name") ("a" "array") ("A" "associative array") ("l" "alias name") ("t" "template name") ("T" "mixin template name") ("x" "candidate kind undetected: x"))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}

#[test]
fn ac_dcd_document_rejects_non_strings_and_preserves_text_properties() {
    let elisp_form = r##"(let* ((candidate
                     (propertize
                      "call"
                      'ac-dcd-help "f"
                      'fixture '(nested value)))
                    (document
                     (ac-dcd-document candidate)))
               (list
                document
                (get-text-property
                 0 'fixture candidate)
                (ac-dcd-document nil)
                (ac-dcd-document 'call)
                candidate))"##;
    let expect = expect![[
        r#"OK ("function or method" (nested value) nil nil #("call" 0 4 (ac-dcd-help "f" fixture (nested value))))"#
    ]];

    assert_ac_dcd_parity(elisp_form, expect);
}
