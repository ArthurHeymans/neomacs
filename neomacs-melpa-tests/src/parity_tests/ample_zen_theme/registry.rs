use expect_test::expect;

use super::{assert_ample_zen_theme_autoload_parity, assert_ample_zen_theme_parity};

#[test]
fn ample_zen_theme_loads_exact_dependency_free_package_and_theme_metadata() {
    let elisp_form = r##"(let* ((description
        (cadr (assq 'ample-zen-theme package-alist)))
       (directory
        (file-name-as-directory (package-desc-dir description))))
  (list
   (package-installed-p 'ample-zen-theme)
   (package-version-join (package-desc-version description))
   (package-desc-reqs description)
   (package-desc-kind description)
   (package-desc-archive description)
   (featurep 'ample-zen-theme)
   (custom-theme-p 'ample-zen)
   (get 'ample-zen 'theme-documentation)
   (member directory custom-theme-load-path)
   (file-readable-p
    (expand-file-name "ample-zen-theme.el" directory))))"##;
    let expect = expect![[
        r#"OK (t "20150119.2154" nil nil nil t (ample-zen user changed) "The AmpleZen color theme" ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ample-zen-theme/20150119.2154/home/.emacs.d/elpa/ample-zen-theme-20150119.2154/" custom-theme-directory t) t)"#
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn palette_preserves_all_ordered_named_colors_used_by_visual_contracts() {
    let elisp_form = r##"(list
 (length ample-zen-colors-alist)
 (mapcar #'car ample-zen-colors-alist)
 (mapcar #'cdr ample-zen-colors-alist)
 (length
  (delete-dups
   (mapcar #'car
           (copy-tree ample-zen-colors-alist))))
 (length
  (delete-dups
   (mapcar #'cdr
           (copy-tree ample-zen-colors-alist)))))"##;
    let expect = expect![[
        r##"OK (36 ("ample-zen-fg-1" "ample-zen-fg" "ample-zen-fg+1" "ample-zen-bg-2" "ample-zen-bg-1" "ample-zen-bg-05" "ample-zen-bg" "ample-zen-bg+1" "ample-zen-bg+2" "ample-zen-bg+3" "ample-zen-red+1" "ample-zen-red" "ample-zen-red-1" "ample-zen-red-2" "ample-zen-red-3" "ample-zen-red-4" "ample-zen-orange-1" "ample-zen-orange" "ample-zen-yellow" "ample-zen-yellow-1" "ample-zen-yellow-2" "ample-zen-green-1" "ample-zen-green" "ample-zen-green+1" "ample-zen-green+2" "ample-zen-green+3" "ample-zen-green+4" "ample-zen-cyan" "ample-zen-blue+1" "ample-zen-blue" "ample-zen-blue-1" "ample-zen-blue-2" "ample-zen-blue-3" "ample-zen-blue-4" "ample-zen-blue-5" "ample-zen-magenta") ("#c9c9c9" "#bdbdb3" "#9b9b9b" "#4c4c4c" "#3b3b3b" "#2e2e2e" "#212121" "#141414" "#0a0a0a" "#000000" "#AA5542" "#CC5542" "#dd5542" "#ee5542" "#ff5542" "#ff6642" "#cc8512" "#fb8512" "#7d7c61" "#bdbc61" "#baba36" "#6abd50" "#6aaf50" "#6aa350" "#6a9550" "#6a8550" "#6a7550" "#9b55c3" "#6380b3" "#5180b3" "#528fd1" "#6CA0A3" "#5C888B" "#4C7073" "#366060" "#DC8CC3") 36 36)"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn color_variable_macro_binds_class_and_every_palette_role_without_leaking() {
    let elisp_form = r##"(list
 (macroexpand-1
  '(ample-zen-with-color-variables
     (list class ample-zen-fg ample-zen-bg)))
 (ample-zen-with-color-variables
   (list
    class
    ample-zen-fg-1 ample-zen-fg ample-zen-fg+1
    ample-zen-bg-2 ample-zen-bg-1 ample-zen-bg-05
    ample-zen-bg ample-zen-bg+1 ample-zen-bg+2 ample-zen-bg+3
    ample-zen-red+1 ample-zen-red ample-zen-red-1
    ample-zen-red-2 ample-zen-red-3 ample-zen-red-4
    ample-zen-orange-1 ample-zen-orange
    ample-zen-yellow ample-zen-yellow-1 ample-zen-yellow-2
    ample-zen-green-1 ample-zen-green ample-zen-green+1
    ample-zen-green+2 ample-zen-green+3
    ample-zen-green+4 ample-zen-cyan
    ample-zen-blue+1 ample-zen-blue ample-zen-blue-1
    ample-zen-blue-2 ample-zen-blue-3
    ample-zen-blue-4 ample-zen-blue-5
    ample-zen-magenta))
 (mapcar
  #'boundp
  '(class ample-zen-fg ample-zen-bg ample-zen-magenta)))"##;
    let expect = expect![[
        r##"OK ((let ((class '#1=((class color) (min-colors 89))) (ample-zen-fg-1 "#c9c9c9") (ample-zen-fg "#bdbdb3") (ample-zen-fg+1 "#9b9b9b") (ample-zen-bg-2 "#4c4c4c") (ample-zen-bg-1 "#3b3b3b") (ample-zen-bg-05 "#2e2e2e") (ample-zen-bg "#212121") (ample-zen-bg+1 "#141414") (ample-zen-bg+2 "#0a0a0a") (ample-zen-bg+3 "#000000") (ample-zen-red+1 "#AA5542") (ample-zen-red "#CC5542") (ample-zen-red-1 "#dd5542") (ample-zen-red-2 "#ee5542") (ample-zen-red-3 "#ff5542") (ample-zen-red-4 "#ff6642") (ample-zen-orange-1 "#cc8512") (ample-zen-orange "#fb8512") (ample-zen-yellow "#7d7c61") (ample-zen-yellow-1 "#bdbc61") (ample-zen-yellow-2 "#baba36") (ample-zen-green-1 "#6abd50") (ample-zen-green "#6aaf50") (ample-zen-green+1 "#6aa350") (ample-zen-green+2 "#6a9550") (ample-zen-green+3 "#6a8550") (ample-zen-green+4 "#6a7550") (ample-zen-cyan "#9b55c3") (ample-zen-blue+1 "#6380b3") (ample-zen-blue "#5180b3") (ample-zen-blue-1 "#528fd1") (ample-zen-blue-2 "#6CA0A3") (ample-zen-blue-3 "#5C888B") (ample-zen-blue-4 "#4C7073") (ample-zen-blue-5 "#366060") (ample-zen-magenta "#DC8CC3")) (list class ample-zen-fg ample-zen-bg)) (#1# "#c9c9c9" "#bdbdb3" "#9b9b9b" "#4c4c4c" "#3b3b3b" "#2e2e2e" "#212121" "#141414" "#0a0a0a" "#000000" "#AA5542" "#CC5542" "#dd5542" "#ee5542" "#ff5542" "#ff6642" "#cc8512" "#fb8512" "#7d7c61" "#bdbc61" "#baba36" "#6abd50" "#6aaf50" "#6aa350" "#6a9550" "#6a8550" "#6a7550" "#9b55c3" "#6380b3" "#5180b3" "#528fd1" "#6CA0A3" "#5C888B" "#4C7073" "#366060" "#DC8CC3") (nil nil nil nil))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn theme_registers_complete_unique_ordered_face_surface() {
    let elisp_form = r##"(let* ((settings (get 'ample-zen 'theme-settings))
       (face-settings
        (delq nil
              (mapcar
               (lambda (setting)
                 (and (eq (car setting) 'theme-face)
                      setting))
               settings)))
       (names (mapcar #'cadr face-settings)))
  (list
   (length settings)
   (length face-settings)
   (length
    (delete-dups (copy-sequence names)))
   (car names)
   (car (last names))
   (vconcat names)))"##;
    let expect = expect![
        "OK (426 421 421 which-func button [which-func whitespace-space-after-tab whitespace-empty whitespace-indentation whitespace-space-before-tab whitespace-line whitespace-trailing whitespace-newline whitespace-tab whitespace-hspace whitespace-space web-mode-whitespaces-face web-mode-warning-face web-mode-symbol-face web-mode-server-string-face web-mode-server-comment-face web-mode-server-background-face web-mode-variable-name-face web-mode-type-face web-mode-string-face web-mode-preprocessor-face web-mode-keyword-face web-mode-html-tag-face web-mode-html-attr-value-face web-mode-html-attr-name-face web-mode-function-name-face web-mode-folded-face web-mode-doctype-face web-mode-css-rule-face web-mode-css-pseudo-class-face web-mode-css-prop-face web-mode-css-at-rule-face web-mode-constant-face web-mode-comment-face web-mode-builtin-face w3m-lnum-minibuffer-prompt w3m-lnum-match w3m-lnum w3m-history-current-url w3m-header-line-location-title w3m-form w3m-arrived-anchor w3m-anchor vhl/default-face term-default-bg-color term-default-fg-color term-color-white term-color-cyan term-color-magenta term-color-blue term-color-yellow term-color-green term-color-red term-color-black slime-repl-inputed-output-face sml-modeline-end-face show-paren-match show-paren-mismatch rst-level-6-face rst-level-5-face rst-level-4-face rst-level-3-face rst-level-2-face rst-level-1-face rpm-spec-var-face rpm-spec-tag-face rpm-spec-section-face rpm-spec-package-face rpm-spec-obsolete-tag-face rpm-spec-macro-face rpm-spec-ghost-face rpm-spec-doc-face rpm-spec-dir-face rcirc-keyword rcirc-url rcirc-track-keyword rcirc-track-nick rcirc-prompt rcirc-nick-in-message-full-line rcirc-nick-in-message rcirc-timestamp rcirc-server-prefix rcirc-server rcirc-dim-nick rcirc-bright-nick rcirc-other-nick rcirc-my-nick rbenv-active-ruby-face rainbow-delimiters-depth-12-face rainbow-delimiters-depth-11-face rainbow-delimiters-depth-10-face rainbow-delimiters-depth-9-face rainbow-delimiters-depth-8-face rainbow-delimiters-depth-7-face rainbow-delimiters-depth-6-face rainbow-delimiters-depth-5-face rainbow-delimiters-depth-4-face rainbow-delimiters-depth-3-face rainbow-delimiters-depth-2-face rainbow-delimiters-depth-1-face powerline-inactive2 powerline-inactive1 powerline-active2 powerline-active1 p4-diff-ins-face p4-diff-head-face p4-diff-file-face p4-diff-del-face p4-diff-change-face p4-depot-unmapped-face p4-depot-deleted-face p4-depot-branch-op-face p4-depot-added-face outline-8 outline-7 outline-6 outline-5 outline-4 outline-3 outline-2 outline-1 org-column-title org-column org-warning org-upcoming-deadline org-todo org-time-grid org-tag org-table org-special-keyword org-sexp-date org-scheduled-today org-scheduled-previously org-scheduled org-link org-level-8 org-level-7 org-level-6 org-level-5 org-level-4 org-level-3 org-level-2 org-level-1 org-hide org-headline-done org-formula org-done org-deadline-announce org-date org-checkbox org-archived org-agenda-structure org-agenda-date-today mu4e-trashed-face mu4e-replied-face mu4e-cited-7-face mu4e-cited-6-face mu4e-cited-5-face mu4e-cited-4-face mu4e-cited-3-face mu4e-cited-2-face mu4e-cited-1-face nav-face-hfile nav-face-file nav-face-hdir nav-face-dir nav-face-button-num nav-face-heading paren-face-no-match paren-face-mismatch paren-face-match message-separator message-mml message-header-xheader message-header-subject message-header-newsgroups message-header-cc message-header-from message-header-to message-header-other message-header-name message-cited-text magit-log-sha1 magit-item-highlight magit-branch magit-header magit-section-title linum jabber-title-large jabber-title-medium jabber-title-small jabber-activity-personal-face jabber-activity-face jabber-chat-prompt-foreign jabber-chat-prompt-local jabber-rare-time-face jabber-roster-user-dnd jabber-roster-user-online jabber-roster-user-away ido-subdir ido-incomplete-regex ido-only-match ido-first-match hl-sexp-face hl-line hl-line-face gnus-x gnus-signature gnus-group-news-low-empty gnus-group-news-6-empty gnus-group-news-5-empty gnus-group-news-4-empty gnus-group-news-3-empty gnus-group-news-2-empty gnus-group-news-1-empty gnus-cite-9 gnus-cite-8 gnus-cite-7 gnus-cite-6 gnus-cite-5 gnus-cite-4 gnus-cite-3 gnus-cite-2 gnus-cite-11 gnus-cite-10 gnus-cite-1 gnus-summary-selected gnus-summary-normal-unread gnus-summary-normal-ticked gnus-summary-normal-read gnus-summary-normal-ancient gnus-summary-low-unread gnus-summary-low-ticked gnus-summary-low-read gnus-summary-low-ancient gnus-summary-high-unread gnus-summary-high-ticked gnus-summary-high-read gnus-summary-high-ancient gnus-summary-cancelled gnus-header-subject gnus-header-newsgroups gnus-header-name gnus-header-from gnus-header-content gnus-group-news-low gnus-group-news-6 gnus-group-news-5 gnus-group-news-4 gnus-group-news-3 gnus-group-news-2 gnus-group-news-1 gnus-group-mail-low-empty gnus-group-mail-low gnus-group-mail-6-empty gnus-group-mail-6 gnus-group-mail-5-empty gnus-group-mail-5 gnus-group-mail-4-empty gnus-group-mail-4 gnus-group-mail-3-empty gnus-group-mail-3 gnus-group-mail-2-empty gnus-group-mail-2 gnus-group-mail-1-empty gnus-group-mail-1 git-gutter-fr:modified git-gutter-fr:deleted git-gutter-fr:added git-gutter:unchanged git-gutter:modified git-gutter:deleted git-gutter:added erc-underline-face erc-timestamp-face erc-prompt-face erc-pal-face erc-notice-face erc-nick-msg-face erc-my-nick-face erc-nick-default-face erc-keyword-face erc-input-face erc-highlight-face erc-fool-face erc-error-face erc-direct-msg-face erc-default-face erc-dangerous-host-face erc-current-nick-face erc-bold-face erc-action-face flyspell-incorrect flyspell-duplicate flymake-infoline flymake-warnline flymake-errline flycheck-fringe-warning flycheck-fringe-error flycheck-warning flycheck-error flx-highlight-face eshell-ls-symlink eshell-ls-special eshell-ls-product eshell-ls-missing eshell-ls-unreadable eshell-ls-executable eshell-ls-directory eshell-ls-clutter eshell-ls-backup eshell-ls-archive eshell-prompt ediff-odd-diff-C ediff-odd-diff-B ediff-odd-diff-Ancestor ediff-odd-diff-A ediff-fine-diff-C ediff-fine-diff-B ediff-fine-diff-Ancestor ediff-fine-diff-A ediff-even-diff-C ediff-even-diff-B ediff-even-diff-Ancestor ediff-even-diff-A ediff-current-diff-C ediff-current-diff-B ediff-current-diff-Ancestor ediff-current-diff-A diff-file-header diff-header diff-refine-removed diff-refine-change diff-refine-added diff-removed diff-changed diff-added clojure-test-success-face clojure-test-error-face clojure-test-failure-face android-mode-warning-face android-mode-verbose-face android-mode-info-face android-mode-error-face android-mode-debug-face popup-isearch-match popup-scroll-bar-background-face popup-scroll-bar-foreground-face popup-tip-face popup-face ac-selection-face ac-candidate-face font-latex-sedate-face font-latex-sectioning-5-face font-latex-warning-face font-latex-bold-face ack-match ack-line ack-file ack-separator anzu-mode-line c-annotation-face font-lock-warning-face font-lock-variable-name-face font-lock-type-face font-lock-string-face font-lock-regexp-grouping-backslash font-lock-regexp-grouping-construct font-lock-preprocessor-face font-lock-negation-char-face font-lock-keyword-face font-lock-function-name-face font-lock-doc-face font-lock-constant-face font-lock-comment-delimiter-face font-lock-comment-face font-lock-builtin-face lazy-highlight isearch-fail isearch match grep-match-face grep-hit-face grep-error-face grep-context-face compilation-mode-line-run compilation-mode-line-fail compilation-mode-line-exit compilation-warning-face compilation-message-face compilation-line-number compilation-line-face compilation-leave-directory-face compilation-info compilation-info-face compilation-face compilation-error-face compilation-enter-directory-face compilation-column-face scroll-bar vertical-border trailing-whitespace secondary-selection region mode-line-inactive mode-line-buffer-id mode-line minibuffer-prompt menu error warning success highlight header-line fringe escape-glyph cursor default link-visited link button])"
    ];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn theme_registers_exact_ansi_fill_column_and_version_control_variables() {
    let elisp_form = r##"(let* ((settings (get 'ample-zen 'theme-settings))
       (value-settings
        (delq nil
              (mapcar
               (lambda (setting)
                 (and (eq (car setting) 'theme-value)
                      setting))
               settings))))
  (list
   (length value-settings)
   value-settings
   (mapcar
    (lambda (symbol)
      (list
       symbol
       (boundp symbol)
       (and (boundp symbol)
            (copy-tree (symbol-value symbol)))))
    '(ansi-color-names-vector
      fci-rule-color
      vc-annotate-color-map
      vc-annotate-very-old-color
      vc-annotate-background))))"##;
    let expect = expect![[
        r##"OK (5 ((theme-value vc-annotate-background ample-zen "#3b3b3b") (theme-value vc-annotate-very-old-color ample-zen "#DC8CC3") (theme-value vc-annotate-color-map ample-zen '((20 . "#dd5542") (40 . "#CC5542") (60 . "#fb8512") (80 . "#baba36") (100 . "#bdbc61") (120 . "#7d7c61") (140 . "#6abd50") (160 . "#6aaf50") (180 . "#6aa350") (200 . "#6a9550") (220 . "#6a8550") (240 . "#6a7550") (260 . "#9b55c3") (280 . "#6CA0A3") (300 . "#528fd1") (320 . "#5180b3") (340 . "#6380b3") (360 . "#DC8CC3"))) (theme-value fci-rule-color ample-zen "#2e2e2e") (theme-value ansi-color-names-vector ample-zen ["#212121" "#CC5542" "#6aaf50" "#7d7c61" "#5180b3" "#DC8CC3" "#9b55c3" "#bdbdb3"])) ((ansi-color-names-vector nil nil) (fci-rule-color nil nil) (vc-annotate-color-map nil nil) (vc-annotate-very-old-color nil nil) (vc-annotate-background nil nil)))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn generated_autoloads_register_theme_directory_and_safe_local_rainbow_form() {
    let elisp_form = r##"(let* ((description
        (cadr (assq 'ample-zen-theme package-alist)))
       (directory
        (file-name-as-directory (package-desc-dir description)))
       (safe-form
        '(when
             (require 'rainbow-mode nil t)
           (rainbow-mode 1))))
  (list
   (featurep 'ample-zen-theme)
   (custom-theme-p 'ample-zen)
   (member directory custom-theme-load-path)
   (member safe-form safe-local-eval-forms)
   (boundp 'ample-zen-colors-alist)
   (boundp 'ample-zen-add-font-lock-keywords)))"##;
    let expect = expect![[
        r#"OK (nil nil ("[ORACLE-WORKSPACE]/tmp/melpa/package-cache/ample-zen-theme/20150119.2154/home/.emacs.d/elpa/ample-zen-theme-20150119.2154/" custom-theme-directory t) ((when (require 'rainbow-mode nil t) (rainbow-mode 1)) (add-hook 'write-file-hooks 'time-stamp) (add-hook 'write-file-functions 'time-stamp) (add-hook 'before-save-hook 'time-stamp nil t) (add-hook 'before-save-hook 'delete-trailing-whitespace nil t)) nil nil)"#
    ]];
    assert_ample_zen_theme_autoload_parity(elisp_form, expect);
}

#[test]
fn rainbow_configuration_defaults_and_safe_local_form_are_registered_after_source_load() {
    let elisp_form = r##"(let ((safe-form
       '(when
            (require 'rainbow-mode nil t)
          (rainbow-mode 1))))
  (list
   ample-zen-add-font-lock-keywords
   ample-zen-colors-font-lock-keywords
   (get 'ample-zen-add-font-lock-keywords
        'variable-documentation)
   (get 'ample-zen-colors-font-lock-keywords
        'variable-documentation)
   (member safe-form safe-local-eval-forms)
   (get 'ample-zen 'theme-feature)))"##;
    let expect = expect![[
        r#"OK (nil nil "Whether to add font-lock keywords for ample-zen color names.\nIn buffers visiting library `ample-zen-theme.el' the ample-zen\nspecific keywords are always added.  In all other Emacs-Lisp\nbuffers this variable controls whether this should be done.\nThis requires library `rainbow-mode'." nil ((when (require 'rainbow-mode nil t) (rainbow-mode 1)) (add-hook 'write-file-hooks 'time-stamp) (add-hook 'write-file-functions 'time-stamp) (add-hook 'before-save-hook 'time-stamp nil t) (add-hook 'before-save-hook 'delete-trailing-whitespace nil t)) ample-zen-theme)"#
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}
