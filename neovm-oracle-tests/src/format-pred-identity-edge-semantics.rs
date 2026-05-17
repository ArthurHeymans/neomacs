//! Oracle parity for format, predicates, and identity deep edge cases.
//! GNU src/editfns.c, src/data.c, src/fns.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, eval_oracle_and_neovm, eval_oracle_and_neovm_with_bootstrap};

// --- format ---

#[test]
fn oracle_format_string_and_integer() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(format "%s=%d" "x" 42)"#);
    assert_ok_eq("\"x=42\"", &o, &n);
}

#[test]
fn oracle_format_zero_pad() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(format "%05d" 7)"#);
    assert_ok_eq("\"00007\"", &o, &n);
}

#[test]
fn oracle_format_float() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(format "%.2f" 3.14159)"#);
    assert_ok_eq("\"3.14\"", &o, &n);
}

// --- null ---

#[test]
fn oracle_null_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(null nil)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_null_empty_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(null '())"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_null_non_nil_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(null t)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- atom ---

#[test]
fn oracle_atom_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(atom 'x)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_atom_cons_is_not_atom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(atom '(a))"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_atom_vector_is_atom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(atom [1 2])"#);
    assert_ok_eq("t", &o, &n);
}

// --- listp ---

#[test]
fn oracle_listp_cons() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(listp '(a b))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_listp_nil_is_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(listp nil)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_listp_non_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(listp 42)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- consp ---

#[test]
fn oracle_consp_cons() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(consp '(a . b))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_consp_nil_is_not_cons() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(consp nil)"#);
    assert_ok_eq("nil", &o, &n);
}

// --- nlistp ---

#[test]
fn oracle_nlistp_not_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nlistp 42)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_nlistp_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(nlistp '(a))"#);
    assert_ok_eq("nil", &o, &n);
}

// --- identity ---

#[test]
fn oracle_identity_returns_same() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(eq (identity 'foo) 'foo)"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_identity_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(identity nil)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_identity_ignore_always_strict_runtime_contract() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let form = r#"
(let ((cell (list 'x))
      (vec (vector 'y))
      (str (copy-sequence "abc")))
  (list
   ;; GNU src/fns.c `identity' returns the exact same Lisp object.
   (eq (identity cell) cell)
   (eq (identity vec) vec)
   (eq (identity str) str)
   ;; GNU lisp/subr.el defines `ignore' and `always' as Lisp varargs.
   (ignore)
   (ignore 1 nil 'x cell vec str)
   (always)
   (always nil 1 'x cell vec str)
   (condition-case err
       (identity)
     (error (list (car err) (cdr err))))
   (condition-case err
       (identity 1 2)
     (error (list (car err) (cdr err))))))"#;
    let (o, n) = eval_oracle_and_neovm_with_bootstrap(form);
    assert_ok_eq(
        "(t t t nil nil t t (wrong-number-of-arguments (identity 0)) (wrong-number-of-arguments (identity 2)))",
        &o,
        &n,
    );
}
