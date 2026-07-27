use expect_test::expect;

use super::assert_ameba_parity;

#[test]
fn package_defaults_custom_metadata_and_loaded_feature_match() {
    let elisp_form = r##"(list
                      (featurep 'ameba)
                      ameba-project-root-files
                      ameba-check-command
                      (key-description ameba-keymap-prefix)
                      (get 'ameba-project-root-files 'custom-type)
                      (get 'ameba-project-root-files 'custom-group)
                      (get 'ameba-check-command 'custom-type)
                      (get 'ameba-check-command 'custom-group)
                      (get 'ameba-keymap-prefix 'custom-type)
                      (get 'ameba-keymap-prefix 'custom-group)
                      (get 'ameba 'group-documentation)
                      (get 'ameba 'custom-prefix))"##;
    let expect = expect![[
        r#"OK (t (".projectile" ".git" ".hg" ".ameba.yml" "shard.yml") "ameba --format flycheck" "C-c C-r" (repeat string) nil string nil string nil "An Emacs interface to Ameba" "ameba-")"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn complete_shipped_callable_surface_has_exact_arglists_and_interactive_contracts() {
    let elisp_form = r##"(mapcar
                      (lambda (symbol)
                        (list
                         symbol
                         (fboundp symbol)
                         (help-function-arglist symbol t)
                         (commandp symbol)
                         (interactive-form symbol)))
                      '(ameba-local-file-name
                        ameba-project-root
                        ameba-project-lib
                        ameba-buffer-name
                        ameba-build-command
                        ameba-ensure-installed
                        ameba--file-command
                        ameba--dir-command
                        ameba-check-current-file
                        ameba-check-project
                        ameba-check-directory
                        ameba-mode))"##;
    let expect = expect![
        "OK ((ameba-local-file-name t (file-name) nil nil) (ameba-project-root t (&optional no-error) nil nil) (ameba-project-lib t nil nil nil) (ameba-buffer-name t (file-or-dir) nil nil) (ameba-build-command t (command path) nil nil) (ameba-ensure-installed t nil nil nil) (ameba--file-command t (command) nil nil) (ameba--dir-command t (command &optional directory) nil nil) (ameba-check-current-file t nil t (interactive nil)) (ameba-check-project t nil t (interactive nil)) (ameba-check-directory t (&optional directory) t (interactive nil)) (ameba-mode t (&optional arg) t (interactive (list (if current-prefix-arg (prefix-numeric-value current-prefix-arg) 'toggle)))))"
    ];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn minor_mode_lifecycle_is_buffer_local_runs_hooks_and_preserves_other_buffers() {
    let elisp_form = r##"(let ((first (generate-new-buffer " *ameba-first*"))
                          (second (generate-new-buffer " *ameba-second*"))
                          events)
                      (unwind-protect
                          (progn
                            (with-current-buffer first
                              (add-hook
                               'ameba-mode-hook
                               (lambda ()
                                 (push
                                  (list 'first ameba-mode)
                                  events))
                               nil t)
                              (ameba-mode 1))
                            (with-current-buffer second
                              (add-hook
                               'ameba-mode-hook
                               (lambda ()
                                 (push
                                  (list 'second ameba-mode)
                                  events))
                               nil t)
                              (ameba-mode 1)
                              (ameba-mode -1))
                            (list
                             (with-current-buffer first
                               (list
                                ameba-mode
                                (assq 'ameba-mode
                                      minor-mode-alist)
                                (assq 'ameba-mode
                                      minor-mode-map-alist)))
                             (with-current-buffer second
                               (list
                                ameba-mode
                                (assq 'ameba-mode
                                      minor-mode-alist)
                                (assq 'ameba-mode
                                      minor-mode-map-alist)))
                             (nreverse events)))
                        (kill-buffer first)
                        (kill-buffer second)))"##;
    let expect = expect![[
        r#"OK ((t #1=(ameba-mode " Ameba") #2=(ameba-mode keymap (3 keymap (18 keymap (102 . ameba-check-current-file))))) (nil #1# #2#) ((first t) (second t) (second nil)))"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}

#[test]
fn mode_keymap_exposes_only_the_documented_file_command_under_the_frozen_prefix() {
    let elisp_form = r##"(let ((original-prefix
                           (key-description ameba-keymap-prefix)))
                      (list
                       original-prefix
                       (lookup-key ameba-mode-map (kbd "C-c C-r"))
                       (lookup-key ameba-mode-map (kbd "C-c C-r f"))
                       (lookup-key ameba-mode-map (kbd "C-c C-r p"))
                       (progn
                         (setq ameba-keymap-prefix (kbd "C-c a"))
                         (list
                          (lookup-key ameba-mode-map (kbd "C-c C-r f"))
                          (lookup-key ameba-mode-map (kbd "C-c a f"))))))"##;
    let expect = expect![[
        r#"OK ("C-c C-r" (keymap (102 . ameba-check-current-file)) ameba-check-current-file nil (ameba-check-current-file 2))"#
    ]];
    assert_ameba_parity(elisp_form, expect);
}
