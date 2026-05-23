//! Divergence tests: format specifiers, width, precision, padding.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_format_integers() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "%d" 42)
  (format "%d" -42)
  (format "%d" 0)
  (format "%+d" 42)
  (format "% d" 42)) "#,
    );
}

#[test]
fn divergence_format_hex_octal() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "%x" 255)
  (format "%X" 255)
  (format "%#x" 255)
  (format "%o" 8)
  (format "%#o" 8)) "#,
    );
}

#[test]
fn divergence_format_width() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "%5d" 42)
  (format "%-5d" 42)
  (format "%05d" 42)
  (format "%10s" "hi")
  (format "%-10s|" "hi")) "#,
    );
}

#[test]
fn divergence_format_floats() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "%f" 3.14)
  (format "%.2f" 3.14159)
  (format "%e" 3.14)
  (format "%g" 3.14)
  (format "%.0f" 3.14)) "#,
    );
}

#[test]
fn divergence_format_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "Hello %s" "World")
  (format "%S" "hello")
  (format "%%")
  (format "%c" 65)
  (format "<<%s>>" nil)) "#,
    );
}

#[test]
fn divergence_format_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "%s is %d years old" "Alice" 30)
  (format "%d + %d = %d" 1 2 3)
  (format "list: %S" '(1 2 3))) "#,
    );
}

#[test]
fn divergence_format_time() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'format-time-string)
  (stringp (format-time-string "%Y-%m-%d"))
  (stringp (format-time-string "%H:%M:%S"))
  (stringp (format-time-string "%A, %B %d, %Y"))) "#,
    );
}

#[test]
fn divergence_format_seconds() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'format-seconds)
  (fboundp 'seconds-to-string)
  (stringp (seconds-to-string 90))
  (stringp (seconds-to-string 3661))) "#,
    );
}

#[test]
fn divergence_format_spec_padding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (format "%10.3f" 3.14)
  (format "%-10.3f" 3.14)
  (format "%+.3f" 3.14)
  (format "% .3f" 3.14)) "#,
    );
}

#[test]
fn divergence_format_propertized() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'format-propertized)
  (fboundp 'format-message)
  (stringp (format-message "`foo' and `bar'"))
  (stringp (format-message "this is `foo'"))) "#,
    );
}
