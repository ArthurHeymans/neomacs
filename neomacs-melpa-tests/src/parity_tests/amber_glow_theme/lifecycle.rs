use expect_test::expect;

use super::assert_amber_glow_theme_parity;

#[test]
fn amber_glow_theme_enable_applies_complete_resolved_palette() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'amber-glow)
           (list
            (custom-theme-enabled-p
             'amber-glow)
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
                 face :weight nil t)
                (face-attribute
                 face :slant nil t)))
             '(default cursor fringe region
               highlight vertical-border
               font-lock-builtin-face
               font-lock-comment-face
               font-lock-constant-face
               font-lock-function-name-face
               font-lock-keyword-face
               font-lock-string-face
               font-lock-type-face
               font-lock-variable-name-face
               font-lock-warning-face
               mode-line mode-line-inactive
               minibuffer-prompt))))
       (disable-theme 'amber-glow))"##;
    let expect = expect![[
        r##"OK (#1=(amber-glow) #1# ((default "#EDE6D6" "#15130C" normal normal) (cursor unspecified "#EDE6D6" unspecified unspecified) (fringe unspecified "#15130C" unspecified unspecified) (region unspecified "#362F21" unspecified unspecified) (highlight "#15130C" "#EDE6D6" unspecified unspecified) (vertical-border "#EDE6D6" "#15130C" unspecified unspecified) (font-lock-builtin-face "#B28E63" unspecified unspecified unspecified) (font-lock-comment-face "#7D6C4B" unspecified unspecified unspecified) (font-lock-constant-face "#D19A66" unspecified unspecified unspecified) (font-lock-function-name-face "#C87850" unspecified unspecified unspecified) (font-lock-keyword-face "#5E3724" unspecified unspecified unspecified) (font-lock-string-face "#93655E" unspecified unspecified unspecified) (font-lock-type-face "#506948" unspecified unspecified unspecified) (font-lock-variable-name-face "#6AC24E" unspecified unspecified unspecified) (font-lock-warning-face "#EDE6D6" unspecified bold unspecified) (mode-line "#EDE6D6" "#362F21" unspecified unspecified) (mode-line-inactive "#EDE6D6" "#15130C" unspecified unspecified) (minibuffer-prompt "#945738" unspecified unspecified unspecified)))"##
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_disable_restores_preexisting_face_attributes() {
    let elisp_form = r##"(let ((before
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
                         '(default
                           font-lock-keyword-face
                           mode-line))))
         (unwind-protect
             (progn
               (enable-theme 'amber-glow)
               (disable-theme 'amber-glow)
               (list
                before
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
                 '(default
                   font-lock-keyword-face
                   mode-line))
                (custom-theme-enabled-p
                 'amber-glow)
                custom-enabled-themes))
           (disable-theme 'amber-glow)))"##;
    let expect = expect![[
        r#"OK (((default "unspecified-fg" "unspecified-bg" normal) (font-lock-keyword-face unspecified unspecified bold) (mode-line unspecified unspecified unspecified)) ((default "unspecified-fg" "unspecified-bg" normal) (font-lock-keyword-face unspecified unspecified bold) (mode-line unspecified unspecified unspecified)) nil nil)"#
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_repeated_enable_is_idempotent_and_disable_is_safe() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'amber-glow)
           (let ((once
                  (list
                   custom-enabled-themes
                   (face-attribute
                    'default :background nil t)
                   (face-attribute
                    'font-lock-string-face
                    :foreground nil t))))
             (enable-theme 'amber-glow)
             (let ((twice
                    (list
                     custom-enabled-themes
                     (face-attribute
                      'default :background nil t)
                     (face-attribute
                      'font-lock-string-face
                      :foreground nil t))))
               (disable-theme 'amber-glow)
               (disable-theme 'amber-glow)
               (list
                once twice
                (equal once twice)
                custom-enabled-themes
                (custom-theme-enabled-p
                 'amber-glow)))))
       (disable-theme 'amber-glow))"##;
    let expect = expect![[
        r##"OK (((amber-glow) "#15130C" "#93655E") ((amber-glow) "#15130C" "#93655E") t nil nil)"##
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_coexists_with_user_theme_and_preserves_precedence() {
    let elisp_form = r##"(let ((fixture 'amber-glow-fixture))
         (unless (custom-theme-p fixture)
           (deftheme amber-glow-fixture
             "Fixture theme."))
         (custom-theme-set-faces
          fixture
          '(default
            ((t
              (:background "#010203"
               :foreground "#FAFBFC")))))
         (unwind-protect
             (progn
               (enable-theme fixture)
               (enable-theme 'amber-glow)
               (let ((amber-last
                      (list
                       custom-enabled-themes
                       (face-attribute
                        'default :background nil t)
                       (face-attribute
                        'default :foreground nil t))))
                 (disable-theme 'amber-glow)
                 (list
                  amber-last
                  custom-enabled-themes
                  (face-attribute
                   'default :background nil t)
                  (face-attribute
                   'default :foreground nil t))))
           (disable-theme 'amber-glow)
           (disable-theme fixture)))"##;
    let expect = expect![[
        r##"OK (((amber-glow . #1=(amber-glow-fixture)) "#15130C" "#EDE6D6") #1# "#010203" "#FAFBFC")"##
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}
