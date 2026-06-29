/// Batch 526: window-start, window-point, window-display-table, window-redisplay.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx526_window_start_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (set-window-start w 1)
  (window-start w))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_cx526_window_point_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (set-window-point w 3)
  (window-point w))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_cx526_window_point_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(with-temp-buffer
  (insert "hello")
  (let ((w (selected-window)))
    (set-window-point w 3)
    (insert "XY")
    (window-point w)))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_cx526_window_vscroll_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (set-window-vscroll w 10.0)
  (window-vscroll w))
"##,
        expect_test::expect![[r#""OK 0""#]],
    );
}

#[test]
fn div_cx526_window_hscroll() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (set-window-hscroll w 5)
  (window-hscroll w))
"##,
        expect_test::expect![[r#""OK 5""#]],
    );
}

#[test]
fn div_cx526_window_params_multi() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (set-window-parameter w 'a 1)
  (set-window-parameter w 'b 2)
  (list (window-parameter w 'a) (window-parameter w 'b)))
"##,
        expect_test::expect![[r#""OK (1 2)""#]],
    );
}

#[test]
fn div_cx526_window_edges_with_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-edges w t))
"##,
        expect_test::expect![[r#""OK (0 0 80 23)""#]],
    );
}

#[test]
fn div_cx526_window_body_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-body-edges w))
"##,
        expect_test::expect![[r#""OK (0 0 80 23)""#]],
    );
}

#[test]
fn div_cx526_window_inside_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-inside-pixel-edges w))
"##,
        expect_test::expect![[r#""OK (0 0 80 23)""#]],
    );
}

#[test]
fn div_cx526_window_absolute_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-absolute-pixel-edges w))
"##,
        expect_test::expect![[r#""ERR (wrong-type-argument number-or-marker-p nil)""#]],
    );
}

#[test]
fn div_cx526_window_config_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-configuration-p (current-window-configuration)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx526_window_config_compare() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(fboundp 'compare-window-configurations)
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx526_window_list_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (list (window-list) (window-list 1 t)))
"##,
        expect_test::expect![[r#""ERR (error \"Window is on a different frame\")""#]],
    );
}

#[test]
fn div_cx526_window_buffer_switch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window))
      (buf (get-buffer-create " *cx526-wb*")))
  (with-current-buffer buf (insert "buf content"))
  (set-window-buffer w buf)
  (window-buffer w))
"##,
        expect_test::expect![[r#""OK #<buffer  *cx526-wb*>""#]],
    );
}

#[test]
fn div_cx526_window_prev_next_buffers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (listp (window-prev-buffers w)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}
