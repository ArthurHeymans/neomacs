/// Batch 526: window-start, window-point, window-display-table, window-redisplay.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx526_window_start_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-start w 1)
  (window-start w))
"##,
    );
}

#[test]
fn div_cx526_window_point_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-point w 3)
  (window-point w))
"##,
    );
}

#[test]
fn div_cx526_window_point_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(with-temp-buffer
  (insert "hello")
  (let ((w (selected-window)))
    (set-window-point w 3)
    (insert "XY")
    (window-point w)))
"##,
    );
}

#[test]
fn div_cx526_window_vscroll_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-vscroll w 10.0)
  (window-vscroll w))
"##,
    );
}

#[test]
fn div_cx526_window_hscroll() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-hscroll w 5)
  (window-hscroll w))
"##,
    );
}

#[test]
fn div_cx526_window_params_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (set-window-parameter w 'a 1)
  (set-window-parameter w 'b 2)
  (list (window-parameter w 'a) (window-parameter w 'b)))
"##,
    );
}

#[test]
fn div_cx526_window_edges_with_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-edges w t))
"##,
    );
}

#[test]
fn div_cx526_window_body_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-body-edges w))
"##,
    );
}

#[test]
fn div_cx526_window_inside_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-inside-pixel-edges w))
"##,
    );
}

#[test]
fn div_cx526_window_absolute_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-absolute-pixel-edges w))
"##,
    );
}

#[test]
fn div_cx526_window_config_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (window-configuration-p (current-window-configuration)))
"##,
    );
}

#[test]
fn div_cx526_window_config_compare() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(fboundp 'compare-window-configurations)
"##,
    );
}

#[test]
fn div_cx526_window_list_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (list (window-list) (window-list 1 t)))
"##,
    );
}

#[test]
fn div_cx526_window_buffer_switch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window))
      (buf (get-buffer-create " *cx526-wb*")))
  (with-current-buffer buf (insert "buf content"))
  (set-window-buffer w buf)
  (window-buffer w))
"##,
    );
}

#[test]
fn div_cx526_window_prev_next_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((w (selected-window)))
  (listp (window-prev-buffers w)))
"##,
    );
}
