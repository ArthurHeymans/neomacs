use expect_test::expect;

use super::assert_agda_editor_tactics_parity;

#[test]
fn agda_editor_tactics_indent_counts_leading_spaces_across_real_declarations() {
    let elisp_form = r##"(mapcar
         (lambda (line)
           (cons line (agda-editor-tactics-indent line)))
         '("record R : Set where"
           " field"
           "  field"
           "    value : Set"
           "        proof = refl"))"##;
    let expect = expect![[
        r#"OK (("record R : Set where" . 1) (" field" . 1) ("  field" . 2) ("    value : Set" . 4) ("        proof = refl" . 8))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}

#[test]
fn agda_editor_tactics_indent_distinguishes_spaces_tabs_and_empty_lines() {
    let elisp_form = r##"(mapcar
         (lambda (line)
           (list
            (prin1-to-string line)
            (agda-editor-tactics-indent line)))
         '("" "value : Set" "\tvalue : Set" " \tvalue : Set"
           "   " "\t" "  λ x → x"))"##;
    let expect = expect![[
        r#"OK (("\"\"" 0) ("\"value : Set\"" 1) ("\"\\11value : Set\"" 1) ("\" \\11value : Set\"" 1) ("\"   \"" 3) ("\"\\11\"" 0) ("\"  λ x → x\"" 2))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}

#[test]
fn agda_editor_tactics_indent_handles_unicode_and_internal_whitespace() {
    let elisp_form = r##"(mapcar
         (lambda (line)
           (list line (agda-editor-tactics-indent line)))
         '("  Σ-value : Set ℓ"
           "      _∙_ : Carrier → Carrier → Carrier"
           "  law : ∀ x → x ∙ ε ≡ x"
           "    spaced   internally"
           "  "))"##;
    let expect = expect![[
        r#"OK (("  Σ-value : Set ℓ" 2) ("      _∙_ : Carrier → Carrier → Carrier" 6) ("  law : ∀ x → x ∙ ε ≡ x" 2) ("    spaced   internally" 4) ("  " 2))"#
    ]];
    assert_agda_editor_tactics_parity(elisp_form, expect);
}
