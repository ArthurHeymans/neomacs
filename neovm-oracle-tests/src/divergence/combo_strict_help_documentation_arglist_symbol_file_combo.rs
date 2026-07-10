//! Strict combo oracle probes, batch 229: help / documentation extraction.
//! documentation, help-function-arglist, substitute-command-keys, symbol-file,
//! and indirect-function.
//! Uses assert_oracle_parity_expect format.

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_v8_documentation_arglist_user_and_builtin() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(progn
  (defun probe-docd (x y) "Sum two args." (+ x y))
  (list (documentation 'probe-docd)
        (documentation 'car)
        (documentation 'cons)
        (help-function-arglist 'probe-docd)
        (indirect-function 'probe-docd)
        (functionp 'probe-docd)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_substitute_command_keys_and_describe() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(list (substitute-command-keys "Use \\[forward-char] to move forward.")
      (substitute-command-keys "Type \\[keyboard-quit] to cancel.")
      (substitute-command-keys "Plain text no keys.")
      (substitute-command-keys "With \\=\\[literal] preserved.")
      (documentation-property 'car 'function-documentation)
      (documentation-property 'cons 'variable-documentation))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}

#[test]
fn div_v8_symbol_file_and_find_function_helpers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r##"
(progn
  (defun probe-sf () 'x)
  (list (stringp (symbol-file 'car))
        (stringp (symbol-file 'cons))
        (symbol-file 'probe-sf)
        (symbol-file 'probe-not-defined)
        (function-get 'car 'compiler-macro)))
"##;
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(form, expect);
}
