use expect_test::expect;

use super::assert_amaranth_dark_theme_parity;

#[test]
fn enable_applies_core_palette_and_disable_restores_theme_and_frame_background_state() {
    let elisp_form = r##"(let ((before
                    (list
                     custom-enabled-themes
                     frame-background-mode
                     (face-attribute
                      'default :foreground nil t)
                     (face-attribute
                      'default :background nil t)))
                   during
                   after)
               (unwind-protect
                   (progn
                     (enable-theme 'amaranth-dark)
                     (setq
                      during
                      (list
                       custom-enabled-themes
                       (custom-theme-enabled-p
                        'amaranth-dark)
                       frame-background-mode
                       (face-attribute
                        'default :foreground nil t)
                       (face-attribute
                        'default :background nil t)
                       (face-attribute
                        'cursor :background nil t)
                       (face-attribute
                        'region :background nil t)
                       (face-attribute
                        'font-lock-keyword-face
                        :foreground nil t)
                       (face-attribute
                        'font-lock-keyword-face
                        :weight nil t)
                       (face-attribute
                        'font-lock-string-face
                        :foreground nil t)
                       (face-attribute
                        'font-lock-comment-face
                        :foreground nil t)
                       (face-attribute
                        'mode-line :background nil t)
                       (face-attribute
                        'mode-line :foreground nil t)))
                     (disable-theme 'amaranth-dark)
                     (setq
                      after
                      (list
                       custom-enabled-themes
                       (custom-theme-enabled-p
                        'amaranth-dark)
                       frame-background-mode
                       (face-attribute
                        'default :foreground nil t)
                       (face-attribute
                        'default :background nil t))))
                 (when
                     (custom-theme-enabled-p 'amaranth-dark)
                   (disable-theme 'amaranth-dark)))
               (list before during after))"##;
    let expect = expect![[
        r##"OK ((nil nil "unspecified-fg" "unspecified-bg") (#1=(amaranth-dark) #1# dark "#e4e4ef" "#000000" "#ffd966" "#4f4949" "#ffd966" bold "#598b43" "#7b7171" "#101010" "#ffffff") (nil nil nil "unspecified-fg" "unspecified-bg"))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn repeated_load_theme_keeps_one_enabled_entry_and_a_stable_complete_registry() {
    let elisp_form = r##"(let ((before
                    (length
                     (get 'amaranth-dark 'theme-settings)))
                   first
                   second)
               (unwind-protect
                   (progn
                     (load-theme 'amaranth-dark t)
                     (setq
                      first
                      (list
                       custom-enabled-themes
                       (length
                        (get
                         'amaranth-dark
                         'theme-settings))
                       (face-attribute
                        'default :background nil t)))
                     (load-theme 'amaranth-dark t)
                     (setq
                      second
                      (list
                       custom-enabled-themes
                       (length
                        (get
                         'amaranth-dark
                         'theme-settings))
                       (face-attribute
                        'default :background nil t))))
                 (when
                     (custom-theme-enabled-p 'amaranth-dark)
                   (disable-theme 'amaranth-dark)))
               (list
                before
                first
                second
                custom-enabled-themes
                (custom-theme-enabled-p 'amaranth-dark)))"##;
    let expect = expect![[
        r##"OK (129 ((amaranth-dark) 129 "#000000") ((amaranth-dark) 129 "#000000") nil nil)"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn optional_package_faces_defined_after_enable_receive_the_registered_theme_specs() {
    let elisp_form = r##"(unwind-protect
               (progn
                 (enable-theme 'amaranth-dark)
                 (eval
                  '(defface company-tooltip
                     '((t
                        (:foreground "fallback"
                         :background "fallback")))
                     "Parity company face."))
                 (eval
                  '(defface magit-log-head-label-remote
                     '((t
                        (:foreground "fallback"
                         :background "fallback")))
                     "Parity Magit face."))
                 (eval
                  '(defface orderless-match-face-0
                     '((t (:foreground "fallback")))
                     "Parity Orderless face."))
                 (eval
                  '(defface proof-locked-face
                     '((t (:background "fallback")))
                     "Parity Proof General face."))
                 (list
                  (face-attribute
                   'company-tooltip :foreground nil t)
                  (face-attribute
                   'company-tooltip :background nil t)
                  (face-attribute
                   'magit-log-head-label-remote
                   :foreground nil t)
                  (face-attribute
                   'magit-log-head-label-remote
                   :background nil t)
                  (face-attribute
                   'orderless-match-face-0
                   :foreground nil t)
                  (face-attribute
                   'proof-locked-face
                   :background nil t)))
             (disable-theme 'amaranth-dark))"##;
    let expect = expect![[r##"OK ("#e4e4ef" "#101010" "#598b43" "#101010" "#ffd966" "#303540")"##]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn disabling_theme_restores_explicit_preexisting_face_attributes_after_real_activation() {
    let elisp_form = r##"(let ((original-foreground
                    (face-attribute
                     'font-lock-string-face
                     :foreground nil nil))
                   (original-background
                    (face-attribute
                     'mode-line
                     :background nil nil))
                   during
                   after)
               (unwind-protect
                   (progn
                     (set-face-attribute
                      'font-lock-string-face nil
                      :foreground "#123456")
                     (set-face-attribute
                      'mode-line nil
                      :background "#654321")
                     (enable-theme 'amaranth-dark)
                     (setq
                      during
                      (list
                       (face-attribute
                        'font-lock-string-face
                        :foreground nil t)
                       (face-attribute
                        'mode-line
                        :background nil t)))
                     (disable-theme 'amaranth-dark)
                     (setq
                      after
                      (list
                       (face-attribute
                        'font-lock-string-face
                        :foreground nil nil)
                       (face-attribute
                        'mode-line
                        :background nil nil)))
                     (list during after))
                 (when
                     (custom-theme-enabled-p 'amaranth-dark)
                   (disable-theme 'amaranth-dark))
                 (set-face-attribute
                  'font-lock-string-face nil
                  :foreground original-foreground)
                 (set-face-attribute
                  'mode-line nil
                  :background original-background)))"##;
    let expect = expect![[r##"OK (("#598b43" "#101010") ("#123456" "#654321"))"##]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}
