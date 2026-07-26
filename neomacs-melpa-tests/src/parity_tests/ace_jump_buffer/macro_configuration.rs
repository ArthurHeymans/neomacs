use expect_test::expect;

use super::assert_ace_jump_buffer_parity;

#[test]
fn ace_jump_buffer_generator_macro_expands_to_filter_command_and_configuration_registration() {
    let elisp_form = r##"(macroexpand-1
               '(make-ace-jump-buffer-function
                    "shell"
                  (with-current-buffer buffer
                    (not
                     (eq
                      major-mode
                      'shell-mode)))))"##;
    let expect = expect![[
        r#"OK (progn (defun ajb/filter-shell-buffers (buffer) (with-current-buffer buffer (not (eq major-mode 'shell-mode)))) (defun ace-jump-shell-buffers nil (interactive) (let ((ajb-bs-configuration "shell")) (ace-jump-buffer))) (add-to-list 'bs-configurations '("shell" nil nil nil ajb/filter-shell-buffers nil)))"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_generated_same_mode_configuration_has_exact_shape_and_single_registration() {
    let elisp_form = r##"(list
               (assoc "same-mode" bs-configurations)
               (cl-count
                "same-mode"
                bs-configurations
                :key #'car
                :test #'equal))"##;
    let expect = expect![[r#"OK (("same-mode" nil nil nil ajb/filter-same-mode-buffers nil) 1)"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_generated_same_mode_filter_compares_target_to_callers_current_mode() {
    let elisp_form = r##"(let ((same
                    (generate-new-buffer "ajb-same"))
                   (different
                    (generate-new-buffer "ajb-different")))
               (unwind-protect
                   (progn
                     (with-current-buffer same
                       (fundamental-mode))
                     (with-current-buffer different
                       (text-mode))
                     (with-temp-buffer
                       (fundamental-mode)
                       (list
                        (ajb/filter-same-mode-buffers same)
                        (ajb/filter-same-mode-buffers
                         different)
                        major-mode)))
                 (kill-buffer same)
                 (kill-buffer different)))"##;
    let expect = expect!["OK (nil t fundamental-mode)"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_generated_same_mode_command_uses_dynamic_configuration_and_restores_outer_value()
{
    let elisp_form = r##"(let ((ajb-bs-configuration "outer")
                   calls)
               (cl-letf
                   (((symbol-function 'ace-jump-buffer)
                     (lambda ()
                       (push ajb-bs-configuration calls)
                       'jump-result)))
                 (list
                  (ace-jump-same-mode-buffers)
                  ajb-bs-configuration
                  (nreverse calls))))"##;
    let expect = expect![[r#"OK (jump-result "outer" ("same-mode"))"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_generator_creates_strict_filter_and_interactive_command() {
    let elisp_form = r##"(progn
               (make-ace-jump-buffer-function
                   "shell"
                 (with-current-buffer buffer
                   (not
                    (eq
                     major-mode
                     'text-mode))))
               (let ((text
                      (generate-new-buffer "ajb-text"))
                     (fundamental
                      (generate-new-buffer
                       "ajb-fundamental"))
                     (ajb-bs-configuration "outer")
                     calls)
                 (unwind-protect
                     (progn
                       (with-current-buffer text
                         (text-mode))
                       (with-current-buffer fundamental
                         (fundamental-mode))
                       (cl-letf
                           (((symbol-function
                              'ace-jump-buffer)
                             (lambda ()
                               (push
                                ajb-bs-configuration
                                calls)
                               'jump-result)))
                         (list
                          (ajb/filter-shell-buffers text)
                          (ajb/filter-shell-buffers
                           fundamental)
                          (interactive-form
                           'ace-jump-shell-buffers)
                          (ace-jump-shell-buffers)
                          ajb-bs-configuration
                          (assoc
                           "shell"
                           bs-configurations)
                          (nreverse calls))))
                   (kill-buffer text)
                   (kill-buffer fundamental))))"##;
    let expect = expect![[
        r#"OK (nil t (interactive nil) jump-result "outer" ("shell" nil nil nil ajb/filter-shell-buffers nil) ("shell"))"#
    ]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_generator_with_empty_filter_body_accepts_every_buffer() {
    let elisp_form = r##"(progn
               (make-ace-jump-buffer-function "everything")
               (with-temp-buffer
                 (list
                  (ajb/filter-everything-buffers
                   (current-buffer))
                  (assoc
                   "everything"
                   bs-configurations))))"##;
    let expect =
        expect![[r#"OK (nil ("everything" nil nil nil ajb/filter-everything-buffers nil))"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_generator_runs_multiple_filter_forms_in_order_and_returns_the_last() {
    let elisp_form = r##"(let (calls)
               (make-ace-jump-buffer-function
                   "multi"
                 (setq calls
                       (append calls '(first)))
                 (setq calls
                       (append calls '(second)))
                 (prog1 'last-result
                   (setq calls
                         (append calls '(third)))))
                 (list
                  (ajb/filter-multi-buffers
                   (current-buffer))
                  calls))"##;
    let expect = expect!["OK (last-result (first second third))"];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}

#[test]
fn ace_jump_buffer_repeated_generation_redefines_functions_without_duplicate_configuration() {
    let elisp_form = r##"(progn
               (make-ace-jump-buffer-function
                   "repeat"
                 'first-filter)
               (make-ace-jump-buffer-function
                   "repeat"
                 'second-filter)
               (list
                (with-temp-buffer
                  (ajb/filter-repeat-buffers
                   (current-buffer)))
                (cl-count
                 "repeat"
                 bs-configurations
                 :key #'car
                 :test #'equal)
                (assoc
                 "repeat"
                 bs-configurations)))"##;
    let expect =
        expect![[r#"OK (second-filter 1 ("repeat" nil nil nil ajb/filter-repeat-buffers nil))"#]];
    assert_ace_jump_buffer_parity(elisp_form, expect);
}
