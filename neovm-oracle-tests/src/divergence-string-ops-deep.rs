//! Divergence tests: string manipulation, substring, split, join deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_string_split() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (split-string "foo bar baz" " ")
  (split-string "foo,bar,baz" ",")
  (split-string "foo  bar  baz" " +")
  (split-string "foo" ",")) "#,
    );
}

#[test]
fn divergence_string_split_omit_nulls() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (split-string "foo  bar  baz" " " t)
  (split-string "  foo  bar  " " " t)
  (split-string "" " ")) "#,
    );
}

#[test]
fn divergence_string_join_mapconcat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (mapconcat 'identity '("foo" "bar" "baz") ", ")
  (mapconcat (lambda (x) (upcase x)) '("foo" "bar") "-")
  (string-join '("a" "b" "c") "|")) "#,
    );
}

#[test]
fn divergence_string_replace() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'string-replace)
  (string-replace "foo" "bar" "foo baz foo quux foo")
  (string-replace "a" "X" "aaa")) "#,
    );
}

#[test]
fn divergence_string_truncate() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'truncate-string-to-width)
  (truncate-string-to-width "Hello World" 5)
  (truncate-string-to-width "Hello World" 20)) "#,
    );
}

#[test]
fn divergence_string_pad() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'string-pad)
  (string-pad "hello" 10)
  (string-pad "hello" 10 ?-)
  (string-pad "hello" 3)) "#,
    );
}

#[test]
fn divergence_string_trim() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'string-trim)
  (fboundp 'string-trim-left)
  (fboundp 'string-trim-right)
  (string-trim "  hello  ")
  (string-trim-left "  hello  ")
  (string-trim-right "  hello  ")) "#,
    );
}

#[test]
fn divergence_string_lines() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'string-lines)
  (fboundp 'string-chop-newline)
  (string-chop-newline "hello\n")
  (string-chop-newline "hello")) "#,
    );
}

#[test]
fn divergence_case_changes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (upcase "hello WORLD")
  (downcase "Hello World")
  (capitalize "hello world")
  (upcase-initials "hello world")) "#,
    );
}

#[test]
fn divergence_string_reverse() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'string-reverse)
  (fboundp 'reverse)
  (reverse "hello")
  (reverse '(1 2 3))
  (reverse [1 2 3])) "#,
    );
}
