use expect_test::expect;

use super::assert_aas_parity;

#[test]
fn aas_mode_adds_one_buffer_local_post_self_insert_hook_and_removes_it_on_disable() {
    let elisp_form = r##"(with-temp-buffer
               (let ((before
                      (list
                       aas-mode
                       (local-variable-p
                        'post-self-insert-hook)
                       (memq
                        #'aas-post-self-insert-hook
                        post-self-insert-hook))))
                 (aas-mode 1)
                 (aas-mode 1)
                 (let ((enabled
                        (list
                         aas-mode
                         (local-variable-p
                          'post-self-insert-hook)
                         (length
                          (delq
                           nil
                           (mapcar
                            (lambda (function)
                              (eq
                               function
                               #'aas-post-self-insert-hook))
                            post-self-insert-hook))))))
                   (aas-mode 0)
                   (list
                    before
                    enabled
                    (list
                     aas-mode
                     (local-variable-p
                      'post-self-insert-hook)
                     (memq
                      #'aas-post-self-insert-hook
                      post-self-insert-hook))))))"##;
    let expect = expect!["OK ((nil nil nil) (t t 1) (nil nil nil))"];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_modes_to_activate_walks_derived_parents_and_function_aliases_from_root_to_leaf() {
    let elisp_form = r##"(progn
               (put
                'neomacs-aas-parent-mode
                'derived-mode-parent
                'neomacs-aas-root-mode)
               (put
                'neomacs-aas-child-mode
                'derived-mode-parent
                'neomacs-aas-parent-mode)
               (defalias
                 'neomacs-aas-alias-mode
                 'neomacs-aas-child-mode)
               (list
                (aas--modes-to-activate
                 'neomacs-aas-child-mode)
                (aas--modes-to-activate
                 'neomacs-aas-alias-mode)
                (aas--modes-to-activate
                 'neomacs-aas-root-mode)))"##;
    let expect = expect![
        "OK ((neomacs-aas-root-mode neomacs-aas-parent-mode neomacs-aas-child-mode) (neomacs-aas-root-mode neomacs-aas-parent-mode neomacs-aas-child-mode neomacs-aas-alias-mode) (neomacs-aas-root-mode))"
    ];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_activate_for_major_mode_enables_mode_and_activates_each_ancestor_with_leaf_precedence() {
    let elisp_form = r##"(progn
               (put
                'neomacs-aas-parent-mode
                'derived-mode-parent
                'neomacs-aas-root-mode)
               (put
                'neomacs-aas-child-mode
                'derived-mode-parent
                'neomacs-aas-parent-mode)
               (dolist
                   (entry
                    '((neomacs-aas-root-mode "ROOT")
                      (neomacs-aas-parent-mode "PARENT")
                      (neomacs-aas-child-mode "CHILD")))
                 (aas-set-snippets
                     (car entry)
                   "x" (cadr entry)))
               (with-temp-buffer
                 (setq major-mode
                       'neomacs-aas-child-mode)
                 (list
                  (aas-activate-for-major-mode)
                  aas-mode
                  (memq
                   #'aas-post-self-insert-hook
                   post-self-insert-hook)
                  aas-active-keymaps
                  (eq
                   (lookup-key aas--prefix-map "x")
                   (lookup-key
                    (gethash
                     'neomacs-aas-child-mode
                     aas-keymaps)
                    "x")))))"##;
    let expect = expect![
        "OK ((neomacs-aas-root-mode neomacs-aas-parent-mode neomacs-aas-child-mode) t (aas-post-self-insert-hook t) (neomacs-aas-child-mode neomacs-aas-parent-mode neomacs-aas-root-mode) t)"
    ];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_embark_menu_passes_exact_prompt_arguments_and_calls_only_a_selected_command() {
    let elisp_form = r##"(let ((aas--prefix-map
                    (make-sparse-keymap))
                   (answers
                    '(neomacs-aas-menu-command nil))
                   events)
               (cl-letf
                   (((symbol-function
                      'embark-completing-read-prompter)
                     (lambda (keymap targets default)
                       (push
                        (list
                         'prompt
                         (eq keymap aas--prefix-map)
                         targets
                         default)
                        events)
                       (pop answers)))
                    ((symbol-function
                      'neomacs-aas-menu-command)
                     (lambda ()
                       (interactive
                        (progn
                          (push
                           '(interactive-spec)
                           events)
                          nil))
                       (push
                        (list
                         'command
                         (called-interactively-p
                          'interactive))
                        events)
                       'menu-result)))
                 (list
                  (aas-embark-menu)
                  (aas-embark-menu)
                  (nreverse events))))"##;
    let expect = expect![[
        r#"OK (menu-result nil ((prompt t nil no-default) (interactive-spec) (command nil) (prompt t nil no-default)))"#
    ]];

    assert_aas_parity(elisp_form, expect);
}

#[test]
fn aas_global_mode_enables_existing_buffers_activates_global_snippets_and_cleans_up_hook() {
    let elisp_form = r##"(let ((target
                    (generate-new-buffer
                     " *neomacs-aas-global*")))
               (unwind-protect
                   (progn
                     (aas-set-snippets
                         'global
                       "gg" "GLOBAL")
                     (cl-letf
                         (((symbol-function 'buffer-list)
                           (lambda ()
                             (list target))))
                       (list
                        (aas-global-mode 1)
                        aas-global-mode
                        (and
                         (memq
                          #'aas-global-mode-enable-in-buffer
                          after-change-major-mode-hook)
                         t)
                        (with-current-buffer target
                          (list
                           aas-mode
                           (copy-sequence
                            aas-active-keymaps)
                           (and
                            (memq
                             #'aas-post-self-insert-hook
                             post-self-insert-hook)
                            t)
                           (functionp
                            (lookup-key
                             aas--prefix-map "gg"))))
                        (aas-global-mode -1)
                        aas-global-mode
                        (and
                         (memq
                          #'aas-global-mode-enable-in-buffer
                          after-change-major-mode-hook)
                         t)
                        (with-current-buffer target
                          (list
                           aas-mode
                           (copy-sequence
                            aas-active-keymaps)
                           (and
                            (memq
                             #'aas-post-self-insert-hook
                             post-self-insert-hook)
                            t))))))
                 (when (buffer-live-p target)
                   (kill-buffer target))))"##;
    let expect = expect!["OK (t t t (t (global) t t) nil nil nil (nil (global) nil))"];

    assert_aas_parity(elisp_form, expect);
}
