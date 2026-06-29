/// Batch 491: mouse-face, mouse-highlight, mouse-avoidance, mouse-sel, mouse-drag.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx491_mouse_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'mouse)
  (list (boundp 'mouse-face) (fboundp 'mouse-set-point)))
"##,
        expect_test::expect![[r#""OK (nil t)""#]],
    );
}

#[test]
fn div_cx491_mouse_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'mouse)
  (boundp 'mouse-highlight))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx491_mouse_avoidance() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'avoid)
  (list (fboundp 'mouse-avoidance-mode) (boundp 'mouse-avoidance-mode)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx491_mouse_drag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'mouse-drag)
  (list (fboundp 'mouse-drag-throw) (boundp 'mouse-drag-mode)))
"##,
        expect_test::expect![[r#""OK (t nil)""#]],
    );
}

#[test]
fn div_cx491_mouse_sensitive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'mouse-sel)
  (list (boundp 'mouse-sel-mode) (fboundp 'mouse-select-region)))
"##,
        expect_test::expect![[
            r#""ERR (file-missing \"Cannot open load file\" \"No such file or directory\" \"mouse-sel\")""#
        ]],
    );
}

#[test]
fn div_cx491_mouse_visible() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (boundp 'make-pointer-invisible) (boundp 'mouse-wheel-follow-mouse))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx491_mouse_wheel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'mwheel)
  (list (boundp 'mouse-wheel-mode) (fboundp 'mwheel-install)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx491_mouse_autoselect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(boundp 'mouse-autoselect-window)
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx491_mouse_avoidance_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'avoid)
  (fboundp 'mouse-avoidance-nudge-mouse))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx491_mouse_avoidance_delta() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn (require 'avoid)
  (boundp 'mouse-avoidance-nudge-dist))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx491_mouse_wheel_scroll() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(boundp 'mouse-wheel-scroll-amount)
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx491_mouse_wheel_tilt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(boundp 'mouse-wheel-tilt-scroll)
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx491_display_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(boundp 'display-mouse-p)
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx491_mouse_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e (framep (car (mouse-pixel-position))) (error (car e)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx491_mouse_reset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e (framep (car (mouse-position))) (error (car e)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}
