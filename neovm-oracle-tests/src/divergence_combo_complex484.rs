/// Batch 484: cl-declarations, cl-the, cl-check-type, cl-etypecase, cl-ctypecase.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx484_cl_declare_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn
  (cl-declaim (type integer *cx484-typed*))
  (setq *cx484-typed* 42)
  *cx484-typed*)
"##,
        expect_test::expect![[r#""OK 42""#]],
    );
}

#[test]
fn div_cx484_cl_the() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-the integer (+ 1 2))"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_cx484_cl_check_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(condition-case e
    (cl-check-type "hello integer)
  (error (car e)))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx484_cl_etypecase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-etypecase 42
  (integer :int)
  (string :str))
"##,
        expect_test::expect![[r#""OK :int""#]],
    );
}

#[test]
fn div_cx484_cl_typecase() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-typecase "hello"
  (integer :int)
  (string :str)
  (t :other))
"##,
        expect_test::expect![[r#""OK :str""#]],
    );
}

#[test]
fn div_cx484_cl_multiple_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-multiple-value-bind (a b c) (values 1 2 3) (+ a b c))
"##,
        expect_test::expect![[r#""ERR (void-function values)""#]],
    );
}

#[test]
fn div_cx484_cl_multiple_value_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-multiple-value-list (values 1 2 3))
"##,
        expect_test::expect![[r#""ERR (void-function values)""#]],
    );
}

#[test]
fn div_cx484_cl_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(multiple-value-list (cl-values 1 2 3))
"##,
        expect_test::expect![[r#""ERR (void-function multiple-value-list)""#]],
    );
}

#[test]
fn div_cx484_cl_progv() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((syms '(a b)) (vals '(1 2)))
  (cl-progv syms vals (+ a b)))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_cx484_cl_destructure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-destructuring-bind ((a b) c) '((1 2) 3) (+ a b c))
"##,
        expect_test::expect![[r#""OK 6""#]],
    );
}

#[test]
fn div_cx484_cl_remf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((pl '(:a 1 :b 2 :c 3)))
  (cl-remf pl :b)
  pl)
"##,
        expect_test::expect![[r#""OK (:a 1 :c 3)""#]],
    );
}

#[test]
fn div_cx484_cl_getf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((pl '(:a 1 :b 2))) (cl-getf pl :a))
"##,
        expect_test::expect![[r#""OK 1""#]],
    );
}

#[test]
fn div_cx484_cl_rotatef() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((a 1) (b 2)) (cl-rotatef a b) (list a b))
"##,
        expect_test::expect![[r#""OK (2 1)""#]],
    );
}

#[test]
fn div_cx484_cl_shiftf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((a 1) (b 2) (c 3)) (cl-shiftf a b c 0) (list a b c))
"##,
        expect_test::expect![[r#""OK (2 3 0)""#]],
    );
}

#[test]
fn div_cx484_cl_psetf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((a 1) (b 2)) (cl-psetf a 2 b 3) (list a b))
"##,
        expect_test::expect![[r#""OK (2 3)""#]],
    );
}
