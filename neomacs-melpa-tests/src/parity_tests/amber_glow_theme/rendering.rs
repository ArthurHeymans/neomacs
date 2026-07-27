use expect_test::expect;

use super::assert_amber_glow_theme_parity;

#[test]
fn amber_glow_theme_real_elisp_fontification_resolves_declared_syntax_colors() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'amber-glow)
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert
              ";; warm comment\n"
              "(defconst amber-value \"glow\")\n"
              "(defun amber-function (argument)\n"
              "  (if argument amber-value nil))\n")
             (font-lock-ensure)
             (mapcar
              (lambda (needle)
                (goto-char (point-min))
                (search-forward needle)
                (let* ((position
                        (match-beginning 0))
                       (face
                        (get-text-property
                         position 'face)))
                  (list
                   needle
                   face
                   (and face
                        (face-attribute
                         face :foreground nil t))
                   (and face
                        (face-attribute
                         face :background nil t))
                   (and face
                        (face-attribute
                         face :weight nil t)))))
              '("warm comment" "defconst"
                "amber-value" "\"glow\""
                "defun" "amber-function"
                "if" "nil"))))
       (disable-theme 'amber-glow))"##;
    let expect = expect![[
        r##"OK (("warm comment" font-lock-comment-face "#7D6C4B" unspecified unspecified) ("defconst" font-lock-keyword-face "#5E3724" unspecified unspecified) ("amber-value" font-lock-variable-name-face "#6AC24E" unspecified unspecified) ("\"glow\"" font-lock-string-face "#93655E" unspecified unspecified) ("defun" font-lock-keyword-face "#5E3724" unspecified unspecified) ("amber-function" font-lock-function-name-face "#C87850" unspecified unspecified) ("if" font-lock-keyword-face "#5E3724" unspecified unspecified) ("nil" nil nil nil nil))"##
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_region_highlight_and_prompt_have_practical_contrast_pairs() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'amber-glow)
           (mapcar
            (lambda (face)
              (list
               face
               (face-attribute
                face :foreground nil t)
               (face-attribute
                face :background nil t)
               (color-values
                (face-attribute
                 face :foreground nil t))
               (color-values
                (face-attribute
                 face :background nil t))))
            '(default region highlight
              mode-line mode-line-inactive
              minibuffer-prompt)))
       (disable-theme 'amber-glow))"##;
    let expect = expect![[
        r##"OK ((default "#EDE6D6" "#15130C" #2=(65535 65535 65535) #1=(65535 0 0)) (region unspecified "#362F21" nil #1#) (highlight "#15130C" "#EDE6D6" #1# #2#) (mode-line "#EDE6D6" "#362F21" #2# #1#) (mode-line-inactive "#EDE6D6" "#15130C" #2# #1#) (minibuffer-prompt "#945738" unspecified #1# nil))"##
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_undeclared_faces_retain_baseline_attributes() {
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
                             face :underline nil t)))
                         '(link error success
                           line-number tooltip))))
         (unwind-protect
             (progn
               (enable-theme 'amber-glow)
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
                     face :underline nil t)))
                 '(link error success
                   line-number tooltip))))
           (disable-theme 'amber-glow)))"##;
    let expect = expect![[
        r##"OK (((link unspecified unspecified t) (error unspecified unspecified unspecified) (success unspecified unspecified unspecified) (line-number "unspecified-fg" "unspecified-bg" nil) (tooltip unspecified unspecified unspecified)) ((link unspecified unspecified t) (error unspecified unspecified unspecified) (success unspecified unspecified unspecified) (line-number "#EDE6D6" "#15130C" nil) (tooltip unspecified unspecified unspecified)))"##
    ]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}

#[test]
fn amber_glow_theme_warning_face_sets_bold_without_overriding_background() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'amber-glow)
           (list
            (face-attribute
             'font-lock-warning-face
             :foreground nil t)
            (face-attribute
             'font-lock-warning-face
             :background nil t)
            (face-attribute
             'font-lock-warning-face
             :weight nil t)
            (face-attribute
             'font-lock-warning-face
             :inherit nil t)))
       (disable-theme 'amber-glow))"##;
    let expect = expect![[r##"OK ("#EDE6D6" unspecified bold unspecified)"##]];
    assert_amber_glow_theme_parity(elisp_form, expect);
}
