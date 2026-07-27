use expect_test::expect;

use super::assert_asn1_mode_parity;

#[test]
fn indentation_formats_a_complete_module_and_top_level_assignments() {
    let elisp_form = r##"(with-temp-buffer
          (insert "Telemetry DEFINITIONS AUTOMATIC TAGS ::= BEGIN\n")
          (insert "DeviceId ::= INTEGER\n")
          (insert "DeviceName ::= UTF8String\n")
          (insert "END\n")
          (asn1-mode)
          (indent-region (point-min) (point-max))
          (buffer-string))"##;
    let expect = expect![[
        r#"OK "Telemetry DEFINITIONS AUTOMATIC TAGS ::= BEGIN\n\11DeviceId ::= INTEGER\n\11DeviceName ::= UTF8String\n\11END\n""#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn indentation_aligns_exports_and_multi_source_imports_in_real_module_context() {
    let elisp_form = r##"(with-temp-buffer
          (insert "Network DEFINITIONS ::= BEGIN\n")
          (insert "EXPORTS Router, Interface;\n")
          (insert "IMPORTS\n")
          (insert "Address, Prefix FROM Core { iso 1 3 6 },\n")
          (insert "Counter FROM Metrics { iso 1 3 7 };\n")
          (insert "END\n")
          (asn1-mode)
          (indent-region (point-min) (point-max))
          (buffer-string))"##;
    let expect = expect![[
        r#"OK "Network DEFINITIONS ::= BEGIN\n\11EXPORTS Router, Interface;\n\11IMPORTS\n\11\11Address, Prefix FROM Core { iso 1 3 6 },\n\11\11Counter FROM Metrics { iso 1 3 7 };\n\11END\n""#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn indentation_nests_sequence_fields_and_nested_choice_members_practically() {
    let elisp_form = r##"(with-temp-buffer
          (insert "Model DEFINITIONS ::= BEGIN\n")
          (insert "Person ::= SEQUENCE {\n")
          (insert "name UTF8String,\n")
          (insert "contact CHOICE {\n")
          (insert "email IA5String,\n")
          (insert "phone NumericString\n")
          (insert "},\n")
          (insert "age INTEGER OPTIONAL\n")
          (insert "}\n")
          (insert "END\n")
          (asn1-mode)
          (indent-region (point-min) (point-max))
          (buffer-string))"##;
    let expect = expect![[
        r#"OK "Model DEFINITIONS ::= BEGIN\n\11Person ::= SEQUENCE {\n\11\11\11\11\11\11name UTF8String,\n\11contact CHOICE {\n\11\11\11\11   email IA5String,\n\11\11\11\11   phone NumericString\n\11},\n\11age INTEGER OPTIONAL\n\11}\n\11END\n""#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn indentation_handles_sequence_of_size_constraints_and_enumerated_values() {
    let elisp_form = r##"(with-temp-buffer
          (insert "Types DEFINITIONS ::= BEGIN\n")
          (insert "Names ::= SEQUENCE SIZE (1..8) OF UTF8String\n")
          (insert "State ::= ENUMERATED {\n")
          (insert "idle (0),\n")
          (insert "running (1),\n")
          (insert "failed (2)\n")
          (insert "}\n")
          (insert "END\n")
          (asn1-mode)
          (indent-region (point-min) (point-max))
          (buffer-string))"##;
    let expect = expect![[
        r#"OK "Types DEFINITIONS ::= BEGIN\n\11Names ::= SEQUENCE SIZE (1..8) OF UTF8String\n\11\11\11\11\11\11\11\11\11  State ::= ENUMERATED {\n\11\11\11\11\11\11\11\11\11\11\11\11\11\11   idle (0),\n\11\11\11\11\11\11\11\11\11  running (1),\n\11\11\11\11\11\11\11\11\11  failed (2)\n\11\11\11\11\11\11\11\11\11  }\n\11END\n""#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn indentation_preserves_comments_and_aligns_multiline_xml_values() {
    let elisp_form = r##"(with-temp-buffer
          (insert "Xml DEFINITIONS ::= BEGIN\n")
          (insert "-- a real XML value\n")
          (insert "record ::= <record>\n")
          (insert "<name>Ada</name>\n")
          (insert "<active><true/></active>\n")
          (insert "</record>\n")
          (insert "END\n")
          (asn1-mode)
          (indent-region (point-min) (point-max))
          (buffer-string))"##;
    let expect = expect![[
        r#"OK "Xml DEFINITIONS ::= BEGIN\n\11-- a real XML value\n\11record ::= <record>\n\11\11\11<name>Ada</name>\n\11\11\11<active><true/></active>\n\11\11</record>\n\11END\n""#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn gdmo_indentation_formats_managed_object_package_and_registration_blocks() {
    let elisp_form = r##"(with-temp-buffer
          (insert "router MANAGED OBJECT CLASS\n")
          (insert "DERIVED FROM networkElement;\n")
          (insert "CHARACTERIZED BY routerPackage;\n")
          (insert "REGISTERED AS { iso 3 6 1 };\n")
          (insert "routerPackage PACKAGE\n")
          (insert "ATTRIBUTES routerName GET-REPLACE;\n")
          (insert "BEHAVIOUR routerBehaviour;\n")
          (gdmo-mode)
          (indent-region (point-min) (point-max))
          (buffer-string))"##;
    let expect = expect![[
        r#"OK "router MANAGED OBJECT CLASS\nDERIVED FROM networkElement;\nCHARACTERIZED BY routerPackage;\nREGISTERED AS { iso 3 6 1 };\nrouterPackage PACKAGE\nATTRIBUTES routerName GET-REPLACE;\nBEHAVIOUR routerBehaviour;\n""#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn repeated_whole_buffer_indentation_reaches_a_stable_fixed_point() {
    let elisp_form = r##"(with-temp-buffer
          (insert "Stable DEFINITIONS ::= BEGIN\n")
          (insert "Item ::= SEQUENCE {\n")
          (insert "id INTEGER,\n")
          (insert "tags SET OF UTF8String\n")
          (insert "}\nEND\n")
          (asn1-mode)
          (indent-region (point-min) (point-max))
          (let ((first (buffer-string)))
            (indent-region (point-min) (point-max))
            (list first (buffer-string)
                  (equal first (buffer-string)))))"##;
    let expect = expect![[
        r#"OK ("Stable DEFINITIONS ::= BEGIN\n\11Item ::= SEQUENCE {\n\11\11\11\11\11  id INTEGER,\n\11tags SET OF UTF8String\n\11}\n\11END\n" "Stable DEFINITIONS ::= BEGIN\n\11Item ::= SEQUENCE {\n\11\11\11\11\11  id INTEGER,\n\11tags SET OF UTF8String\n\11}\n\11END\n" t)"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn narrowing_indentation_changes_only_the_selected_nested_assignment() {
    let elisp_form = r##"(with-temp-buffer
          (insert "prefix-not-asn1\n")
          (let ((start (point)))
            (insert "Scoped DEFINITIONS ::= BEGIN\n")
            (insert "Entry ::= SEQUENCE {\n")
            (insert "value INTEGER\n")
            (insert "}\nEND\n")
            (let ((end (point)))
              (insert "suffix-not-asn1\n")
              (asn1-mode)
              (save-restriction
                (narrow-to-region start end)
                (indent-region (point-min) (point-max)))
              (list
               (buffer-string)
               start end))))"##;
    let expect = expect![[
        r#"OK ("prefix-not-asn1\nScoped DEFINITIONS ::= BEGIN\n\11Entry ::= SEQUENCE {\n\11\11\11\11\11   value INTEGER\n\11}\n\11END\nsuffix-not-asn1\n" 17 87)"#
    ]];
    assert_asn1_mode_parity(elisp_form, expect);
}

#[test]
fn indent_line_preserves_logical_cursor_offset_inside_a_nested_field() {
    let elisp_form = r##"(with-temp-buffer
          (insert "Cursor DEFINITIONS ::= BEGIN\n")
          (insert "Record ::= SEQUENCE {\n")
          (insert "value INTEGER\n")
          (insert "}\nEND\n")
          (asn1-mode)
          (goto-char (point-min))
          (search-forward "value")
          (backward-char 2)
          (let ((before (current-column)))
            (indent-according-to-mode)
            (list before
                  (current-column)
                  (buffer-substring-no-properties
                   (line-beginning-position)
                   (line-end-position))
                  (char-after))))"##;
    let expect = expect![[r#"OK (3 23 "\11\11\11\11\11value INTEGER" 117)"#]];
    assert_asn1_mode_parity(elisp_form, expect);
}
