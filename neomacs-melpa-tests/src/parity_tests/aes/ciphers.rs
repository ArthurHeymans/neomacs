use expect_test::expect;

use super::{assert_aes_parity, assert_aes_signal_parity};

#[test]
fn aes_cipher_matches_fips_197_vectors_for_128_192_and_256_bit_keys() {
    let elisp_form = r##"(let ((plain
                    (apply #'unibyte-string
                           '(0 17 34 51 68 85 102 119
                             136 153 170 187
                             204 221 238 255)))
                   (hex
                    (lambda (string)
                      (mapconcat
                       (lambda (byte)
                         (format "%02x" byte))
                       string ""))))
               (mapcar
                (lambda (nk)
                  (let* ((key
                          (aes--str-to-b
                           (apply #'unibyte-string
                                  (number-sequence
                                   0 (1- (* 4 nk))))))
                         (keys
                          (aes-KeyExpansion key 4))
                         (cipher
                          (aes-Cipher plain keys 4))
                         (explicit
                          (aes-Cipher
                           plain keys 4
                           (+ nk 6)))
                         (decrypted
                          (aes-InvCipher
                           cipher
                           (reverse keys)
                           4))
                         (explicit-decrypted
                          (aes-InvCipher
                           cipher
                           (reverse keys)
                           4
                           (+ nk 6))))
                    (list
                     nk
                     (funcall hex cipher)
                     (funcall hex explicit)
                     (funcall hex decrypted)
                     (funcall hex explicit-decrypted))))
                '(4 6 8)))"##;
    let expect = expect![[
        r#"OK ((4 "69c4e0d86a7b0430d8cdb78070b4c55a" "69c4e0d86a7b0430d8cdb78070b4c55a" "00112233445566778899aabbccddeeff" "00112233445566778899aabbccddeeff") (6 "d79ae2e2ccf42c10aef15a2756dd475d" "d79ae2e2ccf42c10aef15a2756dd475d" "00112233445566778899aabbccddeeff" "00112233445566778899aabbccddeeff") (8 "80c13336aa3ad391226b54e81ebdf482" "80c13336aa3ad391226b54e81ebdf482" "00112233445566778899aabbccddeeff" "00112233445566778899aabbccddeeff"))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn rijndael_cipher_roundtrips_supported_192_and_256_bit_block_widths() {
    let elisp_form = r##"(let ((hex
                    (lambda (string)
                      (mapconcat
                       (lambda (byte)
                         (format "%02x" byte))
                       string ""))))
               (mapcar
                (lambda (spec)
                  (let* ((nb (car spec))
                         (nk (cadr spec))
                         (plain
                          (apply
                           #'unibyte-string
                           (number-sequence
                            0 (1- (* 4 nb)))))
                         (key
                          (aes--str-to-b
                           (apply
                            #'unibyte-string
                            (number-sequence
                             32 (+ 31 (* 4 nk))))))
                         (keys
                          (aes-KeyExpansion key nb))
                         (cipher
                          (aes-Cipher plain keys nb))
                         (decrypted
                          (aes-InvCipher
                           cipher
                           (reverse keys)
                           nb)))
                    (list
                     spec
                     (funcall hex cipher)
                     (funcall hex decrypted)
                     (equal plain decrypted))))
                '((6 4) (6 8) (8 4) (8 8))))"##;
    let expect = expect![[
        r#"OK (((6 4) "176598efafaa2cae4bef2f6b0fa0c8b789f6841875516f02" "000102030405060708090a0b0c0d0e0f1011121314151617" t) ((6 8) "17bdad2309124da4b0e1da32285cfb5ee197b21c22175389" "000102030405060708090a0b0c0d0e0f1011121314151617" t) ((8 4) "016369d11fb583c097c8d9bdb2ddbab7d7bceef23445b195912eafabf46dadc7" "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f" t) ((8 8) "b0161f24d3d92b4d14c8c90981bfa929c6ca3a9a76f54a402812da66c28298c5" "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f" t))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_cbc_matches_nist_multiblock_vector_without_padding_growth() {
    let elisp_form = r##"(let* ((from-hex
                     (lambda (hex)
                       (apply
                        #'unibyte-string
                        (mapcar
                         (lambda (offset)
                           (string-to-number
                            (substring
                             hex offset (+ offset 2))
                            16))
                         (number-sequence
                          0 (- (length hex) 2) 2)))))
                    (hex
                     (lambda (string)
                       (mapconcat
                        (lambda (byte)
                          (format "%02x" byte))
                        string "")))
                    (key
                     (aes--str-to-b
                      (funcall
                       from-hex
                       "2b7e151628aed2a6abf7158809cf4f3c")))
                    (keys
                     (aes-KeyExpansion key 4))
                    (iv
                     (funcall
                      from-hex
                      "000102030405060708090a0b0c0d0e0f"))
                    (plain
                     (funcall
                      from-hex
                      (concat
                       "6bc1bee22e409f96e93d7e117393172a"
                       "ae2d8a571e03ac9c9eb76fac45af8e51")))
                    (cipher
                     (aes-cbc-encrypt
                      plain iv keys 4 "Zero"))
                    (decrypted
                     (aes-cbc-decrypt
                      cipher iv
                      (reverse keys) 4)))
               (list
                (funcall hex cipher)
                (funcall hex decrypted)
                (length cipher)
                (equal plain decrypted)))"##;
    let expect = expect![[
        r#"OK ("7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b2" "6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51" 32 t)"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_cbc_padding_branches_preserve_zeroes_and_expose_pkcs7_bytes_on_decrypt() {
    let elisp_form = r##"(let* ((key
                     (aes--str-to-b
                      (make-string 16 0)))
                    (keys
                     (aes-KeyExpansion key 4))
                    (reverse-keys
                     (reverse keys))
                    (iv
                     (make-string 16 0)))
               (mapcar
                (lambda (spec)
                  (let* ((plain
                          (apply
                           #'unibyte-string
                           (number-sequence
                            1 (car spec))))
                         (padding
                          (cadr spec))
                         (cipher
                          (aes-cbc-encrypt
                           plain iv keys 4 padding))
                         (decrypted
                          (aes-cbc-decrypt
                           cipher iv
                           reverse-keys 4)))
                    (list
                     spec
                     (length cipher)
                     (string-to-list decrypted))))
                '((0 "Zero")
                  (1 "Zero")
                  (15 "PKCS#7")
                  (16 "PKCS#7")
                  (17 "unknown"))))"##;
    let expect = expect![[
        r#"OK (((0 "Zero") 0 nil) ((1 "Zero") 16 (1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0)) ((15 "PKCS#7") 16 (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 1)) ((16 "PKCS#7") 32 (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 16 16 16 16 16 16 16 16 16 16 16 16 16 16 16 16)) ((17 "unknown") 32 (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0)))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_ocb_field_multiplication_mutates_supported_block_widths() {
    let elisp_form = r##"(mapcar
              (lambda (length)
                (let* ((value
                        (concat
                         (unibyte-string 128)
                         (make-string
                          (- length 2) 0)
                         (unibyte-string 1)))
                       (double
                        (copy-sequence value))
                       (triple
                        (copy-sequence value)))
                  (list
                   length
                   (eq double
                       (aes--ocb-double-de
                        double))
                   (string-to-list double)
                   (eq triple
                       (aes--ocb-triple-de
                        triple))
                   (string-to-list triple)
                   (string-to-list value))))
              '(16 24 32 40 48 56 64))"##;
    let expect = expect![
        "OK ((16 t (0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 133) t (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 132) (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1)) (24 t (0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 133) t (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 132) (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1)) (32 t (0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 4 39) t (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 4 38) (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1)) (40 t (0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 25) t (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 24) (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1)) (48 t (0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 16 9) t (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 16 8) (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1)) (56 t (0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 8 65) t (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 8 64) (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1)) (64 t (0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 39) t (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 38) (128 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1)))"
    ];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_ocb_field_multiplication_rejects_unsupported_block_width() {
    let elisp_form = r##"(aes--ocb-double-de
              (make-string 15 0))"##;
    let expect = expect![[
        r#"ERR (error "The specified blocksize of string \"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\" is not allowed")"#
    ]];

    assert_aes_signal_parity(elisp_form, expect);
}

#[test]
fn aes_num2str_emits_fixed_width_big_endian_representations() {
    let elisp_form = r##"(mapcar
              (lambda (spec)
                (string-to-list
                 (aes-num2str
                  (car spec)
                  (cadr spec))))
              '((0 1)
                (1 1)
                (255 2)
                (256 2)
                (65535 4)
                (16909060 8)))"##;
    let expect = expect!["OK ((0) (1) (0 255) (1 0) (0 0 255 255) (0 0 0 0 1 2 3 4))"];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_ocb_pmac_covers_empty_partial_exact_and_multiple_header_blocks() {
    let elisp_form = r##"(let* ((hex
                     (lambda (string)
                       (mapconcat
                        (lambda (byte)
                          (format "%02x" byte))
                        string "")))
                    (key
                     (aes--str-to-b
                      (apply #'unibyte-string
                             (number-sequence 0 15))))
                    (keys
                     (aes-KeyExpansion key 4)))
               (mapcar
                (lambda (length)
                  (let ((header
                         (apply
                          #'unibyte-string
                          (number-sequence
                           1 length))))
                    (list
                     length
                     (funcall
                      hex
                      (aes-ocb-pmac
                       header keys 4)))))
                '(0 1 15 16 17 32 33)))"##;
    let expect = expect![[
        r#"OK ((0 "2326acd9b5ae57e502ecfbaa4ac9bcb0") (1 "9214fc495c2c20c542a95264084bab28") (15 "7278a026c64f3f3bfae68dc1f11e0258") (16 "b8c0a77948e373f19180438189048b9c") (17 "aef8184158513c456ed98fb4d57c0296") (32 "e4c054b1f582b39742119061bcfeab14") (33 "8bba590c9d093732eea9460b142f1046"))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_ocb_encrypt_decrypt_covers_empty_partial_exact_and_multiblock_messages() {
    let elisp_form = r##"(let* ((hex
                     (lambda (string)
                       (mapconcat
                        (lambda (byte)
                          (format "%02x" byte))
                        string "")))
                    (key
                     (aes--str-to-b
                      (apply #'unibyte-string
                             (number-sequence 0 15))))
                    (keys
                     (aes-KeyExpansion key 4))
                    (iv
                     (apply #'unibyte-string
                            (number-sequence 16 31))))
               (mapcar
                (lambda (spec)
                  (let* ((length
                          (car spec))
                         (header
                          (cadr spec))
                         (plain
                          (apply
                           #'unibyte-string
                           (number-sequence
                            1 length)))
                         (encrypted
                          (aes-ocb-encrypt
                           plain header iv keys 4))
                         (cipher
                          (car encrypted))
                         (tag
                          (cdr encrypted))
                         (full
                          (aes-ocb-decrypt
                           cipher header tag
                           iv keys))
                         (short
                          (aes-ocb-decrypt
                           cipher header
                           (substring tag 0 8)
                           iv keys 4))
                         (bad-tag
                          (copy-sequence tag)))
                    (aset bad-tag 0
                          (logxor 1
                                  (aref bad-tag 0)))
                    (list
                     length
                     header
                     (funcall hex cipher)
                     (funcall hex tag)
                     (string-to-list full)
                     (string-to-list short)
                     (aes-ocb-decrypt
                      cipher header bad-tag
                      iv keys 4))))
                '((0 "")
                  (1 "")
                  (15 "header")
                  (16 "header")
                  (17 "header")
                  (32 "a longer authenticated header"))))"##;
    let expect = expect![[
        r#"OK ((0 "" "" "fea52af2b7233ad00f78f6be08af38ae" nil nil nil) (1 "" "6e" "406c8032fdbbd5b4149a67d34bda6f24" (1) (1) nil) (15 "header" "2f76129cfcbee4dc6dd20197c35556" "8ee37ca53160dcdfeabc648990725196" (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15) (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15) nil) (16 "header" "a917cccf3bfa36a21026f752e7ed8283" "2de4227a2186eb6141473048c25d14ce" (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16) (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16) nil) (17 "header" "e5b56eaf4fee8f1608645eb1c3db424d83" "9be6963a230039ed59757e58677ee33a" (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17) (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17) nil) (32 "a longer authenticated header" "e5b56eaf4fee8f1608645eb1c3db424d3f653ec10029ce7d80855048cb2baf45" "9c7a4f467663af649ed6b56b438ad178" (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32) (1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32) nil))"#
    ]];

    assert_aes_parity(elisp_form, expect);
}
