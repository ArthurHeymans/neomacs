use expect_test::expect;

use super::assert_aes_parity;

#[test]
fn aes_complete_callable_surface_is_loaded_with_command_and_macro_classification() {
    let elisp_form = r##"(mapcar
              (lambda (symbol)
                (list
                 symbol
                 (fboundp symbol)
                 (commandp symbol)
                 (macrop symbol)))
              '(aes--xor
                aes--xor-de
                aes--xor-4
                aes--xor-4-de
                aes-pad
                aes-zero-pad
                aes--enlarge-to-multiple-num
                aes-pkcs7-pad
                aes--str-to-b
                aes-SubBytes
                aes-InvSubBytes
                aes-SubWord
                aes-ShiftRows
                aes-InvShiftRows
                aes-SubShiftMixKeys
                aes-InvSubShiftMixKeys
                aes--RotWord
                aes-KeyExpansion
                aes-AddRoundKey
                aes-InvAddRoundKey
                aes-Cipher
                aes-InvCipher
                aes-cbc-encrypt
                aes-cbc-decrypt
                aes--ocb-double-de
                aes--ocb-triple-de
                aes-num2str
                aes-ocb-pmac
                aes-ocb-encrypt
                aes-ocb-decrypt
                aes-clear-plaintext-keys
                aes-idle-clear-plaintext-keys
                aes-exec-passws-hooks
                aes-key-from-passwd
                aes-password-to-key
                aes--fisher-yates-shuffle-array
                aes-user-entropy
                aes-generate-password
                aes-insert-password
                aes--toggle-representation
                aes-encrypt-buffer-or-string
                aes-decrypt-buffer-or-string
                aes-is-encrypted
                aes--encrypt-current-buffer-check
                aes--restore-buffer-from-temp-var
                aes-encrypt-current-buffer
                aes-decrypt-current-buffer
                aes-toggle-encryption
                aes-remove-encryption-hook
                aes-auto-decrypt
                aes-enable-auto-decryption
                aes-disable-auto-decryption))"##;
    let expect = expect![
        "OK ((aes--xor t nil nil) (aes--xor-de t nil nil) (aes--xor-4 t nil nil) (aes--xor-4-de t nil nil) (aes-pad t nil nil) (aes-zero-pad t nil nil) (aes--enlarge-to-multiple-num t nil nil) (aes-pkcs7-pad t nil nil) (aes--str-to-b t nil nil) (aes-SubBytes t nil nil) (aes-InvSubBytes t nil nil) (aes-SubWord t nil nil) (aes-ShiftRows t nil nil) (aes-InvShiftRows t nil nil) (aes-SubShiftMixKeys t nil nil) (aes-InvSubShiftMixKeys t nil nil) (aes--RotWord t nil nil) (aes-KeyExpansion t nil nil) (aes-AddRoundKey t nil nil) (aes-InvAddRoundKey t nil nil) (aes-Cipher t nil nil) (aes-InvCipher t nil nil) (aes-cbc-encrypt t nil nil) (aes-cbc-decrypt t nil nil) (aes--ocb-double-de t nil nil) (aes--ocb-triple-de t nil nil) (aes-num2str t nil nil) (aes-ocb-pmac t nil nil) (aes-ocb-encrypt t nil nil) (aes-ocb-decrypt t nil nil) (aes-clear-plaintext-keys t t nil) (aes-idle-clear-plaintext-keys t nil nil) (aes-exec-passws-hooks t nil nil) (aes-key-from-passwd t nil nil) (aes-password-to-key t nil nil) (aes--fisher-yates-shuffle-array t nil nil) (aes-user-entropy t nil nil) (aes-generate-password t nil nil) (aes-insert-password t t nil) (aes--toggle-representation t nil nil) (aes-encrypt-buffer-or-string t nil nil) (aes-decrypt-buffer-or-string t nil nil) (aes-is-encrypted t nil nil) (aes--encrypt-current-buffer-check t nil nil) (aes--restore-buffer-from-temp-var t nil nil) (aes-encrypt-current-buffer t t nil) (aes-decrypt-current-buffer t t nil) (aes-toggle-encryption t t nil) (aes-remove-encryption-hook t t nil) (aes-auto-decrypt t nil nil) (aes-enable-auto-decryption t nil nil) (aes-disable-auto-decryption t nil nil))"
    ];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_configuration_defaults_and_internal_mutable_state_match_the_frozen_pin() {
    let elisp_form = r##"(list
              (featurep 'aes)
              aes-always-ask-for-passwords
              aes-enable-plaintext-password-storage
              aes--plaintext-passwords
              aes-idle-timer-value
              aes-delete-passwords-after-idle
              aes-path-passwd-hook
              aes-password-char-groups
              aes-user-interaction-entropy
              aes-entropy-of-mousemovement
              aes-entropy-of-keyinput
              aes-discard-undo-after-encryption
              aes-default-method
              aes-Nb
              aes-Nk
              aes--save-temp-buffer
              (get 'aes 'custom-group)
              (get 'aes 'group-documentation))"##;
    let expect = expect![[
        r#"OK (t t nil nil nil 1 nil ((97 t "abcdefghjkmnopqrstuvwxyz") (65 t "ABCDEFGHJKLMNPQRSTUVWXYZ") (53 t "23456789") (48 t "0OilI1") (46 nil ",.!?;:_()[]{}<>") (43 nil "-+*/=") (37 nil "|^~#$%&'")) t 4 2 t "OCB" 4 4 nil ((aes-always-ask-for-passwords custom-variable) (aes-enable-plaintext-password-storage custom-variable) (aes-delete-passwords-after-idle custom-variable) (aes-password-char-groups custom-variable) (aes-user-interaction-entropy custom-variable) (aes-entropy-of-mousemovement custom-variable) (aes-entropy-of-keyinput custom-variable) (aes-discard-undo-after-encryption custom-variable) (aes-default-method custom-variable) (aes-Nb custom-variable) (aes-Nk custom-variable)) "Advanced Encryption Standard implementation")"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_custom_variables_retain_standard_values_types_and_group_membership() {
    let elisp_form = r##"(mapcar
              (lambda (symbol)
                (list
                 symbol
                 (get symbol 'standard-value)
                 (get symbol 'custom-type)
                 (get symbol 'custom-requests)
                 (get symbol 'custom-version)))
              '(aes-always-ask-for-passwords
                aes-enable-plaintext-password-storage
                aes-delete-passwords-after-idle
                aes-password-char-groups
                aes-user-interaction-entropy
                aes-entropy-of-mousemovement
                aes-entropy-of-keyinput
                aes-discard-undo-after-encryption
                aes-default-method
                aes-Nb
                aes-Nk))"##;
    let expect = expect![[
        r#"OK ((aes-always-ask-for-passwords ((funcall #'#[nil (t) #1=(t)])) boolean nil nil) (aes-enable-plaintext-password-storage ((funcall #'#[nil (nil) #1#])) boolean nil nil) (aes-delete-passwords-after-idle ((funcall #'#[nil (1) #1#])) integer nil nil) (aes-password-char-groups ((funcall #'#[nil ('((97 t "abcdefghjkmnopqrstuvwxyz") (65 t "ABCDEFGHJKLMNPQRSTUVWXYZ") (53 t "23456789") (48 t "0OilI1") (46 nil ",.!?;:_()[]{}<>") (43 nil "-+*/=") (37 nil "|^~#$%&'"))) #1#])) (repeat (list character (choice (const :tag "active" t) (const :tag "inactive" nil)) string)) nil nil) (aes-user-interaction-entropy ((funcall #'#[nil (t) #1#])) boolean nil nil) (aes-entropy-of-mousemovement ((funcall #'#[nil (4) #1#])) integer nil nil) (aes-entropy-of-keyinput ((funcall #'#[nil (2) #1#])) integer nil nil) (aes-discard-undo-after-encryption ((funcall #'#[nil (t) #1#])) boolean nil nil) (aes-default-method ((funcall #'#[nil ("OCB") #1#])) (choice (const "OCB") (const "CBC")) nil nil) (aes-Nb ((funcall #'#[nil (4) #1#])) integer nil nil) (aes-Nk ((funcall #'#[nil (4) #1#])) integer nil nil))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}
