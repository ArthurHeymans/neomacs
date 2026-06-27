//! Strict combo oracle probes, batch 31: byte-compiler — compile lambdas and
//! named functions and compare the *results* of calling the byte-compiled
//! functions (the compiled objects themselves differ, so only return values
//! and compiled-function-p are compared). Covers arithmetic, closures,
//! dolist loops, condition-case, and recursion.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_g6_byte_compile_arithmetic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (byte-compile (lambda (x y) (+ (* x 2) y)))))
  (list (funcall f 3 4)
        (funcall f 10 -5)
        (compiled-function-p f)))
"##,
    );
}

#[test]
fn div_g6_byte_compile_closure_counter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((counter 0))
  (let ((f (byte-compile (lambda () (cl-incf counter)))))
    (list (funcall f) (funcall f) (funcall f))))
"##,
    );
}

#[test]
fn div_g6_byte_compile_dolist_loop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (byte-compile
          (lambda (lst)
            (let (acc)
              (dolist (x lst (nreverse acc))
                (push (* x x) acc)))))))
  (list (funcall f '(1 2 3 4)) (funcall f nil)))
"##,
    );
}

#[test]
fn div_g6_byte_compile_condition_case() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((f (byte-compile
          (lambda (x)
            (condition-case err
                (/ 10 x)
              (arith-error 'caught))))))
  (list (funcall f 2) (funcall f 0)))
"##,
    );
}

#[test]
fn div_g6_byte_compile_named_recursion() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn
  (defun probe-bc-fact (n) (if (<= n 1) 1 (* n (probe-bc-fact (1- n)))))
  (byte-compile 'probe-bc-fact)
  (list (probe-bc-fact 6) (probe-bc-fact 0) (probe-bc-fact 10)))
"##,
    );
}
