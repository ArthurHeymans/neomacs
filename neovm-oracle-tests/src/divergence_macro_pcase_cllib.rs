//! Divergence tests: macro expansion, defmacro, pcase, and cl-lib.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_defmacro_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defmacro my-incf (var)
    (list 'setq var (list '1+ var)))
  (let ((x 10))
    (my-incf x)
    x))"#,
    );
}

#[test]
fn divergence_defmacro_backquote() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defmacro my-when (cond &rest body)
    `(if ,cond (progn ,@body)))
  (my-when t (+ 1 2)))"#,
    );
}

#[test]
fn divergence_macroexpand_1() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defmacro my-add (a b) `(+ ,a ,b))
  (list (macroexpand-1 '(my-add 1 2))
        (macroexpand '(my-add 1 2))))"#,
    );
}

#[test]
fn divergence_pcase_basic_patterns() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (pcase 42
    (0 'zero)
    (42 'found)
    (_ 'other))
  (pcase "hello"
    ("world" 'no)
    ("hello" 'yes))
  (pcase '(1 2 3)
    (`(1 ,b ,c) (list b c))
    (_ 'no)))"#,
    );
}

#[test]
fn divergence_pcase_guard_pattern() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (pcase 42
    ((guard (zerop 42)) 'zero)
    ((guard (> 42 10)) 'big)
    (_ 'small))
  (pcase '(1 2 3)
    ((guard t) 'matched)))"#,
    );
}

#[test]
fn divergence_pcase_pred_pattern() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (pcase 42
    ((pred stringp) 'string)
    ((pred integerp) 'int)
    (_ 'other))
  (pcase "hello"
    ((pred stringp) 'string)
    (_ 'other)))"#,
    );
}

#[test]
fn divergence_pcase_let_pattern() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(pcase-defmacro my-as (var pattern)
  `(app (lambda (x) x) ,(if (eq pattern '_) var `(and ,pattern ,var))))
(list
 (pcase '(1 2 3)
   ((my-as x `(,a ,b ,c)) (list x a b c))))"#,
    );
}

#[test]
fn divergence_cl_lib_loop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (cl-loop for i from 1 to 5 collect (* i i))
  (cl-loop for x in '(a b c d) collect (list x (1+ (cl-position x '(a b c d)))))
  (cl-loop for i from 1 to 10 when (cl-oddp i) sum i))"#,
    );
}

#[test]
fn divergence_cl_struct() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (cl-defstruct (test-point (:constructor test-point-create))
    x y)
  (let ((p (test-point-create :x 10 :y 20)))
    (list (test-point-x p)
          (test-point-y p)
          (test-point-p p))))"#,
    );
}

#[test]
fn divergence_cl_defun_with_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (cl-defun my-test-fn (a &key b c)
    (list a b c))
  (list (my-test-fn 1)
        (my-test-fn 1 :b 2)
        (my-test-fn 1 :c 3 :b 2)))"#,
    );
}

#[test]
fn divergence_gv_setf_generalized() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((v (vector 1 2 3 4 5)))
  (setf (aref v 2) 99)
  (let ((pl (list 'a 1 'b 2)))
    (setf (plist-get pl 'b) 99)
    (list v pl)))"#,
    );
}
