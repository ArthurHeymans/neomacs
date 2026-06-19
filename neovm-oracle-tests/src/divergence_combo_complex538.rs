/// Batch 538: compiler macros, function declarations, type declarations.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx538_compiler_macro() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-compiler-macro cx538-cm (&whole form arg)
    (if (numberp arg) (* arg 2) form))
  (cx538-cm 5))
"##,
    );
}

#[test]
fn div_cx538_compiler_macro_expand() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (define-compiler-macro cx538-cm2 (&whole form a b)
    `(+ (* ,a 2) (* ,b 3)))
  (compiler-macroexpand '(cx538-cm2 5 6)))
"##,
    );
}

#[test]
fn div_cx538_declaim_inline() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (declaim (inline cx538-inline-fn))
  (defun cx538-inline-fn (x) (* x 3))
  (cx538-inline-fn 7))
"##,
    );
}

#[test]
fn div_cx538_proclaim_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (cl-proclaim '(type (function (integer) integer) 1+))
  (1+ 5))
"##,
    );
}

#[test]
fn div_cx538_declare_ftype() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (defun cx538-ft (x) (* x 4))
  (cl-declare (ftype (function (number) number) cx538-ft))
  (cx538-ft 5))
"##,
    );
}

#[test]
fn div_cx538_check_declare_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (cl-check-type 42 integer)
  'ok)
"##,
    );
}

#[test]
fn div_cx538_check_declare_type_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (cl-check-type "hello" integer)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx538_assert_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-assert (equal 1 1) t "should be true")
"##,
    );
}

#[test]
fn div_cx538_assert_fail() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (cl-assert nil t "assertion failed")
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx538_multiple_value_bind() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(multiple-value-bind (a b c) (values 1 2 3) (+ a b c))
"##,
    );
}

#[test]
fn div_cx538_multiple_value_call() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(multiple-value-call #'list (values 1 2 3))
"##,
    );
}

#[test]
fn div_cx538_multiple_value_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(multiple-value-list (values 1 2 3))
"##,
    );
}

#[test]
fn div_cx538_values_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(multiple-value-list (values 'a 'b))
"##,
    );
}

#[test]
fn div_cx538_nth_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(nth-value 0 (values 10 20 30))
"##,
    );
}

#[test]
fn div_cx538_nth_value_second() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(nth-value 1 (values 10 20 30))
"##,
    );
}
