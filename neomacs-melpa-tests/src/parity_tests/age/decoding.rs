use expect_test::expect;

use super::assert_age_parity;

#[test]
fn age_percent_decoder_handles_ascii_utf8_literal_percent_and_malformed_sequences() {
    let elisp_form = r##"(mapcar
         (lambda (input)
           (let ((decoded (age--decode-percent-escape input)))
             (list input
                   decoded
                   (string-bytes decoded)
                   (string-to-list decoded))))
         '("plain"
           "%41%42%43"
           "100%%25"
           "%E2%82%AC"
           "%2fpath%2Ffile"
           "%GG%4"
           "x%00y"
           "%2520"))"##;
    let expect = expect![[
        r#"OK (("plain" "plain" 5 (112 108 97 105 110)) ("%41%42%43" "ABC" 3 (65 66 67)) ("100%%25" "100%25" 6 (49 48 48 37 50 53)) ("%E2%82%AC" "������" 3 (226 130 172)) ("%2fpath%2Ffile" "/path/file" 10 (47 112 97 116 104 47 102 105 108 101)) ("%GG%4" "%GG%4" 5 (37 71 71 37 52)) ("x%00y" "x\0y" 3 (120 0 121)) ("%2520" "%20" 3 (37 50 48)))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_utf8_percent_decoder_round_trips_multibyte_mailto_style_values() {
    let elisp_form = r##"(mapcar
         (lambda (input)
           (list input
                 (age--decode-percent-escape-as-utf-8 input)))
         '("caf%C3%A9"
           "%CE%BB%20value"
           "%F0%9F%94%90"
           "mixed%2Fpath"
           "literal%%sign"))"##;
    let expect = expect![[
        r#"OK (("caf%C3%A9" "café") ("%CE%BB%20value" "λ value") ("%F0%9F%94%90" "🔐") ("mixed%2Fpath" "mixed/path") ("literal%%sign" "literal%sign"))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_hex_decoder_consumes_only_contiguous_pairs_from_the_start() {
    let elisp_form = r##"(mapcar
         (lambda (input)
           (let ((decoded (age--decode-hexstring input)))
             (list input decoded (string-to-list decoded))))
         '("414243"
           "48656c6c6f"
           "41zz42"
           "x41"
           "0"
           ""
           "00ff"
           "c3a9"))"##;
    let expect = expect![[
        r#"OK (("414243" "ABC" (65 66 67)) ("48656c6c6f" "Hello" (72 101 108 108 111)) ("41zz42" "Azz42" (65 122 122 52 50)) ("x41" "x41" (120 52 49)) ("0" "0" (48)) ("" "" nil) ("00ff" "\0ÿ" (0 255)) ("c3a9" "Ã©" (195 169)))"#
    ]];
    assert_age_parity(elisp_form, expect);
}

#[test]
fn age_quoted_decoder_handles_escaped_punctuation_hex_and_untouched_backslashes() {
    let elisp_form = r##"(mapcar
         (lambda (input)
           (let ((decoded (age--decode-quotedstring input)))
             (list input decoded (string-to-list decoded))))
         '("plain"
           "\\,\\=\\+\\<\\>\\#\\;\\\""
           "alpha\\20beta"
           "\\41\\42\\43"
           "\\zz"
           "path\\\\name"
           "\\c3\\a9"))"##;
    let expect = expect![[
        r#"OK (("plain" "plain" (112 108 97 105 110)) ("\\,\\=\\+\\<\\>\\#\\;\\\"" ",=+<>#;\"" (44 61 43 60 62 35 59 34)) ("alpha\\20beta" "alpha\0beta" (97 108 112 104 97 0 98 101 116 97)) ("\\41\\42\\43" "\0\0\0" (0 0 0)) ("\\zz" "\\zz" (92 122 122)) ("path\\\\name" "path\\name" (112 97 116 104 92 110 97 109 101)) ("\\c3\\a9" "\0\0" (0 0)))"#
    ]];
    assert_age_parity(elisp_form, expect);
}
