use expect_test::expect;

use super::assert_ansible_vault_parity;

#[test]
fn subcommand_classification_and_legacy_password_file_flags_cover_complete_cli_surface() {
    let elisp_form = r##"(let ((ansible-vault--header-version
                          "1.1")
                         (ansible-vault--header-vault-id
                          nil)
                         (ansible-vault--vault-id
                          nil)
                         (ansible-vault--password-file
                          "/keys/team secret"))
                     (mapcar
                      (lambda (command)
                        (list
                         command
                         (ansible-vault--sub-command-type
                          command)
                         (ansible-vault--command-flags
                          command)
                         (ansible-vault--shell-command
                          command)))
                      '("create"
                        "decrypt"
                        "edit"
                        "view"
                        "encrypt"
                        "encrypt_string"
                        "rekey"
                        "unknown")))"##;
    let expect = expect![[
        r#"OK (("create" :encrypt ("--output=-" "--vault-password-file=\"/keys/team secret\"") "ansible-vault create --output=- --vault-password-file=\"/keys/team secret\"") ("decrypt" :decrypt ("--output=-" "--vault-password-file=\"/keys/team secret\"") "ansible-vault decrypt --output=- --vault-password-file=\"/keys/team secret\"") ("edit" :encrypt ("--output=-" "--vault-password-file=\"/keys/team secret\"") "ansible-vault edit --output=- --vault-password-file=\"/keys/team secret\"") ("view" :decrypt ("--output=-" "--vault-password-file=\"/keys/team secret\"") "ansible-vault view --output=- --vault-password-file=\"/keys/team secret\"") ("encrypt" :encrypt ("--output=-" "--vault-password-file=\"/keys/team secret\"") "ansible-vault encrypt --output=- --vault-password-file=\"/keys/team secret\"") ("encrypt_string" :encrypt ("--output=-" "--vault-password-file=\"/keys/team secret\"") "ansible-vault encrypt_string --output=- --vault-password-file=\"/keys/team secret\"") ("rekey" :unimplemented ("--output=-" "--vault-password-file=\"/keys/team secret\"") "ansible-vault rekey --output=- --vault-password-file=\"/keys/team secret\"") ("unknown" nil ("--output=-" "--vault-password-file=\"/keys/team secret\"") "ansible-vault unknown --output=- --vault-password-file=\"/keys/team secret\""))"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn vault_id_flags_select_matching_secret_and_encrypt_target_for_v12_workflows() {
    let elisp_form = r##"(let ((ansible-vault-command
                          "/opt/ansible vault")
                         (ansible-vault-vault-id-alist
                          '(("prod"
                             . "/keys/prod secret")
                            ("dev"
                             . "/keys/dev")))
                         (ansible-vault--header-version
                          "1.2")
                         (ansible-vault--header-vault-id
                          "prod")
                         (ansible-vault--vault-id
                          "prod"))
                     (list
                      (ansible-vault--command-flags
                       "encrypt")
                      (ansible-vault--shell-command
                       "encrypt")
                      (ansible-vault--command-flags
                       "decrypt")
                      (condition-case error
                          (ansible-vault--shell-command
                           "decrypt")
                        (error
                         (list
                          (car error)
                          (cadr error))))))"##;
    let expect = expect![[
        r#"OK (("--output=-" "--vault-id=\"prod@/keys/prod secret\"" "--encrypt-vault-id=\"prod\"") "/opt/ansible vault encrypt --output=- --vault-id=\"prod@/keys/prod secret\" --encrypt-vault-id=\"prod\"" ("--output=-" "--vault-id=\"prod@/keys/prod secret\"" nil) "/opt/ansible vault decrypt --output=- --vault-id=\"prod@/keys/prod secret\" ")"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn config_discovery_honors_environment_then_project_precedence_and_parses_values() {
    let elisp_form = r##"(let* ((root
                           (make-temp-file
                            "ansible-vault-config-"
                            t))
                          (project
                           (expand-file-name
                            "project/nested"
                            root))
                          (environment-config
                           (expand-file-name
                            "environment.cfg"
                            root))
                          (project-config
                           (expand-file-name
                            "project/ansible.cfg"
                            root))
                          (buffer-file-name
                           (expand-file-name
                            "vault.yml"
                            project))
                          (old-config
                           (getenv
                            "ANSIBLE_CONFIG")))
                     (unwind-protect
                         (progn
                           (make-directory
                            project t)
                           (with-temp-file
                               environment-config
                             (insert
                              "[defaults]\n"
                              "vault_password_file = /keys/from-env ; comment\n"))
                           (with-temp-file
                               project-config
                             (insert
                              "[defaults]\n"
                              "vault_password_file=/keys/from-project\n"))
                           (setenv
                            "ANSIBLE_CONFIG"
                            environment-config)
                           (let ((environment
                                  (ansible-vault--process-config-files)))
                             (setenv
                              "ANSIBLE_CONFIG"
                              nil)
                             (list
                              environment
                              (ansible-vault--process-config-files))))
                       (setenv
                        "ANSIBLE_CONFIG"
                        old-config)
                       (delete-directory
                        root t)))"##;
    let expect = expect![[r#"OK ("/keys/from-env" "/keys/from-project")"#]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn config_parser_reports_missing_or_irrelevant_configuration_without_external_state() {
    let elisp_form = r##"(let* ((root
                           (make-temp-file
                            "ansible-vault-empty-config-"
                            t))
                          (buffer-file-name
                           (expand-file-name
                            "nested/vault.yml"
                            root))
                          (config
                           (expand-file-name
                            "ansible.cfg"
                            root))
                          (old-config
                           (getenv
                            "ANSIBLE_CONFIG")))
                     (unwind-protect
                         (progn
                           (make-directory
                            (file-name-directory
                             buffer-file-name)
                            t)
                           (setenv
                            "ANSIBLE_CONFIG"
                            nil)
                           (let ((missing
                                  (ansible-vault--process-config-files)))
                             (with-temp-file config
                               (insert
                                "[defaults]\n"
                                "inventory = hosts.ini\n"))
                             (list
                              missing
                              (ansible-vault--process-config-files))))
                       (setenv
                        "ANSIBLE_CONFIG"
                        old-config)
                       (delete-directory
                        root t)))"##;
    let expect = expect!["OK (nil nil)"];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn password_guessing_exercises_vault_mapping_environment_config_default_and_prompt_fallbacks() {
    let elisp_form = r##"(let* ((root
                           (make-temp-file
                            "ansible-vault-guess-"
                            t))
                          (mapped
                           (expand-file-name
                            "mapped.pass"
                            root))
                          (environment
                           (expand-file-name
                            "environment.pass"
                            root))
                          (configured
                           (expand-file-name
                            "configured.pass"
                            root))
                          (default
                           (expand-file-name
                            "default.pass"
                            root))
                          (missing
                           (expand-file-name
                            "missing.pass"
                            root))
                          (ansible-vault-vault-id-alist
                           `(("prod" . ,mapped)))
                          (ansible-vault-password-file
                           default)
                          (old-environment
                           (getenv
                            "ANSIBLE_VAULT_PASSWORD_FILE"))
                          (events nil))
                     (unwind-protect
                         (progn
                           (dolist
                               (file
                                (list
                                 mapped
                                 environment
                                 configured
                                 default))
                             (with-temp-file file
                               (insert "secret")))
                           (cl-letf
                               (((symbol-function
                                  'ansible-vault--process-config-files)
                                 (lambda ()
                                   configured))
                                ((symbol-function
                                  'ansible-vault--request-vault-id)
                                 (lambda
                                   (vault-id
                                    &optional _password-file)
                                   (push
                                    (list
                                     'vault-id
                                     vault-id)
                                    events)
                                   (setq-local
                                    ansible-vault--password-file
                                    mapped)))
                                ((symbol-function
                                  'ansible-vault--request-password)
                                 (lambda
                                   (&optional _password)
                                   (interactive)
                                   (push
                                    '(password)
                                    events)
                                   (setq-local
                                    ansible-vault--password-file
                                    default))))
                             (list
                              (with-temp-buffer
                                (setq-local
                                 ansible-vault--header-vault-id
                                 "prod")
                                (eq
                                 (ansible-vault--guess-password-file)
                                 mapped))
                              (with-temp-buffer
                                (setq-local
                                 ansible-vault--header-vault-id
                                 "unknown")
                                (eq
                                 (ansible-vault--guess-password-file)
                                 mapped))
                              (progn
                                (setenv
                                 "ANSIBLE_VAULT_PASSWORD_FILE"
                                 environment)
                                (with-temp-buffer
                                  (eq
                                   (ansible-vault--guess-password-file)
                                   environment)))
                              (progn
                                (setenv
                                 "ANSIBLE_VAULT_PASSWORD_FILE"
                                 nil)
                                (with-temp-buffer
                                  (eq
                                   (ansible-vault--guess-password-file)
                                   configured)))
                              (cl-letf
                                  (((symbol-function
                                     'ansible-vault--process-config-files)
                                    (lambda ()
                                      nil)))
                                (with-temp-buffer
                                  (eq
                                   (ansible-vault--guess-password-file)
                                   default)))
                              (let ((ansible-vault-password-file
                                     missing))
                                (cl-letf
                                    (((symbol-function
                                       'ansible-vault--process-config-files)
                                      (lambda ()
                                        nil)))
                                  (with-temp-buffer
                                    (eq
                                     (ansible-vault--guess-password-file)
                                     default))))
                              (nreverse events))))
                       (setenv
                        "ANSIBLE_VAULT_PASSWORD_FILE"
                        old-environment)
                       (delete-directory
                        root t)))"##;
    let expect = expect![[r#"OK (t t nil t t t ((vault-id "unknown") (password)))"#]];
    assert_ansible_vault_parity(elisp_form, expect);
}
