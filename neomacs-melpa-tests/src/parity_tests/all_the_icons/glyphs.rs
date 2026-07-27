use expect_test::expect;

use super::assert_all_the_icons_parity;

#[test]
fn alltheicon_glyph_carries_exact_character_family_height_and_raise() {
    let elisp_form = r##"(let ((icon
                (all-the-icons-alltheicon
                 "rust" :height 1.5 :v-adjust 0.25
                 :face 'all-the-icons-maroon)))
         (list (string-to-list icon)
               (text-properties-at 0 icon)
               (all-the-icons-icon-family icon)))"##;
    let expect = expect![[
        r#"OK ((59692) (face #1=(:family "all-the-icons" :height 1.7999999999999998 :inherit all-the-icons-maroon) font-lock-face #1# display (raise 0.3) rear-nonsticky t) "all-the-icons")"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn fileicon_glyph_carries_exact_character_family_height_and_raise() {
    let elisp_form = r##"(let ((icon
                (all-the-icons-fileicon
                 "elisp" :height 0.75 :v-adjust -0.1
                 :face 'all-the-icons-purple)))
         (list (string-to-list icon)
               (text-properties-at 0 icon)
               (all-the-icons-icon-family icon)))"##;
    let expect = expect![[
        r#"OK ((59686) (face #1=(:family "file-icons" :height 0.8999999999999999 :inherit all-the-icons-purple) font-lock-face #1# display (raise -0.12) rear-nonsticky t) "file-icons")"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn faicon_glyph_carries_exact_character_family_height_and_raise() {
    let elisp_form = r##"(let ((icon
                (all-the-icons-faicon
                 "cogs" :height 2 :v-adjust 0
                 :face 'all-the-icons-silver)))
         (list (string-to-list icon)
               (text-properties-at 0 icon)
               (all-the-icons-icon-family icon)))"##;
    let expect = expect![[
        r#"OK ((61573) (face #1=(:family "FontAwesome" :height 2.4 :inherit all-the-icons-silver) font-lock-face #1# display (raise 0.0) rear-nonsticky t) "FontAwesome")"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn octicon_glyph_carries_exact_character_family_height_and_raise() {
    let elisp_form = r##"(let ((icon
                (all-the-icons-octicon
                 "file-binary" :height 1.25 :v-adjust 0
                 :face 'all-the-icons-blue)))
         (list (string-to-list icon)
               (text-properties-at 0 icon)
               (all-the-icons-icon-family icon)))"##;
    let expect = expect![[
        r#"OK ((61588) (face #1=(:family "github-octicons" :height 1.5 :inherit all-the-icons-blue) font-lock-face #1# display (raise 0.0) rear-nonsticky t) "github-octicons")"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn wicon_glyph_carries_exact_character_family_height_and_raise() {
    let elisp_form = r##"(let ((icon
                (all-the-icons-wicon
                 "tornado" :height 0.5 :v-adjust 0.4
                 :face 'all-the-icons-blue)))
         (list (string-to-list icon)
               (text-properties-at 0 icon)
               (all-the-icons-icon-family icon)))"##;
    let expect = expect![[
        r#"OK ((61526) (face #1=(:family "Weather Icons" :height 0.6 :inherit all-the-icons-blue) font-lock-face #1# display (raise 0.48) rear-nonsticky t) "Weather Icons")"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn material_glyph_carries_exact_character_family_height_and_raise() {
    let elisp_form = r##"(let ((icon
                (all-the-icons-material
                 "settings" :height 1.1 :v-adjust -0.3
                 :face 'all-the-icons-yellow)))
         (list (string-to-list icon)
               (text-properties-at 0 icon)
               (all-the-icons-icon-family icon)))"##;
    let expect = expect![[
        r#"OK ((59576) (face #1=(:family "Material Icons" :height 1.32 :inherit all-the-icons-yellow) font-lock-face #1# display (raise -0.36) rear-nonsticky t) "Material Icons")"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_scaling_adjustment_and_color_switch_compose_in_real_icons() {
    let elisp_form = r##"(let ((all-the-icons-scale-factor 2)
               (all-the-icons-fileicon-scale-factor 1.5)
               (all-the-icons-default-adjust -0.25)
               (all-the-icons-default-fileicon-adjust 0.05))
         (let ((colored
                (all-the-icons-fileicon
                 "elisp" :height 0.5
                 :face 'all-the-icons-blue))
               (all-the-icons-color-icons nil)
               plain)
           (setq plain
                 (all-the-icons-fileicon
                  "elisp" :height 0.5
                  :face 'all-the-icons-red))
           (list
            (text-properties-at 0 colored)
            (text-properties-at 0 plain))))"##;
    let expect = expect![[
        r#"OK ((face #1=(:family "file-icons" :height 1.5 :inherit all-the-icons-blue) font-lock-face #1# display (raise -0.6000000000000001) rear-nonsticky t) (face #2=(:family "file-icons" :height 1.5) font-lock-face #2# display (raise -0.6000000000000001) rear-nonsticky t))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_unknown_names_report_family_specific_errors() {
    let elisp_form = r##"(mapcar
         (lambda (family)
           (let ((function
                  (intern
                   (format "all-the-icons-%s" family))))
             (condition-case error-data
                 (funcall function "definitely-not-an-icon")
               (error error-data))))
         all-the-icons-font-families)"##;
    let expect = expect![[
        r#"OK ((error "Unable to find icon with name ‘definitely-not-an-icon’ in icon set ‘material’") (error "Unable to find icon with name ‘definitely-not-an-icon’ in icon set ‘wicon’") (error "Unable to find icon with name ‘definitely-not-an-icon’ in icon set ‘octicon’") (error "Unable to find icon with name ‘definitely-not-an-icon’ in icon set ‘faicon’") (error "Unable to find icon with name ‘definitely-not-an-icon’ in icon set ‘fileicon’") (error "Unable to find icon with name ‘definitely-not-an-icon’ in icon set ‘alltheicon’"))"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}

#[test]
fn all_the_icons_definition_macro_builds_a_complete_custom_family_at_runtime() {
    let elisp_form = r##"(progn
         (setq all-the-icons-test-data
               '(("rocket" . "R") ("planet" . "P")))
         (eval
          '(all-the-icons-define-icon
            test-family all-the-icons-test-data
            "Test Icons" "test-font"))
         (let ((icon
                (all-the-icons-test-family
                 "planet" :height 2 :v-adjust 0.5
                 :face 'bold)))
           (list
            (memq 'test-family all-the-icons-font-families)
            (member "test-font.ttf" all-the-icons-font-names)
            (all-the-icons-test-family-family)
            (all-the-icons-test-family-data)
            (string-to-list icon)
            (text-properties-at 0 icon)
            (help-function-arglist
             'all-the-icons-test-family t)
            (commandp 'all-the-icons-insert-test-family))))"##;
    let expect = expect![[
        r#"OK ((test-family material wicon octicon faicon fileicon alltheicon) ("test-font.ttf" "material-design-icons.ttf" "weathericons.ttf" "octicons.ttf" "fontawesome.ttf" "file-icons.ttf" "all-the-icons.ttf") "Test Icons" (("rocket" . "R") ("planet" . "P")) (80) (face #1=(:family "Test Icons" :height 2.4 :inherit bold) font-lock-face #1# display (raise 0.6) rear-nonsticky t) (icon-name &rest args) t)"#
    ]];
    assert_all_the_icons_parity(elisp_form, expect);
}
