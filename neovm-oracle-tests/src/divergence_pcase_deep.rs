//! Divergence tests: pcase-let, pcase-dolist, and pcase-lambda.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_pcase_let() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'pcase)
(pcase-let ((`(,a ,b ,c) '(1 2 3)))
  (list a b c))"#,
    );
}

#[test]
fn divergence_pcase_let_star() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'pcase)
(pcase-let* ((`(,a ,b) '(1 2))
             (`(,c ,d) (list a b)))
  (list a b c d))"#,
    );
}

#[test]
fn divergence_pcase_dolist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'pcase)
(let ((result nil))
  (pcase-dolist (`(,k ,v) '((a 1) (b 2) (c 3)))
    (push (list k v) result))
  result)"#,
    );
}

#[test]
fn divergence_pcase_lambda() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'pcase)
(let ((fn (pcase-lambda (`(,a ,b)) (+ a b))))
  (list (funcall fn '(1 2))
        (funcall fn '(10 20))))"#,
    );
}

#[test]
fn divergence_pcase_app_pattern() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'pcase)
(pcase '(1 2 3)
  ((app length 3) 'three-elements)
  (_ 'other))"#,
    );
}

#[test]
fn divergence_pcase_pred_pattern() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'pcase)
(list
  (pcase 42 ((pred numberp) 'number) (_ 'other))
  (pcase "hi" ((pred stringp) 'string) (_ 'other))
  (pcase '(1 2) ((pred listp) 'list) (_ 'other)))"#,
    );
}

#[test]
fn divergence_pcase_not_pattern() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'pcase)
(list
  (pcase 5 ((and (pred numberp) (not 0)) 'nonzero) (_ 'other))
  (pcase 0 ((and (pred numberp) (not 0)) 'nonzero) (_ 'other)))"#,
    );
}

#[test]
fn divergence_pcase_or_pattern() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'pcase)
(list
  (pcase 'a ((or 'a 'b 'c) 'abc) (_ 'other))
  (pcase 'd ((or 'a 'b 'c) 'abc) (_ 'other))
  (pcase 'b ((or 'a 'b 'c) 'abc) (_ 'other)))"#,
    );
}

#[test]
fn divergence_pcase_and_pattern() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'pcase)
(pcase '(1 2 3)
  ((and `(,a ,b ,c) (guard (> a 0)))
   (list a b c))
  (_ 'no-match))"#,
    );
}

#[test]
fn divergence_pcase_rx_pattern() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'pcase)
(pcase "hello123"
  ((rx (group (+ (any "a-z"))) (group (+ digit)))
   (list (match-string 0) (match-string 1) (match-string 2)))
  (_ 'no-match))"#,
    );
}
