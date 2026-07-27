use expect_test::expect;

use super::{assert_arjen_grey_theme_parity, assert_arjen_grey_theme_with_prelude_parity};

#[test]
fn arjen_grey_theme_load_without_enable_registers_but_does_not_apply_values() {
    let elisp_form = r##"(let ((before
                    (face-attribute
                     'default :background nil t)))
               (list
                before
                (custom-theme-p 'arjen-grey)
                (custom-theme-enabled-p 'arjen-grey)
                custom-enabled-themes
                (face-attribute
                 'default :background nil t)
                (boundp 'hl-paren-colors)))"##;
    let expect = expect![[
        r#"OK ("unspecified-bg" (arjen-grey user changed) nil nil "unspecified-bg" nil)"#
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_enable_applies_core_faces_and_variable_then_disable_restores() {
    let elisp_form = r##"(let ((was-bound
                    (boundp 'hl-paren-colors))
                   (old-value
                    (and
                     (boundp 'hl-paren-colors)
                     (default-value
                      'hl-paren-colors)))
                    before
                    during
                    after)
               (unwind-protect
                   (progn
                     (set-default
                      'hl-paren-colors '(before))
                     (setq before
                           (list
                            custom-enabled-themes
                            (face-attribute
                             'default :foreground nil t)
                            (face-attribute
                             'default :background nil t)
                            (default-value
                             'hl-paren-colors)))
                     (enable-theme 'arjen-grey)
                     (setq during
                           (list
                            custom-enabled-themes
                            (custom-theme-enabled-p
                             'arjen-grey)
                            (face-attribute
                             'default :foreground nil t)
                            (face-attribute
                             'default :background nil t)
                            (face-attribute
                             'cursor :background nil t)
                            (face-attribute
                             'region :background nil t)
                            (face-attribute
                             'mode-line :foreground nil t)
                            (face-attribute
                             'mode-line :background nil t)
                            (default-value
                             'hl-paren-colors)))
                     (disable-theme 'arjen-grey)
                     (setq after
                           (list
                            custom-enabled-themes
                            (custom-theme-enabled-p
                             'arjen-grey)
                            (face-attribute
                             'default :foreground nil t)
                            (face-attribute
                             'default :background nil t)
                            (default-value
                             'hl-paren-colors))))
                 (when
                     (custom-theme-enabled-p 'arjen-grey)
                   (disable-theme 'arjen-grey))
                 (if was-bound
                     (set-default
                      'hl-paren-colors old-value)
                   (makunbound 'hl-paren-colors)))
               (list before during after))"##;
    let expect = expect![[
        r##"OK ((nil "unspecified-fg" "unspecified-bg" #2=(before)) (#1=(arjen-grey) #1# "#bdc3ce" "#2a2f38" "#e1cb8c" "#3c4449" "#bdc3ce" "#242a34" ("#B9F" "#B8D" "#B7B" "#B69" "#B57" "#B45" "#B33" "#B11")) (nil nil "unspecified-fg" "unspecified-bg" #2#))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_enable_disable_enable_cycle_is_stable_and_idempotent() {
    let elisp_form = r##"(let (first disabled second)
               (unwind-protect
                   (progn
                     (enable-theme 'arjen-grey)
                     (setq first
                           (list
                            custom-enabled-themes
                            (face-attribute
                             'font-lock-keyword-face
                             :foreground nil t)
                            (face-attribute
                             'font-lock-warning-face
                             :foreground nil t)
                            (face-attribute
                             'font-lock-warning-face
                             :weight nil t)))
                     (disable-theme 'arjen-grey)
                     (setq disabled
                           (list
                            custom-enabled-themes
                            (custom-theme-enabled-p
                             'arjen-grey)))
                     (enable-theme 'arjen-grey)
                     (setq second
                           (list
                            custom-enabled-themes
                            (face-attribute
                             'font-lock-keyword-face
                             :foreground nil t)
                            (face-attribute
                             'font-lock-warning-face
                             :foreground nil t)
                            (face-attribute
                             'font-lock-warning-face
                             :weight nil t))))
                 (when
                     (custom-theme-enabled-p 'arjen-grey)
                   (disable-theme 'arjen-grey)))
               (list first disabled second
                     (equal first second)))"##;
    let expect = expect![[
        r##"OK (((arjen-grey) "#b894b0" "red" bold) (nil nil) ((arjen-grey) "#b894b0" "red" bold) t)"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_repeated_load_theme_keeps_one_enabled_entry_and_setting_set() {
    let elisp_form = r##"(let ((before
                    (length
                     (get 'arjen-grey 'theme-settings)))
                   first
                   second)
               (unwind-protect
                   (progn
                     (load-theme 'arjen-grey t)
                     (setq first
                           (list
                            custom-enabled-themes
                            (length
                             (get
                              'arjen-grey
                              'theme-settings))))
                     (load-theme 'arjen-grey t)
                     (setq second
                           (list
                            custom-enabled-themes
                            (length
                             (get
                              'arjen-grey
                              'theme-settings)))))
                 (when
                     (custom-theme-enabled-p 'arjen-grey)
                   (disable-theme 'arjen-grey)))
               (list before first second
                     custom-enabled-themes))"##;
    let expect = expect!["OK (38 ((arjen-grey) 38) ((arjen-grey) 38) nil)"];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_applies_to_optional_faces_defined_before_source_load() {
    let prelude = r##"(dolist
                   (face
                    '(helm-source-header
                      company-tooltip-selection
                      persp-selected-face
                      gnus-summary-normal-read))
                 (face-spec-set
                  face
                  '((t
                     (:foreground "fallback"
                      :background "fallback"
                      :weight normal)))
                  'face-defface-spec))"##;
    let elisp_form = r##"(unwind-protect
               (progn
                 (enable-theme 'arjen-grey)
                 (mapcar
                  (lambda (request)
                    (list
                     (car request)
                     (cadr request)
                     (face-attribute
                      (car request)
                      (cadr request)
                      nil t)))
                  '((helm-source-header :foreground)
                    (helm-source-header :background)
                    (helm-source-header :weight)
                    (helm-source-header :box)
                    (company-tooltip-selection :background)
                    (persp-selected-face :foreground)
                    (gnus-summary-normal-read :foreground))))
             (disable-theme 'arjen-grey))"##;
    let expect = expect![[
        r##"OK ((helm-source-header :foreground "#bdc3ce") (helm-source-header :background "#2a2f38") (helm-source-header :weight bold) (helm-source-header :box (:line-width -1 :style released-button)) (company-tooltip-selection :background "#464a4d") (persp-selected-face :foreground "#eacc8c") (gnus-summary-normal-read :foreground "#909fab"))"##
    ]];
    assert_arjen_grey_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn arjen_grey_theme_enabled_specs_apply_to_optional_faces_defined_late() {
    let elisp_form = r##"(unwind-protect
               (progn
                 (enable-theme 'arjen-grey)
                 (dolist
                     (face
                      '(company-tooltip
                        helm-selection
                        gnus-header-subject))
                   (face-spec-set
                    face
                    '((t
                       (:foreground "fallback"
                        :background "fallback")))
                    'face-defface-spec))
                 (mapcar
                  (lambda (request)
                    (list
                     (car request)
                     (cadr request)
                     (face-attribute
                      (car request)
                      (cadr request)
                      nil t)))
                  '((company-tooltip :foreground)
                    (company-tooltip :background)
                    (helm-selection :background)
                    (helm-selection :underline)
                    (gnus-header-subject :foreground))))
             (disable-theme 'arjen-grey))"##;
    let expect = expect![[
        r##"OK ((company-tooltip :foreground "#bdc3ce") (company-tooltip :background "#242a34") (helm-selection :background "#3c4449") (helm-selection :underline nil) (gnus-header-subject :foreground "#eacc8c"))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_stacks_over_user_theme_and_reveals_it_after_disable() {
    let elisp_form = r##"(let ((theme 'arjen-grey-parity-base)
                    during
                    after)
               (custom-declare-theme
                theme "Parity base theme.")
               (custom-theme-set-faces
                theme
                '(default
                   ((t
                     (:foreground "base-fg"
                      :background "base-bg"))))
                '(font-lock-keyword-face
                   ((t
                     (:foreground "base-keyword")))))
               (unwind-protect
                   (progn
                     (enable-theme theme)
                     (enable-theme 'arjen-grey)
                     (setq during
                           (list
                            custom-enabled-themes
                            (face-attribute
                             'default :foreground nil t)
                            (face-attribute
                             'default :background nil t)
                            (face-attribute
                             'font-lock-keyword-face
                             :foreground nil t)))
                     (disable-theme 'arjen-grey)
                     (setq after
                           (list
                            custom-enabled-themes
                            (face-attribute
                             'default :foreground nil t)
                            (face-attribute
                             'default :background nil t)
                            (face-attribute
                             'font-lock-keyword-face
                             :foreground nil t))))
                 (when
                     (custom-theme-enabled-p 'arjen-grey)
                   (disable-theme 'arjen-grey))
                 (when
                     (custom-theme-enabled-p theme)
                   (disable-theme theme)))
               (list during after))"##;
    let expect = expect![[
        r##"OK (((arjen-grey . #1=(arjen-grey-parity-base)) "#bdc3ce" "#2a2f38" "#b894b0") (#1# "base-fg" "base-bg" "base-keyword"))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}
