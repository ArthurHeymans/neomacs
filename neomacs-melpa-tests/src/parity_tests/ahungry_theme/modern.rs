use expect_test::expect;

use super::assert_ahungry_theme_parity;

#[test]
fn ahungry_theme_selected_raw_face_specs_cover_core_programming_and_packages() {
    let elisp_form = r##"(mapcar
         (lambda (face)
           (list face (cdr (assq 'ahungry (get face 'theme-face)))))
         '(default
           cursor
           mode-line
           font-lock-comment-face
           font-lock-keyword-face
           font-lock-string-face
           diff-added
           diff-removed
           org-level-1
           org-link
           magit-diff-added
           helm-selection
           rainbow-delimiters-depth-5-face
           link
           hackernews-link))"##;
    let expect = expect![
        "OK ((default nil) (cursor nil) (mode-line nil) (font-lock-comment-face nil) (font-lock-keyword-face nil) (font-lock-string-face nil) (diff-added nil) (diff-removed nil) (org-level-1 nil) (org-link nil) (magit-diff-added nil) (helm-selection nil) (rainbow-delimiters-depth-5-face nil) (link nil) (hackernews-link nil))"
    ];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_default_face_contains_terminal_font_settings_and_transparent_background() {
    let elisp_form = r##"(let ((spec
                (cdr (assq 'ahungry (get 'default 'theme-face)))))
         (list
          spec
          (plist-get (cadr (car spec)) :foreground)
          (plist-get (cadr (car spec)) :background)
          (plist-get (cadr (car spec)) :family)
          (plist-get (cadr (car spec)) :foundry)
          (plist-get (cadr (car spec)) :height)))"##;
    let expect = expect!["OK (nil nil nil nil nil nil)"];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_enable_resolves_practical_core_face_attributes() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ahungry)
           (mapcar
            (lambda (face)
              (list
               face
               (face-attribute face :foreground nil 'default)
               (face-attribute face :background nil 'default)
               (face-attribute face :weight nil 'default)
               (face-attribute face :slant nil 'default)
               (face-attribute face :underline nil 'default)))
            '(default
              cursor
              mode-line
              font-lock-comment-face
              font-lock-keyword-face
              font-lock-string-face
              link)))
       (disable-theme 'ahungry))"##;
    let expect = expect![[
        r##"OK ((default "#ffffff" unspecified normal normal nil) (cursor "#ffffff" "#fce94f" normal normal nil) (mode-line "#0022aa" "#77ff00" bold normal nil) (font-lock-comment-face "#888a85" unspecified normal italic nil) (font-lock-keyword-face "#3cff00" unspecified bold normal nil) (font-lock-string-face "#ff0077" unspecified normal normal nil) (link "#33ff99" unspecified normal normal t))"##
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_disable_restores_prior_face_attributes_and_theme_state() {
    let elisp_form = r##"(let ((before
                (list
                 (face-attribute 'font-lock-keyword-face :foreground nil 'default)
                 (face-attribute 'mode-line :background nil 'default))))
         (enable-theme 'ahungry)
         (let ((during
                (list
                 (face-attribute 'font-lock-keyword-face :foreground nil 'default)
                 (face-attribute 'mode-line :background nil 'default)
                 (copy-sequence custom-enabled-themes))))
           (disable-theme 'ahungry)
           (list
            before
            during
            (list
             (face-attribute 'font-lock-keyword-face :foreground nil 'default)
             (face-attribute 'mode-line :background nil 'default)
             (copy-sequence custom-enabled-themes)))))"##;
    let expect = expect![[
        r##"OK (("unspecified-fg" "unspecified-bg") ("#3cff00" "#77ff00" (ahungry)) ("unspecified-fg" "unspecified-bg" nil))"##
    ]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_custom_variable_setting_applies_and_rolls_back() {
    let elisp_form = r##"(let ((original
                (and (boundp 'red) (symbol-value 'red))))
         (unwind-protect
             (progn
               (enable-theme 'ahungry)
               (let ((during
                      (list
                       (boundp 'red)
                       (and (boundp 'red) (symbol-value 'red))
                       (get 'red 'theme-value))))
                 (disable-theme 'ahungry)
                 (list
                  original
                  during
                  (and (boundp 'red) (symbol-value 'red))
                  (get 'red 'theme-value))))
           (when (memq 'ahungry custom-enabled-themes)
             (disable-theme 'ahungry))))"##;
    let expect = expect![[r##"OK (nil (nil nil ((ahungry "#ffffff"))) nil nil)"##]];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_nil_font_settings_reload_preserves_user_font_shape() {
    let elisp_form = r##"(let ((ahungry-theme-font-settings nil))
         (load (getenv "NEOMACS_PACKAGE_SOURCE") nil t t)
         (let ((spec
                (cdr (assq 'ahungry (get 'default 'theme-face)))))
           (list
            spec
            (plist-member (cadr (car spec)) :family)
            (plist-member (cadr (car spec)) :height)
            (plist-get (cadr (car spec)) :foreground)
            (plist-get (cadr (car spec)) :background))))"##;
    let expect = expect!["OK (nil nil nil nil nil)"];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_graphical_reload_selects_documented_dark_background() {
    let elisp_form = r##"(let ((ahungry-theme-font-settings nil))
         (cl-letf (((symbol-function 'display-graphic-p)
                    (lambda (&optional _display) t)))
           (load (getenv "NEOMACS_PACKAGE_SOURCE") nil t t))
         (let* ((spec
                 (cdr (assq 'ahungry (get 'default 'theme-face))))
                (attributes (cadr (car spec))))
           (list
            (plist-get attributes :foreground)
            (plist-get attributes :background)
            (plist-member attributes :family))))"##;
    let expect = expect!["OK (nil nil nil)"];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_link_override_and_rainbow_faces_have_final_intended_weights() {
    let elisp_form = r##"(mapcar
         (lambda (face)
           (let* ((spec
                   (cdr (assq 'ahungry (get face 'theme-face))))
                  (attributes (cadr (car spec))))
             (list
              face
              (plist-get attributes :foreground)
              (plist-get attributes :bold)
              (plist-get attributes :weight)
              (plist-get attributes :underline))))
         '(link
           hackernews-link
           rainbow-delimiters-depth-1-face
           rainbow-delimiters-depth-9-face
           rainbow-blocks-depth-1-face
           rainbow-blocks-depth-9-face))"##;
    let expect = expect![
        "OK ((link nil nil nil nil) (hackernews-link nil nil nil nil) (rainbow-delimiters-depth-1-face nil nil nil nil) (rainbow-delimiters-depth-9-face nil nil nil nil) (rainbow-blocks-depth-1-face nil nil nil nil) (rainbow-blocks-depth-9-face nil nil nil nil))"
    ];
    assert_ahungry_theme_parity(elisp_form, expect);
}

#[test]
fn ahungry_theme_repeated_enable_is_idempotent_and_disable_removes_one_entry() {
    let elisp_form = r##"(unwind-protect
         (progn
           (enable-theme 'ahungry)
           (enable-theme 'ahungry)
           (let ((enabled
                  (list
                   (copy-sequence custom-enabled-themes)
                   (seq-count (lambda (theme) (eq theme 'ahungry))
                              custom-enabled-themes)
                   (face-attribute 'font-lock-keyword-face
                                   :foreground nil 'default))))
             (disable-theme 'ahungry)
             (list enabled
                   (copy-sequence custom-enabled-themes))))
       (when (memq 'ahungry custom-enabled-themes)
         (disable-theme 'ahungry)))"##;
    let expect = expect![[r##"OK (((ahungry) 1 "#3cff00") nil)"##]];
    assert_ahungry_theme_parity(elisp_form, expect);
}
