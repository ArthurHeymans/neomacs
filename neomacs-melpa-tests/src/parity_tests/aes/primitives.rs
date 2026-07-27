use expect_test::expect;

use super::assert_aes_parity;

#[test]
fn aes_xor_helpers_return_and_mutate_complete_byte_sequences() {
    let elisp_form = r##"(let* ((x (unibyte-string 0 1 127 128 255))
                    (y (unibyte-string 255 128 127 1 170))
                    (destructive (copy-sequence x))
                    (word-x '((1 . 2) . (3 . 4)))
                    (word-y '((16 . 32) . (48 . 64)))
                    (word-destructive (copy-tree word-x)))
               (list
                (string-to-list (aes--xor x y))
                (string-to-list
                 (aes--xor-de destructive y))
                (string-to-list x)
                (eq destructive
                    (aes--xor-de destructive
                                 (make-string 5 0)))
                (aes--xor-4 word-x word-y)
                (progn
                  (aes--xor-4-de
                   word-destructive word-y)
                  word-destructive)
                word-x))"##;
    let expect = expect![
        "OK ((255 129 0 129 85) (255 129 0 129 85) (0 1 127 128 255) t ((17 . 34) 51 . 68) ((17 . 34) 51 . 68) ((1 . 2) 3 . 4))"
    ];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_padding_and_block_rounding_cover_empty_exact_partial_and_fallback_formats() {
    let elisp_form = r##"(let ((source (unibyte-string 1 2 3 4 5)))
               (list
                (mapcar
                 (lambda (args)
                   (let ((value
                          (apply #'aes-pad
                                 source args)))
                     (list
                      (string-to-list value)
                      (multibyte-string-p value))))
                 '((4)
                   (4 "Zero")
                   (4 "PKCS#7")
                   (4 "unknown")))
                (string-to-list
                 (aes-zero-pad
                  (unibyte-string 1 2 3 4)
                  4))
                (string-to-list
                 (aes-zero-pad "" 8))
                (string-to-list
                 (aes-pkcs7-pad
                  (unibyte-string 1 2 3 4)
                  4))
                (string-to-list
                 (aes-pkcs7-pad "" 4))
                (mapcar
                 (lambda (n)
                   (aes--enlarge-to-multiple-num
                    8 n))
                 '(0 1 7 8 9 17))
                (string-to-list source)))"##;
    let expect = expect![
        "OK ((((1 2 3 4 5 0 0 0) nil) ((1 2 3 4 5 0 0 0) nil) ((1 2 3 4 5 3 3 3) nil) ((1 2 3 4 5 0 0 0) nil)) (1 2 3 4) nil (1 2 3 4 4 4 4 4) (4 4 4 4) (0 8 8 8 16 24) (1 2 3 4 5))"
    ];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_string_to_word_conversion_preserves_column_order_and_dotted_byte_shape() {
    let elisp_form = r##"(list
              (aes--str-to-b
               (unibyte-string
                0 1 2 3
                252 253 254 255))
              (aes--str-to-b "")
              (mapcar
               (lambda (word)
                 (list
                  (car (car word))
                  (cdr (car word))
                  (car (cdr word))
                  (cdr (cdr word))))
               (aes--str-to-b
                (apply #'unibyte-string
                       (number-sequence 16 31)))))"##;
    let expect = expect![
        "OK ((((0 . 1) 2 . 3) ((252 . 253) 254 . 255)) nil ((16 17 18 19) (20 21 22 23) (24 25 26 27) (28 29 30 31)))"
    ];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_lookup_tables_encode_galois_products_inverses_and_bijective_s_boxes() {
    let elisp_form = r##"(list
              (mapcar
               #'boundp
               '(aes--l
                 aes--mt
                 aes--inv-table
                 aes--l2 aes--l3 aes--l9
                 aes--lb aes--ld aes--le
                 aes--s-boxes-enc
                 aes--s-boxes-dec))
              (mapcar
               #'length
               (list
                aes--inv-table
                aes--l2 aes--l3 aes--l9
                aes--lb aes--ld aes--le
                aes--s-boxes-enc
                aes--s-boxes-dec))
              (mapcar
               (lambda (byte)
                 (list
                  byte
                  (aref aes--inv-table byte)
                  (aref aes--l2 byte)
                  (aref aes--l3 byte)
                  (aref aes--l9 byte)
                  (aref aes--lb byte)
                  (aref aes--ld byte)
                  (aref aes--le byte)
                  (aref aes--s-boxes-enc byte)
                  (aref aes--s-boxes-dec byte)))
               '(0 1 2 83 127 128 255))
              (let ((ok t)
                    (byte 0))
                (while (< byte 256)
                  (unless
                      (= byte
                         (aref
                          aes--s-boxes-dec
                          (aref
                           aes--s-boxes-enc
                           byte)))
                    (setq ok nil))
                  (setq byte (1+ byte)))
                ok))"##;
    let expect = expect![
        "OK ((t t t t t t t t t t t) (256 256 256 256 256 256 256 256 256) ((0 0 0 0 0 0 0 0 99 82) (1 1 2 3 9 11 13 14 124 9) (2 141 4 6 18 22 26 28 119 106) (83 202 166 245 253 91 170 95 237 80) (127 130 254 129 170 84 77 204 210 107) (128 131 27 155 236 247 218 65 205 58) (255 28 229 26 70 163 151 141 22 125)) t)"
    ];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_substitution_helpers_mutate_bytes_and_words_then_inverse_restores_state() {
    let elisp_form = r##"(let* ((state
                     (unibyte-string
                      0 1 2 83 124 127 128 255))
                    (original
                     (copy-sequence state))
                    (word
                     '((0 . 83) . (124 . 255)))
                    (sub-result
                     (aes-SubBytes state))
                    (after-sub
                     (string-to-list state))
                    (inv-result
                     (aes-InvSubBytes state))
                    (word-result
                     (aes-SubWord word)))
               (list
                sub-result
                after-sub
                inv-result
                (string-to-list state)
                (equal state original)
                word-result
                word))"##;
    let expect = expect![
        "OK (nil (99 124 119 237 16 210 205 22) nil (0 1 2 83 124 127 128 255) t 22 ((99 . 237) 16 . 22))"
    ];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_shift_rows_and_inverse_cover_all_supported_state_widths() {
    let elisp_form = r##"(mapcar
              (lambda (length)
                (let* ((state
                        (apply #'unibyte-string
                               (number-sequence
                                0 (1- length))))
                       (original
                        (copy-sequence state))
                       (shift-result
                        (aes-ShiftRows state))
                       (shifted
                        (string-to-list state))
                       (inverse-result
                        (aes-InvShiftRows state)))
                  (list
                   length
                   shift-result
                   shifted
                   inverse-result
                   (string-to-list state)
                   (equal state original))))
              '(16 24 32))"##;
    let expect = expect![
        "OK ((16 11 (0 5 10 15 4 9 14 3 8 13 2 7 12 1 6 11) 3 (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15) t) (24 11 (0 5 10 15 4 9 14 19 8 13 18 23 12 17 22 3 16 21 2 7 20 1 6 11) 3 (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23) t) (32 11 (0 5 10 15 4 9 14 19 8 13 18 23 12 17 22 27 16 21 26 31 20 25 30 3 24 29 2 7 28 1 6 11) 3 (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31) t))"
    ];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_rot_word_and_key_expansion_cover_128_192_256_bit_keys_and_explicit_rounds() {
    let elisp_form = r##"(let ((flatten
                    (lambda (word)
                      (list
                       (car (car word))
                       (cdr (car word))
                       (car (cdr word))
                       (cdr (cdr word))))))
               (list
                (let ((word
                       '((1 . 2) . (3 . 4))))
                  (list
                   (aes--RotWord word)
                   word))
                (mapcar
                 (lambda (nk)
                   (let* ((key
                           (aes--str-to-b
                            (apply
                             #'unibyte-string
                             (number-sequence
                              0 (1- (* 4 nk))))))
                          (expanded
                           (aes-KeyExpansion key 4)))
                     (list
                      nk
                      (length expanded)
                      (mapcar
                       flatten
                       (seq-take expanded nk))
                      (mapcar
                       flatten
                       (last expanded 4)))))
                 '(4 6 8))
                (let* ((key
                        (aes--str-to-b
                         (apply #'unibyte-string
                                (number-sequence 0 15))))
                       (expanded
                        (aes-KeyExpansion key 4 2)))
                  (list
                   (length expanded)
                   (mapcar flatten expanded)))))"##;
    let expect = expect![
        "OK ((1 ((2 . 3) 4 . 1)) ((4 44 ((0 1 2 3) (4 5 6 7) (8 9 10 11) (12 13 14 15)) ((19 17 29 127) (227 148 74 23) (243 7 167 139) (77 43 48 197))) (6 52 ((0 1 2 3) (4 5 6 7) (8 9 10 11) (12 13 14 15) (16 17 18 19) (20 21 22 23)) ((163 34 97 86) (196 55 177 133) (65 62 17 190) (113 165 32 4))) (8 60 ((0 1 2 3) (4 5 6 7) (8 9 10 11) (12 13 14 15) (16 17 18 19) (20 21 22 23) (24 25 26 27) (28 29 30 31)) ((129 176 200 251) (64 7 249 113) (140 142 105 34) (134 31 106 190)))) (12 ((0 1 2 3) (4 5 6 7) (8 9 10 11) (12 13 14 15) (214 170 116 253) (210 175 114 250) (218 166 120 241) (214 171 118 254) (182 146 207 11) (100 61 189 241) (190 155 197 0) (104 48 179 254))))"
    ];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_round_key_helpers_mutate_full_16_24_and_32_byte_states_and_reverse_order() {
    let elisp_form = r##"(mapcar
              (lambda (length)
                (let* ((state
                        (apply #'unibyte-string
                               (number-sequence
                                0 (1- length))))
                       (original
                        (copy-sequence state))
                       (keys
                        (aes--str-to-b
                         (apply #'unibyte-string
                                (number-sequence
                                 32 (+ 31 length)))))
                       (forward-result
                        (aes-AddRoundKey state keys))
                       (encrypted
                        (string-to-list state))
                       (inverse-result
                        (aes-InvAddRoundKey
                         state
                         (reverse keys))))
                  (list
                   length
                   forward-result
                   encrypted
                   inverse-result
                   (string-to-list state)
                   (equal state original))))
              '(16 24 32))"##;
    let expect = expect![
        "OK ((16 nil (32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32) nil (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15) t) (24 nil (32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32) nil (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23) t) (32 nil (32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32 32) nil (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31) t))"
    ];

    assert_aes_parity(elisp_form, expect);
}

#[test]
fn aes_combined_round_helpers_expose_fips_first_round_and_inverse_round_results() {
    let elisp_form = r##"(let* ((plain
                     (apply #'unibyte-string
                            '(0 17 34 51 68 85 102 119
                              136 153 170 187
                              204 221 238 255)))
                    (key
                     (aes--str-to-b
                      (apply #'unibyte-string
                             (number-sequence 0 15))))
                    (keys
                     (aes-KeyExpansion key 4))
                    (state
                     (copy-sequence plain))
                    (copy
                     (make-string 16 0)))
               (aes-AddRoundKey state keys)
               (let ((after-add
                      (string-to-list state))
                     (round-result
                      (aes-SubShiftMixKeys
                       state
                       (store-substring
                        copy 0 state)
                       (nthcdr 4 keys))))
                 (list
                  after-add
                  round-result
                  (string-to-list state)
                  (let* ((cipher
                          (aes-Cipher
                           plain keys 4))
                         (reverse-keys
                          (reverse keys))
                         (inverse-state
                          (copy-sequence cipher))
                         (inverse-copy
                          (make-string 16 0)))
                    (aes-InvAddRoundKey
                     inverse-state reverse-keys)
                    (aes-InvShiftRows
                     inverse-state)
                    (aes-InvSubBytes
                     inverse-state)
                    (let ((inverse-round-result
                           (aes-InvSubShiftMixKeys
                            inverse-state
                            (store-substring
                             inverse-copy
                             0 inverse-state)
                            (nthcdr
                             4 reverse-keys))))
                      (list
                       inverse-round-result
                       (string-to-list
                        inverse-state)))))))"##;
    let expect = expect![
        "OK ((0 16 32 48 64 80 96 112 128 144 160 176 192 208 224 240) nil (137 216 16 232 133 90 206 104 45 24 67 216 203 18 143 228) (nil (253 227 186 210 5 229 208 215 53 71 150 78 241 254 55 241)))"
    ];

    assert_aes_parity(elisp_form, expect);
}
