//! Strict combo oracle probes, batch 333: format-message + substitute-command-
//! keys deep. format-message, substitute-command-keys with various key specs,
//! and format-message with %s/%d.
//! Uses assert_oracle_parity_expect format.

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_format_message_substitute_keys_variants() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(list (format-message "Use `%s' to proceed" "M-x")
      (format-message "Type %s now" "C-c")
      (substitute-command-keys "Press \\[forward-char] to move")
      (substitute-command-keys "Use \\[keyboard-quit] to abort")
      (substitute-command-keys "\\[foo] is undefined")
      (substitute-command-keys "Plain text with no keys")
      (format-message "Count: %d items" 42))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_substitute_command_keys_literal_faced() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(list (substitute-command-keys "\\=\\[literal] preserved")
      (substitute-command-keys "Multiple \\[forward-char] and \\[backward-char] keys")
      (length (substitute-command-keys "\\[forward-char]"))
      (> (length (substitute-command-keys "\\[forward-char]")) 0)
      (format-message "Done"))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_format_message_help_echo_documentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(list (format-message "Hello %s" "world")
      (format-message "Value: %d, Float: %.2f" 42 3.14)
      (format-message "Mixed %s and %d" "str" 7)
      (substitute-command-keys "\\`backtick test")
      (stringp (documentation 'car))
      (stringp (documentation-property 'car 'function-documentation)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
