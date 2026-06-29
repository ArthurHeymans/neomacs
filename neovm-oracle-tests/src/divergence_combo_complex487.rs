/// Batch 487: display-buffer, pop-to-buffer, switch-to-buffer, other-window, split-window.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx487_display_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((buf (get-buffer-create " *cx487-disp*")))
  (with-current-buffer buf (insert "test"))
  (window-live-p (display-buffer buf))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx487_pop_to_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((buf (get-buffer-create " *cx487-pop*")))
  (with-current-buffer buf (insert "pop"))
  (pop-to-buffer buf))
"##,
        expect_test::expect![[r#""OK #<buffer  *cx487-pop*>""#]],
    );
}

#[test]
fn div_cx487_switch_to_buffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((buf (get-buffer-create " *cx487-switch*")))
  (with-current-buffer buf (insert "switch"))
  (switch-to-buffer buf))
"##,
        expect_test::expect![[r#""OK #<buffer  *cx487-switch*>""#]],
    );
}

#[test]
fn div_cx487_other_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((buf (get-buffer-create " *cx487-other*")))
  (with-current-buffer buf (insert "other"))
  (other-window 1)
  (switch-to-buffer buf))
"##,
        expect_test::expect![[r#""OK #<buffer  *cx487-other*>""#]],
    );
}

#[test]
fn div_cx487_split_window_horiz() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (split-window w nil 'right)
  (count-windows))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_cx487_split_window_vert() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (split-window w nil 'below)
  (count-windows))
"##,
        expect_test::expect![[r#""OK 4""#]],
    );
}

#[test]
fn div_cx487_delete_other_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (split-window w nil 'right)
  (delete-other-windows)
  (count-windows))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_cx487_delete_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (split-window w nil 'right)
  (delete-window)
  (count-windows))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_cx487_balance_windows() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (balance-windows)
  (error (car e)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx487_enlarge_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (enlarge-window 1)
  (error (car e)))
"##,
        expect_test::expect![[r#""OK error""#]],
    );
}

#[test]
fn div_cx487_shrink_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (shrink-window 1)
  (error (car e)))
"##,
        expect_test::expect![[r#""OK error""#]],
    );
}

#[test]
fn div_cx487_window_config() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((c (current-window-configuration)))
  (window-configuration-p c))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx487_save_window_excursion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (save-window-excursion
    (split-window w nil 'right))
  (count-windows))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_cx487_with_selected_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (with-selected-window w (point)))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_cx487_window_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((w (selected-window)))
  (list (window-group w) (window-group-1 w)))
"##,
        expect_test::expect![[r#""ERR (void-function window-group)""#]],
    );
}
