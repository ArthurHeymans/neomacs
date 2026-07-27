use expect_test::expect;

use super::assert_almost_mono_themes_parity;

#[test]
fn enabling_each_variant_applies_practical_core_face_attributes() {
    let elisp_form = r##"(let ((themes
       '(almost-mono-white almost-mono-black
         almost-mono-gray almost-mono-cream))
      (attributes
       '((default :background :foreground)
         (region :background :foreground)
         (isearch :background :foreground :weight)
         (mode-line :background :foreground :box)
         (mode-line-inactive :background :foreground :box)
         (font-lock-comment-face :foreground :slant)
         (font-lock-string-face :foreground)
         (show-paren-match :background :foreground :weight)
         (show-paren-mismatch :background :foreground :weight))))
  (mapcar
   (lambda (theme)
     (unwind-protect
         (progn
           (load-theme theme t)
           (list
            theme
            (copy-sequence custom-enabled-themes)
            (mapcar
             (lambda (entry)
               (cons
                (car entry)
                (mapcar
                 (lambda (attribute)
                   (list
                    attribute
                    (face-attribute
                     (car entry) attribute nil 'default)))
                 (cdr entry))))
             attributes)))
       (when (memq theme custom-enabled-themes)
         (disable-theme theme))))
   themes))"##;
    let expect = expect![[
        r##"OK ((almost-mono-white (almost-mono-white) ((default (:background "#ffffff") (:foreground "#000000")) (region (:background "#fda50f") (:foreground "#000000")) (isearch (:background "#888888") (:foreground "#000000") (:weight bold)) (mode-line (:background "#efefef") (:foreground "#000000") (:box (:line-width -1 :color "#dddddd"))) (mode-line-inactive (:background "#ffffff") (:foreground "#dddddd") (:box (:line-width -1 :color "#dddddd"))) (font-lock-comment-face (:foreground "#888888") (:slant italic)) (font-lock-string-face (:foreground "#3c5e2b")) (show-paren-match (:background "#ffffff") (:foreground "#00ff00") (:weight bold)) (show-paren-mismatch (:background "#ffffff") (:foreground "#ff0000") (:weight bold)))) (almost-mono-black (almost-mono-black) ((default (:background "#000000") (:foreground "#ffffff")) (region (:background "#fda50f") (:foreground "#ffffff")) (isearch (:background "#aaaaaa") (:foreground "#ffffff") (:weight bold)) (mode-line (:background "#222222") (:foreground "#ffffff") (:box (:line-width -1 :color "#666666"))) (mode-line-inactive (:background "#000000") (:foreground "#666666") (:box (:line-width -1 :color "#666666"))) (font-lock-comment-face (:foreground "#aaaaaa") (:slant italic)) (font-lock-string-face (:foreground "#a7bca4")) (show-paren-match (:background "#000000") (:foreground "#00ff00") (:weight bold)) (show-paren-mismatch (:background "#000000") (:foreground "#ff0000") (:weight bold)))) (almost-mono-gray (almost-mono-gray) ((default (:background "#2b2b2b") (:foreground "#ffffff")) (region (:background "#fda50f") (:foreground "#ffffff")) (isearch (:background "#aaaaaa") (:foreground "#ffffff") (:weight bold)) (mode-line (:background "#222222") (:foreground "#ffffff") (:box (:line-width -1 :color "#666666"))) (mode-line-inactive (:background "#2b2b2b") (:foreground "#666666") (:box (:line-width -1 :color "#666666"))) (font-lock-comment-face (:foreground "#aaaaaa") (:slant italic)) (font-lock-string-face (:foreground "#a7bca4")) (show-paren-match (:background "#2b2b2b") (:foreground "#00ff00") (:weight bold)) (show-paren-mismatch (:background "#2b2b2b") (:foreground "#ff0000") (:weight bold)))) (almost-mono-cream (almost-mono-cream) ((default (:background "#f0e5da") (:foreground "#000000")) (region (:background "#fda50f") (:foreground "#000000")) (isearch (:background "#7d7165") (:foreground "#000000") (:weight bold)) (mode-line (:background "#dbd0c5") (:foreground "#000000") (:box (:line-width -1 :color "#c4baaf"))) (mode-line-inactive (:background "#f0e5da") (:foreground "#c4baaf") (:box (:line-width -1 :color "#c4baaf"))) (font-lock-comment-face (:foreground "#7d7165") (:slant italic)) (font-lock-string-face (:foreground "#3c5e2b")) (show-paren-match (:background "#f0e5da") (:foreground "#00ff00") (:weight bold)) (show-paren-mismatch (:background "#f0e5da") (:foreground "#ff0000") (:weight bold)))))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn loading_without_enable_then_enabling_uses_registered_theme_settings() {
    let elisp_form = r##"(let ((theme 'almost-mono-white))
  (unwind-protect
      (progn
        (load-theme theme t t)
        (let ((loaded
               (list
                (custom-theme-p theme)
                custom-enabled-themes
                (get theme 'theme-immediate)
                (length (get theme 'theme-settings)))))
          (enable-theme theme)
          (list
           loaded
           custom-enabled-themes
           (face-attribute 'default :background nil 'default)
           (face-attribute 'default :foreground nil 'default))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (((almost-mono-white user changed) nil t 73) (almost-mono-white) "#ffffff" "#000000")"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn enabling_two_variants_gives_latest_theme_precedence_then_restores_previous_theme() {
    let elisp_form = r##"(let ((white 'almost-mono-white)
      (black 'almost-mono-black))
  (unwind-protect
      (progn
        (load-theme white t)
        (let ((white-state
               (list
                (copy-sequence custom-enabled-themes)
                (face-attribute
                 'default :background nil 'default)
                (face-attribute
                 'font-lock-string-face
                 :foreground nil 'default))))
          (load-theme black t)
          (let ((black-state
                 (list
                  (copy-sequence custom-enabled-themes)
                  (face-attribute
                   'default :background nil 'default)
                  (face-attribute
                   'font-lock-string-face
                   :foreground nil 'default))))
            (disable-theme black)
            (list
             white-state
             black-state
             (copy-sequence custom-enabled-themes)
             (face-attribute
              'default :background nil 'default)
             (face-attribute
              'font-lock-string-face
              :foreground nil 'default)))))
    (dolist (theme (list black white))
      (when (memq theme custom-enabled-themes)
        (disable-theme theme)))))"##;
    let expect = expect![[
        r##"OK (((almost-mono-white) "#ffffff" "#3c5e2b") ((almost-mono-black almost-mono-white) "#000000" "#a7bca4") (almost-mono-white) "#ffffff" "#3c5e2b")"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn sequential_variant_switching_leaves_only_requested_theme_enabled() {
    let elisp_form = r##"(let ((themes
       '(almost-mono-white almost-mono-black
         almost-mono-gray almost-mono-cream))
      states)
  (unwind-protect
      (dolist (theme themes (nreverse states))
        (mapc #'disable-theme
              (copy-sequence custom-enabled-themes))
        (load-theme theme t)
        (push
         (list
          theme
          (copy-sequence custom-enabled-themes)
          (face-attribute
           'default :background nil 'default)
          (face-attribute
           'default :foreground nil 'default)
          (face-attribute
           'region :background nil 'default))
         states))
    (mapc #'disable-theme
          (copy-sequence custom-enabled-themes))))"##;
    let expect = expect![[
        r##"OK ((almost-mono-white (almost-mono-white) "#ffffff" "#000000" "#fda50f") (almost-mono-black (almost-mono-black) "#000000" "#ffffff" "#fda50f") (almost-mono-gray (almost-mono-gray) "#2b2b2b" "#ffffff" "#fda50f") (almost-mono-cream (almost-mono-cream) "#f0e5da" "#000000" "#fda50f"))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn disabling_theme_restores_pre_theme_face_attributes_and_registry_state() {
    let elisp_form = r##"(let* ((theme 'almost-mono-cream)
       (faces
        '(default region mode-line
          font-lock-comment-face
          font-lock-string-face))
       (attributes
        '(:background :foreground :weight
          :slant :underline :box :inherit))
       (snapshot
        (lambda ()
          (mapcar
           (lambda (face)
             (cons
              face
              (mapcar
               (lambda (attribute)
                 (list
                  attribute
                  (face-attribute
                   face attribute nil 'default)))
               attributes)))
           faces)))
       (before (funcall snapshot)))
  (unwind-protect
      (progn
        (load-theme theme t)
        (let ((during (funcall snapshot)))
          (disable-theme theme)
          (let ((after (funcall snapshot)))
            (list
             (equal before during)
             (equal before after)
             (memq theme custom-enabled-themes)
             before during after))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (nil t nil ((default (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil)) (region (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil)) (mode-line (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil)) (font-lock-comment-face (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight bold) (:slant italic) (:underline nil) (:box nil) (:inherit nil)) (font-lock-string-face (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant italic) (:underline nil) (:box nil) (:inherit nil))) ((default (:background "#f0e5da") (:foreground "#000000") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil)) (region (:background "#fda50f") (:foreground "#000000") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil)) (mode-line (:background "#dbd0c5") (:foreground "#000000") (:weight normal) (:slant normal) (:underline nil) (:box (:line-width -1 :color "#c4baaf")) (:inherit nil)) (font-lock-comment-face (:background "#f0e5da") (:foreground "#7d7165") (:weight normal) (:slant italic) (:underline nil) (:box nil) (:inherit nil)) (font-lock-string-face (:background "#f0e5da") (:foreground "#3c5e2b") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil))) ((default (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil)) (region (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil)) (mode-line (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil)) (font-lock-comment-face (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight bold) (:slant italic) (:underline nil) (:box nil) (:inherit nil)) (font-lock-string-face (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant italic) (:underline nil) (:box nil) (:inherit nil))))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn loading_an_already_enabled_variant_is_idempotent_without_duplicate_registry_entries() {
    let elisp_form = r##"(let ((theme 'almost-mono-gray))
  (unwind-protect
      (progn
        (load-theme theme t)
        (let ((before
               (list
                (copy-sequence custom-enabled-themes)
                (face-attribute
                 'default :background nil 'default)
                (face-attribute
                 'font-lock-string-face
                 :foreground nil 'default))))
          (let ((second-load (load-theme theme t)))
            (list
             second-load
             before
             (copy-sequence custom-enabled-themes)
             (face-attribute
              'default :background nil 'default)
             (face-attribute
              'font-lock-string-face
              :foreground nil 'default)
             (length
              (delq nil
                    (mapcar
                     (lambda (enabled)
                       (and (eq enabled theme) enabled))
                     custom-enabled-themes)))))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (t ((almost-mono-gray) "#2b2b2b" "#a7bca4") (almost-mono-gray) "#2b2b2b" "#a7bca4" 1)"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn enable_disable_enable_cycle_reapplies_same_rendering_attributes() {
    let elisp_form = r##"(let ((theme 'almost-mono-black)
      (snapshot
       (lambda ()
         (list
          (copy-sequence custom-enabled-themes)
          (face-attribute
           'default :background nil 'default)
          (face-attribute
           'default :foreground nil 'default)
          (face-attribute
           'region :background nil 'default)
          (face-attribute
           'mode-line :box nil 'default)
          (face-attribute
           'font-lock-comment-face :foreground nil 'default)))))
  (unwind-protect
      (progn
        (load-theme theme t t)
        (enable-theme theme)
        (let ((first (funcall snapshot)))
          (disable-theme theme)
          (enable-theme theme)
          (let ((second (funcall snapshot)))
            (list first second (equal first second)))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (((almost-mono-black) "#000000" "#ffffff" "#fda50f" #1=(:line-width -1 :color "#666666") "#aaaaaa") ((almost-mono-black) "#000000" "#ffffff" "#fda50f" #1# "#aaaaaa") t)"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}
