//! Divergence tests: accessibility, mouse, drag-n-drop, menu deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_accessibility_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (boundp 'window-min-height)
  (boundp 'window-min-width)
  (boundp 'window-resize-pixelwise)
  (boundp 'frame-resize-pixelwise))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_mouse_event_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'mouse-set-point)
  (fboundp 'mouse-set-region)
  (fboundp 'mouse-yank-at-click)
  (fboundp 'mouse-start-end))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_drag_n_drop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'x-begin-drag)
  (featurep 'dnd))"#,
        expect_test::expect![[r#""OK (t t)""#]],
    );

    crate::common::assert_oracle_parity_expect(
        r#"(progn
  (require 'x-dnd)
  (list
    (fboundp 'x-dnd-handle-drag-n-drop-event)
    (featurep 'x-dnd)))"#,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn divergence_menu_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'menu-bar-open)
  (fboundp 'popup-menu)
  (boundp 'menu-bar-mode)
  (boundp 'tool-bar-mode))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_tool_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'tool-bar-add-item)
  (fboundp 'tool-bar-local-item)
  (featurep 'tool-bar))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_tab_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'tab-bar-mode)
  (fboundp 'tab-new)
  (fboundp 'tab-close)
  (featurep 'tab-bar))"#,
        expect_test::expect![[r#""OK (t t t t)""#]],
    );
}

#[test]
fn divergence_scroll_bar() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (boundp 'scroll-bar-mode)
  (fboundp 'scroll-bar-toolkit-scroll)
  (featurep 'scroll-bar))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_tooltip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'tooltip-mode)
  (boundp 'tooltip-delay)
  (featurep 'tooltip))"#,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn divergence_notification_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'notifications-notify)
  (fboundp 'alerts-add-alert)
  (featurep 'notifications))"#,
        expect_test::expect![[r#""OK (nil nil nil)""#]],
    );
}

#[test]
fn divergence_sound_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    crate::common::assert_oracle_parity_expect(
        r#"(list
  (fboundp 'play-sound)
  (boundp 'ring-bell-function)
  (featurep 'sound))"#,
        expect_test::expect![[r#""OK (t t nil)""#]],
    );
}
