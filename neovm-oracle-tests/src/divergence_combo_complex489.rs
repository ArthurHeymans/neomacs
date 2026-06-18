/// Batch 489: display-buffer-alist, display-buffer-base-action, window-combine.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx489_display_buffer_alist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((display-buffer-alist '(("\\*cx489\\*" . (display-buffer-same-window)))))
  (let ((buf (get-buffer-create "*cx489*")))
    (display-buffer buf)))
"##,
    );
}

#[test]
fn div_cx489_display_buffer_action() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((display-buffer-base-action '(display-buffer-same-window)))
  (let ((buf (get-buffer-create "*cx489-action*")))
    (display-buffer buf)))
"##,
    );
}

#[test]
fn div_cx489_window_combination_resize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-combination-resize w t)
  (window-combination-resize w))
"##,
    );
}

#[test]
fn div_cx489_window_combination_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-combination-limit w))
"##,
    );
}

#[test]
fn div_cx489_window_splits() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-splits w))
"##,
    );
}

#[test]
fn div_cx489_window_use_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (integerp (window-use-time w)))
"##,
    );
}

#[test]
fn div_cx489_window_new_total() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-new-total w))
"##,
    );
}

#[test]
fn div_cx489_window_new_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-new-pixel w))
"##,
    );
}

#[test]
fn div_cx489_window_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (list (window-pixel-left w) (window-pixel-top w)))
"##,
    );
}

#[test]
fn div_cx489_window_resize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (window-resize (selected-window) 1)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx489_window_resize_apply() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (window-resize-apply (selected-window))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx489_window_edges_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (list (window-edges w t) (window-pixel-edges w)))
"##,
    );
}

#[test]
fn div_cx489_window_absolute() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-absolute-pixel-edges w))
"##,
    );
}

#[test]
fn div_cx489_window_inside() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-inside-pixel-edges w))
"##,
    );
}

#[test]
fn div_cx489_window_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (list (window-parameters w) (window-prev-buffers w)))
"##,
    );
}
