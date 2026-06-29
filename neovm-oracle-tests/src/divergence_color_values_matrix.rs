//! Per-defined-color *color-values* matrix (all tty defined colors).
//!
//! One focused #[test] per color in (defined-colors): query color-values.
//! tty color RGB tables may differ between Neomacs and GNU.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_color_val_black() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(color-values black)",
        expect_test::expect![[r#""ERR (void-variable black)""#]],
    );
}

#[test]
fn div_color_val_blue() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(color-values blue)",
        expect_test::expect![[r#""ERR (void-variable blue)""#]],
    );
}

#[test]
fn div_color_val_cyan() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(color-values cyan)",
        expect_test::expect![[r#""ERR (void-variable cyan)""#]],
    );
}

#[test]
fn div_color_val_green() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(color-values green)",
        expect_test::expect![[r#""ERR (void-variable green)""#]],
    );
}

#[test]
fn div_color_val_magenta() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(color-values magenta)",
        expect_test::expect![[r#""ERR (void-variable magenta)""#]],
    );
}

#[test]
fn div_color_val_red() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(color-values red)",
        expect_test::expect![[r#""ERR (void-variable red)""#]],
    );
}

#[test]
fn div_color_val_white() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(color-values white)",
        expect_test::expect![[r#""ERR (void-variable white)""#]],
    );
}

#[test]
fn div_color_val_yellow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        "(color-values yellow)",
        expect_test::expect![[r#""ERR (void-variable yellow)""#]],
    );
}
