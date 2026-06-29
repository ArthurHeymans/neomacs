//! Divergence tests: frame parameters, multi-monitor, display info deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_frame_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'frame-parameters)
  (fboundp 'frame-parameter)
  (fboundp 'set-frame-parameter)
  (fboundp 'modify-frame-parameters))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_frame_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'frame-list)
  (fboundp 'selected-frame)
  (fboundp 'next-frame)
  (fboundp 'delete-frame)
  (fboundp 'make-frame))"#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_frame_title() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'set-frame-name)
  (frame-parameter (selected-frame) 'title)
  (frame-parameter (selected-frame) 'name)
  (stringp (frame-parameter (selected-frame) 'name)))"#,
        expect_test::expect![[r#""OK (t nil \"F1\" t)""#]],
    );
}

#[test]
fn divergence_frame_visibility() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'frame-visible-p)
  (fboundp 'iconify-frame)
  (fboundp 'make-frame-visible)
  (fboundp 'make-frame-invisible))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_multi_monitor() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'display-monitor-attributes-list)
  (fboundp 'frame-monitor-attributes)
  (fboundp 'display-screens))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_display_color_cells() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'display-color-cells)
  (fboundp 'display-color-p)
  (fboundp 'display-grayscale-p)
  (fboundp 'color-values))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_frame_font() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'frame-font)
  (fboundp 'set-frame-font)
  (fboundp 'font-get)
  (fboundp 'font-put)
  (featurep 'font))"#,
        expect_test::expect![[r#""OK (nil t t t nil)""#]],
    );
}

#[test]
fn divergence_frame_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'frame-position)
  (fboundp 'set-frame-position)
  (fboundp 'set-frame-size))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_frame_child_frames() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'frame-parent)
  (fboundp 'frame-ancestor-p)
  (fboundp 'make-frame-on-monitor))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_x_display_info() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'x-display-screens)
  (fboundp 'x-server-version)
  (fboundp 'x-server-vendor)
  (fboundp 'x-display-pixel-width)
  (fboundp 'x-display-pixel-height))"#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}
