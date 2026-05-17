//! Oracle parity for plist + obarray operations.
//! GNU src/fns.c, src/lread.c.

use super::common::return_if_neovm_enable_oracle_proptest_not_set;
use super::common::{assert_ok_eq, assert_oracle_parity_with_bootstrap, eval_oracle_and_neovm};

#[test]
fn oracle_plist_get_present() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(plist-get '(a 1 b 2 c 3) 'b)"#);
    assert_ok_eq("2", &o, &n);
}

#[test]
fn oracle_plist_get_missing() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(plist-get '(a 1 b 2) 'x)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_plist_put_new_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(plist-get (plist-put '(a 1) 'b 2) 'b)"#);
    assert_ok_eq("2", &o, &n);
}

#[test]
fn oracle_plist_put_overwrites() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(plist-get (plist-put '(a 1 b 2) 'b 99) 'b)"#);
    assert_ok_eq("99", &o, &n);
}

#[test]
fn oracle_plist_put_member_predicate_and_malformed_tail_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU src/fns.c: Fplist_put/Fplist_member call the custom predicate on
    // property positions and only signal `wrong-type-argument plistp' after
    // reaching a malformed tail.  `plist-put' mutates the original cons cells
    // when it finds or appends a property.
    let form = r#"
(list
 (let ((plist (list "a" 1 "b" 2)))
   (list
    (plist-put plist (copy-sequence "a") 9 #'equal)
    plist))
 (let ((plist (list "a" 1)))
   (list
    (plist-put plist "b" 2 #'equal)
    plist))
 (let ((calls nil))
   (list
    (condition-case err
        (plist-member '(a 1 b . bad) 'x
                      (lambda (actual requested)
                        (push (list actual requested) calls)
                        nil))
      (error (list (car err) (cdr err))))
    (nreverse calls)))
 (let ((calls nil))
   (list
    (condition-case err
        (plist-member '(a 1 b . bad) 'b
                      (lambda (actual requested)
                        (push (list actual requested) calls)
                        (eq actual requested)))
      (error (list (car err) (cdr err))))
    (nreverse calls)))
 (let ((calls nil))
   (list
    (condition-case err
        (plist-put '(a 1 b . bad) 'x 9
                   (lambda (actual requested)
                     (push (list actual requested) calls)
                     nil))
      (error (list (car err) (cdr err))))
    (nreverse calls))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_plist_member_found() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(plist-member '(a 1 b 2 c 3) 'b)"#);
    assert_ok_eq("(b 2 c 3)", &o, &n);
}

#[test]
fn oracle_plist_member_not_found() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(plist-member '(a 1 b 2) 'x)"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_intern_same_name_same_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(eq (intern "xyz-test") (intern "xyz-test"))"#);
    assert_ok_eq("t", &o, &n);
}

#[test]
fn oracle_intern_soft_nonexistent() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(r#"(intern-soft "no-such-sym-99999")"#);
    assert_ok_eq("nil", &o, &n);
}

#[test]
fn oracle_mapatoms_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let (o, n) = eval_oracle_and_neovm(
        r#"(progn (defvar neovm--test-ma-count 0) (mapatoms (lambda (_s) (setq neovm--test-ma-count (1+ neovm--test-ma-count))) obarray) (> neovm--test-ma-count 0))"#,
    );
    assert_ok_eq("t", &o, &n);
}
