//! Strict combo oracle probes, batch 93: display-table glyph entries (setting
//! and reading actual character→glyph mappings) and window-display-table.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_q7_display_table_glyph_entries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((dt (make-display-table)))
  (aset dt ?a [?A])
  (aset dt ?\n [?B])
  (aset dt 128 [?X ?Y])
  (list (aref dt ?a)
        (aref dt ?\n)
        (aref dt ?b)
        (aref dt 128)
        (length (aref dt 128))
        (char-table-p dt)))
"##,
    );
}

#[test]
fn div_q7_window_display_table() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((dt (make-display-table)))
  (aset dt ?a [?A])
  (set-window-display-table nil dt)
  (list (eq (window-display-table) dt)
        (aref (window-display-table) ?a)
        (aref (window-display-table) ?b)))
"##,
    );
}

#[test]
fn div_q7_standard_display_table_entries() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((dt (standard-display-table)))
  (list (null dt)
        (if dt (aref dt ?\t) nil)
        (if dt (aref dt ?\n) nil)
        (if dt (char-table-p dt) nil)))
"##,
    );
}
