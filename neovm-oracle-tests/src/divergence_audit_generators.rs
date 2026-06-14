//! Generator (iter) divergences (generator.el CPS vs neovm-core).
//!
//! Confirmed entry point: iter-next end-of-sequence signaling differs — GNU
//! signals the default "Iteration terminated", neomacs propagates the
//! generator body's return value as the end-of-sequence signal. Probes
//! iter-yield/iter-next variants, iter-do, iter-close, iter-defun, yield-from,
//! cleanup-on-close, and repeated-next-past-end.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_agen_basic_yield_next_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (let ((g (funcall (iter-lambda () (iter-yield 1) (iter-yield 2) :done))))
    (list (iter-next g) (iter-next g)
          (condition-case e (iter-next g)
            (iter-end-of-sequence (cons :eos (cdr e)))
            (error (cons :err (car e)))))))
"##,
    );
}

#[test]
fn div_agen_iter_next_explicit_end_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (let ((g (funcall (iter-lambda () (iter-yield 1) (iter-yield 2)))))
    (list (iter-next g) (iter-next g) (iter-next g :eof))))
"##,
    );
}

#[test]
fn div_agen_iter_do_collect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (let (acc)
    (iter-do (x (iter-lambda () (iter-yield 1) (iter-yield 2) (iter-yield 3)))
      (push x acc))
    (nreverse acc)))
"##,
    );
}

#[test]
fn div_agen_iter_close() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (let ((g (funcall (iter-lambda () (iter-yield 1) (iter-yield 2)))))
    (iter-next g)
    (iter-close g)
    :closed))
"##,
    );
}

#[test]
fn div_agen_iter_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (iter-defun neo-igen (n) (dotimes (i n) (iter-yield i)))
  (let ((g (neo-igen 4)))
    (list (iter-next g) (iter-next g) (iter-next g) (iter-next g))))
"##,
    );
}

#[test]
fn div_agen_infinite_generator_external_limit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (let ((g (funcall (iter-lambda () (let ((i 0)) (while t (iter-yield (setq i (1+ i)))))))))
    (list (iter-next g) (iter-next g) (iter-next g))))
"##,
    );
}

#[test]
fn div_agen_cleanup_on_close() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (let (cleaned)
    (let ((g (funcall (iter-lambda () (unwind-protect (iter-yield 1) (setq cleaned :ran))))))
      (iter-next g)
      (iter-close g))
    cleaned))
"##,
    );
}

#[test]
fn div_agen_repeated_next_past_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (let ((g (funcall (iter-lambda () (iter-yield 1)))))
    (iter-next g)
    (list (condition-case e (iter-next g) (iter-end-of-sequence :eos1) (error :other1))
          (condition-case e (iter-next g) (iter-end-of-sequence :eos2) (error :other2)))))
"##,
    );
}

#[test]
fn div_agen_yield_from_delegation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (let ((g (funcall (iter-lambda ()
                      (iter-yield-from (funcall (iter-lambda () (iter-yield 1) (iter-yield 2))))
                      (iter-yield 3)))))
    (list (iter-next g) (iter-next g) (iter-next g))))
"##,
    );
}

#[test]
fn div_agen_generator_final_value_then_end() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (let ((g (funcall (iter-lambda () (iter-yield :a) :final))))
    (list (iter-next g)
          (condition-case e (iter-next g)
            (iter-end-of-sequence (cons :eos (cdr e)))
            (error (cons :err (car e)))))))
"##,
    );
}

#[test]
fn div_agen_iter_next_end_of_sequence_lambda() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(progn (require 'generator)
  (let ((g (funcall (iter-lambda () (iter-yield 1))))
    (iter-next g)
    (iter-next g (lambda () :custom-eos))))
"##,
    );
}
