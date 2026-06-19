/// Batch 516: pcase deep pattern matching, cl-loop more complex clauses.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx516_pcase_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(pcase "hello"
  ((and (pred stringp) s) (format "string: %s" s))
  (_ "other"))
"##,
    );
}

#[test]
fn div_cx516_pcase_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(pcase [1 2 3]
  (`[,a ,b ,c] (+ a b c))
  (_ 0))
"##,
    );
}

#[test]
fn div_cx516_pcase_backquote() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(pcase '(a b c)
  (`(,a ,b ,c) (list c b a))
  (_ nil))
"##,
    );
}

#[test]
fn div_cx516_pcase_or() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(pcase 42
  ((or 0 1 2) :small)
  ((or 42 43) :answer)
  (_ :other))
"##,
    );
}

#[test]
fn div_cx516_pcase_app() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(pcase 25
  ((and (pred numberp)
        (app sqrt (and (pred numberp) val)))
   (format "sqrt: %.1f" val))
  (_ "none"))
"##,
    );
}

#[test]
fn div_cx516_pcase_cl_struct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'cl-lib)
  (cl-defstruct cx516-point x y)
  (let ((p (make-cx516-point :x 3 :y 4)))
    (pcase p
      ((cl-struct cx516-point x y) (+ x y))
      (_ nil))))
"##,
    );
}

#[test]
fn div_cx516_cl_loop_finish() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i from 1 to 10
           when (> i 5) return i
           finally return 0)
"##,
    );
}

#[test]
fn div_cx516_cl_loop_counting() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '(1 2 3 4 5)
           counting (oddp i))
"##,
    );
}

#[test]
fn div_cx516_pcase_guard() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((x 5))
  (pcase x
    ((guard (> x 3)) :high)
    (_ :low)))
"##,
    );
}

#[test]
fn div_cx516_pcase_pred() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(pcase '(2 . 4)
  ((and (pred consp)
        (app car a)
        (app cdr b)
        (guard (= a 2)))
   :matched)
  (_ :no))
"##,
    );
}

#[test]
fn div_cx516_cl_loop_by() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i from 0 to 10 by 3 collect i)
"##,
    );
}

#[test]
fn div_cx516_cl_loop_unless() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '(1 2 3 4 5)
           unless (oddp i) collect i)
"##,
    );
}

#[test]
fn div_cx516_cl_loop_thereis() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '(1 3 5 2 4)
           thereis (< i 3))
"##,
    );
}

#[test]
fn div_cx516_cl_loop_named() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop named cx516-loop
           for i from 1 to 10
           do (when (> i 3) (return-from cx516-loop i)))
"##,
    );
}

#[test]
fn div_cx516_cl_loop_multiple_for() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-loop for i in '(a b c)
           for j from 1
           collect (cons j i))
"##,
    );
}
