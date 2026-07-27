use expect_test::expect;

use super::assert_asn1_mode_parity;

#[test]
fn syntax_table_classifies_language_punctuation_strings_comments_and_xml_pairs() {
    let elisp_form = r##"(with-syntax-table asn1-mode-syntax-table
          (mapcar
           (lambda (character)
             (list character
                   (char-syntax character)
                   (string (char-syntax character))))
           '(?A ?0 ?- ?& ?? ?! ?\" ?' ?\( ?\) ?\{ ?\}
             ?\[ ?\] ?< ?> ?/ ?* ?\n ?\s ?\t)))"##;
    let expect = expect![[
        r#"OK ((65 119 "w") (48 119 "w") (45 119 "w") (38 119 "w") (63 119 "w") (33 46 ".") (34 34 "\"") (39 34 "\"") (40 40 "(") (41 41 ")") (123 40 "(") (125 41 ")") (91 40 "(") (93 41 ")") (60 40 "(") (62 41 ")") (47 46 ".") (42 46 ".") (10 62 ">") (32 32 " ") (9 32 " "))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn syntax_parser_distinguishes_dash_block_comments_and_both_literal_styles() {
    let elisp_form = r##"(with-temp-buffer
          (insert "alpha -- line comment\n")
          (insert "\"double literal\" 'single literal'\n")
          (insert "beta /* block comment */ gamma")
          (asn1-mode)
          (let (positions)
            (goto-char (point-min))
            (search-forward "line")
            (push (list :dash-comment (nth 4 (syntax-ppss))) positions)
            (search-forward "double")
            (push (list :double-string (nth 3 (syntax-ppss))) positions)
            (search-forward "single")
            (push (list :single-string (nth 3 (syntax-ppss))) positions)
            (search-forward "block")
            (push (list :block-comment (nth 4 (syntax-ppss))) positions)
            (search-forward "gamma")
            (push (list :after-comment (nth 4 (syntax-ppss))) positions)
            (nreverse positions)))"##;
    let expect = expect![
        "OK ((:dash-comment t) (:double-string 34) (:single-string 39) (:block-comment t) (:after-comment nil))"
    ];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn word_motion_treats_hyphen_ampersand_and_question_mark_as_identifier_content() {
    let elisp_form = r##"(with-temp-buffer
          (insert "alpha-beta &field ?choice plain_name")
          (asn1-mode)
          (goto-char (point-min))
          (let (words)
            (while (re-search-forward "\\w+" nil t)
              (push (match-string-no-properties 0) words))
            (nreverse words)))"##;
    let expect = expect![[r#"OK ("alpha-beta" "&field" "?choice" "plain" "name")"#]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn xml_angle_brackets_and_nested_delimiters_expose_expected_scan_boundaries() {
    let elisp_form = r##"(with-temp-buffer
          (insert "<value>{[item(one)]}</value>")
          (asn1-mode)
          (goto-char (point-min))
          (let ((xml-end (scan-sexps (point) 1)))
            (search-forward "{")
            (backward-char)
            (let ((brace-end (scan-sexps (point) 1)))
              (list xml-end
                    brace-end
                    (buffer-substring-no-properties 1 xml-end)
                    (car (syntax-ppss (1- brace-end)))))))"##;
    let expect = expect![[r#"OK (8 21 "<value>" 1)"#]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn asn1_mode_initializes_all_documented_buffer_local_editor_settings() {
    let elisp_form = r##"(with-temp-buffer
          (asn1-mode)
          (list
           major-mode mode-name
           (eq (syntax-table) asn1-mode-syntax-table)
           (eq local-abbrev-table asn1-mode-abbrev-table)
           parse-sexp-ignore-comments tab-width
           comment-start comment-end comment-start-skip
           outline-regexp outline-level
           imenu-generic-expression font-lock-defaults
           (local-variable-p 'smie-forward-token-function)
           (local-variable-p 'smie-backward-token-function)))"##;
    let expect = expect![[
        r#"OK (asn1-mode "ASN.1" t t t 4 "--" "" nil "-- +[0-9]+\\(\\.[0-9]+\\)* " asn1-mode-outline-level ((nil "^\\([A-Za-z-_]+\\).*::=.*" 1)) (asn1-mode-font-lock-keywords nil nil) t t)"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn gdmo_mode_shares_parser_contract_but_selects_its_extended_font_lock_rules() {
    let elisp_form = r##"(with-temp-buffer
          (gdmo-mode)
          (list
           major-mode mode-name
           (derived-mode-p 'prog-mode)
           (eq (syntax-table) asn1-mode-syntax-table)
           (eq local-abbrev-table asn1-mode-abbrev-table)
           font-lock-defaults
           comment-start tab-width
           smie-forward-token-function
           smie-backward-token-function))"##;
    let expect = expect![[
        r#"OK (gdmo-mode "GDMO" prog-mode t t (gdmo-mode-font-lock-keywords nil nil) "--" 4 asn1-mode-forward-token asn1-mode-backward-token)"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn asn1_font_lock_marks_real_assignments_keywords_and_numeric_constants() {
    let elisp_form = r##"(with-temp-buffer
          (insert "People DEFINITIONS AUTOMATIC TAGS ::= BEGIN\n")
          (insert "age INTEGER ::= (42)\n")
          (insert "Name ::= UTF8String\nEND\n")
          (asn1-mode)
          (font-lock-ensure)
          (let ((position (point-min))
                runs)
            (while (< position (point-max))
              (let* ((face (get-text-property position 'face))
                     (next (next-single-property-change
                            position 'face nil (point-max))))
                (when face
                  (push
                   (list
                    (buffer-substring-no-properties position next)
                    face)
                   runs))
                (setq position next)))
            (nreverse runs)))"##;
    let expect = expect![[
        r#"OK (("People" font-lock-variable-name-face) ("DEFINITIONS" font-lock-keyword-face) ("AUTOMATIC" font-lock-keyword-face) ("TAGS" font-lock-keyword-face) ("BEGIN" font-lock-keyword-face) ("age" font-lock-variable-name-face) ("INTEGER" font-lock-keyword-face) ("(42)" font-lock-constant-face) ("Name" font-lock-variable-name-face) ("UTF8String" font-lock-keyword-face) ("END" font-lock-keyword-face))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn gdmo_font_lock_distinguishes_template_names_keywords_and_assignment_names() {
    let elisp_form = r##"(with-temp-buffer
          (insert "router MANAGED OBJECT CLASS\n")
          (insert "DERIVED FROM baseClass;\n")
          (insert "alarmPackage PACKAGE\n")
          (insert "BEHAVIOUR alarmBehaviour;\n")
          (insert "REGISTERED AS { 1 3 6 1 };\n")
          (gdmo-mode)
          (font-lock-ensure)
          (let ((position (point-min))
                runs)
            (while (< position (point-max))
              (let* ((face (get-text-property position 'face))
                     (next (next-single-property-change
                            position 'face nil (point-max))))
                (when face
                  (push
                   (list
                    (buffer-substring-no-properties position next)
                    face)
                   runs))
                (setq position next)))
            (nreverse runs)))"##;
    let expect = expect![[
        r#"OK (("router" font-lock-function-name-face) ("OBJECT" font-lock-keyword-face) ("CLASS" font-lock-keyword-face) ("FROM" font-lock-keyword-face) ("alarmPackage" font-lock-function-name-face))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn imenu_extracts_real_type_and_value_assignments_in_source_order() {
    let elisp_form = r##"(progn
          (require 'imenu)
          (with-temp-buffer
          (insert "People DEFINITIONS ::= BEGIN\n")
          (insert "Person ::= SEQUENCE { name UTF8String }\n")
          (insert "answer INTEGER ::= 42\n")
          (insert "END\n")
          (asn1-mode)
          (mapcar
           (lambda (entry)
             (cons (car entry)
                   (if (markerp (cdr entry))
                     (marker-position (cdr entry))
                     (cdr entry))))
           (imenu--make-index-alist t))))"##;
    let expect =
        expect![[r#"OK (("*Rescan*" . -99) ("People" . 1) ("Person" . 30) ("answer" . 70))"#]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn normal_mode_selection_uses_real_asn1_and_gdmo_file_names() {
    let elisp_form = r##"(mapcar
          (lambda (name)
            (with-temp-buffer
              (setq buffer-file-name
                    (expand-file-name name default-directory))
              (normal-mode)
              (list name major-mode mode-name)))
          '("telemetry.asn1" "objects.gdmo" "notes.txt"))"##;
    let expect = expect![[
        r#"OK (("telemetry.asn1" asn1-mode "ASN.1") ("objects.gdmo" gdmo-mode "GDMO") ("notes.txt" text-mode "Text"))"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}
