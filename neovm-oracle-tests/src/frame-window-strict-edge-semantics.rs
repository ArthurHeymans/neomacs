//! Oracle parity for frame/window operations.
//! GNU src/frame.c, src/window.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_framep_on_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(framep (selected-frame))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_framep_on_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(framep nil)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_windowp_on_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(windowp (selected-window))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_windowp_on_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(windowp nil)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_selected_frame_is_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(framep (selected-frame))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_selected_window_is_window() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(windowp (selected-window))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_frame_root_frame_returns_frame() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(framep (frame-root-frame))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_frame_id_returns_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(integerp (frame-id (selected-frame)))"#);
    assert_ok_eq("t", &o, &n);
}
