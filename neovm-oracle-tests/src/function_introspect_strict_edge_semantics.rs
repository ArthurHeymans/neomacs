//! Oracle parity tests for function introspection: `defalias`,
//! `indirect-function`, `fset`, `symbol-function`, `fmakunbound`,
//! `macrop`, `subrp`, `primitive-function-p`, `functionp`.
//!
//! GNU src/eval.c, src/data.c: function cell manipulation and type
//! predicates are central to Emacs' Lisp-2 nature.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_err_kind, assert_ok_eq, eval_oracle_and_neovm};

#[test]
fn oracle_defalias_creates_alias() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defalias 'neovm--test-alias 'car)
  (functionp 'neovm--test-alias))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_indirect_function_follows_alias() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (defalias 'neovm--test-indirect 'car)
  (subrp (indirect-function 'neovm--test-indirect)))"#,
    );
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_indirect_function_on_subr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(subrp (indirect-function 'car))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_fset_sets_function_cell() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (fset 'neovm--test-fset (lambda () 42))
  (funcall 'neovm--test-fset))"#,
    );
    assert_ok_eq("42", &o, &n);
}

#[test]
fn oracle_fmakunbound_clears_function_cell() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn
  (fset 'neovm--test-fmu (lambda () 1))
  (fmakunbound 'neovm--test-fmu)
  (fboundp 'neovm--test-fmu))"#,
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_subrp_on_subr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(subrp (symbol-function 'car))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_subrp_on_lambda_is_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(subrp (lambda () 1))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_functionp_on_subr() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(functionp 'car)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_functionp_on_lambda() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(functionp (lambda () 1))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_functionp_on_symbol_without_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (fmakunbound 'neovm--test-nofn) (functionp 'neovm--test-nofn))"#,
    );
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_macrop_on_non_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(macrop 'car)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_commandp_on_non_interactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(commandp 'car)"#);
    assert_ok_eq("nil", &o, &n);
}
