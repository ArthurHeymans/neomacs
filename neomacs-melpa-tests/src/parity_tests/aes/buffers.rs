use expect_test::expect;

use super::assert_aes_parity;

#[test]
fn aes_toggle_representation_roundtrips_multibyte_text_and_raw_utf8_bytes() {
    let elisp_form = r##"(let* ((multibyte "λ café")
                    (unibyte
                     (aes--toggle-representation
                      multibyte))
                    (restored
                     (aes--toggle-representation
                      unibyte))
                    (raw
                     (unibyte-string
                      195 169 0 255))
                    (raw-multibyte
                     (aes--toggle-representation raw))
                    (raw-restored
                     (aes--toggle-representation
                      raw-multibyte)))
               (list
                (list
                 (multibyte-string-p multibyte)
                 (string-bytes multibyte)
                 (length multibyte)
                 (string-to-list multibyte))
                (list
                 (multibyte-string-p unibyte)
                 (string-bytes unibyte)
                 (length unibyte)
                 (string-to-list unibyte))
                (list
                 (multibyte-string-p restored)
                 (string-bytes restored)
                 (length restored)
                 restored
                 (equal restored multibyte))
                (list
                 (multibyte-string-p
                  raw-multibyte)
                 (string-to-list raw-multibyte))
                (list
                 (multibyte-string-p
                  raw-restored)
                 (string-to-list raw-restored)
                 (equal raw raw-restored))))"##;
    let expect = expect![[
        r#"OK ((t 8 6 (955 32 99 97 102 233)) (nil 8 8 (206 187 32 99 97 102 195 169)) (t 8 6 "λ café" t) (t (233 0 4194303)) (nil (195 169 0 255) t))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_string_encryption_roundtrips_ocb_cbc_base64_binary_and_multibyte_cases() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (cl-letf
                  (((symbol-function 'aes-user-entropy)
                    (lambda (length &optional border)
                      (mapcar
                       (lambda (index)
                         (% index
                            (or border 256)))
                       (number-sequence
                        0 (1- length))))))
                (mapcar
                 (lambda (spec)
                   (let* ((plain
                           (nth 0 spec))
                          (type
                           (nth 1 spec))
                          (nk
                           (nth 2 spec))
                          (nb
                           (nth 3 spec))
                          (nonb64
                           (nth 4 spec))
                          (encrypted
                           (aes-encrypt-buffer-or-string
                            plain "secret"
                            type nk nb nonb64))
                          (header-end
                           (1+ (string-match
                                "\n" encrypted)))
                          (decrypted
                           (aes-decrypt-buffer-or-string
                            encrypted "secret")))
                     (list
                      (substring
                       encrypted 0 header-end)
                      (length encrypted)
                      (secure-hash
                       'sha256 encrypted)
                      (multibyte-string-p
                       encrypted)
                      (multibyte-string-p
                       decrypted)
                      (string-to-list decrypted)
                      (equal plain decrypted))))
                 (list
                  (list
                   "λ café\n" "OCB" 4 4 nil)
                  (list
                   (unibyte-string
                    0 1 2 127 128 255)
                   "OCB" 4 4 t)
                  (list
                   "CBC text" "CBC" 4 4 nil)
                  (list
                   (apply #'unibyte-string
                          (number-sequence 0 24))
                   "CBC" 8 6 t)))))"##;
    let expect = expect![[
        r#"OK (("aes-encrypted V 1.3-OCB-B-4-4-M\n" 88 "e79af5a4394bb1110af907461b685671819b29495e9604474fe139daba970ce1" nil t (955 32 99 97 102 233 10) t) ("aes-encrypted V 1.3-OCB-N-4-4-U\n" 70 "86fcd47787caae02d89635ea1cdfeb9b75473e6ed4e97f59465fe659b40f439e" nil nil (0 1 2 127 128 255) t) ("aes-encrypted V 1.3-CBC-B-4-4-U\n" 76 "f18c180553791a40c5ffdcc651a6166e8ead371b22caaddcf1027e1272f19122" nil nil (67 66 67 32 116 101 120 116) t) ("aes-encrypted V 1.3-CBC-N-6-8-U\n" 104 "9fdc4e6b54fae74b9ad1c901e4fad6c8513b1b768c23c3bd841047e53ae92087" nil nil (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24) t))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_string_encryption_reports_wrong_method_and_malformed_ciphertext() {
    let elisp_form = r##"(let ((wrong-method
                    (aes-encrypt-buffer-or-string
                     "plain" "secret"
                     "CTR" 4 4 nil))
                   (wrong-message
                    (current-message)))
               (let ((malformed
                      (aes-decrypt-buffer-or-string
                       "ordinary text"
                       "secret"))
                     (malformed-message
                      (current-message)))
                 (list
                  wrong-method
                  wrong-message
                  malformed
                  malformed-message)))"##;
    let expect = expect![[r#"OK ("Wrong type." nil nil nil)"#]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_decryption_rejects_wrong_password_and_tampered_ocb_tag_without_plaintext() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (cl-letf
                  (((symbol-function 'aes-user-entropy)
                    (lambda (length &optional _)
                      (make-list length 7))))
                (let* ((encrypted
                        (aes-encrypt-buffer-or-string
                         "authenticated payload"
                         "correct" "OCB" 4 4 t))
                       (wrong-password
                        (aes-decrypt-buffer-or-string
                         encrypted "wrong"))
                       (wrong-message
                        (current-message))
                       (tampered
                        (copy-sequence encrypted)))
                  (aset tampered
                        (1- (length tampered))
                        (logxor
                         1
                         (aref tampered
                               (1- (length
                                    tampered)))))
                  (let ((tampered-result
                         (aes-decrypt-buffer-or-string
                          tampered "correct"))
                        (tampered-message
                         (current-message)))
                    (list
                     wrong-password
                     (and
                      wrong-message
                      (string-match-p
                       "could not be decrypted"
                       wrong-message))
                     tampered-result
                     (and
                      tampered-message
                      (string-match-p
                       "could not be decrypted"
                       tampered-message)))))))"##;
    let expect = expect!["OK (nil nil nil nil)"];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_buffer_encryption_returns_true_clears_or_preserves_undo_and_restores_text() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (cl-letf
                  (((symbol-function 'aes-user-entropy)
                    (lambda (length &optional _)
                      (make-list length 3))))
                (mapcar
                 (lambda (discard)
                   (with-temp-buffer
                     (buffer-enable-undo)
                     (insert "buffer λ text")
                     (let ((aes-discard-undo-after-encryption
                            discard)
                           (before-multibyte
                            enable-multibyte-characters))
                       (let ((encrypted-result
                              (aes-encrypt-buffer-or-string
                               (current-buffer)
                               "secret"
                               "OCB" 4 4 nil))
                             (encrypted-p
                              (aes-is-encrypted))
                             (undo-empty
                              (null buffer-undo-list)))
                         (let ((decrypted-result
                                (aes-decrypt-buffer-or-string
                                 (current-buffer)
                                 "secret")))
                           (list
                            discard
                            encrypted-result
                            encrypted-p
                            undo-empty
                            decrypted-result
                            (buffer-string)
                            before-multibyte
                            enable-multibyte-characters))))))
                 '(t nil))))"##;
    let expect =
        expect![[r#"OK ((t t t t t "buffer λ text" t t) (nil t t nil t "buffer λ text" t t))"#]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_buffer_name_argument_selects_the_live_buffer_instead_of_encrypting_the_name_string() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (cl-letf
                  (((symbol-function 'aes-user-entropy)
                    (lambda (length &optional _)
                      (make-list length 9))))
                (let ((buffer
                       (generate-new-buffer
                        "aes-named-buffer")))
                  (unwind-protect
                      (with-current-buffer buffer
                        (insert "named contents")
                        (let ((encrypt-result
                               (aes-encrypt-buffer-or-string
                                (buffer-name buffer)
                                "secret"
                                "CBC" 4 4 nil)))
                          (list
                           encrypt-result
                           (aes-is-encrypted)
                           (aes-decrypt-buffer-or-string
                            (buffer-name buffer)
                            "secret")
                           (buffer-string))))
                    (kill-buffer buffer)))))"##;
    let expect = expect![[r#"OK (t t t "named contents")"#]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_current_buffer_commands_delegate_real_encryption_and_decryption() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (cl-letf
                  (((symbol-function 'aes-user-entropy)
                    (lambda (length &optional _)
                      (make-list length 11))))
                (with-temp-buffer
                  (insert "current buffer")
                  (let ((encrypt-result
                         (aes-encrypt-current-buffer
                          "secret")))
                    (list
                     encrypt-result
                     (aes-is-encrypted)
                     (aes-decrypt-current-buffer
                      "secret")
                     (buffer-string))))))"##;
    let expect = expect![[r#"OK (t t t "current buffer")"#]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_is_encrypted_only_recognizes_matching_first_line_at_buffer_start() {
    let elisp_form = r##"(mapcar
              (lambda (contents)
                (with-temp-buffer
                  (insert contents)
                  (aes-is-encrypted)))
              '("aes-encrypted V 1.3-OCB-B-4-4-M\npayload"
                "aes-encrypted V 99.42-anything\npayload"
                "prefix\naes-encrypted V 1.3-CBC-B-4-4-U\n"
                "aes-encrypted V 1x3-OCB-B-4-4-M\n"
                "ordinary"))"##;
    let expect = expect!["OK (t t nil t nil)"];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_encrypt_current_buffer_check_skips_ciphertext_and_installs_restore_hook_for_plaintext() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (let ((aes--save-temp-buffer
                     'unchanged)
                    (after-save-hook nil)
                    events)
                (cl-letf
                    (((symbol-function
                       'aes-encrypt-buffer-or-string)
                      (lambda (buffer &rest _)
                        (push
                         (list
                          'encrypt
                          (buffer-name buffer)
                          (point))
                         events)
                        (erase-buffer)
                        (insert
                         "aes-encrypted V 1.3-OCB-B-4-4-M\nstub")
                        t)))
                  (let ((encrypted-case
                         (with-temp-buffer
                           (insert
                            "aes-encrypted V 1.3-OCB-B-4-4-M\nexisting")
                           (list
                            (aes--encrypt-current-buffer-check)
                            aes--save-temp-buffer
                            after-save-hook
                            (buffer-string)))))
                    (setq aes--save-temp-buffer nil
                          after-save-hook nil)
                    (let ((plain-case
                           (with-temp-buffer
                             (insert "plain")
                             (goto-char 3)
                             (list
                              (aes--encrypt-current-buffer-check)
                              aes--save-temp-buffer
                              (memq
                               'aes--restore-buffer-from-temp-var
                               after-save-hook)
                              (buffer-string)))))
                      (list
                       encrypted-case
                       plain-case
                       (reverse events)))))))"##;
    let expect = expect![[
        r#"OK ((nil unchanged nil "aes-encrypted V 1.3-OCB-B-4-4-M\nexisting") (nil 3 (aes--restore-buffer-from-temp-var) "aes-encrypted V 1.3-OCB-B-4-4-M\nstub") ((encrypt " *temp*" 3)))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_restore_buffer_removes_hook_restores_point_and_clears_modified_state() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (with-temp-buffer
                (insert "encrypted placeholder")
                (setq aes--save-temp-buffer 4)
                (add-hook
                 'after-save-hook
                 'aes--restore-buffer-from-temp-var)
                (set-buffer-modified-p t)
                (cl-letf
                    (((symbol-function
                       'aes-decrypt-current-buffer)
                      (lambda (&optional _)
                        (erase-buffer)
                        (insert "restored plaintext")
                        t)))
                  (let ((result
                         (aes--restore-buffer-from-temp-var)))
                    (list
                     result
                     (buffer-string)
                     (point)
                     aes--save-temp-buffer
                     (memq
                      'aes--restore-buffer-from-temp-var
                      after-save-hook)
                     (buffer-modified-p))))))"##;
    let expect = expect![[r#"OK (nil "restored plaintext" 4 nil nil nil)"#]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_toggle_encryption_roundtrips_and_preserves_point_modification_and_local_save_hook() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (let (prompts)
                (cl-letf
                    (((symbol-function 'aes-user-entropy)
                      (lambda (length &optional _)
                        (make-list length 13)))
                     ((symbol-function 'read-passwd)
                      (lambda (prompt &optional confirm)
                        (push
                         (list prompt confirm)
                         prompts)
                        "secret")))
                  (with-temp-buffer
                    (insert "toggle plaintext")
                    (goto-char 5)
                    (set-buffer-modified-p nil)
                    (let ((encrypt-result
                           (aes-toggle-encryption))
                          (after-encrypt-point
                           (point))
                          (encrypted
                           (aes-is-encrypted)))
                      (set-buffer-modified-p nil)
                      (let ((decrypt-result
                             (aes-toggle-encryption)))
                        (list
                         encrypt-result
                         after-encrypt-point
                         encrypted
                         decrypt-result
                         (buffer-string)
                         (point)
                         (buffer-modified-p)
                         (memq
                          'aes--encrypt-current-buffer-check
                          write-file-functions)
                         (reverse prompts))))))))"##;
    let expect = expect![[
        r#"OK (5 5 t 5 "toggle plaintext" 5 nil (aes--encrypt-current-buffer-check t) (("encryption Password for  *temp*: " t) ("decryption Password for  *temp*: " nil)))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_remove_encryption_hook_only_removes_the_buffer_local_save_function() {
    let elisp_form = r##"(with-temp-buffer
              (add-hook
               'write-file-functions
               'aes--encrypt-current-buffer-check
               nil t)
              (let ((before
                     (memq
                      'aes--encrypt-current-buffer-check
                      write-file-functions))
                    (result
                     (aes-remove-encryption-hook)))
                (list
                 (not (null before))
                 result
                 (memq
                  'aes--encrypt-current-buffer-check
                  write-file-functions)
                 (current-message))))"##;
    let expect = expect![[r#"OK (t "Encryption Hook removed." nil nil)"#]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_auto_decrypt_handles_ciphertext_and_plain_buffers_with_format_contract_results() {
    let elisp_form = r##"(progn
              (require 'cl-lib)
              (let (prompts)
                (cl-letf
                    (((symbol-function 'aes-user-entropy)
                      (lambda (length &optional _)
                        (make-list length 15)))
                     ((symbol-function 'read-passwd)
                      (lambda (prompt &optional confirm)
                        (push
                         (list prompt confirm)
                         prompts)
                        "secret")))
                  (let ((encrypted-case
                         (with-temp-buffer
                           (insert "auto plaintext")
                           (aes-encrypt-current-buffer
                            "secret")
                           (set-buffer-modified-p nil)
                           (goto-char (point-max))
                           (let ((result
                                  (aes-auto-decrypt
                                   2 9)))
                             (list
                              result
                              (buffer-string)
                              (point)
                              auto-save-default
                              (buffer-modified-p)
                              (memq
                               'aes--encrypt-current-buffer-check
                               write-file-functions)))))
                        (plain-case
                         (with-temp-buffer
                           (insert "plain")
                           (goto-char (point-max))
                           (list
                            (aes-auto-decrypt
                             'ignored)
                            (point)
                            (buffer-string)
                            auto-save-default))))
                    (list
                     encrypted-case
                     plain-case
                     (reverse prompts))))))"##;
    let expect = expect![[
        r#"OK ((15 "auto plaintext" 1 nil nil (aes--encrypt-current-buffer-check t)) (6 1 "plain" t) (("decryption Password for  *temp*: " nil)))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_auto_decryption_registration_replaces_duplicates_and_disable_is_idempotent() {
    let elisp_form = r##"(let ((format-alist
                    '((other
                       "Other" "other" nil nil))))
               (let ((first
                      (aes-enable-auto-decryption))
                     (after-first
                      (copy-tree format-alist)))
                 (aes-enable-auto-decryption)
                 (let ((after-second
                        (copy-tree format-alist))
                       (disable-result
                        (aes-disable-auto-decryption)))
                   (let ((after-disable
                          (copy-tree format-alist))
                         (second-disable
                          (aes-disable-auto-decryption)))
                     (list
                      first
                      after-first
                      after-second
                      disable-result
                      after-disable
                      second-disable
                      format-alist)))))"##;
    let expect = expect![[
        r#"OK (((aes "AES-encrypted format" "aes-encrypted V [0-9]+.[0-9]+-.+\n" aes-auto-decrypt nil t nil) . #1=((other "Other" "other" nil nil))) ((aes "AES-encrypted format" "aes-encrypted V [0-9]+.[0-9]+-.+\n" aes-auto-decrypt nil t nil) (other "Other" "other" nil nil)) ((aes "AES-encrypted format" "aes-encrypted V [0-9]+.[0-9]+-.+\n" aes-auto-decrypt nil t nil) (other "Other" "other" nil nil)) #1# ((other "Other" "other" nil nil)) nil #1#)"#
    ]];

    assert_aes_parity(elisp_form, expect);
}
