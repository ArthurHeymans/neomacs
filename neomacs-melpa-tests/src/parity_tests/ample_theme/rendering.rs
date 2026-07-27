use expect_test::expect;

use super::assert_ample_theme_parity;

#[test]
fn ample_theme_triplet_fontifies_real_elisp_with_variant_specific_colors() {
    let elisp_form = r##"(let ((directory
                        (file-name-directory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE")))
               results)
         (load
          (expand-file-name
           "ample-flat-theme.el" directory)
          nil t t)
         (load
          (expand-file-name
           "ample-light-theme.el" directory)
          nil t t)
         (unwind-protect
             (dolist (theme
                      '(ample
                        ample-flat
                        ample-light))
               (mapc
                #'disable-theme
                '(ample ample-flat
                  ample-light))
               (enable-theme theme)
               (with-temp-buffer
                 (emacs-lisp-mode)
                 (insert
                  ";; comment\n"
                  "(defconst ample-value \"text\")\n"
                  "(defun ample-call (argument)\n"
                  "  (if argument ample-value nil))\n")
                 (font-lock-ensure)
                 (push
                  (list
                   theme
                   (mapcar
                    (lambda (needle)
                      (goto-char (point-min))
                      (search-forward needle)
                      (let ((face
                             (get-text-property
                              (match-beginning 0)
                              'face)))
                        (list
                         needle face
                         (and
                          face
                          (face-attribute
                           face :foreground
                           nil t)))))
                    '("comment" "defconst"
                      "ample-value" "\"text\""
                      "defun" "ample-call"
                      "if" "nil")))
                  results)))
           (mapc
            #'disable-theme
            '(ample ample-flat
              ample-light)))
         (nreverse results))"##;
    let expect = expect![[
        r##"OK ((ample (("comment" font-lock-comment-face "#757575") ("defconst" font-lock-keyword-face "#5180b3") ("ample-value" font-lock-variable-name-face "#baba36") ("\"text\"" font-lock-string-face "#bdbc61") ("defun" font-lock-keyword-face "#5180b3") ("ample-call" font-lock-function-name-face "#6aaf50") ("if" font-lock-keyword-face "#5180b3") ("nil" nil nil))) (ample-flat (("comment" font-lock-comment-face "#857575") ("defconst" font-lock-keyword-face "#91a0b3") ("ample-value" font-lock-variable-name-face "#aaca86") ("\"text\"" font-lock-string-face "#ddbc91") ("defun" font-lock-keyword-face "#91a0b3") ("ample-call" font-lock-function-name-face "#a9df90") ("if" font-lock-keyword-face "#91a0b3") ("nil" nil nil))) (ample-light (("comment" font-lock-comment-face "#959595") ("defconst" font-lock-keyword-face "#4170B3") ("ample-value" font-lock-variable-name-face "#787800") ("\"text\"" font-lock-string-face "#5D5C01") ("defun" font-lock-keyword-face "#4170B3") ("ample-call" font-lock-function-name-face "#4A8F30") ("if" font-lock-keyword-face "#4170B3") ("nil" nil nil))))"##
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_dark_applies_terminal_and_compilation_status_colors() {
    let elisp_form = r##"(progn
         (require 'term)
         (require 'compile)
         (unwind-protect
          (progn
           (enable-theme 'ample)
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
                face :inherit nil t)))
            '(term-color-black
              term-color-red
              term-color-green
              term-color-yellow
              term-color-blue
              term-color-magenta
              term-color-cyan
              term-color-white
              compilation-error
              compilation-warning
              compilation-info)))
          (disable-theme 'ample)))"##;
    let expect = expect![[
        r##"OK ((term-color-black "#252525" "#252525" unspecified unspecified) (term-color-red "#cd5542" "#cd5542" unspecified unspecified) (term-color-green "#6aaf50" "#6aaf50" unspecified unspecified) (term-color-yellow "#baba36" "#baba36" unspecified unspecified) (term-color-blue "#5180b3" "#5180b3" unspecified unspecified) (term-color-magenta "#ab75c3" "#ab75c3" unspecified unspecified) (term-color-cyan "#68a5e9" "#68a5e9" unspecified unspecified) (term-color-white "#bdbdb3" "#bdbdb3" unspecified unspecified) (compilation-error "#cd5542" unspecified bold unspecified) (compilation-warning "#dF9522" unspecified bold unspecified) (compilation-info "#6aaf50" unspecified bold unspecified))"##
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_dark_applies_real_org_magit_company_ivy_and_lsp_faces() {
    let elisp_form = r##"(let ((faces
                        '(org-level-1
                          org-todo
                          magit-diff-added
                          magit-diff-removed
                          company-tooltip
                          company-tooltip-selection
                          ivy-current-match
                          ivy-minibuffer-match-face-1
                          lsp-headerline-breadcrumb-path-error-face
                          neo-vc-unlocked-changes-face
                          realgud-bp-enabled-face)))
         (mapc
          (lambda (face)
            (unless (facep face)
              (make-face face)))
          faces)
         (unwind-protect
          (progn
           (enable-theme 'ample)
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
                face :underline nil t)
               (face-attribute
                face :inherit nil t)))
            faces))
          (disable-theme 'ample)))"##;
    let expect = expect![[
        r##"OK ((org-level-1 unspecified unspecified unspecified unspecified unspecified) (org-todo "#cd5542" unspecified unspecified unspecified unspecified) (magit-diff-added "#6aaf50" unspecified unspecified unspecified unspecified) (magit-diff-removed "#cd5542" unspecified unspecified unspecified unspecified) (company-tooltip "gray13" "#bdbdb3" unspecified unspecified unspecified) (company-tooltip-selection "#bdbdb3" "#5180b3" unspecified unspecified unspecified) (ivy-current-match unspecified unspecified unspecified unspecified unspecified) (ivy-minibuffer-match-face-1 unspecified unspecified unspecified unspecified unspecified) (lsp-headerline-breadcrumb-path-error-face "gray13" unspecified unspecified "#cd5542" unspecified) (neo-vc-unlocked-changes-face "#cd5542" "Blue" unspecified unspecified unspecified) (realgud-bp-enabled-face "red" unspecified unspecified unspecified error))"##
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}

#[test]
fn ample_theme_ansi_vector_is_applied_and_restored_by_theme_lifecycle() {
    let elisp_form = r##"(progn
         (require 'ansi-color)
         (let ((before
                        (copy-sequence
                         ansi-color-names-vector)))
         (unwind-protect
             (progn
               (enable-theme 'ample)
               (let ((during
                      (copy-sequence
                       ansi-color-names-vector)))
                 (disable-theme 'ample)
                 (list before during
                       ansi-color-names-vector
                       (equal
                        before
                        ansi-color-names-vector))))
           (disable-theme 'ample))))"##;
    let expect = expect![[
        r##"OK (["black" "red3" "green3" "yellow3" "blue2" "magenta3" "cyan3" "gray90"] ["#454545" "#cd5542" "#6aaf50" "#baba36" "#5180b3" "#ab75c3" "#68a5e9" "#bdbdb3"] ["black" "red3" "green3" "yellow3" "blue2" "magenta3" "cyan3" "gray90"] t)"##
    ]];
    assert_ample_theme_parity(elisp_form, expect);
}
