//! Divergence tests: print, format, charset, and coding edge cases.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_format_edge_cases() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "%05d" 42)
  (format "%-10s" "hi")
  (format "%10s" "hi")
  (format "%%")
  (format "%.2f" 3.14159)
  (format "%e" 1000.0)
  (format "%d" most-positive-fixnum))"#,
    );
}

#[test]
fn divergence_format_propertize() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(r#"(format-propertize "hello" 'face 'bold)"#);
}

#[test]
fn divergence_prin1_vs_princ() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((s "hello \"world\""))
  (list (prin1-to-string s)
        (princ-to-string s)))"#,
    );
}

#[test]
fn divergence_print_length_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((print-length 3))
  (prin1-to-string '(1 2 3 4 5 6 7 8 9 10)))"#,
    );
}

#[test]
fn divergence_print_level_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((print-level 2))
  (prin1-to-string '(a (b (c (d))) e)))"#,
    );
}

#[test]
fn divergence_print_circle() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((print-circle t)
        (x (list 1 2 3)))
  (setcar (nthcdr 2 x) x)
  (prin1-to-string x))"#,
    );
}

#[test]
fn divergence_print_gensym() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((print-gensym t)
        (gs (gensym "test-")))
  (list (symbol-name gs)
        (prin1-to-string gs)))"#,
    );
}

#[test]
fn divergence_charset_map() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (charsetp 'ascii)
  (charsetp 'unicode)
  (encode-char ?A 'ascii)
  (decode-char 'ascii 65))"#,
    );
}

#[test]
fn divergence_char_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list (char-width ?A)
              (char-width ?中)
              (char-width ?ā)
              (char-width ? ))"#,
    );
}

#[test]
fn divergence_string_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list (string-width "hello")
              (string-width "中文")
              (string-width "ābc"))"#,
    );
}

#[test]
fn divergence_truncate_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (truncate-string-to-width "abcdefghij" 5)
  (truncate-string-to-width "abcdefghij" 5 nil ?…))"#,
    );
}
