use expect_test::expect;

use super::assert_asn1_mode_parity;

#[test]
fn regexp_opt_builds_word_bounded_capturing_alternatives() {
    let elisp_form = r##"(let ((regexp (asn1-mode-regexp-opt
                         "SEQUENCE OF" "SET OF" "CHOICE")))
          (list
           regexp
           (mapcar
            (lambda (text)
              (and (string-match regexp text)
                   (list
                    (match-string 0 text)
                    (match-string 1 text)
                    (match-beginning 0)
                    (match-end 0))))
            '("SEQUENCE OF Thing" "x SET OF Value"
              "CHOICE" "SEQUENCE-OF" "lower choice"))))"##;
    let expect = expect![[
        r#"OK ("\\b\\(CHOICE\\|SE\\(?:\\(?:QUENCE\\|T\\) OF\\)\\)\\b" (("SEQUENCE OF" "SEQUENCE OF" 0 11) ("SET OF" "SET OF" 2 8) ("CHOICE" "CHOICE" 0 6) nil ("choice" "choice" 6 12)))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn token_match_group_maps_real_match_data_to_first_matching_registry_entry() {
    let elisp_form = r##"(mapcar
          (lambda (text)
            (string-match asn1-mode-token-regexp text)
            (list text
                  (asn1-mode-token-match-group
                   (match-data)
                   asn1-mode-token-alist)
                  (match-string 0 text)))
          '("AUTOMATIC" "WITH SYNTAX" "CLASS" "TAGS"
            "SEQUENCE OF" "OBJECT IDENTIFIER" "<field/>"
            "..." "::="))"##;
    let expect = expect![[
        r#"OK (("AUTOMATIC" "_TAG_KIND" "AUTOMATIC") ("WITH SYNTAX" "_WITH_SYNTAX" "WITH SYNTAX") ("CLASS" "_CLASS" "CLASS") ("TAGS" "TAGS" "TAGS") ("SEQUENCE OF" "_SET" "SEQUENCE OF") ("OBJECT IDENTIFIER" "_UCASE_ID" "OBJECT IDENTIFIER") ("<field/>" "_XML_SINGLE" "<field/>") ("..." "..." "...") ("::=" "::=" "::="))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn forward_tokenizer_walks_a_real_module_and_preserves_consumed_lexemes() {
    let elisp_form = r##"(with-temp-buffer
          (insert "-- imported telemetry\n")
          (insert "People DEFINITIONS AUTOMATIC TAGS ::= BEGIN\n")
          (insert "IMPORTS Person, Address FROM Core { iso 1 };\n")
          (insert "Names ::= SEQUENCE OF UTF8String\n")
          (insert "flag ::= \"quoted value\"\n")
          (insert "doc ::= <value/>\nEND")
          (asn1-mode)
          (goto-char (point-min))
          (let (tokens)
            (while (< (point) (point-max))
              (let ((start (point))
                    (token (asn1-mode-forward-token)))
                (push
                 (list token
                       (buffer-substring-no-properties start (point))
                       start (point))
                 tokens)))
            (nreverse tokens)))"##;
    let expect = expect![[
        r#"OK (("_UCASE_ID" "-- imported telemetry\nPeople" 1 29) ("DEFINITIONS" " DEFINITIONS" 29 41) ("_TAG_KIND" " AUTOMATIC" 41 51) ("TAGS" " TAGS" 51 56) ("::=" " ::=" 56 60) ("BEGIN" " BEGIN" 60 66) ("IMPORTS" "\nIMPORTS" 66 74) ("_UCASE_ID" " Person" 74 81) ("," "," 81 82) ("_UCASE_ID" " Address" 82 90) ("FROM" " FROM" 90 95) ("_UCASE_ID" " Core" 95 100) ("_BRACE" " { iso 1 }" 100 110) (";" ";" 110 111) ("_UCASE_ID" "\nNames" 111 117) ("::=" " ::=" 117 121) ("_SET" " SEQUENCE OF" 121 133) ("_UCASE_ID" " UTF8String" 133 144) ("_LCASE_ID" "\nflag" 144 149) ("::=" " ::=" 149 153) ("_LITERAL" " \"quoted value\"" 153 168) ("_LCASE_ID" "\ndoc" 168 172) ("::=" " ::=" 172 176) ("_XML_SINGLE" " <value/>" 176 185) ("END" "\nEND" 185 189))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn backward_tokenizer_reverses_a_real_assignment_without_losing_boundaries() {
    let elisp_form = r##"(with-temp-buffer
          (insert "Result ::= SEQUENCE { value INTEGER, label UTF8String }")
          (asn1-mode)
          (goto-char (point-max))
          (let (tokens)
            (while (> (point) (point-min))
              (let ((end (point))
                    (token (asn1-mode-backward-token)))
                (push
                 (list token
                       (buffer-substring-no-properties (point) end)
                       (point) end)
                 tokens)))
            tokens))"##;
    let expect = expect![[
        r#"OK (("_UCASE_ID" "Result " 1 8) ("::=" "::= " 8 12) ("_SEQ" "SEQUENCE " 12 21) ("_BRACE" "{ value INTEGER, label UTF8String }" 21 56))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn tokenizers_are_case_sensitive_and_distinguish_identifiers_from_keywords() {
    let elisp_form = r##"(with-temp-buffer
          (insert "BEGIN begin Begin Upper lower &field 123")
          (asn1-mode)
          (goto-char (point-min))
          (let (tokens)
            (dotimes (_ 7)
              (push (asn1-mode-forward-token) tokens))
            (nreverse tokens)))"##;
    let expect = expect![[
        r#"OK ("BEGIN" "_LCASE_ID" "_UCASE_ID" "_UCASE_ID" "_LCASE_ID" "_LCASE_ID" "_LITERAL")"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn literal_brace_parenthesis_and_xml_opener_tokens_preserve_balanced_units() {
    let elisp_form = r##"(with-temp-buffer
          (insert "\"quoted text\" { alpha { beta } } (SIZE (1..8)) ")
          (insert "<outer><inner/></outer>")
          (asn1-mode)
          (goto-char (point-min))
          (let (tokens)
            (dotimes (_ 4)
              (let ((start (point))
                    (token (asn1-mode-forward-token)))
                (push
                 (cons token
                       (buffer-substring-no-properties start (point)))
                 tokens)))
            (nreverse tokens)))"##;
    let expect = expect![[
        r#"OK (("_LITERAL" . "\"quoted text\"") ("_BRACE" . " { alpha { beta } }") ("_PAREN" . " (SIZE (1..8))") ("_XML_OPENER" . " <outer>"))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn malformed_delimiters_make_forward_and_backward_tokenizers_progress_safely() {
    let elisp_form = r##"(list
          (with-temp-buffer
            (insert "\"unterminated")
            (asn1-mode)
            (goto-char (point-min))
            (list (asn1-mode-forward-token) (point)))
          (with-temp-buffer
            (insert "{unterminated")
            (asn1-mode)
            (goto-char (point-min))
            (list (asn1-mode-forward-token) (point)))
          (with-temp-buffer
            (insert "unterminated}")
            (asn1-mode)
            (goto-char (point-max))
            (list (asn1-mode-backward-token) (point))))"##;
    let expect = expect![[r#"OK (("\"" 2) ("{" 2) ("}" 13))"#]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn gdmo_registry_recognizes_real_multiword_template_and_support_tokens() {
    let elisp_form = r##"(mapcar
          (lambda (text)
            (let ((case-fold-search nil))
              (let ((matched
                     (string-match gdmo-mode-token-regexp text)))
                (list text
                      (and matched
                           (asn1-mode-token-match-group
                            (match-data)
                            gdmo-mode-token-alist))
                      (and matched
                           (match-string 0 text))))))
          '("MANAGED OBJECT CLASS" "DERIVED FROM"
            "NAMED BY SUPERIOR OBJECT CLASS" "WITH ATTRIBUTE"
            "REGISTERED AS" "DESCRIPTION" "ATTRIBUTE"
            "lowercase template"))"##;
    let expect = expect![[
        r#"OK (("MANAGED OBJECT CLASS" "_GDMO_OPEN" "MANAGED OBJECT CLASS") ("DERIVED FROM" "_GDMO_OPEN" "DERIVED FROM") ("NAMED BY SUPERIOR OBJECT CLASS" "_GDMO_OPEN" "NAMED BY SUPERIOR OBJECT CLASS") ("WITH ATTRIBUTE" "_GDMO_OPEN" "WITH ATTRIBUTE") ("REGISTERED AS" "_REGISTERED_AS" "REGISTERED AS") ("DESCRIPTION" "_GDMO_OPEN" "DESCRIPTION") ("ATTRIBUTE" nil nil) ("lowercase template" nil nil))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn backward_token_to_finds_the_real_assignment_boundary_from_nested_content() {
    let elisp_form = r##"(with-temp-buffer
          (insert "Person ::= SEQUENCE {\n")
          (insert "  name UTF8String,\n")
          (insert "  age INTEGER\n}")
          (asn1-mode)
          (goto-char (point-max))
          (asn1-mode-backward-token-to "::=")
          (list
           (point)
           (buffer-substring-no-properties
            (line-beginning-position) (line-end-position))
           (asn1-mode-forward-token)
           (point)))"##;
    let expect = expect![[r#"OK (8 "Person ::= SEQUENCE {" "::=" 11)"#]];
    assert_asn1_mode_parity(elisp_form, expect);
}
