use expect_test::expect;

use super::assert_ancient_one_dark_theme_with_prelude_parity;

#[test]
fn ancient_one_dark_theme_real_emacs_lisp_buffer_renders_language_semantics() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))"##;
    let elisp_form = r##"(unwind-protect
         (progn
           (load-theme
            'ancient-one-dark
            t)
           (with-temp-buffer
             (emacs-lisp-mode)
             (insert
              "(defun settle-invoice (invoice)\n\
  \"Return payment state for INVOICE.\"\n\
  ;; Preserve an auditable decision.\n\
  (let ((state :paid))\n\
    (when invoice\n\
      (message \"settled\"))\n\
    state))\n")
             (font-lock-ensure)
             (mapcar
              (lambda (token)
                (goto-char
                 (point-min))
                (search-forward token)
                (let* ((start
                        (-
                         (point)
                         (length token)))
                       (face
                        (get-text-property
                         start
                         'face))
                       (primary
                        (if
                            (consp face)
                            (car face)
                          face)))
                  (list
                   token
                   face
                   (and
                    (facep primary)
                    (face-attribute
                     primary :foreground nil t))
                   (and
                    (facep primary)
                    (face-attribute
                     primary :background nil t))
                   (and
                    (facep primary)
                    (face-attribute
                     primary :weight nil t))
                   (and
                    (facep primary)
                    (face-attribute
                     primary :slant nil t)))))
              '("defun"
                "settle-invoice"
                "Return payment state"
                "Preserve"
                "let"
                ":paid"
                "when"
                "message"
                "\"settled\""))))
       (disable-theme
        'ancient-one-dark))"##;
    let expect = expect![[
        r##"OK (("defun" font-lock-keyword-face "#8b76bc" unspecified bold unspecified) ("settle-invoice" font-lock-function-name-face "#8e7ed9" unspecified unspecified unspecified) ("Return payment state" font-lock-doc-face "#736a8c" unspecified unspecified unspecified) ("Preserve" font-lock-comment-face "#736a8c" unspecified unspecified unspecified) ("let" font-lock-keyword-face "#8b76bc" unspecified bold unspecified) (":paid" font-lock-builtin-face "#b273b1" unspecified unspecified unspecified) ("when" font-lock-keyword-face "#8b76bc" unspecified bold unspecified) ("message" nil nil nil nil nil) ("\"settled\"" font-lock-string-face "#f3cb89" unspecified unspecified unspecified))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_real_org_runbook_renders_workflow_structure() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))"##;
    let elisp_form = r##"(progn
         (require 'org)
         (unwind-protect
             (progn
               (load-theme
                'ancient-one-dark
                t)
               (with-temp-buffer
                 (org-mode)
                 (insert
                  "#+title: Settlement Runbook\n\
* TODO Validate transaction\n\
DEADLINE: <2026-07-31 Fri>\n\
See [[https://example.invalid][ledger documentation]].\n\
#+begin_quote\n\
Settlement must remain auditable.\n\
#+end_quote\n\
#+begin_src emacs-lisp\n\
(message \"validate\")\n\
#+end_src\n")
                 (font-lock-ensure)
                 (mapcar
                  (lambda (token)
                    (goto-char
                     (point-min))
                    (search-forward token)
                    (let* ((start
                            (-
                             (point)
                             (length token)))
                           (face
                            (get-text-property
                             start
                             'face))
                           (primary
                            (if
                                (consp face)
                                (car face)
                              face)))
                      (list
                       token
                       face
                       (and
                        (facep primary)
                        (face-attribute
                         primary :foreground nil t))
                       (and
                        (facep primary)
                        (face-attribute
                         primary :background nil t))
                       (and
                        (facep primary)
                        (face-attribute
                         primary :inherit nil nil))
                       (and
                        (facep primary)
                        (face-attribute
                         primary :weight nil t)))))
                  '("#+title:"
                    "Settlement Runbook"
                    "TODO"
                    "Validate transaction"
                    "2026-07-31"
                    "https://example.invalid"
                    "ledger documentation"
                    "#+begin_quote"
                    "Settlement must remain"
                    "#+begin_src"
                    "message"
                    "#+end_src"))))
           (disable-theme
            'ancient-one-dark)))"##;
    let expect = expect![[
        r##"OK (("#+title:" org-document-info-keyword "#8e7ed9" unspecified unspecified unspecified) ("Settlement Runbook" org-document-title unspecified unspecified unspecified bold) ("TODO" (org-todo org-level-1) "#8b76bc" unspecified unspecified bold) ("Validate transaction" org-level-1 "#c0bac4" unspecified unspecified bold) ("2026-07-31" (org-date) "#d1cad5" unspecified unspecified unspecified) ("https://example.invalid" org-link "#b273b1" unspecified unspecified unspecified) ("ledger documentation" org-link "#b273b1" unspecified unspecified unspecified) ("#+begin_quote" org-block-begin-line "#736a8c" unspecified org-meta-line unspecified) ("Settlement must remain" nil nil nil nil nil) ("#+begin_src" org-block-begin-line "#736a8c" unspecified org-meta-line unspecified) ("message" (org-block) "#b0aab3" unspecified unspecified unspecified) ("#+end_src" org-block-end-line "#736a8c" unspecified org-block-begin-line unspecified))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_real_info_buffer_renders_manual_cross_references_and_code() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))"##;
    let elisp_form = r##"(progn
         (require 'info)
         (unwind-protect
             (progn
               (load-theme
                'ancient-one-dark
                t)
               (with-temp-buffer
                 (Info-mode)
                 (let ((inhibit-read-only t))
                   (insert
                    "File: parity,  Node: Top,  Up: (dir)\n\n\
* Menu:\n\
* Settlement API:: Validate a payment.\n\n\
Use `settle-invoice' with the string \"paid\".\n"))
                 (font-lock-ensure)
                 (mapcar
                  (lambda (token)
                    (goto-char
                     (point-min))
                    (search-forward token)
                    (let* ((start
                            (-
                             (point)
                             (length token)))
                           (face
                            (get-text-property
                             start
                             'face))
                           (primary
                            (if
                                (consp face)
                                (car face)
                              face)))
                      (list
                       token
                       face
                       (and
                        (facep primary)
                        (face-attribute
                         primary :foreground nil t))
                       (and
                        (facep primary)
                        (face-attribute
                         primary :underline nil nil))
                       (and
                        (facep primary)
                        (face-attribute
                         primary :weight nil t)))))
                  '("File:"
                    "Top"
                    "* Menu:"
                    "Settlement API"
                    "settle-invoice"
                    "\"paid\""))))
           (disable-theme
            'ancient-one-dark)))"##;
    let expect = expect![[
        r#"OK (("File:" nil nil nil nil) ("Top" nil nil nil nil) ("* Menu:" nil nil nil nil) ("Settlement API" nil nil nil nil) ("settle-invoice" nil nil nil nil) ("\"paid\"" nil nil nil nil))"#
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_company_completion_popup_resolves_selection_and_annotation() {
    let prelude = r##"(progn
         (fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))
         (dolist
             (definition
              '((company-tooltip
                 (:foreground "fallback"
                  :background "fallback"))
                (company-tooltip-selection
                 (:foreground "fallback"
                  :background "fallback"))
                (company-tooltip-common
                 (:foreground "fallback"))
                (company-tooltop-annotation
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
           (let ((popup
                  (concat
                   (propertize
                    "settleInvoice"
                    'face
                    'company-tooltip-selection)
                   (propertize
                    " : Invoice -> Receipt"
                    'face
                    'company-tooltop-annotation)
                   "\n"
                   (propertize
                    "settleBatch"
                    'face
                    'company-tooltip-common))))
             (list
              (substring-no-properties
               popup)
              (with-temp-buffer
                (insert popup)
                (mapcar
                 (lambda (token)
                   (goto-char
                    (point-min))
                   (search-forward token)
                   (let* ((start
                           (-
                            (point)
                            (length token)))
                          (face
                           (get-text-property
                            start
                            'face)))
                     (list
                      token
                      face
                      (face-attribute
                       face :foreground nil t)
                      (face-attribute
                       face :background nil t)
                      (face-attribute
                       face :weight nil t))))
                 '("settleInvoice"
                   " : Invoice -> Receipt"
                   "settleBatch"))))))
       (disable-theme
        'ancient-one-dark))"##;
    let expect = expect![[
        r##"OK ("settleInvoice : Invoice -> Receipt\nsettleBatch" (("settleInvoice" company-tooltip-selection "#b0aab3" "#524a61" unspecified) (" : Invoice -> Receipt" company-tooltop-annotation "#b273b1" unspecified unspecified) ("settleBatch" company-tooltip-common "#b0aab3" unspecified unspecified)))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_magit_status_rows_resolve_section_and_process_states() {
    let prelude = r##"(progn
         (fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))
         (dolist
             (definition
              '((magit-section-heading
                 (:foreground "fallback"
                  :weight normal))
                (magit-section-highlight
                 (:background "fallback"))
                (magit-diffstat-added
                 (:foreground "fallback"))
                (magit-diffstat-removed
                 (:foreground "fallback"))
                (magit-process-ok
                 (:foreground "fallback"
                  :weight normal))
                (magit-process-ng
                 (:foreground "fallback"
                  :weight normal))))
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
           (let ((status
                  (concat
                   (propertize
                    "Staged changes"
                    'face
                    'magit-section-heading)
                   "\n"
                   (propertize
                    " payment.el | +12 -3"
                    'face
                    'magit-section-highlight)
                   "\n"
                   (propertize
                    "+12"
                    'face
                    'magit-diffstat-added)
                   " "
                   (propertize
                    "-3"
                    'face
                    'magit-diffstat-removed)
                   "\n"
                   (propertize
                    "Process finished"
                    'face
                    'magit-process-ok))))
             (with-temp-buffer
               (insert status)
               (mapcar
                (lambda (token)
                  (goto-char
                   (point-min))
                  (search-forward token)
                  (let* ((start
                          (-
                           (point)
                           (length token)))
                         (face
                          (get-text-property
                           start
                           'face)))
                    (list
                     token
                     face
                     (face-attribute
                      face :foreground nil t)
                     (face-attribute
                      face :background nil t)
                     (face-attribute
                      face :weight nil t))))
                '("Staged changes"
                  "payment.el"
                  "+12"
                  "-3"
                  "Process finished")))))
       (disable-theme
        'ancient-one-dark))"##;
    let expect = expect![[
        r##"OK (("Staged changes" magit-section-heading "#8b76bc" unspecified bold) ("payment.el" magit-section-highlight unspecified "#413952" unspecified) ("+12" magit-section-highlight unspecified "#413952" unspecified) ("-3" magit-section-highlight unspecified "#413952" unspecified) ("Process finished" magit-process-ok "#8e7ed9" unspecified bold))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_terminal_palette_renders_practical_status_grid() {
    let prelude = r##"(progn
         (fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))
         (dolist
             (face
              '(term-color-black
                term-color-blue
                term-color-red
                term-color-green
                term-color-yellow
                term-color-magenta
                term-color-cyan
                term-color-white))
           (eval
            `(defface ,face
               '((t
                  (:foreground "fallback"
                   :background "fallback")))
               "Parity fixture."))))"##;
    let elisp_form = r##"(unwind-protect
         (progn
           (load-theme
            'ancient-one-dark
            t)
           (let* ((entries
                   '((term-color-black . "idle")
                     (term-color-blue . "info")
                     (term-color-red . "failed")
                     (term-color-green . "passed")
                     (term-color-yellow . "pending")
                     (term-color-magenta . "queued")
                     (term-color-cyan . "running")
                     (term-color-white . "plain")))
                  (grid
                   (apply
                    #'concat
                    (mapcar
                     (lambda (entry)
                       (concat
                        (propertize
                         (cdr entry)
                         'face
                         (car entry))
                        " "))
                     entries))))
             (list
              (substring-no-properties
               grid)
              (with-temp-buffer
                (insert grid)
                (mapcar
                 (lambda (entry)
                   (let ((token
                          (cdr entry)))
                     (goto-char
                      (point-min))
                     (search-forward token)
                     (let* ((start
                             (-
                              (point)
                              (length token)))
                            (face
                             (get-text-property
                              start
                              'face)))
                       (list
                        token
                        face
                        (face-attribute
                         face :foreground nil t)
                        (face-attribute
                         face :background nil t)))))
                 entries)))))
       (disable-theme
        'ancient-one-dark))"##;
    let expect = expect![[
        r##"OK ("idle info failed passed pending queued running plain " (("idle" term-color-black "#c0bac4" unspecified) ("info" term-color-blue "#8e7ed9" "#8e7ed9") ("failed" term-color-red "#8b76bc" "#524a61") ("passed" term-color-green "#b273b1" "#524a61") ("pending" term-color-yellow "#d1cad5" "#d1cad5") ("queued" term-color-magenta "#b273b1" "#b273b1") ("running" term-color-cyan "#f3cb89" "#f3cb89") ("plain" term-color-white "#c0bac4" "#c0bac4")))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn ancient_one_dark_theme_web_template_font_lock_renders_tags_attributes_and_embedded_code() {
    let prelude = r##"(progn
         (fset 'display-color-cells
               (lambda (&optional _display)
                 16777216))
         (dolist
             (definition
              '((web-mode-html-tag-face
                 (:foreground "fallback"))
                (web-mode-html-attr-name-face
                 (:foreground "fallback"))
                (web-mode-html-attr-value-face
                 (:foreground "fallback"))
                (web-mode-keyword-face
                 (:foreground "fallback"))
                (web-mode-string-face
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
           (with-temp-buffer
             (insert
              "<invoice status=\"paid\">{{ when \"settled\" }}</invoice>")
             (font-lock-add-keywords
              nil
              '(("</?\\([[:alpha:]-]+\\)"
                 1
                 'web-mode-html-tag-face)
                ("\\([[:alpha:]-]+\\)=\\(\"[^\"]+\"\\)"
                 (1
                  'web-mode-html-attr-name-face)
                 (2
                  'web-mode-html-attr-value-face))
                ("{{ \\([[:alpha:]-]+\\)"
                 1
                 'web-mode-keyword-face)
                ("{{ [[:alpha:]-]+ \\(\"[^\"]+\"\\)"
                 1
                 'web-mode-string-face)))
             (font-lock-mode 1)
             (font-lock-ensure)
             (mapcar
              (lambda (token)
                (goto-char
                 (point-min))
                (search-forward token)
                (let* ((start
                        (-
                         (point)
                         (length token)))
                       (face
                        (get-text-property
                         start
                         'face)))
                  (list
                   token
                   face
                   (face-attribute
                    face :foreground nil t)
                   (face-attribute
                    face :background nil t)
                   (face-attribute
                    face :inherit nil nil))))
              '("invoice"
                "status"
                "\"paid\""
                "when"
                "\"settled\""))))
       (disable-theme
        'ancient-one-dark))"##;
    let expect = expect![[
        r##"OK (("invoice" web-mode-html-tag-face "#b273b1" unspecified unspecified) ("status" web-mode-html-attr-name-face "#8e7ed9" unspecified unspecified) ("\"paid\"" font-lock-string-face "#f3cb89" unspecified unspecified) ("when" web-mode-keyword-face "#8b76bc" unspecified unspecified) ("\"settled\"" font-lock-string-face "#f3cb89" unspecified unspecified))"##
    ]];

    assert_ancient_one_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}
