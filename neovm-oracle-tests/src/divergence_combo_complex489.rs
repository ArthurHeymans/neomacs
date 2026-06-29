/// Batch 489: display-buffer-alist, display-buffer-base-action, window-combine.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx489_display_buffer_alist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((display-buffer-alist '(("\\*cx489\\*" . (display-buffer-same-window)))))
  (let ((buf (get-buffer-create "*cx489*")))
    (display-buffer buf)))
"##,
        expect_test::expect![[r#""OK #<window 1 on *cx489*>""#]],
    );
}

#[test]
fn div_cx489_display_buffer_action() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((display-buffer-base-action '(display-buffer-same-window)))
  (let ((buf (get-buffer-create "*cx489-action*")))
    (display-buffer buf)))
"##,
        expect_test::expect![[r#""OK #<window 1 on *cx489-action*>""#]],
    );
}

#[test]
fn div_cx489_window_combination_resize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (set-window-combination-resize w t)
  (window-combination-resize w))
"##,
        expect_test::expect![[r#""ERR (void-function set-window-combination-resize)""#]],
    );
}

#[test]
fn div_cx489_window_combination_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-combination-limit w))
"##,
        expect_test::expect![[
            r#""ERR (error \"Combination limit is meaningful for internal windows only\")""#
        ]],
    );
}

#[test]
fn div_cx489_window_splits() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-splits w))
"##,
        expect_test::expect![[r#""ERR (void-function window-splits)""#]],
    );
}

#[test]
fn div_cx489_window_use_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (integerp (window-use-time w)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx489_window_new_total() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-new-total w))
"##,
        expect_test::expect![[r#""OK 40""#]],
    );
}

#[test]
fn div_cx489_window_new_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-new-pixel w))
"##,
        expect_test::expect![[r#""OK 40""#]],
    );
}

#[test]
fn div_cx489_window_pixel() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (list (window-pixel-left w) (window-pixel-top w)))
"##,
        expect_test::expect![[r#""OK (0 1)""#]],
    );
}

#[test]
fn div_cx489_window_resize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (window-resize (selected-window) 1)
  (error (car e)))
"##,
        expect_test::expect![[r#""OK error""#]],
    );
}

#[test]
fn div_cx489_window_resize_apply() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (window-resize-apply (selected-window))
  (error (car e)))
"##,
        expect_test::expect![[r#""OK wrong-type-argument""#]],
    );
}

#[test]
fn div_cx489_window_edges_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (list (window-edges w t) (window-pixel-edges w)))
"##,
        expect_test::expect![[r#""OK ((0 1 80 23) (0 1 80 24))""#]],
    );
}

#[test]
fn div_cx489_window_absolute() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-absolute-pixel-edges w))
"##,
        expect_test::expect![[r#""ERR (wrong-type-argument number-or-marker-p nil)""#]],
    );
}

#[test]
fn div_cx489_window_inside() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (window-inside-pixel-edges w))
"##,
        expect_test::expect![[r#""OK (0 1 80 23)""#]],
    );
}

#[test]
fn div_cx489_window_parameters() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (list (window-parameters w) (window-prev-buffers w)))
"##,
        expect_test::expect![[
            r#""OK (((quit-restore-prev other (#<buffer *cx489*> 1 #<marker at 1 in *cx489*> 80) #<window 1 on *cx489-action*> #<buffer *cx489-action*>) (quit-restore other (#<buffer *scratch*> 1 #<marker at 1 in *scratch*> 80) #<window 1 on *cx489-action*> #<buffer *cx489*>)) ((#<buffer *cx489*> #<marker at 1 in *cx489*> #<marker at 1 in *cx489*>) (#<buffer *scratch*> #<marker at 1 in *scratch*> #<marker at 1 in *scratch*>)))""#
        ]],
    );
}
