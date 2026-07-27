use expect_test::expect;

use super::{assert_ansible_vault_autoload_parity, assert_ansible_vault_parity};

#[test]
fn installed_descriptor_source_and_feature_match_exact_melpa_transaction() {
    let elisp_form = r##"(let ((descriptor
                         (cadr
                          (assq
                           'ansible-vault
                           package-alist))))
                     (list
                      (featurep 'ansible-vault)
                      (package-desc-name descriptor)
                      (package-version-join
                       (package-desc-version descriptor))
                      (package-desc-reqs descriptor)
                      (package-desc-summary descriptor)
                      ansible-vault-version
                      (file-name-nondirectory
                       (symbol-file
                        'ansible-vault-mode
                        'defun))))"##;
    let expect = expect![[
        r#"OK (t ansible-vault "20251029.2146" ((emacs (26 1))) "Minor mode for editing ansible vault files." "0.6.1" "ansible-vault.el")"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn generated_autoload_exposes_only_documented_entry_points_before_source_load() {
    let elisp_form = r##"(list
                      (featurep 'ansible-vault)
                      (featurep
                       'ansible-vault-autoloads)
                      (mapcar
                       (lambda (symbol)
                         (list
                          symbol
                          (fboundp symbol)
                          (and
                           (fboundp symbol)
                           (autoloadp
                            (symbol-function
                             symbol)))))
                       '(ansible-vault--is-encrypted-vault-file
                         ansible-vault--kill-emacs-hook
                         ansible-vault-mode))
                      (boundp
                       'ansible-vault-version)
                      (and
                       (member
                        (file-name-directory
                         (getenv
                          "NEOMACS_PACKAGE_SOURCE"))
                        load-path)
                       t))"##;
    let expect = expect![
        "OK (nil t ((ansible-vault--is-encrypted-vault-file t t) (ansible-vault--kill-emacs-hook t t) (ansible-vault-mode t t)) nil nil)"
    ];
    assert_ansible_vault_autoload_parity(elisp_form, expect);
}

#[test]
fn complete_callable_surface_has_exact_arglists_and_command_status() {
    let elisp_form = r##"(mapcar
                      (lambda (function)
                        (list
                         function
                         (fboundp function)
                         (copy-tree
                          (help-function-arglist
                           function t))
                         (commandp function)))
                      '(ansible-vault--fingerprint-buffer
                        ansible-vault--is-encrypted-vault-file
                        ansible-vault--sub-command-type
                        ansible-vault--command-flags
                        ansible-vault--shell-command
                        ansible-vault--process-config-files
                        ansible-vault--create-password-file
                        ansible-vault--request-password
                        ansible-vault--request-vault-id
                        ansible-vault--guess-password-file
                        ansible-vault--flush-password-file
                        ansible-vault--flush-vault-id
                        ansible-vault--cleanup-password-error
                        ansible-vault--clear-local-variables
                        ansible-vault--error-buffer
                        ansible-vault--execute-on-region
                        ansible-vault-decrypt-current-buffer
                        ansible-vault-decrypt-current-file
                        ansible-vault-encrypt-current-buffer
                        ansible-vault-encrypt-current-file
                        ansible-vault-decrypt-region
                        ansible-vault-encrypt-region
                        ansible-vault--chord
                        ansible-vault--before-save-hook
                        ansible-vault--after-save-hook
                        ansible-vault--kill-buffer-hook
                        ansible-vault--kill-emacs-hook
                        ansible-vault-mode))"##;
    let expect = expect![
        "OK ((ansible-vault--fingerprint-buffer t nil nil) (ansible-vault--is-encrypted-vault-file t nil nil) (ansible-vault--sub-command-type t (sub-command) nil) (ansible-vault--command-flags t (sub-command) nil) (ansible-vault--shell-command t (sub-command) nil) (ansible-vault--process-config-files t nil nil) (ansible-vault--create-password-file t (password) nil) (ansible-vault--request-password t (password) t) (ansible-vault--request-vault-id t (vault-id &optional password-file) t) (ansible-vault--guess-password-file t nil t) (ansible-vault--flush-password-file t nil nil) (ansible-vault--flush-vault-id t nil nil) (ansible-vault--cleanup-password-error t nil nil) (ansible-vault--clear-local-variables t nil nil) (ansible-vault--error-buffer t nil nil) (ansible-vault--execute-on-region t (command &optional start end buffer error-buffer) nil) (ansible-vault-decrypt-current-buffer t nil t) (ansible-vault-decrypt-current-file t nil t) (ansible-vault-encrypt-current-buffer t nil t) (ansible-vault-encrypt-current-file t nil t) (ansible-vault-decrypt-region t (start end) t) (ansible-vault-encrypt-region t (start end) t) (ansible-vault--chord t (chord) nil) (ansible-vault--before-save-hook t nil nil) (ansible-vault--after-save-hook t nil nil) (ansible-vault--kill-buffer-hook t nil nil) (ansible-vault--kill-emacs-hook t nil nil) (ansible-vault-mode t (&optional arg) t))"
    ];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn configuration_state_locality_custom_metadata_aliases_and_integrations_match() {
    let elisp_form = r##"(let ((locals
                          '(ansible-vault--header-version
                            ansible-vault--header-cipher-algorithm
                            ansible-vault--header-vault-id
                            ansible-vault--point
                            ansible-vault--password-file
                            ansible-vault--vault-id
                            ansible-vault--auto-encryption-enabled)))
                     (list
                      (list
                       ansible-vault-command
                       (file-name-nondirectory
                        ansible-vault-password-file)
                       ansible-vault-vault-id-alist
                       ansible-vault-minor-mode-prefix
                       ansible-vault--password-file-list
                       ansible-vault--sub-command-type-alist)
                      (mapcar
                       (lambda (variable)
                         (list
                          variable
                          (default-boundp variable)
                          (default-value variable)
                          (local-variable-if-set-p
                           variable)
                          (get variable
                               'permanent-local)))
                       locals)
                      (mapcar
                       (lambda (variable)
                         (list
                          variable
                          (custom-variable-p variable)
                          (get variable 'custom-type)
                          (get variable 'custom-group)))
                       '(ansible-vault-command
                         ansible-vault-password-file
                         ansible-vault-vault-id-alist
                         ansible-vault-minor-mode-prefix))
                      (mapcar
                       (lambda (alias)
                         (list
                          alias
                          (indirect-variable alias)
                          (fboundp alias)))
                       '(ansible-vault-pass-file
                         ansible-vault--is-vault-file
                         ansible-vault--flush-password))
                      (eq
                       (cdr
                        (assq
                         #'ansible-vault--is-encrypted-vault-file
                         magic-mode-alist))
                       #'ansible-vault-mode)
                      (memq
                       #'ansible-vault--kill-emacs-hook
                       kill-emacs-hook)))"##;
    let expect = expect![[
        r#"OK (("ansible-vault" ".vault-pass" nil "C-c a" nil (("create" . :encrypt) ("decrypt" . :decrypt) ("edit" . :encrypt) ("view" . :decrypt) ("encrypt" . :encrypt) ("encrypt_string" . :encrypt) ("rekey" . :unimplemented))) ((ansible-vault--header-version t nil t t) (ansible-vault--header-cipher-algorithm t nil t t) (ansible-vault--header-vault-id t nil t t) (ansible-vault--point t 0 t t) (ansible-vault--password-file t nil t t) (ansible-vault--vault-id t nil t t) (ansible-vault--auto-encryption-enabled t nil t t)) ((ansible-vault-command ((funcall #'#[nil ("ansible-vault") #1=(t)])) string nil) (ansible-vault-password-file ((funcall #'#[nil ((expand-file-name ".vault-pass" "~")) #1#])) string nil) (ansible-vault-vault-id-alist ((funcall #'#[nil ('nil) #1#])) (alist :key-type string :value-type string) nil) (ansible-vault-minor-mode-prefix ((funcall #'#[nil ("C-c a") #1#])) string nil)) ((ansible-vault-pass-file ansible-vault-password-file nil) (ansible-vault--is-vault-file ansible-vault--is-encrypted-vault-file nil) (ansible-vault--flush-password ansible-vault--flush-password-file nil)) t nil)"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn mode_keymap_resolves_every_documented_workflow_under_configurable_prefix() {
    let elisp_form = r##"(list
                      (ansible-vault--chord "D")
                      (mapcar
                       (lambda (key)
                         (cons
                          key
                          (lookup-key
                           ansible-vault-mode-map
                           (kbd
                            (concat
                             ansible-vault-minor-mode-prefix
                             " "
                             key)))))
                       '("d" "D" "e" "E" "p" "i"))
                      (keymapp
                       ansible-vault-mode-map))"##;
    let expect = expect![[
        r#"OK ("\3aD" (("d" . ansible-vault-decrypt-current-file) ("D" . ansible-vault-decrypt-region) ("e" . ansible-vault-encrypt-current-file) ("E" . ansible-vault-encrypt-region) ("p" . ansible-vault--request-password) ("i" . ansible-vault--request-vault-id)) t)"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}
