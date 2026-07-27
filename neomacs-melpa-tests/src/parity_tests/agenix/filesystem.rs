use expect_test::expect;

use super::assert_agenix_parity;

#[test]
fn agenix_secret_path_helpers_walk_real_deterministic_nested_filesystem() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agenix-paths"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (nested
                 (expand-file-name
                  "teams/backend"
                  root))
                (secret
                 (expand-file-name
                  "database.age"
                  nested))
                (outside
                 (expand-file-name
                  "outside/secret.age"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
         (unwind-protect
             (progn
               (make-directory nested t)
               (make-directory
                (file-name-directory outside) t)
               (write-region
                "{ }\n" nil
                (expand-file-name "secrets.nix" root)
                nil 'silent)
               (write-region
                "ciphertext\n" nil secret nil 'silent)
               (write-region
                "outside\n" nil outside nil 'silent)
               (mapcar
                (lambda (pathname)
                  (list
                   pathname
                   (agenix-secrets-base-dir pathname)
                   (agenix-locate-secrets-nix pathname)
                   (agenix-path-relative-to-secrets-nix
                    pathname)))
                (list
                 secret
                 nested
                 root
                 outside)))
           (when (file-directory-p root)
             (delete-directory root t))
           (let ((outside-root
                  (file-name-directory outside)))
             (when (file-directory-p outside-root)
               (delete-directory outside-root t)))))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-SANDBOX]/agenix-paths/teams/backend/database.age" "[ORACLE-SANDBOX]/agenix-paths/" "[ORACLE-SANDBOX]/agenix-paths/secrets.nix" "teams/backend/database.age") ("[ORACLE-SANDBOX]/agenix-paths/teams/backend" "[ORACLE-SANDBOX]/agenix-paths/" "[ORACLE-SANDBOX]/agenix-paths/secrets.nix" "teams/backend") ("[ORACLE-SANDBOX]/agenix-paths" "[ORACLE-SANDBOX]/agenix-paths/" "[ORACLE-SANDBOX]/agenix-paths/secrets.nix" ".") ("[ORACLE-SANDBOX]/outside/secret.age" nil nil nil))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_key_file_processing_resolves_strings_functions_invalid_entries_and_missing_paths() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agenix-keys"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (first
                 (expand-file-name "first-key" root))
                (second
                 (expand-file-name "second-key" root))
                (missing
                 (expand-file-name "missing-key" root)))
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region
                "FIRST" nil first nil 'silent)
               (write-region
                "SECOND" nil second nil 'silent)
               (let ((agenix-key-files
                      (list
                       first
                       (lambda () second)
                       missing
                       42
                       ""
                       first)))
                 (list
                  (agenix--process-agenix-key-files)
                  (mapcar
                   #'file-exists-p
                   (list first second missing)))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (("[ORACLE-SANDBOX]/agenix-keys/first-key" "[ORACLE-SANDBOX]/agenix-keys/second-key" "" "[ORACLE-SANDBOX]" "[ORACLE-SANDBOX]/agenix-keys/first-key") (t t nil))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_mode_gate_only_delegates_for_files_below_a_real_secrets_manifest() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agenix-mode-gate"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (inside
                 (expand-file-name "nested/inside.age" root))
                (outside
                 (expand-file-name
                  "outside.age"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                calls)
         (unwind-protect
             (progn
               (make-directory
                (file-name-directory inside) t)
               (write-region
                "{}" nil
                (expand-file-name "secrets.nix" root)
                nil 'silent)
               (cl-letf (((symbol-function 'agenix-mode)
                          (lambda ()
                            (push
                             (list
                              (buffer-file-name)
                              major-mode)
                             calls)
                            'entered)))
                 (list
                  (with-temp-buffer
                    (setq buffer-file-name inside)
                    (agenix-mode-if-with-secrets-nix))
                  (with-temp-buffer
                    (setq buffer-file-name outside)
                    (agenix-mode-if-with-secrets-nix))
                  (nreverse calls))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK (entered nil (("[ORACLE-SANDBOX]/agenix-mode-gate/nested/inside.age" fundamental-mode)))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}

#[test]
fn agenix_secret_manifest_helpers_handle_nil_root_directory_and_manifest_itself() {
    let elisp_form = r##"(let* ((root
                 (expand-file-name
                  "agenix-helper-boundaries"
                  (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
                (manifest
                 (expand-file-name "secrets.nix" root)))
         (unwind-protect
             (progn
               (make-directory root t)
               (write-region "{}" nil manifest nil 'silent)
               (list
                (agenix-secrets-base-dir root)
                (agenix-locate-secrets-nix root)
                (agenix-path-relative-to-secrets-nix
                 root)
                (agenix-secrets-base-dir manifest)
                (agenix-locate-secrets-nix manifest)
                (agenix-path-relative-to-secrets-nix
                 manifest)
                (condition-case error-data
                    (agenix-secrets-base-dir nil)
                  (error
                   (list
                    (car error-data)
                    (cadr error-data))))))
           (when (file-directory-p root)
             (delete-directory root t))))"##;
    let expect = expect![[
        r#"OK ("[ORACLE-SANDBOX]/agenix-helper-boundaries/" "[ORACLE-SANDBOX]/agenix-helper-boundaries/secrets.nix" "." "[ORACLE-SANDBOX]/agenix-helper-boundaries/" "[ORACLE-SANDBOX]/agenix-helper-boundaries/secrets.nix" "secrets.nix" (wrong-type-argument stringp))"#
    ]];
    assert_agenix_parity(elisp_form, expect);
}
