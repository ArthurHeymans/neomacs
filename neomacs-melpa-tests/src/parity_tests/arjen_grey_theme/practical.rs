use expect_test::expect;

use super::assert_arjen_grey_theme_parity;

#[test]
fn arjen_grey_theme_real_elisp_font_lock_workflow_uses_semantic_palette() {
    let elisp_form = r##"(unwind-protect
               (progn
                 (enable-theme 'arjen-grey)
                 (with-temp-buffer
                   (emacs-lisp-mode)
                   (insert
                    ";; explain\n(defun parity-demo (value)\n  (let ((answer 42))\n    (if value\n        (message \"value=%s\" answer)\n      (error \"missing\"))))\n")
                   (font-lock-ensure)
                   (mapcar
                    (lambda (needle)
                      (goto-char (point-min))
                      (search-forward needle)
                      (let* ((position
                              (match-beginning 0))
                             (face
                              (get-text-property
                               position 'face))
                             (primary-face
                              (if (symbolp face)
                                  face
                                (car-safe face))))
                        (list
                         needle
                         face
                         (and
                          (facep primary-face)
                          (face-attribute
                           primary-face
                           :foreground nil t))
                         (and
                          (facep primary-face)
                          (face-attribute
                           primary-face
                           :weight nil t)))))
                    '("explain" "defun" "parity-demo"
                      "value" "let" "42" "if"
                      "message" "\"value=%s\""
                      "error"))))
             (disable-theme 'arjen-grey))"##;
    let expect = expect![[
        r##"OK (("explain" font-lock-comment-face "#63747c" unspecified) ("defun" font-lock-keyword-face "#b894b0" unspecified) ("parity-demo" font-lock-function-name-face "#909fab" unspecified) ("value" nil nil nil) ("let" font-lock-keyword-face "#b894b0" unspecified) ("42" nil nil nil) ("if" font-lock-keyword-face "#b894b0" unspecified) ("message" nil nil nil) ("\"value=%s\"" font-lock-string-face "#a8c194" unspecified) ("error" font-lock-warning-face "red" bold))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_real_modeline_selection_and_prompt_attributes_are_usable() {
    let elisp_form = r##"(unwind-protect
               (progn
                 (enable-theme 'arjen-grey)
                 (mapcar
                  (lambda (request)
                    (let ((face (car request)))
                      (cons
                       face
                       (mapcar
                        (lambda (attribute)
                          (list
                           attribute
                           (face-attribute
                            face attribute nil t)))
                        (cdr request)))))
                  '((default :foreground :background)
                    (cursor :background)
                    (fringe :background)
                    (mode-line :foreground :background)
                    (region :background)
                    (secondary-selection :background)
                    (minibuffer-prompt
                     :foreground :weight))))
             (disable-theme 'arjen-grey))"##;
    let expect = expect![[
        r##"OK ((default (:foreground "#bdc3ce") (:background "#2a2f38")) (cursor (:background "#e1cb8c")) (fringe (:background "#2b303a")) (mode-line (:foreground "#bdc3ce") (:background "#242a34")) (region (:background "#3c4449")) (secondary-selection (:background "#464a4d")) (minibuffer-prompt (:foreground "#a8c194") (:weight bold)))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_real_company_completion_ui_has_coherent_state_palette() {
    let elisp_form = r##"(unwind-protect
               (progn
                 (dolist
                     (face
                      '(company-tooltip
                        company-tooltip-annotation
                        company-tooltip-selection
                        company-tooltip-mouse
                        company-tooltip-common
                        company-scrollbar-fg
                        company-scrollbar-bg
                        company-preview
                        company-preview-common))
                   (face-spec-set
                    face
                    '((t
                       (:foreground "fallback"
                        :background "fallback")))
                    'face-defface-spec))
                 (enable-theme 'arjen-grey)
                 (mapcar
                  (lambda (face)
                    (list
                     face
                     (face-attribute
                      face :foreground nil t)
                     (face-attribute
                      face :background nil t)))
                  '(company-tooltip
                    company-tooltip-annotation
                    company-tooltip-selection
                    company-tooltip-mouse
                    company-tooltip-common
                    company-scrollbar-fg
                    company-scrollbar-bg
                    company-preview
                    company-preview-common)))
             (disable-theme 'arjen-grey))"##;
    let expect = expect![[
        r##"OK ((company-tooltip "#bdc3ce" "#242a34") (company-tooltip-annotation "#eacc8c" unspecified) (company-tooltip-selection unspecified "#464a4d") (company-tooltip-mouse unspecified "#464a4d") (company-tooltip-common "#909fab" unspecified) (company-scrollbar-fg unspecified "#464a4d") (company-scrollbar-bg unspecified "#242a34") (company-preview "#bdc3ce" "#242a34") (company-preview-common "#909fab" unspecified))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_real_helm_candidate_ui_distinguishes_headers_and_selection() {
    let elisp_form = r##"(unwind-protect
               (progn
                 (dolist
                     (face
                      '(helm-header
                        helm-source-header
                        helm-ff-directory
                        helm-selection
                        helm-selection-line))
                   (face-spec-set
                    face
                    '((t
                       (:foreground "fallback"
                        :background "fallback"
                        :weight normal
                        :underline t
                        :box t)))
                    'face-defface-spec))
                 (enable-theme 'arjen-grey)
                 (mapcar
                  (lambda (face)
                    (cons
                     face
                     (mapcar
                      (lambda (attribute)
                        (list
                         attribute
                         (face-attribute
                          face attribute nil t)))
                      '(:foreground :background
                        :weight :underline :box))))
                  '(helm-header
                    helm-source-header
                    helm-ff-directory
                    helm-selection
                    helm-selection-line)))
             (disable-theme 'arjen-grey))"##;
    let expect = expect![[
        r##"OK ((helm-header (:foreground "#bdc3ce") (:background "#2a2f38") (:weight unspecified) (:underline nil) (:box nil)) (helm-source-header (:foreground "#bdc3ce") (:background "#2a2f38") (:weight bold) (:underline nil) (:box (:line-width -1 :style released-button))) (helm-ff-directory (:foreground "#bdc3ce") (:background "#2a2f38") (:weight bold) (:underline nil) (:box unspecified)) (helm-selection (:foreground unspecified) (:background "#3c4449") (:weight unspecified) (:underline nil) (:box unspecified)) (helm-selection-line (:foreground unspecified) (:background "#2a2f38") (:weight unspecified) (:underline unspecified) (:box unspecified)))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_real_gnus_header_and_summary_workflow_is_distinct() {
    let elisp_form = r##"(unwind-protect
               (progn
                 (dolist
                     (face
                      '(gnus-header-name
                        gnus-header-content
                        gnus-header-subject
                        gnus-summary-normal-read
                        widget-button))
                   (face-spec-set
                    face
                    '((t
                       (:foreground "fallback")))
                    'face-defface-spec))
                 (enable-theme 'arjen-grey)
                 (let ((sample
                        (concat
                         (propertize
                          "Subject:"
                          'face 'gnus-header-name)
                         " "
                         (propertize
                          "Parity report"
                          'face 'gnus-header-subject)
                         "\n"
                         (propertize
                          "read article"
                          'face
                          'gnus-summary-normal-read))))
                   (list
                    (substring-no-properties sample)
                    (mapcar
                     (lambda (position)
                       (let ((face
                              (get-text-property
                               position 'face sample)))
                         (list
                          position
                          face
                          (face-attribute
                           face :foreground nil t))))
                     '(0 9 23))
                    (face-attribute
                     'gnus-header-content
                     :foreground nil t)
                    (face-attribute
                     'widget-button
                     :foreground nil t))))
             (disable-theme 'arjen-grey))"##;
    let expect = expect![[
        r##"OK ("Subject: Parity report\nread article" ((0 gnus-header-name "#909fab") (9 gnus-header-subject "#eacc8c") (23 gnus-summary-normal-read "#909fab")) "#bdc3ce" "#909fab")"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}

#[test]
fn arjen_grey_theme_batch_terminal_quantizes_hex_palette_consistently() {
    let elisp_form = r##"(let ((colors
                    '("#bdc3ce" "#2a2f38" "#e1cb8c"
                      "#2b303a" "#242a34" "#3c4449"
                      "#595e66" "#464a4d" "#eacc8c"
                      "#63747c" "#909fab" "#b894b0"
                      "#a8c194" "#a0a5a0" "#8b9db0"
                      "#8294ac" "red")))
               (mapcar
                (lambda (color)
                  (list
                   color
                   (color-values color)
                   (color-name-to-rgb color)))
                colors))"##;
    let expect = expect![[
        r##"OK (("#bdc3ce" #4=(65535 65535 65535) (1.0 1.0 1.0)) ("#2a2f38" #1=(0 0 65535) (0.0 0.0 1.0)) ("#e1cb8c" #3=(65535 65535 0) (1.0 1.0 0.0)) ("#2b303a" #1# (0.0 0.0 1.0)) ("#242a34" #1# (0.0 0.0 1.0)) ("#3c4449" #1# (0.0 0.0 1.0)) ("#595e66" #2=(0 0 0) (0.0 0.0 0.0)) ("#464a4d" #2# (0.0 0.0 0.0)) ("#eacc8c" #3# (1.0 1.0 0.0)) ("#63747c" #1# (0.0 0.0 1.0)) ("#909fab" #5=(0 65535 65535) (0.0 1.0 1.0)) ("#b894b0" (65535 0 65535) (1.0 0.0 1.0)) ("#a8c194" #3# (1.0 1.0 0.0)) ("#a0a5a0" #4# (1.0 1.0 1.0)) ("#8b9db0" #5# (0.0 1.0 1.0)) ("#8294ac" #5# (0.0 1.0 1.0)) ("red" (65535 0 0) (1.0 0.0 0.0)))"##
    ]];
    assert_arjen_grey_theme_parity(elisp_form, expect);
}
