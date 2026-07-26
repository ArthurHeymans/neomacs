use expect_test::expect;

use super::assert_ac_sly_autoload_parity;

#[test]
fn ac_sly_fresh_autoload_defines_faces_sources_setup_and_no_runtime_state() {
    let elisp_form = r##"(list
               (featurep
                'ac-sly)
               (featurep
                'ac-sly-autoloads)
               (boundp
                'ac-sly-show-flags)
               (boundp
                'ac-sly-current-doc)
               (boundp
                'ac-source-sly-fuzzy)
               (boundp
                'ac-source-sly-simple)
               (facep
                'ac-sly-menu-face)
               (facep
                'ac-sly-selection-face)
               (fboundp
                'set-up-sly-ac)
               (autoloadp
                (symbol-function
                 'set-up-sly-ac))
               (get
                'ac-sly
                'custom-loads)
               (gethash
                "ac-s"
                definition-prefixes))"##;
    let expect = expect![[
        r#"OK (nil t nil nil t t [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] t t nil ("ac-sly" "ac-sly"))"#
    ]];

    assert_ac_sly_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fresh_autoload_preserves_exact_source_values_and_documentation() {
    let elisp_form = r##"(list
               ac-source-sly-fuzzy
               (get
                'ac-source-sly-fuzzy
                'variable-documentation)
               ac-source-sly-simple
               (get
                'ac-source-sly-simple
                'variable-documentation))"##;
    let expect = expect![[
        r#"OK (((init . ac-sly-init) (candidates . ac-source-sly-fuzzy-candidates) (candidate-face . ac-sly-menu-face) (selection-face . ac-sly-selection-face) (prefix . sly-symbol-start-pos) (symbol . "l") (match lambda (prefix candidates) candidates) (document . ac-sly-documentation)) "Source for fuzzy slime completion." ((init . ac-sly-init) (candidates . ac-source-sly-simple-candidates) (candidate-face . ac-sly-menu-face) (selection-face . ac-sly-selection-face) (prefix . sly-symbol-start-pos) (symbol . "l") (document . ac-sly-documentation) (match . ac-source-sly-case-correcting-completions)) "Source for slime completion.")"#
    ]];

    assert_ac_sly_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fresh_autoload_preserves_exact_face_specs_and_documentation() {
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
              '(ac-sly-menu-face
                ac-sly-selection-face))"##;
    let expect = expect![[
        r#"OK ((ac-sly-menu-face ((t (:inherit ac-candidate-face))) "Face for slime candidate menu." (ac-sly-menu-face custom-face)) (ac-sly-selection-face ((t (:inherit ac-selection-face))) "Face for the slime selected candidate." (ac-sly-selection-face custom-face)))"#
    ]];

    assert_ac_sly_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fresh_autoload_defines_setup_with_exact_interactive_autoload_object() {
    let elisp_form = r##"(list
               (fboundp
                'set-up-sly-ac)
               (autoloadp
                (symbol-function
                 'set-up-sly-ac))
               (copy-tree
                (symbol-function
                 'set-up-sly-ac))
               (interactive-form
                'set-up-sly-ac)
               (symbol-file
                'set-up-sly-ac
                'defun))"##;
    let expect = expect![[
        r#"OK (t t (autoload "ac-sly" "Add an optionally-fuzzy slime completion source to `ac-sources'.\n\n(fn &optional FUZZY)" t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ac-sly/20170728.1027/home/.emacs.d/elpa/ac-sly-20170728.1027/ac-sly.el")"#
    ]];

    assert_ac_sly_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fresh_autoload_does_not_define_fuzzy_candidates() {
    let elisp_form = r##"(list
               (featurep
                'ac-sly)
               (featurep
                'ac-sly-autoloads)
               (fboundp
                'ac-source-sly-fuzzy-candidates))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_sly_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fresh_autoload_does_not_define_simple_candidates() {
    let elisp_form = r##"(list
               (featurep
                'ac-sly)
               (featurep
                'ac-sly-autoloads)
               (fboundp
                'ac-source-sly-simple-candidates))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_sly_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fresh_autoload_does_not_define_case_correcting_completions() {
    let elisp_form = r##"(list
               (featurep
                'ac-sly)
               (featurep
                'ac-sly-autoloads)
               (fboundp
                'ac-source-sly-case-correcting-completions))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_sly_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fresh_autoload_does_not_define_documentation() {
    let elisp_form = r##"(list
               (featurep
                'ac-sly)
               (featurep
                'ac-sly-autoloads)
               (fboundp
                'ac-sly-documentation))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_sly_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_sly_fresh_autoload_does_not_define_init() {
    let elisp_form = r##"(list
               (featurep
                'ac-sly)
               (featurep
                'ac-sly-autoloads)
               (fboundp
                'ac-sly-init))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_sly_autoload_parity(elisp_form, expect);
}
