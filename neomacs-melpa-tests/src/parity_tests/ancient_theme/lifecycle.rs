use expect_test::expect;

use super::assert_ancient_theme_parity;

#[test]
fn ancient_theme_enable_and_disable_apply_then_restore_core_palette() {
    let elisp_form = r##"(let ((snapshot
                        (lambda ()
                          (mapcar
                           (lambda (face)
                             (list
                              face
                              (face-attribute
                               face :foreground nil t)
                              (face-attribute
                               face :background nil t)
                              (face-attribute
                               face :weight nil t)
                              (face-attribute
                               face :slant nil t)
                              (face-attribute
                               face :underline nil t)))
                           '(default cursor region
                             font-lock-comment-face
                             font-lock-keyword-face
                             font-lock-string-face
                             mode-line link)))))
         (let ((before (funcall snapshot)))
           (unwind-protect
               (progn
                 (enable-theme 'ancient)
                 (let ((enabled
                        (funcall snapshot)))
                   (disable-theme 'ancient)
                   (list
                    before
                    enabled
                    (funcall snapshot)
                    custom-enabled-themes)))
             (disable-theme 'ancient))))"##;
    let expect = expect![[
        r##"OK (((default "unspecified-fg" "unspecified-bg" normal normal nil) (cursor unspecified "white" unspecified unspecified unspecified) (region unspecified unspecified unspecified unspecified unspecified) (font-lock-comment-face unspecified unspecified bold italic unspecified) (font-lock-keyword-face unspecified unspecified bold unspecified unspecified) (font-lock-string-face unspecified unspecified unspecified italic unspecified) (mode-line unspecified unspecified unspecified unspecified unspecified) (link unspecified unspecified unspecified unspecified t)) ((default "#e8dcc8" "#1a1710" normal normal nil) (cursor unspecified "#3d8a6e" unspecified unspecified unspecified) (region unspecified "#4a4234" unspecified unspecified unspecified) (font-lock-comment-face "#665a48" unspecified unspecified italic unspecified) (font-lock-keyword-face "#3d8a6e" unspecified unspecified unspecified unspecified) (font-lock-string-face "#c8a05a" unspecified unspecified unspecified unspecified) (mode-line "#8a7a64" "#2d2820" unspecified unspecified unspecified) (link "#7ecfb4" unspecified unspecified unspecified t)) ((default "unspecified-fg" "unspecified-bg" normal normal nil) (cursor unspecified "white" unspecified unspecified unspecified) (region unspecified unspecified unspecified unspecified unspecified) (font-lock-comment-face unspecified unspecified bold italic unspecified) (font-lock-keyword-face unspecified unspecified bold unspecified unspecified) (font-lock-string-face unspecified unspecified unspecified italic unspecified) (mode-line unspecified unspecified unspecified unspecified unspecified) (link unspecified unspecified unspecified unspecified t)) nil)"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_repeated_enable_is_unique_and_disable_is_idempotent() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ancient)
           (enable-theme 'ancient)
           (let ((enabled
                  (list
                   custom-enabled-themes
                   (cl-count
                    'ancient
                    custom-enabled-themes)
                   (face-attribute
                    'default :foreground nil t)
                   (face-attribute
                    'default :background nil t))))
             (disable-theme 'ancient)
             (disable-theme 'ancient)
             (list
              enabled
              custom-enabled-themes
              (face-attribute
               'default :foreground nil t)
              (face-attribute
               'default :background nil t))))
       (disable-theme 'ancient))"##;
    let expect = expect![[
        r##"OK (((ancient) 1 "#e8dcc8" "#1a1710") nil "unspecified-fg" "unspecified-bg")"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_precedence_with_user_theme_unwinds_in_enable_order() {
    let elisp_form = r##"(progn
         (deftheme ancient-parity-overlay)
         (custom-theme-set-faces
          'ancient-parity-overlay
          '(default
             ((t
               (:foreground "#abcdef"
                :background "#123456"))))
          '(font-lock-keyword-face
             ((t
               (:foreground "#fedcba")))))
         (unwind-protect
             (progn
               (enable-theme 'ancient)
               (enable-theme
                'ancient-parity-overlay)
               (let ((overlay-last
                      (list
                       custom-enabled-themes
                       (face-attribute
                        'default :foreground nil t)
                       (face-attribute
                        'default :background nil t)
                       (face-attribute
                        'font-lock-keyword-face
                        :foreground nil t))))
                 (disable-theme
                  'ancient-parity-overlay)
                 (list
                  overlay-last
                  custom-enabled-themes
                  (face-attribute
                   'default :foreground nil t)
                  (face-attribute
                   'default :background nil t)
                  (face-attribute
                   'font-lock-keyword-face
                   :foreground nil t))))
           (mapc
            #'disable-theme
            '(ancient-parity-overlay
              ancient))))"##;
    let expect = expect![[
        r##"OK (((ancient-parity-overlay . #1=(ancient)) "#abcdef" "#123456" "#fedcba") #1# "#e8dcc8" "#1a1710" "#3d8a6e")"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}

#[test]
fn ancient_theme_box_underline_and_strike_attributes_resolve_exactly() {
    let elisp_form = r##"(let ((faces
                        '(mode-line mode-line-inactive
                          header-line tab-bar-tab link
                          flymake-error dired-broken-symlink
                          eglot-diagnostic-tag-deprecated-face)))
         (mapc
          (lambda (face)
            (unless (facep face)
              (make-face face)))
          faces)
         (unwind-protect
         (progn
           (enable-theme 'ancient)
           (mapcar
            (lambda (entry)
              (let ((face (car entry))
                    (attributes (cdr entry)))
                (cons
                 face
                 (mapcar
                  (lambda (attribute)
                    (list
                     attribute
                     (face-attribute
                      face attribute nil t)))
                  attributes))))
            '((mode-line :box)
              (mode-line-inactive :box)
              (header-line :box)
              (tab-bar-tab :box)
              (link :underline)
              (flymake-error :underline)
              (dired-broken-symlink
               :strike-through)
              (eglot-diagnostic-tag-deprecated-face
               :strike-through))))
           (disable-theme 'ancient)))"##;
    let expect = expect![[
        r##"OK ((mode-line (:box (:line-width 1 :color "#4a4234"))) (mode-line-inactive (:box (:line-width 1 :color "#2d2820"))) (header-line (:box (:line-width 1 :color "#2d2820"))) (tab-bar-tab (:box (:line-width 1 :color "#4a4234"))) (link (:underline t)) (flymake-error (:underline (:style wave :color "#e08c68"))) (dired-broken-symlink (:strike-through t)) (eglot-diagnostic-tag-deprecated-face (:strike-through t)))"##
    ]];
    assert_ancient_theme_parity(elisp_form, expect);
}
