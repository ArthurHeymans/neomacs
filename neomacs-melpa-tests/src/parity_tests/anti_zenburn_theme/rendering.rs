use expect_test::expect;

use super::assert_anti_zenburn_theme_parity;

#[test]
fn anti_zenburn_theme_real_emacs_lisp_buffer_renders_language_semantics() {
    let elisp_form = r##"(unwind-protect
         (progn
           (load-theme
            'anti-zenburn
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
                   (copy-tree face)
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
       (when
           (custom-theme-enabled-p
            'anti-zenburn)
         (disable-theme
          'anti-zenburn)))"##;
    let expect = expect![[
        r##"OK (("defun" font-lock-keyword-face "#0f2050" unspecified bold unspecified) ("settle-invoice" font-lock-function-name-face "#6c1f1c" unspecified unspecified unspecified) ("Return payment state" font-lock-doc-face "#603a60" unspecified unspecified unspecified) ("Preserve" font-lock-comment-face "#806080" unspecified unspecified unspecified) ("let" font-lock-keyword-face "#0f2050" unspecified bold unspecified) (":paid" font-lock-builtin-face "#232333" unspecified bold unspecified) ("when" font-lock-keyword-face "#0f2050" unspecified bold unspecified) ("message" nil nil nil nil nil) ("\"settled\"" font-lock-string-face "#336c6c" unspecified unspecified unspecified))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_real_org_runbook_renders_workflow_structure() {
    let elisp_form = r##"(progn
         (require 'org)
         (unwind-protect
             (progn
               (load-theme
                'anti-zenburn
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
                       (copy-tree face)
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
           (when
               (custom-theme-enabled-p
                'anti-zenburn)
             (disable-theme
              'anti-zenburn))))"##;
    let expect = expect![[
        r##"OK (("#+title:" org-document-info-keyword unspecified unspecified shadow unspecified) ("Settlement Runbook" org-document-title "#732f2c" unspecified unspecified unspecified) ("TODO" (org-todo org-level-1) "#336c6c" unspecified unspecified bold) ("Validate transaction" org-level-1 "#205070" unspecified unspecified unspecified) ("2026-07-31" (org-date) "#732f2c" unspecified unspecified unspecified) ("https://example.invalid" org-link "#2f4070" unspecified unspecified unspecified) ("ledger documentation" org-link "#2f4070" unspecified unspecified unspecified) ("#+begin_quote" org-block-begin-line "#806080" unspecified org-meta-line unspecified) ("Settlement must remain" nil nil nil nil nil) ("#+begin_src" org-block-begin-line "#806080" unspecified org-meta-line unspecified) ("message" (org-block) unspecified unspecified shadow unspecified) ("#+end_src" org-block-end-line "#806080" unspecified org-block-begin-line unspecified))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_real_unified_diff_renders_headers_hunks_and_changed_lines() {
    let elisp_form = r##"(progn
         (require 'diff-mode)
         (unwind-protect
             (progn
               (load-theme
                'anti-zenburn
                t)
               (with-temp-buffer
                 (insert
                  "diff --git a/ledger.el b/ledger.el\n\
index 0123456..abcdef0 100644\n\
--- a/ledger.el\n\
+++ b/ledger.el\n\
@@ -1,3 +1,3 @@\n\
-(settle old-invoice)\n\
+(settle audited-invoice)\n\
 context-line\n")
                 (diff-mode)
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
                       (copy-tree face)
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
                         primary :weight nil t)))))
                  '("diff --git"
                    "index 0123456"
                    "--- a/ledger.el"
                    "+++ b/ledger.el"
                    "@@ -1,3"
                    "old-invoice"
                    "audited-invoice"
                    "context-line"))))
           (when
               (custom-theme-enabled-p
                'anti-zenburn)
             (disable-theme
              'anti-zenburn))))"##;
    let expect = expect![[
        r##"OK (("diff --git" diff-header unspecified "#a0a0a0" unspecified) ("index 0123456" diff-header unspecified "#a0a0a0" unspecified) ("--- a/ledger.el" diff-header unspecified "#a0a0a0" unspecified) ("+++ b/ledger.el" diff-header unspecified "#a0a0a0" unspecified) ("@@ -1,3" diff-hunk-header unspecified "#a0a0a0" unspecified) ("old-invoice" diff-removed "#235c5c" "#93cccc" unspecified) ("audited-invoice" diff-added "#603a60" "#d0b0d0" unspecified) ("context-line" diff-context unspecified unspecified unspecified))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_real_compilation_buffer_exposes_modern_and_legacy_diagnostics() {
    let elisp_form = r##"(progn
         (require 'compile)
         (unwind-protect
             (progn
               (load-theme
                'anti-zenburn
                t)
               (eval
                '(defface
                     compilation-error-face
                   '((t
                      (:foreground "error-fallback")))
                   "Parity legacy compilation error face."))
               (eval
                '(defface
                     compilation-warning-face
                   '((t
                      (:foreground "warning-fallback")))
                   "Parity legacy compilation warning face."))
               (with-temp-buffer
                 (insert
                  "worker.rs:12:7: error: settlement failed\n\
worker.rs:18:3: warning: retry is slow\n\
worker.rs:22:1: note: transaction is auditable\n")
                 (compilation-mode)
                 (font-lock-ensure)
                 (list
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
                        (copy-tree face)
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
                          primary :underline nil nil)))))
                   '("worker.rs:12"
                     "error"
                     "worker.rs:18"
                     "warning"
                     "worker.rs:22"
                     "note"))
                  (mapcar
                   (lambda (face)
                     (list
                      face
                      (face-attribute
                       face :foreground nil t)
                      (face-attribute
                       face :weight nil t)
                      (face-attribute
                       face :underline nil nil)))
                   '(compilation-error-face
                     compilation-warning-face)))))
           (when
               (custom-theme-enabled-p
                'anti-zenburn)
             (disable-theme
              'anti-zenburn))))"##;
    let expect = expect![[
        r##"OK ((("worker.rs:12" font-lock-function-name-face "#6c1f1c" unspecified unspecified unspecified) ("error" nil nil nil nil nil) ("worker.rs:18" font-lock-function-name-face "#6c1f1c" unspecified unspecified unspecified) ("warning" nil nil nil nil nil) ("worker.rs:22" font-lock-function-name-face "#6c1f1c" unspecified unspecified unspecified) ("note" nil nil nil nil nil)) ((compilation-error-face "#437c7c" bold t) (compilation-warning-face "#205070" bold t)))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}

#[test]
fn anti_zenburn_theme_real_whitespace_buffer_distinguishes_layout_defects() {
    let elisp_form = r##"(progn
         (require 'whitespace)
         (unwind-protect
             (progn
               (load-theme
                'anti-zenburn
                t)
               (with-temp-buffer
                 (setq-local
                  whitespace-style
                  '(face spaces tabs
                         trailing
                         lines-tail
                         space-before-tab
                         indentation
                         empty
                         space-after-tab))
                 (setq-local
                  whitespace-line-column
                  12)
                 (insert
                  "alpha \t \n\
\tindented\n\
this-line-is-far-too-long\n\
\n")
                 (whitespace-turn-on)
                 (font-lock-ensure)
                 (let ((positions
                        (list
                         (cons 'space 6)
                         (cons 'tab 7)
                         (cons 'trailing 8)
                         (cons 'indentation 10)
                         (cons 'long-line 38)
                         (cons 'empty-line
                               (1-
                                (point-max))))))
                   (mapcar
                    (lambda (entry)
                      (let* ((position
                              (cdr entry))
                             (face
                              (get-text-property
                               position
                               'face))
                             (faces
                              (if
                                  (listp face)
                                  face
                                (list face))))
                        (list
                         (car entry)
                         (copy-tree face)
                         (mapcar
                          (lambda (candidate)
                            (and
                             (facep candidate)
                             (list
                              candidate
                              (face-attribute
                               candidate
                               :foreground
                               nil t)
                              (face-attribute
                               candidate
                               :background
                               nil t))))
                          faces))))
                    positions))))
           (when
               (custom-theme-enabled-p
                'anti-zenburn)
             (disable-theme
              'anti-zenburn))))"##;
    let expect = expect![[
        r##"OK ((space whitespace-space-before-tab ((whitespace-space-before-tab "#205070" "#205070"))) (tab whitespace-trailing ((whitespace-trailing unspecified "#336c6c"))) (trailing whitespace-trailing ((whitespace-trailing unspecified "#336c6c"))) (indentation whitespace-tab ((whitespace-tab unspecified "#437c7c"))) (long-line (whitespace-line) ((whitespace-line "#23733c" "#c0c0c0"))) (empty-line nil nil))"##
    ]];

    assert_anti_zenburn_theme_parity(elisp_form, expect);
}
