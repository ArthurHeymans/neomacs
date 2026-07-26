use expect_test::expect;

use super::assert_ac_skk_autoload_parity;

#[test]
fn ac_skk_fresh_autoload_registers_prefix_and_leaves_runtime_state_unbound() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (boundp 'ac-skk-special-sources)
               (boundp 'ac-source-skk)
               (boundp 'ac-skk-selected-candidate)
               (boundp 'ac-source-skk-hiracomp)
               (boundp 'ac-skk-enable)
               (boundp 'ac-skk-save-variable)
               (boundp 'ac-skk-ac-trigger-commands-orig)
               (boundp 'ac-skk-ac-sources-orig)
               (get 'ac-skk 'custom-loads)
               (gethash "ac-s" definition-prefixes))"##;
    let expect = expect![[r#"OK (nil t nil nil nil nil nil nil nil nil nil ("ac-skk" "ac-skk"))"#]];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_defines_enable_as_an_interactive_autoload() {
    let elisp_form = r##"(list
               (fboundp 'ac-skk-enable)
               (autoloadp (symbol-function 'ac-skk-enable))
               (copy-tree (symbol-function 'ac-skk-enable))
               (interactive-form 'ac-skk-enable)
               (symbol-file 'ac-skk-enable 'defun))"##;
    let expect = expect![[
        r#"OK (t t (autoload "ac-skk" nil t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ac-skk/20141230.119/home/.emacs.d/elpa/ac-skk-20141230.119/ac-skk.el")"#
    ]];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_defines_disable_as_an_interactive_autoload() {
    let elisp_form = r##"(list
               (fboundp 'ac-skk-disable)
               (autoloadp (symbol-function 'ac-skk-disable))
               (copy-tree (symbol-function 'ac-skk-disable))
               (interactive-form 'ac-skk-disable)
               (symbol-file 'ac-skk-disable 'defun))"##;
    let expect = expect![[
        r#"OK (t t (autoload "ac-skk" nil t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ac-skk/20141230.119/home/.emacs.d/elpa/ac-skk-20141230.119/ac-skk.el")"#
    ]];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_defines_toggle_as_an_interactive_autoload() {
    let elisp_form = r##"(list
               (fboundp 'ac-skk-toggle)
               (autoloadp (symbol-function 'ac-skk-toggle))
               (copy-tree (symbol-function 'ac-skk-toggle))
               (interactive-form 'ac-skk-toggle)
               (symbol-file 'ac-skk-toggle 'defun))"##;
    let expect = expect![[
        r#"OK (t t (autoload "ac-skk" nil t nil) (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ac-skk/20141230.119/home/.emacs.d/elpa/ac-skk-20141230.119/ac-skk.el")"#
    ]];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_prefix() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-prefix))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_make_cand() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-make-cand))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_make_cand_list() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-make-cand-list))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_candidates() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-candidates))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_kakutei() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-kakutei))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_henkan_forward() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-henkan-forward))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_start_henkan() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-start-henkan))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_prefix_hiracomp() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-prefix-hiracomp))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_hiracomp_candidates() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-hiracomp-candidates))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_hiracomp_mes() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-hiracomp-mes))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_setup() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-setup))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}

#[test]
fn ac_skk_fresh_autoload_does_not_define_cleanup() {
    let elisp_form = r##"(list
               (featurep 'ac-skk)
               (featurep 'ac-skk-autoloads)
               (fboundp 'ac-skk-cleanup))"##;
    let expect = expect!["OK (nil t nil)"];

    assert_ac_skk_autoload_parity(elisp_form, expect);
}
