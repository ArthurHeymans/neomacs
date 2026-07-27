use expect_test::expect;

use super::assert_ansible_parity;

#[test]
fn ansible_temp_password_file_contains_secret_and_tracks_cleanup_contract() {
    let elisp_form = r##"(let ((path
                (expand-file-name
                 "deterministic-vault-password"
                 default-directory)))
         (when (file-exists-p path)
           (delete-file path))
         (unwind-protect
             (cl-letf
                 (((symbol-function 'make-temp-file)
                   (lambda (&rest _arguments)
                     path)))
               (let ((argument
                      (ansible-vault-create-temp-password-file
                       "correct horse battery staple")))
                 (list
                  (string-prefix-p
                   "--vault-password-file="
                   argument)
                  (file-name-nondirectory argument)
                  (ansible-test-read-file path)
                  (file-equal-p
                   ansible-vault-store-cleanup-file
                   path)
                  (file-modes path))))
           (when (file-exists-p path)
             (delete-file path))
           (setq
            ansible-vault-store-cleanup-file
            nil)))"##;
    let expect =
        expect![[r#"OK (t "deterministic-vault-password" "correct horse battery staple" t 420)"#]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_prompt_and_environment_password_providers_write_exact_secrets() {
    let elisp_form = r##"(let ((prompt-path
                (expand-file-name
                 "prompt-vault-password"
                 default-directory))
               (environment-path
                (expand-file-name
                 "environment-vault-password"
                 default-directory))
               (calls 0)
               (old-value
                (getenv "ANSIBLE_PARITY_VAULT_PASSWORD"))
               prompt-result
               environment-result)
         (unwind-protect
             (progn
               (setenv
                "ANSIBLE_PARITY_VAULT_PASSWORD"
                "environment-secret")
               (let ((ansible-vault-password-environment-variable
                      "ANSIBLE_PARITY_VAULT_PASSWORD"))
                 (cl-letf
                     (((symbol-function 'make-temp-file)
                       (lambda (&rest _arguments)
                         (setq calls (1+ calls))
                         (if (= calls 1)
                             prompt-path
                           environment-path)))
                      ((symbol-function 'read-passwd)
                       (lambda (prompt &rest _arguments)
                         (setq prompt-result
                               (list :prompt prompt))
                         "prompt-secret")))
                   (let ((prompt-argument
                          (ansible-vault-prompt-for-password)))
                     (setq prompt-result
                           (append
                            prompt-result
                            (list
                             (file-name-nondirectory
                              prompt-argument)
                             (ansible-test-read-file
                              prompt-path)))))
                   (let ((environment-argument
                          (ansible-vault-password-from-environment)))
                     (setq environment-result
                           (list
                            (file-name-nondirectory
                             environment-argument)
                            (ansible-test-read-file
                             environment-path))))
                   (list
                    prompt-result
                    environment-result
                    calls
                    (file-equal-p
                     ansible-vault-store-cleanup-file
                     environment-path)))))
           (setenv
            "ANSIBLE_PARITY_VAULT_PASSWORD"
            old-value)
           (dolist
               (path
                (list
                 prompt-path
                 environment-path))
             (when (file-exists-p path)
               (delete-file path)))
           (setq
            ansible-vault-store-cleanup-file
            nil)))"##;
    let expect = expect![[
        r#"OK ((:prompt "Vault Password: " "prompt-vault-password" "prompt-secret") ("environment-vault-password" "environment-secret") 2 t)"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_vault_password_selection_supports_file_string_list_and_invalid_values() {
    let elisp_form = r##"(let ((default-directory
                (file-name-as-directory
                 (make-temp-file
                  "ansible-password-selection-"
                  t))))
         (unwind-protect
             (let* ((ansible-vault-password-file
                     "secrets/team vault.pass")
                    (file-result
                     (let ((ansible-vault-password
                            'file))
                       (ansible-vault-get-password)))
                    (string-result
                     (let ((ansible-vault-password
                            (lambda ()
                              "--ask-vault-pass")))
                       (ansible-vault-get-password)))
                    (list-result
                     (let ((ansible-vault-password
                            (lambda ()
                              '("--vault-id"
                                "production@prompt"))))
                       (ansible-vault-get-password)))
                    (invalid-result
                     (let ((ansible-vault-password
                            'unsupported))
                       (condition-case error-data
                           (ansible-vault-get-password)
                         (error error-data)))))
               (list
                (list
                 (length file-result)
                 (string-prefix-p
                  "--vault-password-file="
                  (car file-result))
                 (string-suffix-p
                  "secrets/team vault.pass"
                  (car file-result)))
                string-result
                list-result
                invalid-result))
           (delete-directory
            default-directory
            t)))"##;
    let expect = expect![[
        r#"OK ((1 t t) ("--ask-vault-pass") ("--vault-id" "production@prompt") (error "Invalid ansible-vault-password value"))"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_vault_invokes_process_directly_and_preserves_indented_tail_structure() {
    let elisp_form = r##"(let ((path
                (expand-file-name
                 "vault-process-input"
                 default-directory))
               invocation
               process-input)
         (when (file-exists-p path)
           (delete-file path))
         (unwind-protect
             (cl-letf
                 (((symbol-function 'make-temp-file)
                   (lambda (&rest _arguments)
                     path))
                  ((symbol-function 'call-process)
                   (lambda
                       (program infile destination display &rest arguments)
                     (setq process-input
                           (ansible-test-read-file
                            (car (last arguments))))
                     (setq invocation
                           (list
                            program
                            infile
                            (bufferp destination)
                            display
                            (butlast arguments)
                            (file-name-nondirectory
                             (car (last arguments)))))
                     (with-current-buffer destination
                       (insert
                        "cipher-line-one\n"
                        "cipher-line-two\n"))
                     0)))
               (let ((output
                      (ansible-vault
                       "encrypt"
                       (concat
                        "  secret-one\n"
                        "  secret-two\n"
                        "outside-block\n"
                        "  untouched-tail")
                       '("--vault-id"
                         "production@/safe path/pass"))))
                 (list
                  output
                  process-input
                  invocation
                  (file-exists-p path)
                  (get-buffer
                   " *ansible-vault-output*"))))
           (when (file-exists-p path)
             (delete-file path))))"##;
    let expect = expect![[
        r#"OK ("  cipher-line-one\n  cipher-line-two\noutside-block\n  untouched-tail" "secret-one\nsecret-two" ("ansible-vault" nil t nil ("encrypt" "--vault-id" "production@/safe path/pass") "vault-process-input") nil nil)"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_vault_failure_reports_exit_code_and_cleans_output_buffer() {
    let elisp_form = r##"(let ((path
                (expand-file-name
                 "vault-failed-input"
                 default-directory))
               invocation
               result)
         (when (file-exists-p path)
           (delete-file path))
         (unwind-protect
             (cl-letf
                 (((symbol-function 'make-temp-file)
                   (lambda (&rest _arguments)
                     path))
                  ((symbol-function 'call-process)
                   (lambda
                       (program _infile destination _display &rest arguments)
                     (setq invocation
                           (list
                            program
                            (bufferp destination)
                            (butlast arguments)
                            (file-name-nondirectory
                             (car (last arguments)))))
                     (with-current-buffer destination
                       (insert "fatal: invalid password\n"))
                     23)))
               (setq result
                     (condition-case error-data
                         (ansible-vault
                          "decrypt"
                          "$ANSIBLE_VAULT;1.1;AES256\ncipher"
                          '("--ask-vault-pass"))
                       (error error-data)))
               (list
                result
                invocation
                (file-exists-p path)
                (get-buffer
                 " *ansible-vault-output*")))
           (when (file-exists-p path)
             (delete-file path))))"##;
    let expect = expect![[
        r#"OK ((error "Error in ‘ansible-vault‘ execution! Exit code: 23") ("ansible-vault" t ("decrypt" "--ask-vault-pass") "vault-failed-input") t nil)"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_vault_string_passes_credentials_cleans_temp_secret_and_rejects_mode() {
    let elisp_form = r##"(let ((cleanup
                (expand-file-name
                 "vault-string-cleanup"
                 default-directory))
               calls)
         (with-temp-file cleanup
           (insert "temporary-secret"))
         (setq
          ansible-vault-store-cleanup-file
          cleanup)
         (unwind-protect
             (cl-letf
                 (((symbol-function
                    'ansible-vault-get-password)
                   (lambda ()
                     '("--vault-id"
                       "staging@prompt")))
                  ((symbol-function 'ansible-vault)
                   (lambda (mode value params)
                     (setq calls
                           (append
                            calls
                            (list
                             (list
                              mode
                              value
                              params))))
                     "transformed-vault")))
               (let ((valid
                      (ansible-vault-string
                       "decrypt"
                       "ciphertext"))
                     (cleanup-after-valid
                      (list
                       (file-exists-p cleanup)
                       ansible-vault-store-cleanup-file))
                     (invalid
                      (condition-case error-data
                          (ansible-vault-string
                           "rotate"
                           "ciphertext")
                        (error error-data))))
                 (list
                  valid
                  cleanup-after-valid
                  invalid
                  calls)))
           (when (file-exists-p cleanup)
             (delete-file cleanup))
           (setq
            ansible-vault-store-cleanup-file
            nil)))"##;
    let expect = expect![[
        r#"OK ("transformed-vault" (nil nil) (error "MODE should be one of ’encrypt’ or ’decrypt’") (("decrypt" "ciphertext" ("--vault-id" "staging@prompt"))))"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_vault_region_encrypts_selection_and_decrypt_region_marks_buffer_clean() {
    let elisp_form = r##"(cl-letf
         (((symbol-function 'ansible-vault-string)
           (lambda (mode value)
             (format
              "<%s:%s>"
              mode
              value))))
         (list
          (with-temp-buffer
            (insert "before SECRET after")
            (set-buffer-modified-p nil)
            (ansible-encrypt-region 8 14)
            (list
             (buffer-string)
             (buffer-modified-p)
             (point)))
          (with-temp-buffer
            (insert "left CIPHERTEXT right")
            (set-buffer-modified-p t)
            (ansible-decrypt-region 6 16)
            (list
             (buffer-string)
             (buffer-modified-p)
             (point)))
          (with-temp-buffer
            (insert "whole buffer")
            (ansible-vault-buffer "encrypt")
            (buffer-string))))"##;
    let expect = expect![[
        r#"OK (("before  after<encrypt:SECRET>" t 30) ("left  right<decrypt:CIPHERTEXT>" nil 32) "<encrypt:whole buffer>")"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_encrypt_and_decrypt_buffer_choose_transform_or_revert_by_modified_state() {
    let elisp_form = r##"(let (events)
         (cl-letf
             (((symbol-function 'ansible-vault-buffer)
               (lambda (mode)
                 (setq events
                       (append
                        events
                        (list
                         (list
                          :vault
                          mode
                          (buffer-modified-p)))))))
              ((symbol-function 'revert-buffer)
               (lambda (&rest arguments)
                 (setq events
                       (append
                        events
                        (list
                         (list
                          :revert
                          arguments
                          (buffer-modified-p))))))))
           (with-temp-buffer
             (insert "modified plaintext")
             (ansible-encrypt-buffer)
             (set-buffer-modified-p nil)
             (ansible-encrypt-buffer)
             (set-buffer-modified-p t)
             (ansible-decrypt-buffer)
             (list
              events
              (buffer-modified-p)))))"##;
    let expect =
        expect![[r#"OK (((:vault "encrypt" t) (:revert (t t) nil) (:vault "decrypt" t)) nil)"#]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_auto_decrypt_transforms_vault_and_installs_buffer_local_save_hooks() {
    let elisp_form = r##"(with-temp-buffer
         (insert
          "$ANSIBLE_VAULT;1.1;AES256\n"
          "616263646566\n")
         (let (events)
           (cl-letf
               (((symbol-function 'ansible-decrypt-buffer)
                 (lambda ()
                   (setq events
                         (append
                          events
                          '(decrypted)))
                   (erase-buffer)
                   (insert
                    "api_token: secret\n"))))
             (ansible-auto-decrypt-encrypt)
             (list
              events
              (buffer-string)
              (local-variable-p
               'before-save-hook)
              (memq
               'ansible-encrypt-buffer
               before-save-hook)
              (local-variable-p
               'after-save-hook)
              (memq
               'ansible-decrypt-buffer
               after-save-hook)))))"##;
    let expect = expect![[
        r#"OK ((decrypted) "api_token: secret\n" t (ansible-encrypt-buffer t) t (ansible-decrypt-buffer t))"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}

#[test]
fn ansible_auto_decrypt_ignores_plaintext_and_reports_decryption_failures() {
    let elisp_form = r##"(let (messages calls)
         (cl-letf
             (((symbol-function 'ansible-decrypt-buffer)
               (lambda ()
                 (setq calls
                       (1+ (or calls 0)))
                 (error "bad vault password")))
              ((symbol-function 'message)
               (lambda (format-string &rest arguments)
                 (setq messages
                       (append
                        messages
                        (list
                         (apply
                          #'format
                          format-string
                          arguments)))))))
           (list
            (with-temp-buffer
              (insert
               "api_token: already-plain\n")
              (ansible-auto-decrypt-encrypt)
              (list
               calls
               before-save-hook
               after-save-hook))
            (with-temp-buffer
              (insert
               "$ANSIBLE_VAULT;1.2;AES256\n"
               "deadbeef\n")
              (ansible-auto-decrypt-encrypt)
              (list
               calls
               before-save-hook
               after-save-hook))
            messages)))"##;
    let expect = expect![[
        r#"OK ((nil nil nil) (1 nil nil) ("Could not decrypt file. Make sure `ansible-vault-password-file' or the environment variable ANSIBLE_VAULT_PASSWORD_FILE is correctly set"))"#
    ]];

    assert_ansible_parity(elisp_form, expect);
}
