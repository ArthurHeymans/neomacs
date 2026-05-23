//! Oracle parity tests for `last` and `butlast`.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;

use super::common::{assert_ok_eq, assert_oracle_parity, eval_oracle_and_neovm};

#[test]
fn oracle_prop_last_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // `last` returns the last cons cell
    let (o, n) = eval_oracle_and_neovm("(last '(1 2 3 4 5))");
    assert_ok_eq("(5)", &o, &n);
}

#[test]
fn oracle_prop_last_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm("(last '(42))");
    assert_ok_eq("(42)", &o, &n);
}

#[test]
fn oracle_prop_last_with_n() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm("(last '(1 2 3 4 5) 2)");
    assert_ok_eq("(4 5)", &o, &n);

    let (o, n) = eval_oracle_and_neovm("(last '(1 2 3 4 5) 0)");
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_prop_last_dotted() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // last on a dotted list
    assert_oracle_parity("(last '(1 2 . 3))");
}

#[test]
fn oracle_last_circular_list_uses_safe_length_tail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((cycle (list 'a 'b 'c))
       (_ (setcdr (last cycle) cycle))
       (detected-length (safe-length cycle)))
  (list detected-length
        (eq (last cycle) (nthcdr (1- detected-length) cycle))
        (car (last cycle))
        (car (last cycle 2))
        (eq (last cycle detected-length) cycle)
        (eq (last cycle (1+ detected-length)) cycle)))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_prop_butlast_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm("(butlast '(1 2 3 4 5))");
    assert_ok_eq("(1 2 3 4)", &o, &n);
}

#[test]
fn oracle_prop_butlast_with_n() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm("(butlast '(1 2 3 4 5) 2)");
    assert_ok_eq("(1 2 3)", &o, &n);

    let (o, n) = eval_oracle_and_neovm("(butlast '(1 2 3 4 5) 5)");
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_prop_butlast_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm("(butlast nil)");
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_prop_butlast_single() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let (o, n) = eval_oracle_and_neovm("(butlast '(42))");
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_prop_last_butlast_complement() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // butlast + last should reconstruct the original list (by append)
    let form = "(let ((lst '(1 2 3 4 5)))
                  (equal lst (append (butlast lst) (last lst))))";
    let (o, n) = eval_oracle_and_neovm(form);
    assert_ok_eq("t", &o, &n);
}
