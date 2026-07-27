use expect_test::expect;

use super::assert_ansible_vault_parity;

#[test]
fn decrypt_region_handles_indented_yaml_vault_blocks_and_restores_widened_buffer() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "before: keep\n"
                       "    password: !vault |\n"
                       "      $ANSIBLE_VAULT;1.2;AES256;prod\n"
                       "      616263\n"
                       "after: keep\n")
                      (goto-char
                       (point-min))
                      (forward-line 1)
                      (let ((start
                             (point))
                            (observed nil))
                        (forward-line 3)
                        (let ((end
                               (point)))
                          (cl-letf
                              (((symbol-function
                                 'ansible-vault-decrypt-current-buffer)
                                (lambda ()
                                  (setq observed
                                        (list
                                         (buffer-string)
                                         ansible-vault--header-version
                                         ansible-vault--header-cipher-algorithm
                                         ansible-vault--header-vault-id
                                         (point-min)
                                         (point-max)))
                                  (delete-region
                                   (point-min)
                                   (point-max))
                                  (insert
                                   "clear: yes\n"
                                   "roles:\n"
                                   "  - reader\n"))))
                            (ansible-vault-decrypt-region
                             start end))
                          (list
                           observed
                           (buffer-string)
                           (buffer-narrowed-p)
                           (point-min)
                           (point-max)))))"##;
    let expect = expect![[
        r#"OK (("$ANSIBLE_VAULT;1.2;AES256;prod\n616263\n" "1.2" "AES256" "prod" 14 52) "before: keep\n    password: clear: yes\nroles:\n  - reader\nafter: keep\n" nil 1 69)"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn enabling_mode_on_plaintext_configures_local_safety_and_persistent_hooks() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "database_password: clear\n")
                      (let ((events nil)
                            (auto-save-default
                             t))
                        (cl-letf
                            (((symbol-function
                               'auto-save-mode)
                              (lambda
                                (argument)
                                (push
                                 (list
                                  'auto-save
                                  argument)
                                 events)))
                             ((symbol-function
                               'normal-mode)
                              (lambda
                                (&optional _find-file)
                                (push
                                 '(normal-mode)
                                 events))))
                          (ansible-vault-mode 1)
                          (list
                           ansible-vault-mode
                           backup-inhibited
                           ansible-vault--auto-encryption-enabled
                           (memq
                            #'ansible-vault--before-save-hook
                            before-save-hook)
                           (memq
                            #'ansible-vault--after-save-hook
                            after-save-hook)
                           (memq
                            #'ansible-vault--kill-buffer-hook
                            kill-buffer-hook)
                           (get
                            'before-save-hook
                            'permanent-local)
                           (get
                            'after-save-hook
                            'permanent-local)
                           (get
                            'kill-buffer-hook
                            'permanent-local)
                           (nreverse events)))))"##;
    let expect = expect![
        "OK (t t nil (ansible-vault--before-save-hook) (ansible-vault--after-save-hook) (ansible-vault--kill-buffer-hook) t t t ((auto-save -1) (normal-mode)))"
    ];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn enabling_mode_on_encrypted_file_fingerprints_decrypts_and_marks_plaintext_clean() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "$ANSIBLE_VAULT;1.2;AES256;prod\n"
                       "616263\n")
                      (set-buffer-modified-p
                       nil)
                      (let ((events nil)
                            (auto-save-default
                             nil))
                        (cl-letf
                            (((symbol-function
                               'ansible-vault-decrypt-current-buffer)
                              (lambda ()
                                (push
                                 (list
                                  'decrypt
                                  ansible-vault--header-version
                                  ansible-vault--header-cipher-algorithm
                                  ansible-vault--header-vault-id)
                                 events)
                                (erase-buffer)
                                (insert
                                 "database_password: clear\n")))
                             ((symbol-function
                               'normal-mode)
                              (lambda
                                (&optional _find-file)
                                (push
                                 '(normal-mode)
                                 events))))
                          (ansible-vault-mode 1)
                          (list
                           (buffer-string)
                           (buffer-modified-p)
                           ansible-vault-mode
                           ansible-vault--auto-encryption-enabled
                           ansible-vault--header-version
                           ansible-vault--header-cipher-algorithm
                           ansible-vault--header-vault-id
                           (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("database_password: clear\n" nil t t "1.2" "AES256" "prod" ((decrypt "1.2" "AES256" "prod") (normal-mode)))"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn save_hooks_encrypt_then_decrypt_and_restore_cursor_and_clean_state() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "first: alpha\n"
                       "secret: bravo\n"
                       "last: charlie\n")
                      (goto-char 22)
                      (setq-local
                       ansible-vault--auto-encryption-enabled
                       t)
                      (let ((events nil))
                        (cl-letf
                            (((symbol-function
                               'ansible-vault-encrypt-current-buffer)
                              (lambda ()
                                (push
                                 (list
                                  'encrypt
                                  (point)
                                  (buffer-string))
                                 events)
                                (erase-buffer)
                                (insert
                                 "$ANSIBLE_VAULT;1.1;AES256\n"
                                 "deadbeef\n")))
                             ((symbol-function
                               'ansible-vault-decrypt-current-buffer)
                              (lambda ()
                                (push
                                 (list
                                  'decrypt
                                  (point)
                                  (buffer-string))
                                 events)
                                (erase-buffer)
                                (insert
                                 "first: alpha\n"
                                 "secret: bravo\n"
                                 "last: charlie\n"))))
                          (ansible-vault--before-save-hook)
                          (let ((encrypted
                                 (buffer-string))
                                (stored-point
                                 ansible-vault--point))
                            (ansible-vault--after-save-hook)
                            (list
                             encrypted
                             stored-point
                             (buffer-string)
                             (point)
                             ansible-vault--point
                             (buffer-modified-p)
                             (nreverse events))))))"##;
    let expect = expect![[
        r#"OK ("$ANSIBLE_VAULT;1.1;AES256\ndeadbeef\n" 22 "first: alpha\nsecret: bravo\nlast: charlie\n" 22 0 nil ((encrypt 22 "first: alpha\nsecret: bravo\nlast: charlie\n") (decrypt 36 "$ANSIBLE_VAULT;1.1;AES256\ndeadbeef\n")))"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn disabling_modified_mode_reencrypts_removes_hooks_and_clears_session_state() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "database_password: changed\n")
                      (let ((events nil)
                            (auto-save-default
                             t)
                            (ansible-vault--password-file-list
                             nil))
                        (cl-letf
                            (((symbol-function
                               'normal-mode)
                              (lambda
                                (&optional _find-file)))
                             ((symbol-function
                               'auto-save-mode)
                              (lambda
                                (argument)
                                (push
                                 (list
                                  'auto-save
                                  argument)
                                 events)))
                             ((symbol-function
                               'ansible-vault-encrypt-current-buffer)
                              (lambda ()
                                (push
                                 (list
                                  'encrypt
                                  (buffer-string))
                                 events)
                                (erase-buffer)
                                (insert
                                 "$ANSIBLE_VAULT;1.1;AES256\n"
                                 "deadbeef\n"))))
                          (ansible-vault-mode 1)
                          (setq-local
                           ansible-vault--password-file
                           "/keys/external")
                          (set-buffer-modified-p
                           t)
                          (ansible-vault-mode -1)
                          (list
                           (buffer-string)
                           ansible-vault-mode
                           backup-inhibited
                           (memq
                            #'ansible-vault--before-save-hook
                            before-save-hook)
                           (memq
                            #'ansible-vault--after-save-hook
                            after-save-hook)
                           (memq
                            #'ansible-vault--kill-buffer-hook
                            kill-buffer-hook)
                           (mapcar
                            (lambda (variable)
                              (local-variable-p
                               variable))
                            '(ansible-vault--header-version
                              ansible-vault--password-file
                              ansible-vault--auto-encryption-enabled))
                           (nreverse events)))))"##;
    let expect = expect![[
        r#"OK ("$ANSIBLE_VAULT;1.1;AES256\ndeadbeef\n" nil nil nil nil nil (t t t) ((auto-save -1) (encrypt "database_password: changed\n") (auto-save 1)))"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn disabling_clean_mode_reverts_disk_content_instead_of_reencrypting() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "database_password: unchanged\n")
                      (let ((events nil)
                            (auto-save-default
                             nil))
                        (cl-letf
                            (((symbol-function
                               'normal-mode)
                              (lambda
                                (&optional _find-file)))
                             ((symbol-function
                               'revert-buffer)
                              (lambda
                                (&optional ignore-auto noconfirm
                                preserve-modes)
                                (push
                                 (list
                                  'revert
                                  ignore-auto
                                  noconfirm
                                  preserve-modes
                                  (buffer-string))
                                 events)))
                             ((symbol-function
                               'ansible-vault-encrypt-current-buffer)
                              (lambda ()
                                (push
                                 '(unexpected-encrypt)
                                 events))))
                          (ansible-vault-mode 1)
                          (set-buffer-modified-p
                           nil)
                          (ansible-vault-mode -1)
                          (list
                           ansible-vault-mode
                           backup-inhibited
                           (nreverse events)))))"##;
    let expect = expect![[r#"OK (nil nil ((revert nil t nil "database_password: unchanged\n")))"#]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn killing_managed_buffer_flushes_vault_registration_and_generated_password_file() {
    let elisp_form = r##"(let ((ansible-vault-vault-id-alist
                         nil)
                        (ansible-vault--password-file-list
                         nil))
                     (with-temp-buffer
                       (let ((generated
                              (ansible-vault--create-password-file
                               "secret")))
                         (setq-local
                          ansible-vault--vault-id
                          "prod")
                         (push
                          (cons
                           "prod"
                           generated)
                          ansible-vault-vault-id-alist)
                         (ansible-vault--kill-buffer-hook)
                         (list
                          (file-exists-p generated)
                          ansible-vault--vault-id
                          ansible-vault--password-file
                          ansible-vault-vault-id-alist
                          ansible-vault--password-file-list))))"##;
    let expect = expect!["OK (nil nil nil nil nil)"];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn error_buffer_is_reused_read_only_and_kill_hooks_remove_all_generated_secrets() {
    let elisp_form = r##"(let ((ansible-vault--password-file-list
                         nil))
                     (unwind-protect
                         (let ((first
                                (ansible-vault--error-buffer))
                               (generated-one
                                (expand-file-name
                                 "ansible-vault-kill-one.pass"
                                 temporary-file-directory))
                               (generated-two
                                (expand-file-name
                                 "ansible-vault-kill-two.pass"
                                 temporary-file-directory)))
                           (with-temp-file generated-one
                             (insert "one"))
                           (with-temp-file generated-two
                             (insert "two"))
                           (setq
                            ansible-vault--password-file-list
                            (list
                             generated-one
                             generated-two
                             "/definitely/missing/vault-pass"))
                           (ansible-vault--kill-emacs-hook)
                           (list
                            (eq
                             first
                             (ansible-vault--error-buffer))
                            (with-current-buffer first
                              buffer-read-only)
                            (file-exists-p
                             generated-one)
                            (file-exists-p
                             generated-two)
                            ansible-vault--password-file-list))
                       (when
                           (get-buffer
                            "*ansible-vault-error*")
                         (kill-buffer
                          "*ansible-vault-error*"))))"##;
    let expect = expect![[
        r#"OK (t t nil nil ("[ORACLE-TMPDIR]/ansible-vault-kill-one.pass" "[ORACLE-TMPDIR]/ansible-vault-kill-two.pass" "/definitely/missing/vault-pass"))"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}
