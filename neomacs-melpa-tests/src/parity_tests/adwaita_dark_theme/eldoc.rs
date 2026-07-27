use expect_test::expect;

use super::assert_adwaita_dark_theme_parity;

#[test]
fn adwaita_dark_theme_eldoc_configuration_updates_real_face_parameters_and_hook_order() {
    let elisp_form = r##"(progn
         (defvar eldoc-frame-parameters nil)
         (defvar eldoc-frame-buffer-hook nil)
         (let ((eldoc-frame-parameters
                '((width . 80)
                  (left-fringe . 1)))
               (eldoc-frame-buffer-hook nil))
         (unless (facep 'eldoc-frame-default)
           (make-empty-face 'eldoc-frame-default))
         (adwaita-dark-theme-eldoc-frame-configuration-enable)
         (list
          (face-background
           'eldoc-frame-default nil 'default)
          eldoc-frame-parameters
          (length eldoc-frame-buffer-hook)
          (mapcar
           (lambda (function)
             (list
             (functionp function)
              (help-function-arglist function t)))
           eldoc-frame-buffer-hook))))"##;
    let expect = expect![[
        r##"OK ("#000000" ((alpha-background . 80) (right-fringe . 12) (left-fringe . 12) (width . 80) (left-fringe . 1)) 1 ((t nil)))"##
    ]];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_eldoc_hook_renders_real_vertical_padding_and_buffer_state() {
    let elisp_form = r##"(progn
         (defvar eldoc-frame-parameters nil)
         (defvar eldoc-frame-buffer-hook nil)
         (let ((eldoc-frame-parameters nil)
               (eldoc-frame-buffer-hook nil))
         (unless (facep 'eldoc-frame-default)
           (make-empty-face 'eldoc-frame-default))
         (adwaita-dark-theme-eldoc-frame-configuration-enable)
         (with-temp-buffer
           (insert
            (propertize
             "Signature: (function argument)"
             'face 'font-lock-function-name-face))
           (run-hooks 'eldoc-frame-buffer-hook)
           (list
            (buffer-string)
            (point)
            line-spacing
            (text-properties-at (point-min))
            (text-properties-at
             (1- (point-max)))
            (buffer-substring-no-properties
             (point-min)
             (point-max))
            eldoc-frame-parameters))))"##;
    let expect = expect![[
        r#"OK (#(" \nSignature: (function argument) \n" 0 2 (face (:height 0.2)) 2 32 (face font-lock-function-name-face) 32 34 (face (:height 0.2))) 35 0.25 (face (:height 0.2)) (face (:height 0.2)) " \nSignature: (function argument) \n" ((alpha-background . 80) (right-fringe . 12) (left-fringe . 12)))"#
    ]];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}
