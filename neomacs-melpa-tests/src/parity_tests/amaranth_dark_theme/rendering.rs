use expect_test::expect;

use super::assert_amaranth_dark_theme_parity;

#[test]
fn real_emacs_lisp_fontification_maps_tokens_to_theme_faces_and_resolved_colors() {
    let elisp_form = r##"(unwind-protect
               (progn
                 (enable-theme 'amaranth-dark)
                 (with-temp-buffer
                   (emacs-lisp-mode)
                   (insert
                    ";; fixture comment\n\
(defun amaranth-fixture (value)\n\
  (let ((message \"green string\"))\n\
    (if value message nil)))\n")
                   (font-lock-ensure)
                   (mapcar
                    (lambda (token)
                      (goto-char (point-min))
                      (search-forward token)
                      (let* ((position
                              (1- (point)))
                             (face
                              (or
                               (get-text-property
                                position 'face)
                               (get-text-property
                                position 'font-lock-face))))
                        (list
                         token
                         face
                         (and
                          (facep face)
                          (face-attribute
                           face :foreground nil t))
                         (and
                          (facep face)
                          (face-attribute
                           face :weight nil t)))))
                    '("comment"
                      "defun"
                      "amaranth-fixture"
                      "value"
                      "\"green string\""
                      "if"
                      "nil"))))
             (disable-theme 'amaranth-dark))"##;
    let expect = expect![[
        r##"OK (("comment" font-lock-comment-face "#7b7171" unspecified) ("defun" font-lock-keyword-face "#ffd966" bold) ("amaranth-fixture" font-lock-function-name-face "#97a1b5" unspecified) ("value" nil nil nil) ("\"green string\"" font-lock-string-face "#598b43" unspecified) ("if" font-lock-keyword-face "#ffd966" bold) ("nil" nil nil nil))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn concrete_editor_search_selection_whitespace_and_modeline_attributes_match_rendering_contract() {
    let elisp_form = r##"(progn
               (require 'isearch)
               (require 'whitespace)
               (dolist
                   (face
                    '(isearch-lazy-highlight-face
                      whitespace-line))
                 (unless (facep face)
                   (eval
                    `(defface ,face
                       '((t (:inherit default)))
                       "Parity compatibility face."))))
               (unwind-protect
                   (progn
                     (enable-theme 'amaranth-dark)
                     (mapcar
                  (lambda (entry)
                    (let ((face (car entry))
                          (attributes (cdr entry)))
                      (cons
                       face
                       (mapcar
                        (lambda (attribute)
                          (cons
                           attribute
                           (face-attribute
                            face attribute nil t)))
                        attributes))))
                  '((default
                     :foreground :background)
                    (cursor :background)
                    (region
                     :foreground :background)
                    (isearch
                     :foreground :background)
                    (isearch-fail
                     :foreground :background)
                    (isearch-lazy-highlight-face
                     :foreground :background)
                    (trailing-whitespace
                     :foreground :background)
                    (whitespace-line
                     :foreground :background)
                    (mode-line
                     :foreground :background)
                    (mode-line-inactive
                     :foreground :background)
                    (tooltip
                     :foreground :background))))
                 (disable-theme 'amaranth-dark)))"##;
    let expect = expect![[
        r##"OK ((default (:foreground . "#e4e4ef") (:background . "#000000")) (cursor (:background . "#ffd966")) (region (:foreground . unspecified) (:background . "#4f4949")) (isearch (:foreground . "#000000") (:background . "#f5f5f5")) (isearch-fail (:foreground . "#000000") (:background . "#a02e2e")) (isearch-lazy-highlight-face (:foreground . "#f4f4ff") (:background . "#616775")) (trailing-whitespace (:foreground . "#000000") (:background . "#a02e2e")) (whitespace-line (:foreground . "#c81a1a") (:background . "#302d2d")) (mode-line (:foreground . "#ffffff") (:background . "#101010")) (mode-line-inactive (:foreground . "#959da3") (:background . "#101010")) (tooltip (:foreground . "#ffffff") (:background . "#7b7171")))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn concrete_inheritance_weight_and_unspecified_attributes_match_theme_intent() {
    let elisp_form = r##"(progn
               (require 'compile)
               (require 'dired)
               (require 'tab-bar)
               (dolist
                   (face
                    '(completions-annotations
                      compilation-info
                      compilation-warning
                      dired-ignored
                      line-number
                      line-number-current-line
                      tab-bar-tab
                      tab-bar-tab-inactive))
                 (unless (facep face)
                   (eval
                    `(defface ,face
                       '((t (:inherit default)))
                       "Parity compatibility face."))))
               (unwind-protect
                   (progn
                     (enable-theme 'amaranth-dark)
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
                            face attribute nil nil)
                           (face-attribute
                            face attribute nil t)))
                        attributes))))
                  '((completions-annotations
                     :inherit :foreground)
                    (compilation-info
                     :inherit :foreground)
                    (compilation-warning
                     :inherit :foreground :weight)
                    (dired-ignored
                     :inherit :foreground)
                    (line-number
                     :inherit :foreground)
                    (line-number-current-line
                     :inherit :foreground :weight)
                    (tab-bar-tab
                     :background :foreground :weight)
                    (tab-bar-tab-inactive
                     :background :foreground))))
                 (disable-theme 'amaranth-dark)))"##;
    let expect = expect![[
        r##"OK ((completions-annotations (:inherit #1='shadow #1#) (:foreground unspecified unspecified)) (compilation-info (:inherit unspecified unspecified) (:foreground "#598b43" "#598b43")) (compilation-warning (:inherit unspecified unspecified) (:foreground "#7b7171" "#7b7171") (:weight bold bold)) (dired-ignored (:inherit unspecified unspecified) (:foreground "#959da3" "#959da3")) (line-number (:inherit default default) (:foreground "#7b7171" "#7b7171")) (line-number-current-line (:inherit line-number line-number) (:foreground "#ffd966" "#ffd966") (:weight unspecified normal)) (tab-bar-tab (:background unspecified unspecified) (:foreground "#ffd966" "#ffd966") (:weight bold bold)) (tab-bar-tab-inactive (:background unspecified unspecified) (:foreground unspecified unspecified)))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn conditional_diagnostic_faces_select_a_concrete_branch_and_retain_registered_fallbacks() {
    let elisp_form = r##"(unwind-protect
               (progn
                 (dolist
                     (definition
                      '((flymake-errline
                         "Flymake error.")
                        (flymake-warnline
                         "Flymake warning.")
                        (flymake-infoline
                         "Flymake info.")
                        (flyspell-incorrect
                         "Flyspell incorrect.")
                        (flyspell-duplicate
                         "Flyspell duplicate.")))
                   (eval
                    `(defface ,(car definition)
                       '((t (:inherit default)))
                       ,(cadr definition))))
                 (enable-theme 'amaranth-dark)
                 (mapcar
                  (lambda (face)
                    (let* ((setting
                            (catch 'found
                              (dolist
                                  (entry
                                   (get
                                    'amaranth-dark
                                    'theme-settings))
                                (when
                                    (and
                                     (eq
                                      (car entry)
                                      'theme-face)
                                     (eq
                                      (cadr entry)
                                      face))
                                  (throw 'found entry)))))
                           (spec (nth 3 setting)))
                      (list
                       face
                       (length spec)
                       (face-attribute
                        face :foreground nil t)
                       (face-attribute
                        face :background nil t)
                       (face-attribute
                        face :underline nil t)
                       (face-attribute
                        face :weight nil t))))
                  '(flymake-errline
                    flymake-warnline
                    flymake-infoline
                    flyspell-incorrect
                    flyspell-duplicate)))
             (disable-theme 'amaranth-dark))"##;
    let expect = expect![[
        r##"OK ((flymake-errline 2 "#a02e2e" unspecified t bold) (flymake-warnline 2 "#ffd966" unspecified t bold) (flymake-infoline 2 "#598b43" unspecified t bold) (flyspell-incorrect 2 "#a02e2e" unspecified t bold) (flyspell-duplicate 2 "#ffd966" unspecified t bold))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}

#[test]
fn terminal_and_late_optional_completion_faces_resolve_to_documented_concrete_colors() {
    let elisp_form = r##"(progn
               (require 'term)
               (unwind-protect
                   (progn
                     (dolist
                     (definition
                      '((term-color-black
                         "Terminal black.")
                        (term-color-red
                         "Terminal red.")
                        (term-color-green
                         "Terminal green.")
                        (term-color-blue
                         "Terminal blue.")
                        (term-color-yellow
                         "Terminal yellow.")
                        (term-color-magenta
                         "Terminal magenta.")
                        (term-color-cyan
                         "Terminal cyan.")
                        (term-color-white
                         "Terminal white.")
                        (company-tooltip
                         "Company tooltip.")
                        (company-tooltip-selection
                         "Company selection.")
                        (company-preview-common
                         "Company preview.")
                        (orderless-match-face-0
                         "Orderless rank zero.")
                        (proof-locked-face
                         "Proof locked.")))
                       (unless (facep (car definition))
                         (eval
                          `(defface ,(car definition)
                             '((t
                                (:foreground "fallback"
                                 :background "fallback")))
                             ,(cadr definition)))))
                 (enable-theme 'amaranth-dark)
                 (mapcar
                  (lambda (face)
                    (list
                     face
                     (face-attribute
                      face :foreground nil t)
                     (face-attribute
                      face :background nil t)))
                  '(term-color-black
                    term-color-red
                    term-color-green
                    term-color-blue
                    term-color-yellow
                    term-color-magenta
                    term-color-cyan
                    term-color-white
                    company-tooltip
                    company-tooltip-selection
                    company-preview-common
                    orderless-match-face-0
                    proof-locked-face)))
                 (disable-theme 'amaranth-dark)))"##;
    let expect = expect![[
        r##"OK ((term-color-black "#4f4949" "#7b7171") (term-color-red "#c73c3f" "#c73c3f") (term-color-green "#598b43" "#598b43") (term-color-blue "#97a1b5" "#97a1b5") (term-color-yellow "#ffd966" "#ffd966") (term-color-magenta "#a64d79" "#a64d79") (term-color-cyan "#959da3" "#959da3") (term-color-white "#e4e4ef" "#ffffff") (company-tooltip "#e4e4ef" "#101010") (company-tooltip-selection "#e4e4ef" "#080808") (company-preview-common "#598b43" "#080808") (orderless-match-face-0 "#ffd966" unspecified) (proof-locked-face unspecified "#303540"))"##
    ]];

    assert_amaranth_dark_theme_parity(elisp_form, expect);
}
