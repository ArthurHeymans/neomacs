use expect_test::expect;

use super::assert_almost_mono_themes_parity;

#[test]
fn theme_shims_load_all_variants_without_enabling_them() {
    let elisp_form = r##"(let ((themes
       '(almost-mono-white almost-mono-black
         almost-mono-gray almost-mono-cream)))
  (mapc (lambda (theme) (load-theme theme t t)) themes)
  (list
   custom-enabled-themes
   (mapcar
    (lambda (theme)
      (list
       theme
       (custom-theme-p theme)
       (get theme 'theme-documentation)
       (get theme 'theme-immediate)
       (length (get theme 'theme-settings))
       (featurep
        (intern (format "%s-theme" theme)))))
    themes)))"##;
    let expect = expect![[
        r#"OK (nil ((almost-mono-white #1=(almost-mono-white user changed) "almost mono theme (white version)" t 73 t) (almost-mono-black #2=(almost-mono-black . #1#) "almost mono theme (black version)" t 73 t) (almost-mono-gray #3=(almost-mono-gray . #2#) "almost mono theme (gray version)" t 73 t) (almost-mono-cream (almost-mono-cream . #3#) "almost mono theme (cream version)" t 73 t)))"#
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn every_variant_defines_the_same_complete_ordered_face_surface() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((themes
         '(almost-mono-white almost-mono-black
           almost-mono-gray almost-mono-cream)))
    (mapc (lambda (theme) (load-theme theme t t)) themes)
    (mapcar
     (lambda (theme)
       (let* ((settings (get theme 'theme-settings))
              (names
               (mapcar
                (lambda (setting) (cadr setting))
                settings)))
         (list theme
               (length settings)
               (length (delete-dups (copy-sequence names)))
               (car names)
               (car (last names))
               (equal names
                      (mapcar
                       (lambda (setting) (cadr setting))
                       (get 'almost-mono-white
                            'theme-settings))))))
     themes)))"##;
    let expect = expect![
        "OK ((almost-mono-white 73 72 default fringe t) (almost-mono-black 73 72 default fringe t) (almost-mono-gray 73 72 default fringe t) (almost-mono-cream 73 72 default fringe t))"
    ];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn variants_store_exact_core_background_selection_and_mode_line_specs() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((themes
         '(almost-mono-white almost-mono-black
           almost-mono-gray almost-mono-cream))
        (faces
         '(default fringe region isearch lazy-highlight
           mode-line mode-line-inactive)))
    (mapc (lambda (theme) (load-theme theme t t)) themes)
    (mapcar
     (lambda (theme)
       (list
        theme
        (mapcar
         (lambda (face)
           (copy-tree
            (cl-find-if
             (lambda (setting)
               (and (eq (car setting) 'theme-face)
                    (eq (cadr setting) face)))
             (get theme 'theme-settings))))
         faces)))
     themes)))"##;
    let expect = expect![[
        r##"OK ((almost-mono-white ((theme-face default almost-mono-white ((t (:background "#ffffff" :foreground "#000000")))) (theme-face fringe almost-mono-white ((t (:background "#ffffff")))) (theme-face region almost-mono-white ((t (:background "#fda50f" :foreground "#000000")))) (theme-face isearch almost-mono-white ((t (:background "#888888" :foreground "#000000" :bold t)))) (theme-face lazy-highlight almost-mono-white ((t (:background "#dddddd" :foreground "#000000")))) (theme-face mode-line almost-mono-white ((t (:box (:line-width -1 :color "#dddddd") :background "#efefef" :foreground "#000000")))) (theme-face mode-line-inactive almost-mono-white ((t (:box (:line-width -1 :color "#dddddd") :background "#ffffff" :foreground "#dddddd")))))) (almost-mono-black ((theme-face default almost-mono-black ((t (:background "#000000" :foreground "#ffffff")))) (theme-face fringe almost-mono-black ((t (:background "#000000")))) (theme-face region almost-mono-black ((t (:background "#fda50f" :foreground "#ffffff")))) (theme-face isearch almost-mono-black ((t (:background "#aaaaaa" :foreground "#ffffff" :bold t)))) (theme-face lazy-highlight almost-mono-black ((t (:background "#666666" :foreground "#ffffff")))) (theme-face mode-line almost-mono-black ((t (:box (:line-width -1 :color "#666666") :background "#222222" :foreground "#ffffff")))) (theme-face mode-line-inactive almost-mono-black ((t (:box (:line-width -1 :color "#666666") :background "#000000" :foreground "#666666")))))) (almost-mono-gray ((theme-face default almost-mono-gray ((t (:background "#2b2b2b" :foreground "#ffffff")))) (theme-face fringe almost-mono-gray ((t (:background "#2b2b2b")))) (theme-face region almost-mono-gray ((t (:background "#fda50f" :foreground "#ffffff")))) (theme-face isearch almost-mono-gray ((t (:background "#aaaaaa" :foreground "#ffffff" :bold t)))) (theme-face lazy-highlight almost-mono-gray ((t (:background "#666666" :foreground "#ffffff")))) (theme-face mode-line almost-mono-gray ((t (:box (:line-width -1 :color "#666666") :background "#222222" :foreground "#ffffff")))) (theme-face mode-line-inactive almost-mono-gray ((t (:box (:line-width -1 :color "#666666") :background "#2b2b2b" :foreground "#666666")))))) (almost-mono-cream ((theme-face default almost-mono-cream ((t (:background "#f0e5da" :foreground "#000000")))) (theme-face fringe almost-mono-cream ((t (:background "#f0e5da")))) (theme-face region almost-mono-cream ((t (:background "#fda50f" :foreground "#000000")))) (theme-face isearch almost-mono-cream ((t (:background "#7d7165" :foreground "#000000" :bold t)))) (theme-face lazy-highlight almost-mono-cream ((t (:background "#c4baaf" :foreground "#000000")))) (theme-face mode-line almost-mono-cream ((t (:box (:line-width -1 :color "#c4baaf") :background "#dbd0c5" :foreground "#000000")))) (theme-face mode-line-inactive almost-mono-cream ((t (:box (:line-width -1 :color "#c4baaf") :background "#f0e5da" :foreground "#c4baaf")))))))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn variants_store_exact_font_lock_and_diagnostic_specs_used_while_editing() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((themes
         '(almost-mono-white almost-mono-black
           almost-mono-gray almost-mono-cream))
        (faces
         '(font-lock-keyword-face
           font-lock-function-name-face
           font-lock-variable-name-face
           font-lock-warning-face
           font-lock-comment-face
           font-lock-string-face
           font-lock-doc-face
           show-paren-match
           show-paren-mismatch)))
    (mapc (lambda (theme) (load-theme theme t t)) themes)
    (mapcar
     (lambda (theme)
       (list
        theme
        (mapcar
         (lambda (face)
           (mapcar
            #'copy-tree
            (cl-remove-if-not
             (lambda (setting)
               (and (eq (car setting) 'theme-face)
                    (eq (cadr setting) face)))
             (get theme 'theme-settings))))
         faces)))
     themes)))"##;
    let expect = expect![[
        r##"OK ((almost-mono-white (((theme-face font-lock-keyword-face almost-mono-white ((t (:bold t))))) ((theme-face font-lock-function-name-face almost-mono-white ((t (:bold t))))) ((theme-face font-lock-variable-name-face almost-mono-white ((t (:foreground "#000000" :italic t)))) (theme-face font-lock-variable-name-face almost-mono-white ((t (:foreground "#000000"))))) ((theme-face font-lock-warning-face almost-mono-white ((t (:foreground "#000000" :underline (:color "#ff0000" :style wave)))))) ((theme-face font-lock-comment-face almost-mono-white ((t (:foreground "#888888" :italic t))))) ((theme-face font-lock-string-face almost-mono-white ((t (:foreground "#3c5e2b"))))) ((theme-face font-lock-doc-face almost-mono-white ((t (:inherit font-lock-comment-face))))) ((theme-face show-paren-match almost-mono-white ((t (:background "#ffffff" :foreground "#00ff00" :bold t))))) ((theme-face show-paren-mismatch almost-mono-white ((t (:background "#ffffff" :foreground "#ff0000" :bold t))))))) (almost-mono-black (((theme-face font-lock-keyword-face almost-mono-black ((t (:bold t))))) ((theme-face font-lock-function-name-face almost-mono-black ((t (:bold t))))) ((theme-face font-lock-variable-name-face almost-mono-black ((t (:foreground "#ffffff" :italic t)))) (theme-face font-lock-variable-name-face almost-mono-black ((t (:foreground "#ffffff"))))) ((theme-face font-lock-warning-face almost-mono-black ((t (:foreground "#ffffff" :underline (:color "#ff0000" :style wave)))))) ((theme-face font-lock-comment-face almost-mono-black ((t (:foreground "#aaaaaa" :italic t))))) ((theme-face font-lock-string-face almost-mono-black ((t (:foreground "#a7bca4"))))) ((theme-face font-lock-doc-face almost-mono-black ((t (:inherit font-lock-comment-face))))) ((theme-face show-paren-match almost-mono-black ((t (:background "#000000" :foreground "#00ff00" :bold t))))) ((theme-face show-paren-mismatch almost-mono-black ((t (:background "#000000" :foreground "#ff0000" :bold t))))))) (almost-mono-gray (((theme-face font-lock-keyword-face almost-mono-gray ((t (:bold t))))) ((theme-face font-lock-function-name-face almost-mono-gray ((t (:bold t))))) ((theme-face font-lock-variable-name-face almost-mono-gray ((t (:foreground "#ffffff" :italic t)))) (theme-face font-lock-variable-name-face almost-mono-gray ((t (:foreground "#ffffff"))))) ((theme-face font-lock-warning-face almost-mono-gray ((t (:foreground "#ffffff" :underline (:color "#ff0000" :style wave)))))) ((theme-face font-lock-comment-face almost-mono-gray ((t (:foreground "#aaaaaa" :italic t))))) ((theme-face font-lock-string-face almost-mono-gray ((t (:foreground "#a7bca4"))))) ((theme-face font-lock-doc-face almost-mono-gray ((t (:inherit font-lock-comment-face))))) ((theme-face show-paren-match almost-mono-gray ((t (:background "#2b2b2b" :foreground "#00ff00" :bold t))))) ((theme-face show-paren-mismatch almost-mono-gray ((t (:background "#2b2b2b" :foreground "#ff0000" :bold t))))))) (almost-mono-cream (((theme-face font-lock-keyword-face almost-mono-cream ((t (:bold t))))) ((theme-face font-lock-function-name-face almost-mono-cream ((t (:bold t))))) ((theme-face font-lock-variable-name-face almost-mono-cream ((t (:foreground "#000000" :italic t)))) (theme-face font-lock-variable-name-face almost-mono-cream ((t (:foreground "#000000"))))) ((theme-face font-lock-warning-face almost-mono-cream ((t (:foreground "#000000" :underline (:color "#ff0000" :style wave)))))) ((theme-face font-lock-comment-face almost-mono-cream ((t (:foreground "#7d7165" :italic t))))) ((theme-face font-lock-string-face almost-mono-cream ((t (:foreground "#3c5e2b"))))) ((theme-face font-lock-doc-face almost-mono-cream ((t (:inherit font-lock-comment-face))))) ((theme-face show-paren-match almost-mono-cream ((t (:background "#f0e5da" :foreground "#00ff00" :bold t))))) ((theme-face show-paren-mismatch almost-mono-cream ((t (:background "#f0e5da" :foreground "#ff0000" :bold t))))))))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn generated_themes_keep_bold_attribute_tail_cells_independent_between_variants() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (load-theme 'almost-mono-white t t)
  (load-theme 'almost-mono-black t t)
  (let* ((setting
          (lambda (theme face)
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             (get theme 'theme-settings))))
         (attributes
          (lambda (theme face)
            (cadr (car (cadddr (funcall setting theme face))))))
         (white
          (funcall attributes 'almost-mono-white 'isearch))
         (black
          (funcall attributes 'almost-mono-black 'isearch)))
    (list
     (eq (nthcdr 4 white) (nthcdr 4 black))
     (copy-tree white)
     (copy-tree black))))"##;
    let expect = expect![[
        r##"OK (nil (:background "#888888" :foreground "#000000" :bold t) (:background "#aaaaaa" :foreground "#ffffff" :bold t))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn generated_themes_keep_italic_and_underline_tail_cells_independent_between_variants() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (load-theme 'almost-mono-white t t)
  (load-theme 'almost-mono-black t t)
  (let* ((setting
          (lambda (theme face)
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             (get theme 'theme-settings))))
         (attributes
          (lambda (theme face)
            (cadr (car (cadddr (funcall setting theme face))))))
         (white-comment
          (funcall attributes
                   'almost-mono-white
                   'font-lock-comment-face))
         (black-comment
          (funcall attributes
                   'almost-mono-black
                   'font-lock-comment-face))
         (white-warning
          (funcall attributes
                   'almost-mono-white
                   'font-lock-warning-face))
         (black-warning
          (funcall attributes
                   'almost-mono-black
                   'font-lock-warning-face))
         (white-underline
          (plist-get white-warning :underline))
         (black-underline
          (plist-get black-warning :underline)))
    (list
     (eq (nthcdr 2 white-comment)
         (nthcdr 2 black-comment))
     (eq (nthcdr 2 white-underline)
         (nthcdr 2 black-underline))
     (copy-tree white-comment)
     (copy-tree black-comment)
     (copy-tree white-underline)
     (copy-tree black-underline))))"##;
    let expect = expect![[
        r##"OK (nil nil (:foreground "#888888" :italic t) (:foreground "#aaaaaa" :italic t) (:color "#ff0000" :style wave) (:color "#ff0000" :style wave))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn theme_specs_cover_inheritance_for_documentation_shell_and_current_line_faces() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (load-theme 'almost-mono-black t t)
  (mapcar
   (lambda (face)
     (cl-find-if
      (lambda (setting)
        (and (eq (car setting) 'theme-face)
             (eq (cadr setting) face)))
      (get 'almost-mono-black 'theme-settings)))
   '(font-lock-doc-face linum
     eshell-ls-archive eshell-ls-backup
     eshell-ls-clutter eshell-ls-executable
     eshell-ls-missing eshell-ls-product
     eshell-ls-readonly eshell-ls-special
     eshell-ls-symlink
     highlight-current-line-face)))"##;
    let expect = expect![
        "OK ((theme-face font-lock-doc-face almost-mono-black ((t (:inherit font-lock-comment-face)))) (theme-face linum almost-mono-black ((t (:inherit line-number)))) (theme-face eshell-ls-archive almost-mono-black ((t (:inherit eshell-ls-unreadable)))) (theme-face eshell-ls-backup almost-mono-black ((t (:inherit eshell-ls-unreadable)))) (theme-face eshell-ls-clutter almost-mono-black ((t (:inherit eshell-ls-unreadable)))) (theme-face eshell-ls-executable almost-mono-black ((t (:inherit eshell-ls-unreadable)))) (theme-face eshell-ls-missing almost-mono-black ((t (:inherit eshell-ls-unreadable)))) (theme-face eshell-ls-product almost-mono-black ((t (:inherit eshell-ls-unreadable)))) (theme-face eshell-ls-readonly almost-mono-black ((t (:inherit eshell-ls-unreadable)))) (theme-face eshell-ls-special almost-mono-black ((t (:inherit eshell-ls-unreadable)))) (theme-face eshell-ls-symlink almost-mono-black ((t (:inherit eshell-ls-unreadable)))) (theme-face highlight-current-line-face almost-mono-black ((t (:inherit hl-line)))))"
    ];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn theme_specs_cover_practical_company_org_and_completion_rendering() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (load-theme 'almost-mono-cream t t)
  (mapcar
   (lambda (face)
     (cl-find-if
      (lambda (setting)
        (and (eq (car setting) 'theme-face)
             (eq (cadr setting) face)))
      (get 'almost-mono-cream 'theme-settings)))
   '(company-tooltip company-tooltip-selection
     company-tooltip-common company-scrollbar-bg
     company-scrollbar-fg
     company-tooltip-annotation-selection
     org-document-title org-drawer
     org-special-keyword org-property-value
     org-table org-todo org-done
     org-headline-done org-hide
     vertico-current completions-common-part
     orderless-match-face-0
     orderless-match-face-1
     orderless-match-face-2
     orderless-match-face-3)))"##;
    let expect = expect![[
        r##"OK ((theme-face company-tooltip almost-mono-cream ((t (:background "#dbd0c5" :foreground "#000000")))) (theme-face company-tooltip-selection almost-mono-cream ((t (:background "#c4baaf" :foreground "#000000")))) (theme-face company-tooltip-common almost-mono-cream ((t (:bold t)))) (theme-face company-scrollbar-bg almost-mono-cream ((t (:background "#c4baaf")))) (theme-face company-scrollbar-fg almost-mono-cream ((t (:background "#7d7165")))) (theme-face company-tooltip-annotation-selection almost-mono-cream ((t (:background "#c4baaf" :foreground "#000000" :italic t)))) (theme-face org-document-title almost-mono-cream ((t (:foreground "#000000")))) (theme-face org-drawer almost-mono-cream ((t (:foreground "#7d7165")))) (theme-face org-special-keyword almost-mono-cream ((t (:bold t :foreground "#7d7165")))) (theme-face org-property-value almost-mono-cream ((t (:italic t :foreground "#7d7165")))) (theme-face org-table almost-mono-cream ((t (:foreground "#7d7165")))) (theme-face org-todo almost-mono-cream ((t (:bold t :foreground "#fda50f")))) (theme-face org-done almost-mono-cream ((t (:bold t :foreground "#00ff00")))) (theme-face org-headline-done almost-mono-cream ((t (:bold t :foreground "#000000")))) (theme-face org-hide almost-mono-cream ((t (:foreground "#f0e5da")))) (theme-face vertico-current almost-mono-cream ((t (:bold t :foreground "#000000" :background "#fda50f")))) (theme-face completions-common-part almost-mono-cream ((t (:bold t :underline t)))) (theme-face orderless-match-face-0 almost-mono-cream ((t (:bold t :underline t)))) (theme-face orderless-match-face-1 almost-mono-cream ((t (:bold t :underline t)))) (theme-face orderless-match-face-2 almost-mono-cream ((t (:bold t :underline t)))) (theme-face orderless-match-face-3 almost-mono-cream ((t (:bold t :underline t)))))"##
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn define_theme_macro_expands_variant_name_documentation_faces_and_provision() {
    let elisp_form = r##"(macroexpand-1
 '(almost-mono-themes--define-theme cream))"##;
    let expect = expect![[
        r#"OK (progn (deftheme almost-mono-cream "almost mono theme (cream version)") (put 'almost-mono-cream 'theme-immediate t) (almost-mono-themes--variant-with-colors 'cream (apply 'custom-theme-set-faces 'almost-mono-cream (almost-mono-themes--faces-spec))) (provide-theme 'almost-mono-cream))"#
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}

#[test]
fn define_theme_macro_creates_a_complete_theme_when_invoked_directly() {
    let elisp_form = r##"(progn
  (almost-mono-themes--define-theme gray)
  (list
   (custom-theme-p 'almost-mono-gray)
   (get 'almost-mono-gray 'theme-documentation)
   (get 'almost-mono-gray 'theme-immediate)
   (length (get 'almost-mono-gray 'theme-settings))
   (featurep 'almost-mono-gray-theme)
   custom-enabled-themes))"##;
    let expect = expect![[
        r#"OK ((almost-mono-gray user changed) "almost mono theme (gray version)" t 73 t nil)"#
    ]];
    assert_almost_mono_themes_parity(elisp_form, expect);
}
