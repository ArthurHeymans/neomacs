use expect_test::expect;

use super::{assert_ace_jump_buffer_parity, assert_ace_jump_buffer_with_prelude_parity};

#[test]
fn ace_jump_buffer_optional_integrations_are_absent_without_their_features() {
    let elisp_form = r##"(list
               (featurep 'perspective)
               (featurep 'persp-mode)
               (featurep 'projectile)
               (fboundp 'ajb/filter-persp-buffers)
               (fboundp 'ace-jump-persp-buffers)
               (fboundp 'ajb/filter-projectile-buffers)
               (fboundp 'ace-jump-projectile-buffers)
               (assoc "persp" bs-configurations)
               (assoc "projectile" bs-configurations))"##;
    let expect = expect!["OK (nil nil nil nil nil nil nil nil nil)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_perspective_integration_filters_against_current_perspective_buffers() {
    let elisp_form = r##"(progn
               (setq persp-curr 'current-perspective)
               (let ((included
                      (generate-new-buffer "ajb-included"))
                     (excluded
                      (generate-new-buffer "ajb-excluded"))
                     calls)
                 (unwind-protect
                     (cl-letf
                         (((symbol-function 'persp-buffers)
                           (lambda (perspective)
                             (push
                              (list
                               perspective
                               (buffer-name))
                              calls)
                             (list included))))
                       (list
                        (ajb/filter-persp-buffers
                         included)
                        (ajb/filter-persp-buffers
                         excluded)
                        (assoc
                         "persp"
                         bs-configurations)
                        (interactive-form
                         'ace-jump-persp-buffers)
                        (nreverse calls)))
                   (kill-buffer included)
                   (kill-buffer excluded))))"##;
    let expect = expect![[
        r#"OK (nil t ("persp" nil nil nil ajb/filter-persp-buffers nil) (interactive nil) ((current-perspective "ajb-included") (current-perspective "ajb-excluded")))"#
    ]];
    assert_ace_jump_buffer_with_prelude_parity(
        "(defvar persp-curr nil)\n(provide 'perspective)",
        elisp_form,
        expect,
    );
}

#[test]
fn ace_jump_buffer_perspective_generated_filter_and_command_surface_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ajb/filter-persp-buffers
                t)
               (interactive-form
                'ajb/filter-persp-buffers)
               (documentation
                'ajb/filter-persp-buffers
                t)
               (file-name-nondirectory
                (symbol-file
                 'ajb/filter-persp-buffers
                 'defun))
               (help-function-arglist
                'ace-jump-persp-buffers
                t)
               (interactive-form
                'ace-jump-persp-buffers)
               (documentation
                'ace-jump-persp-buffers
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-persp-buffers
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((buffer) nil nil "ace-jump-buffer.el" nil (interactive nil) nil "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_with_prelude_parity(
        "(defvar persp-curr nil)\n(provide 'perspective)",
        elisp_form,
        expect,
    );
}

#[test]
fn ace_jump_buffer_persp_mode_integration_filters_against_its_buffer_list() {
    let elisp_form = r##"(let ((included
                    (generate-new-buffer "ajb-included"))
                   (excluded
                    (generate-new-buffer "ajb-excluded"))
                   calls)
               (unwind-protect
                   (cl-letf
                       (((symbol-function 'persp-buffer-list)
                         (lambda ()
                           (push (buffer-name) calls)
                           (list included))))
                     (list
                      (ajb/filter-persp-buffers included)
                      (ajb/filter-persp-buffers excluded)
                      (assoc
                       "persp"
                       bs-configurations)
                      (nreverse calls)))
                 (kill-buffer included)
                 (kill-buffer excluded)))"##;
    let expect = expect![[
        r#"OK (nil t ("persp" nil nil nil ajb/filter-persp-buffers nil) ("ajb-included" "ajb-excluded"))"#
    ]];
    assert_ace_jump_buffer_with_prelude_parity("(provide 'persp-mode)", elisp_form, expect);
}

#[test]
fn ace_jump_buffer_persp_mode_definition_wins_when_both_perspective_features_exist() {
    let elisp_form = r##"(let ((buffer
                    (generate-new-buffer "ajb-both"))
                   calls)
               (unwind-protect
                   (cl-letf
                       (((symbol-function 'persp-buffers)
                         (lambda (_perspective)
                           (push 'perspective calls)
                           nil))
                        ((symbol-function 'persp-buffer-list)
                         (lambda ()
                           (push 'persp-mode calls)
                           (list buffer))))
                     (list
                      (ajb/filter-persp-buffers buffer)
                      (cl-count
                       "persp"
                       bs-configurations
                       :key #'car
                       :test #'equal)
                      (nreverse calls)))
                 (kill-buffer buffer)))"##;
    let expect = expect!["OK (nil 1 (persp-mode))"];
    assert_ace_jump_buffer_with_prelude_parity(
        "(defvar persp-curr nil)\n(provide 'perspective)\n(provide 'persp-mode)",
        elisp_form,
        expect,
    );
}

#[test]
fn ace_jump_buffer_projectile_integration_computes_root_in_caller_then_tests_target_buffer() {
    let elisp_form = r##"(let ((target
                    (generate-new-buffer "ajb-project"))
                   calls)
               (unwind-protect
                   (with-temp-buffer
                     (rename-buffer "ajb-caller" t)
                     (cl-letf
                         (((symbol-function
                            'projectile-project-root)
                           (lambda ()
                             (push
                              (list
                               'root
                               (buffer-name))
                              calls)
                             "/workspace/project/"))
                          ((symbol-function
                            'projectile-project-buffer-p)
                           (lambda (buffer root)
                             (push
                              (list
                               'predicate
                               (buffer-name)
                               (buffer-name buffer)
                               root)
                              calls)
                             t)))
                       (list
                        (ajb/filter-projectile-buffers
                         target)
                        (assoc
                         "projectile"
                         bs-configurations)
                        (interactive-form
                         'ace-jump-projectile-buffers)
                        (nreverse calls))))
                 (kill-buffer target)))"##;
    let expect = expect![[
        r#"OK (nil ("projectile" nil nil nil ajb/filter-projectile-buffers nil) (interactive nil) ((root "ajb-caller") (predicate "ajb-project" "ajb-project" "/workspace/project/")))"#
    ]];
    assert_ace_jump_buffer_with_prelude_parity("(provide 'projectile)", elisp_form, expect);
}

#[test]
fn ace_jump_buffer_projectile_filter_rejects_a_buffer_when_project_predicate_is_nil() {
    let elisp_form = r##"(let ((target
                    (generate-new-buffer "ajb-rejected"))
                   calls)
               (unwind-protect
                   (with-temp-buffer
                     (rename-buffer "ajb-caller" t)
                     (cl-letf
                         (((symbol-function
                            'projectile-project-root)
                           (lambda ()
                             (push
                              (list
                               'root
                               (buffer-name))
                              calls)
                             "/workspace/project/"))
                          ((symbol-function
                            'projectile-project-buffer-p)
                           (lambda (buffer root)
                             (push
                              (list
                               'predicate
                               (buffer-name)
                               (buffer-name buffer)
                               root)
                              calls)
                             nil)))
                       (list
                        (ajb/filter-projectile-buffers
                         target)
                        (nreverse calls))))
                 (kill-buffer target)))"##;
    let expect = expect![[
        r#"OK (t ((root "ajb-caller") (predicate "ajb-rejected" "ajb-rejected" "/workspace/project/")))"#
    ]];
    assert_ace_jump_buffer_with_prelude_parity("(provide 'projectile)", elisp_form, expect);
}

#[test]
fn ace_jump_buffer_projectile_generated_filter_and_command_surface_match() {
    let elisp_form = r##"(list
               (help-function-arglist
                'ajb/filter-projectile-buffers
                t)
               (interactive-form
                'ajb/filter-projectile-buffers)
               (documentation
                'ajb/filter-projectile-buffers
                t)
               (file-name-nondirectory
                (symbol-file
                 'ajb/filter-projectile-buffers
                 'defun))
               (help-function-arglist
                'ace-jump-projectile-buffers
                t)
               (interactive-form
                'ace-jump-projectile-buffers)
               (documentation
                'ace-jump-projectile-buffers
                t)
               (file-name-nondirectory
                (symbol-file
                 'ace-jump-projectile-buffers
                 'defun)))"##;
    let expect = expect![[
        r#"OK ((buffer) nil nil "ace-jump-buffer.el" nil (interactive nil) nil "ace-jump-buffer.el")"#
    ]];
    assert_ace_jump_buffer_with_prelude_parity("(provide 'projectile)", elisp_form, expect);
}

#[test]
fn ace_jump_buffer_optional_generated_commands_use_their_named_configurations() {
    let elisp_form = r##"(let ((ajb-bs-configuration "outer")
                   calls)
               (cl-letf
                   (((symbol-function 'ace-jump-buffer)
                     (lambda ()
                       (push ajb-bs-configuration calls)
                       'jump-result)))
                 (list
                  (ace-jump-persp-buffers)
                  (ace-jump-projectile-buffers)
                  ajb-bs-configuration
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (jump-result jump-result "outer" ("persp" "projectile"))"#]];
    assert_ace_jump_buffer_with_prelude_parity(
        "(provide 'persp-mode)\n(provide 'projectile)",
        elisp_form,
        expect,
    );
}
