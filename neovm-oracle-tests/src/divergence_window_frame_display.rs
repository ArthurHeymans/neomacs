//! Divergence tests: window, frame, display, and face attributes.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_window_basic_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((w (selected-window)))
  (list (windowp w)
        (window-live-p w)
        (window-buffer w)
        (window-point w)
        (window-start w)
        (window-valid-p w)))"#,
        expect_test::expect![[r#""OK (t t #<buffer *scratch*> 1 1 t)""#]],
    );
}

#[test]
fn divergence_window_dimensions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((w (selected-window)))
  (list (> (window-total-width w) 0)
        (> (window-total-height w) 0)
        (> (window-body-width w) 0)
        (> (window-body-height w) 0)
        (>= (window-hscroll w) 0)))"#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_window_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((w (selected-window)))
  (list (consp (window-edges w))
        (consp (window-inside-edges w))
        (length (window-edges w))))"#,
        expect_test::expect![[r#""OK (t t 4)""#]],
    );
}

#[test]
fn divergence_window_configuration() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((cfg (current-window-configuration)))
  (list (window-configuration-p cfg)
        (framep (window-configuration-frame cfg))
        (eq (window-configuration-frame cfg) (selected-frame))))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_frame_basic_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((f (selected-frame)))
  (list (framep f)
        (frame-live-p f)
        (frame-visible-p f)
        (stringp (frame-parameter f 'name))
        (frame-parameter f 'minibuffer)))"#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_frame_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(let ((f (selected-frame)))
  (list (consp (frame-parameters f))
        (assq 'name (frame-parameters f))
        (assq 'foreground-color (frame-parameters f))
        (assq 'background-color (frame-parameters f))))"#,
        expect_test::expect![[
            r#""OK (t (name . \"F1\") (foreground-color . \"unspecified-fg\") (background-color . \"unspecified-bg\"))""#
        ]],
    );
}

#[test]
fn divergence_face_attribute_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (facep 'default)
  (facep 'bold)
  (facep 'nonexistent-face-xyz)
  (consp (face-all-attributes 'default (selected-frame))))"#,
        expect_test::expect![[
            r#""OK ([face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] [face unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified unspecified] nil t)""#
        ]],
    );
}

#[test]
fn divergence_face_attribute_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (face-attribute 'default :family (selected-frame))
  (face-attribute 'default :height (selected-frame))
  (face-attribute 'default :weight (selected-frame)))"#,
        expect_test::expect![[r#""OK (\"default\" 1 normal)""#]],
    );
}

#[test]
fn divergence_display_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "before")
  (put-text-property (point-min) (point-max) 'display '(image :type xpm))
  (list (get-text-property (point-min) 'display)
        (buffer-string)))"#,
        expect_test::expect![[
            r#""beforeOK ((image :type xpm) #(\"before\" 0 6 (display (image :type xpm))))""#
        ]],
    );
}

#[test]
fn divergence_invisible_text_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "visible")
  (insert (propertize "hidden" 'invisible t))
  (insert "visible2")
  (list (get-text-property 7 'invisible (current-buffer))
        (get-text-property 9 'invisible (current-buffer))
        (get-text-property 15 'invisible (current-buffer))
        (length (buffer-string))))"#,
        expect_test::expect![[r#""visiblehiddenvisible2OK (nil t nil 21)""#]],
    );
}

#[test]
fn divergence_line_beginning_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "line1\nline2\nline3")
  (goto-char 1)
  (list (line-beginning-position)
        (line-end-position)
        (progn (forward-line 1) (line-beginning-position))
        (progn (forward-line 1) (line-beginning-position))
        (line-number-at-pos)))"#,
        expect_test::expect![[r#""line1\nline2\nline3OK (1 6 7 13 3)""#]],
    );
}

#[test]
fn divergence_point_bounds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (insert "Hello World")
  (list (point-min)
        (point-max)
        (point)
        (bobp)
        (eobp)
        (progn (goto-char 5) (point))
        (bolp)
        (eolp)))"#,
        expect_test::expect![[r#""Hello WorldOK (1 12 12 nil t 5 nil nil)""#]],
    );
}
