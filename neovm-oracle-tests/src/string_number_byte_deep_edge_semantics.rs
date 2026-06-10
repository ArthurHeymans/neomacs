//! Oracle parity for string-to-number, byte ops, format deep edges.
//! GNU src/editfns.c, src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_string_to_number_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-number "1111" 2)"#);
    assert_ok_eq("15", &o, &n);
}

#[test]
fn oracle_string_to_number_octal() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-number "77" 8)"#);
    assert_ok_eq("63", &o, &n);
}

#[test]
fn oracle_string_to_number_hex() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-number "ff" 16)"#);
    assert_ok_eq("255", &o, &n);
}

#[test]
fn oracle_string_to_number_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-number "")"#);
    assert_ok_eq("0", &o, &n);
}

#[test]
fn oracle_byte_to_string_A() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(byte-to-string 65)"#);
    assert_ok_eq("\"A\"", &o, &n);
}

#[test]
fn oracle_string_to_char_returns_first() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(string-to-char "XYZ")"#);
    assert_ok_eq("88", &o, &n);
}

#[test]
fn oracle_format_percent_escape() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(format "%%")"#);
    assert_ok_eq("\"%\"", &o, &n);
}

#[test]
fn oracle_format_char_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(format "%c" 65)"#);
    assert_ok_eq("\"A\"", &o, &n);
}

#[test]
fn oracle_format_multiple_specifiers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(format "%d %s" 42 "items")"#);
    assert_ok_eq("\"42 items\"", &o, &n);
}

#[test]
fn oracle_format_S_vs_s() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // %S prints with prin1 (readable), %s prints without quotes
    let (o, n) = eval_oracle_and_neovm(r#"(list (format "%S" 'foo) (format "%s" "bar"))"#);
    assert_ok_eq("(\"foo\" \"bar\")", &o, &n);
}
