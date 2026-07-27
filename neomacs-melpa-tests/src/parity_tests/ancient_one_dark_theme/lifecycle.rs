use expect_test::expect;

use super::{
    assert_ancient_one_dark_theme_parity, assert_ancient_one_dark_theme_with_prelude_parity,
};

#[test]
fn ancient_one_dark_theme_enable_applies_core_palette_then_disable_restores_state() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))"##;
    let elisp_form = r##"(let ((before
                (list
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
               during
               after)
         (unwind-protect
             (progn
               (load-theme
                'ancient-one-dark
                t)
               (setq during
                     (list
                      (copy-sequence
                       custom-enabled-themes)
                      (and
                       (custom-theme-enabled-p
                        'ancient-one-dark)
                       t)
                      (face-attribute
                       'default :foreground nil t)
                      (face-attribute
                       'default :background nil t)
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
                       'region :foreground nil t)
                      (face-attribute
                       'region :background nil t)
                      (face-attribute
                       'mode-line :foreground nil t)
                      (face-attribute
                       'mode-line :background nil t)
                      (copy-tree
                       (face-attribute
                        'mode-line :box nil nil)))))
           (disable-theme
            'ancient-one-dark))
         (setq after
               (list
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
         (list
          before
          during
          after
          (equal before after)
          custom-enabled-themes
          (custom-theme-enabled-p
           'ancient-one-dark)))"##;
    let expect = expect![[
        r##"OK (("unspecified-fg" "unspecified-bg" unspecified bold) ((ancient-one-dark) t "#d1cad5" "#312843" "#8b76bc" bold "#f3cb89" "#312843" "#d1cad5" "#c0bac4" "#413952" (:line-width 3 :color "#413952")) ("unspecified-fg" "unspecified-bg" unspecified bold) t nil nil)"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_repeated_enable_is_idempotent_and_moves_theme_to_front() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))"##;
    let elisp_form = r##"(unwind-protect
         (progn
           (deftheme parity-underlay)
           (custom-theme-set-faces
            'parity-underlay
            '(default
              ((t
                (:foreground "underlay-fg"
                 :background "underlay-bg")))))
           (enable-theme
            'parity-underlay)
           (let ((first
                  (load-theme
                   'ancient-one-dark
                   t))
                 first-state
                 second-state)
             (setq first-state
                   (list
                    first
                    (copy-sequence
                     custom-enabled-themes)
                    (face-attribute
                     'default :foreground nil t)))
             (enable-theme
              'ancient-one-dark)
             (setq second-state
                   (list
                    (copy-sequence
                     custom-enabled-themes)
                    (face-attribute
                     'default :foreground nil t)
                    (let ((count 0))
                      (dolist
                          (theme
                           custom-enabled-themes
                           count)
                        (when
                            (eq theme
                                'ancient-one-dark)
                          (setq count
                                (1+ count)))))))
             (list
              first-state
              second-state)))
       (when
           (custom-theme-enabled-p
            'ancient-one-dark)
         (disable-theme
          'ancient-one-dark))
       (when
           (custom-theme-enabled-p
            'parity-underlay)
         (disable-theme
          'parity-underlay)))"##;
    let expect = expect![[
        r##"OK ((t (ancient-one-dark parity-underlay) "#d1cad5") ((ancient-one-dark parity-underlay) "#d1cad5" 1))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_disable_reveals_underlying_theme_attributes() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))"##;
    let elisp_form = r##"(unwind-protect
         (progn
           (deftheme parity-underlay)
           (custom-theme-set-faces
            'parity-underlay
            '(default
              ((t
                (:foreground "underlay-fg"
                 :background "underlay-bg"))))
            '(font-lock-keyword-face
              ((t
                (:foreground "underlay-keyword"
                 :weight light)))))
           (enable-theme
            'parity-underlay)
           (let (overlaid revealed)
             (load-theme
              'ancient-one-dark
              t)
             (setq overlaid
                   (list
                    (copy-sequence
                     custom-enabled-themes)
                    (face-attribute
                     'default :foreground nil t)
                    (face-attribute
                     'font-lock-keyword-face
                     :foreground nil t)
                    (face-attribute
                     'font-lock-keyword-face
                     :weight nil t)))
             (disable-theme
              'ancient-one-dark)
             (setq revealed
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
             (list overlaid revealed)))
       (when
           (custom-theme-enabled-p
            'ancient-one-dark)
         (disable-theme
          'ancient-one-dark))
       (when
           (custom-theme-enabled-p
            'parity-underlay)
         (disable-theme
          'parity-underlay)))"##;
    let expect = expect![[
        r##"OK (((ancient-one-dark parity-underlay) "#d1cad5" "#8b76bc" bold) ((parity-underlay) "underlay-fg" "underlay-bg" "underlay-keyword" light))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_optional_faces_defined_before_activation_resolve_practical_specs() {
    let prelude = r##"(progn
         (fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))
         (dolist
             (definition
              '((company-tooltip
                 (:foreground "fallback"
                  :background "fallback"))
                (magit-section-heading
                 (:foreground "fallback"
                  :weight normal))
                (web-mode-html-tag-face
                 (:foreground "fallback"))
                (rainbow-delimiters-depth-4-face
                 (:foreground "fallback"))))
           (eval
            `(defface
                 ,(car definition)
               '((t ,(cadr definition)))
               "Parity fixture."))))"##;
    let elisp_form = r##"(unwind-protect
         (progn
           (load-theme
            'ancient-one-dark
            t)
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
                face :inherit nil nil)))
            '(company-tooltip
              magit-section-heading
              web-mode-html-tag-face
              rainbow-delimiters-depth-4-face)))
       (disable-theme
        'ancient-one-dark))"##;
    let expect = expect![[
        r##"OK ((company-tooltip "#c0bac4" "#312843" bold unspecified) (magit-section-heading "#8b76bc" unspecified bold unspecified) (web-mode-html-tag-face "#b273b1" unspecified unspecified unspecified) (rainbow-delimiters-depth-4-face "#b273b1" unspecified unspecified unspecified))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_enabled_specs_apply_to_optional_faces_defined_late() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))"##;
    let elisp_form = r##"(unwind-protect
         (progn
           (load-theme
            'ancient-one-dark
            t)
           (dolist
               (definition
                '((helm-source-header
                   (:foreground "fallback"
                    :background "fallback"
                    :weight normal))
                  (company-tooltip-selection
                   (:foreground "fallback"
                    :background "fallback"))
                  (org-scheduled-today
                   (:foreground "fallback"
                    :weight normal))
                  (term-color-cyan
                   (:foreground "fallback"
                    :background "fallback"))))
             (eval
              `(defface
                   ,(car definition)
                 '((t ,(cadr definition)))
                 "Parity fixture.")))
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
                face :inherit nil nil)))
            '(helm-source-header
              company-tooltip-selection
              org-scheduled-today
              term-color-cyan)))
       (disable-theme
        'ancient-one-dark))"##;
    let expect = expect![[
        r##"OK ((helm-source-header "#8b76bc" "#312843" bold unspecified) (company-tooltip-selection "#b0aab3" "#524a61" unspecified unspecified) (org-scheduled-today "#8e7ed9" unspecified bold unspecified) (term-color-cyan "#f3cb89" "#f3cb89" unspecified unspecified))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_duplicate_term_black_setting_uses_last_source_declaration() {
    let prelude = r##"(progn
         (fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))
         (eval
          '(defface term-color-black
             '((t
                (:foreground "fallback"
                 :background "fallback")))
             "Parity fixture.")))"##;
    let elisp_form = r##"(let* ((settings
                     (reverse
                      (copy-sequence
                       (get
                        'ancient-one-dark
                        'theme-settings))))
                    (specs
                     (mapcar
                      (lambda (setting)
                        (copy-tree
                         (nth 3 setting)))
                      (seq-filter
                       (lambda (setting)
                         (eq
                          (cadr setting)
                          'term-color-black))
                       settings)))
                    resolved)
         (unwind-protect
             (progn
               (load-theme
                'ancient-one-dark
                t)
               (setq resolved
                     (list
                      (face-attribute
                       'term-color-black
                       :foreground nil t)
                      (face-attribute
                       'term-color-black
                       :background nil t))))
           (disable-theme
            'ancient-one-dark))
         (list specs resolved))"##;
    let expect = expect![[
        r##"OK ((((((class color) (min-colors 89)) (:foreground "#c0bac4" :background nil))) ((((class color) (min-colors 89)) (:foreground "#524a61" :background "#524a61")))) ("#c0bac4" unspecified))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_late_face_disable_restores_original_definition() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))"##;
    let elisp_form = r##"(let (during after)
         (unwind-protect
             (progn
               (load-theme
                'ancient-one-dark
                t)
               (eval
                '(defface company-preview-search
                   '((t
                      (:foreground "fallback-fg"
                       :background "fallback-bg")))
                   "Parity fixture."))
               (setq during
                     (list
                      (face-attribute
                       'company-preview-search
                       :foreground nil t)
                      (face-attribute
                       'company-preview-search
                       :background nil t)))
               (disable-theme
                'ancient-one-dark)
               (setq after
                     (list
                      (face-attribute
                       'company-preview-search
                       :foreground nil t)
                      (face-attribute
                       'company-preview-search
                       :background nil t))))
           (when
               (custom-theme-enabled-p
                'ancient-one-dark)
             (disable-theme
              'ancient-one-dark)))
         (list during after))"##;
    let expect = expect![[r##"OK (("#b273b1" "#312843") ("fallback-fg" "fallback-bg"))"##]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_theme_settings_are_structurally_independent() {
    let elisp_form = r##"(let* ((settings
                     (get
                      'ancient-one-dark
                      'theme-settings))
                    (find-spec
                     (lambda (face)
                       (nth
                        3
                        (seq-find
                         (lambda (setting)
                           (eq
                            (cadr setting)
                            face))
                         settings))))
                    (level-one
                     (funcall
                      find-spec
                      'rainbow-delimiters-depth-1-face))
                    (level-six
                     (funcall
                      find-spec
                      'rainbow-delimiters-depth-6-face))
                    (org-quote
                     (funcall
                      find-spec
                      'org-quote))
                    (org-verse
                     (funcall
                      find-spec
                      'org-verse)))
         (list
          (equal level-one level-six)
          (eq level-one level-six)
          (equal org-quote org-verse)
          (eq org-quote org-verse)
          (eq
           (cdar level-one)
           (cdar level-six))
          (eq
           (cdar org-quote)
           (cdar org-verse))))"##;
    let expect = expect!["OK (t nil t nil nil nil)"];

    assert_ancient_one_dark_theme_parity(elisp_form, expect);
}
