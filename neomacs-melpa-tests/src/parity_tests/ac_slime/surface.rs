use expect_test::expect;

use super::assert_ac_slime_parity;

#[test]
fn ac_slime_internal_state_and_source_variable_documentation_match() {
    let elisp_form = r##"(list
               ac-slime-current-doc
               (get
                'ac-slime-current-doc
                'variable-documentation)
               (default-boundp
                'ac-slime-current-doc)
               (get
                'ac-source-slime-fuzzy
                'variable-documentation)
               (get
                'ac-source-slime-simple
                'variable-documentation)
               (default-value
                'ac-source-slime-fuzzy)
               (default-value
                'ac-source-slime-simple))"##;
    let expect = expect![[
        r#"OK (nil "Holds slime docstring for current symbol." t "Source for fuzzy slime completion." "Source for slime completion." ((init . ac-slime-init) (candidates . ac-source-slime-fuzzy-candidates) (candidate-face . ac-slime-menu-face) (selection-face . ac-slime-selection-face) (prefix . slime-symbol-start-pos) (symbol . "l") (match lambda (prefix candidates) candidates) (document . ac-slime-documentation)) ((init . ac-slime-init) (candidates . ac-source-slime-simple-candidates) (candidate-face . ac-slime-menu-face) (selection-face . ac-slime-selection-face) (prefix . slime-symbol-start-pos) (symbol . "l") (document . ac-slime-documentation) (match . ac-source-slime-case-correcting-completions)))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_packaged_source_descriptor_autoload_and_readme_assets_have_exact_hashes() {
    let elisp_form = r##"(let ((root
                    (file-name-directory
                     (symbol-file
                      'ac-slime-init
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
                '("ac-slime.el"
                  "ac-slime-pkg.el"
                  "ac-slime-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ac-slime.el" 4511 "899eeecd4dda81a7ccbae8691c20749c5a97719d22792258b6041ecfc88265a2") ("ac-slime-pkg.el" 468 "6a3a7991955dbd81c852c1c3bbb42aad2d6cf5ef943dadde6f4167ccf3abef0b") ("ac-slime-autoloads.el" 1773 "59307da5dd48a61b4b9403b5610368bc26bc6765d468000b26fb86fd85dbcfa0") ("README-elpa" 221 "edb084bdbcfdf4964caedb3c180f4913de5490f91b0dd74949fc164d035759fd"))"#
    ]];

    assert_ac_slime_parity(elisp_form, expect);
}

#[test]
fn ac_slime_source_descriptors_keep_distinct_match_callbacks_and_shared_live_symbols() {
    let elisp_form = r##"(let ((candidates
                    (list
                     'one
                     'two)))
               (list
                (eq
                 (cdr
                  (assq
                   'init
                   ac-source-slime-fuzzy))
                 (cdr
                  (assq
                   'init
                   ac-source-slime-simple)))
                (eq
                 (cdr
                  (assq
                   'prefix
                   ac-source-slime-fuzzy))
                 (cdr
                  (assq
                   'prefix
                   ac-source-slime-simple)))
                (eq
                 (cdr
                  (assq
                   'document
                   ac-source-slime-fuzzy))
                 (cdr
                  (assq
                   'document
                   ac-source-slime-simple)))
                (functionp
                 (cdr
                  (assq
                   'match
                   ac-source-slime-fuzzy)))
                (symbolp
                 (cdr
                  (assq
                   'match
                   ac-source-slime-simple)))
                (eq
                 candidates
                 (funcall
                  (cdr
                   (assq
                    'match
                    ac-source-slime-fuzzy))
                  "ignored"
                  candidates))
                candidates))"##;
    let expect = expect!["OK (t t t t t t (one two))"];

    assert_ac_slime_parity(elisp_form, expect);
}
