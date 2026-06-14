//! Per-defined-color *color-values* matrix (all tty defined colors).
//!
//! One focused #[test] per color in (defined-colors): query color-values.
//! tty color RGB tables may differ between Neomacs and GNU.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_color_val_black() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(color-values black)");
}

#[test]
fn div_color_val_blue() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(color-values blue)");
}

#[test]
fn div_color_val_cyan() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(color-values cyan)");
}

#[test]
fn div_color_val_green() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(color-values green)");
}

#[test]
fn div_color_val_magenta() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(color-values magenta)");
}

#[test]
fn div_color_val_red() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(color-values red)");
}

#[test]
fn div_color_val_white() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(color-values white)");
}

#[test]
fn div_color_val_yellow() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity("(color-values yellow)");
}
