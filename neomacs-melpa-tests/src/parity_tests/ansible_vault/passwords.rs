use expect_test::expect;

use super::assert_ansible_vault_parity;

#[test]
fn generated_password_file_has_private_mode_exact_contents_and_workspace_local_lifecycle() {
    let elisp_form = r##"(let ((ansible-vault--password-file-list
                         nil))
                     (with-temp-buffer
                       (let* ((path
                               (ansible-vault--create-password-file
                                "correct horse\nbattery staple"))
                              (snapshot
                               (list
                                (file-exists-p path)
                                (file-readable-p path)
                                (file-writable-p path)
                                (file-modes path)
                                (with-temp-buffer
                                  (insert-file-contents
                                   path)
                                  (buffer-string))
                                (equal
                                 (file-name-directory
                                  path)
                                 (file-name-as-directory
                                  temporary-file-directory))
                                (equal
                                 ansible-vault--password-file
                                 path)
                                (equal
                                 ansible-vault--password-file-list
                                 (list path)))))
                         (ansible-vault--flush-password-file)
                         (append
                          snapshot
                          (list
                           (file-exists-p path)
                           ansible-vault--password-file
                           ansible-vault--password-file-list)))))"##;
    let expect = expect![[r#"OK (t t nil 256 "correct horse\nbattery staple" t t t nil nil nil)"#]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn vault_id_request_reuses_registered_secret_or_registers_explicit_new_secret() {
    let elisp_form = r##"(let ((ansible-vault-vault-id-alist
                         '(("prod"
                            . "/keys/prod")))
                        (events nil))
                     (cl-letf
                         (((symbol-function
                            'ansible-vault--request-password)
                           (lambda
                             (&optional _password)
                             (interactive)
                             (push
                              'prompted
                              events)
                             "/keys/prompted")))
                       (list
                        (with-temp-buffer
                          (list
                           (ansible-vault--request-vault-id
                            "prod")
                           ansible-vault--vault-id
                           ansible-vault--password-file))
                        (with-temp-buffer
                          (list
                           (ansible-vault--request-vault-id
                            "stage"
                            "/keys/stage")
                           ansible-vault--vault-id
                           ansible-vault--password-file))
                        ansible-vault-vault-id-alist
                        events)))"##;
    let expect = expect![[
        r#"OK ((#2=("prod" . "/keys/prod") "prod" "/keys/prod") (#1=("stage" . "/keys/stage") "stage" "/keys/stage") (#1# #2#) nil)"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn flushing_vault_state_deletes_generated_secrets_but_preserves_external_files() {
    let elisp_form = r##"(let* ((root
                           (make-temp-file
                            "ansible-vault-flush-"
                            t))
                          (external
                           (expand-file-name
                            "external.pass"
                            root))
                          (ansible-vault-vault-id-alist
                           nil)
                          (ansible-vault--password-file-list
                           nil))
                     (unwind-protect
                         (progn
                           (with-temp-file external
                             (insert "external"))
                           (list
                            (with-temp-buffer
                              (setq-local
                               ansible-vault--vault-id
                               "generated")
                              (let ((generated
                                     (ansible-vault--create-password-file
                                      "generated")))
                                (push
                                 (cons
                                  "generated"
                                  generated)
                                 ansible-vault-vault-id-alist)
                                (ansible-vault--flush-vault-id)
                                (list
                                 (file-exists-p
                                  generated)
                                 ansible-vault--vault-id
                                 ansible-vault--password-file)))
                            (with-temp-buffer
                              (setq-local
                               ansible-vault--vault-id
                               "external")
                              (setq-local
                               ansible-vault--password-file
                               external)
                              (push
                               (cons
                                "external"
                                external)
                               ansible-vault-vault-id-alist)
                              (ansible-vault--flush-vault-id)
                              (list
                               (file-exists-p
                                external)
                               ansible-vault--vault-id
                               ansible-vault--password-file))
                            ansible-vault-vault-id-alist
                            ansible-vault--password-file-list))
                       (delete-directory
                        root t)))"##;
    let expect = expect!["OK ((nil nil nil) (t nil nil) nil nil)"];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn cleanup_and_clear_local_state_reset_failed_session_without_cross_buffer_leaks() {
    let elisp_form = r##"(let ((ansible-vault-vault-id-alist
                         '(("prod"
                            . "/keys/prod")))
                        (ansible-vault--password-file-list
                         nil))
                     (with-temp-buffer
                       (setq-local
                        ansible-vault--header-version
                        "1.2")
                       (setq-local
                        ansible-vault--header-cipher-algorithm
                        "AES256")
                       (setq-local
                        ansible-vault--header-vault-id
                        "prod")
                       (setq-local
                        ansible-vault--point
                        18)
                       (setq-local
                        ansible-vault--password-file
                        "/keys/prod")
                       (setq-local
                        ansible-vault--vault-id
                        "prod")
                       (setq-local
                        ansible-vault--auto-encryption-enabled
                        t)
                       (ansible-vault--cleanup-password-error)
                       (let ((after-cleanup
                              (list
                               ansible-vault--vault-id
                               ansible-vault--password-file
                               ansible-vault-vault-id-alist)))
                         (ansible-vault--clear-local-variables)
                         (list
                          after-cleanup
                          (mapcar
                           (lambda (variable)
                             (list
                              variable
                              (boundp variable)
                              (local-variable-p
                               variable)))
                           '(ansible-vault--header-version
                             ansible-vault--header-cipher-algorithm
                             ansible-vault--header-vault-id
                             ansible-vault--point
                             ansible-vault--password-file
                             ansible-vault--vault-id
                             ansible-vault--auto-encryption-enabled))))))"##;
    let expect = expect![
        "OK ((nil nil nil) ((ansible-vault--header-version nil t) (ansible-vault--header-cipher-algorithm nil t) (ansible-vault--header-vault-id nil t) (ansible-vault--point nil t) (ansible-vault--password-file nil t) (ansible-vault--vault-id nil t) (ansible-vault--auto-encryption-enabled nil t)))"
    ];
    assert_ansible_vault_parity(elisp_form, expect);
}
