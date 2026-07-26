use expect_test::expect;

use super::assert_abyss_theme_parity;

#[test]
fn abyss_theme_core_ui_and_whitespace_face_specs_match_every_literal_attribute() {
    let elisp_form = r##"(let ((settings
                    (get 'abyss 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((remaining settings)
                        value)
                    (while remaining
                      (when
                          (eq
                           face
                           (cadr (car remaining)))
                        (setq value
                              (nth 3 (car remaining))
                              remaining nil))
                      (when remaining
                        (setq remaining
                              (cdr remaining))))
                    (list face (copy-tree value))))
                '(bold
                  bold-italic
                  border-glyph
                  default
                  fringe
                  buffers-tab
                  gui-element
                  text-cursor
                  region
                  italic
                  left-margin
                  toolbar
                  whitespace-tab
                  whitespace-line
                  magit-item-highlight)))"##;
    let expect = expect![[
        r##"OK ((bold ((t (:bold t)))) (bold-italic ((t (:bold t)))) (border-glyph ((t (nil)))) (default ((t (:foreground "#bbe0f0" :background "#050000")))) (fringe ((t (:background "#0d1000")))) (buffers-tab ((t (:foreground "#bbe0f0" :background "#050000")))) (gui-element ((t (:foreground "#0d1000" :background "#bbe0f0")))) (text-cursor ((t (:foreground "#bbe0f0" :background "#050000")))) (region ((t (:foreground "#050000" :background "#cc79a7")))) (italic ((t (nil)))) (left-margin ((t (nil)))) (toolbar ((t (nil)))) (whitespace-tab ((t (:background "#050000")))) (whitespace-line ((t (:foreground "#ffffff" :background "#dd5542")))) (magit-item-highlight ((t (:inherit region)))))"##
    ]];

    assert_abyss_theme_parity(elisp_form, expect);
}

#[test]
fn abyss_theme_font_lock_face_specs_match_every_literal_attribute() {
    let elisp_form = r##"(let ((settings
                    (get 'abyss 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((remaining settings)
                        value)
                    (while remaining
                      (when
                          (eq
                           face
                           (cadr (car remaining)))
                        (setq value
                              (nth 3 (car remaining))
                              remaining nil))
                      (when remaining
                        (setq remaining
                              (cdr remaining))))
                    (list face (copy-tree value))))
                '(font-lock-builtin-face
                  font-lock-comment-delimiter-face
                  font-lock-comment-face
                  font-lock-constant-face
                  font-lock-doc-face
                  font-lock-doc-string-face
                  font-lock-string-face
                  font-lock-function-name-face
                  font-lock-keyword-face
                  font-lock-preprocessor-face
                  font-lock-type-face
                  font-lock-variable-name-face
                  font-lock-negation-char-face
                  font-lock-warning-face)))"##;
    let expect = expect![[
        r##"OK ((font-lock-builtin-face ((t (:foreground "#fcfbe3")))) (font-lock-comment-delimiter-face ((t (:foreground "#d55e00" :italic t)))) (font-lock-comment-face ((t (:foreground "#d55e00" :italic t)))) (font-lock-constant-face ((t (:foreground "#cc79a7")))) (font-lock-doc-face ((t (:foreground "#e69f00")))) (font-lock-doc-string-face ((t (:foreground "#d55e00")))) (font-lock-string-face ((t (:foreground "#ff00ff")))) (font-lock-function-name-face ((t (:foreground "#56b4e9")))) (font-lock-keyword-face ((t (:foreground "#f8ec59")))) (font-lock-preprocessor-face ((t (:foreground "#0072b2")))) (font-lock-type-face ((t (:foreground "#56b4e9")))) (font-lock-variable-name-face ((t (:foreground "#00ff00")))) (font-lock-negation-char-face ((t (:foreground "#cc79a7")))) (font-lock-warning-face ((t (:foreground "#FF1A00" :bold t)))))"##
    ]];

    assert_abyss_theme_parity(elisp_form, expect);
}

#[test]
fn abyss_theme_mode_line_status_and_envrc_face_specs_match_every_literal_attribute() {
    let elisp_form = r##"(let ((settings
                    (get 'abyss 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((remaining settings)
                        value)
                    (while remaining
                      (when
                          (eq
                           face
                           (cadr (car remaining)))
                        (setq value
                              (nth 3 (car remaining))
                              remaining nil))
                      (when remaining
                        (setq remaining
                              (cdr remaining))))
                    (list face (copy-tree value))))
                '(mode-line
                  mode-line-highlight
                  mode-line-emphasis
                  mode-line-buffer-id
                  mode-line-inactive
                  success
                  warning
                  error
                  envrc-mode-line-on-face
                  envrc-mode-line-error-face
                  envrc-mode-line-none-face)))"##;
    let expect = expect![[
        r##"OK ((mode-line ((t (:foreground "#050000" :background "#56b4e9" :box nil)))) (mode-line-highlight ((t (:foreground "#ffffff" :weight bold :box nil)))) (mode-line-emphasis ((t (:foreground "#050000" :weight bold)))) (mode-line-buffer-id ((t (:foreground "#050000" :weight bold)))) (mode-line-inactive ((t (:foreground "#cc79a7" :background "#0d1000" :box nil)))) (success ((t (:foreground "#009e73" :weight bold)))) (warning ((t (:foreground "#050000" :weight bold)))) (error ((t (:foreground "#FF1A00" :weight bold)))) (envrc-mode-line-on-face ((t (:inherit nil :foreground "#009e73" :weight bold)))) (envrc-mode-line-error-face ((t (:inherit nil :foreground "#FF1A00" :weight bold)))) (envrc-mode-line-none-face ((t (:inherit nil :foreground "#050000" :weight bold)))))"##
    ]];

    assert_abyss_theme_parity(elisp_form, expect);
}

#[test]
fn abyss_theme_flycheck_and_compilation_face_specs_match_every_literal_attribute() {
    let elisp_form = r##"(let ((settings
                    (get 'abyss 'theme-settings)))
               (mapcar
                (lambda (face)
                  (let ((remaining settings)
                        value)
                    (while remaining
                      (when
                          (eq
                           face
                           (cadr (car remaining)))
                        (setq value
                              (nth 3 (car remaining))
                              remaining nil))
                      (when remaining
                        (setq remaining
                              (cdr remaining))))
                    (list face (copy-tree value))))
                '(flycheck-error
                  flycheck-warning
                  flycheck-info
                  flycheck-fringe-error
                  flycheck-fringe-warning
                  flycheck-fringe-info
                  compilation-error
                  compilation-warning
                  compilation-info
                  compilation-mode-line-exit
                  compilation-mode-line-fail
                  compilation-mode-line-run)))"##;
    let expect = expect![[
        r##"OK ((flycheck-error ((t (:foreground "#FF1A00" :weight bold)))) (flycheck-warning ((t (:foreground "#050000" :weight bold)))) (flycheck-info ((t (:foreground "#009e73" :weight bold)))) (flycheck-fringe-error ((t (:foreground "#FF1A00")))) (flycheck-fringe-warning ((t (:foreground "#e69f00")))) (flycheck-fringe-info ((t (:foreground "#009e73")))) (compilation-error ((t (:foreground "#FF1A00" :weight bold)))) (compilation-warning ((t (:foreground "#050000" :weight bold)))) (compilation-info ((t (:foreground "#009e73" :weight bold)))) (compilation-mode-line-exit ((t (:foreground "#009e73" :weight bold)))) (compilation-mode-line-fail ((t (:foreground "#FF1A00" :weight bold)))) (compilation-mode-line-run ((t (:foreground "#050000" :weight bold)))))"##
    ]];

    assert_abyss_theme_parity(elisp_form, expect);
}

#[test]
fn abyss_theme_backquoted_repeated_attribute_tails_have_independent_identity() {
    let elisp_form = r##"(let* ((settings
                     (get 'abyss 'theme-settings))
                    (find-spec
                     (lambda (face)
                       (catch 'found
                         (dolist (setting settings)
                           (when
                               (eq face (cadr setting))
                             (throw
                              'found
                              (nth 3 setting)))))))
                    (flycheck-error-plist
                     (cadar
                      (funcall
                       find-spec
                       'flycheck-error)))
                    (compilation-error-plist
                     (cadar
                      (funcall
                       find-spec
                       'compilation-error)))
                    (comment-plist
                     (cadar
                      (funcall
                       find-spec
                       'font-lock-comment-face)))
                    (comment-delimiter-plist
                     (cadar
                      (funcall
                       find-spec
                       'font-lock-comment-delimiter-face)))
                    (mode-line-emphasis-plist
                     (cadar
                      (funcall
                       find-spec
                       'mode-line-emphasis)))
                    (mode-line-buffer-id-plist
                     (cadar
                      (funcall
                       find-spec
                       'mode-line-buffer-id))))
               (list
                (eq
                 (cddr flycheck-error-plist)
                 (cddr compilation-error-plist))
                (eq
                 (cddr comment-plist)
                 (cddr comment-delimiter-plist))
                (eq
                 (cddr mode-line-emphasis-plist)
                 (cddr mode-line-buffer-id-plist))))"##;
    let expect = expect!["OK (nil nil nil)"];

    assert_abyss_theme_parity(elisp_form, expect);
}
