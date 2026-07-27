use expect_test::expect;

use super::assert_apel_source_parity;

#[test]
fn mime_charset_selection_handles_known_subsets_empty_and_custom_fallbacks() {
    let elisp_form = r##"(let ((default-mime-charset-for-write 'utf-8)
                           (default-mime-charset-detect-method-for-write
                            (lambda (mode charsets &rest args)
                              (list :fallback mode charsets args))))
                      (list
                       (charsets-to-mime-charset '(ascii))
                       (charsets-to-mime-charset '(ascii latin-iso8859-1))
                       (charsets-to-mime-charset
                        '(ascii japanese-jisx0208))
                       (charsets-to-mime-charset nil)
                       (find-mime-charset-by-charsets
                        '(ascii) 'string "hello")
                       (find-mime-charset-by-charsets
                        '(ascii unicode) 'region 2 8)))"##;
    let expect = expect![
        "OK (us-ascii iso-8859-1 iso-2022-jp nil us-ascii (:fallback region (ascii unicode) (2 8)))"
    ];
    assert_apel_source_parity("mcharset.el", elisp_form, expect);
}

#[test]
fn mime_charset_coding_conversion_supports_strings_line_endings_and_callback() {
    let elisp_form = r##"(let ((mime-charset-to-coding-system-default-method
                           (lambda (charset lbt attempted)
                             (list :missing charset lbt attempted))))
                      (list
                       (mime-charset-to-coding-system 'utf-8)
                       (mime-charset-to-coding-system "UTF-8" 'LF)
                       (mime-charset-to-coding-system 'utf-8 'CRLF)
                       (mime-charset-to-coding-system 'utf-8 'CR)
                       (mime-charset-to-coding-system 'apel-unknown 'dos)
                       (mime-charset-p 'us-ascii)
                       (coding-system-to-mime-charset 'utf-8)
                       (coding-system-to-mime-charset 'utf-8-dos)))"##;
    let expect = expect![
        "OK (utf-8 utf-8-unix utf-8-dos utf-8-mac (:missing apel-unknown dos apel-unknown-dos) us-ascii utf-8 utf-8)"
    ];
    assert_apel_source_parity("mcharset.el", elisp_form, expect);
}

#[test]
fn mime_encoding_round_trip_preserves_unicode_and_exposes_encoded_bytes() {
    let elisp_form = r##"(let* ((original "Résumé 日本語 λ\nsecond line")
                           (utf8 (encode-mime-charset-string original 'utf-8))
                           (roundtrip
                            (decode-mime-charset-string utf8 'utf-8))
                           (latin (encode-mime-charset-string "Résumé" 'iso-8859-1)))
                      (list (multibyte-string-p original)
                            (multibyte-string-p utf8)
                            (string-to-list utf8)
                            roundtrip
                            (string-to-list latin)
                            (decode-mime-charset-string latin 'iso-8859-1)))"##;
    let expect = expect![[
        r#"OK (t nil (82 195 169 115 117 109 195 169 32 230 151 165 230 156 172 232 170 158 32 206 187 10 115 101 99 111 110 100 32 108 105 110 101) "Résumé 日本語 λ\nsecond line" (82 233 115 117 109 233) #("Résumé" 0 6 (charset iso-8859-1)))"#
    ]];
    assert_apel_source_parity("mcharset.el", elisp_form, expect);
}

#[test]
fn mime_detection_uses_real_ascii_latin_unicode_strings_and_regions() {
    let elisp_form = r##"(list
                      (detect-mime-charset-string "plain ASCII")
                      (detect-mime-charset-string "Résumé")
                      (detect-mime-charset-string "日本語")
                      (let ((detect-mime-charset-from-coding-system t))
                        (detect-mime-charset-string "Résumé 日本語"))
                      (with-temp-buffer
                        (insert "prefix Résumé 日本語 suffix")
                        (list (detect-mime-charset-region 1 7)
                              (detect-mime-charset-region 8 14)
                              (detect-mime-charset-region 15 18))))"##;
    let expect =
        expect!["OK (us-ascii iso-8859-1 iso-2022-jp utf-8 (us-ascii iso-8859-1 iso-2022-jp))"];
    assert_apel_source_parity("mcharset.el", elisp_form, expect);
}

#[test]
fn binary_raw_crlf_and_utf8_file_helpers_round_trip_real_bytes() {
    let elisp_form = r##"(let ((binary-file (expand-file-name "apel-binary.dat"
                                                        default-directory))
                           (crlf-file (expand-file-name "apel-crlf.txt"
                                                       default-directory))
                           (utf8-file (expand-file-name "apel-utf8.txt"
                                                       default-directory)))
                      (with-temp-buffer
                        (set-buffer-multibyte nil)
                        (insert (unibyte-string 0 10 13 127 128 255))
                        (write-region-as-binary
                         (point-min) (point-max) binary-file))
                      (with-temp-buffer
                        (insert "alpha\nRésumé\n")
                        (write-region-as-raw-text-CRLF
                         (point-min) (point-max) crlf-file)
                        (write-region-as-coding-system
                         'utf-8 (point-min) (point-max) utf8-file))
                      (list
                       (with-temp-buffer
                         (insert-file-contents-as-binary binary-file)
                         (string-to-list (buffer-string)))
                       (with-temp-buffer
                         (insert-file-contents-as-raw-text-CRLF crlf-file)
                         (buffer-string))
                       (with-temp-buffer
                         (insert-file-contents-as-coding-system 'utf-8 utf8-file)
                         (buffer-string))
                       (with-temp-buffer
                         (insert-file-contents-literally crlf-file)
                         (string-to-list (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ((0 10 13 127 4194176 4194303) "alpha\nR\303\251sum\303\251\n" "alpha\nRésumé\n" (97 108 112 104 97 13 10 82 4194243 4194217 115 117 109 4194243 4194217 13 10))"#
    ]];
    assert_apel_source_parity("pces.el", elisp_form, expect);
}

#[test]
fn binary_context_macros_scope_coding_variables_without_leaking() {
    let elisp_form = r##"(let ((coding-system-for-read 'utf-8)
                           (coding-system-for-write 'utf-16))
                      (list
                       (as-binary-process
                         (list coding-system-for-read
                               coding-system-for-write
                               selective-display))
                       (as-binary-input-file coding-system-for-read)
                       (as-binary-output-file coding-system-for-write)
                       coding-system-for-read
                       coding-system-for-write
                       (find-coding-system "UTF-8")
                       (find-coding-system 'utf-8)
                       (find-coding-system 'does-not-exist)))"##;
    let expect = expect!["OK ((binary binary nil) binary binary utf-8 utf-16 nil utf-8 nil)"];
    assert_apel_source_parity("pces.el", elisp_form, expect);
}

#[test]
fn caesar_rotation_is_reversible_for_practical_ascii_mail_subjects() {
    let elisp_form = r##"(with-temp-buffer
                     (insert "Meet Me At 09:30 - Project Phoenix!")
                     (let ((original (buffer-string)))
                       (mule-caesar-region (point-min) (point-max))
                       (let ((rot13 (buffer-string)))
                         (mule-caesar-region (point-min) (point-max))
                         (let ((roundtrip (buffer-string)))
                           (erase-buffer)
                           (insert "Abc-Xyz")
                           (mule-caesar-region (point-min) (point-max) 5)
                           (list original rot13 roundtrip (buffer-string))))))"##;
    let expect = expect![[
        r#"OK ("Meet Me At 09:30 - Project Phoenix!" "Zrrg Zr Ng 09:30 - Cebwrpg Cubravk!" "Meet Me At 09:30 - Project Phoenix!" "Fgh-Cde")"#
    ]];
    assert_apel_source_parity("mule-caesar.el", elisp_form, expect);
}

#[test]
fn charset_fontset_character_index_and_mutation_helpers_cover_multilingual_text() {
    let elisp_form = r##"(let ((text (copy-sequence "Aλ中")))
                      (sset text 0 ?B)
                      (list text
                            (fontset-pixel-size
                             "-misc-fixed-medium-r-normal--16-160-75-75-c-80")
                            (fontset-pixel-size "not-a-fontset")
                            (find-non-ascii-charset-string "ASCII")
                            (find-non-ascii-charset-string "Résumé 日本語")
                            (with-temp-buffer
                              (insert "A Résumé 日本語 Z")
                              (find-non-ascii-charset-region 3 12))
                            (let ((index 3))
                              (list (char-next-index ?中 index) index))
                            (looking-at-as-unibyte "A")))"##;
    let expect = expect![[r#"OK ("Bλ中" 16 0 nil (unicode-bmp) (unicode-bmp) (4 3) nil)"#]];
    assert_apel_source_parity("poem.el", elisp_form, expect);
}

#[test]
fn portable_ccl_surface_reports_capabilities_and_all_compatibility_checks() {
    let elisp_form = r##"(list
                      (featurep 'pccl)
                      (help-function-arglist 'make-ccl-coding-system t)
                      (mapcar
                       (lambda (facility)
                         (list facility
                               (get facility 'broken)
                               (broken-p facility)))
                       '(ccl-usable
                         ccl-accept-symbol-as-program
                         ccl-execute-eof-block-on-encoding-null
                         ccl-execute-eof-block-on-encoding-some
                         ccl-execute-eof-block-on-decoding-null
                         ccl-execute-eof-block-on-decoding-some
                         ccl-execute-eof-block-on-encoding
                         ccl-execute-eof-block-on-decoding
                         ccl-execute-eof-block)))"##;
    let expect = expect![
        "OK (t (coding-system mnemonic docstring decoder encoder) ((ccl-usable nil nil) (ccl-accept-symbol-as-program nil nil) (ccl-execute-eof-block-on-encoding-null nil nil) (ccl-execute-eof-block-on-encoding-some nil nil) (ccl-execute-eof-block-on-decoding-null nil nil) (ccl-execute-eof-block-on-decoding-some nil nil) (ccl-execute-eof-block-on-encoding nil nil) (ccl-execute-eof-block-on-decoding nil nil) (ccl-execute-eof-block nil nil)))"
    ];
    assert_apel_source_parity("pccl.el", elisp_form, expect);
}

#[test]
fn portable_ccl_defines_a_real_identity_coding_system_and_round_trips_bytes() {
    let elisp_form = r##"(progn
                      (define-ccl-program apel-test-ccl-identity
                        '(1 ((read r0)
                             (loop
                              (write-read-repeat r0)))))
                      (make-ccl-coding-system
                       'apel-test-ccl ?A "APEL test identity CCL"
                       'apel-test-ccl-identity
                       'apel-test-ccl-identity)
                      (let* ((input (unibyte-string 0 1 65 127 128 255))
                             (encoded
                              (encode-coding-string
                               input 'apel-test-ccl))
                             (decoded
                              (decode-coding-string
                               encoded 'apel-test-ccl)))
                        (list (coding-system-p 'apel-test-ccl)
                              (coding-system-get
                               'apel-test-ccl 'coding-type)
                              (string-to-list encoded)
                              (string-to-list decoded)
                              (equal input decoded))))"##;
    let expect = expect!["OK (t ccl (0 1 65 127 128 255) (0 1 65 127 128 255) nil)"];
    assert_apel_source_parity("pccl.el", elisp_form, expect);
}
