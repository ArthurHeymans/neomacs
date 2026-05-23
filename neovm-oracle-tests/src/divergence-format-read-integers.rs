//! Divergence tests: print integers, read integers, format integers.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_format_hex_octal_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list\n  (format \"%x\" 255)\n  (format \"%X\" 255)\n  (format \"%o\" 8)\n  (format \"%#x\" 255)\n  (format \"%#o\" 8)))",
    );
}

#[test]
fn divergence_read_hex_octal_binary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list\n  (string-to-number \"ff\" 16)\n  (string-to-number \"77\" 8)\n  (string-to-number \"1010\" 2)\n  (string-to-number \"0xff\" 16)\n  (string-to-number \"42\" 10))",
    );
}

#[test]
fn divergence_prin1_large_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((big (expt 2 64)))
  (list (prin1-to-string big)
        (string-to-number (prin1-to-string big))
        (= (string-to-number (prin1-to-string big)) big)))"#,
    );
}

#[test]
fn divergence_read_negative_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (read-from-string "-42")
  (read-from-string "(- 42)")
  (car (read-from-string "-42"))) "#,
    );
}

#[test]
fn divergence_format_escaped_percent() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (format "100%%")
  (format "%d%%" 42)
  (format "%.1f%%" 99.9))"#,
    );
}

#[test]
fn divergence_format_zero_pad() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (format "%05d" 42)
  (format "%08d" 0)
  (format "%05x" 255)
  (format "%04o" 8))"#,
    );
}

#[test]
fn divergence_print_symbol_with_special_chars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (prin1-to-string 'foo-bar)
  (prin1-to-string 'foo_bar)
  (prin1-to-string 'foo::bar)
  (symbol-name 'foo-bar))"#,
    );
}

#[test]
fn divergence_read_string_escape_sequences() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (read-from-string "\"hello\\nworld\"")
  (read-from-string "\"tab\\there\"")
  (read-from-string "\"back\\\\slash\""))#" ,
    );
}

#[test]
fn divergence_print_cons_cell() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (prin1-to-string '(a . b))
  (prin1-to-string '(a b c))
  (prin1-to-string '(a b . c)))"#,
    );
}

#[test]
fn divergence_print_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (prin1-to-string [1 2 3])
  (prin1-to-string [])
  (prin1-to-string [a "b" (c d)]))"#,
    );
}
