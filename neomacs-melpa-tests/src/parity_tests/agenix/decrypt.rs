use expect_test::expect;

use super::assert_agenix_parity;

#[test]
fn agenix_cleartext_identity_decrypt_replaces_real_buffer_and_restores_saved_undo_state() {
    let elisp_form = r##"(let (calls)
         (cl-letf (((symbol-function
                     'agenix--process-exit-code-and-output)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      '(0 "plain secret λ\n"))))
           (with-temp-buffer
             (setq buffer-file-name
                   (expand-file-name
                    "secret.age"
                    (getenv
                     "NEOMACS_TEST_SANDBOX_ROOT"))
                   buffer-undo-list
                   '((1 . 4))
                   agenix--undo-list
                   '((saved . undo)))
             (insert "AGE-ENCRYPTED-CONTENT")
             (read-only-mode 1)
             (set-buffer-modified-p t)
             (let ((result
                    (agenix--decrypt-current-buffer-using-cleartext-identities
                     '("/keys/first"
                       "/path with spaces/second"))))
               (list
                result
                (buffer-string)
                buffer-read-only
                (buffer-modified-p)
                buffer-undo-list
                (nreverse calls))))))"##;
    let expect = expect![[
        r#"OK (#1=((saved . undo)) "plain secret λ\n" nil nil #1# (("age" "--decrypt" "--identity" "/keys/first" "--identity" "/path with spaces/second" "[ORACLE-SANDBOX]/secret.age")))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_cleartext_identity_decrypt_failure_preserves_ciphertext_and_signals_output() {
    let elisp_form = r##"(cl-letf (((symbol-function
                     'agenix--process-exit-code-and-output)
                    (lambda (&rest _arguments)
                      '(9 "no matching identity\n"))))
         (with-temp-buffer
           (setq buffer-file-name
                 (expand-file-name
                  "secret.age"
                  (getenv
                   "NEOMACS_TEST_SANDBOX_ROOT")))
           (insert "CIPHERTEXT")
           (read-only-mode 1)
           (list
            (condition-case error-data
                (agenix--decrypt-current-buffer-using-cleartext-identities
                 '("/keys/wrong"))
              (error
               (list
                (car error-data)
                (cadr error-data))))
            (buffer-string)
            buffer-read-only
            (buffer-modified-p))))"##;
    let expect = expect![[
        r#"OK ((error "Decryption failed: no matching identity\n. Please close the buffer and try again") "CIPHERTEXT" t t)"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_decrypt_reports_nix_evaluation_warning_without_touching_buffer_state() {
    let elisp_form = r##"(let (calls warnings)
         (cl-letf (((symbol-function
                     'agenix--process-exit-code-and-output)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      '(1 "attribute missing")))
                   ((symbol-function 'warn)
                    (lambda (&rest arguments)
                      (push arguments warnings)
                      'warned))
                   ((symbol-function
                     'agenix-locate-secrets-nix)
                    (lambda (_pathname)
                      "/repo/secrets.nix"))
                   ((symbol-function
                     'agenix-path-relative-to-secrets-nix)
                    (lambda (_pathname)
                      "nested/secret.age")))
           (with-temp-buffer
             (setq buffer-file-name
                   "/repo/nested/secret.age"
                   agenix--encrypted-fp
                   'before-fingerprint
                   agenix--keys
                   'before-keys)
             (insert "CIPHERTEXT")
             (read-only-mode 1)
             (list
              (agenix-decrypt-buffer)
              (buffer-string)
              buffer-read-only
              agenix--encrypted-fp
              agenix--keys
              (nreverse calls)
              (nreverse warnings)))))"##;
    let expect = expect![[
        r#"OK (warned "CIPHERTEXT" t before-fingerprint before-keys (("nix-instantiate" "--strict" "--json" "--eval" "--expr" "(import \"/repo/secrets.nix\").\"nested/secret.age\".publicKeys")) (("Nix evaluation error.\nProbably file %s is not declared as a secret in 'secrets.nix' file.\nError: %s" "/repo/nested/secret.age" "attribute missing")))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_decrypt_initializes_new_missing_secret_from_nix_keys_without_calling_age() {
    let elisp_form = r##"(let* ((file
                 (expand-file-name
                  "new-secret.age"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                calls
                messages
                age-called)
         (when (file-exists-p file)
           (delete-file file))
         (cl-letf (((symbol-function
                     'agenix--process-exit-code-and-output)
                    (lambda (&rest arguments)
                      (push arguments calls)
                      '(0 "[\"age1alice\",\"ssh-ed25519 AAA\"]")))
                   ((symbol-function
                     'agenix-locate-secrets-nix)
                    (lambda (_pathname)
                      "/repo/secrets.nix"))
                   ((symbol-function
                     'agenix-path-relative-to-secrets-nix)
                    (lambda (_pathname)
                      "new-secret.age"))
                   ((symbol-function
                     'agenix--decrypt-current-buffer-using-cleartext-identities)
                    (lambda (&rest arguments)
                      (setq age-called arguments)))
                   ((symbol-function 'message)
                    (lambda (&rest arguments)
                      (push arguments messages)
                      'messaged)))
           (with-temp-buffer
             (setq buffer-file-name file)
             (read-only-mode 1)
             (list
              (agenix-decrypt-buffer)
              agenix--encrypted-fp
              agenix--keys
              buffer-read-only
              age-called
              (nreverse calls)
              (nreverse messages)
              (file-exists-p file)))))"##;
    let expect = expect![[
        r#"OK (nil "[ORACLE-SANDBOX]/new-secret.age" ("age1alice" "ssh-ed25519 AAA") nil nil (("nix-instantiate" "--strict" "--json" "--eval" "--expr" "(import \"/repo/secrets.nix\").\"new-secret.age\".publicKeys")) (("Not decrypting. File %s does not exist and will be created when you will save this buffer." "[ORACLE-SANDBOX]/new-secret.age")) nil)"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_decrypt_uses_every_existing_cleartext_key_in_real_file_order() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agenix-cleartext-flow"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (secret
                 (expand-file-name "secret.age" root))
                (first
                 (expand-file-name "first.key" root))
                (second
                 (expand-file-name "second.key" root))
                calls
                decrypt-calls)
         (unwind-protect
             (progn
               (make-directory root t)
               (dolist (file (list secret first second))
                 (write-region
                  "data" nil file nil 'silent))
               (cl-letf (((symbol-function
                           'agenix--process-exit-code-and-output)
                          (lambda (&rest arguments)
                            (push arguments calls)
                            '(0 "[\"recipient-a\",\"recipient-b\"]")))
                         ((symbol-function
                           'agenix-locate-secrets-nix)
                          (lambda (_pathname)
                            "/repo/secrets.nix"))
                         ((symbol-function
                           'agenix-path-relative-to-secrets-nix)
                          (lambda (_pathname)
                            "secret.age"))
                         ((symbol-function
                           'agenix--identity-protected-p)
                          (lambda (identity)
                            (push
                             (list 'protected identity)
                             calls)
                            nil))
                         ((symbol-function
                           'agenix--decrypt-current-buffer-using-cleartext-identities)
                          (lambda (identities)
                            (push identities decrypt-calls)
                            'decrypted)))
                 (with-temp-buffer
                   (setq buffer-file-name secret)
                   (let ((agenix-key-files
                          (list first second)))
                     (list
                      (agenix-decrypt-buffer)
                      agenix--encrypted-fp
                      agenix--keys
                      (nreverse decrypt-calls)
                      (nreverse calls))))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (decrypted "[ORACLE-SANDBOX]/agenix-cleartext-flow/secret.age" ("recipient-a" "recipient-b") (("[ORACLE-SANDBOX]/agenix-cleartext-flow/first.key" "[ORACLE-SANDBOX]/agenix-cleartext-flow/second.key")) (("nix-instantiate" "--strict" "--json" "--eval" "--expr" "(import \"/repo/secrets.nix\").\"secret.age\".publicKeys") (protected "[ORACLE-SANDBOX]/agenix-cleartext-flow/first.key") (protected "[ORACLE-SANDBOX]/agenix-cleartext-flow/second.key")))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_decrypt_password_flow_selects_key_creates_temp_identity_and_always_deletes_it() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agenix-password-flow"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (secret
                 (expand-file-name "secret.age" root))
                (first
                 (expand-file-name "first.key" root))
                (second
                 (expand-file-name "second.key" root))
                (temporary
                 (expand-file-name
                  "temporary-clear-key"
                  root))
                events)
         (unwind-protect
             (progn
               (make-directory root t)
               (dolist (file (list secret first second))
                 (write-region
                  "data" nil file nil 'silent))
               (cl-letf (((symbol-function
                           'agenix--process-exit-code-and-output)
                          (lambda (&rest arguments)
                            (push arguments events)
                            '(0 "[\"recipient\"]")))
                         ((symbol-function
                           'agenix-locate-secrets-nix)
                          (lambda (_pathname)
                            "/repo/secrets.nix"))
                         ((symbol-function
                           'agenix-path-relative-to-secrets-nix)
                          (lambda (_pathname)
                            "secret.age"))
                         ((symbol-function
                           'agenix--identity-protected-p)
                          (lambda (identity)
                            (push
                             (list 'protected identity)
                             events)
                            t))
                         ((symbol-function 'completing-read)
                          (lambda (&rest arguments)
                            (push
                             (cons
                              'completing-read
                              arguments)
                             events)
                            second))
                         ((symbol-function
                           'agenix--prompt-password)
                          (lambda (identity)
                            (push
                             (list 'password identity)
                             events)
                            "secret-password"))
                         ((symbol-function
                           'agenix--create-temp-identity)
                          (lambda (identity password)
                            (push
                             (list
                              'create identity password)
                             events)
                            (write-region
                             "CLEAR" nil temporary
                             nil 'silent)
                            temporary))
                         ((symbol-function
                           'agenix--decrypt-current-buffer-using-cleartext-identities)
                          (lambda (identities)
                            (push
                             (list 'decrypt identities)
                             events)
                            'decrypted)))
                 (with-temp-buffer
                   (setq buffer-file-name secret)
                   (let ((agenix-key-files
                          (list first second)))
                     (list
                      (agenix-decrypt-buffer)
                      agenix--encrypted-fp
                      agenix--keys
                      (file-exists-p temporary)
                      (nreverse events))))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (decrypted "[ORACLE-SANDBOX]/agenix-password-flow/secret.age" ("recipient") nil (("nix-instantiate" "--strict" "--json" "--eval" "--expr" "(import \"/repo/secrets.nix\").\"secret.age\".publicKeys") (protected "[ORACLE-SANDBOX]/agenix-password-flow/first.key") (completing-read "Select private key to use (or enter a custom path): " ("[ORACLE-SANDBOX]/agenix-password-flow/first.key" "[ORACLE-SANDBOX]/agenix-password-flow/second.key") nil nil) (protected "[ORACLE-SANDBOX]/agenix-password-flow/second.key") (password "[ORACLE-SANDBOX]/agenix-password-flow/second.key") (create "[ORACLE-SANDBOX]/agenix-password-flow/second.key" "secret-password") (decrypt ("[ORACLE-SANDBOX]/agenix-password-flow/temporary-clear-key"))))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}
