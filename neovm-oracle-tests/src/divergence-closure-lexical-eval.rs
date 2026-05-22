//! Divergence tests: closure, lexical binding, and eval edge cases.
//!
//! Tests for closure capture semantics, lexical/dynamic interaction,
//! eval with lexical-binding flag, and function introspection.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_closure_captures_outer_lexical() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((x 10))
  (funcall
   (let ((x 20))
     (lambda () x))))"#,
    );
}

#[test]
fn divergence_closure_mutates_captured_var() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((counter 0))
  (let ((inc (lambda () (setq counter (1+ counter)))))
    (funcall inc)
    (funcall inc)
    (funcall inc)
    counter))"#,
    );
}

#[test]
fn divergence_let_parallel_vs_let_star() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (let ((a 10)) (let ((a 1) (a (1+ a))) a))
  (let ((a 10)) (let* ((a 1) (a (1+ a))) a)))"#,
    );
}

#[test]
fn divergence_lexical_shadow_dynamic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defvar dyn-var 100)
  (let ((dyn-var 200))
    (list dyn-var
          (let ((dyn-var 300)) dyn-var)
          dyn-var
          (eval 'dyn-var))))"#,
    );
}

#[test]
fn divergence_closure_over_loop_variable() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((fns nil))
  (dotimes (i 5)
    (push (lambda () i) fns))
  (mapcar #'funcall (nreverse fns)))"#,
    );
}

#[test]
fn divergence_funcall_with_and_rest() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(r#"(funcall (lambda (a &rest b) (list a b)) 1 2 3 4)"#);
}

#[test]
fn divergence_funcall_with_and_optional() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (funcall (lambda (a &optional b c) (list a b c)) 1)
  (funcall (lambda (a &optional b c) (list a b c)) 1 2)
  (funcall (lambda (a &optional b c) (list a b c)) 1 2 3))"#,
    );
}

#[test]
fn divergence_function_type_introspection() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (subrp (symbol-function 'car))
  (byte-code-function-p (symbol-function 'car))
  (functionp 'car)
  (functionp (lambda (x) x))
  (commandp 'car))"#,
    );
}

#[test]
fn divergence_apply_spreads_last_arg() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (apply #'+ 1 2 '(3 4 5))
  (apply #'+ nil)
  (apply #'list 1 2 '(3 4)))"#,
    );
}

#[test]
fn divergence_mapcar_mapc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (mapcar #'1+ '(1 2 3 4 5))
  (let ((acc nil))
    (mapc (lambda (x) (push x acc)) '(a b c))
    acc))"#,
    );
}

#[test]
fn divergence_recursive_lambda() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(letrec ((fact (lambda (n)
                  (if (<= n 1) 1 (* n (funcall fact (1- n)))))))
  (funcall fact 10))"#,
    );
}

#[test]
fn divergence_dyn_wind_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((log nil))
  (condition-case nil
      (unwind-protect
          (unwind-protect
              (progn
                (push 'body log)
                (signal 'error "test"))
            (push 'inner-cleanup log))
        (push 'outer-cleanup log))
    (error nil))
  log)"#,
    );
}

#[test]
fn divergence_catch_throw_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (catch 'outer
    (catch 'inner
      (throw 'outer 'from-inner))
    'not-reached)
  (catch 'tag
    (throw 'tag 42)))"#,
    );
}

#[test]
fn divergence_eval_with_lexical_flag() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (eval '(let ((x 5)) (funcall (lambda () x))) t)
  (eval '(let ((x 5)) (funcall (lambda () x))) nil))"#,
    );
}

#[test]
fn divergence_closure_with_docstring() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((fn (lambda (x) "docstring" (1+ x))))
  (list (funcall fn 41) (documentation fn)))"#,
    );
}
