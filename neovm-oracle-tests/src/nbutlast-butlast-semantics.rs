//! Oracle parity tests for GNU `butlast`/`nbutlast` edge semantics.
//!
//! GNU implements these in `lisp/subr.el`: `butlast` uses `take` and returns
//! LIST unchanged for N <= 0, while `nbutlast` computes `length` first and then
//! destructively truncates only when N is smaller than the list length.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_butlast_negative_zero_and_identity() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((lst (list 'a 'b 'c)))
  (list (butlast lst -2)
        (eq (butlast lst -2) lst)
        (butlast lst 0)
        (eq (butlast lst 0) lst)
        (butlast lst nil)
        lst))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_butlast_positive_copy_and_improper_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((lst (list 'a 'b 'c 'd))
       (copy (butlast lst 2)))
  (setcar copy 'changed)
  (list copy
        lst
        (eq copy lst)
        (condition-case err
            (butlast '(a b . c) 1)
          (error (list (car err) (cdr err))))
        (condition-case err
            (butlast '(a b . c) 0)
          (error (list (car err) (cdr err))))
        (condition-case err
            (butlast '(a b . c) -1)
          (error (list (car err) (cdr err)))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_nbutlast_negative_zero_and_large_n() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((a (list 'a 'b 'c))
       (ra (nbutlast a -1))
       (b (list 'a 'b 'c))
       (rb (nbutlast b 0))
       (c (list 'a 'b 'c))
       (rc (nbutlast c 99)))
  (list ra a (eq ra a)
        rb b (eq rb b)
        rc c))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_nbutlast_destructive_identity_and_improper_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((lst (list 1 2 3 4 5))
       (first lst)
       (result (nbutlast lst 2)))
  (list result
        lst
        (eq result first)
        (condition-case err
            (nbutlast '(a b . c) 1)
          (error (list (car err) (cdr err))))
        (condition-case err
            (nbutlast '(a b . c) 99)
          (error (list (car err) (cdr err))))
        (condition-case err
            (nbutlast 42 1)
          (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
