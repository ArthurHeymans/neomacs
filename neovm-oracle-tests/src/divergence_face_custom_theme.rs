//! Divergence tests: faces deep - face remapping, face inheritance, theme.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_face_all_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (consp (face-all-attributes 'default))
  (consp (face-all-attributes 'bold))
  (plist-get (face-all-attributes 'default) :family))"#,
    );
}

#[test]
fn divergence_face_remapping() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'face-remap-add-relative)
  (fboundp 'face-remap-remove-relative)
  (listp (get 'default 'face-remapping)))"#,
    );
}

#[test]
fn divergence_face_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (member 'default (face-list))
  (member 'bold (face-list))
  (>= (length (face-list)) 10))"#,
    );
}

#[test]
fn divergence_face_underline_colors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (face-attribute 'underline :underline)
  (face-attribute 'highlight :background))"#,
    );
}

#[test]
fn divergence_face_realized() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'face-spec-recalc)
  (fboundp 'face-spec-set)
  (facep 'my-test-face-xyz))"#,
    );
}

#[test]
fn divergence_make_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (make-face 'my-test-face-123)
  (list (facep 'my-test-face-123)
        (face-attribute 'my-test-face-123 :family)))"#,
    );
}

#[test]
fn divergence_face_inheritance() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defface my-inherited-face '((t :inherit bold)) "test")
  (list (facep 'my-inherited-face)
        (plist-get (face-all-attributes 'my-inherited-face) :inherit)))"#,
    );
}

#[test]
fn divergence_custom_theme_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'custom-set-faces)
  (fboundp 'custom-set-variables)
  (fboundp 'custom-theme-p)
  (fboundp 'load-theme)
  (fboundp 'enable-theme)
  (fboundp 'disable-theme))"#,
    );
}

#[test]
fn divergence_custom_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'customize-set-variable)
  (fboundp 'customize-save-variable)
  (fboundp 'custom-variable-p)
  (boundp 'custom-file)
  (or (null custom-file) (stringp custom-file)))"#,
    );
}

#[test]
fn divergence_defcustom_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defcustom my-custom-var-xyz 42 "A test variable" :type 'integer)
  (list (custom-variable-p 'my-custom-var-xyz)
        my-custom-var-xyz
        (get 'my-custom-var-xyz 'custom-type)))"#,
    );
}
