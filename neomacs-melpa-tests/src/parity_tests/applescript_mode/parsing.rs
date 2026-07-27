use expect_test::expect;

use super::assert_applescript_mode_parity;

#[test]
fn applescript_mode_parse_result_converts_practical_scalar_output_types_exactly() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (list
            value
            (as-parse-result value)
            (type-of
             (as-parse-result value))))
         '("0"
           "42"
           "  9001  "
           "-7"
           "3.14"
           "true"
           "false"
           "missing value"
           "\"hello world\""
           "  \" padded string \"  "))"##;
    let expect = expect![[
        r#"OK (("0" 0 integer) ("42" 42 integer) ("  9001  " 9001 integer) ("-7" \-7 symbol) ("3.14" \3.14 symbol) ("true" true symbol) ("false" false symbol) ("missing value" missing\ value symbol) ("\"hello world\"" "hello world" string) ("  \" padded string \"  " " padded string " string))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_parse_result_builds_lists_records_and_nested_realistic_values() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (list
            value
            (as-parse-result value)))
         '("{}"
           "{1,2,3}"
           "{\"alpha\", \"beta\", 7}"
           "name:\"Ada\""
           "count:42"
           "enabled:true"
           "{name:\"Ada\",count:42,enabled:true}"
           "outer:{1,2}"
           "{{1,2},{3,4}}"))"##;
    let expect = expect![[
        r#"OK (("{}" (##)) ("{1,2,3}" (1 2 3)) ("{\"alpha\", \"beta\", 7}" ("alpha" "beta" 7)) ("name:\"Ada\"" (name . "Ada")) ("count:42" (count . 42)) ("enabled:true" (enabled . true)) ("{name:\"Ada\",count:42,enabled:true}" ((name . "Ada") (count . 42) (enabled . true))) ("outer:{1,2}" (outer 1 2)) ("{{1,2},{3,4}}" ({1 2} {3 4})))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_parse_result_exposes_whitespace_commas_colons_and_escape_edge_cases() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (condition-case error
               (list
                value
                :ok
                (as-parse-result value))
             (error
              (list
               value
               :error
               (car error)
               (cadr error)))))
         '("  { 1 , 2 , word }  "
           "\"a,b,c\""
           "\"say \\\"hello\\\"\""
           "\"path\\\\to\\\\file\""
           "key:\"value:with:colon\""
           "a:b:c"
           "{trailing,}"
           "{,leading}"))"##;
    let expect = expect![[
        r#"OK (("  { 1 , 2 , word }  " :ok (1 2 \ word\ )) ("\"a,b,c\"" :ok "a,b,c") ("\"say \\\"hello\\\"\"" :ok "say \"hello\"") ("\"path\\\\to\\\\file\"" :ok "path\\to\\file") ("key:\"value:with:colon\"" :ok (key . "value:with:colon")) ("a:b:c" :ok (a b . c)) ("{trailing,}" :ok (trailing ##)) ("{,leading}" :ok (## leading)))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_escape_and_unescape_transform_backslashes_across_real_strings() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (let ((escaped
                  (as-escape-string value)))
             (list
              value
              escaped
              (as-unescape-string escaped)
              (as-unescape-string value))))
         '(""
           "plain"
           "one\\two"
           "\\leading"
           "trailing\\"
           "C:\\Users\\Ada\\Script.scpt"
           "quote\\\"inside"))"##;
    let expect = expect![[
        r#"OK (("" "" "" "") ("plain" "plain" "plain" "plain") ("one\\two" "one\\\\two" "one\\two" "onetwo") ("\\leading" "\\\\leading" "\\leading" "leading") ("trailing\\" "trailing\\\\" "trailing\\" "trailing\\") ("C:\\Users\\Ada\\Script.scpt" "C:\\\\Users\\\\Ada\\\\Script.scpt" "C:\\Users\\Ada\\Script.scpt" "C:UsersAdaScript.scpt") ("quote\\\"inside" "quote\\\\\"inside" "quote\\\"inside" "quote\"inside"))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_sjis_byte_escaping_duplicates_every_backslash_without_reordering_bytes() {
    let elisp_form = r##"(mapcar
         (lambda (bytes)
           (list
            bytes
            (as-sjis-byte-list-escape
             bytes)))
         '(nil
           (1 2 3)
           (92)
           (65 92 66)
           (92 92 92)
           (0 91 92 93 255)))"##;
    let expect = expect![
        "OK ((nil nil) ((1 2 3) (1 2 3)) ((92) (92 92)) ((65 92 66) (65 92 92 66)) ((92 92 92) (92 92 92 92 92 92)) ((0 91 92 93 255) (0 91 92 92 93 255)))"
    ];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_sjis_encoding_and_decoding_round_trip_ascii_japanese_and_mixed_text() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (let* ((encoded
                   (as-encode-string value))
                  (decoded
                   (as-decode-string encoded)))
             (list
              value
              (multibyte-string-p value)
              (multibyte-string-p encoded)
              (string-bytes encoded)
              (string-to-list encoded)
              decoded
              (equal value decoded))))
         '("plain ASCII"
           "日本語"
           "Mac 日本語 123"
           "path\\名前"))"##;
    let expect = expect![[
        r#"OK (("plain ASCII" nil nil 11 (112 108 97 105 110 32 65 83 67 73 73) "plain ASCII" t) ("日本語" t nil 6 (147 250 150 123 140 234) #("日本語" 0 3 (charset japanese-jisx0208)) t) ("Mac 日本語 123" t nil 14 (77 97 99 32 147 250 150 123 140 234 32 49 50 51) #("Mac 日本語 123" 4 11 (charset japanese-jisx0208)) t) ("path\\名前" t nil 9 (112 97 116 104 92 150 188 145 79) #("path\\名前" 5 7 (charset japanese-jisx0208)) t))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_string_to_sjis_with_escape_combines_encoding_and_backslash_duplication() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (let ((converted
                  (as-string-to-sjis-string-with-escape
                   value)))
             (list
              value
              (string-to-list converted)
              (string-bytes converted)
              (as-decode-string
               (replace-regexp-in-string
                "\\\\\\\\"
                "\\\\"
                converted
                t
                t)))))
         '("return 42"
           "set p to \"C:\\\\Temp\""
           "display dialog \"日本語\""))"##;
    let expect = expect![[
        r#"OK (("return 42" (114 101 116 117 114 110 32 52 50) 9 "return 42") ("set p to \"C:\\\\Temp\"" (115 101 116 32 112 32 116 111 32 34 67 58 92 92 92 92 84 101 109 112 34) 21 "set p to \"C:\\\\\\\\Temp\"") ("display dialog \"日本語\"" (100 105 115 112 108 97 121 32 100 105 97 108 111 103 32 34 147 250 150 123 140 234 34) 28 "display dialog \"ú{ê\""))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}

#[test]
fn applescript_mode_parse_result_decodes_sjis_encoded_quoted_output_to_unicode() {
    let elisp_form = r##"(mapcar
         (lambda (value)
           (let* ((quoted
                   (concat
                    "\""
                    value
                    "\""))
                  (encoded
                   (as-encode-string
                    quoted))
                  (parsed
                   (as-parse-result
                    encoded)))
             (list
              value
              (string-to-list encoded)
              parsed
              (equal value parsed))))
         '("こんにちは"
           "名前: Ada"
           "引用 \\\"text\\\""))"##;
    let expect = expect![[
        r#"OK (("こんにちは" (34 130 177 130 241 130 201 130 191 130 205 34) #("こんにちは" 0 5 (charset japanese-jisx0208)) t) ("名前: Ada" (34 150 188 145 79 58 32 65 100 97 34) #("名前: Ada" 0 7 (charset japanese-jisx0208)) t) ("引用 \\\"text\\\"" (34 136 248 151 112 32 92 34 116 101 120 116 92 34 34) #("引用 \"text\"" 0 9 (charset japanese-jisx0208)) nil))"#
    ]];

    assert_applescript_mode_parity(elisp_form, expect);
}
