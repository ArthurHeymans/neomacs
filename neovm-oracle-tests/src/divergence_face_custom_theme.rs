//! Divergence tests: faces deep - face remapping, face inheritance, theme.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_face_all_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (consp (face-all-attributes 'default))
  (consp (face-all-attributes 'bold))
  (plist-get (face-all-attributes 'default) :family))"#,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}

#[test]
fn divergence_face_remapping() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'face-remap-add-relative)
  (fboundp 'face-remap-remove-relative)
  (listp (get 'default 'face-remapping)))"#,
        expect_test::expect![[r#""OK (t nil t)""#]],
    );
}

#[test]
fn divergence_face_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (member 'default (face-list))
  (member 'bold (face-list))
  (>= (length (face-list)) 10))"#,
        expect_test::expect![[r#""OK ((default) (bold default) t)""#]],
    );
}

#[test]
fn divergence_face_underline_colors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (face-attribute 'underline :underline)
  (face-attribute 'highlight :background))"#,
        expect_test::expect![[r#""OK (t unspecified)""#]],
    );
}

#[test]
fn divergence_face_realized() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'face-spec-recalc)
  (fboundp 'face-spec-set)
  (facep 'my-test-face-xyz))"#,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}

#[test]
fn divergence_make_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (make-face 'my-test-face-123)
  (list (facep 'my-test-face-123)
        (face-attribute 'my-test-face-123 :family)))"#,
        expect_test::expect![[
            r#""OK ([face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] unspecified)""#
        ]],
    );
}

#[test]
fn divergence_face_inheritance() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (defface my-inherited-face '((t :inherit bold)) "test")
  (list (facep 'my-inherited-face)
        (plist-get (face-all-attributes 'my-inherited-face) :inherit)))"#,
        expect_test::expect![[
            r#""OK ([face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] nil)""#
        ]],
    );
}

#[test]
fn divergence_custom_theme_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'custom-set-faces)
  (fboundp 'custom-set-variables)
  (fboundp 'custom-theme-p)
  (fboundp 'load-theme)
  (fboundp 'enable-theme)
  (fboundp 'disable-theme))"#,
        expect_test::expect![[r#""OK (t t t t t t)""#]],
    );
}

#[test]
fn divergence_custom_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'customize-set-variable)
  (fboundp 'customize-save-variable)
  (fboundp 'custom-variable-p)
  (boundp 'custom-file)
  (or (null custom-file) (stringp custom-file)))"#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_defcustom_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (defcustom my-custom-var-xyz 42 "A test variable" :type 'integer)
  (list (custom-variable-p 'my-custom-var-xyz)
        my-custom-var-xyz
        (get 'my-custom-var-xyz 'custom-type)))"#,
        expect_test::expect![[r#""OK (((funcall #'(closure (t) nil 42))) 42 integer)""#]],
    );
}
