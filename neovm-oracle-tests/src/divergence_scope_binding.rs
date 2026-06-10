//! Divergence tests: let-binding, dynamic binding, lexical scope edge.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_let_parallel() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((x 1) (y 2))
  (list x y (+ x y))) "#,
    );
}

#[test]
fn divergence_let_star() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((x 1) (y (1+ x)) (z (+ x y)))
  (list x y z)) "#,
    );
}

#[test]
fn divergence_let_parallel_vs_sequential() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((x 10))
  (let ((x 20) (y x))
    (list x y))) "#,
    );
}

#[test]
fn divergence_let_star_shadow() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((x 10))
  (let* ((x 20) (y x))
    (list x y))) "#,
    );
}

#[test]
fn divergence_dynamic_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(defvar test-dynamic-var-xxx 10)
(let ((test-dynamic-var-xxx 20))
  (list test-dynamic-var-xxx
        (default-value 'test-dynamic-var-xxx))) "#,
    );
}

#[test]
fn divergence_lexical_closure() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((counter 0))
  (let ((inc (lambda () (setq counter (1+ counter)))))
    (list (funcall inc)
          (funcall inc)
          (funcall inc)
          counter))) "#,
    );
}

#[test]
fn divergence_closure_over_let() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((closures nil))
  (dotimes (i 3)
    (push (let ((x i)) (lambda () x)) closures))
  (list (funcall (nth 0 closures))
        (funcall (nth 1 closures))
        (funcall (nth 2 closures)))) "#,
    );
}

#[test]
fn divergence_nested_let_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((a 1))
  (let ((b (1+ a)))
    (let ((c (1+ b)))
      (let ((d (1+ c)))
        (list a b c d))))) "#,
    );
}

#[test]
fn divergence_unwind_protect_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((x 0))
  (ignore-errors
    (setq x 1)
    (error "oops")
    (setq x 2))
  (list x)) "#,
    );
}

#[test]
fn divergence_setq_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'setq-default)
  (fboundp 'default-value)
  (fboundp 'set-default)
  (fboundp 'default-boundp)
  (fboundp 'makunbound)) "#,
    );
}
