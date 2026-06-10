//! Divergence tests: subrp, bytecode, compiled-function deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_subrp_builtin() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (subrp (symbol-function 'car))
  (subrp (symbol-function 'cons))
  (subrp (symbol-function 'list))
  (subrp (symbol-function '+))
  (subrp (symbol-function 'lambda))
  (subrp 'not-a-function))"#,
    );
}

#[test]
fn divergence_compiled_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((fn (symbol-function 'car)))
  (list (compiled-function-p fn)
        (subrp fn)
        (or (subrp fn) (compiled-function-p fn))))"#,
    );
}

#[test]
fn divergence_byte_code_function_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (byte-code-function-p (symbol-function 'car))
  (byte-code-function-p (symbol-function 'list))
  (byte-code-function-p (lambda (x) x)))"#,
    );
}

#[test]
fn divergence_function_type_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (type-of (symbol-function 'car))
  (type-of (lambda (x) x))
  (type-of (symbol-function 'list)))"#,
    );
}

#[test]
fn divergence_interactive_lambda() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((fn (lambda (x) (interactive "nNum: ") x)))
  (list (functionp fn)
        (commandp fn)
        (interactive-form fn)))"#,
    );
}

#[test]
fn divergence_closure_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((x 42)
        (fn (lambda () x)))
  (list (type-of fn)
        (functionp fn)
        (closurep fn)))"#,
    );
}

#[test]
fn divergence_function_documentation_builtin() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (stringp (documentation 'car))
  (stringp (documentation 'cons))
  (documentation 'list))"#,
    );
}

#[test]
fn divergence_advice_info() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'advice-info)
  (fboundp 'advice-map-tree)
  (fboundp 'advice--tweak))"#,
    );
}

#[test]
fn divergence_func_arity() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (func-arity 'car)
  (func-arity 'list)
  (func-arity '+)
  (func-arity 'format))"#,
    );
}

#[test]
fn divergence_help_function_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'help-function-arglist)
  (fboundp 'help-split-fundoc)
  (fboundp 'describe-function))"#,
    );
}
