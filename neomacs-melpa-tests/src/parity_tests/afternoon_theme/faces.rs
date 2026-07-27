use expect_test::expect;

use super::assert_afternoon_theme_with_prelude_parity;

#[test]
fn afternoon_theme_fingerprints_every_true_color_face_in_diagnostic_chunks() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 16777216))"##;
    let elisp_form = r##"(let* ((remaining
                 (seq-filter
                  (lambda (setting)
                    (eq (car setting) 'theme-face))
                  (reverse
                   (copy-sequence
                    (get 'afternoon 'theme-settings)))))
                (all-faces (mapcar #'cadr remaining))
                chunks)
         (while remaining
           (let* ((chunk (seq-take remaining 40))
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
         (let (duplicates)
           (dolist (face all-faces)
             (when (> (seq-count
                       (lambda (candidate)
                         (eq candidate face))
                       all-faces)
                      1)
               (cl-pushnew face duplicates)))
           (list
            (length all-faces)
            (length (delete-dups (copy-sequence all-faces)))
            (nreverse duplicates)
            (nreverse chunks))))"##;
    let expect = expect![[
        r#"OK (411 410 (erc-keyword-face) ((default clojure-brackets 40 "78f18fd044f822f1a18b0ff86067967f64f7616ca4f5a6b73806bdd2e3f6be41") (clojure-double-quote mode-line-inactive 40 "5ca2a9e165a3c77606c566dda9d604060d49477c82382bcd6fe187c9a892551d") (mode-line-emphasis diff-refine-added 40 "469c735f657d8fe133fc0f84287398513f11608601bf99ab0688fbd730461b3a") (diff-refine-removed magit-log-graph 40 "feaf5db32cc37785338d9852d7826a9a6bbc3d0a7223e765fa3e2265fa410aec") (magit-log-head-label-bisect-bad org-column-title 40 "2cee4437035d3e5f1dbb4a205a527a0e69e0e8a93dfaf41db522b99e0c26a5fa") (org-date js3-instance-member-face 40 "1f72156c54f196e4dcb468f68ca45eb9889859441b99d77a9fefa9979361a1ba") (js3-private-function-call-face powerline-active1 40 "0fd6495ca44136adb138d65a8551aa141081fa7831e9bc2e42726ed87655aace") (powerline-active2 mu4e-title-face 40 "489c11203a599d31bcb0bfc12638969153a0d212903a96180e68251dfb1d7a3a") (gnus-cite-1 gnus-group-mail-6-empty 40 "de07f3524690f446432b14e958ef75b79cfe32163e8429f0c24acdae8cd8e3e3") (gnus-group-news-1 custom-variable-tag 40 "5f9c2d330de855ab5b959f588da1281ca072a906aca1def76bee8de7fd1fcdb2") (custom-group-tag term-color-white 11 "f21fb01736cd3c98f55e38d1954bb3b5c06024626ffbf16e999d2adfc6d6d363")))"#
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn afternoon_theme_fingerprints_every_256_color_face_and_palette_branch() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 256))"##;
    let elisp_form = r##"(let* ((settings
                 (seq-filter
                  (lambda (setting)
                    (eq (car setting) 'theme-face))
                  (reverse
                   (copy-sequence
                    (get 'afternoon 'theme-settings)))))
                (entry-hashes
                 (mapcar
                  (lambda (entry)
                    (secure-hash
                     'sha256
                     (prin1-to-string entry)))
                  settings)))
         (list
          (length settings)
          (secure-hash
           'sha256
           (prin1-to-string entry-hashes))
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
           '(default
             fringe
             highlight
             mode-line
             org-block-background
             widget-field
             stripe-highlight
             term-color-white))))"##;
    let expect = expect![[
        r##"OK (411 "748e2e46cf3d5b3532276bb4dbabec0b4ac7b7cbedcab6e9a6f1388159e6ef85" ((default ((#1=((class color) (min-colors 89)) (:foreground "#eaeaea" :background "#1c1c1c")))) (fringe ((#1# (:background "#121212")))) (highlight ((#1# (:inverse-video nil :background "#121212")))) (mode-line ((#1# (:foreground nil :background "#121212" :box (:line-width 1 :color "#eaeaea") :family "Lucida Grande")))) (org-block-background ((#1# (:background "#262626")))) (widget-field ((#1# (:background "#121212" :box (:line-width 1 :color "#eaeaea"))))) (stripe-highlight ((#1# (:background "#121212")))) (term-color-white ((#1# (:foreground "#1c1c1c" :background "#1c1c1c"))))))"##
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn afternoon_theme_core_editing_faces_preserve_complete_specs() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 16777216))"##;
    let elisp_form = r##"(let ((settings
                (get 'afternoon 'theme-settings)))
         (mapcar
          (lambda (face)
            (let ((entry
                   (seq-find
                    (lambda (setting)
                      (and
                       (eq (car setting) 'theme-face)
                       (eq (nth 1 setting) face)))
                    settings)))
              (list face
                    (nth 0 entry)
                    (nth 2 entry)
                    (nth 3 entry))))
          '(default
            bold
            bold-italic
            underline
            italic
            font-lock-builtin-face
            font-lock-comment-face
            font-lock-constant-face
            font-lock-doc-face
            font-lock-function-name-face
            font-lock-keyword-face
            font-lock-string-face
            font-lock-type-face
            font-lock-variable-name-face
            font-lock-warning-face
            success
            error
            warning
            match
            isearch
            isearch-fail
            cursor
            fringe
            highlight
            mode-line
            mode-line-inactive
            region
            secondary-selection
            header-line
            trailing-whitespace
            show-paren-match-face
            show-paren-mismatch-face)))"##;
    let expect = expect![[
        r##"OK ((default theme-face afternoon ((#1=((class color) (min-colors 89)) (:foreground "#eaeaea" :background "#181a26")))) (bold theme-face afternoon ((#1# (:weight bold)))) (bold-italic theme-face afternoon ((#1# (:slant italic :weight bold)))) (underline theme-face afternoon ((#1# (:underline t)))) (italic theme-face afternoon ((#1# (:slant italic)))) (font-lock-builtin-face theme-face afternoon ((#1# (:foreground "LightCoral")))) (font-lock-comment-face theme-face afternoon ((#1# (:foreground "#969896")))) (font-lock-constant-face theme-face afternoon ((#1# (:foreground "DarkOliveGreen3")))) (font-lock-doc-face theme-face afternoon ((#1# (:foreground "moccasin")))) (font-lock-function-name-face theme-face afternoon ((#1# (:foreground "goldenrod")))) (font-lock-keyword-face theme-face afternoon ((#1# (:foreground "DeepSkyBlue1")))) (font-lock-string-face theme-face afternoon ((#1# (:foreground "burlywood")))) (font-lock-type-face theme-face afternoon ((#1# (:foreground "CadetBlue1")))) (font-lock-variable-name-face theme-face afternoon ((#1# (:foreground "#e7c547")))) (font-lock-warning-face theme-face afternoon ((#1# (:weight bold :foreground "#d54e53")))) (success theme-face afternoon ((#1# (:foreground "SeaGreen2")))) (error theme-face afternoon ((#1# (:foreground "#d54e53")))) (warning theme-face afternoon ((#1# (:foreground "goldenrod")))) (match theme-face afternoon ((#1# (:foreground "DeepSkyBlue1" :background "#181a26" :inverse-video t)))) (isearch theme-face afternoon ((#1# (:foreground "#e7c547" :background "#181a26" :inverse-video t)))) (isearch-fail theme-face afternoon ((#1# (:background "#181a26" :inherit font-lock-warning-face :inverse-video t)))) (cursor theme-face afternoon ((#1# (:background "goldenrod")))) (fringe theme-face afternoon ((#1# (:background "#14151E")))) (highlight theme-face afternoon ((#1# (:inverse-video nil :background "#14151E")))) (mode-line theme-face afternoon ((#1# (:foreground nil :background "#14151E" :box (:line-width 1 :color "#eaeaea") :family "Lucida Grande")))) (mode-line-inactive theme-face afternoon ((#1# (:inherit mode-line :foreground "#969896" :background "#14151E" :weight normal :box (:line-width 1 :color "#eaeaea"))))) (region theme-face afternoon ((#1# (:background "#103050")))) (secondary-selection theme-face afternoon ((#1# (:background "#14151E")))) (header-line theme-face afternoon ((#1# (:inherit mode-line :foreground "#c397d8" :background nil)))) (trailing-whitespace theme-face afternoon ((#1# (:foreground "#d54e53" :inverse-video t :underline nil)))) (show-paren-match-face theme-face afternoon ((#1# (:background "dodgerblue1" :foreground "white")))) (show-paren-mismatch-face theme-face afternoon ((#1# (:background "red1" :foreground "white")))))"##
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn afternoon_theme_real_ecosystem_faces_cover_major_package_workflows() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 16777216))"##;
    let elisp_form = r##"(let ((settings
                (get 'afternoon 'theme-settings)))
         (mapcar
          (lambda (face)
            (list
             face
             (copy-tree
              (nth
               3
               (seq-find
                (lambda (setting)
                  (and
                   (eq (car setting) 'theme-face)
                   (eq (nth 1 setting) face)))
                settings)))))
          '(flycheck-error
            flymake-errline
            rainbow-delimiters-depth-5-face
            mmm-code-submode-face
            anzu-replace-highlight
            ido-first-match
            flx-highlight-face
            sp-show-pair-match-face
            slime-repl-prompt-face
            diff-refine-added
            diredp-dir-heading
            magit-log-head-label-local
            git-gutter-fr:modified
            compilation-mode-line-fail
            grep-match-face
            org-block
            org-block-background
            org-document-title
            markdown-link-face
            js2-function-param
            nxml-name-face
            message-header-subject
            outline-7
            ledger-font-posting-account-pending-face
            mu4e-view-link-face
            gnus-summary-high-unread
            gnus-group-news-4
            emms-browser-artist-face
            erc-prompt-face
            twittering-timeline-header-face
            term-color-red
            term-color-cyan
            term-color-white)))"##;
    let expect = expect![[
        r##"OK ((flycheck-error ((((class color) (min-colors 89)) (:underline (:style wave :color "#d54e53"))))) (flymake-errline ((((class color) (min-colors 89)) (:underline (:style wave :color "#d54e53") :background "#181a26")))) (rainbow-delimiters-depth-5-face ((((class color) (min-colors 89)) (:foreground "DeepSkyBlue1")))) (mmm-code-submode-face ((((class color) (min-colors 89)) (:background "#14151E")))) (anzu-replace-highlight ((((class color) (min-colors 89)) (:inherit isearch-lazy-highlight-face)))) (ido-first-match ((((class color) (min-colors 89)) (:foreground "goldenrod")))) (flx-highlight-face ((((class color) (min-colors 89)) (:inherit nil :foreground "#e7c547" :weight bold :underline nil)))) (sp-show-pair-match-face ((((class color) (min-colors 89)) (:foreground nil :background nil :inherit show-paren-match)))) (slime-repl-prompt-face ((((class color) (min-colors 89)) (:underline nil :weight bold :foreground "#c397d8")))) (diff-refine-added ((((class color) (min-colors 89)) (:inherit diff-added :inverse-video t)))) (diredp-dir-heading ((((class color) (min-colors 89)) (:foreground "DarkOliveGreen3" :weight bold)))) (magit-log-head-label-local ((((class color) (min-colors 89)) (:foreground "#c397d8" :box nil :weight bold)))) (git-gutter-fr:modified ((((class color) (min-colors 89)) (:foreground "#c397d8" :weight bold)))) (compilation-mode-line-fail ((((class color) (min-colors 89)) (:foreground "#d54e53")))) (grep-match-face ((((class color) (min-colors 89)) (:foreground nil :background nil :inherit match)))) (org-block ((((class color) (min-colors 89)) (:foreground "goldenrod")))) (org-block-background ((((class color) (min-colors 89)) (:background "#1F2232")))) (org-document-title ((((class color) (min-colors 89)) (:weight bold :foreground "goldenrod" :height 1.44)))) (markdown-link-face ((((class color) (min-colors 89)) (:foreground "DeepSkyBlue1" :underline t)))) (js2-function-param ((((class color) (min-colors 89)) (:foreground "DeepSkyBlue1")))) (nxml-name-face ((((class color) (min-colors 89)) (:foreground unspecified :inherit font-lock-constant-face)))) (message-header-subject ((((class color) (min-colors 89)) (:inherit message-header-other :weight bold :foreground "#e7c547")))) (outline-7 ((((class color) (min-colors 89)) (:inherit nil :foreground "aquamarine1")))) (ledger-font-posting-account-pending-face ((((class color) (min-colors 89)) (:foreground "#e7c547")))) (mu4e-view-link-face ((((class color) (min-colors 89)) (:inherit link :foreground "DeepSkyBlue1")))) (gnus-summary-high-unread ((((class color) (min-colors 89)) (:foreground "#e7c547" :weight normal)))) (gnus-group-news-4 ((((class color) (min-colors 89)) (:foreground nil :weight normal :inherit outline-8)))) (emms-browser-artist-face ((((class color) (min-colors 89)) (:foreground "#d54e53" :height 1.3)))) (erc-prompt-face ((((class color) (min-colors 89)) (:foreground "DeepSkyBlue1")))) (twittering-timeline-header-face ((((class color) (min-colors 89)) (:foreground "DarkOliveGreen3" :weight bold)))) (term-color-red ((((class color) (min-colors 89)) (:foreground "#d54e53" :background "#d54e53")))) (term-color-cyan ((((class color) (min-colors 89)) (:foreground "#70c0b1" :background "#70c0b1")))) (term-color-white ((((class color) (min-colors 89)) (:foreground "#181a26" :background "#181a26")))))"##
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn afternoon_theme_unrelated_bold_face_specs_must_not_share_mutable_list_tails() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 16777216))"##;
    let elisp_form = r##"(let* ((settings
                 (get 'afternoon 'theme-settings))
                (attributes
                 (lambda (face)
                   (cadr
                    (car
                     (nth
                      3
                      (seq-find
                       (lambda (setting)
                         (and
                          (eq (car setting) 'theme-face)
                          (eq (nth 1 setting) face)))
                       settings))))))
                (dired-weight-tail
                 (memq
                  :weight
                  (funcall attributes 'diredp-dir-heading)))
                (twitter-weight-tail
                 (memq
                  :weight
                  (funcall
                   attributes
                   'twittering-timeline-header-face))))
         (list
          (equal dired-weight-tail twitter-weight-tail)
          (eq dired-weight-tail twitter-weight-tail)
          dired-weight-tail
          twitter-weight-tail))"##;
    let expect = expect!["OK (t nil (:weight bold) (:weight bold))"];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn afternoon_theme_legacy_and_unusual_face_forms_remain_exact() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 16777216))"##;
    let elisp_form = r##"(let ((settings
                (get 'afternoon 'theme-settings)))
         (mapcar
          (lambda (face)
            (list
             face
             (mapcar
              (lambda (entry)
                (nth 3 entry))
              (seq-filter
               (lambda (setting)
                 (and
                  (eq (car setting) 'theme-face)
                  (eq (nth 1 setting) face)))
               settings))))
          '(font-lock-negation-char-face
            border-glyph
            edts-face-warning-line
            edts-face-error-line
            jabber-roster-user-xa
            jabber-roster-user-dnd
            gnus-header-from
            erc-keyword-face)))"##;
    let expect = expect![[
        r##"OK ((font-lock-negation-char-face (((#1=((class color) (min-colors 89)) (:foreground "DeepSkyBlue1"))))) (border-glyph (((#1# (nil))))) (edts-face-warning-line (((t (:background nil :inherit flymake-warnline))))) (edts-face-error-line (((t (:background nil :inherit flymake-errline))))) (jabber-roster-user-xa (((#1# :foreground "#969896")))) (jabber-roster-user-dnd (((#1# :foreground "#e7c547")))) (gnus-header-from (((#1# (:inherit message-header-other-face :weight bold :foreground "goldenrod"))))) (erc-keyword-face (((#1# (:foreground "DarkOliveGreen3"))) ((#1# (:foreground "#e7c547"))))))"##
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}

#[test]
fn afternoon_theme_all_rainbow_outline_and_terminal_sequences_match() {
    let prelude = r##"(fset 'display-color-cells
               (lambda (&optional _display) 16777216))"##;
    let elisp_form = r##"(let ((settings
                (get 'afternoon 'theme-settings)))
         (mapcar
          (lambda (prefix-and-count)
            (pcase-let ((`(,prefix ,count) prefix-and-count))
              (mapcar
               (lambda (index)
                 (let* ((face
                         (intern
                          (format "%s%d%s"
                                  prefix
                                  index
                                  (if
                                   (string-prefix-p
                                    "rainbow"
                                    prefix)
                                   "-face"
                                 ""))))
                        (entry
                         (seq-find
                          (lambda (setting)
                            (and
                             (eq (car setting) 'theme-face)
                             (eq (nth 1 setting) face)))
                          settings)))
                   (list face (nth 3 entry))))
               (number-sequence 1 count))))
          '(("rainbow-delimiters-depth-" 9)
            ("outline-" 9)
            ("gnus-cite-" 8))))"##;
    let expect = expect![[
        r##"OK (((rainbow-delimiters-depth-1-face ((#1=((class color) (min-colors 89)) (:foreground "#eaeaea")))) (rainbow-delimiters-depth-2-face ((#1# (:foreground "#70c0b1")))) (rainbow-delimiters-depth-3-face ((#1# (:foreground "#e7c547")))) (rainbow-delimiters-depth-4-face ((#1# (:foreground "DarkOliveGreen3")))) (rainbow-delimiters-depth-5-face ((#1# (:foreground "DeepSkyBlue1")))) (rainbow-delimiters-depth-6-face ((#1# (:foreground "#eaeaea")))) (rainbow-delimiters-depth-7-face ((#1# (:foreground "#70c0b1")))) (rainbow-delimiters-depth-8-face ((#1# (:foreground "#e7c547")))) (rainbow-delimiters-depth-9-face ((#1# (:foreground "DarkOliveGreen3"))))) ((outline-1 ((#1# (:inherit nil :foreground "SkyBlue1")))) (outline-2 ((#1# (:inherit nil :foreground "#e7c547")))) (outline-3 ((#1# (:inherit nil :foreground "#c397d8")))) (outline-4 ((#1# (:inherit nil :foreground "#70c0b1")))) (outline-5 ((#1# (:inherit nil :foreground "goldenrod")))) (outline-6 ((#1# (:inherit nil :foreground "CadetBlue1")))) (outline-7 ((#1# (:inherit nil :foreground "aquamarine1")))) (outline-8 ((#1# (:inherit nil :foreground "turquoise2")))) (outline-9 ((#1# (:inherit nil :foreground "LightSteelBlue1"))))) ((gnus-cite-1 ((#1# (:inherit outline-1 :foreground nil)))) (gnus-cite-2 ((#1# (:inherit outline-2 :foreground nil)))) (gnus-cite-3 ((#1# (:inherit outline-3 :foreground nil)))) (gnus-cite-4 ((#1# (:inherit outline-4 :foreground nil)))) (gnus-cite-5 ((#1# (:inherit outline-5 :foreground nil)))) (gnus-cite-6 ((#1# (:inherit outline-6 :foreground nil)))) (gnus-cite-7 ((#1# (:inherit outline-7 :foreground nil)))) (gnus-cite-8 ((#1# (:inherit outline-8 :foreground nil))))))"##
    ]];
    assert_afternoon_theme_with_prelude_parity(prelude, elisp_form, expect);
}
