//! Divergence tests: funcall, apply, mapcar with various arg types.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_funcall_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (funcall '+ 1 2)
  (funcall 'list 1 2 3)
  (funcall (lambda (x) (* x x)) 5)
  (funcall 'concat "a" "b" "c")) "#,
    );
}

#[test]
fn divergence_apply_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (apply '+ '(1 2 3))
  (apply '+ 1 2 '(3 4))
  (apply 'list nil)
  (apply 'vector '(1 2 3))) "#,
    );
}

#[test]
fn divergence_funcall_composed() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (funcall (apply-partially '+ 1) 2)
  (funcall (apply-partially 'list 'a) 'b 'c)
  (fboundp 'apply-partially)) "#,
    );
}

#[test]
fn divergence_higher_order() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((fns (list #'car #'cdr #'cadr)))
  (list (funcall (nth 0 fns) '(a b c))
        (funcall (nth 1 fns) '(a b c))
        (funcall (nth 2 fns) '(a b c)))) "#,
    );
}

#[test]
fn divergence_closure_capture() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((make-adder
       (lambda (n)
         (lambda (x) (+ x n)))))
  (let ((add5 (funcall make-adder 5))
        (add10 (funcall make-adder 10)))
    (list (funcall add5 3)
          (funcall add10 3)
          (funcall add5 10)))) "#,
    );
}

#[test]
fn divergence_lambda_varargs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (funcall (lambda (&rest args) args) 1 2 3)
  (funcall (lambda (x &rest args) (cons x args)) 1 2 3)
  (funcall (lambda (&optional x y) (list x y)) 1 2)) "#,
    );
}

#[test]
fn divergence_funcall_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (funcall (lambda (x) (list x (* x x) (* x x x))) 3)
  (funcall (lambda (s) (upcase s)) "hello")
  (funcall (lambda (s) (downcase s)) "HELLO")) "#,
    );
}

#[test]
fn divergence_recursive_funcall() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(letrec ((fact (lambda (n)
                       (if (<= n 1) 1 (* n (funcall fact (1- n)))))))
  (list (funcall fact 1)
        (funcall fact 5)
        (funcall fact 10))) "#,
    );
}

#[test]
fn divergence_function_quote() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (functionp (lambda (x) x))
  (functionp 'car)
  (type-of (lambda (x) x))
  (type-of 'car)) "#,
    );
}

#[test]
fn divergence_advice_funcall() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defun test-fn-adv-xxx (x) (* x 2))
  (let ((orig (symbol-function 'test-fn-adv-xxx)))
    (list (funcall orig 5)
          (funcall 'test-fn-adv-xxx 5)
          (eq orig (symbol-function 'test-fn-adv-xxx))))) "#,
    );
}
