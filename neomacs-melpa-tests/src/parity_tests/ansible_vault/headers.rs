use expect_test::expect;

use super::assert_ansible_vault_parity;

#[test]
fn fingerprint_parses_supported_real_world_header_versions_ciphers_and_vault_ids() {
    let elisp_form = r##"(mapcar
                      (lambda (header)
                        (with-temp-buffer
                          (insert header "\n616263\n")
                          (ansible-vault--fingerprint-buffer)
                          (list
                           ansible-vault--header-version
                           ansible-vault--header-cipher-algorithm
                           ansible-vault--header-vault-id)))
                      '("$ANSIBLE_VAULT;1.0;AES"
                        "$ANSIBLE_VAULT;1.1;AES256"
                        "$ANSIBLE_VAULT;1.2;AES256;production"
                        "$ANSIBLE_VAULT;1.2;AES;team-west"))"##;
    let expect = expect![[
        r#"OK (("1.0" "AES" nil) ("1.1" "AES256" nil) ("1.2" "AES256" "production") ("1.2" "AES" "team-west"))"#
    ]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn fingerprint_uses_only_first_line_and_preserves_prior_state_for_non_vault_content() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "$ANSIBLE_VAULT;1.2;AES256;prod\n"
                       "616263\n")
                      (ansible-vault--fingerprint-buffer)
                      (let ((valid
                             (list
                              ansible-vault--header-version
                              ansible-vault--header-cipher-algorithm
                              ansible-vault--header-vault-id)))
                        (erase-buffer)
                        (insert
                         "plain: true\n"
                         "$ANSIBLE_VAULT;1.1;AES256\n")
                        (ansible-vault--fingerprint-buffer)
                        (list
                         valid
                         (list
                          ansible-vault--header-version
                          ansible-vault--header-cipher-algorithm
                          ansible-vault--header-vault-id))))"##;
    let expect = expect![[r#"OK (("1.2" "AES256" "prod") ("1.2" "AES256" "prod"))"#]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn encrypted_file_detection_accepts_exact_supported_headers_and_rejects_near_misses() {
    let elisp_form = r##"(mapcar
                      (lambda (text)
                        (with-temp-buffer
                          (insert text)
                          (ansible-vault--is-encrypted-vault-file)))
                      '("$ANSIBLE_VAULT;1.0;AES\n"
                        "$ANSIBLE_VAULT;1.1;AES256\npayload"
                        "$ANSIBLE_VAULT;1.2;AES256;prod\npayload"
                        "$ANSIBLE_VAULT;1.3;AES256\n"
                        "$ANSIBLE_VAULT;1.2;CHACHA\n"
                        " $ANSIBLE_VAULT;1.2;AES256\n"
                        "plain\n$ANSIBLE_VAULT;1.2;AES256\n"
                        ""))"##;
    let expect = expect!["OK (t t t nil nil nil nil nil)"];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn encrypted_file_detection_respects_narrowed_buffer_boundaries() {
    let elisp_form = r##"(with-temp-buffer
                      (insert
                       "plain preface\n"
                       "$ANSIBLE_VAULT;1.2;AES256;prod\n"
                       "616263\n")
                      (let ((whole
                             (ansible-vault--is-encrypted-vault-file)))
                        (goto-char
                         (point-min))
                        (forward-line 1)
                        (narrow-to-region
                         (point)
                         (point-max))
                        (list
                         whole
                         (ansible-vault--is-encrypted-vault-file)
                         (point-min)
                         (buffer-substring-no-properties
                          (point-min)
                          (line-end-position)))))"##;
    let expect = expect![[r#"OK (nil t 15 "$ANSIBLE_VAULT;1.2;AES256;prod")"#]];
    assert_ansible_vault_parity(elisp_form, expect);
}

#[test]
fn parsed_vault_state_is_buffer_local_and_survives_major_mode_changes() {
    let elisp_form = r##"(let ((one
                          (generate-new-buffer
                           " *vault-one*"))
                         (two
                          (generate-new-buffer
                           " *vault-two*")))
                     (unwind-protect
                         (progn
                           (with-current-buffer one
                             (insert
                              "$ANSIBLE_VAULT;1.2;AES256;prod\n")
                             (ansible-vault--fingerprint-buffer)
                             (fundamental-mode))
                           (with-current-buffer two
                             (insert
                              "$ANSIBLE_VAULT;1.1;AES\n")
                             (ansible-vault--fingerprint-buffer)
                             (text-mode))
                           (list
                            (with-current-buffer one
                              (list
                               ansible-vault--header-version
                               ansible-vault--header-cipher-algorithm
                               ansible-vault--header-vault-id))
                            (with-current-buffer two
                              (list
                               ansible-vault--header-version
                               ansible-vault--header-cipher-algorithm
                               ansible-vault--header-vault-id))
                            (list
                             (default-value
                              'ansible-vault--header-version)
                             (default-value
                              'ansible-vault--header-vault-id))))
                       (kill-buffer one)
                       (kill-buffer two)))"##;
    let expect = expect![[r#"OK (("1.2" "AES256" "prod") ("1.1" "AES" nil) (nil nil))"#]];
    assert_ansible_vault_parity(elisp_form, expect);
}
