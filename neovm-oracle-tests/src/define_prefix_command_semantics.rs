//! Oracle parity tests for GNU `subr.el' `define-prefix-command'.

use super::common::{
    assert_ok_eq, eval_oracle_and_neovm, return_if_neovm_enable_oracle_proptest_not_set,
};

#[test]
fn oracle_define_prefix_command_installs_function_and_value_keymaps() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((command 'neomacs--oracle-prefix-command)
      (mapvar 'neomacs--oracle-prefix-map))
  (unwind-protect
      (list
       (define-prefix-command command)
       (eq (symbol-function command) (symbol-value command))
       (keymapp (symbol-function command))
       (cdr (symbol-function command))
       (define-prefix-command command mapvar "Menu")
       (eq (symbol-function command) (symbol-value mapvar))
       (keymapp (symbol-value mapvar))
       (cdr (symbol-value mapvar))
       (boundp command))
    (fmakunbound command)
    (makunbound command)
    (makunbound mapvar)))"#;
    let (oracle, neovm) = eval_oracle_and_neovm(form);
    assert_ok_eq(
        r#"(neomacs--oracle-prefix-command t t nil neomacs--oracle-prefix-command t t ("Menu") t)"#,
        &oracle,
        &neovm,
    );
}
