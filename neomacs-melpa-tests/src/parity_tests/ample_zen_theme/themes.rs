use expect_test::expect;

use super::assert_ample_zen_theme_parity;

#[test]
fn core_face_specs_preserve_dark_canvas_selection_cursor_and_mode_line_contracts() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((setting
         (lambda (face)
           (copy-tree
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             (get 'ample-zen 'theme-settings))))))
    (mapcar
     setting
     '(default cursor fringe header-line highlight
       region secondary-selection trailing-whitespace
       mode-line mode-line-buffer-id mode-line-inactive
       vertical-border scroll-bar
       minibuffer-prompt link link-visited
       success warning error))))"##;
    let expect = expect![[
        r##"OK ((theme-face default ample-zen ((t (:foreground "#bdbdb3" :background "#212121")))) (theme-face cursor ample-zen ((t (:foreground "#bdbdb3" :background "#cc8512")))) (theme-face fringe ample-zen ((t (:foreground "#bdbdb3" :background "#212121")))) (theme-face header-line ample-zen ((t (:foreground "#7d7c61" :background "#3b3b3b" :box (:line-width -1 :style released-button))))) (theme-face highlight ample-zen ((t (:background "#2e2e2e")))) (theme-face region ample-zen ((((class color) (min-colors 89)) (:background "#3b3b3b")) (t :inverse-video t))) (theme-face secondary-selection ample-zen ((t (:background "#0a0a0a")))) (theme-face trailing-whitespace ample-zen ((t (:background "#CC5542")))) (theme-face mode-line ample-zen ((((class color) (min-colors 89)) (:foreground "#c9c9c9" :background "#000000" :box (:line-width -1 :style released-button))) (t :inverse-video t))) (theme-face mode-line-buffer-id ample-zen ((t (:foreground "#cc8512" :weight bold)))) (theme-face mode-line-inactive ample-zen ((t (:foreground "#9b9b9b" :background "#3b3b3b" :box nil :weight light)))) (theme-face vertical-border ample-zen ((t (:foreground "#bdbdb3")))) (theme-face scroll-bar ample-zen ((t (:background "#0a0a0a" :foreground "#9b9b9b")))) (theme-face minibuffer-prompt ample-zen ((t (:foreground "#7d7c61")))) (theme-face link ample-zen ((t (:foreground "#7d7c61" :underline t :weight bold)))) (theme-face link-visited ample-zen ((t (:foreground "#baba36" :underline t :weight normal)))) (theme-face success ample-zen ((t (:foreground "#6aaf50" :weight bold)))) (theme-face warning ample-zen ((t (:foreground "#fb8512" :weight bold)))) (theme-face error ample-zen ((t (:foreground "#AA5542" :weight bold)))))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn font_lock_specs_preserve_practical_code_semantics_across_every_token_role() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((setting
         (lambda (face)
           (copy-tree
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             (get 'ample-zen 'theme-settings))))))
    (mapcar
     setting
     '(font-lock-builtin-face
       font-lock-comment-face
       font-lock-comment-delimiter-face
       font-lock-constant-face
       font-lock-doc-face
       font-lock-function-name-face
       font-lock-keyword-face
       font-lock-negation-char-face
       font-lock-preprocessor-face
       font-lock-regexp-grouping-construct
       font-lock-regexp-grouping-backslash
       font-lock-string-face
       font-lock-type-face
       font-lock-variable-name-face
       font-lock-warning-face
       c-annotation-face))))"##;
    let expect = expect![[
        r##"OK ((theme-face font-lock-builtin-face ample-zen ((t (:foreground "#bdbdb3" :weight bold)))) (theme-face font-lock-comment-face ample-zen ((t (:foreground "#6aaf50")))) (theme-face font-lock-comment-delimiter-face ample-zen ((t (:foreground "#6abd50")))) (theme-face font-lock-constant-face ample-zen ((t (:foreground "#6a7550")))) (theme-face font-lock-doc-face ample-zen ((t (:foreground "#6a9550")))) (theme-face font-lock-function-name-face ample-zen ((t (:foreground "#9b55c3")))) (theme-face font-lock-keyword-face ample-zen ((t (:foreground "#7d7c61" :weight bold)))) (theme-face font-lock-negation-char-face ample-zen ((t (:foreground "#7d7c61" :weight bold)))) (theme-face font-lock-preprocessor-face ample-zen ((t (:foreground "#6380b3")))) (theme-face font-lock-regexp-grouping-construct ample-zen ((t (:foreground "#7d7c61" :weight bold)))) (theme-face font-lock-regexp-grouping-backslash ample-zen ((t (:foreground "#6aaf50" :weight bold)))) (theme-face font-lock-string-face ample-zen ((t (:foreground "#CC5542")))) (theme-face font-lock-type-face ample-zen ((t (:foreground "#528fd1")))) (theme-face font-lock-variable-name-face ample-zen ((t (:foreground "#fb8512")))) (theme-face font-lock-warning-face ample-zen ((t (:foreground "#baba36" :weight bold)))) (theme-face c-annotation-face ample-zen ((t (:inherit font-lock-constant-face)))))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn compilation_grep_search_and_diff_specs_keep_success_warning_error_distinct() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((setting
         (lambda (face)
           (copy-tree
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             (get 'ample-zen 'theme-settings))))))
    (mapcar
     setting
     '(compilation-info compilation-warning-face
       compilation-error-face compilation-mode-line-exit
       compilation-mode-line-fail compilation-mode-line-run
       grep-context-face grep-error-face grep-hit-face
       grep-match-face match isearch isearch-fail
       lazy-highlight diff-added diff-changed diff-removed
       diff-refine-added diff-refine-change
       diff-refine-removed diff-header diff-file-header))))"##;
    let expect = expect![[
        r##"OK ((theme-face compilation-info ample-zen ((t (:foreground "#6a7550" :underline t)))) (theme-face compilation-warning-face ample-zen ((t (:foreground "#fb8512" :weight bold :underline t)))) (theme-face compilation-error-face ample-zen ((t (:foreground "#dd5542" :weight bold :underline t)))) (theme-face compilation-mode-line-exit ample-zen ((t (:foreground "#6a9550" :weight bold)))) (theme-face compilation-mode-line-fail ample-zen ((t (:foreground "#CC5542" :weight bold)))) (theme-face compilation-mode-line-run ample-zen ((t (:foreground "#7d7c61" :weight bold)))) (theme-face grep-context-face ample-zen ((t (:foreground "#bdbdb3")))) (theme-face grep-error-face ample-zen ((t (:foreground "#dd5542" :weight bold :underline t)))) (theme-face grep-hit-face ample-zen ((t (:foreground "#5180b3")))) (theme-face grep-match-face ample-zen ((t (:foreground "#fb8512" :weight bold)))) (theme-face match ample-zen ((t (:background "#3b3b3b" :foreground "#fb8512" :weight bold)))) (theme-face isearch ample-zen ((t (:foreground "#baba36" :weight bold :background "#3b3b3b")))) (theme-face isearch-fail ample-zen ((t (:foreground "#bdbdb3" :background "#ff6642")))) (theme-face lazy-highlight ample-zen ((t (:foreground "#baba36" :weight bold :background "#2e2e2e")))) (theme-face diff-added ample-zen ((((class color) (min-colors 89)) (:foreground "#6a7550" :background nil)) (t (:foreground "#6abd50" :background nil)))) (theme-face diff-changed ample-zen ((t (:foreground "#7d7c61")))) (theme-face diff-removed ample-zen ((((class color) (min-colors 89)) (:foreground "#CC5542" :background nil)) (t (:foreground "#ff5542" :background nil)))) (theme-face diff-refine-added ample-zen ((t :inherit diff-added :weight bold))) (theme-face diff-refine-change ample-zen ((t :inherit diff-changed :weight bold))) (theme-face diff-refine-removed ample-zen ((t :inherit diff-removed :weight bold))) (theme-face diff-header ample-zen ((((class color) (min-colors 89)) (:background "#0a0a0a")) (t (:background "#bdbdb3" :foreground "#212121")))) (theme-face diff-file-header ample-zen ((((class color) (min-colors 89)) (:background "#0a0a0a" :foreground "#bdbdb3" :bold t)) (t (:background "#bdbdb3" :foreground "#212121" :bold t)))))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn diagnostic_specs_preserve_wave_underline_capability_fallbacks_and_colors() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((setting
         (lambda (face)
           (copy-tree
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             (get 'ample-zen 'theme-settings))))))
    (mapcar
     setting
     '(flycheck-error flycheck-warning
       flycheck-fringe-error flycheck-fringe-warning
       flymake-errline flymake-warnline flymake-infoline
       flyspell-duplicate flyspell-incorrect
       clojure-test-failure-face
       clojure-test-error-face
       clojure-test-success-face))))"##;
    let expect = expect![[
        r##"OK ((theme-face flycheck-error ample-zen ((((supports :underline (:style wave))) (:underline (:style wave :color "#CC5542") :inherit unspecified)) (t (:foreground "#dd5542" :weight bold :underline t)))) (theme-face flycheck-warning ample-zen ((((supports :underline (:style wave))) (:underline (:style wave :color "#fb8512") :inherit unspecified)) (t (:foreground "#fb8512" :weight bold :underline t)))) (theme-face flycheck-fringe-error ample-zen ((t (:foreground "#dd5542" :weight bold)))) (theme-face flycheck-fringe-warning ample-zen ((t (:foreground "#fb8512" :weight bold)))) (theme-face flymake-errline ample-zen ((((supports :underline (:style wave))) (:underline (:style wave :color "#CC5542") :inherit unspecified :foreground unspecified :background unspecified)) (t (:foreground "#dd5542" :weight bold :underline t)))) (theme-face flymake-warnline ample-zen ((((supports :underline (:style wave))) (:underline (:style wave :color "#fb8512") :inherit unspecified :foreground unspecified :background unspecified)) (t (:foreground "#fb8512" :weight bold :underline t)))) (theme-face flymake-infoline ample-zen ((((supports :underline (:style wave))) (:underline (:style wave :color "#6aaf50") :inherit unspecified :foreground unspecified :background unspecified)) (t (:foreground "#6abd50" :weight bold :underline t)))) (theme-face flyspell-duplicate ample-zen ((((supports :underline (:style wave))) (:underline (:style wave :color "#fb8512") :inherit unspecified)) (t (:foreground "#fb8512" :weight bold :underline t)))) (theme-face flyspell-incorrect ample-zen ((((supports :underline (:style wave))) (:underline (:style wave :color "#CC5542") :inherit unspecified)) (t (:foreground "#dd5542" :weight bold :underline t)))) (theme-face clojure-test-failure-face ample-zen ((t (:foreground "#fb8512" :weight bold :underline t)))) (theme-face clojure-test-error-face ample-zen ((t (:foreground "#CC5542" :weight bold :underline t)))) (theme-face clojure-test-success-face ample-zen ((t (:foreground "#6aa350" :weight bold :underline t)))))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn org_and_outline_specs_cover_real_planning_document_structure_and_state() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((setting
         (lambda (face)
           (copy-tree
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             (get 'ample-zen 'theme-settings))))))
    (mapcar
     setting
     '(org-agenda-date-today org-agenda-structure
       org-archived org-checkbox org-date
       org-deadline-announce org-done org-formula
       org-headline-done org-hide
       org-level-1 org-level-2 org-level-3 org-level-4
       org-level-5 org-level-6 org-level-7 org-level-8
       org-link org-scheduled org-scheduled-previously
       org-scheduled-today org-sexp-date
       org-special-keyword org-table org-tag
       org-time-grid org-todo org-upcoming-deadline
       org-warning org-column org-column-title
       outline-1 outline-2 outline-3 outline-4
       outline-5 outline-6 outline-7 outline-8))))"##;
    let expect = expect![[
        r##"OK ((theme-face org-agenda-date-today ample-zen ((t (:foreground "white" :slant italic :weight bold)))) (theme-face org-agenda-structure ample-zen ((t (:inherit font-lock-comment-face)))) (theme-face org-archived ample-zen ((t (:foreground "#bdbdb3" :weight bold)))) (theme-face org-checkbox ample-zen ((t (:background "#0a0a0a" :foreground "white" :box (:line-width 1 :style released-button))))) (theme-face org-date ample-zen ((t (:foreground "#5180b3" :underline t)))) (theme-face org-deadline-announce ample-zen ((t (:foreground "#dd5542")))) (theme-face org-done ample-zen ((t (:bold t :weight bold :foreground "#6a8550")))) (theme-face org-formula ample-zen ((t (:foreground "#baba36")))) (theme-face org-headline-done ample-zen ((t (:foreground "#6a8550")))) (theme-face org-hide ample-zen ((t (:foreground "#3b3b3b")))) (theme-face org-level-1 ample-zen ((t (:foreground "#fb8512")))) (theme-face org-level-2 ample-zen ((t (:foreground "#6a7550")))) (theme-face org-level-3 ample-zen ((t (:foreground "#528fd1")))) (theme-face org-level-4 ample-zen ((t (:foreground "#baba36")))) (theme-face org-level-5 ample-zen ((t (:foreground "#9b55c3")))) (theme-face org-level-6 ample-zen ((t (:foreground "#6a9550")))) (theme-face org-level-7 ample-zen ((t (:foreground "#ff6642")))) (theme-face org-level-8 ample-zen ((t (:foreground "#4C7073")))) (theme-face org-link ample-zen ((t (:foreground "#baba36" :underline t)))) (theme-face org-scheduled ample-zen ((t (:foreground "#6a7550")))) (theme-face org-scheduled-previously ample-zen ((t (:foreground "#ff6642")))) (theme-face org-scheduled-today ample-zen ((t (:foreground "#6380b3")))) (theme-face org-sexp-date ample-zen ((t (:foreground "#6380b3" :underline t)))) (theme-face org-special-keyword ample-zen ((t (:foreground "#c9c9c9" :weight normal)))) (theme-face org-table ample-zen ((t (:foreground "#6a9550")))) (theme-face org-tag ample-zen ((t (:bold t :weight bold)))) (theme-face org-time-grid ample-zen ((t (:foreground "#fb8512")))) (theme-face org-todo ample-zen ((t (:bold t :foreground "#CC5542" :weight bold)))) (theme-face org-upcoming-deadline ample-zen ((t (:inherit font-lock-keyword-face)))) (theme-face org-warning ample-zen ((t (:bold t :foreground "#CC5542" :weight bold :underline nil)))) (theme-face org-column ample-zen ((t (:background "#3b3b3b")))) (theme-face org-column-title ample-zen ((t (:background "#3b3b3b" :underline t :weight bold)))) (theme-face outline-1 ample-zen ((t (:foreground "#fb8512")))) (theme-face outline-2 ample-zen ((t (:foreground "#6a7550")))) (theme-face outline-3 ample-zen ((t (:foreground "#528fd1")))) (theme-face outline-4 ample-zen ((t (:foreground "#baba36")))) (theme-face outline-5 ample-zen ((t (:foreground "#9b55c3")))) (theme-face outline-6 ample-zen ((t (:foreground "#6a9550")))) (theme-face outline-7 ample-zen ((t (:foreground "#ff6642")))) (theme-face outline-8 ample-zen ((t (:foreground "#4C7073")))))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn terminal_whitespace_and_parenthesis_specs_preserve_editor_feedback_colors() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((setting
         (lambda (face)
           (copy-tree
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             (get 'ample-zen 'theme-settings))))))
    (mapcar
     setting
     '(show-paren-match show-paren-mismatch
       term-color-black term-color-red term-color-green
       term-color-yellow term-color-blue term-color-magenta
       term-color-cyan term-color-white
       term-default-fg-color term-default-bg-color
       whitespace-space whitespace-hspace whitespace-tab
       whitespace-newline whitespace-trailing whitespace-line
       whitespace-space-before-tab whitespace-indentation
       whitespace-empty whitespace-space-after-tab))))"##;
    let expect = expect![[
        r##"OK ((theme-face show-paren-match ample-zen ((t (:foreground "#528fd1" :background "#212121" :weight bold)))) (theme-face show-paren-mismatch ample-zen ((t (:foreground "#ff5542" :background "#212121" :weight bold)))) (theme-face term-color-black ample-zen ((t (:foreground "#212121" :background "#3b3b3b")))) (theme-face term-color-red ample-zen ((t (:foreground "#ee5542" :background "#ff6642")))) (theme-face term-color-green ample-zen ((t (:foreground "#6aaf50" :background "#6a9550")))) (theme-face term-color-yellow ample-zen ((t (:foreground "#fb8512" :background "#7d7c61")))) (theme-face term-color-blue ample-zen ((t (:foreground "#528fd1" :background "#4C7073")))) (theme-face term-color-magenta ample-zen ((t (:foreground "#DC8CC3" :background "#CC5542")))) (theme-face term-color-cyan ample-zen ((t (:foreground "#9b55c3" :background "#5180b3")))) (theme-face term-color-white ample-zen ((t (:foreground "#bdbdb3" :background "#c9c9c9")))) (theme-face term-default-fg-color ample-zen ((t (:inherit term-color-white)))) (theme-face term-default-bg-color ample-zen ((t (:inherit term-color-black)))) (theme-face whitespace-space ample-zen ((t (:background "#141414" :foreground "#141414")))) (theme-face whitespace-hspace ample-zen ((t (:background "#141414" :foreground "#141414")))) (theme-face whitespace-tab ample-zen ((t (:background "#dd5542")))) (theme-face whitespace-newline ample-zen ((t (:foreground "#141414")))) (theme-face whitespace-trailing ample-zen ((t (:background "#CC5542")))) (theme-face whitespace-line ample-zen ((t (:background "#212121" :foreground "#DC8CC3")))) (theme-face whitespace-space-before-tab ample-zen ((t (:background "#fb8512" :foreground "#fb8512")))) (theme-face whitespace-indentation ample-zen ((t (:background "#7d7c61" :foreground "#CC5542")))) (theme-face whitespace-empty ample-zen ((t (:background "#7d7c61")))) (theme-face whitespace-space-after-tab ample-zen ((t (:background "#7d7c61" :foreground "#CC5542")))))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn ecosystem_specs_cover_completion_version_control_chat_mail_and_web_workflows() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((setting
         (lambda (face)
           (copy-tree
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             (get 'ample-zen 'theme-settings))))))
    (mapcar
     setting
     '(ac-candidate-face ac-selection-face popup-tip-face
       flx-highlight-face ido-first-match ido-only-match
       git-gutter:added git-gutter:deleted git-gutter:modified
       magit-section-title magit-branch magit-item-highlight
       powerline-active1 powerline-active2 powerline-inactive1
       message-header-name message-header-to
       message-header-from message-header-subject
       erc-current-nick-face erc-input-face erc-prompt-face
       rcirc-my-nick rcirc-other-nick rcirc-prompt
       mu4e-cited-1-face mu4e-cited-6-face mu4e-trashed-face
       web-mode-html-tag-face web-mode-html-attr-name-face
       web-mode-html-attr-value-face web-mode-keyword-face
       web-mode-server-background-face
       web-mode-whitespaces-face))))"##;
    let expect = expect![[
        r##"OK ((theme-face ac-candidate-face ample-zen ((t (:background "#000000" :foreground "#4c4c4c")))) (theme-face ac-selection-face ample-zen ((t (:background "#4C7073" :foreground "#bdbdb3")))) (theme-face popup-tip-face ample-zen ((t (:background "#cc8512" :foreground "#000000")))) (theme-face flx-highlight-face ample-zen ((t (:foreground "#6a9550" :weight bold)))) (theme-face ido-first-match ample-zen ((t (:foreground "#7d7c61" :weight bold)))) (theme-face ido-only-match ample-zen ((t (:foreground "#fb8512" :weight bold)))) (theme-face git-gutter:added ample-zen ((t (:foreground "#6aaf50" :weight bold)))) (theme-face git-gutter:deleted ample-zen ((t (:foreground "#CC5542" :weight bold)))) (theme-face git-gutter:modified ample-zen ((t (:foreground "#baba36" :weight bold)))) (theme-face magit-section-title ample-zen ((t (:foreground "#7d7c61" :weight bold)))) (theme-face magit-branch ample-zen ((t (:foreground "#fb8512" :weight bold)))) (theme-face magit-item-highlight ample-zen ((t (:background "#141414" :bold nil)))) (theme-face powerline-active1 ample-zen ((t (:background "#2e2e2e" :inherit mode-line)))) (theme-face powerline-active2 ample-zen ((t (:background "#0a0a0a" :inherit mode-line)))) (theme-face powerline-inactive1 ample-zen ((t (:background "#141414" :inherit mode-line-inactive)))) (theme-face message-header-name ample-zen ((t (:foreground "#6aa350")))) (theme-face message-header-to ample-zen ((t (:foreground "#7d7c61" :weight bold)))) (theme-face message-header-from ample-zen ((t (:foreground "#7d7c61" :weight bold)))) (theme-face message-header-subject ample-zen ((t (:foreground "#fb8512" :weight bold)))) (theme-face erc-current-nick-face ample-zen ((t (:foreground "#5180b3" :weight bold)))) (theme-face erc-input-face ample-zen ((t (:foreground "#7d7c61")))) (theme-face erc-prompt-face ample-zen ((t (:foreground "#fb8512" :background "#212121" :weight bold)))) (theme-face rcirc-my-nick ample-zen ((t (:foreground "#5180b3")))) (theme-face rcirc-other-nick ample-zen ((t (:foreground "#fb8512")))) (theme-face rcirc-prompt ample-zen ((t (:foreground "#7d7c61" :bold t)))) (theme-face mu4e-cited-1-face ample-zen ((t (:foreground "#5180b3" :slant italic)))) (theme-face mu4e-cited-6-face ample-zen ((t (:foreground "#6abd50" :slant italic)))) (theme-face mu4e-trashed-face ample-zen ((t (:foreground "#000000" :strike-through t)))) (theme-face web-mode-html-tag-face ample-zen ((t (:foreground "#9b55c3")))) (theme-face web-mode-html-attr-name-face ample-zen ((t (:foreground "#fb8512")))) (theme-face web-mode-html-attr-value-face ample-zen ((t (:inherit font-lock-string-face)))) (theme-face web-mode-keyword-face ample-zen ((t (:inherit font-lock-keyword-face)))) (theme-face web-mode-server-background-face ample-zen ((t (:background "#212121")))) (theme-face web-mode-whitespaces-face ample-zen ((t (:background "#CC5542")))))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn inheritance_specs_preserve_alias_chains_for_optional_package_faces() {
    let elisp_form = r##"(progn
  (require 'cl-lib)
  (let ((setting
         (lambda (face)
           (copy-tree
            (cl-find-if
             (lambda (entry)
               (and (eq (car entry) 'theme-face)
                    (eq (cadr entry) face)))
             (get 'ample-zen 'theme-settings))))))
    (mapcar
     setting
     '(c-annotation-face
       font-latex-bold-face font-latex-warning-face
       diff-refine-added diff-refine-change diff-refine-removed
       eshell-ls-backup eshell-ls-clutter eshell-ls-missing
       eshell-ls-product
       erc-action-face erc-direct-msg-face erc-error-face
       message-cited-text message-separator
       org-agenda-structure org-upcoming-deadline
       p4-depot-added-face p4-depot-branch-op-face
       p4-depot-deleted-face p4-diff-file-face
       powerline-active1 powerline-inactive1
       term-default-fg-color term-default-bg-color
       w3m-history-current-url
       web-mode-html-attr-value-face web-mode-keyword-face
       web-mode-server-comment-face
       web-mode-server-string-face))))"##;
    let expect = expect![[
        r##"OK ((theme-face c-annotation-face ample-zen ((t (:inherit font-lock-constant-face)))) (theme-face font-latex-bold-face ample-zen ((t (:inherit bold)))) (theme-face font-latex-warning-face ample-zen ((t (:inherit font-lock-warning)))) (theme-face diff-refine-added ample-zen ((t :inherit diff-added :weight bold))) (theme-face diff-refine-change ample-zen ((t :inherit diff-changed :weight bold))) (theme-face diff-refine-removed ample-zen ((t :inherit diff-removed :weight bold))) (theme-face eshell-ls-backup ample-zen ((t (:inherit font-lock-comment)))) (theme-face eshell-ls-clutter ample-zen ((t (:inherit font-lock-comment)))) (theme-face eshell-ls-missing ample-zen ((t (:inherit font-lock-warning)))) (theme-face eshell-ls-product ample-zen ((t (:inherit font-lock-doc)))) (theme-face erc-action-face ample-zen ((t (:inherit erc-default-face)))) (theme-face erc-direct-msg-face ample-zen ((t (:inherit erc-default)))) (theme-face erc-error-face ample-zen ((t (:inherit font-lock-warning)))) (theme-face message-cited-text ample-zen ((t (:inherit font-lock-comment)))) (theme-face message-separator ample-zen ((t (:inherit font-lock-comment)))) (theme-face org-agenda-structure ample-zen ((t (:inherit font-lock-comment-face)))) (theme-face org-upcoming-deadline ample-zen ((t (:inherit font-lock-keyword-face)))) (theme-face p4-depot-added-face ample-zen ((t :inherit diff-added))) (theme-face p4-depot-branch-op-face ample-zen ((t :inherit diff-changed))) (theme-face p4-depot-deleted-face ample-zen ((t :inherit diff-removed))) (theme-face p4-diff-file-face ample-zen ((t :inherit diff-file-header))) (theme-face powerline-active1 ample-zen ((t (:background "#2e2e2e" :inherit mode-line)))) (theme-face powerline-inactive1 ample-zen ((t (:background "#141414" :inherit mode-line-inactive)))) (theme-face term-default-fg-color ample-zen ((t (:inherit term-color-white)))) (theme-face term-default-bg-color ample-zen ((t (:inherit term-color-black)))) (theme-face w3m-history-current-url ample-zen ((t (:inherit match)))) (theme-face web-mode-html-attr-value-face ample-zen ((t (:inherit font-lock-string-face)))) (theme-face web-mode-keyword-face ample-zen ((t (:inherit font-lock-keyword-face)))) (theme-face web-mode-server-comment-face ample-zen ((t (:inherit web-mode-comment-face)))) (theme-face web-mode-server-string-face ample-zen ((t (:inherit web-mode-string-face)))))"##
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn every_registered_face_setting_has_theme_owner_symbol_and_nonempty_display_specs() {
    let elisp_form = r##"(let ((face-settings
       (delq nil
             (mapcar
              (lambda (setting)
                (and (eq (car setting) 'theme-face)
                     setting))
              (get 'ample-zen 'theme-settings)))))
  (list
   (length face-settings)
   (delq nil
         (mapcar
          (lambda (setting)
            (and
             (or (not (symbolp (cadr setting)))
                 (not (eq (caddr setting) 'ample-zen))
                 (null (cadddr setting)))
             setting))
          face-settings))
   (mapcar
    (lambda (setting)
      (list
       (cadr setting)
       (length setting)
       (length (cadddr setting))))
    (seq-take face-settings 12))))"##;
    let expect = expect![
        "OK (421 nil ((which-func 4 1) (whitespace-space-after-tab 4 1) (whitespace-empty 4 1) (whitespace-indentation 4 1) (whitespace-space-before-tab 4 1) (whitespace-line 4 1) (whitespace-trailing 4 1) (whitespace-newline 4 1) (whitespace-tab 4 1) (whitespace-hspace 4 1) (whitespace-space 4 1) (web-mode-whitespaces-face 4 1)))"
    ];
    assert_ample_zen_theme_parity(elisp_form, expect);
}

#[test]
fn face_surface_spans_every_documented_builtin_and_third_party_family() {
    let elisp_form = r##"(let* ((names
        (mapcar
         #'cadr
         (delq nil
               (mapcar
                (lambda (setting)
                  (and (eq (car setting) 'theme-face)
                       setting))
                (get 'ample-zen 'theme-settings)))))
       (families
        '("compilation-" "grep-" "font-lock-" "diff-"
          "ediff-" "eshell-" "flycheck-" "flymake-"
          "flyspell-" "erc-" "git-gutter" "gnus-"
          "jabber-" "magit-" "message-" "mu4e-"
          "org-" "outline-" "p4-" "powerline-"
          "rainbow-delimiters-" "rcirc-" "rpm-"
          "rst-" "term-" "w3m-" "web-mode-"
          "whitespace-")))
  (mapcar
   (lambda (prefix)
     (list
      prefix
      (length
       (delq nil
             (mapcar
              (lambda (name)
                (and
                 (string-prefix-p
                  prefix (symbol-name name))
                 name))
              names)))))
   families))"##;
    let expect = expect![[
        r#"OK (("compilation-" 14) ("grep-" 4) ("font-lock-" 15) ("diff-" 8) ("ediff-" 16) ("eshell-" 11) ("flycheck-" 4) ("flymake-" 3) ("flyspell-" 2) ("erc-" 19) ("git-gutter" 7) ("gnus-" 60) ("jabber-" 11) ("magit-" 5) ("message-" 11) ("mu4e-" 9) ("org-" 32) ("outline-" 8) ("p4-" 9) ("powerline-" 4) ("rainbow-delimiters-" 12) ("rcirc-" 14) ("rpm-" 9) ("rst-" 6) ("term-" 10) ("w3m-" 8) ("web-mode-" 24) ("whitespace-" 10))"#
    ]];
    assert_ample_zen_theme_parity(elisp_form, expect);
}
