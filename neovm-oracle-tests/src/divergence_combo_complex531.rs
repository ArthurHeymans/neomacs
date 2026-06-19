/// Batch 531: face-attribute, face-all-attributes, face-font, color-related deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx531_face_attribute_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (face-attribute 'bold :weight nil 'default)
      (face-attribute 'italic :slant nil 'default))
"##,
    );
}

#[test]
fn div_cx531_face_all_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(car (face-all-attributes 'bold))
"##,
    );
}

#[test]
fn div_cx531_face_documentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(face-documentation 'bold)
"##,
    );
}

#[test]
fn div_cx531_face_font() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(face-font 'default)
"##,
    );
}

#[test]
fn div_cx531_color_name_to_rgb() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (color-name-to-rgb "red") (color-name-to-rgb "blue"))
"##,
    );
}

#[test]
fn div_cx531_color_rgb_to_hex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (color-rgb-to-hex 1.0 0 0) (color-rgb-to-hex 0 1.0 0))
"##,
    );
}

#[test]
fn div_cx531_color_distance() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (color-distance "red" "blue") (color-distance "red" "#ff0000"))
"##,
    );
}

#[test]
fn div_cx531_color_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (color-values "red") (color-values "#ff0000") (color-values "alice blue"))
"##,
    );
}

#[test]
fn div_cx531_color_gray() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (color-gray-p "gray50") (color-gray-p "red"))
"##,
    );
}

#[test]
fn div_cx531_color_supported() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (color-supported-p "red" nil t) (color-supported-p "#ff0000" t nil))
"##,
    );
}

#[test]
fn div_cx531_defined_colors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((colors (defined-colors))) (list (listp colors) (> (length colors) 10)))
"##,
    );
}

#[test]
fn div_cx531_face_inherit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-face 'cx531-test-face)))
  (set-face-attribute f nil :foreground "red" :inherit 'bold)
  (face-attribute f :foreground nil 'default))
"##,
    );
}

#[test]
fn div_cx531_face_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-face 'cx531-set-face)))
  (face-spec-set f '((t (:foreground "green"))) nil)
  (face-attribute f :foreground nil 'default))
"##,
    );
}

#[test]
fn div_cx531_face_reset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (make-face 'cx531-reset-face)))
  (face-spec-set f '((t (:foreground "blue"))) nil)
  (face-spec-reset-face f)
  (face-attribute f :foreground nil 'default))
"##,
    );
}

#[test]
fn div_cx531_color_name_to_rgb_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (color-name-to-rgb "red") (color-name-to-rgb "gainsboro") (color-name-to-rgb "teal"))
"##,
    );
}
