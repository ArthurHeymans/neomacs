//! Divergence tests: rx macro, regex-builder, and complex pattern matching.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_rx_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match (rx "hello") "say hello world")
  (string-match (rx (+ (any "a-z"))) "hello123")
  (string-match (rx bol (+ word) eol) "hello"))"#,
    );
}

#[test]
fn divergence_rx_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (string-match (rx (group (+ digit)) "." (group (+ digit))) "version 12.34")
  (list (match-string 0 "version 12.34")
        (match-string 1 "version 12.34")
        (match-string 2 "version 12.34")))"#,
    );
}

#[test]
fn divergence_rx_char_classes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match (rx (one-or-more digit)) "abc123def")
  (match-string 0 "abc123def")
  (string-match (rx (zero-or-more alpha)) "123abc")
  (match-string 0 "123abc"))"#,
    );
}

#[test]
fn divergence_rx_alternation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match (rx (or "foo" "bar" "baz")) "xyzbar")
  (match-string 0 "xyzbar"))"#,
    );
}

#[test]
fn divergence_rx_repeat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match (rx (= 3 digit)) "12345")
  (match-string 0 "12345")
  (string-match (rx (>= 2 alpha)) "abcde")
  (match-string 0 "abcde"))"#,
    );
}

#[test]
fn divergence_rx_unicode() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (string-match (rx (any "äöü")) "Grüße")
  (match-string 0 "Grüße")
  (string-match (rx nonl (+ nonl)) "Hello World"))"#,
    );
}
