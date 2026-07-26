use expect_test::expect;

use super::assert_ac_skk_parity;

#[test]
fn ac_skk_prefix_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-prefix t)
               (interactive-form 'ac-skk-prefix)
               (documentation 'ac-skk-prefix)
               (file-name-nondirectory
                (symbol-file 'ac-skk-prefix 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_make_cand_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-make-cand t)
               (interactive-form 'ac-skk-make-cand)
               (documentation 'ac-skk-make-cand)
               (file-name-nondirectory
                (symbol-file 'ac-skk-make-cand 'defun)))"##;
    let expect = expect![[r#"OK ((cand action midasi count) nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_make_cand_list_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-make-cand-list t)
               (interactive-form 'ac-skk-make-cand-list)
               (documentation 'ac-skk-make-cand-list)
               (file-name-nondirectory
                (symbol-file 'ac-skk-make-cand-list 'defun)))"##;
    let expect = expect![[r#"OK ((midasi prog-list) nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_candidates_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-candidates t)
               (interactive-form 'ac-skk-candidates)
               (documentation 'ac-skk-candidates)
               (file-name-nondirectory
                (symbol-file 'ac-skk-candidates 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_kakutei_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-kakutei t)
               (interactive-form 'ac-skk-kakutei)
               (documentation 'ac-skk-kakutei)
               (file-name-nondirectory
                (symbol-file 'ac-skk-kakutei 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_henkan_forward_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-henkan-forward t)
               (interactive-form 'ac-skk-henkan-forward)
               (documentation 'ac-skk-henkan-forward)
               (file-name-nondirectory
                (symbol-file 'ac-skk-henkan-forward 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_start_henkan_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-start-henkan t)
               (interactive-form 'ac-skk-start-henkan)
               (documentation 'ac-skk-start-henkan)
               (file-name-nondirectory
                (symbol-file 'ac-skk-start-henkan 'defun)))"##;
    let expect = expect![[r#"OK ((count) nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_prefix_hiracomp_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-prefix-hiracomp t)
               (interactive-form 'ac-skk-prefix-hiracomp)
               (documentation 'ac-skk-prefix-hiracomp)
               (file-name-nondirectory
                (symbol-file 'ac-skk-prefix-hiracomp 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_hiracomp_candidates_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-hiracomp-candidates t)
               (interactive-form 'ac-skk-hiracomp-candidates)
               (documentation 'ac-skk-hiracomp-candidates)
               (file-name-nondirectory
                (symbol-file 'ac-skk-hiracomp-candidates 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_hiracomp_mes_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-hiracomp-mes t)
               (interactive-form 'ac-skk-hiracomp-mes)
               (documentation 'ac-skk-hiracomp-mes)
               (file-name-nondirectory
                (symbol-file 'ac-skk-hiracomp-mes 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_enable_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-enable t)
               (interactive-form 'ac-skk-enable)
               (documentation 'ac-skk-enable)
               (file-name-nondirectory
                (symbol-file 'ac-skk-enable 'defun)))"##;
    let expect = expect![[r#"OK (nil (interactive nil) nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_disable_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-disable t)
               (interactive-form 'ac-skk-disable)
               (documentation 'ac-skk-disable)
               (file-name-nondirectory
                (symbol-file 'ac-skk-disable 'defun)))"##;
    let expect = expect![[r#"OK (nil (interactive nil) nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_toggle_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-toggle t)
               (interactive-form 'ac-skk-toggle)
               (documentation 'ac-skk-toggle)
               (file-name-nondirectory
                (symbol-file 'ac-skk-toggle 'defun)))"##;
    let expect = expect![[r#"OK (nil (interactive nil) nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_setup_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-setup t)
               (interactive-form 'ac-skk-setup)
               (documentation 'ac-skk-setup)
               (file-name-nondirectory
                (symbol-file 'ac-skk-setup 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}

#[test]
fn ac_skk_cleanup_callable_metadata_matches() {
    let elisp_form = r##"(list
               (help-function-arglist 'ac-skk-cleanup t)
               (interactive-form 'ac-skk-cleanup)
               (documentation 'ac-skk-cleanup)
               (file-name-nondirectory
                (symbol-file 'ac-skk-cleanup 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ac-skk.el")"#]];

    assert_ac_skk_parity(elisp_form, expect);
}
