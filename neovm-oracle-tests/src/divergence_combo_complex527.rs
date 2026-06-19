/// Batch 527: window-margins, window-fringes, window-scroll-bars deep.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx527_window_margins_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-margins w 3 4)
  (window-margins w))
"##,
    );
}

#[test]
fn div_cx527_window_fringes_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-fringes w 5 10 nil)
  (window-fringes w))
"##,
    );
}

#[test]
fn div_cx527_window_scroll_bars_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-scroll-bars w nil 8 nil t)
  (window-scroll-bars w))
"##,
    );
}

#[test]
fn div_cx527_window_margins_clear() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-margins w nil nil)
  (window-margins w))
"##,
    );
}

#[test]
fn div_cx527_window_fringes_clear() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-fringes w nil nil nil)
  (window-fringes w))
"##,
    );
}

#[test]
fn div_cx527_window_dedicated() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-dedicated-p w t)
  (window-dedicated-p w))
"##,
    );
}

#[test]
fn div_cx527_window_combination() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (list (window-combined-p w nil) (window-combined-p w t)))
"##,
    );
}

#[test]
fn div_cx527_window_next_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-next-buffers w))
"##,
    );
}

#[test]
fn div_cx527_window_new_total() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-new-total w))
"##,
    );
}

#[test]
fn div_cx527_window_new_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-new-pixel w))
"##,
    );
}

#[test]
fn div_cx527_window_new_normal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-new-total w))
"##,
    );
}

#[test]
fn div_cx527_window_total_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (list (window-total-width w) (window-total-height w)))
"##,
    );
}

#[test]
fn div_cx527_window_pixel_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (list (window-pixel-left w) (window-pixel-top w)))
"##,
    );
}

#[test]
fn div_cx527_window_resize_apply() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (window-resize-apply (selected-window))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx527_window_freeze() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-fringes w 10 10)
  (set-window-scroll-bars w 'left 10 t)
  (window-scroll-bars w))
"##,
    );
}
