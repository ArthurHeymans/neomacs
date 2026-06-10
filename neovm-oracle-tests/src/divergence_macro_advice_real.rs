//! Divergence tests: real macro & advice behavioral differences.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_macroexpand_all() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (defmacro test-when-xxx (cond &rest body)
    \\`(if ,cond (progn ,@body)))
  (list (macroexpand '(test-when-xxx (> 5 3) 'yes 'ok))
        (macroexpand-all '(test-when-xxx (> 5 3) 'yes 'ok))
        (macroexpand '(test-when-xxx nil 'no)))) ",
    );
}

#[test]
fn divergence_nested_macro_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (defmacro test-add1-xxx (x) \\`(+ 1 ,x))
  (defmacro test-add2-xxx (x) \\`(+ (test-add1-xxx ,x) 1))
  (list (macroexpand '(test-add2-xxx 5))
        (macroexpand-all '(test-add2-xxx 5)))) ",
    );
}

#[test]
fn divergence_advice_add_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (defun test-advice-fn-xxx (x) (* x 2))
  (advice-add 'test-advice-fn-xxx :filter-return
               (lambda (r) (+ r 10)))
  (let ((r1 (test-advice-fn-xxx 5)))
    (advice-remove 'test-advice-fn-xxx
                    (lambda (r) (+ r 10)))
    (let ((r2 (test-advice-fn-xxx 5)))
      (list r1 r2
            (advice-member-p (lambda (r) (+ r 10))
                             'test-advice-fn-xxx))))) ",
    );
}

#[test]
fn divergence_advice_before_after() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (defvar test-advice-log-xxx nil)
  (defun test-advice-fn2-xxx (x)
    (push (format \"fn:%d\" x) test-advice-log-xxx)
    (* x 3))
  (advice-add 'test-advice-fn2-xxx :before
               (lambda (x)
                 (push (format \"before:%d\" x) test-advice-log-xxx)))
  (advice-add 'test-advice-fn2-xxx :after
               (lambda (x)
                 (push (format \"after:%d\" x) test-advice-log-xxx)))
  (let ((result (test-advice-fn2-xxx 7)))
    (list result
          (nreverse test-advice-log-xxx)
          (advice-member-p nil 'test-advice-fn2-xxx)))) ",
    );
}

#[test]
fn divergence_defsubst_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (defsubst test-subst-xxx (x) (+ x 100))
  (list (test-subst-xxx 5)
        (test-subst-xxx -3)
        (symbol-function 'test-subst-xxx)
        (subrp (symbol-function 'test-subst-xxx)))) ",
    );
}

#[test]
fn deficiency_closure_capture() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((closures nil))
  (dolist (i '(1 2 3))
    (push (let ((n i)) (lambda () n)) closures))
  (list (mapcar #'funcall (nreverse closures))
        (length closures))) ",
    );
}

#[test]
fn divergence_inline_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (define-inline test-inline-xxx (x)
    (inline-leteval (x)
      (inline-quote (+ ,x 1))))
  (list (test-inline-xxx 5)
        (test-inline-xxx 0)
        (test-inline-xxx -1))) ",
    );
}

#[test]
fn divergence_compiled_function_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (compiled-function-p (lambda (x) x))
  (compiled-function-p #'car)
  (compiled-function-p 'car)
  (compiled-function-p nil)
  (byte-code-function-p #'car)) ",
    );
}

#[test]
fn divergence_gv_generalized() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((lst '(1 2 3 4 5)))
  (cl-incf (nth 2 lst))
  (list lst
        (nth 2 lst)
        (cl-decf (nth 0 lst))
        lst)) ",
    );
}

#[test]
fn divergence_setf_on_plist() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((sym (make-symbol \"test\")))
  (setf (get sym 'x) 10)
  (setf (get sym 'y) 20)
  (list (get sym 'x)
        (get sym 'y)
        (symbol-plist sym)
        (setf (get sym 'x) 99)
        (get sym 'x))) ",
    );
}
