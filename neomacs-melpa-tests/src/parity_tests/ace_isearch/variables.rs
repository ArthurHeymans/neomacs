use expect_test::expect;

use super::{
    assert_ace_isearch_autoload_signal_parity, assert_ace_isearch_avy_backend_parity,
    assert_ace_isearch_parity, assert_ace_isearch_signal_parity,
    assert_ace_isearch_with_prelude_parity,
};

#[test]
fn ace_isearch_ace_jump_function_list_default_metadata_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch--ace-jump-function-list
               (get 'ace-isearch--ace-jump-function-list 'variable-documentation)
               (file-name-nondirectory
                (symbol-file 'ace-isearch--ace-jump-function-list 'defvar)))"##;
    let expect =
        expect![[r#"OK (("ace-jump-word-mode" "ace-jump-char-mode") nil "ace-isearch.el")"#]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_ace_jump_two_function_list_default_metadata_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch--ace-jump-2-function-list
               (get 'ace-isearch--ace-jump-2-function-list
                    'variable-documentation)
               (file-name-nondirectory
                (symbol-file 'ace-isearch--ace-jump-2-function-list 'defvar)))"##;
    let expect = expect![[r#"OK (nil nil "ace-isearch.el")"#]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_avy_function_list_default_metadata_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch--avy-function-list
               (get 'ace-isearch--avy-function-list 'variable-documentation)
               (file-name-nondirectory
                (symbol-file 'ace-isearch--avy-function-list 'defvar)))"##;
    let expect = expect![[
        r#"OK (("avy-goto-word-1" "avy-goto-subword-1" "avy-goto-word-or-subword-1" "avy-goto-char") nil "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_avy_two_function_list_default_metadata_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch--avy-2-function-list
               (get 'ace-isearch--avy-2-function-list 'variable-documentation)
               (file-name-nondirectory
                (symbol-file 'ace-isearch--avy-2-function-list 'defvar)))"##;
    let expect = expect![[
        r#"OK (("avy-goto-char-2" "avy-goto-char-2-above" "avy-goto-char-2-below") nil "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_combined_function_list_default_metadata_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch--function-list
               (get 'ace-isearch--function-list 'variable-documentation)
               (file-name-nondirectory
                (symbol-file 'ace-isearch--function-list 'defvar)))"##;
    let expect = expect![[
        r#"OK (("ace-jump-word-mode" "ace-jump-char-mode" "avy-goto-word-1" "avy-goto-subword-1" "avy-goto-word-or-subword-1" "avy-goto-char") "List of functions for jumping using 1 character." "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_combined_two_function_list_default_metadata_and_source_match() {
    let elisp_form = r##"(list
               ace-isearch-2--function-list
               (get 'ace-isearch-2--function-list 'variable-documentation)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-2--function-list 'defvar)))"##;
    let expect = expect![[
        r#"OK (("avy-goto-char-2" "avy-goto-char-2-above" "avy-goto-char-2-below") "List of functions for jumping using 2 characters." "ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_backend_marker_default_metadata_and_source_match() {
    let elisp_form = r##"(list
               (boundp 'ace-isearch--ace-jump-or-avy)
               (default-boundp 'ace-isearch--ace-jump-or-avy)
               (get 'ace-isearch--ace-jump-or-avy 'variable-documentation)
               (symbol-file 'ace-isearch--ace-jump-or-avy 'defvar))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_unbound_backend_marker_remains_void_inside_a_lexical_let_caller() {
    let elisp_form = r##"(let ((ace-isearch--ace-jump-or-avy 'ace-jump)
                   calls)
               (cl-letf (((symbol-function 'ace-jump-mode-pop-mark)
                          (lambda ()
                            (push 'ace calls)
                            'ace-result)))
                 (list
                  (ace-isearch-pop-mark)
                  (nreverse calls))))"##;
    let expect = expect!["ERR (void-variable ace-isearch--ace-jump-or-avy)"];

    assert_ace_isearch_signal_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_switch_submode_function_alias_metadata_match() {
    let elisp_form = r##"(list
               (indirect-function 'ace-isearch-switch-submode)
               (indirect-function 'ace-isearch-switch-function)
               (get 'ace-isearch-switch-submode 'byte-obsolete-info)
               (help-function-arglist 'ace-isearch-switch-submode t)
               (interactive-form 'ace-isearch-switch-submode)
               (symbol-file
                'ace-isearch-switch-submode
                'defun))"##;
    let expect = expect![[
        r#"OK (#1=#[nil ((let ((func (completing-read (format "Function for ace-isearch (current is %s): " ace-isearch-function) ace-isearch--function-list nil t))) (setq ace-isearch-function (intern-soft func)) (ace-isearch--make-ace-jump-or-avy) (message "Function for ace-isearch is set to %s." func))) (ace-isearch--ace-jump-or-avy t) nil nil nil] #1# (ace-isearch-switch-function nil "0.1.3") nil (interactive nil) "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-isearch/20220809.1748/home/.emacs.d/elpa/ace-isearch-20220809.1748/ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_switch_submode_calls_the_current_target_function_cell() {
    let elisp_form = r##"(let (calls)
               (cl-letf (((symbol-function 'ace-isearch-switch-function)
                          (lambda ()
                            (push 'target calls)
                            'target-result)))
                 (list
                  (ace-isearch-switch-submode)
                  (nreverse calls))))"##;
    let expect = expect!["OK (target-result (target))"];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_submode_variable_alias_metadata_match() {
    let elisp_form = r##"(list
               (indirect-variable 'ace-isearch-submode)
               ace-isearch-submode
               (get 'ace-isearch-submode 'byte-obsolete-variable)
               (symbol-file 'ace-isearch-submode 'defvar))"##;
    let expect = expect![[
        r#"OK (ace-isearch-function ace-jump-word-mode (ace-isearch-function nil "0.1.3") "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-isearch/20220809.1748/home/.emacs.d/elpa/ace-isearch-20220809.1748/ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_jump_delay_variable_alias_metadata_match() {
    let elisp_form = r##"(list
               (indirect-variable 'ace-isearch-input-idle-jump-delay)
               ace-isearch-input-idle-jump-delay
               (get 'ace-isearch-input-idle-jump-delay 'byte-obsolete-variable)
               (symbol-file 'ace-isearch-input-idle-jump-delay 'defvar))"##;
    let expect = expect![[
        r#"OK (ace-isearch-jump-delay 0.3 (ace-isearch-jump-delay nil "0.1.3") "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-isearch/20220809.1748/home/.emacs.d/elpa/ace-isearch-20220809.1748/ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_function_delay_variable_alias_metadata_match() {
    let elisp_form = r##"(list
               (indirect-variable 'ace-isearch-input-idle-func-delay)
               ace-isearch-input-idle-func-delay
               (get 'ace-isearch-input-idle-func-delay 'byte-obsolete-variable)
               (symbol-file 'ace-isearch-input-idle-func-delay 'defvar))"##;
    let expect = expect![[
        r#"OK (ace-isearch-func-delay 0.0 (ace-isearch-func-delay nil "0.1.3") "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-isearch/20220809.1748/home/.emacs.d/elpa/ace-isearch-20220809.1748/ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_use_jump_variable_alias_metadata_match() {
    let elisp_form = r##"(list
               (indirect-variable 'ace-isearch-use-ace-jump)
               ace-isearch-use-ace-jump
               (get 'ace-isearch-use-ace-jump 'byte-obsolete-variable)
               (symbol-file 'ace-isearch-use-ace-jump 'defvar))"##;
    let expect = expect![[
        r#"OK (ace-isearch-use-jump t (ace-isearch-use-jump nil "0.1.3") "[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ace-isearch/20220809.1748/home/.emacs.d/elpa/ace-isearch-20220809.1748/ace-isearch.el")"#
    ]];

    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_submode_assignment_writes_through_to_the_target() {
    let elisp_form = r##"(progn
               (setq ace-isearch-submode 'avy-goto-char)
               (list ace-isearch-submode ace-isearch-function))"##;
    let expect = expect!["OK (avy-goto-char avy-goto-char)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_jump_delay_assignment_writes_through_to_the_target() {
    let elisp_form = r##"(progn
               (setq ace-isearch-input-idle-jump-delay 1.25)
               (list ace-isearch-input-idle-jump-delay
                     ace-isearch-jump-delay))"##;
    let expect = expect!["OK (1.25 1.25)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_function_delay_assignment_writes_through_to_the_target() {
    let elisp_form = r##"(progn
               (setq ace-isearch-input-idle-func-delay 2.5)
               (list ace-isearch-input-idle-func-delay
                     ace-isearch-func-delay))"##;
    let expect = expect!["OK (2.5 2.5)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_obsolete_use_jump_assignment_writes_through_to_the_target() {
    let elisp_form = r##"(progn
               (setq ace-isearch-use-ace-jump 'printing-char)
               (list ace-isearch-use-ace-jump ace-isearch-use-jump))"##;
    let expect = expect!["OK (printing-char printing-char)"];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_avy_only_load_selects_avy_defaults() {
    let elisp_form = r##"(list
               (featurep 'ace-jump-mode)
               (featurep 'avy)
               ace-isearch-function
               ace-isearch-2-function
               ace-isearch-function-from-isearch
               ace-isearch-use-function-from-isearch)"##;
    let expect = expect!["OK (nil t avy-goto-word-1 avy-goto-char-2 nil nil)"];

    assert_ace_isearch_avy_backend_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_load_without_any_jump_backend_signals_the_exact_user_error() {
    let elisp_form = r##"(load
               (symbol-file 'ace-isearch-mode 'defun)
               nil
               t
               t)"##;
    let expect =
        expect![[r#"ERR (user-error "You need to install either ace-jump-mode or avy.")"#]];

    assert_ace_isearch_autoload_signal_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_line_search_backend_precedence_prefers_helm_swoop() {
    let elisp_form = r##"(list
               ace-isearch-function-from-isearch
               ace-isearch-use-function-from-isearch)"##;
    let expect = expect!["OK (ace-isearch-helm-swoop-from-isearch t)"];

    assert_ace_isearch_with_prelude_parity(
        "(progn (provide 'ace-jump-mode) (provide 'helm-swoop) (provide 'helm-occur) (provide 'swiper) (provide 'consult))",
        elisp_form,
        expect,
    );
}

#[test]
fn ace_isearch_line_search_backend_falls_back_to_helm_occur() {
    let elisp_form = r##"(list
               ace-isearch-function-from-isearch
               ace-isearch-use-function-from-isearch)"##;
    let expect = expect!["OK (ace-isearch-helm-occur-from-isearch t)"];

    assert_ace_isearch_with_prelude_parity(
        "(progn (provide 'ace-jump-mode) (provide 'helm-occur))",
        elisp_form,
        expect,
    );
}

#[test]
fn ace_isearch_line_search_backend_falls_back_to_swiper() {
    let elisp_form = r##"(list
               ace-isearch-function-from-isearch
               ace-isearch-use-function-from-isearch)"##;
    let expect = expect!["OK (ace-isearch-swiper-from-isearch t)"];

    assert_ace_isearch_with_prelude_parity(
        "(progn (provide 'ace-jump-mode) (provide 'swiper))",
        elisp_form,
        expect,
    );
}

#[test]
fn ace_isearch_line_search_backend_falls_back_to_consult() {
    let elisp_form = r##"(list
               ace-isearch-function-from-isearch
               ace-isearch-use-function-from-isearch)"##;
    let expect = expect!["OK (ace-isearch-consult-line-from-isearch t)"];

    assert_ace_isearch_with_prelude_parity(
        "(progn (provide 'ace-jump-mode) (provide 'consult))",
        elisp_form,
        expect,
    );
}

#[test]
fn ace_isearch_missing_line_search_backend_disables_transition_and_emits_exact_message() {
    let elisp_form = r##"(list
               ace-isearch-function-from-isearch
               ace-isearch-use-function-from-isearch
               (cl-remove-if-not
                (lambda (arguments)
                  (and (stringp (car arguments))
                       (string-prefix-p
                        "You don't have a suitable line-searching"
                        (car arguments))))
                (nreverse ace-isearch-load-messages)))"##;
    let expect = expect![[
        r#"OK (nil nil (("You don't have a suitable line-searching package installed.\nIn order to seamlessly transition to a line-searching command\nthrough `ace-isearch', you should install the following\npackage(s) of your choice:\n\n* Helm (in order to use `helm-occur')\n* Helm and helm-swoop (in order to use `helm-swoop')\n* Swiper (in order to use `swiper'), or\n* Consult (in order to use `consult-line').")))"#
    ]];

    assert_ace_isearch_with_prelude_parity(
        "(progn (provide 'ace-jump-mode) (defvar ace-isearch-load-messages nil) (fset 'message (lambda (&rest arguments) (push arguments ace-isearch-load-messages))))",
        elisp_form,
        expect,
    );
}

#[test]
fn ace_isearch_missing_line_search_backend_honors_preconfigured_message_suppression() {
    let elisp_form = r##"(list
               ace-isearch-disable-isearch-function-from-isearch-message
               ace-isearch-function-from-isearch
               ace-isearch-use-function-from-isearch
               (cl-remove-if-not
                (lambda (arguments)
                  (and (stringp (car arguments))
                       (string-prefix-p
                        "You don't have a suitable line-searching"
                        (car arguments))))
                ace-isearch-load-messages))"##;
    let expect = expect!["OK (t nil nil nil)"];

    assert_ace_isearch_with_prelude_parity(
        "(progn (provide 'ace-jump-mode) (setq ace-isearch-disable-isearch-function-from-isearch-message t) (defvar ace-isearch-load-messages nil) (fset 'message (lambda (&rest arguments) (push arguments ace-isearch-load-messages))))",
        elisp_form,
        expect,
    );
}
