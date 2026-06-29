//! Bytecode-interpreter divergences (neovm-compiler/executor vs GNU bytecode.c).
//!
//! Tests BEHAVIOR of byte-compiled functions (each engine compiles + runs its
//! own bytecode; can't compare bytecode text). Compiles a lambda, calls it,
//! compares the result value. Targets non-local exits, save-excursion/
//! save-restriction, recursive compiled defun, lexical closures (capture +
//! mutable capture), byte-compile-function-p, and dynamic let.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_abc_byte_compile_function_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (fboundp 'byte-compile-function-p)
      (condition-case e (byte-compile-function-p 'car) (error (car e)))
      (condition-case e (byte-compile-function-p (byte-compile (lambda (x) x))) (error (car e))))
"##,
        expect_test::expect![[r#""OK (nil void-function void-function)""#]],
    );
}

#[test]
fn div_abc_lexical_closure_called() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(funcall (funcall (byte-compile (lambda (x) (let ((y (* x 10))) (lambda () y)))) 5))
"##,
        expect_test::expect![[r#""OK 50""#]],
    );
}

#[test]
fn div_abc_independent_closures() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((f1 (funcall (byte-compile (lambda (x) (lambda () x))) 1))
      (f2 (funcall (byte-compile (lambda (x) (lambda () x))) 2)))
  (list (funcall f1) (funcall f2)))
"##,
        expect_test::expect![[r#""OK (1 2)""#]],
    );
}

#[test]
fn div_abc_mutable_closure_counter() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let ((counter (funcall (byte-compile (lambda () (let ((n 0)) (lambda () (setq n (1+ n)))))))))
  (list (funcall counter) (funcall counter) (funcall counter)))
"##,
        expect_test::expect![[r#""OK (1 2 3)""#]],
    );
}

#[test]
fn div_abc_recursive_compiled_defun() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(progn
  (defun neo-bcfact (n) (if (= n 0) 1 (* n (neo-bcfact (1- n)))))
  (byte-compile 'neo-bcfact)
  (neo-bcfact 5))
"##,
        expect_test::expect![[r#""OK 120""#]],
    );
}

#[test]
fn div_abc_condition_case_compiled_arith() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(funcall (byte-compile (lambda () (condition-case e (/ 1 0) (arith-error :caught)))))
"##,
        expect_test::expect![[r#""OK :caught""#]],
    );
}

#[test]
fn div_abc_unwind_protect_cleanup_on_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let (cleaned)
  (ignore-errors (funcall (byte-compile (lambda () (unwind-protect (error "x") (setq cleaned :ran))))))
  cleaned)
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_abc_unwind_protect_throw_cleanup() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(let (cleaned)
  (list (catch 'tag
          (funcall (byte-compile (lambda () (unwind-protect (throw 'tag :thrown) (setq cleaned :ran))))))
        cleaned))
"##,
        expect_test::expect![[r#""OK (:thrown nil)""#]],
    );
}

#[test]
fn div_abc_save_excursion_restores_point() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (goto-char 1)
  (funcall (byte-compile (lambda () (save-excursion (goto-char 5) (point)))))
  (point))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_abc_save_restriction_restores() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(with-temp-buffer
  (insert "abcdef")
  (narrow-to-region 2 4)
  (funcall (byte-compile (lambda () (save-restriction (widen) (point-max)))))
  (point-max))
"##,
        expect_test::expect![[r#""OK 4""#]],
    );
}

#[test]
fn div_abc_dynamic_let_many_vars() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(funcall (byte-compile (lambda () (let ((a 1) (b 2) (c 3) (d 4) (e 5)) (+ a b c d e)))))
"##,
        expect_test::expect![[r#""OK 15""#]],
    );
}

#[test]
fn div_abc_apply_spread_compiled() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(funcall (byte-compile (lambda (a b c) (apply '+ a b (list c)))) 1 2 3)
"##,
        expect_test::expect![[r#""OK 6""#]],
    );
}

#[test]
fn div_abc_compiled_loop_accumulate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(funcall (byte-compile (lambda (n) (let ((s 0)) (dotimes (i n) (setq s (+ s i))) s))) 10)
"##,
        expect_test::expect![[r#""OK 45""#]],
    );
}

#[test]
fn div_abc_compiled_and_or_short_circuit() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(list (funcall (byte-compile (lambda (x) (and x (/= x 0) (/ 10 x)))) 2)
      (funcall (byte-compile (lambda (x) (and x (/= x 0) (/ 10 x)))) 0)
      (funcall (byte-compile (lambda (x) (or (null x) :fallback))) nil))
"##,
        expect_test::expect![[r#""OK (5 nil t)""#]],
    );
}

#[test]
fn div_abc_compiled_cond_dispatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"
(funcall (byte-compile
          (lambda (x)
            (cond ((< x 0) :neg) ((= x 0) :zero) ((< x 10) :small) (t :big))))
        5)
"##,
        expect_test::expect![[r#""OK :small""#]],
    );
}
