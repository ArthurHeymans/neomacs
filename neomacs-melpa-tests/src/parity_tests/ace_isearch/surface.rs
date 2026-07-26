use expect_test::expect;

use super::assert_ace_isearch_parity;

#[test]
fn ace_isearch_regexp_function_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch--isearch-regexp-function t)
               (interactive-form 'ace-isearch--isearch-regexp-function)
               (documentation 'ace-isearch--isearch-regexp-function t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch--isearch-regexp-function 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ace-isearch.el")"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_switch_function_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch-switch-function t)
               (interactive-form 'ace-isearch-switch-function)
               (documentation 'ace-isearch-switch-function t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-switch-function 'defun)))"##;
    let expect = expect![[r#"OK (nil (interactive nil) nil "ace-isearch.el")"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_two_switch_function_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch-2-switch-function t)
               (interactive-form 'ace-isearch-2-switch-function)
               (documentation 'ace-isearch-2-switch-function t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-2-switch-function 'defun)))"##;
    let expect = expect![[r#"OK (nil (interactive nil) nil "ace-isearch.el")"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_fboundp_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch--fboundp t)
               (interactive-form 'ace-isearch--fboundp)
               (documentation 'ace-isearch--fboundp t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch--fboundp 'defun)))"##;
    let expect = expect![[r#"OK ((func flag) nil nil "ace-isearch.el")"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jumper_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch--jumper-function t)
               (interactive-form 'ace-isearch--jumper-function)
               (documentation 'ace-isearch--jumper-function t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch--jumper-function 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ace-isearch.el")"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_pop_mark_callable_metadata_documentation_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch-pop-mark t)
               (interactive-form 'ace-isearch-pop-mark)
               (documentation 'ace-isearch-pop-mark t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-pop-mark 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Jump back to the last location of `ace-jump-mode' invoked or `avy-push-mark'." "ace-isearch.el")"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_make_backend_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch--make-ace-jump-or-avy t)
               (interactive-form 'ace-isearch--make-ace-jump-or-avy)
               (documentation 'ace-isearch--make-ace-jump-or-avy t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch--make-ace-jump-or-avy 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ace-isearch.el")"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_make_two_backend_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch-2--make-ace-jump-or-avy t)
               (interactive-form 'ace-isearch-2--make-ace-jump-or-avy)
               (documentation 'ace-isearch-2--make-ace-jump-or-avy t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-2--make-ace-jump-or-avy 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ace-isearch.el")"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_helm_occur_adapter_callable_metadata_documentation_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch-helm-occur-from-isearch t)
               (interactive-form 'ace-isearch-helm-occur-from-isearch)
               (documentation 'ace-isearch-helm-occur-from-isearch t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-helm-occur-from-isearch 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Invoke `helm-swoop' from ace-isearch." "ace-isearch.el")"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_helm_swoop_adapter_callable_metadata_documentation_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch-helm-swoop-from-isearch t)
               (interactive-form 'ace-isearch-helm-swoop-from-isearch)
               (documentation 'ace-isearch-helm-swoop-from-isearch t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-helm-swoop-from-isearch 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Invoke `helm-swoop' from ace-isearch." "ace-isearch.el")"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_swiper_adapter_callable_metadata_documentation_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch-swiper-from-isearch t)
               (interactive-form 'ace-isearch-swiper-from-isearch)
               (documentation 'ace-isearch-swiper-from-isearch t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-swiper-from-isearch 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Invoke `swiper' from ace-isearch." "ace-isearch.el")"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_consult_adapter_callable_metadata_documentation_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch-consult-line-from-isearch t)
               (interactive-form 'ace-isearch-consult-line-from-isearch)
               (documentation 'ace-isearch-consult-line-from-isearch t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-consult-line-from-isearch 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Invoke `consult-line' from ace-isearch." "ace-isearch.el")"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_jump_during_callable_metadata_documentation_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch-jump-during-isearch t)
               (interactive-form 'ace-isearch-jump-during-isearch)
               (documentation 'ace-isearch-jump-during-isearch t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-jump-during-isearch 'defun)))"##;
    let expect = expect![[
        r#"OK (nil (interactive nil) "Jump to one of the current isearch candidates." "ace-isearch.el")"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_minor_mode_callable_metadata_documentation_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch-mode t)
               (interactive-form 'ace-isearch-mode)
               (documentation 'ace-isearch-mode t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch-mode 'defun)))"##;
    let expect = expect![[
        r#"OK ((&optional arg) (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "Minor-mode that combines isearch, ace-jump-mode, avy, helm-swoop and swiper seamlessly.\n\nThis is a minor mode.  If called interactively, toggle the `Ace-Isearch\nmode' mode.  If the prefix argument is positive, enable the mode, and if\nit is zero or negative, disable the mode.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.  Enable the\nmode if ARG is nil, omitted, or is a positive number.  Disable the mode\nif ARG is a negative number.\n\nTo check whether the minor mode is enabled in the current buffer,\nevaluate the variable `ace-isearch-mode'.\n\nThe mode's hook is called both when the mode is enabled and when it is\ndisabled." "ace-isearch.el")"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_turn_on_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'ace-isearch--turn-on t)
               (interactive-form 'ace-isearch--turn-on)
               (documentation 'ace-isearch--turn-on t)
               (file-name-nondirectory
                (symbol-file 'ace-isearch--turn-on 'defun)))"##;
    let expect = expect![[r#"OK (nil nil nil "ace-isearch.el")"#]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_global_mode_callable_metadata_documentation_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist 'global-ace-isearch-mode t)
               (interactive-form 'global-ace-isearch-mode)
               (documentation 'global-ace-isearch-mode t)
               (file-name-nondirectory
                (symbol-file 'global-ace-isearch-mode 'defun)))"##;
    let expect = expect![[
        r#"OK ((&optional arg) (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle))) "Toggle Ace-Isearch mode in many buffers.\nSpecifically, Ace-Isearch mode is enabled in all buffers where\n`ace-isearch--turn-on' would do it.\n\nWith prefix ARG, enable Global Ace-Isearch mode if ARG is positive;\notherwise, disable it.\n\nIf called from Lisp, toggle the mode if ARG is `toggle'.\nEnable the mode if ARG is nil, omitted, or is a positive number.\nDisable the mode if ARG is a negative number.\n\nSee `ace-isearch-mode' for more information on Ace-Isearch mode." "ace-isearch.el")"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}

#[test]
fn ace_isearch_packaged_source_descriptor_autoload_and_readme_assets_have_exact_hashes() {
    let elisp_form = r##"(let ((root
                    (file-name-directory
                     (symbol-file 'ace-isearch-mode 'defun))))
               (mapcar
                (lambda (file)
                  (with-temp-buffer
                    (insert-file-contents-literally
                     (expand-file-name file root))
                    (list file
                          (buffer-size)
                          (secure-hash 'sha256 (current-buffer)))))
                '("ace-isearch.el"
                  "ace-isearch-pkg.el"
                  "ace-isearch-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ace-isearch.el" 17054 "f4efce24f88103e03b3fbb53937979905176d20ac0860df20084ce73e3e4a04f") ("ace-isearch-pkg.el" 327 "07f7826261d8be06df0ad4c31ebe356dc306b0aa220d50ef0a122bc2e29cc8b8") ("ace-isearch-autoloads.el" 2501 "8dc84ad8afbdd19cd5344ea34c0a6ccef1b9c0031277730756803abdfc98d12a") ("README-elpa" 901 "c165a81cd752f70e707c336b32ebcbe54b0b0190431f703f11139916860e50a2"))"#
    ]];
    assert_ace_isearch_parity(elisp_form, expect);
}
