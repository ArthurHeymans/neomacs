use expect_test::expect;

use super::{
    assert_ample_flat_theme_parity, assert_ample_light_theme_parity, assert_ample_theme_parity,
};

#[test]
fn ample_theme_dark_enable_and_disable_apply_and_restore_core_palette() {
    let elisp_form = r##"(let ((before
                        (mapcar
                         (lambda (face)
                           (list
                            face
                            (face-attribute
                             face :foreground nil t)
                            (face-attribute
                             face :background nil t)))
                         '(default cursor region
                           font-lock-keyword-face
                           font-lock-string-face
                           mode-line))))
         (unwind-protect
             (progn
               (enable-theme 'ample)
               (let ((enabled
                      (mapcar
                       (lambda (face)
                         (list
                          face
                          (face-attribute
                           face :foreground nil t)
                          (face-attribute
                           face :background nil t)))
                       '(default cursor region
                         font-lock-keyword-face
                         font-lock-string-face
                         mode-line))))
                 (disable-theme 'ample)
                 (list
                  before enabled
                  (mapcar
                   (lambda (face)
                     (list
                      face
                      (face-attribute
                       face :foreground nil t)
                      (face-attribute
                       face :background nil t)))
                   '(default cursor region
                     font-lock-keyword-face
                     font-lock-string-face
                     mode-line))
                  custom-enabled-themes)))
           (disable-theme 'ample)))"##;
    let expect = expect![[
        r##"OK (((default "unspecified-fg" "unspecified-bg") (cursor unspecified "white") (region unspecified unspecified) (font-lock-keyword-face unspecified unspecified) (font-lock-string-face unspecified unspecified) (mode-line unspecified unspecified)) ((default "#bdbdb3" "gray13") (cursor "gray13" "#f57e00") (region unspecified "#303030") (font-lock-keyword-face "#5180b3" unspecified) (font-lock-string-face "#bdbc61" unspecified) (mode-line "#252525" "cornsilk4")) ((default "unspecified-fg" "unspecified-bg") (cursor unspecified "white") (region unspecified unspecified) (font-lock-keyword-face unspecified unspecified) (font-lock-string-face unspecified unspecified) (mode-line unspecified unspecified)) nil)"##
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_flat_enable_resolves_desaturated_core_palette() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ample-flat)
           (list
            custom-enabled-themes
            (mapcar
             (lambda (face)
               (list
                face
                (face-attribute
                 face :foreground nil t)
                (face-attribute
                 face :background nil t)
                (face-attribute
                 face :weight nil t)))
             '(default cursor region
               font-lock-comment-face
               font-lock-function-name-face
               mode-line minibuffer-prompt))))
       (disable-theme 'ample-flat))"##;
    let expect = expect![[
        r##"OK ((ample-flat) ((default "#bdbdb3" "gray15" normal) (cursor "gray15" "#afffef" unspecified) (region unspecified "#343030" unspecified) (font-lock-comment-face "#857575" unspecified unspecified) (font-lock-function-name-face "#a9df90" unspecified unspecified) (mode-line "#302525" "cornsilk4" unspecified) (minibuffer-prompt "#caca86" unspecified bold)))"##
    ]];
    assert_ample_flat_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_light_enable_resolves_light_core_palette() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ample-light)
           (list
            custom-enabled-themes
            (mapcar
             (lambda (face)
               (list
                face
                (face-attribute
                 face :foreground nil t)
                (face-attribute
                 face :background nil t)
                (face-attribute
                 face :weight nil t)))
             '(default cursor region
               font-lock-comment-face
               font-lock-function-name-face
               mode-line minibuffer-prompt))))
       (disable-theme 'ample-light))"##;
    let expect = expect![[
        r##"OK ((ample-light) ((default "gray43" "#cBc9b1" normal) (cursor "#cBc9b1" "#F57E00" unspecified) (region unspecified "#BBB9A1" unspecified) (font-lock-comment-face "#959595" unspecified unspecified) (font-lock-function-name-face "#4A8F30" unspecified unspecified) (mode-line "gray43" "#BBB9A1" unspecified) (minibuffer-prompt "#9B55C3" unspecified bold)))"##
    ]];
    assert_ample_light_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_interactive_loaders_call_load_theme_with_no_confirmation() {
    let elisp_form = r##"(let ((directory
                        (file-name-directory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE")))
               calls)
         (load
          (expand-file-name
           "ample-flat-theme.el" directory)
          nil t t)
         (load
          (expand-file-name
           "ample-light-theme.el" directory)
          nil t t)
         (cl-letf
             (((symbol-function 'load-theme)
               (lambda (&rest arguments)
                 (push arguments calls)
                 (car arguments))))
           (list
            (ample-theme)
            (ample-flat-theme)
            (ample-light-theme)
            (nreverse calls))))"##;
    let expect =
        expect!["OK (ample ample-flat ample-light ((ample t) (ample-flat t) (ample-light t)))"];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_triplet_precedence_changes_with_enable_order_and_unwinds_cleanly() {
    let elisp_form = r##"(let ((directory
                        (file-name-directory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE"))))
         (load
          (expand-file-name
           "ample-flat-theme.el" directory)
          nil t t)
         (load
          (expand-file-name
           "ample-light-theme.el" directory)
          nil t t)
         (unwind-protect
             (progn
               (enable-theme 'ample)
               (enable-theme 'ample-flat)
               (enable-theme 'ample-light)
               (let ((light-last
                      (list
                       custom-enabled-themes
                       (face-attribute
                        'default :foreground nil t)
                       (face-attribute
                        'default :background nil t))))
                 (disable-theme 'ample-light)
                 (let ((flat-next
                        (list
                         custom-enabled-themes
                         (face-attribute
                          'default :foreground nil t)
                         (face-attribute
                          'default :background nil t))))
                   (disable-theme 'ample-flat)
                   (list
                    light-last flat-next
                    custom-enabled-themes
                    (face-attribute
                     'default :foreground nil t)
                    (face-attribute
                     'default :background nil t)))))
           (mapc
            #'disable-theme
            '(ample-light
              ample-flat ample))))"##;
    let expect = expect![[
        r##"OK (((ample-light . #1=(ample-flat . #2=(ample))) "gray43" "#cBc9b1") (#1# "#bdbdb3" "gray15") #2# "#bdbdb3" "gray13")"##
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}
