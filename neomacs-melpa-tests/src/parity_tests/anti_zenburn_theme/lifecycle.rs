use expect_test::expect;

use super::assert_anti_zenburn_theme_parity;

#[test]
fn anti_zenburn_theme_enable_applies_representative_faces_then_disable_restores_state() {
    let elisp_form = r##"(let ((before
                    (list
                     custom-enabled-themes
                     (face-attribute
                      'default :foreground nil t)
                     (face-attribute
                      'default :background nil t)
                     (face-attribute
                      'mode-line :foreground nil t)
                     (face-attribute
                      'mode-line :background nil t)
                     (face-attribute
                      'region :background nil t)))
                   during
                   after)
         (unwind-protect
             (progn
               (load-theme
                'anti-zenburn
                t)
               (setq during
                     (list
                      (copy-sequence
                       custom-enabled-themes)
                      (custom-theme-enabled-p
                       'anti-zenburn)
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
                       :weight nil t)
                      (face-attribute
                       'region :background nil t)
                      (face-attribute
                       'region :inverse-video nil nil)))
               (disable-theme
                'anti-zenburn)
               (setq after
                     (list
                      (copy-sequence
                       custom-enabled-themes)
                      (custom-theme-enabled-p
                       'anti-zenburn)
                      (face-attribute
                       'default :foreground nil t)
                      (face-attribute
                       'default :background nil t)
                      (face-attribute
                       'mode-line :foreground nil t)
                      (face-attribute
                       'mode-line :background nil t)
                      (face-attribute
                       'region :background nil t))))
           (when
               (custom-theme-enabled-p
                'anti-zenburn)
             (disable-theme
              'anti-zenburn)))
         (list before during after))"##;
    let expect = expect![[
        r##"OK ((nil "unspecified-fg" "unspecified-bg" unspecified unspecified unspecified) ((anti-zenburn) (anti-zenburn) "#232333" "#c0c0c0" "#704d70" "#d4d4d4" (:line-width -1 :style released-button) "#806080" unspecified "#d4d4d4" unspecified) (nil nil "unspecified-fg" "unspecified-bg" unspecified unspecified unspecified))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_overlay_precedence_and_disable_restore_each_theme_layer() {
    let elisp_form = r##"(progn
         (eval
          '(deftheme
               anti-zenburn-parity-overlay
             "Parity overlay."))
         (custom-theme-set-faces
          'anti-zenburn-parity-overlay
          '(default
             ((t
               (:foreground "#102030"
                :background "#f0e0d0"))))
          '(font-lock-keyword-face
             ((t
               (:foreground "#aa00aa"
                :weight normal)))))
         (provide-theme
          'anti-zenburn-parity-overlay)
         (let (anti overlay restored)
           (unwind-protect
               (progn
                 (load-theme
                  'anti-zenburn
                  t)
                 (setq anti
                       (list
                        (copy-sequence
                         custom-enabled-themes)
                        (face-attribute
                         'default :foreground nil t)
                        (face-attribute
                         'default :background nil t)
                        (face-attribute
                         'font-lock-keyword-face
                         :foreground nil t)
                        (face-attribute
                         'font-lock-keyword-face
                         :weight nil t)))
                 (enable-theme
                  'anti-zenburn-parity-overlay)
                 (setq overlay
                       (list
                        (copy-sequence
                         custom-enabled-themes)
                        (face-attribute
                         'default :foreground nil t)
                        (face-attribute
                         'default :background nil t)
                        (face-attribute
                         'font-lock-keyword-face
                         :foreground nil t)
                        (face-attribute
                         'font-lock-keyword-face
                         :weight nil t)))
                 (disable-theme
                  'anti-zenburn-parity-overlay)
                 (setq restored
                       (list
                        (copy-sequence
                         custom-enabled-themes)
                        (face-attribute
                         'default :foreground nil t)
                        (face-attribute
                         'default :background nil t)
                        (face-attribute
                         'font-lock-keyword-face
                         :foreground nil t)
                        (face-attribute
                         'font-lock-keyword-face
                         :weight nil t))))
             (when
                 (custom-theme-enabled-p
                  'anti-zenburn-parity-overlay)
               (disable-theme
                'anti-zenburn-parity-overlay))
             (when
                 (custom-theme-enabled-p
                  'anti-zenburn)
               (disable-theme
                'anti-zenburn)))
           (list anti overlay restored)))"##;
    let expect = expect![[
        r##"OK (((anti-zenburn) "#232333" "#c0c0c0" "#0f2050" bold) ((anti-zenburn-parity-overlay anti-zenburn) "#102030" "#f0e0d0" "#aa00aa" normal) ((anti-zenburn) "#232333" "#c0c0c0" "#0f2050" bold))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_temporarily_overrides_an_enabled_baseline_theme_and_values() {
    let elisp_form = r##"(progn
         (defvar fci-rule-color
           "user-original")
         (eval
          '(deftheme
               anti-zenburn-parity-baseline
             "Parity baseline."))
         (custom-theme-set-faces
          'anti-zenburn-parity-baseline
          '(default
             ((t
               (:foreground "#112233"
                :background "#ddeeff"))))
          '(region
             ((t
               (:background "#abcdef")))))
         (custom-theme-set-variables
          'anti-zenburn-parity-baseline
          '(fci-rule-color
            "#baseline"))
         (provide-theme
          'anti-zenburn-parity-baseline)
         (let (baseline anti restored)
           (unwind-protect
               (progn
                 (enable-theme
                  'anti-zenburn-parity-baseline)
                 (setq baseline
                       (list
                        (copy-sequence
                         custom-enabled-themes)
                        (face-attribute
                         'default :foreground nil t)
                        (face-attribute
                         'default :background nil t)
                        (face-attribute
                         'region :background nil t)
                        fci-rule-color))
                 (load-theme
                  'anti-zenburn
                  t)
                 (setq anti
                       (list
                        (copy-sequence
                         custom-enabled-themes)
                        (face-attribute
                         'default :foreground nil t)
                        (face-attribute
                         'default :background nil t)
                        (face-attribute
                         'region :background nil t)
                        fci-rule-color))
                 (disable-theme
                  'anti-zenburn)
                 (setq restored
                       (list
                        (copy-sequence
                         custom-enabled-themes)
                        (face-attribute
                         'default :foreground nil t)
                        (face-attribute
                         'default :background nil t)
                        (face-attribute
                         'region :background nil t)
                        fci-rule-color)))
             (when
                 (custom-theme-enabled-p
                  'anti-zenburn)
               (disable-theme
                'anti-zenburn))
             (when
                 (custom-theme-enabled-p
                  'anti-zenburn-parity-baseline)
               (disable-theme
                'anti-zenburn-parity-baseline)))
           (list baseline anti restored)))"##;
    let expect = expect![[
        r##"OK (((anti-zenburn-parity-baseline) "#112233" "#ddeeff" "#abcdef" "#baseline") ((anti-zenburn anti-zenburn-parity-baseline) "#232333" "#c0c0c0" "#d4d4d4" "#c7c7c7") ((anti-zenburn-parity-baseline) "#112233" "#ddeeff" "#abcdef" "#baseline"))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_repeated_loads_keep_one_enabled_entry_and_stable_settings() {
    let elisp_form = r##"(let ((before
                    (length
                     (get
                      'anti-zenburn
                      'theme-settings)))
                   observations)
         (unwind-protect
             (progn
               (dolist (_ '(first second third))
                 (load-theme
                  'anti-zenburn
                  t)
                 (push
                  (list
                   (copy-sequence
                    custom-enabled-themes)
                   (length
                    (get
                     'anti-zenburn
                     'theme-settings))
                   (face-attribute
                    'default :foreground nil t)
                   (face-attribute
                    'default :background nil t))
                  observations))
               (list
                before
                (nreverse observations)))
           (when
               (custom-theme-enabled-p
                'anti-zenburn)
             (disable-theme
              'anti-zenburn))))"##;
    let expect = expect![[
        r##"OK (1058 (((anti-zenburn) 1058 "#232333" "#c0c0c0") ((anti-zenburn) 1058 "#232333" "#c0c0c0") ((anti-zenburn) 1058 "#232333" "#c0c0c0")))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}
