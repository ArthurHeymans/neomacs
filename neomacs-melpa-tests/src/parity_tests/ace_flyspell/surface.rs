use expect_test::expect;

use super::assert_ace_flyspell_parity;

#[test]
fn ace_flyspell_new_word_no_query_default_documentation_and_source_match() {
    let elisp_form = r##"(list
               ace-flyspell-new-word-no-query
               (default-boundp
                'ace-flyspell-new-word-no-query)
               (local-variable-if-set-p
                'ace-flyspell-new-word-no-query)
               (get
                'ace-flyspell-new-word-no-query
                'variable-documentation)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell-new-word-no-query
                 'defvar)))"##;
    let expect = expect![[
        r#"OK (nil t nil "If t, don't ask for confirmation when adding new words." "ace-flyspell.el")"#
    ]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_handler_default_metadata_and_source_match() {
    let elisp_form = r##"(list
               ace-flyspell-handler
               (default-boundp
                'ace-flyspell-handler)
               (local-variable-if-set-p
                'ace-flyspell-handler)
               (get
                'ace-flyspell-handler
                'variable-documentation)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell-handler
                 'defvar)))"##;
    let expect = expect![[r#"OK (nil t nil nil "ace-flyspell.el")"#]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_current_word_default_metadata_and_source_match() {
    let elisp_form = r##"(list
               ace-flyspell--current-word
               (default-boundp
                'ace-flyspell--current-word)
               (local-variable-if-set-p
                'ace-flyspell--current-word)
               (get
                'ace-flyspell--current-word
                'variable-documentation)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell--current-word
                 'defvar)))"##;
    let expect = expect![[r#"OK (nil t nil nil "ace-flyspell.el")"#]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_overlay_constant_metadata_and_source_match() {
    let elisp_form = r##"(list
               (boundp
                'ace-flyspell--ov)
               (default-boundp
                'ace-flyspell--ov)
               (get
                'ace-flyspell--ov
                'risky-local-variable)
               (get
                'ace-flyspell--ov
                'variable-documentation)
               (get
                'ace-flyspell--ov
                'standard-value)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell--ov
                 'defvar)))"##;
    let expect = expect![[r#"OK (t t t nil nil "ace-flyspell.el")"#]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_overlay_constant_starts_deleted_and_face_tagged() {
    let elisp_form = r##"(list
               (overlayp
                ace-flyspell--ov)
               (overlay-buffer
                ace-flyspell--ov)
               (overlay-start
                ace-flyspell--ov)
               (overlay-end
                ace-flyspell--ov)
               (overlay-get
                ace-flyspell--ov
                'face))"##;
    let expect = expect!["OK (t nil nil nil ace-flyspell--background)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_overlay_constant_retains_front_and_rear_boundary_insertion_behavior() {
    let elisp_form = r##"(with-temp-buffer
               (insert
                "abcd")
               (move-overlay
                ace-flyspell--ov
                2
                4)
               (goto-char
                (overlay-start
                 ace-flyspell--ov))
               (insert
                "S")
               (let ((after-front
                      (list
                       (overlay-start
                        ace-flyspell--ov)
                       (overlay-end
                        ace-flyspell--ov)
                       (buffer-string))))
                 (goto-char
                  (overlay-end
                   ace-flyspell--ov))
                 (insert
                  "E")
                 (prog1
                     (list
                      after-front
                      (overlay-start
                       ace-flyspell--ov)
                      (overlay-end
                       ace-flyspell--ov)
                      (buffer-string))
                   (delete-overlay
                    ace-flyspell--ov))))"##;
    let expect = expect![[r#"OK ((2 5 "aSbcd") 2 6 "aSbcEd")"#]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_face_specification_documentation_group_and_source_match() {
    let elisp_form = r##"(list
               (facep
                'ace-flyspell--background)
               (get
                'ace-flyspell--background
                'face-defface-spec)
               (face-documentation
                'ace-flyspell--background)
               (assq
                'ace-flyspell--background
                (get
                 'ace-flyspell
                 'custom-group))
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell--background
                 'defface)))"##;
    let expect = expect![[
        r#"OK ([face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] ((t (:box t :bold t))) "face for ace-flyspell" (ace-flyspell--background custom-face) "ace-flyspell.el")"#
    ]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_collect_candidates_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell--collect-candidates
                t)
               (interactive-form
                'ace-flyspell--collect-candidates)
               (documentation
                'ace-flyspell--collect-candidates
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell--collect-candidates
                 'defun)))"##;
    let expect = expect!["OK (nil nil nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_has_overlay_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell--has-flyspell-overlay-p
                t)
               (interactive-form
                'ace-flyspell--has-flyspell-overlay-p)
               (documentation
                'ace-flyspell--has-flyspell-overlay-p
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell--has-flyspell-overlay-p
                 'defun)))"##;
    let expect = expect!["OK ((ovs) nil nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_help_default_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell-help-default
                t)
               (interactive-form
                'ace-flyspell-help-default)
               (documentation
                'ace-flyspell-help-default
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell-help-default
                 'defun)))"##;
    let expect = expect!["OK (nil nil nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_auto_correct_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell--auto-correct-word
                t)
               (interactive-form
                'ace-flyspell--auto-correct-word)
               (documentation
                'ace-flyspell--auto-correct-word
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell--auto-correct-word
                 'defun)))"##;
    let expect = expect!["OK (nil nil nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_insert_word_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell--insert-word
                t)
               (interactive-form
                'ace-flyspell--insert-word)
               (documentation
                'ace-flyspell--insert-word
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell--insert-word
                 'defun)))"##;
    let expect = expect!["OK (nil (interactive nil) nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_reset_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell--reset
                t)
               (interactive-form
                'ace-flyspell--reset)
               (documentation
                'ace-flyspell--reset
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell--reset
                 'defun)))"##;
    let expect = expect!["OK (nil (interactive nil) nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_avy_word_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell--avy-word
                t)
               (interactive-form
                'ace-flyspell--avy-word)
               (documentation
                'ace-flyspell--avy-word
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell--avy-word
                 'defun)))"##;
    let expect = expect!["OK (nil nil nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_correct_word_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell-correct-word
                t)
               (interactive-form
                'ace-flyspell-correct-word)
               (documentation
                'ace-flyspell-correct-word
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell-correct-word
                 'defun)))"##;
    let expect = expect!["OK (nil (interactive nil) nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_default_handler_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell-default-handler
                t)
               (interactive-form
                'ace-flyspell-default-handler)
               (documentation
                'ace-flyspell-default-handler
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell-default-handler
                 'defun)))"##;
    let expect = expect!["OK (nil nil nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_jump_word_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell-jump-word
                t)
               (interactive-form
                'ace-flyspell-jump-word)
               (documentation
                'ace-flyspell-jump-word
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell-jump-word
                 'defun)))"##;
    let expect = expect!["OK (nil (interactive nil) nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_dwim_callable_metadata_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell-dwim
                t)
               (interactive-form
                'ace-flyspell-dwim)
               (documentation
                'ace-flyspell-dwim
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell-dwim
                 'defun)))"##;
    let expect = expect!["OK (nil (interactive nil) nil \"ace-flyspell.el\")"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_setup_callable_metadata_documentation_and_source_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ace-flyspell-setup
                t)
               (interactive-form
                'ace-flyspell-setup)
               (documentation
                'ace-flyspell-setup
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-flyspell-setup
                 'defun)))"##;
    let expect =
        expect![[r#"OK (nil (interactive nil) "Set up default keybindings." "ace-flyspell.el")"#]];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_required_features_match() {
    let elisp_form = r##"(mapcar
               #'featurep
               '(ace-flyspell
                 avy
                 flyspell))"##;
    let expect = expect!["OK (t t t)"];

    assert_ace_flyspell_parity(elisp_form, expect);
}

#[test]
fn ace_flyspell_packaged_source_descriptor_autoload_and_readme_assets_have_exact_hashes() {
    let elisp_form = r##"(let ((root
                    (file-name-directory
                     (symbol-file
                      'ace-flyspell-dwim
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
                '("ace-flyspell.el"
                  "ace-flyspell-pkg.el"
                  "ace-flyspell-autoloads.el"
                  "README-elpa")))"##;
    let expect = expect![[
        r#"OK (("ace-flyspell.el" 9128 "1fdcc93e0ab98d38e2286883c07348e7c69e49aea4d7804f694b312b1ad71912") ("ace-flyspell-pkg.el" 466 "3720ae52f9328df7299f5e5788709a81c4db9009d84c6153a623bc22c3fe60ea") ("ace-flyspell-autoloads.el" 892 "52041c6f3530133f0a121074e813ad4ef2bbb137e4e5a868f060d82d09d7bbe7") ("README-elpa" 3584 "0f043f453e7bccff84808f2c8b9d1d9e66f45dc1c27379cf571080181f7d7cd9"))"#
    ]];

    assert_ace_flyspell_parity(elisp_form, expect);
}
