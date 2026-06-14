//! Source-audit divergences: casefiddle / casetab / category / syntax / width.
//!
//! From a direct GNU src vs neovm-core Rust audit: case operations ignore the
//! installed case table (hardcoded Rust mapping), word boundaries use
//! is_alphanumeric() not the syntax table, char-width table mutations are
//! ignored, and several special-case mappings differ (ß, İ, Greek final sigma).

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_ac_case_table_ignored_for_downcase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Buffer-local case table mapping A→x; GNU reads it, Neomacs ignores it.
    assert_oracle_parity(
        r##"
(let ((ct (copy-case-table)))
  (set-char-table-range ct ?A ?x)
  (set-case-table ct)
  (downcase "A"))
"##,
    );
}

#[test]
fn div_ac_case_table_ignored_for_upcase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ct (copy-case-table)))
  (set-char-table-range ct ?a ?X)
  (set-case-table ct)
  (upcase "a"))
"##,
    );
}

#[test]
fn div_ac_upcase_sharp_s_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // char-upcase of ß: GNU returns ß unchanged (SS is multi-char),
    // Neomacs maps ß→ẞ (7838).
    assert_oracle_parity(r##"(upcase ?ß)"##);
}

#[test]
fn div_ac_downcase_dotted_I_char() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // downcase of İ (U+0130): GNU unchanged (one-to-many), Neomacs → i (105).
    assert_oracle_parity(r##"(downcase ?İ)"##);
}

#[test]
fn div_ac_greek_final_sigma_downcase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Σ at end of word → ς (final sigma) in GNU; Neomacs → σ always.
    assert_oracle_parity(r##"(downcase "ΑΣ")"##);
}

#[test]
fn div_ac_upcase_strasse_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(upcase "straße")"##);
}

#[test]
fn div_ac_with_case_table_missing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (with-case-table (let ((ct (copy-case-table)))
                       (set-char-table-range ct ?a ?B) ct)
      (downcase "a"))
  (error (car e)))
"##,
    );
}

#[test]
fn div_ac_case_symbols_as_words() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // GNU: _ is word-constituent with case-symbols-as-words -> foo_bar one word.
    assert_oracle_parity(
        r##"
(let ((case-symbols-as-words t))
  (capitalize "foo_bar baz"))
"##,
    );
}

#[test]
fn div_ac_forward_sexp_syntax_text_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Override syntax of "(" to word-constituent via text property.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "ab(cd)ef")
  (put-text-property 3 4 'syntax-table (string-to-syntax "w"))
  (goto-char 1)
  (let ((parse-sexp-lookup-properties t))
    (forward-sexp 1)
    (point)))
"##,
    );
}

#[test]
fn div_ac_char_width_table_mutation_ignored() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // GNU consults the (mutable) char-width-table; Neomacs hardcodes width.
    assert_oracle_parity(
        r##"
(let ((cw (char-width ?\x300)))
  (set-char-table-range (char-width-table) ?\x300 1)
  (list cw (char-width ?\x300)))
"##,
    );
}

#[test]
fn div_ac_display_table_string_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // string-width should honor buffer-display-table glyph replacement.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (setq buffer-display-table (make-display-table))
  (aset (char-table-extra-slot buffer-display-table 0) ?a (vector ?X ?Y))
  (string-width "a"))
"##,
    );
}

#[test]
fn div_ac_standard_category_docstring() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (category-docstring ?l (standard-category-table))
      (category-docstring ?a (standard-category-table))
      (category-docstring ?r (standard-category-table)))
"##,
    );
}

#[test]
fn div_ac_make_category_set_uppercase_letter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Category letter "A" -> bit position; uppercase letters map to bits 27-52.
    assert_oracle_parity(r##"(aref (make-category-set "A") 28)"##);
}

#[test]
fn div_ac_char_width_display_property_in_column() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // char-width text property / display glyph affecting column accounting.
    assert_oracle_parity(
        r##"
(with-temp-buffer
  (insert "x")
  (put-text-property 1 2 'display (vector ?a ?b ?c))
  (list (current-column) (string-width (buffer-substring 1 2))))
"##,
    );
}
