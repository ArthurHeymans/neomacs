use expect_test::expect;

use super::assert_ansible_vault_parity;

#[test]
fn successful_region_execution_replaces_exact_range_and_restores_process_environment() {
    let elisp_form = r##"(let* ((password
                           (expand-file-name
                            "ansible-vault-execute.pass"
                            temporary-file-directory))
                          (old-environment
                           (getenv
                            "ANSIBLE_VAULT_PASSWORD_FILE"))
                          (calls nil))
                     (unwind-protect
                         (progn
                           (with-temp-file password
                             (insert "secret"))
                           (setenv
                            "ANSIBLE_VAULT_PASSWORD_FILE"
                            "outer-secret")
                           (with-temp-buffer
                             (insert
                              "before<secret: 42>after")
                             (setq-local
                              ansible-vault--password-file
                              password)
                             (cl-letf
                                 (((symbol-function
                                    'shell-command-on-region)
                                   (lambda
                                     (start end command output
                                            &optional _replace error
                                            _display)
                                     (push
                                      (list
                                       (buffer-substring-no-properties
                                        start end)
                                       command
                                       (getenv
                                        "ANSIBLE_VAULT_PASSWORD_FILE"))
                                      calls)
                                     (with-current-buffer
                                         output
                                       (erase-buffer)
                                       (insert
                                        "$ANSIBLE_VAULT;1.1;AES256\n"
                                        "deadbeef\n"))
                                     (with-current-buffer
                                         error
                                       (erase-buffer))
                                     0)))
                               (ansible-vault--execute-on-region
                                "encrypt"
                                8 18)
                               (list
                                (buffer-string)
                                (nreverse calls)
                                (getenv
                                 "ANSIBLE_VAULT_PASSWORD_FILE")
                                (get-buffer
                                 "*ansible-vault-stdout*")
                                (get-buffer
                                 "*ansible-vault-stderr*")))))
                       (setenv
                        "ANSIBLE_VAULT_PASSWORD_FILE"
                        old-environment)
                       (delete-file password)))"##;
    let expect = expect![[
        r#"OK ("before<>after$ANSIBLE_VAULT;1.1;AES256\ndeadbeef\n" (("secret: 42" "ansible-vault encrypt --output=- --vault-password-file=\"[ORACLE-TMPDIR]/ansible-vault-execute.pass\"" nil)) "outer-secret" nil nil)"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn failed_region_execution_preserves_source_reports_command_and_cleans_credentials() {
    let elisp_form = r##"(let* ((password
                           (expand-file-name
                            "ansible-vault-failure.pass"
                            temporary-file-directory))
                          (ansible-vault--password-file-list
                           (list password))
                          (source
                           (generate-new-buffer
                            " *vault-source*"))
                          (error-buffer
                           (get-buffer-create
                            " *vault-errors*")))
                     (unwind-protect
                         (progn
                           (with-temp-file password
                             (insert "secret"))
                           (with-current-buffer source
                             (insert
                              "$ANSIBLE_VAULT;1.2;AES256;prod\n"
                              "deadbeef\n")
                             (setq-local
                              ansible-vault--header-version
                              "1.2")
                             (setq-local
                              ansible-vault--header-vault-id
                              "prod")
                             (setq-local
                              ansible-vault--vault-id
                              "prod")
                             (setq-local
                              ansible-vault--password-file
                              password)
                             (let ((ansible-vault-vault-id-alist
                                    `(("prod" . ,password))))
                               (cl-letf
                                   (((symbol-function
                                      'shell-command-on-region)
                                     (lambda
                                       (_start _end _command output
                                               &optional _replace error
                                               _display)
                                       (with-current-buffer
                                           output
                                         (erase-buffer))
                                       (with-current-buffer
                                           error
                                         (erase-buffer)
                                         (insert
                                          "ERROR! Decryption failed\n"))
                                       1)))
                                 (ansible-vault--execute-on-region
                                  "decrypt"
                                  nil nil nil error-buffer)
                                 (list
                                  (with-current-buffer source
                                    (buffer-string))
                                  (with-current-buffer error-buffer
                                    (buffer-string))
                                  (file-exists-p password)
                                  ansible-vault--vault-id
                                  ansible-vault--password-file
                                  ansible-vault-vault-id-alist
                                  ansible-vault--password-file-list)))))
                       (when
                           (buffer-live-p source)
                         (kill-buffer source))
                       (when
                           (buffer-live-p error-buffer)
                         (kill-buffer error-buffer))
                       (when
                           (file-exists-p password)
                         (delete-file password))))"##;
    let expect = expect![[
        r#"OK ("$ANSIBLE_VAULT;1.2;AES256;prod\ndeadbeef\n" "$ ansible-vault decrypt --output=- --vault-id=\"prod@[ORACLE-TMPDIR]/ansible-vault-failure.pass\" \nERROR! Decryption failed\n\n" t nil nil (("prod" . "[ORACLE-TMPDIR]/ansible-vault-failure.pass")) ("[ORACLE-TMPDIR]/ansible-vault-failure.pass"))"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn process_signal_still_restores_environment_and_destroys_transient_buffers() {
    let elisp_form = r##"(let* ((password
                           (make-temp-file
                            "ansible-vault-signal-"))
                          (old-environment
                           (getenv
                            "ANSIBLE_VAULT_PASSWORD_FILE")))
                     (unwind-protect
                         (progn
                           (setenv
                            "ANSIBLE_VAULT_PASSWORD_FILE"
                            "outer")
                           (with-temp-buffer
                             (insert "secret")
                             (setq-local
                              ansible-vault--password-file
                              password)
                             (list
                              (condition-case error
                                  (cl-letf
                                      (((symbol-function
                                         'shell-command-on-region)
                                        (lambda
                                          (&rest _arguments)
                                          (error
                                           "simulated process crash"))))
                                    (ansible-vault--execute-on-region
                                     "encrypt"))
                                (error
                                 (list
                                  (car error)
                                  (cadr error))))
                              (buffer-string)
                              (getenv
                               "ANSIBLE_VAULT_PASSWORD_FILE")
                              (get-buffer
                               "*ansible-vault-stdout*")
                              (get-buffer
                               "*ansible-vault-stderr*"))))
                       (setenv
                        "ANSIBLE_VAULT_PASSWORD_FILE"
                        old-environment)
                       (delete-file password)))"##;
    let expect = expect![[r#"OK ((error "simulated process crash") "secret" "outer" nil nil)"#]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn current_buffer_and_region_commands_delegate_exact_cli_operations_and_boundaries() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "alpha secret omega")
                      (let ((calls nil))
                        (cl-letf
                            (((symbol-function
                               'ansible-vault--execute-on-region)
                              (lambda
                                (command
                                 &optional start end buffer
                                 error-buffer)
                                (push
                                 (list
                                  command start end buffer
                                  error-buffer
                                  (and
                                   start end
                                   (buffer-substring-no-properties
                                    start end)))
                                 calls))))
                          (ansible-vault-decrypt-current-buffer)
                          (ansible-vault-encrypt-current-buffer)
                          (ansible-vault-encrypt-region
                           7 13)
                          (nreverse calls))))"##;
    let expect = expect![[
        r#"OK (("decrypt" nil nil nil nil nil) ("encrypt" nil nil nil nil nil) ("encrypt_string" 7 13 nil nil "secret"))"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn current_file_commands_coordinate_decryption_encryption_save_and_refingerprint_workflow() {
    let elisp_form = r##"(with-temp-buffer
                      (insert "secret: clear\n")
                      (setq-local
                       ansible-vault--auto-encryption-enabled
                       t)
                      (let ((events nil))
                        (cl-letf
                            (((symbol-function
                               'ansible-vault-decrypt-current-buffer)
                              (lambda ()
                                (push
                                 (list
                                  'decrypt
                                  ansible-vault--auto-encryption-enabled)
                                 events)))
                             ((symbol-function
                               'ansible-vault-encrypt-current-buffer)
                              (lambda ()
                                (push
                                 (list
                                  'encrypt
                                  ansible-vault--auto-encryption-enabled
                                  (buffer-modified-p))
                                 events)
                                (erase-buffer)
                                (insert
                                 "$ANSIBLE_VAULT;1.1;AES256\n"
                                 "deadbeef\n")))
                             ((symbol-function
                               'save-buffer)
                              (lambda
                                (&optional argument)
                                (push
                                 (list
                                  'save
                                  argument
                                  ansible-vault--auto-encryption-enabled
                                  (buffer-modified-p))
                                 events)))
                             ((symbol-function
                               'ansible-vault--fingerprint-buffer)
                              (lambda ()
                                (push
                                 (list
                                  'fingerprint
                                  (buffer-substring-no-properties
                                   (point-min)
                                   (line-end-position
                                    (point-min))))
                                 events))))
                          (ansible-vault-decrypt-current-file)
                          (set-buffer-modified-p
                           nil)
                          (ansible-vault-encrypt-current-file)
                          (list
                           ansible-vault--auto-encryption-enabled
                           (buffer-modified-p)
                           (buffer-string)
                           (nreverse events)))))"##;
    let expect = expect![[
        r#"OK (t t "secret: clear\n" ((decrypt nil) (save 0 nil t) (save 0 t t) (fingerprint "secret: clear\n")))"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}
