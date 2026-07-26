use expect_test::expect;

use super::assert_ac_slime_autoload_parity;

#[test]
fn ac_slime_fresh_autoload_defines_faces_sources_setup_and_no_runtime_state() {
    let elisp_form = r##"(list
               (featurep
                'ac-slime)
               (featurep
                'ac-slime-autoloads)
               (boundp
                'ac-slime-show-flags)
               (boundp
                'ac-slime-current-doc)
               (boundp
                'ac-source-slime-fuzzy)
               (boundp
                'ac-source-slime-simple)
               (facep
                'ac-slime-menu-face)
               (facep
                'ac-slime-selection-face)
               (fboundp
                'set-up-slime-ac)
               (autoloadp
                (symbol-function
                 'set-up-slime-ac))
               (get
                'ac-slime
                'custom-loads)
               (gethash
                "ac-s"
                definition-prefixes))"##;
    let expect = expect![[
        r#"OK (nil t nil nil t t [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] t t nil ("ac-slime" "ac-slime"))"#
    ]];

    assert_ac_slime_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_slime_fresh_autoload_preserves_exact_source_values_and_documentation() {
    let elisp_form = r##"(list
               ac-source-slime-fuzzy
               (get
                'ac-source-slime-fuzzy
                'variable-documentation)
               ac-source-slime-simple
               (get
                'ac-source-slime-simple
                'variable-documentation))"##;
    let expect = expect![[
        r#"OK (((init . ac-slime-init) (candidates . ac-source-slime-fuzzy-candidates) (candidate-face . ac-slime-menu-face) (selection-face . ac-slime-selection-face) (prefix . slime-symbol-start-pos) (symbol . "l") (match lambda (prefix candidates) candidates) (document . ac-slime-documentation)) "Source for fuzzy slime completion." ((init . ac-slime-init) (candidates . ac-source-slime-simple-candidates) (candidate-face . ac-slime-menu-face) (selection-face . ac-slime-selection-face) (prefix . slime-symbol-start-pos) (symbol . "l") (document . ac-slime-documentation) (match . ac-source-slime-case-correcting-completions)) "Source for slime completion.")"#
    ]];

    assert_ac_slime_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_slime_fresh_autoload_preserves_exact_face_specs_and_documentation() {
    let elisp_form = r##"(mapcar
              (lambda (face)
                (list
                 face
                 (get
                  face
                  'face-defface-spec)
                 (get
                  face
                  'face-documentation)
                 (assq
                  face
                  (get
                   'auto-complete
                   'custom-group))))
              '(ac-slime-menu-face
                ac-slime-selection-face))"##;
    let expect = expect![[
        r#"OK ((ac-slime-menu-face ((t (:inherit ac-candidate-face))) "Face for slime candidate menu." (ac-slime-menu-face custom-face)) (ac-slime-selection-face ((t (:inherit ac-selection-face))) "Face for the slime selected candidate." (ac-slime-selection-face custom-face)))"#
    ]];

    assert_ac_slime_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_slime_fresh_autoload_defines_setup_with_exact_interactive_autoload_object() {
    let elisp_form = r##"(list
               (fboundp
                'set-up-slime-ac)
               (autoloadp
                (symbol-function
                 'set-up-slime-ac))
               (copy-tree
                (symbol-function
                 'set-up-slime-ac))
               (interactive-form
                'set-up-slime-ac)
               (symbol-file
                'set-up-slime-ac
                'defun))"##;
    let expect = expect![[
        r#"OK (t t (autoload "ac-slime" "Add an optionally FUZZY slime completion source to `ac-sources'.\n\n(fn &optional FUZZY)" t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ac-slime/20171027.2100/home/.emacs.d/elpa/ac-slime-20171027.2100/ac-slime.el")"#
    ]];

    assert_ac_slime_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_slime_fresh_autoload_does_not_define_fuzzy_candidates() {
    let elisp_form = r##"(list
               (featurep
                'ac-slime)
               (featurep
                'ac-slime-autoloads)
               (fboundp
                'ac-source-slime-fuzzy-candidates))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_slime_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_slime_fresh_autoload_does_not_define_simple_candidates() {
    let elisp_form = r##"(list
               (featurep
                'ac-slime)
               (featurep
                'ac-slime-autoloads)
               (fboundp
                'ac-source-slime-simple-candidates))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_slime_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_slime_fresh_autoload_does_not_define_case_correcting_completions() {
    let elisp_form = r##"(list
               (featurep
                'ac-slime)
               (featurep
                'ac-slime-autoloads)
               (fboundp
                'ac-source-slime-case-correcting-completions))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_slime_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_slime_fresh_autoload_does_not_define_documentation() {
    let elisp_form = r##"(list
               (featurep
                'ac-slime)
               (featurep
                'ac-slime-autoloads)
               (fboundp
                'ac-slime-documentation))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_slime_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_slime_fresh_autoload_does_not_define_init() {
    let elisp_form = r##"(list
               (featurep
                'ac-slime)
               (featurep
                'ac-slime-autoloads)
               (fboundp
                'ac-slime-init))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_slime_autoload_parity(elisp_form, expect);
}
