/// Batch 491: mouse-face, mouse-highlight, mouse-avoidance, mouse-sel, mouse-drag.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx491_mouse_face() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'mouse)
  (list (boundp 'mouse-face) (fboundp 'mouse-set-point)))
"##,
    );
}

#[test]
fn div_cx491_mouse_highlight() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'mouse)
  (boundp 'mouse-highlight))
"##,
    );
}

#[test]
fn div_cx491_mouse_avoidance() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'avoid)
  (list (fboundp 'mouse-avoidance-mode) (boundp 'mouse-avoidance-mode)))
"##,
    );
}

#[test]
fn div_cx491_mouse_drag() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'mouse-drag)
  (list (fboundp 'mouse-drag-throw) (boundp 'mouse-drag-mode)))
"##,
    );
}

#[test]
fn div_cx491_mouse_sensitive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'mouse-sel)
  (list (boundp 'mouse-sel-mode) (fboundp 'mouse-select-region)))
"##,
    );
}

#[test]
fn div_cx491_mouse_visible() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (boundp 'make-pointer-invisible) (boundp 'mouse-wheel-follow-mouse))
"##,
    );
}

#[test]
fn div_cx491_mouse_wheel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'mwheel)
  (list (boundp 'mouse-wheel-mode) (fboundp 'mwheel-install)))
"##,
    );
}

#[test]
fn div_cx491_mouse_autoselect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(boundp 'mouse-autoselect-window)
"##,
    );
}

#[test]
fn div_cx491_mouse_avoidance_mode() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'avoid)
  (fboundp 'mouse-avoidance-nudge-mouse))
"##,
    );
}

#[test]
fn div_cx491_mouse_avoidance_delta() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'avoid)
  (boundp 'mouse-avoidance-nudge-dist))
"##,
    );
}

#[test]
fn div_cx491_mouse_wheel_scroll() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(boundp 'mouse-wheel-scroll-amount)
"##,
    );
}

#[test]
fn div_cx491_mouse_wheel_tilt() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(boundp 'mouse-wheel-tilt-scroll)
"##,
    );
}

#[test]
fn div_cx491_display_mouse() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(boundp 'display-mouse-p)
"##,
    );
}

#[test]
fn div_cx491_mouse_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (framep (car (mouse-pixel-position))) (error (car e)))
"##,
    );
}

#[test]
fn div_cx491_mouse_reset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e (framep (car (mouse-position))) (error (car e)))
"##,
    );
}
