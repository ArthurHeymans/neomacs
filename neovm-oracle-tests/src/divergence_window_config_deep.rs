//! Divergence tests: window configurations, window slots deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_window_config() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'current-window-configuration)
  (fboundp 'set-window-configuration)
  (fboundp 'window-configuration-p)
  (fboundp 'compare-window-configurations)) "#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_window_config_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'window-configuration-frame)
  (fboundp 'window-configuration-buffer)
  (fboundp 'window-configuration-window)) "#,
        expect_test::expect![[r#""OK (t nil nil)""#]],
    );
}

#[test]
fn divergence_window_split_delete() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'split-window-below)
  (fboundp 'split-window-right)
  (fboundp 'delete-window)
  (fboundp 'delete-other-windows)
  (fboundp 'balance-windows)) "#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_window_buffer_swap() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'set-window-buffer)
  (fboundp 'window-buffer)
  (fboundp 'get-buffer-window)
  (fboundp 'get-buffer-window-list)) "#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_window_tree() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'window-tree)
  (fboundp 'window-at)
  (fboundp 'window-absolute-pixel-edges)
  (fboundp 'window-body-edges)
  (fboundp 'window-top-line)
  (fboundp 'window-left-column)) "#,
        expect_test::expect![[r#""OK (t t t t t t)""#]],
    );
}

#[test]
fn divergence_window_size_fixed() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'window-size-fixed-p)
  (fboundp 'window-resizable)
  (fboundp 'window-size)
  (fboundp 'window-full-height-p)
  (fboundp 'window-full-width-p)) "#,
        expect_test::expect![[r#""OK (t t t t t)""#]],
    );
}

#[test]
fn divergence_window_minibuffer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'minibuffer-window)
  (fboundp 'minibuffer-window-active-p)
  (fboundp 'window-minibuffer-p)
  (fboundp 'set-minibuffer-window)) "#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_window_margins() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'set-window-margins)
  (fboundp 'window-margins)
  (fboundp 'set-window-fringes)
  (fboundp 'window-fringes)
  (boundp 'fringe-mode)
  (member fringe-mode '(nil no-fringe minimal default))) "#,
        expect_test::expect![[r#""OK (t t t t t (nil no-fringe minimal default))""#]],
    );
}

#[test]
fn divergence_window_scroll_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'scroll-up-command)
  (fboundp 'scroll-down-command)
  (fboundp 'scroll-other-window)
  (fboundp 'scroll-other-window-down)
  (boundp 'scroll-conservatively)
  (integerp scroll-conservatively)
  (boundp 'scroll-margin)
  (integerp scroll-margin)) "#,
        expect_test::expect![[r#""OK (t t t t t t t t)""#]],
    );
}

#[test]
fn divergence_window_preserve() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'window-use-time)
  (fboundp 'get-most-recent-window)
  (fboundp 'window-old-buffer)
  (boundp 'window-configuration-change-hook)
  (listp window-configuration-change-hook)) "#,
        expect_test::expect![[r#""OK (t nil t t t)""#]],
    );
}
