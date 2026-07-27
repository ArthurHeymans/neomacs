use expect_test::expect;

use super::assert_ample_zen_theme_parity;

#[test]
fn loading_without_enable_registers_complete_theme_then_enable_applies_core_palette() {
    let elisp_form = r##"(let ((theme 'ample-zen))
  (unwind-protect
      (progn
        (load-theme theme t t)
        (let ((loaded
               (list
                (custom-theme-p theme)
                custom-enabled-themes
                (length (get theme 'theme-settings)))))
          (enable-theme theme)
          (list
           loaded
           (copy-sequence custom-enabled-themes)
           (face-attribute
            'default :background nil 'default)
           (face-attribute
            'default :foreground nil 'default)
           (face-attribute
            'cursor :background nil 'default)
           (face-attribute
            'region :background nil 'default))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK (((ample-zen user changed) nil 426) (ample-zen) "#212121" "#bdbdb3" "#cc8512" "#212121")"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn enabling_theme_applies_practical_core_font_lock_search_and_status_attributes() {
    let elisp_form = r##"(let ((theme 'ample-zen)
      (faces
       '((default :background :foreground)
         (cursor :background :foreground)
         (region :background :foreground)
         (mode-line :background :foreground :box :inverse-video)
         (mode-line-inactive :background :foreground :weight)
         (font-lock-comment-face :foreground :weight :slant)
         (font-lock-function-name-face :foreground :weight)
         (font-lock-keyword-face :foreground :weight)
         (font-lock-string-face :foreground)
         (font-lock-variable-name-face :foreground)
         (isearch :background :foreground :weight)
         (show-paren-match :background :foreground :weight)
         (show-paren-mismatch :background :foreground :weight)
         (success :foreground :weight)
         (warning :foreground :weight)
         (error :foreground :weight))))
  (unwind-protect
      (progn
        (load-theme theme t)
        (mapcar
         (lambda (entry)
           (cons
            (car entry)
            (mapcar
             (lambda (attribute)
               (list
                attribute
                (copy-tree
                 (face-attribute
                  (car entry) attribute nil 'default))))
             (cdr entry))))
         faces))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![[
        r##"OK ((default (:background "#212121") (:foreground "#bdbdb3")) (cursor (:background "#cc8512") (:foreground "#bdbdb3")) (region (:background "#212121") (:foreground "#bdbdb3")) (mode-line (:background "#212121") (:foreground "#bdbdb3") (:box nil) (:inverse-video t)) (mode-line-inactive (:background "#3b3b3b") (:foreground "#9b9b9b") (:weight light)) (font-lock-comment-face (:foreground "#6aaf50") (:weight normal) (:slant normal)) (font-lock-function-name-face (:foreground "#9b55c3") (:weight normal)) (font-lock-keyword-face (:foreground "#7d7c61") (:weight bold)) (font-lock-string-face (:foreground "#CC5542")) (font-lock-variable-name-face (:foreground "#fb8512")) (isearch (:background "#3b3b3b") (:foreground "#baba36") (:weight bold)) (show-paren-match (:background "#212121") (:foreground "#528fd1") (:weight bold)) (show-paren-mismatch (:background "#212121") (:foreground "#ff5542") (:weight bold)) (success (:foreground "#6aaf50") (:weight bold)) (warning (:foreground "#fb8512") (:weight bold)) (error (:foreground "#AA5542") (:weight bold)))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn disabling_theme_restores_preexisting_face_attributes_and_enabled_registry() {
    let elisp_form = r##"(let* ((theme 'ample-zen)
       (faces
        '(default region mode-line
          font-lock-comment-face
          font-lock-string-face
          show-paren-match))
       (attributes
        '(:background :foreground :weight :slant
          :underline :box :inherit :inverse-video))
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
                  (copy-tree
                   (face-attribute
                    face attribute nil 'default))))
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
        r##"OK (nil t nil ((default (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video nil)) (region (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video t)) (mode-line (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video t)) (font-lock-comment-face (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight bold) (:slant italic) (:underline nil) (:box nil) (:inherit nil) (:inverse-video nil)) (font-lock-string-face (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant italic) (:underline nil) (:box nil) (:inherit nil) (:inverse-video nil)) (show-paren-match (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline t) (:box nil) (:inherit underline) (:inverse-video nil))) ((default (:background "#212121") (:foreground "#bdbdb3") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video nil)) (region (:background "#212121") (:foreground "#bdbdb3") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video t)) (mode-line (:background "#212121") (:foreground "#bdbdb3") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video t)) (font-lock-comment-face (:background "#212121") (:foreground "#6aaf50") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video nil)) (font-lock-string-face (:background "#212121") (:foreground "#CC5542") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video nil)) (show-paren-match (:background "#212121") (:foreground "#528fd1") (:weight bold) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video nil))) ((default (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video nil)) (region (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video t)) (mode-line (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline nil) (:box nil) (:inherit nil) (:inverse-video t)) (font-lock-comment-face (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight bold) (:slant italic) (:underline nil) (:box nil) (:inherit nil) (:inverse-video nil)) (font-lock-string-face (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant italic) (:underline nil) (:box nil) (:inherit nil) (:inverse-video nil)) (show-paren-match (:background "unspecified-bg") (:foreground "unspecified-fg") (:weight normal) (:slant normal) (:underline t) (:box nil) (:inherit underline) (:inverse-video nil))))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn enabling_and_disabling_theme_applies_then_restores_all_five_theme_variables() {
    let elisp_form = r##"(let* ((theme 'ample-zen)
       (symbols
        '(ansi-color-names-vector
          fci-rule-color
          vc-annotate-color-map
          vc-annotate-very-old-color
          vc-annotate-background))
       (snapshot
        (lambda ()
          (mapcar
           (lambda (symbol)
             (list
              symbol
              (boundp symbol)
              (and (boundp symbol)
                   (copy-tree
                    (symbol-value symbol)))))
           symbols)))
       (before (funcall snapshot)))
  (unwind-protect
      (progn
        (load-theme theme t)
        (let ((during (funcall snapshot)))
          (disable-theme theme)
          (let ((after (funcall snapshot)))
            (list
             before during after
             (equal before after)))))
    (when (memq theme custom-enabled-themes)
      (disable-theme theme))))"##;
    let expect = expect![
        "OK (((ansi-color-names-vector nil nil) (fci-rule-color nil nil) (vc-annotate-color-map nil nil) (vc-annotate-very-old-color nil nil) (vc-annotate-background nil nil)) ((ansi-color-names-vector nil nil) (fci-rule-color nil nil) (vc-annotate-color-map nil nil) (vc-annotate-very-old-color nil nil) (vc-annotate-background nil nil)) ((ansi-color-names-vector nil nil) (fci-rule-color nil nil) (vc-annotate-color-map nil nil) (vc-annotate-very-old-color nil nil) (vc-annotate-background nil nil)) t)"
    ];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn loading_already_enabled_theme_is_idempotent_without_duplicate_registry_entries() {
    let elisp_form = r##"(let ((theme 'ample-zen))
  (unwind-protect
      (progn
        (load-theme theme t)
        (let ((before
               (list
                (copy-sequence custom-enabled-themes)
                (length (get theme 'theme-settings))
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
             (length (get theme 'theme-settings))
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
        r##"OK (t ((ample-zen) 426 "#212121" "#CC5542") (ample-zen) 426 "#212121" "#CC5542" 1)"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn reloading_source_preserves_single_complete_theme_definition_and_safe_form() {
    let elisp_form = r##"(let* ((before (copy-tree (get 'ample-zen 'theme-settings)))
       (safe-form
        '(when
             (require 'rainbow-mode nil t)
           (rainbow-mode 1)))
       (safe-count-before
        (length
         (delq nil
               (mapcar
                (lambda (form)
                  (and (equal form safe-form) form))
                safe-local-eval-forms)))))
  (load "ample-zen-theme" nil t)
  (let ((after (get 'ample-zen 'theme-settings)))
    (list
     (length before)
     (length after)
     (equal before after)
     safe-count-before
     (length
      (delq nil
            (mapcar
             (lambda (form)
               (and (equal form safe-form) form))
             safe-local-eval-forms)))
     (custom-theme-p 'ample-zen))))"##;
    let expect = expect!["OK (426 852 nil 1 1 (ample-zen user changed))"];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn enable_disable_enable_cycle_reapplies_identical_faces_and_theme_variables() {
    let elisp_form = r##"(let ((theme 'ample-zen)
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
           'font-lock-comment-face
           :foreground nil 'default)
          (and (boundp 'ansi-color-names-vector)
               (copy-sequence ansi-color-names-vector))
          (and (boundp 'vc-annotate-background)
               vc-annotate-background)))))
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
        r##"OK (((ample-zen) "#212121" "#bdbdb3" "#212121" "#6aaf50" nil nil) ((ample-zen) "#212121" "#bdbdb3" "#212121" "#6aaf50" nil nil) t)"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}
