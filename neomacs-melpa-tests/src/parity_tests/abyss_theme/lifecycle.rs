use expect_test::expect;

use super::assert_abyss_theme_parity;

#[test]
fn abyss_theme_enable_applies_representative_faces_then_disable_restores_theme_state() {
    let elisp_form = r##"(let (during)
               (unwind-protect
                   (progn
                     (abyss-theme)
                     (setq during
                           (list
                            custom-enabled-themes
                            (custom-theme-enabled-p
                             'abyss)
                            (face-attribute
                             'default :foreground nil t)
                            (face-attribute
                             'default :background nil t)
                            (face-attribute
                             'mode-line :foreground nil t)
                            (face-attribute
                             'mode-line :background nil t)
                            (face-attribute
                             'mode-line :box nil nil)
                            (face-attribute
                             'font-lock-comment-face
                             :foreground nil t)
                            (face-attribute
                             'font-lock-comment-face
                             :slant nil t)
                            (face-attribute
                             'region :foreground nil t)
                            (face-attribute
                             'region :background nil t))))
                 (disable-theme 'abyss))
               (list
                during
                custom-enabled-themes
                (custom-theme-enabled-p 'abyss)))"##;
    let expect = expect![[
        r##"OK ((#1=(abyss) #1# "#bbe0f0" "#050000" "#050000" "#56b4e9" nil "#d55e00" italic "#050000" "#cc79a7") nil nil)"##
    ]];

    assert_abyss_theme_parity(elisp_form, expect);
}

#[test]
fn abyss_theme_enabled_specs_apply_when_optional_package_faces_are_defined_late() {
    let elisp_form = r##"(unwind-protect
               (progn
                 (abyss-theme)
                 (eval
                  '(defface
                       envrc-mode-line-on-face
                     '((t
                        (:inherit default
                         :foreground "fallback")))
                     "Parity face."))
                 (eval
                  '(defface
                       flycheck-warning
                     '((t
                        (:foreground "fallback")))
                     "Parity face."))
                 (eval
                  '(defface
                       compilation-mode-line-exit
                     '((t
                        (:foreground "fallback")))
                     "Parity face."))
                 (list
                  (and
                   (facep 'envrc-mode-line-on-face)
                   t)
                  (face-attribute
                   'envrc-mode-line-on-face
                   :inherit nil nil)
                  (face-attribute
                   'envrc-mode-line-on-face
                   :foreground nil t)
                  (face-attribute
                   'envrc-mode-line-on-face
                   :weight nil t)
                  (face-attribute
                   'flycheck-warning
                   :foreground nil t)
                  (face-attribute
                   'flycheck-warning
                   :weight nil t)
                  (face-attribute
                   'compilation-mode-line-exit
                   :foreground nil t)
                  (face-attribute
                   'compilation-mode-line-exit
                   :weight nil t)))
             (disable-theme 'abyss))"##;
    let expect = expect![[r##"OK (t nil "#009e73" bold "#050000" bold "#009e73" bold)"##]];

    assert_abyss_theme_parity(elisp_form, expect);
}

#[test]
fn abyss_theme_repeated_loads_keep_one_enabled_entry_and_do_not_duplicate_settings() {
    let elisp_form = r##"(let ((before
                    (length
                     (get 'abyss 'theme-settings)))
                   first
                   second)
               (unwind-protect
                   (progn
                     (load-theme 'abyss t)
                     (setq first
                           (list
                            custom-enabled-themes
                            (length
                             (get
                              'abyss
                              'theme-settings))))
                     (load-theme 'abyss t)
                     (setq second
                           (list
                            custom-enabled-themes
                            (length
                             (get
                              'abyss
                              'theme-settings)))))
                 (disable-theme 'abyss))
               (list
                before
                first
                second
                custom-enabled-themes
                (custom-theme-enabled-p 'abyss)))"##;
    let expect = expect!["OK (52 ((abyss) 52) ((abyss) 52) nil nil)"];

    assert_abyss_theme_parity(elisp_form, expect);
}
