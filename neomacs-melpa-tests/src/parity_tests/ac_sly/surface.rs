use expect_test::expect;

use super::assert_ac_sly_parity;

#[test]
fn ac_sly_internal_state_and_source_variable_documentation_match() {
    let elisp_form = r##"(list
               ac-sly-current-doc
               (get
                'ac-sly-current-doc
                'variable-documentation)
               (default-boundp
                'ac-sly-current-doc)
               (get
                'ac-source-sly-fuzzy
                'variable-documentation)
               (get
                'ac-source-sly-simple
                'variable-documentation)
               (default-value
                'ac-source-sly-fuzzy)
               (default-value
                'ac-source-sly-simple))"##;
    let expect = expect![[
        r#"OK (nil "Holds slime docstring for current symbol." t "Source for fuzzy slime completion." "Source for slime completion." ((init . ac-sly-init) (candidates . ac-source-sly-fuzzy-candidates) (candidate-face . ac-sly-menu-face) (selection-face . ac-sly-selection-face) (prefix . sly-symbol-start-pos) (symbol . "l") (match lambda (prefix candidates) candidates) (document . ac-sly-documentation)) ((init . ac-sly-init) (candidates . ac-source-sly-simple-candidates) (candidate-face . ac-sly-menu-face) (selection-face . ac-sly-selection-face) (prefix . sly-symbol-start-pos) (symbol . "l") (document . ac-sly-documentation) (match . ac-source-sly-case-correcting-completions)))"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_packaged_source_descriptor_and_autoload_assets_have_exact_hashes() {
    let elisp_form = r##"(let ((root
                    (file-name-directory
                     (symbol-file
                      'ac-sly-init
                      'defun))))
               (mapcar
                (lambda (file)
                  (with-temp-buffer
                    (insert-file-contents-literally
                     (expand-file-name
                      file
                      root))
                    (list
                     file
                     (buffer-size)
                     (secure-hash
                      'sha256
                      (current-buffer)))))
                '("ac-sly.el"
                  "ac-sly-pkg.el"
                  "ac-sly-autoloads.el")))"##;
    let expect = expect![[
        r#"OK (("ac-sly.el" 4486 "48d69223893780fc6e7d646717d26db38836cf7008456981de7ae6059e9130df") ("ac-sly-pkg.el" 487 "c3c955160b1f7fb8f6c461dadce7e0cd565cf893d34ffe301045b51269076514") ("ac-sly-autoloads.el" 1725 "1ee1a768c61ff287d2b2ca48051e412a37b71819fa3b89d5cd08f500f9ce1e83"))"#
    ]];

    assert_ac_sly_parity(elisp_form, expect);
}

#[test]
fn ac_sly_source_descriptors_keep_distinct_match_callbacks_and_shared_live_symbols() {
    let elisp_form = r##"(let ((candidates
                    (list
                     'one
                     'two)))
               (list
                (eq
                 (cdr
                  (assq
                   'init
                   ac-source-sly-fuzzy))
                 (cdr
                  (assq
                   'init
                   ac-source-sly-simple)))
                (eq
                 (cdr
                  (assq
                   'prefix
                   ac-source-sly-fuzzy))
                 (cdr
                  (assq
                   'prefix
                   ac-source-sly-simple)))
                (eq
                 (cdr
                  (assq
                   'document
                   ac-source-sly-fuzzy))
                 (cdr
                  (assq
                   'document
                   ac-source-sly-simple)))
                (functionp
                 (cdr
                  (assq
                   'match
                   ac-source-sly-fuzzy)))
                (symbolp
                 (cdr
                  (assq
                   'match
                   ac-source-sly-simple)))
                (eq
                 candidates
                 (funcall
                  (cdr
                   (assq
                    'match
                    ac-source-sly-fuzzy))
                  "ignored"
                  candidates))
                candidates))"##;
    let expect = expect!["OK (t t t t t t (one two))"];

    assert_ac_sly_parity(elisp_form, expect);
}
