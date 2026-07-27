use expect_test::expect;

use super::assert_alchemist_parity;

#[test]
fn alchemist_exact_pin_dependencies_features_defaults_and_archive_source_match() {
    let elisp_form = r##"(let ((descriptor
                           (cadr (assq 'alchemist package-alist))))
                      (list
                       (package-desc-name descriptor)
                       (package-version-join
                        (package-desc-version descriptor))
                       (package-desc-reqs descriptor)
                       (mapcar
                        #'featurep
                        '(alchemist alchemist-company alchemist-compile
                          alchemist-complete alchemist-eval alchemist-execute
                          alchemist-file alchemist-goto alchemist-help
                          alchemist-hex alchemist-hooks alchemist-iex
                          alchemist-info alchemist-interact alchemist-key
                          alchemist-macroexpand alchemist-message alchemist-mix
                          alchemist-phoenix alchemist-project alchemist-refcard
                          alchemist-report alchemist-scope alchemist-server
                          alchemist-test-mode alchemist-utils))
                       (list
                        alchemist-compile-command
                        alchemist-execute-command
                        alchemist-goto-erlang-source-dir
                        alchemist-goto-elixir-source-dir
                        alchemist-help-buffer-name
                        alchemist-hex-api-url
                        alchemist-iex-program-name
                        alchemist-key-command-prefix
                        alchemist-mix-command
                        alchemist-mix-test-task
                        alchemist-mix-test-default-options
                        alchemist-mix-env
                        alchemist-server-env
                        alchemist-test-mode-highlight-tests
                        alchemist-test-display-compilation-output
                        alchemist-test-truncate-lines
                        alchemist-test-status-modeline
                        alchemist-test-ask-about-save)
                       (file-name-nondirectory
                        (getenv "NEOMACS_PACKAGE_SOURCE"))))"##;
    let expect = expect![[
        r#"OK (alchemist "20180312.1304" ((elixir-mode (2 2 5)) (dash (2 11 0)) (emacs (24 4)) (company (0 8 0)) (pkg-info (0 4)) (s (1 11 0))) (t t t t t t t t t t t t t t t t t t t t t t t t t t) ("elixirc" "elixir" "" "" "*alchemist help*" "https://hex.pm/api/packages/" "iex" "\3a" "mix" "test" nil nil "dev" t nil t t t) "alchemist.el")"#
    ]];
    assert_alchemist_parity(elisp_form, expect);
}

#[test]
fn alchemist_complete_callable_surface_names_macros_and_commands_match() {
    let elisp_form = r##"(let* ((source-directory
                               (file-name-directory
                                (file-truename
                                 (getenv "NEOMACS_PACKAGE_SOURCE"))))
                              rows)
                      (mapatoms
                       (lambda (symbol)
                         (when (and
                                (string-prefix-p
                                 "alchemist" (symbol-name symbol))
                                (fboundp symbol)
                                (when-let
                                    ((file
                                      (symbol-file symbol 'defun)))
                                  (string=
                                   source-directory
                                   (file-name-directory
                                    (file-truename file)))))
                           (push
                            (list
                             symbol
                             (condition-case nil
                                 (copy-tree
                                  (help-function-arglist symbol t))
                               (error :unavailable))
                             (macrop symbol)
                             (commandp symbol))
                            rows))))
                      (setq
                       rows
                       (sort
                        rows
                        (lambda (left right)
                          (string-lessp
                           (symbol-name (car left))
                           (symbol-name (car right))))))
                      (list
                       (length rows)
                       (mapcar #'car rows)
                       (mapcar
                        #'car
                        (seq-filter
                         (lambda (row)
                           (nth 2 row))
                         rows))
                       (mapcar
                        #'car
                        (seq-filter
                         (lambda (row)
                           (nth 3 row))
                         rows))))"##;
    let expect = expect![
        "OK (313 (alchemist-company alchemist-company--annotation alchemist-company-build-scope-arg alchemist-company-build-server-arg alchemist-company-doc-buffer-filter alchemist-company-filter alchemist-company-get-prefix alchemist-company-open-definition alchemist-company-serve-candidates-to-callback alchemist-company-show-documentation alchemist-compile alchemist-compile--file alchemist-compile--read-command alchemist-compile-file alchemist-compile-mode alchemist-compile-this-buffer alchemist-complete--add-prefix-to-function alchemist-complete--build-candidates alchemist-complete--build-candidates-from-process-output alchemist-complete--build-help-candidates alchemist-complete--completing-prompt alchemist-complete--concat-prefix-with-functions alchemist-complete--dabbrev-code-candidates alchemist-complete--output-to-list alchemist-elixir-version alchemist-eval--expression alchemist-eval--expression-and-print alchemist-eval--insert alchemist-eval--quote-expression alchemist-eval--quote-expression-and-print alchemist-eval-buffer alchemist-eval-close-popup alchemist-eval-current-line alchemist-eval-filter alchemist-eval-insert-filter alchemist-eval-mode alchemist-eval-print-buffer alchemist-eval-print-current-line alchemist-eval-print-quoted-buffer alchemist-eval-print-quoted-current-line alchemist-eval-print-quoted-region alchemist-eval-print-region alchemist-eval-quoted-buffer alchemist-eval-quoted-current-line alchemist-eval-quoted-filter alchemist-eval-quoted-insert-filter alchemist-eval-quoted-region alchemist-eval-region alchemist-execute alchemist-execute--file alchemist-execute--read-command alchemist-execute-file alchemist-execute-mode alchemist-execute-this-buffer alchemist-file--files-from alchemist-file-find-files alchemist-file-read-dir alchemist-goto--build-elixir-erl-core-file alchemist-goto--build-elixir-ex-core-file alchemist-goto--build-erlang-core-file alchemist-goto--extract-symbol alchemist-goto--extract-symbol-bare alchemist-goto--fetch-symbol-definitions alchemist-goto--fetch-symbols-from-propertize-list alchemist-goto--file-contains-defs-p alchemist-goto--get-symbol-from-position alchemist-goto--get-symbol-from-position-bare alchemist-goto--goto-symbol alchemist-goto--jump-to-elixir-source alchemist-goto--jump-to-erlang-source alchemist-goto--open-definition alchemist-goto--open-file alchemist-goto--search-for-symbols alchemist-goto--symbol-definition-p alchemist-goto-definition-at-point alchemist-goto-elixir-file-p alchemist-goto-erlang-file-p alchemist-goto-filter alchemist-goto-jump-back alchemist-goto-jump-to-next-def-symbol alchemist-goto-jump-to-previous-def-symbol alchemist-goto-list-symbol-definitions alchemist-help alchemist-help--completion-server-arguments alchemist-help--elixir-modules-to-list alchemist-help--prepare-search-expr alchemist-help--search-at-point alchemist-help--search-marked-region alchemist-help--server-arguments alchemist-help-complete-filter-output alchemist-help-display-doc alchemist-help-filter-output alchemist-help-history alchemist-help-lookup-doc alchemist-help-minor-mode alchemist-help-minor-mode-key-binding-summary alchemist-help-module alchemist-help-modules-filter alchemist-help-no-doc-available-p alchemist-help-search-at-point alchemist-help-store-search-in-history alchemist-hex--deps-name-at-point alchemist-hex--display-info-for alchemist-hex--display-releases-for alchemist-hex--fetch-package-info alchemist-hex--fetch-search-packages alchemist-hex-all-dependencies alchemist-hex-info alchemist-hex-info-at-point alchemist-hex-mode alchemist-hex-releases alchemist-hex-releases-at-point alchemist-hex-search alchemist-hooks-compile-on-save alchemist-hooks-test-on-save alchemist-iex--remove-newlines alchemist-iex--send-command alchemist-iex-clear-buffer alchemist-iex-command alchemist-iex-compile-this-buffer alchemist-iex-compile-this-buffer-and-go alchemist-iex-mode alchemist-iex-open-input-ring alchemist-iex-process alchemist-iex-project-run alchemist-iex-reload-module alchemist-iex-run alchemist-iex-send-current-line alchemist-iex-send-current-line-and-go alchemist-iex-send-last-sexp alchemist-iex-send-region alchemist-iex-send-region-and-go alchemist-iex-spot-prompt alchemist-iex-start-process alchemist-info-close-popup alchemist-info-datatype-at-point alchemist-info-datatype-filter alchemist-info-expression-at-point alchemist-info-mode alchemist-info-types-at-point alchemist-interact-create-popup alchemist-interact-insert-as-comment alchemist-macroexpand-close-popup alchemist-macroexpand-current-line alchemist-macroexpand-expand-and-print-request alchemist-macroexpand-expand-once-and-print-request alchemist-macroexpand-expand-once-request alchemist-macroexpand-expand-request alchemist-macroexpand-filter alchemist-macroexpand-insert-filter alchemist-macroexpand-mode alchemist-macroexpand-once-current-line alchemist-macroexpand-once-print-current-line alchemist-macroexpand-once-print-region alchemist-macroexpand-once-region alchemist-macroexpand-print-current-line alchemist-macroexpand-print-region alchemist-macroexpand-region alchemist-message alchemist-message--initialize-buffer alchemist-message-mode alchemist-mix alchemist-mix--completing-read alchemist-mix--execute-test alchemist-mix--test-file alchemist-mix-compile alchemist-mix-display-mix-buffer alchemist-mix-execute alchemist-mix-filter alchemist-mix-mode alchemist-mix-rerun-last-task alchemist-mix-rerun-last-test alchemist-mix-run alchemist-mix-send-input-to-mix-process alchemist-mix-test alchemist-mix-test-at-point alchemist-mix-test-file alchemist-mix-test-stale alchemist-mix-test-this-buffer alchemist-mode alchemist-mode-hook alchemist-mode-menu alchemist-phoenix-enable-mode alchemist-phoenix-find-channels alchemist-phoenix-find-controllers alchemist-phoenix-find-dir alchemist-phoenix-find-models alchemist-phoenix-find-static alchemist-phoenix-find-templates alchemist-phoenix-find-views alchemist-phoenix-find-web alchemist-phoenix-mode alchemist-phoenix-project-p alchemist-phoenix-router alchemist-phoenix-routes alchemist-project--create-test-for-current-file alchemist-project--grok-module-name alchemist-project--insert-test-boilerplate alchemist-project-create-file alchemist-project-elixir-p alchemist-project-elixir-root alchemist-project-file-under-test alchemist-project-find-dir alchemist-project-find-lib alchemist-project-find-test alchemist-project-name alchemist-project-open-file-for-current-tests alchemist-project-open-tests-for-current-file alchemist-project-p alchemist-project-root alchemist-project-root-or-default-dir alchemist-project-run-tests-for-current-file alchemist-project-toggle-file-and-tests alchemist-project-toggle-file-and-tests-other-window alchemist-project-top-level-dir-p alchemist-refcard alchemist-refcard--buffer alchemist-refcard--build-empty-tabulated-row alchemist-refcard--build-tabulated-refcard-title-row alchemist-refcard--build-tabulated-row alchemist-refcard--build-tabulated-title-row alchemist-refcard--describe-funtion-at-point alchemist-refcard--get-keybinding alchemist-refcard--tabulated-list-entries alchemist-refcard-mode alchemist-report--handle-exit alchemist-report--kill-process alchemist-report--last-run-successful-p alchemist-report--render-report alchemist-report--sentinel alchemist-report--store-process-status alchemist-report-activate-mode alchemist-report-cleanup-process-buffer alchemist-report-display-buffer alchemist-report-filter alchemist-report-interrupt-current-process alchemist-report-run alchemist-report-update-mode-name alchemist-scope--modules alchemist-scope-alias-full-path alchemist-scope-aliases alchemist-scope-all-modules alchemist-scope-expression alchemist-scope-extract-function alchemist-scope-extract-module alchemist-scope-import-modules alchemist-scope-inside-module-p alchemist-scope-inside-string-p alchemist-scope-module alchemist-scope-use-modules alchemist-server--store-process alchemist-server-api-code alchemist-server-build-request-string alchemist-server-complete-candidates alchemist-server-contains-end-marker-p alchemist-server-eval alchemist-server-goto alchemist-server-help alchemist-server-help-with-modules alchemist-server-info alchemist-server-prepare-filter-output alchemist-server-process alchemist-server-process-name alchemist-server-process-p alchemist-server-send-request alchemist-server-start alchemist-server-start-if-not-running alchemist-server-start-in-env alchemist-server-status alchemist-test--handle-exit alchemist-test--open-file alchemist-test--render-file alchemist-test--render-files alchemist-test--render-report alchemist-test--render-stacktrace-files alchemist-test--render-test-failing-files alchemist-test--set-modeline-color alchemist-test-clean-compilation-output alchemist-test-enable-mode alchemist-test-execute alchemist-test-initialize-modeline alchemist-test-mode alchemist-test-mode--buffer-contains-tests-p alchemist-test-mode--highlight-syntax alchemist-test-mode--tests-in-buffer alchemist-test-mode-jump-to-next-test alchemist-test-mode-jump-to-previous-test alchemist-test-mode-list-tests alchemist-test-next-result alchemist-test-next-stacktrace-file alchemist-test-previous-result alchemist-test-previous-stacktrace-file alchemist-test-report-mode alchemist-test-reset-modeline alchemist-test-save-buffers alchemist-test-toggle-test-report-display alchemist-utils--snakecase-to-camelcase alchemist-utils-add-ext-to-path-if-not-present alchemist-utils-add-trailing-slash alchemist-utils-build-command alchemist-utils-count-char-occurence alchemist-utils-elixir-version alchemist-utils-elixir-version-check-p alchemist-utils-jump-to-next-matching-line alchemist-utils-jump-to-previous-matching-line alchemist-utils-jump-to-regex alchemist-utils-occur-in-buffer-p alchemist-utils-path-to-module-name alchemist-utils-prepare-aliases-for-elixir alchemist-utils-prepare-modules-for-elixir alchemist-utils-remove-dot-at-the-end alchemist-utils-test-file-p alchemist-version) nil (alchemist-company alchemist-company-open-definition alchemist-company-show-documentation alchemist-compile alchemist-compile-file alchemist-compile-mode alchemist-compile-this-buffer alchemist-elixir-version alchemist-eval-buffer alchemist-eval-close-popup alchemist-eval-current-line alchemist-eval-mode alchemist-eval-print-buffer alchemist-eval-print-current-line alchemist-eval-print-quoted-buffer alchemist-eval-print-quoted-current-line alchemist-eval-print-quoted-region alchemist-eval-print-region alchemist-eval-quoted-buffer alchemist-eval-quoted-current-line alchemist-eval-quoted-region alchemist-eval-region alchemist-execute alchemist-execute-file alchemist-execute-mode alchemist-execute-this-buffer alchemist-goto-definition-at-point alchemist-goto-jump-back alchemist-goto-jump-to-next-def-symbol alchemist-goto-jump-to-previous-def-symbol alchemist-goto-list-symbol-definitions alchemist-help alchemist-help-history alchemist-help-minor-mode alchemist-help-minor-mode-key-binding-summary alchemist-help-module alchemist-help-search-at-point alchemist-hex-all-dependencies alchemist-hex-info alchemist-hex-info-at-point alchemist-hex-mode alchemist-hex-releases alchemist-hex-releases-at-point alchemist-hex-search alchemist-iex-clear-buffer alchemist-iex-compile-this-buffer alchemist-iex-compile-this-buffer-and-go alchemist-iex-mode alchemist-iex-open-input-ring alchemist-iex-project-run alchemist-iex-reload-module alchemist-iex-run alchemist-iex-send-current-line alchemist-iex-send-current-line-and-go alchemist-iex-send-last-sexp alchemist-iex-send-region alchemist-iex-send-region-and-go alchemist-iex-start-process alchemist-info-close-popup alchemist-info-datatype-at-point alchemist-info-mode alchemist-info-types-at-point alchemist-macroexpand-close-popup alchemist-macroexpand-current-line alchemist-macroexpand-mode alchemist-macroexpand-once-current-line alchemist-macroexpand-once-print-current-line alchemist-macroexpand-once-print-region alchemist-macroexpand-once-region alchemist-macroexpand-print-current-line alchemist-macroexpand-print-region alchemist-macroexpand-region alchemist-message-mode alchemist-mix alchemist-mix-compile alchemist-mix-display-mix-buffer alchemist-mix-mode alchemist-mix-rerun-last-task alchemist-mix-rerun-last-test alchemist-mix-run alchemist-mix-send-input-to-mix-process alchemist-mix-test alchemist-mix-test-at-point alchemist-mix-test-file alchemist-mix-test-stale alchemist-mix-test-this-buffer alchemist-mode alchemist-mode-menu alchemist-phoenix-find-channels alchemist-phoenix-find-controllers alchemist-phoenix-find-models alchemist-phoenix-find-static alchemist-phoenix-find-templates alchemist-phoenix-find-views alchemist-phoenix-find-web alchemist-phoenix-mode alchemist-phoenix-router alchemist-phoenix-routes alchemist-project-create-file alchemist-project-find-lib alchemist-project-find-test alchemist-project-run-tests-for-current-file alchemist-project-toggle-file-and-tests alchemist-project-toggle-file-and-tests-other-window alchemist-refcard alchemist-refcard--describe-funtion-at-point alchemist-refcard-mode alchemist-report-interrupt-current-process alchemist-server-start alchemist-server-status alchemist-test-mode alchemist-test-mode-jump-to-next-test alchemist-test-mode-jump-to-previous-test alchemist-test-mode-list-tests alchemist-test-next-result alchemist-test-next-stacktrace-file alchemist-test-previous-result alchemist-test-previous-stacktrace-file alchemist-test-report-mode alchemist-test-toggle-test-report-display alchemist-version))"
    ];
    assert_alchemist_parity(elisp_form, expect);
}

#[test]
fn alchemist_callback_arglists_preserve_documented_ignored_parameter_names() {
    let elisp_form = r##"(mapcar
                      (lambda (function)
                        (list
                         function
                         (help-function-arglist function t)))
                      '(alchemist-company-doc-buffer-filter
                        alchemist-company-filter
                        alchemist-goto-filter
                        alchemist-help-complete-filter-output
                        alchemist-help-filter-output
                        alchemist-help-modules-filter
                        alchemist-info-datatype-filter
                        alchemist-macroexpand-filter
                        alchemist-macroexpand-insert-filter))"##;
    let expect = expect![[
        "OK ((alchemist-company-doc-buffer-filter (_process output)) (alchemist-company-filter (_process output)) (alchemist-goto-filter (_process output)) (alchemist-help-complete-filter-output (_process output)) (alchemist-help-filter-output (_process output)) (alchemist-help-modules-filter (_process output)) (alchemist-info-datatype-filter (_process output)) (alchemist-macroexpand-filter (_process output)) (alchemist-macroexpand-insert-filter (_process output)))"
    ]];
    assert_alchemist_parity(elisp_form, expect);
}

#[test]
fn alchemist_global_keymap_hooks_menu_and_core_mode_contract_match() {
    let elisp_form = r##"(with-temp-buffer
                      (let ((alchemist-server-processes nil))
                        (cl-letf
                            (((symbol-function
                               'alchemist-server-start-if-not-running)
                              (lambda () 'server-started)))
                          (alchemist-mode 1)
                          (list
                           alchemist-mode
                           mode-name
                           (mapcar
                            (lambda (key)
                              (list
                               key
                               (lookup-key alchemist-mode-keymap
                                           (kbd key))))
                            '("C-c a x" "C-c a t" "C-c a r"
                              "C-c a c b" "C-c a e b" "C-c a p s"
                              "C-c a h e" "C-c a i l" "C-c a v l"
                              "C-c a o k" "C-c a n r" "M-." "M-,"))
                           (memq #'alchemist-mode-hook
                                 elixir-mode-hook)
                           (memq #'alchemist-test-enable-mode
                                 elixir-mode-hook)
                           (memq #'alchemist-phoenix-enable-mode
                                 alchemist-mode-hook)
                           (memq #'alchemist-hooks-test-on-save
                                 after-save-hook)
                           (memq #'alchemist-hooks-compile-on-save
                                 after-save-hook)
                           (keymapp
                            (lookup-key alchemist-mode-keymap
                                        [menu-bar alchemist]))))))"##;
    let expect = expect![[
        r#"OK (t (:eval (propertize "Elixir" 'face alchemist-test--mode-name-face)) (("C-c a x" 1) ("C-c a t" 1) ("C-c a r" 1) ("C-c a c b" 1) ("C-c a e b" 1) ("C-c a p s" 1) ("C-c a h e" 1) ("C-c a i l" 1) ("C-c a v l" 1) ("C-c a o k" 1) ("C-c a n r" 1) ("M-." nil) ("M-," nil)) (alchemist-mode-hook) nil (alchemist-phoenix-enable-mode) #1=(alchemist-hooks-test-on-save) (alchemist-hooks-compile-on-save . #1#) nil)"#
    ]];
    assert_alchemist_parity(elisp_form, expect);
}
