use expect_test::expect;

use super::{assert_acme_theme_parity, assert_acme_theme_with_prelude_parity};

#[test]
fn acme_theme_enable_applies_core_and_inherited_faces_then_disable_restores_attributes() {
    let elisp_form = r##"(let ((before
                (list
                 (face-attribute
                  'default
                  :foreground
                  nil
                  t)
                 (face-attribute
                  'default
                  :background
                  nil
                  t)
                 (face-attribute
                  'font-lock-keyword-face
                  :foreground
                  nil
                  t)
                 (face-attribute
                  'font-lock-keyword-face
                  :weight
                  nil
                  t)
                 (face-attribute
                  'link
                  :foreground
                  nil
                  t)
                 (face-attribute
                  'link
                  :weight
                  nil
                  t)
                 (face-attribute
                  'highlight
                  :inherit
                  nil
                  nil)
                 (face-attribute
                  'highlight
                  :foreground
                  nil
                  t)))
               during
               after)
         (unwind-protect
             (progn
               (load-theme
                'acme
                t)
               (setq during
                     (list
                      (copy-sequence
                       custom-enabled-themes)
                      (and
                       (custom-theme-enabled-p
                        'acme)
                       t)
                      (face-attribute
                       'default
                       :foreground
                       nil
                       t)
                      (face-attribute
                       'default
                       :background
                       nil
                       t)
                      (face-attribute
                       'cursor
                       :foreground
                       nil
                       t)
                      (face-attribute
                       'cursor
                       :background
                       nil
                       t)
                      (face-attribute
                       'region
                       :foreground
                       nil
                       t)
                      (face-attribute
                       'region
                       :background
                       nil
                       t)
                      (face-attribute
                       'region
                       :extend
                       nil
                       nil)
                      (face-attribute
                       'font-lock-keyword-face
                       :foreground
                       nil
                       t)
                      (face-attribute
                       'font-lock-keyword-face
                       :weight
                       nil
                       t)
                      (face-attribute
                       'link
                       :foreground
                       nil
                       t)
                      (face-attribute
                       'link
                       :weight
                       nil
                       t)
                      (face-attribute
                       'highlight
                       :inherit
                       nil
                       nil)
                      (face-attribute
                       'highlight
                       :foreground
                       nil
                       t))))
           (disable-theme
            'acme))
         (setq after
               (list
                (face-attribute
                 'default
                 :foreground
                 nil
                 t)
                (face-attribute
                 'default
                 :background
                 nil
                 t)
                (face-attribute
                 'font-lock-keyword-face
                 :foreground
                 nil
                 t)
                (face-attribute
                 'font-lock-keyword-face
                 :weight
                 nil
                 t)
                (face-attribute
                 'link
                 :foreground
                 nil
                 t)
                (face-attribute
                 'link
                 :weight
                 nil
                 t)
                (face-attribute
                 'highlight
                 :inherit
                 nil
                 nil)
                (face-attribute
                 'highlight
                 :foreground
                 nil
                 t)))
         (list
          before
          during
          after
          (equal
           before
           after)
          custom-enabled-themes
          (custom-theme-enabled-p
           'acme)))"##;
    let expect = expect![[
        r##"OK (("unspecified-fg" "unspecified-bg" unspecified bold unspecified unspecified unspecified unspecified) ((acme) t "#444444" "#FFFFE8" "#FFFFE8" "#444444" unspecified unspecified unspecified "#1054AF" bold "#0066cc" normal link "#0066cc") ("unspecified-fg" "unspecified-bg" unspecified bold unspecified unspecified unspecified unspecified) t nil nil)"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_black_foreground_option_applies_to_eligible_faces_in_batch() {
    let prelude = r##"(setq acme-theme-black-fg t)"##;
    let elisp_form = r##"(unwind-protect
         (progn
           (load-theme
            'acme
            t)
           (list
            acme-theme-black-fg
            (face-attribute
             'default
             :foreground
             nil
             t)
            (face-attribute
             'cursor
             :background
             nil
             t)
            (face-attribute
             'minibuffer-prompt
             :foreground
             nil
             t)
            (face-attribute
             'font-lock-function-name-face
             :foreground
             nil
             t)
            (face-attribute
             'mode-line
             :foreground
             nil
             t)))
       (disable-theme
        'acme))"##;
    let expect = expect![[r##"OK (t "#000000" "#000000" "#000000" "#000000" unspecified)"##]];
    assert_acme_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn acme_theme_enabled_specs_apply_to_optional_faces_defined_after_activation() {
    let elisp_form = r##"(let (during
               after)
         (unwind-protect
             (progn
               (load-theme
                'acme
                t)
               (eval
                '(defface
                     company-tooltip
                   '((t
                      (:background
                       "fallback")))
                   "Parity fixture."))
               (eval
                '(defface
                     magit-branch-current
                   '((t
                      (:foreground
                       "fallback")))
                   "Parity fixture."))
               (eval
                '(defface
                     org-todo
                   '((t
                      (:foreground
                       "fallback")))
                   "Parity fixture."))
               (eval
                '(defface
                     git-gutter:added
                   '((t
                      (:foreground
                       "fallback")))
                   "Parity fixture."))
               (setq during
                     (list
                      (face-attribute
                       'company-tooltip
                       :background
                       nil
                       t)
                      (face-attribute
                       'magit-branch-current
                       :foreground
                       nil
                       t)
                      (face-attribute
                       'magit-branch-current
                       :background
                       nil
                       t)
                      (face-attribute
                       'magit-branch-current
                       :box
                       nil
                       nil)
                      (face-attribute
                       'org-todo
                       :foreground
                       nil
                       t)
                      (face-attribute
                       'org-todo
                       :background
                       nil
                       t)
                      (face-attribute
                       'git-gutter:added
                       :foreground
                       nil
                       t)
                      (face-attribute
                       'git-gutter:added
                       :background
                       nil
                       t))))
           (disable-theme
            'acme))
         (setq after
               (list
                (face-attribute
                 'company-tooltip
                 :background
                 nil
                 t)
                (face-attribute
                 'magit-branch-current
                 :foreground
                 nil
                 t)
                (face-attribute
                 'org-todo
                 :foreground
                 nil
                 t)
                (face-attribute
                 'git-gutter:added
                 :foreground
                 nil
                 t)))
         (list
          during
          after))"##;
    let expect = expect![[
        r##"OK (("#E1FAFF" "#007777" "#A8EFEB" (:line-width 1 :color "#007777") "#888838" "#EFEFD8" "#006600" "#006600") ("fallback" "fallback" "fallback" "fallback"))"##
    ]];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_reloads_accumulate_exact_settings_but_keep_one_theme_path_entry() {
    let elisp_form = r##"(let* ((source
                     (getenv
                      "NEOMACS_PACKAGE_SOURCE"))
                    (directory
                     (file-name-as-directory
                      (file-name-directory
                       source)))
                    (custom-theme-load-path
                     '(sentinel))
                    observations)
         (dolist
             (_
              '(first second))
           (load
            source
            nil
            t
            t)
           (push
            (list
             (equal
              (car custom-theme-load-path)
              directory)
             (length custom-theme-load-path)
             (length
              (get
               'acme
               'theme-settings)))
            observations))
         (list
          (nreverse observations)
          (let ((count 0))
            (dolist
                (entry
                 custom-theme-load-path
                 count)
              (when
                  (equal
                   entry
                   directory)
                (setq count
                      (1+ count)))))))"##;
    let expect = expect!["OK (((t 2 628) (t 2 942)) 1)"];
    assert_acme_theme_parity(elisp_form, expect);
}

#[test]
fn acme_theme_repeated_load_theme_keeps_one_enabled_entry_and_stable_settings() {
    let elisp_form = r##"(let ((before
                (length
                 (get
                  'acme
                  'theme-settings)))
               first
               second)
         (unwind-protect
             (progn
               (load-theme
                'acme
                t)
               (setq first
                     (list
                      (copy-sequence
                       custom-enabled-themes)
                      (length
                       (get
                        'acme
                        'theme-settings))))
               (load-theme
                'acme
                t)
               (setq second
                     (list
                      (copy-sequence
                       custom-enabled-themes)
                      (length
                       (get
                        'acme
                        'theme-settings)))))
           (disable-theme
            'acme))
         (list
          before
          first
          second
          custom-enabled-themes
          (custom-theme-enabled-p
           'acme)))"##;
    let expect = expect!["OK (314 ((acme) 314) ((acme) 314) nil nil)"];
    assert_acme_theme_parity(elisp_form, expect);
}
