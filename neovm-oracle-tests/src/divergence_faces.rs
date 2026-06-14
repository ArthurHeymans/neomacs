//! Face & color divergence probes.
//!
//! Probes face lifecycle (make-face/defface/set-face-attribute/face-attribute),
//! inheritance, face-bold/italic/underline-p, face-all-attributes, face
//! documentation, face-list size, and color functions (color-defined-p,
//! defined-colors, color-values, color-rgb-to-hex, color-distance). Faces and
//! colors are resolved against the batch tty frame, identical in both engines.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_face_make_face_basics() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((f (make-face 'neo-face-x)))
  (list (facep f) (facep 'neo-face-x) (face-name f) (integerp (face-id f))))
"#,
    );
}

#[test]
fn div_face_set_attribute_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((f (make-face 'neo-face-y)))
  (set-face-attribute f nil :foreground "red" :weight 'bold :slant 'italic)
  (list (face-attribute f :foreground)
        (face-attribute f :weight)
        (face-attribute f :slant)))
"#,
    );
}

#[test]
fn div_face_defface_attribute() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(progn
  (defface neo-defface-x '((t :foreground "blue" :weight bold)) "doc")
  (list (facep 'neo-defface-x)
        (face-attribute 'neo-defface-x :foreground)
        (face-attribute 'neo-defface-x :weight)))
"#,
    );
}

#[test]
fn div_face_inheritance_resolves() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(progn
  (defface neo-parent-face '((t :foreground "green")) "doc")
  (defface neo-child-face '((t :inherit neo-parent-face :weight bold)) "doc")
  (list (face-attribute 'neo-child-face :foreground)
        (face-attribute 'neo-child-face :weight)
        (face-attribute 'neo-child-face :inherit)))
"#,
    );
}

#[test]
fn div_face_bold_italic_underline_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((f (make-face 'neo-biu-face)))
  (set-face-attribute f nil :weight 'bold :slant 'italic :underline t)
  (list (face-bold-p f) (face-italic-p f) (face-underline-p f)))
"#,
    );
}

#[test]
fn div_face_default_attributes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (face-attribute 'default :foreground)
      (face-attribute 'default :background)
      (face-attribute 'default :weight)
      (face-attribute 'bold :weight)
      (face-attribute 'italic :slant))
"#,
    );
}

#[test]
fn div_face_all_attributes_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((f (make-face 'neo-all-attr)))
  (set-face-attribute f nil :foreground "red" :weight 'bold)
  (face-all-attributes f (selected-frame)))
"#,
    );
}

#[test]
fn div_face_documentation_builtins() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (face-documentation 'bold)
      (face-documentation 'default)
      (face-documentation 'highlight))
"#,
    );
}

#[test]
fn div_face_list_count_and_known_faces() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (facep 'default) (facep 'bold) (facep 'region)
      (facep 'font-lock-keyword-face)
      (length (face-list)))
"#,
    );
}

#[test]
fn div_color_defined_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        "(list (color-defined-p \"red\") (color-defined-p \"blue\") (color-defined-p \"nonexistent\") (color-defined-p \"#ff0000\") (color-defined-p \"#000000\"))",
    );
}

#[test]
fn div_color_values_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        "(list (color-values \"red\") (color-values \"#00ff00\") (color-values \"black\") (color-values \"white\"))",
    );
}

#[test]
fn div_color_rgb_hex_distance() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        "(list (color-rgb-to-hex 1 0 0) (color-rgb-to-hex 0.5 0.5 0.5) (color-distance \"red\" \"blue\") (color-distance \"#000000\" \"#ffffff\"))",
    );
}

#[test]
fn div_color_defined_colors_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(list (length (defined-colors))
      (member "red" (defined-colors))
      (member "black" (defined-colors)))
"#,
    );
}

#[test]
fn div_face_unspecified_and_reset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r#"
(let ((f (make-face 'neo-unspec-face)))
  (list (eq (face-attribute f :foreground) 'unspecified)
        (eq (face-attribute f :weight) 'unspecified)
        (set-face-attribute f nil :foreground nil)
        (eq (face-attribute f :foreground) 'unspecified)))
"#,
    );
}
