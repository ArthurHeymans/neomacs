use expect_test::expect;

use super::assert_aes_parity;

/// The published known-answer test.  NIST FIPS-197 appendix C.1 fixes the
/// AES-128 output for one key and one block, so this pins the cipher core to an
/// external authority rather than to whatever the package happens to produce:
/// encrypting the vector must yield exactly 69c4e0d86a7b0430d8cdb78070b4c55a,
/// decrypting must give the input back, and the plaintext must be left alone.
#[test]
fn the_fips_197_known_answer_vector_encrypts_and_decrypts_exactly() {
    let elisp_form = r##"(let* ((key (aes-test-unhex "000102030405060708090a0b0c0d0e0f"))
       (plain (aes-test-unhex "00112233445566778899aabbccddeeff"))
       (keys (aes-KeyExpansion (aes--str-to-b key) 4))
       (cipher (aes-Cipher plain keys 4))
       (back (aes-InvCipher cipher
                            (nreverse (aes-KeyExpansion (aes--str-to-b key) 4))
                            4)))
  (list :cipher (aes-test-hex cipher)
        :expected "69c4e0d86a7b0430d8cdb78070b4c55a"
        :matches (string= (aes-test-hex cipher) "69c4e0d86a7b0430d8cdb78070b4c55a")
        :roundtrip (aes-test-hex back)
        :plain-unchanged (aes-test-hex plain)))"##;

    let expect = expect![[
        r#"OK (:cipher "69c4e0d86a7b0430d8cdb78070b4c55a" :expected "69c4e0d86a7b0430d8cdb78070b4c55a" :matches t :roundtrip "00112233445566778899aabbccddeeff" :plain-unchanged "00112233445566778899aabbccddeeff")"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

/// The everyday workflow: visit a real file holding Unicode, encrypt the buffer
/// with `aes-encrypt-current-buffer`, save it, then reopen and decrypt.  The
/// saved file must be pure ASCII, because the default encoding is base64, and
/// the decrypted text must equal the original byte for byte -- multibyte
/// content survives the unibyte conversion the package does internally.
#[test]
fn a_real_file_round_trips_through_the_buffer_commands() {
    let elisp_form = r##"(let* ((path (aes-test-path "notes/geheim.txt"))
       (plaintext "Grüße, Welt!\nZeile zwei — mit Unicode.\n"))
  (make-directory (file-name-directory path) t)
  (with-temp-buffer (insert plaintext)
    (write-region (point-min) (point-max) path nil 'silent))
  (let ((buffer (find-file-noselect path)) encrypted-text on-disk)
    (unwind-protect
        (progn
          (with-current-buffer buffer
            (aes-encrypt-current-buffer "geheimes Passwort")
            (setq encrypted-text (buffer-substring-no-properties (point-min) (point-max)))
            (save-buffer))
          (setq on-disk (aes-test-bytes path))
          (let ((all-ascii (cl-every (lambda (b) (< b 128)) on-disk)))
            (kill-buffer buffer)
            (setq buffer (find-file-noselect path))
            (with-current-buffer buffer
              (aes-decrypt-current-buffer "geheimes Passwort")
              (list :header (aes-test-header encrypted-text)
                    :encrypted-all-ascii all-ascii
                    :encrypted-length (length on-disk)
                    :decrypted (buffer-substring-no-properties (point-min) (point-max))
                    :roundtrip-exact (string= plaintext
                                              (buffer-substring-no-properties
                                               (point-min) (point-max)))))))
      (when (buffer-live-p buffer)
        (with-current-buffer buffer (set-buffer-modified-p nil))
        (kill-buffer buffer)))))"##;

    let expect = expect![[
        r#"OK (:header "aes-encrypted V 1.3-OCB-B-4-4-M" :encrypted-all-ascii t :encrypted-length 134 :decrypted "Grüße, Welt!\nZeile zwei — mit Unicode.\n" :roundtrip-exact t)"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

/// Asked for raw output the package emits real high bytes, so how those bytes
/// reach the disk decides whether the ciphertext survives at all.  Written
/// through `binary` they come back identical and still decrypt.  The second
/// half writes a *fixed* cipher block -- the FIPS-197 output, so the bytes are
/// deterministic -- through `japanese-shift-jis` and `latin-1` as well, and
/// pins all three byte sequences: a coding conversion silently destroys
/// ciphertext, and this records exactly how.
#[test]
fn raw_ciphertext_survives_a_binary_write_and_is_destroyed_by_a_legacy_coding() {
    let elisp_form = r##"(let* ((plaintext "Grüße — streng geheim\n")
       (binary (aes-encrypt-buffer-or-string plaintext "pw" "CBC" nil nil t))
       (bin-path (aes-test-path "vault/cipher.bin"))
       (fips (aes-Cipher (aes-test-unhex "00112233445566778899aabbccddeeff")
                         (aes-KeyExpansion
                          (aes--str-to-b (aes-test-unhex "000102030405060708090a0b0c0d0e0f"))
                          4)
                         4))
       (fips-binary (aes-test-path "vault/fips.bin"))
       (fips-sjis (aes-test-path "vault/fips-sjis.bin"))
       (fips-latin (aes-test-path "vault/fips-latin1.bin")))
  (make-directory (file-name-directory bin-path) t)
  (with-temp-buffer
    (set-buffer-multibyte nil)
    (insert binary)
    (let ((coding-system-for-write 'binary))
      (write-region (point-min) (point-max) bin-path nil 'silent)))
  (dolist (spec (list (cons fips-binary 'binary)
                      (cons fips-sjis 'japanese-shift-jis)
                      (cons fips-latin 'latin-1)))
    (with-temp-buffer
      (set-buffer-multibyte nil)
      (insert fips)
      (let ((coding-system-for-write (cdr spec)))
        (write-region (point-min) (point-max) (car spec) nil 'silent))))
  (let ((back (with-temp-buffer
                (set-buffer-multibyte nil)
                (let ((coding-system-for-read 'binary))
                  (insert-file-contents-literally bin-path))
                (buffer-string))))
    (list :header (aes-test-header binary)
          :has-high-bytes (and (cl-some (lambda (c) (> c 127)) (string-to-list binary)) t)
          :length (length binary)
          :on-disk-length (length (aes-test-bytes bin-path))
          :bytes-survived (string= binary back)
          :decrypts (aes-decrypt-buffer-or-string back "pw")
          :fips-hex (aes-test-hex fips)
          :fips-binary-bytes (aes-test-bytes fips-binary)
          :fips-shift-jis-bytes (aes-test-bytes fips-sjis)
          :fips-latin-1-bytes (aes-test-bytes fips-latin))))"##;

    let expect = expect![[
        r#"OK (:header "aes-encrypted V 1.3-CBC-N-4-4-M" :has-high-bytes t :length 80 :on-disk-length 80 :bytes-survived t :decrypts "Grüße — streng geheim\n" :fips-hex "69c4e0d86a7b0430d8cdb78070b4c55a" :fips-binary-bytes (105 196 224 216 106 123 4 48 216 205 183 128 112 180 197 90) :fips-shift-jis-bytes (105 196 224 216 106 123 4 48 216 32 128 112 180 197 90) :fips-latin-1-bytes (105 196 224 216 106 123 4 48 216 32 128 112 180 197 90))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

/// What happens when decryption should not succeed.  The package does not
/// signal: a wrong password, text that was never ciphertext, and a truncated
/// blob all return nil, and only the right password returns the plaintext.
/// The wrong-password case is the interesting one -- with OCB it is the
/// authentication tag, not a padding check, that refuses.
#[test]
fn a_wrong_password_or_damaged_ciphertext_yields_nothing() {
    let elisp_form = r##"(let* ((plaintext "streng geheim\n")
       (encrypted (aes-encrypt-buffer-or-string plaintext "richtig")))
  (list :wrong-password (condition-case error
                            (aes-decrypt-buffer-or-string encrypted "falsch")
                          (error (list 'error (car error))))
        :not-encrypted (condition-case error
                           (aes-decrypt-buffer-or-string "kein Chiffretext" "pw")
                         (error (list 'error (car error))))
        :truncated (condition-case error
                       (aes-decrypt-buffer-or-string
                        (substring encrypted 0 (- (length encrypted) 20)) "richtig")
                     (error (list 'error (car error))))
        :correct (aes-decrypt-buffer-or-string encrypted "richtig")))"##;

    let expect = expect![[
        r#"OK (:wrong-password nil :not-encrypted nil :truncated nil :correct "streng geheim\n")"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

/// Called without a password the package asks for one, and asks differently for
/// the two directions -- with confirmation when encrypting, without when
/// decrypting.  The rest of the workflow walks the cipher matrix the header
/// advertises: CBC and OCB at 128, 192 and 256-bit keys, each round-tripping
/// and each recording its own header.
#[test]
fn the_password_prompt_and_every_cipher_and_key_size_round_trip() {
    let elisp_form = r##"(let (prompts)
  (cl-letf (((symbol-function 'read-passwd)
             (lambda (prompt &optional confirm default)
               (push (list prompt (and confirm t)) prompts)
               "aus dem Minipuffer")))
    (let* ((aes-always-ask-for-passwords t)
           (encrypted (aes-encrypt-buffer-or-string "Geheimnis\n"))
           (decrypted (aes-decrypt-buffer-or-string encrypted)))
      (list :prompts (reverse prompts)
            :header (aes-test-header encrypted)
            :decrypted decrypted
            :variants
            (mapcar (lambda (spec)
                      (let* ((type (nth 0 spec)) (nk (nth 1 spec))
                             (enc (aes-encrypt-buffer-or-string "Daten\n" "pw" type nk)))
                        (list type nk (aes-test-header enc)
                              (aes-decrypt-buffer-or-string enc "pw"))))
                    '(("CBC" 4) ("OCB" 6) ("CBC" 8)))))))"##;

    let expect = expect![[
        r#"OK (:prompts (("encryption Password for string: " t) ("decryption Password for string: " nil)) :header "aes-encrypted V 1.3-OCB-B-4-4-U" :decrypted "Geheimnis\n" :variants (("CBC" 4 "aes-encrypted V 1.3-CBC-B-4-4-U" "Daten\n") ("OCB" 6 "aes-encrypted V 1.3-OCB-B-4-6-U" "Daten\n") ("CBC" 8 "aes-encrypted V 1.3-CBC-B-4-8-U" "Daten\n")))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}
