//! UTF-8 / multibyte *fill, case-region & category* divergence probes.
//!
//! Probes text layout (`fill-region` / `fill-paragraph`) over multibyte —
//! which depends on display-width word wrapping — plus case-region operations
//! and category-table modification.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_utf8_fill_region_ascii_baseline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (let ((fill-column 10))
    (insert "the quick brown fox jumps over\n")
    (fill-region (point-min) (point-max))
    (buffer-string)))
"#,
    );
}

#[test]
fn div_utf8_fill_region_latin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (let ((fill-column 12))
    (insert "café thé résumé hello world greeting\n")
    (fill-region (point-min) (point-max))
    (buffer-string)))
"#,
    );
}

#[test]
fn div_utf8_fill_paragraph_cjk_display_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // CJK fill must account for display width (each CJK char = 2 columns).
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (let ((fill-column 20))
    (insert "你好世界 这是测试 hello world wide line here\n")
    (fill-paragraph)
    (buffer-string)))
"#,
    );
}

#[test]
fn div_utf8_case_region_ops_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list
 (with-temp-buffer
   (insert "Café Résumé Straße\n")
   (downcase-region (point-min) (point-max))
   (buffer-string))
 (with-temp-buffer
   (insert "café résumé\n")
   (capitalize-region (point-min) (point-max))
   (buffer-string)))
"#,
    );
}

#[test]
fn div_utf8_modify_category_entry() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((tbl (category-table)))
  (modify-category-entry ?é ?l tbl t)
  (modify-category-entry ?\x3042 ?l tbl t)
  (list (char-in-category-p ?é ?l tbl)
        (char-in-category-p ?\x3042 ?l tbl)
        (char-in-category-p ?a ?l tbl)))
"#,
    );
}

#[test]
fn div_utf8_center_line_and_tab_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(with-temp-buffer
  (let ((fill-column 40))
    (insert "café 世界\n")
    (center-line)
    (buffer-string)))
"#,
    );
}
