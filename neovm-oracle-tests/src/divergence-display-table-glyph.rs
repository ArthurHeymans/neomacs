//! Divergence tests: display-table, glyphless-char, composition.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_display_table_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((dt (standard-display-table)))
  (list (or (null dt) (char-table-p dt))
        (char-table-p (make-display-table))
        (fboundp 'make-display-table)))"#,
    );
}

#[test]
fn divergence_display_table_set_glyph() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((dt (make-display-table)))
  (aset dt ?\t (vector ?\xBB ?\t))
  (list (aref dt ?\t)
        (vectorp (aref dt ?\t))))"#,
    );
}

#[test]
fn divergence_glyphless_char_display() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (boundp 'glyphless-char-display)
  (char-table-p glyphless-char-display)
  (aref glyphless-char-display #x80))"#,
    );
}

#[test]
fn divergence_glyphless_char_method() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'glyphless-char-display)
  (member 'zero-width (list 'zero-width 'thin-space 'empty-box 'acronym 'text))
  (boundp 'glyphless-char-display-control))"#,
    );
}

#[test]
fn divergence_composition_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'compose-region-internal)
  (fboundp 'find-composition)
  (fboundp 'compose-string)
  (fboundp 'decompose-region)
  (fboundp 'decompose-string))"#,
    );
}

#[test]
fn divergence_display_pixels_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'display-pixel-width)
  (fboundp 'display-pixel-height)
  (fboundp 'display-mm-width)
  (fboundp 'display-mm-height)
  (fboundp 'display-backing-store)
  (fboundp 'display-save-under))"#,
    );
}

#[test]
fn divergence_face_color() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        "(list\n  (consp (color-values \"red\"))\n  (consp (color-values \"#FF0000\"))\n  (color-defined-p \"red\")\n  (color-defined-p \"nonexistent-color-xyz\"))",
    );
}

#[test]
fn divergence_face_fonts() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'x-list-fonts)
  (fboundp 'font-family-list)
  (listp (font-family-list)))"#,
    );
}
