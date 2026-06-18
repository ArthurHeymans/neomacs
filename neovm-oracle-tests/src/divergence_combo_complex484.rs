/// Batch 484: cl-declarations, cl-the, cl-check-type, cl-etypecase, cl-ctypecase.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx484_cl_declare_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn
  (cl-declaim (type integer *cx484-typed*))
  (setq *cx484-typed* 42)
  *cx484-typed*)
"##,
    );
}

#[test]
fn div_cx484_cl_the() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(cl-the integer (+ 1 2))"##);
}

#[test]
fn div_cx484_cl_check_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (cl-check-type "hello integer)
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx484_cl_etypecase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-etypecase 42
  (integer :int)
  (string :str))
"##,
    );
}

#[test]
fn div_cx484_cl_typecase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-typecase "hello"
  (integer :int)
  (string :str)
  (t :other))
"##,
    );
}

#[test]
fn div_cx484_cl_multiple_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-multiple-value-bind (a b c) (values 1 2 3) (+ a b c))
"##,
    );
}

#[test]
fn div_cx484_cl_multiple_value_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-multiple-value-list (values 1 2 3))
"##,
    );
}

#[test]
fn div_cx484_cl_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(multiple-value-list (cl-values 1 2 3))
"##,
    );
}

#[test]
fn div_cx484_cl_progv() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((syms '(a b)) (vals '(1 2)))
  (cl-progv syms vals (+ a b)))
"##,
    );
}

#[test]
fn div_cx484_cl_destructure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-destructuring-bind ((a b) c) '((1 2) 3) (+ a b c))
"##,
    );
}

#[test]
fn div_cx484_cl_remf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((pl '(:a 1 :b 2 :c 3)))
  (cl-remf pl :b)
  pl)
"##,
    );
}

#[test]
fn div_cx484_cl_getf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((pl '(:a 1 :b 2))) (cl-getf pl :a))
"##,
    );
}

#[test]
fn div_cx484_cl_rotatef() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((a 1) (b 2)) (cl-rotatef a b) (list a b))
"##,
    );
}

#[test]
fn div_cx484_cl_shiftf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((a 1) (b 2) (c 3)) (cl-shiftf a b c 0) (list a b c))
"##,
    );
}

#[test]
fn div_cx484_cl_psetf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((a 1) (b 2)) (cl-psetf a 2 b 3) (list a b))
"##,
    );
}
