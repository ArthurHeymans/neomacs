use expect_test::expect;

use super::{assert_aider_autoload_parity, assert_aider_parity};

#[test]
fn aider_exact_package_metadata_dependencies_and_revision_match() {
    let elisp_form = r##"(let ((descriptor (cadr (assq 'aider package-alist))))
         (list
          (package-desc-name descriptor)
          (package-version-join (package-desc-version descriptor))
          (package-desc-summary descriptor)
          (package-desc-kind descriptor)
          (package-desc-reqs descriptor)
          (package-desc-extras descriptor)
          (featurep 'aider)
          (get 'aider 'group-documentation)))"##;
    let expect = expect![[
        r#"OK (aider "20251201.133" "AI assisted programming with Aider and LLM." nil ((emacs (26 1)) (transient (0 9 0)) (magit (2 1 0)) (markdown-mode (2 5)) (s (1 13 0))) ((:maintainers ("Kang Tu" . "tninja@gmail.com")) (:authors ("Kang Tu" . "tninja@gmail.com")) (:keywords "ai" "gpt" "sonnet" "llm" "aider" "gemini-pro" "deepseek" "ai-assisted-coding") (:revdesc . "5c2c093f20e1") (:commit . "5c2c093f20e14ca5f47ebbb35d4e198550f9fffc") (:url . "https://github.com/tninja/aider.el")) t "Customization group for the Aider package.")"#
    ]];
    assert_aider_parity(elisp_form, expect);
}

#[test]
fn aider_installed_payload_inventory_and_source_hashes_match() {
    let elisp_form = r##"(let* ((descriptor (cadr (assq 'aider package-alist)))
                (directory (package-desc-dir descriptor))
                (sources
                 (sort
                  (directory-files directory nil "\\`aider.*\\.el\\'")
                  #'string-lessp)))
         (list
          sources
          (mapcar
           (lambda (file)
             (list file
                   (file-attribute-size
                    (file-attributes (expand-file-name file directory)))
                   (secure-hash 'sha256 (expand-file-name file directory))))
           sources)
          (directory-files directory nil "\\`test")
          (directory-files directory nil "\\.tar\\'")))"##;
    let expect = expect![[
        r#"OK (("aider-agile.el" "aider-autoloads.el" "aider-bootstrap.el" "aider-code-change.el" "aider-code-read.el" "aider-comint-markdown.el" "aider-core.el" "aider-discussion.el" "aider-doom.el" "aider-file.el" "aider-git.el" "aider-helm.el" "aider-highlight-changes.el" "aider-legacy-code.el" "aider-pkg.el" "aider-prompt-mode.el" "aider-software-planning.el" "aider-utils.el" "aider.el") (("aider-agile.el" 18900 "ddc0fb6f5bf983d1e8a976de1efd7eebdf87332c5205df48009b78f7a11d20c1") ("aider-autoloads.el" 16475 "bd535d5a5a62505273a6406ffaa126adc205203eb8a5a91c6d75fb5957e8c544") ("aider-bootstrap.el" 24404 "02afc4b8465316f69975b799ecc4466b8b7de773b9245cc69c9639280d9159bc") ("aider-code-change.el" 28433 "8c6bb21818e617125650c2c7a216d0f95d54baf9c1f3ad00044a55cfe39b9375") ("aider-code-read.el" 16141 "db23da4ba968ae6914981a17228929df465a72869fb89b6283d2284da693c616") ("aider-comint-markdown.el" 3373 "083720b342f0a6a3d1688def6995f096e05def1cb86ff6e26fe2e4ab143fd2f9") ("aider-core.el" 21237 "5d44beb7517309d5180d68823ad2a0a45ca2f3829c790e948ffe50ea921f84e6") ("aider-discussion.el" 8172 "83020b3e290ba66cf86d1e6797e3fe789051c209c1a44089911a7cf14db17364") ("aider-doom.el" 4239 "37c866f239ae095a4e494e7d8ab927097a38502e815a112ec50fd48ebc1b24cb") ("aider-file.el" 21549 "a002688e6abc6234fdbc3b09276a26a3d72def049aea579defc8bb75b37b49ec") ("aider-git.el" 21817 "ecfb3415ef5ae6d7ebaa416d606cf2f823000b6bc2b78b99c1621f54b279be30") ("aider-helm.el" 4399 "5d1606ecd2b2ba83db0007ea6a6c4be131dcd4414df0a93893f1c10999701023") ("aider-highlight-changes.el" 8100 "57896cc4dd63dc43469fd906153e6d1c6e0d78a350f0e60f47fdb98c5c846777") ("aider-legacy-code.el" 33893 "02302c63309327cd8a8c66c7798925ad64556b6cd1a1886334d91feabf0a1f12") ("aider-pkg.el" 594 "d25d548c9b613c4e9f70d33a1a2f87150fd7c71e2251a8e28a538278e2e5b685") ("aider-prompt-mode.el" 13954 "a1e0fa4cdcd17c29294f6899954827f1cc0e4083909bdf5c0443883fbc537c68") ("aider-software-planning.el" 8393 "7eccd8e91458ed7809f5d403241bd0803da8148d7fb58db25d5a4f028f31c085") ("aider-utils.el" 9998 "c0c06d4e29462c0bbf025219f1ea11918a9d46c5faf95a3a6ba6cfa8bc16f115") ("aider.el" 11122 "45bbef23fc918b14d68984c6ecb189c456b117ba7741cc3376ca8ebba8cf5d16")) nil nil)"#
    ]];
    assert_aider_parity(elisp_form, expect);
}

#[test]
fn aider_custom_defaults_metadata_and_command_registry_match() {
    let elisp_form = r##"(list
         (mapcar
          (lambda (symbol)
            (list symbol
                  (symbol-value symbol)
                  (get symbol 'custom-type)
                  (get symbol 'custom-group)
                  (local-variable-p symbol)))
          '(aider-program
            aider-args
            aider-auto-trigger-prompt
            aider-use-branch-specific-buffers
            aider-confirm-on-main-branch
            aider-auto-trigger-command-completion
            aider-auto-trigger-file-path-insertion
            aider-enable-markdown-highlighting
            aider-todo-keyword-pair
            aider-prompt-file-name
            aider-popular-models))
         aider--command-list)"##;
    let expect = expect![[
        r#"OK (((aider-program "aider" string nil nil) (aider-args nil (repeat string) nil nil) (aider-auto-trigger-prompt nil boolean nil nil) (aider-use-branch-specific-buffers nil boolean nil nil) (aider-confirm-on-main-branch t boolean nil nil) (aider-auto-trigger-command-completion t boolean nil nil) (aider-auto-trigger-file-path-insertion t boolean nil nil) (aider-enable-markdown-highlighting t boolean nil nil) (aider-todo-keyword-pair ("TODO" . "comment line START with string: TODO:") (cons string string) nil nil) (aider-prompt-file-name ".aider.prompt.org" string nil nil) (aider-popular-models ("sonnet" "o4-mini" "deepseek/deepseek-reasoner") (repeat string) nil nil)) ("/add" "/architect" "/ask" "/code" "/reset" "/undo" "/lint" "/read-only" "/drop" "/copy" "/copy-context" "/clear" "/commit" "/exit" "/quit" "/paste" "/help" "/chat-mode" "/diff" "/editor" "/git" "/load" "/ls" "/map" "/map-refresh" "/think-tokens" "/tokens" "/model" "/editor-model" "/weak-model" "/models" "/reasoning-effort" "/multiline-mode" "/report" "/run" "/save" "/settings" "/test" "/voice" "/web"))"#
    ]];
    assert_aider_parity(elisp_form, expect);
}

#[test]
fn aider_complete_callable_surface_arglists_and_command_status_match() {
    let elisp_form = r##"(let (surface)
         (mapatoms
          (lambda (symbol)
            (when (and (string-prefix-p "aider-" (symbol-name symbol))
                       (fboundp symbol))
              (push
               (list symbol
                     (copy-tree
                      (help-function-arglist symbol t))
                     (commandp symbol)
                     (macrop symbol)
                     (autoloadp (symbol-function symbol)))
               surface))))
         (sort surface
               (lambda (left right)
                 (string-lessp (symbol-name (car left))
                               (symbol-name (car right))))))"##;
    let expect = expect![[
        r#"OK ((aider--analyze-class nil nil nil nil) (aider--analyze-code-unit nil nil nil nil) (aider--analyze-code-unit-region nil nil nil nil) (aider--analyze-dependencies nil nil nil nil) (aider--analyze-file nil nil nil nil) (aider--analyze-for-maintainability nil nil nil nil) (aider--analyze-for-performance nil nil nil nil) (aider--analyze-for-security nil nil nil nil) (aider--analyze-module nil nil nil nil) (aider--analyze-program-structure nil nil nil nil) (aider--apply-markdown-highlighting nil nil nil nil) (aider--ask-about-function (function-name) nil nil nil) (aider--ask-about-region (function-name) nil nil nil) (aider--ask-with-function-choice (function-name) nil nil nil) (aider--batch-add-dired-marked-files-with-command (command-prefix) nil nil nil) (aider--bootstrap-basic-file nil t nil nil) (aider--bootstrap-class-module-outline nil t nil nil) (aider--bootstrap-cli-app nil t nil nil) (aider--bootstrap-contextual-note nil t nil nil) (aider--bootstrap-data-model nil t nil nil) (aider--bootstrap-docker-config nil t nil nil) (aider--bootstrap-general-plan nil t nil nil) (aider--bootstrap-org-slides nil t nil nil) (aider--bootstrap-project-structure nil t nil nil) (aider--bootstrap-readme nil t nil nil) (aider--buffer-name-for-git-repo (git-repo-path-true) nil nil nil) (aider--build-log-prompt (repo-name analysis-instructions) nil nil nil) (aider--build-region-question-context (prompt) nil nil nil) (aider--choose-flycheck-scope nil nil nil nil) (aider--clear-diff-overlays nil nil nil nil) (aider--comint-send-string-syntax-highlight (buffer text) nil nil nil) (aider--create-aider-buffer (buffer-name current-args) nil nil nil) (aider--default-log-analysis-instructions (keyword) nil nil nil) (aider--drop-file-under-cursor nil nil nil nil) (aider--ensure-git-log (git-root repo-name keyword) nil nil nil) (aider--ensure-highlight-timer nil nil nil nil) (aider--extract-comment-content (comment-text) nil nil nil) (aider--extract-filename-from-command (command-str) nil nil nil) (aider--extract-search-replace-blocks nil nil nil nil) (aider--file-mentions-basename (file-path basename) nil nil nil) (aider--file-path-under-cursor-is-file nil nil nil nil) (aider--filter-files-by-content-regex (files content-regex) nil nil nil) (aider--filter-test-files (files include-tests &optional test-file-regex) nil nil nil) (aider--find-conflict-at-point (point) nil nil nil) (aider--find-file-dependencies (file-path search-root) nil nil nil) (aider--find-file-dependents (file-path search-root) nil nil nil) (aider--find-files-by-patterns (search-root patterns) nil nil nil) (aider--find-search-replace-block-at-point (point) nil nil nil) (aider--format-file-path (file-path) nil nil nil) (aider--generate-branch-or-commit-diff (diff-params diff-file) nil nil nil) (aider--generate-history-file-name nil nil nil nil) (aider--generate-staged-diff (diff-file) nil nil nil) (aider--get-candidate-list nil nil nil nil) (aider--get-class-at-point nil nil nil nil) (aider--get-comment-instruction (comment-content function-name) nil nil nil) (aider--get-current-git-branch (repo-root-path) nil nil nil) (aider--get-current-input nil nil nil nil) (aider--get-diff-type-choice nil nil nil nil) (aider--get-file-path (file-path) nil nil nil) (aider--get-files-in-directory (directory suffixes) nil nil nil) (aider--get-full-branch-ref (branch) nil nil nil) (aider--get-full-expanded-file-path-at-point nil nil nil nil) (aider--get-function-name-for-comment nil nil nil nil) (aider--get-git-repo-root nil nil nil nil) (aider--get-language-from-extension (filename) nil nil nil) (aider--get-question-candidates nil nil nil nil) (aider--get-refactoring-context nil nil nil nil) (aider--get-refactoring-techniques (region-active) nil nil nil) (aider--get-relevant-directory-for-history nil nil nil nil) (aider--get-source-file-patterns (file-ext) nil nil nil) (aider--get-standard-instruction (region-active function-name) nil nil nil) (aider--handle-ask-llm-suggestion (context tdd-mode) nil nil nil) (aider--handle-base-vs-head-diff-generation (git-root) nil nil nil) (aider--handle-branch-range-diff-generation (git-root) nil nil nil) (aider--handle-comment-requirement (line-text function-name) nil nil nil) (aider--handle-commit-diff-generation (git-root) nil nil nil) (aider--handle-commit-range-diff-generation (git-root) nil nil nil) (aider--handle-multi-line-comment (region-text function-name) nil nil nil) (aider--handle-region-or-function (region-active function-name) nil nil nil) (aider--handle-specific-refactoring (selected-technique all-techniques context tdd-mode) nil nil nil) (aider--handle-staged-diff-generation (git-root) nil nil nil) (aider--handle-standard-change (region-active region-text function-name) nil nil nil) (aider--handle-subtree-command (subdir-rel-path) nil nil nil) (aider--identify-patterns nil nil nil nil) (aider--idle-timer-change-block-highlight nil nil nil nil) (aider--infix-switch-to-buffer-other-frame nil t nil nil) (aider--is-all-comment-lines (text) nil nil nil) (aider--is-comment-line (line) nil nil nil) (aider--is-default-directory-git-root nil nil nil nil) (aider--legacy-code-adapt-parameter nil nil nil nil) (aider--legacy-code-analyze-change-points nil nil nil nil) (aider--legacy-code-break-dependencies nil nil nil nil) (aider--legacy-code-characterization-test nil nil nil nil) (aider--legacy-code-encapsulate-global-references nil t nil nil) (aider--legacy-code-extract-and-override-call nil nil nil nil) (aider--legacy-code-extract-and-override-getter nil nil nil nil) (aider--legacy-code-extract-and-override-setter nil t nil nil) (aider--legacy-code-extract-override-factory-method nil t nil nil) (aider--legacy-code-identify-seams nil t nil nil) (aider--legacy-code-introduce-instance-delegator nil nil nil nil) (aider--legacy-code-introduce-null-object nil t nil nil) (aider--legacy-code-introduce-static-setter nil t nil nil) (aider--legacy-code-replace-conditional-polymorphism nil t nil nil) (aider--legacy-code-replace-function-with-function-pointer nil nil nil nil) (aider--legacy-code-sensing-variable nil nil nil nil) (aider--legacy-code-sprout-class nil nil nil nil) (aider--legacy-code-sprout-method nil nil nil nil) (aider--legacy-code-wrap-class nil nil nil nil) (aider--legacy-code-wrap-method nil nil nil nil) (aider--line-has-import-keyword-p (line) nil nil nil) (aider--magit-generate-feature-branch-diff-file nil t nil nil) (aider--maybe-prompt-and-set-reasoning-effort (command model) nil nil nil) (aider--maybe-prompt-subtree-only-for-special-modes (current-args) nil nil nil) (aider--open-diff-file (diff-file) nil nil nil) (aider--parse-aider-cli-history (file-path) nil nil nil) (aider--plot-module-architecture nil nil nil nil) (aider--prepare-aider-args (edit-args subtree-only) nil nil nil) (aider--process-context-files (current-file dependencies dependents) nil nil nil) (aider--process-message-if-multi-line (str) nil nil nil) (aider--process-refactoring-parameters (selected-technique technique-description context) nil nil nil) (aider--region-location-info (start end) nil nil nil) (aider--replace-input (text) nil nil nil) (aider--resolve-diff-branches (type input-base-branch input-feature-branch &optional branch-scope) nil nil nil) (aider--run-search (program args) nil nil nil) (aider--safe-get-start-fence-regexp (origfn &rest args) nil nil nil) (aider--safe-maybe-funcall-regexp (origfn object &optional arg) nil nil nil) (aider--safe-syntax-propertize-fenced-block-constructs (origfn start end) nil nil nil) (aider--scan-buffer-for-basename (basename) nil nil nil) (aider--search-files-containing-pattern (search-root pattern file-patterns) nil nil nil) (aider--send-command (command &optional switch-to-buffer log) nil nil nil) (aider--send-trimmed-line (line) nil nil nil) (aider--setup-snippets nil nil nil nil) (aider--smerge-refine-conflict (conflict-data) nil nil nil) (aider--switch-to-buffer-type (&rest slots) nil nil nil) (aider--switch-to-buffer-type--eieio-childp (obj) nil nil nil) (aider--switch-to-buffer-type-child-p (obj) nil nil nil) (aider--switch-to-buffer-type-p (obj) nil nil nil) (aider--tdd-green-stage (function-name) nil nil nil) (aider--tdd-red-stage (function-name) nil nil nil) (aider--validate-aider-buffer nil nil nil nil) (aider--validate-buffer-file nil nil nil nil) (aider--validate-git-repository nil nil nil nil) (aider--verify-branches (base-branch feature-branch) nil nil nil) (aider--write-unit-test-source-file (function-name) nil nil nil) (aider--write-unit-test-test-file (function-name) nil nil nil) (aider-action-current-file (command-prefix) nil nil nil) (aider-add-current-file nil t nil nil) (aider-add-current-file-or-dired-marked-files (&optional read-only) t nil nil) (aider-add-current-file-or-dired-marked-files-read-only nil t nil nil) (aider-add-files-in-current-window nil t nil nil) (aider-add-module (&optional read-only directory suffix-input content-regex) t nil nil) (aider-architect-discussion nil t nil nil) (aider-ask-question nil t nil nil) (aider-batch-add-dired-marked-files nil t nil nil) (aider-batch-add-dired-marked-files-read-only nil t nil nil) (aider-bootstrap nil t nil nil) (aider-buffer-name nil nil nil nil) (aider-change-model (leaderboards) t nil nil) (aider-clear-buffer nil t nil nil) (aider-code-change nil t nil nil) (aider-code-read nil t nil nil) (aider-comint-mode nil t nil nil) (aider-compare-search-replace-blocks nil t nil nil) (aider-copy-to-clipboard nil t nil nil) (aider-core--auto-trigger-command-completion nil nil nil nil) (aider-core--auto-trigger-file-path-insertion nil nil nil nil) (aider-core--auto-trigger-insert-prompt nil nil nil nil) (aider-core--command-completion nil nil nil nil) (aider-core--parse-added-file-list nil t nil nil) (aider-core-insert-prompt nil t nil nil) (aider-current-file-command-and-switch (prefix command) nil nil nil) (aider-current-file-read-only nil t nil nil) (aider-debug-exception nil t nil nil) (aider-doom-enable "[Arg list not available until function definition is loaded.]" t nil t) (aider-drop-current-file nil t nil nil) (aider-exit nil t nil nil) (aider-expand-context-current-file nil t nil nil) (aider-expand-context-given-file (file-path &optional include-tests) nil nil nil) (aider-flycheck--format-error-list (errors file-path-for-error-reporting) nil nil nil) (aider-flycheck--get-errors-in-scope (start end) nil nil nil) (aider-flycheck-fix-errors-in-scope nil t nil nil) (aider-function-or-region-change nil t nil nil) (aider-general-question nil t nil nil) (aider-go-ahead nil t nil nil) (aider-help (&optional homepage) t nil nil) (aider-history-next nil t nil nil) (aider-history-prev nil t nil nil) (aider-implement-todo nil t nil nil) (aider-input-sender (proc string) nil nil nil) (aider-legacy-code nil t nil nil) (aider-magit-blame-analyze nil t nil nil) (aider-magit-blame-or-log-analyze (&optional arg) t nil nil) (aider-magit-log-analyze nil t nil nil) (aider-magit-setup-transients nil t nil nil) (aider-magit-show-last-commit-or-log (&optional log) t nil nil) (aider-open-aider-home nil t nil nil) (aider-open-history nil t nil nil) (aider-open-prompt-file nil t nil nil) (aider-plain-read-string (prompt &optional initial-input candidate-list) nil nil nil) (aider-prompt-cycle-file-command nil t nil nil) (aider-prompt-insert-add-file-path nil t nil nil) (aider-prompt-insert-drop-file-path nil t nil nil) (aider-prompt-mode nil t nil nil) (aider-prompt-mode-setup-font-lock nil nil nil nil) (aider-pull-or-review-diff-file nil t nil nil) (aider-read-string (prompt &optional initial-input candidate-list) nil nil nil) (aider-refactor-book-method (&optional tdd-mode) t nil nil) (aider-region-change-generate-command (region-text function-name user-command) nil nil nil) (aider-reset (&optional clear) t nil nil) (aider-run-aider (&optional edit-args subtree-only) t nil nil) (aider-run-current-file nil t nil nil) (aider-send-block-by-line nil t nil nil) (aider-send-block-or-region nil t nil nil) (aider-send-line-or-region (&optional arg) t nil nil) (aider-send-region-by-line nil t nil nil) (aider-start-software-planning nil t nil nil) (aider-switch-to-buffer nil t nil nil) (aider-tdd-cycle nil t nil nil) (aider-transient-menu nil t nil nil) (aider-transient-menu-1col nil t nil nil) (aider-transient-menu-2cols nil t nil nil) (aider-undo-last-change nil t nil nil) (aider-write-unit-test nil t nil nil))"#
    ]];
    assert_aider_parity(elisp_form, expect);
}

#[test]
fn aider_mode_maps_derived_modes_hooks_and_alias_contract_match() {
    let elisp_form = r##"(list
         (eq (indirect-function 'aider-read-string)
             (indirect-function 'aider-plain-read-string))
         (get 'aider-comint-mode 'derived-mode-parent)
         (get 'aider-prompt-mode 'derived-mode-parent)
         (mapcar
          (lambda (key)
            (list key
                  (lookup-key aider-comint-mode-map (kbd key))
                  (lookup-key aider-prompt-mode-map (kbd key))))
          '("TAB" "C-c C-f" "C-c C-y" "C-c C-z"
            "C-c C-n" "C-c C-b" "C-c C-c" "M-<up>" "M-<down>"))
         (assq 'aider-prompt-mode auto-mode-alist)
         (and
          (advice-member-p #'aider--safe-maybe-funcall-regexp
                           'markdown-maybe-funcall-regexp)
          t))"##;
    let expect = expect![[
        r#"OK (t comint-mode org-mode (("TAB" aider-core-insert-prompt nil) ("C-c C-f" aider-prompt-insert-add-file-path aider-prompt-insert-add-file-path) ("C-c C-y" aider-go-ahead aider-prompt-cycle-file-command) ("C-c C-z" comint-stop-subjob aider-switch-to-buffer) ("C-c C-n" comint-next-prompt aider-send-line-or-region) ("C-c C-b" nil aider-send-block-by-line) ("C-c C-c" comint-interrupt-subjob aider-send-block-or-region) ("M-<up>" aider-history-prev nil) ("M-<down>" aider-history-next nil)) nil t)"#
    ]];
    assert_aider_parity(elisp_form, expect);
}

#[test]
fn aider_autoload_surface_exposes_public_commands_without_eager_main_load() {
    let elisp_form = r##"(list
         (featurep 'aider)
         (mapcar
          (lambda (symbol)
            (let ((definition (symbol-function symbol)))
              (list symbol
                    (autoloadp definition)
                    (and (autoloadp definition) (nth 1 definition))
                    (commandp symbol))))
          '(aider-run-aider
            aider-transient-menu
            aider-add-current-file
            aider-function-or-region-change
            aider-implement-todo
            aider-ask-question
            aider-code-read
            aider-bootstrap
            aider-start-software-planning
            aider-open-prompt-file))
         (file-name-nondirectory (getenv "NEOMACS_PACKAGE_SOURCE")))"##;
    let expect = expect![[
        r#"OK (nil ((aider-run-aider t "aider-core" t) (aider-transient-menu t "aider" t) (aider-add-current-file t "aider-file" t) (aider-function-or-region-change t "aider-code-change" t) (aider-implement-todo t "aider-code-change" t) (aider-ask-question t "aider-discussion" t) (aider-code-read t "aider-code-read" t) (aider-bootstrap t "aider-bootstrap" t) (aider-start-software-planning t "aider-software-planning" t) (aider-open-prompt-file t "aider-prompt-mode" t)) "aider-autoloads.el")"#
    ]];
    assert_aider_autoload_parity(elisp_form, expect);
}
