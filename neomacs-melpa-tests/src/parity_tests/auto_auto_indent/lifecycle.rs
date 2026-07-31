use expect_test::expect;

use super::assert_auto_auto_indent_batch;

#[test]
fn lifecycle_public_surface_batch() {
    assert_auto_auto_indent_batch(&[
        (
            "auto_auto_indent_raw_source_enable_without_legacy_cl_pushnew_surfaces_upstream_failure",
            r##"(let ((definition
                                (symbol-function
                                 'pushnew)))
          (unwind-protect
              (progn
                (fmakunbound 'pushnew)
                (load
                 (getenv
                  "NEOMACS_PACKAGE_SOURCE")
                 nil
                 t
                 t)
                (with-temp-buffer
                  (fundamental-mode)
                  (list
                   (auto-auto-indent-test-error-data
                    (lambda ()
                      (auto-auto-indent-mode
                       1)))
                   auto-auto-indent-mode
                   (memq
                    'aai-post-command-hook
                    post-command-hook)
                   (memq
                    'aai-before-change-function
                    before-change-functions))))
            (fset 'pushnew definition)))"##,
            true,
            expect!["OK ((:error void-function (pushnew)) t (aai-post-command-hook) nil)"],
        ),
        (
            "auto_auto_indent_enable_installs_buffer_local_hooks_keymap_and_mode_line",
            r##"(with-temp-buffer
          (fundamental-mode)
          (auto-auto-indent-mode 1)
          (list
           auto-auto-indent-mode
           aai-mode
           (assq
            'auto-auto-indent-mode
            minor-mode-alist)
           (auto-auto-indent-test-hook-count
            'aai-post-command-hook
            'post-command-hook)
           (auto-auto-indent-test-hook-count
            'aai-before-change-function
            'before-change-functions)
           (local-variable-p
            'post-command-hook)
           (local-variable-p
            'before-change-functions)
           aai-indent-function
           (minor-mode-key-binding
            [remap newline])
           (minor-mode-key-binding
            [remap yank])
           (minor-mode-key-binding
            [remap delete-char])))"##,
            true,
            expect![[
        r#"OK (t t (auto-auto-indent-mode " aai") 1 1 t nil aai-indent-line-maybe ((auto-auto-indent-mode . aai-newline-and-indent)) ((auto-auto-indent-mode . aai-indented-yank)) ((auto-auto-indent-mode . aai-delete-char)))"#
    ]],
        ),
        (
            "auto_auto_indent_repeated_enable_disable_and_reenable_are_hook_idempotent",
            r##"(with-temp-buffer
          (fundamental-mode)
          (let (states)
            (dolist (argument '(1 1 -1 -1 1))
              (auto-auto-indent-mode argument)
              (push
               (list
                argument
                auto-auto-indent-mode
                (auto-auto-indent-test-hook-count
                 'aai-post-command-hook
                 'post-command-hook)
                (auto-auto-indent-test-hook-count
                 'aai-before-change-function
                 'before-change-functions)
                (minor-mode-key-binding
                 [remap newline]))
               states))
            (nreverse states)))"##,
            true,
            expect![
        "OK ((1 t 1 1 ((auto-auto-indent-mode . aai-newline-and-indent))) (1 t 1 1 ((auto-auto-indent-mode . aai-newline-and-indent))) (-1 nil 1 1 nil) (-1 nil 1 1 nil) (1 t 1 1 ((auto-auto-indent-mode . aai-newline-and-indent))))"
    ],
        ),
        (
            "auto_auto_indent_major_mode_setup_selects_defun_strategy_only_for_lisp_modes",
            r##"(mapcar
          (lambda (mode)
            (with-temp-buffer
              (funcall mode)
              (auto-auto-indent-mode 1)
              (list
               mode
               major-mode
               aai-indent-function
               (local-variable-p
                'aai-indent-function))))
          '(emacs-lisp-mode
            lisp-interaction-mode
            fundamental-mode
            text-mode))"##,
            true,
            expect![
        "OK ((emacs-lisp-mode emacs-lisp-mode aai-indent-defun t) (lisp-interaction-mode lisp-interaction-mode aai-indent-defun t) (fundamental-mode fundamental-mode aai-indent-line-maybe nil) (text-mode text-mode aai-indent-line-maybe nil))"
    ],
        ),
        (
            "auto_auto_indent_mode_hook_runs_before_internal_hook_key_and_strategy_setup",
            r##"(with-temp-buffer
          (emacs-lisp-mode)
          (let (observed)
            (add-hook
             'aai-mode-hook
             (lambda ()
               (setq observed
                     (list
                      aai-mode
                      aai-indent-function
                      (memq
                       'aai-post-command-hook
                       post-command-hook)
                      (memq
                       'aai-before-change-function
                       before-change-functions)
                      (lookup-key
                       auto-auto-indent-mode-map
                       [remap newline]))))
             nil
             t)
            (auto-auto-indent-mode 1)
            (list
             observed
             aai-indent-function
             (memq
              'aai-post-command-hook
              post-command-hook)
             (memq
              'aai-before-change-function
              before-change-functions)
             (lookup-key
              auto-auto-indent-mode-map
              [remap newline]))))"##,
            true,
            expect![
        "OK ((t aai-indent-line-maybe nil nil 1) aai-indent-defun (aai-post-command-hook) (aai-before-change-function) aai-newline-and-indent)"
    ],
        ),
        (
            "auto_auto_indent_c_v_binding_is_added_only_when_current_binding_is_cua_paste",
            r##"(mapcar
          (lambda (binding)
            (with-temp-buffer
              (use-local-map
               (let ((map
                      (make-sparse-keymap)))
                 (define-key map
                   (kbd "C-v")
                   binding)
                 map))
              (setcdr
               auto-auto-indent-mode-map
               nil)
              (auto-auto-indent-mode 1)
              (list
               binding
               (key-binding
                (kbd "C-v"))
               (lookup-key
                auto-auto-indent-mode-map
                (kbd "C-v")))))
          '(cua-paste
            scroll-up-command
            ignore))"##,
            true,
            expect![
        "OK ((cua-paste aai-indented-yank aai-indented-yank) (scroll-up-command scroll-up-command nil) (ignore ignore nil))"
    ],
        ),
        (
            "auto_auto_indent_mode_map_contains_every_documented_command_remapping",
            r##"(with-temp-buffer
          (fundamental-mode)
          (auto-auto-indent-mode 1)
          (mapcar
           (lambda (command)
             (list
              command
              (lookup-key
               auto-auto-indent-mode-map
               (vector 'remap command))
              (minor-mode-key-binding
               (vector 'remap command))))
           '(yank
             cua-paste
             newline
             open-line
             delete-char
             forward-delete
             backward-delete-char-untabify
             autopair-backspace
             backward-delete-char
             delete-backward-char)))"##,
            true,
            expect![
        "OK ((yank aai-indented-yank ((auto-auto-indent-mode . aai-indented-yank))) (cua-paste aai-indented-yank ((auto-auto-indent-mode . aai-indented-yank))) (newline aai-newline-and-indent ((auto-auto-indent-mode . aai-newline-and-indent))) (open-line aai-open-line ((auto-auto-indent-mode . aai-open-line))) (delete-char aai-delete-char ((auto-auto-indent-mode . aai-delete-char))) (forward-delete aai-delete-char ((auto-auto-indent-mode . aai-delete-char))) (backward-delete-char-untabify aai-backspace ((auto-auto-indent-mode . aai-backspace))) (autopair-backspace aai-backspace ((auto-auto-indent-mode . aai-backspace))) (backward-delete-char aai-backspace ((auto-auto-indent-mode . aai-backspace))) (delete-backward-char aai-backspace ((auto-auto-indent-mode . aai-backspace))))"
    ],
        ),
        (
            "auto_auto_indent_mode_state_hooks_and_change_flags_are_isolated_per_buffer",
            r##"(let ((first
                                (generate-new-buffer
                                 " *aai-first*"))
                               (second
                                (generate-new-buffer
                                 " *aai-second*")))
          (unwind-protect
              (progn
                (with-current-buffer first
                  (emacs-lisp-mode)
                  (auto-auto-indent-mode 1)
                  (setq aai--change-flag
                        :first))
                (with-current-buffer second
                  (fundamental-mode)
                  (setq aai--change-flag
                        :second))
                (list
                 (with-current-buffer first
                   (list
                    aai-mode
                    aai-indent-function
                    aai--change-flag
                    (memq
                     'aai-post-command-hook
                     post-command-hook)))
                 (with-current-buffer second
                   (list
                    aai-mode
                    aai-indent-function
                    aai--change-flag
                    (memq
                     'aai-post-command-hook
                     post-command-hook)))
                 (default-value
                  'aai--change-flag)))
            (when (buffer-live-p first)
              (kill-buffer first))
            (when (buffer-live-p second)
              (kill-buffer second))))"##,
            true,
            expect![
        "OK ((t aai-indent-defun :first (aai-post-command-hook)) (nil aai-indent-line-maybe :second nil) nil)"
    ],
        ),
        (
            "auto_auto_indent_aai_mode_function_and_variable_aliases_stay_synchronized",
            r##"(with-temp-buffer
          (fundamental-mode)
          (let ((same-function
                 (eq
                  (indirect-function 'aai-mode)
                  (indirect-function
                   'auto-auto-indent-mode)))
                states)
            (dolist (operation
                     '((aai-mode 1)
                       (auto-auto-indent-mode -1)
                       (setq aai-mode t)
                       (setq
                        auto-auto-indent-mode
                        nil)))
              (eval operation)
              (push
               (list
                operation
                aai-mode
                auto-auto-indent-mode)
               states))
            (list
             same-function
             (indirect-variable
              'aai-mode)
             (nreverse states))))"##,
            true,
            expect![
        "OK (t auto-auto-indent-mode (((aai-mode 1) t t) ((auto-auto-indent-mode -1) nil nil) ((setq aai-mode t) t t) ((setq auto-auto-indent-mode nil) nil nil)))"
    ],
        ),
        (
            "auto_auto_indent_late_multiple_cursors_and_paredit_integrations_apply_once",
            r##"(progn
          (defvar mc/unsupported-minor-modes nil)
          (setq mc/unsupported-minor-modes nil)
          (setq features
                (delq
                 'multiple-cursors-core
                 (delq 'paredit features)))
          (aai--minor-mode-setup)
          (provide 'multiple-cursors-core)
          (provide 'paredit)
          (list
           mc/unsupported-minor-modes
           (length
            (seq-filter
             (lambda (mode)
               (eq mode 'aai-mode))
             mc/unsupported-minor-modes))
           (lookup-key
            auto-auto-indent-mode-map
            [remap paredit-forward-delete])
           (lookup-key
            auto-auto-indent-mode-map
            [remap paredit-backward-delete])))"##,
            true,
            expect!["OK ((aai-mode) 1 aai-delete-char aai-backspace)"],
        ),
    ]);
}
