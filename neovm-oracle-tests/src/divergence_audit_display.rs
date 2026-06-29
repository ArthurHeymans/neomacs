//! Display/indent-engine divergence probes (indent.rs scan_for_column + friends).
//!
//! Confirmed systematic pattern: Neomacs column-accounting (current-column,
//! move-to-column, indent-to) ignores display-affecting constructs that GNU
//! honors — the `display` text property (glyph width), `buffer-display-table`
//! glyph replacement, and composition (multiple chars → one glyph). Source:
//! GNU indent.c current_column_1 vs neovm-core indent.rs scan_for_column.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_adisp_current_column_display_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "x")
  (put-text-property 1 2 'display "abc")
  (current-column))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_adisp_move_to_column_display_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (put-text-property 1 3 'display (make-string 8 88))
  (move-to-column 5)
  (current-column))
"##,
        expect_test::expect![[r#""OK 8""#]],
    );
}

#[test]
fn div_adisp_current_column_display_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abc")
  (let ((dt (make-display-table)))
    (aset dt ?a (vector 88 88))
    (setq buffer-display-table dt))
  (current-column))
"##,
        expect_test::expect![[r#""OK 4""#]],
    );
}

#[test]
fn div_adisp_current_column_composition() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abc")
  (compose-region 1 3 "")
  (current-column))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_adisp_move_to_column_display_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abcde")
  (let ((dt (make-display-table)))
    (aset dt ?a (vector 88 88 88))
    (setq buffer-display-table dt))
  (move-to-column 3)
  (current-column))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_adisp_indent_to_after_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "ab")
  (put-text-property 1 2 'display (vector ?x ?x ?x ?x))
  (indent-to 8)
  (current-column))
"##,
        expect_test::expect![[r#""OK 8""#]],
    );
}

#[test]
fn div_adisp_current_column_display_integer_height() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // (height N) display spec changes line height, not width — column unaffected.
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abc")
  (put-text-property 2 3 'display '(height 2.0))
  (current-column))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_adisp_current_column_display_slice() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (put-text-property 2 4 'display (make-string 5 90))
  (current-column))
"##,
        expect_test::expect![[r#""OK 9""#]],
    );
}

#[test]
fn div_adisp_string_width_display_control() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Control: string-width measures actual chars (display affects rendering only).
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((s (propertize "x" 'display "abcde")))
  (string-width s))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_adisp_current_column_multiple_display_glyphs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "aXb")
  (put-text-property 2 3 'display "1234")
  (current-column))
"##,
        expect_test::expect![[r#""OK 6""#]],
    );
}

#[test]
fn div_adisp_current_column_invisible_text() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // Invisible text contributes 0 columns.
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (put-text-property 2 5 'invisible t)
  (current-column))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_adisp_move_to_column_with_tabs() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "a\tb")
  (move-to-column 8)
  (current-column))
"##,
        expect_test::expect![[r#""OK 8""#]],
    );
}
