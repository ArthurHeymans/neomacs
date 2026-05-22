//! Divergence tests: format edge cases, propertize, and string conversion.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_number_to_string_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (number-to-string 0)
  (number-to-string -0)
  (number-to-string most-positive-fixnum)
  (number-to-string most-negative-fixnum)
  (number-to-string 1.5)
  (number-to-string -1.5e10))"#,
    );
}

#[test]
fn divergence_string_to_number_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-to-number "42")
  (string-to-number "0xff")
  (string-to-number "1e5")
  (string-to-number "hello")
  (string-to-number "42abc")
  (string-to-number ""))"#,
    );
}

#[test]
fn divergence_format_percent_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "%d" 0)
  (format "%d" -1)
  (format "%+d" 42)
  (format "% d" 42)
  (format "%x" 255)
  (format "%o" 8)
  (format "%b" 10)
  (format "%s" nil)
  (format "%S" '(a b c)))"#,
    );
}

#[test]
fn divergence_format_float_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "%f" 0.0)
  (format "%f" -0.0)
  (format "%.0f" 3.7)
  (format "%g" 0.0001)
  (format "%g" 100000.0)
  (format "%e" 0.0))"#,
    );
}

#[test]
fn divergence_format_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "%10d" 42)
  (format "%-10d" 42)
  (format "%010d" 42)
  (format "%5s" "hi")
  (format "%-5s" "hi"))"#,
    );
}

#[test]
fn divergence_char_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (char-to-string ?A)
  (char-to-string ?中)
  (string-to-char "Hello")
  (string-to-char ""))"#,
    );
}

#[test]
fn divergence_concat_vs_mapconcat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (concat "a" "b" "c")
  (concat)
  (mapconcat #'identity '("a" "b" "c") "-")
  (mapconcat (lambda (x) (upcase x)) '("a" "b") " "))"#,
    );
}

#[test]
fn divergence_string_equals_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string= "" "")
  (string= "abc" "abc")
  (string= "abc" "ABC")
  (string-equal "abc" "abc")
  (string< "" "a")
  (string> "b" "a")
  (string-version-compare "1.2" "1.10"))"#,
    );
}

#[test]
fn divergence_string_reverse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-reverse "abc")
  (string-reverse "")
  (string-reverse "a"))"#,
    );
}

#[test]
fn divergence_string_fill() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-pad "" 5 ?x)
  (string-chop-newline "hello\n")
  (string-chop-newline "hello")
  (string-trim "  hello  ")
  (string-trim-left "  hello")
  (string-trim-right "hello  "))"#,
    );
}
