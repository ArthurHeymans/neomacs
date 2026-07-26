use expect_test::expect;

use super::assert_ace_isearch_autoload_parity;

#[test]
fn ace_isearch_regexp_helper_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch--isearch-regexp-function)
               (symbol-file
                'ace-isearch--isearch-regexp-function
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_switch_function_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch-switch-function)
               (symbol-file 'ace-isearch-switch-function 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_two_switch_function_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch-2-switch-function)
               (symbol-file 'ace-isearch-2-switch-function 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_fboundp_helper_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch--fboundp)
               (symbol-file 'ace-isearch--fboundp 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_make_backend_helper_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch--make-ace-jump-or-avy)
               (symbol-file
                'ace-isearch--make-ace-jump-or-avy
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_make_two_backend_helper_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch-2--make-ace-jump-or-avy)
               (symbol-file
                'ace-isearch-2--make-ace-jump-or-avy
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_helm_occur_adapter_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch-helm-occur-from-isearch)
               (symbol-file
                'ace-isearch-helm-occur-from-isearch
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_helm_swoop_adapter_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch-helm-swoop-from-isearch)
               (symbol-file
                'ace-isearch-helm-swoop-from-isearch
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_swiper_adapter_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch-swiper-from-isearch)
               (symbol-file
                'ace-isearch-swiper-from-isearch
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_consult_adapter_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch-consult-line-from-isearch)
               (symbol-file
                'ace-isearch-consult-line-from-isearch
                'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_turn_on_helper_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch--turn-on)
               (symbol-file 'ace-isearch--turn-on 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_function_alias_is_not_defined_by_the_autoload_file() {
    let elisp_form = r##"(list
               (fboundp 'ace-isearch-switch-submode)
               (symbol-file 'ace-isearch-switch-submode 'defun))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_two_function_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-2-function)
               (symbol-file 'ace-isearch-2-function 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_disable_message_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp
                'ace-isearch-disable-isearch-function-from-isearch-message)
               (symbol-file
                'ace-isearch-disable-isearch-function-from-isearch-message
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_transition_function_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-function-from-isearch)
               (symbol-file
                'ace-isearch-function-from-isearch
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_lighter_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-lighter)
               (symbol-file 'ace-isearch-lighter 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jump_basis_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-jump-based-on-one-char)
               (symbol-file
                'ace-isearch-jump-based-on-one-char
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jump_delay_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-jump-delay)
               (symbol-file 'ace-isearch-jump-delay 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_function_delay_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-func-delay)
               (symbol-file 'ace-isearch-func-delay 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_input_length_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-input-length)
               (symbol-file 'ace-isearch-input-length 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_use_jump_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-use-jump)
               (symbol-file 'ace-isearch-use-jump 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_use_transition_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-use-function-from-isearch)
               (symbol-file
                'ace-isearch-use-function-from-isearch
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_fallback_function_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-fallback-function)
               (symbol-file
                'ace-isearch-fallback-function
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_use_fallback_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-use-fallback-function)
               (symbol-file
                'ace-isearch-use-fallback-function
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_evil_mode_option_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-on-evil-mode)
               (symbol-file 'ace-isearch-on-evil-mode 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_ace_jump_function_list_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch--ace-jump-function-list)
               (symbol-file
                'ace-isearch--ace-jump-function-list
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_ace_jump_two_function_list_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch--ace-jump-2-function-list)
               (symbol-file
                'ace-isearch--ace-jump-2-function-list
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_avy_function_list_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch--avy-function-list)
               (symbol-file
                'ace-isearch--avy-function-list
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_avy_two_function_list_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch--avy-2-function-list)
               (symbol-file
                'ace-isearch--avy-2-function-list
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_combined_function_list_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch--function-list)
               (symbol-file 'ace-isearch--function-list 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_combined_two_function_list_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-2--function-list)
               (symbol-file
                'ace-isearch-2--function-list
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_backend_marker_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch--ace-jump-or-avy)
               (symbol-file
                'ace-isearch--ace-jump-or-avy
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_submode_variable_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-submode)
               (symbol-file 'ace-isearch-submode 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_jump_delay_variable_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-input-idle-jump-delay)
               (symbol-file
                'ace-isearch-input-idle-jump-delay
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_function_delay_variable_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-input-idle-func-delay)
               (symbol-file
                'ace-isearch-input-idle-func-delay
                'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_use_jump_variable_is_not_bound_by_the_autoload_file() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch-use-ace-jump)
               (symbol-file 'ace-isearch-use-ace-jump 'defvar))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ace_isearch_autoload_parity(elisp_form, expect);
}
