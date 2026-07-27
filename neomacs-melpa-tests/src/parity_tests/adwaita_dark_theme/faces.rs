use expect_test::expect;

use super::{assert_adwaita_dark_theme_parity, assert_adwaita_dark_theme_with_prelude_parity};

#[test]
fn adwaita_dark_theme_fingerprints_every_face_in_diagnostic_chunks() {
    let elisp_form = r##"(let ((remaining
                (reverse
                 (copy-sequence
                  (get 'adwaita-dark 'theme-settings))))
               chunks)
         (while remaining
           (let* ((chunk (seq-take remaining 50))
                  (faces (mapcar #'cadr chunk))
                  (entry-hashes
                   (mapcar
                    (lambda (entry)
                      (secure-hash
                       'sha256
                       (prin1-to-string entry)))
                    chunk)))
             (push
              (list
               (car faces)
               (car (last faces))
               (length chunk)
               (secure-hash
                'sha256
                (prin1-to-string entry-hashes)))
              chunks)
             (setq remaining (nthcdr (length chunk) remaining))))
         (let* ((settings
                 (get 'adwaita-dark 'theme-settings))
                (faces (mapcar #'cadr settings))
                duplicates)
           (dolist (face faces)
             (when (> (seq-count
                       (lambda (candidate)
                         (eq candidate face))
                       faces)
                      1)
               (cl-pushnew face duplicates)))
           (list
            (length settings)
            (length (delete-dups (copy-sequence faces)))
            (nreverse duplicates)
            (nreverse chunks))))"##;
    let expect = expect![[
        r#"OK (520 519 (helm-swoop-target-line-face) ((default ansi-color-green 50 "f74ccb0455a1928125a36cb226af759efaa0901344d320b635fe36a709560d47") (ansi-color-yellow diff-changed 50 "a8b418c2bc62b8ad56b659901d9d820a6495407f78b737477bd964251121ff1e") (diff-context eshell-ls-missing 50 "6e7862fca2fc6304c4baac69298aa553f9fac40e4b7810076f1a2cad552cce0e") (eshell-ls-product reb-match-0 50 "348e1bca51530c86d9bbccd4985ff8b1e619ab21b82c9dc1928ddf9d5203a96a") (reb-match-1 cider-stacktrace-fn-face 50 "126335f91cfd3a07b01a5b7295010924d164378d1a4e62c1e4f89df8d70e9eab") (cider-stacktrace-error-class-face git-gutter+-deleted 50 "09eb9cff3bdeee46532462cf8549af410c547823b3f465315d90e51a4c861f85") (git-gutter-fr:modified lsp-ui-doc-header 50 "2541483a6a44e2c18695a5e2ac713da659cc7b381c73b54e411cb72e8f28ad9d") (lsp-ui-peek-filename magit-reflog-merge 50 "2edba4876632669a857ce98ee1080dcacc6638a710a9431d84ca555b4d91afda") (magit-reflog-other nlinum-hl-face 50 "e6b6e471017f3906d5fc72cef25a6610f4adf401cd6d19f98afe58073d570c33") (nlinum-relative-current-face treemacs-tags-face 50 "9af19dc66ba46daa8af97209721fb0ea6993bda16fa78cb7d33d7d23631194b7") (treemacs-git-modified-face yas-field-highlight-face 20 "ff1f412a8fe733a3fba39ee4b43962630f1a465ed9e6987e63f904bcfc3b3714")))"#
    ]];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_default_256_color_specs_cover_core_builtin_and_external_faces() {
    let elisp_form = r##"(let ((settings
                (get 'adwaita-dark 'theme-settings)))
         (mapcar
          (lambda (face)
            (let ((entry
                   (seq-find
                    (lambda (setting)
                      (eq (nth 1 setting) face))
                    settings)))
              (list face
                    (nth 0 entry)
                    (nth 2 entry)
                    (nth 3 entry))))
          '(default
            error
            warning
            success
            region
            highlight
            font-lock-keyword-face
            font-lock-negation-char-face
            mode-line
            mode-line-inactive
            ansi-color-bright-blue
            completions-first-difference
            dired-directory
            flymake-error
            outline-1
            tab-line-tab
            tab-bar-tab
            company-tooltip
            magit-diff-added
            rainbow-delimiters-depth-1-face
            treemacs-directory-face
            vertico-current
            which-key-key-face)))"##;
    let expect = expect![[
        r##"OK ((default theme-face adwaita-dark ((#1=((class color) (min-colors 256)) (:background "gray11" :foreground "gray86")))) (error theme-face adwaita-dark ((#1# (:foreground "indianred2")))) (warning theme-face adwaita-dark ((#1# (:foreground "gold2")))) (success theme-face adwaita-dark ((#1# (:foreground "seagreen3")))) (region theme-face adwaita-dark ((#1# (:background "gray27" :distant-foreground "gray86")))) (highlight theme-face adwaita-dark ((#1# (:background "steelblue2" :foreground "gray13" :distant-foreground "gray87")))) (font-lock-keyword-face theme-face adwaita-dark ((#1# (:foreground "orange2" :weight bold)))) (font-lock-negation-char-face theme-face adwaita-dark ((#1# (::weight bold)))) (mode-line theme-face adwaita-dark ((#1# (:background "gray19" :foreground "gray86" :box nil)))) (mode-line-inactive theme-face adwaita-dark ((#1# (:background "gray14" :foreground "gray40" :box nil)))) (ansi-color-bright-blue theme-face adwaita-dark ((#1# (:foreground "#99c1f1" :background "#99c1f1")))) (completions-first-difference theme-face adwaita-dark ((#1# (:weight bold)))) (dired-directory theme-face adwaita-dark ((#1# (:foreground "orchid3")))) (flymake-error theme-face adwaita-dark ((#1# (:underline (:color "indianred2"))))) (outline-1 theme-face adwaita-dark ((#1# (:foreground "steelblue2" :weight bold)))) (tab-line-tab theme-face adwaita-dark ((#1# (:background "gray11" :foreground "gray86" :box nil)))) (tab-bar-tab theme-face adwaita-dark ((#1# (:background "gray11" :foreground "gray86" :box nil)))) (company-tooltip theme-face adwaita-dark ((#1# (:background "gray19" :foreground "gray87")))) (magit-diff-added theme-face adwaita-dark ((#1# (:background "gray13" :foreground "mediumaquamarine")))) (rainbow-delimiters-depth-1-face theme-face adwaita-dark ((#1# (:foreground "steelblue2")))) (treemacs-directory-face theme-face adwaita-dark ((#1# (:foreground "steelblue2")))) (vertico-current theme-face adwaita-dark ((#1# (:background "gray19" :bold nil)))) (which-key-key-face theme-face adwaita-dark ((#1# (:foreground "seagreen3")))))"##
    ]];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_true_color_loading_changes_the_complete_palette_contract() {
    let prelude = r##"(progn
         (fset 'daemonp (lambda () nil))
         (fset 'display-graphic-p (lambda (&optional _display) t))
         (fset 'tty-display-color-cells
               (lambda (&optional _terminal) 16777216)))"##;
    let elisp_form = r##"(let ((settings
                (get 'adwaita-dark 'theme-settings)))
         (mapcar
          (lambda (face)
            (let ((entry
                   (seq-find
                    (lambda (setting)
                      (eq (nth 1 setting) face))
                    settings)))
              (list face (nth 3 entry))))
          '(default
            fringe
            region
            highlight
            tooltip
            link
            font-lock-builtin-face
            font-lock-keyword-face
            font-lock-string-face
            mode-line
            mode-line-inactive
            dired-directory
            flymake-warning
            outline-1
            company-tooltip
            magit-diff-added
            rainbow-delimiters-depth-5-face
            vertico-current)))"##;
    let expect = expect![[
        r##"OK ((default ((#1=((class color) (min-colors 256)) (:background "#1c1c1c" :foreground "#deddda")))) (fringe ((#1# (:inherit default :foreground "#454545")))) (region ((#1# (:background "#454545" :distant-foreground "#deddda")))) (highlight ((#1# (:background "#64a6f4" :foreground "#202020" :distant-foreground "#dfdfdf")))) (tooltip ((#1# (:background "#060606" :foreground "#f0f0f0")))) (link ((#1# (:foreground "#64a6f4" :underline t :weight bold)))) (font-lock-builtin-face ((#1# (:foreground "#7d8ac7")))) (font-lock-keyword-face ((#1# (:foreground "#ffa348" :weight bold)))) (font-lock-string-face ((#1# (:foreground "#5bc8af")))) (mode-line ((#1# (:background "#303030" :foreground "#deddda" :box nil)))) (mode-line-inactive ((#1# (:background "#242424" :foreground "#656565" :box nil)))) (dired-directory ((#1# (:foreground "#dd80de")))) (flymake-warning ((#1# (:underline (:color "#f8e45c"))))) (outline-1 ((#1# (:foreground "#64a6f4" :weight bold)))) (company-tooltip ((#1# (:background "#303030" :foreground "#dfdfdf")))) (magit-diff-added ((#1# (:background "#202020" :foreground "#5bc8af")))) (rainbow-delimiters-depth-5-face ((#1# (:foreground "#5bc8af")))) (vertico-current ((#1# (:background "#303030" :bold nil)))))"##
    ]];
    assert_adwaita_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_all_configuration_toggles_change_their_real_face_specs() {
    let prelude = r##"(progn
         (setq adwaita-dark-theme-pad-mode-line t
               adwaita-dark-theme-pad-tab-line t
               adwaita-dark-theme-pad-tab-bar t
               adwaita-dark-theme-no-completions-first-difference t
               adwaita-dark-theme-bold-vertico-current t
               adwaita-dark-theme-gray-rainbow-delimiters t
               adwaita-dark-theme-gray-outlines t)
         (fset 'daemonp (lambda () nil))
         (fset 'display-graphic-p (lambda (&optional _display) nil))
         (fset 'tty-display-color-cells
               (lambda (&optional _terminal) 256)))"##;
    let elisp_form = r##"(let ((settings
                (get 'adwaita-dark 'theme-settings)))
         (list
          (mapcar
           (lambda (face)
             (list
              face
              (nth
               3
               (seq-find
                (lambda (setting)
                  (eq (nth 1 setting) face))
                settings))))
           '(mode-line
             mode-line-inactive
             tab-line-tab
             tab-line-tab-inactive
             tab-bar-tab
             tab-bar-tab-inactive
             completions-first-difference
             vertico-current))
          (mapcar
           (lambda (face)
             (list
              face
              (nth
               3
               (seq-find
                (lambda (setting)
                  (eq (nth 1 setting) face))
                settings))))
           '(outline-1 outline-2 outline-3 outline-4
             outline-5 outline-6 outline-7 outline-8))
          (mapcar
           (lambda (face)
             (list
              face
              (nth
               3
               (seq-find
                (lambda (setting)
                  (eq (nth 1 setting) face))
                settings))))
           '(rainbow-delimiters-depth-1-face
             rainbow-delimiters-depth-2-face
             rainbow-delimiters-depth-3-face
             rainbow-delimiters-depth-4-face
             rainbow-delimiters-depth-5-face
             rainbow-delimiters-depth-6-face
             rainbow-delimiters-depth-7-face
             rainbow-delimiters-depth-8-face
             rainbow-delimiters-depth-9-face))))"##;
    let expect = expect![[
        r#"OK (((mode-line ((#1=((class color) (min-colors 256)) (:background "gray19" :foreground "gray86" :box (:line-width 10 :color "gray19"))))) (mode-line-inactive ((#1# (:background "gray14" :foreground "gray40" :box (:line-width 10 :color "gray14"))))) (tab-line-tab ((#1# (:background "gray11" :foreground "gray86" :box (:line-width 10 :color "gray11"))))) (tab-line-tab-inactive ((#1# (:background "gray14" :foreground "gray47" :box (:line-width 10 :color "gray14"))))) (tab-bar-tab ((#1# (:background "gray11" :foreground "gray86" :box (:line-width 10 :color "gray11"))))) (tab-bar-tab-inactive ((#1# (:background "gray14" :foreground "gray47" :box (:line-width 10 :color "gray14"))))) (completions-first-difference ((#1# nil))) (vertico-current ((#1# (:background "gray19" :bold bold))))) ((outline-1 ((#1# (:foreground "gray48" :weight bold)))) (outline-2 ((#1# (:foreground "gray65" :weight bold)))) (outline-3 ((#1# (:foreground "gray48" :weight bold)))) (outline-4 ((#1# (:foreground "gray40" :weight bold)))) (outline-5 ((#1# (:foreground "gray48" :weight bold)))) (outline-6 ((#1# (:foreground "gray65" :weight bold)))) (outline-7 ((#1# (:foreground "gray48" :weight bold)))) (outline-8 ((#1# (:foreground "gray40" :weight bold))))) ((rainbow-delimiters-depth-1-face ((#1# (:foreground "gray65")))) (rainbow-delimiters-depth-2-face ((#1# (:foreground "gray65")))) (rainbow-delimiters-depth-3-face ((#1# (:foreground "gray65")))) (rainbow-delimiters-depth-4-face ((#1# (:foreground "gray65")))) (rainbow-delimiters-depth-5-face ((#1# (:foreground "gray65")))) (rainbow-delimiters-depth-6-face ((#1# (:foreground "gray65")))) (rainbow-delimiters-depth-7-face ((#1# (:foreground "gray65")))) (rainbow-delimiters-depth-8-face ((#1# (:foreground "gray65")))) (rainbow-delimiters-depth-9-face ((#1# (:foreground "gray65"))))))"#
    ]];
    assert_adwaita_dark_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn adwaita_dark_theme_enable_disable_lifecycle_applies_to_real_and_external_faces() {
    let elisp_form = r##"(let ((external-faces
                '(company-tooltip
                  magit-diff-added
                  rainbow-delimiters-depth-1-face
                  vertico-current)))
         (dolist (face external-faces)
           (unless (facep face)
             (make-empty-face face)))
         (unwind-protect
             (progn
               (enable-theme 'adwaita-dark)
               (let ((enabled
                      (list
                       (custom-theme-enabled-p 'adwaita-dark)
                       custom-enabled-themes
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
                        (append
                         '(default error warning success
                           font-lock-keyword-face mode-line)
                         external-faces)))))
                 (disable-theme 'adwaita-dark)
                 (list
                  enabled
                  (custom-theme-enabled-p 'adwaita-dark)
                  custom-enabled-themes)))
           (when (custom-theme-enabled-p 'adwaita-dark)
             (disable-theme 'adwaita-dark))))"##;
    let expect = expect![[
        r#"OK ((#1=(adwaita-dark) #1# ((default "unspecified-fg" "unspecified-bg" normal nil) (error unspecified unspecified bold unspecified) (warning unspecified unspecified bold unspecified) (success unspecified unspecified bold unspecified) (font-lock-keyword-face unspecified unspecified bold unspecified) (mode-line unspecified unspecified unspecified unspecified) (company-tooltip unspecified unspecified unspecified unspecified) (magit-diff-added unspecified unspecified unspecified unspecified) (rainbow-delimiters-depth-1-face unspecified unspecified unspecified unspecified) (vertico-current unspecified unspecified unspecified unspecified))) nil nil)"#
    ]];
    assert_adwaita_dark_theme_parity(elisp_form, expect);
}
