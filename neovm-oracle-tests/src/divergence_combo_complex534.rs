/// Batch 534: record, type-of, cl-struct, cl-typep deep probes.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx534_record_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((r (record 'test-type 1 2 3)))
  (list (recordp r) (type-of r) (length r)))
"##,
        expect_test::expect![[r#""OK (t test-type 4)""#]],
    );
}

#[test]
fn div_cx534_record_aref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((r (record 'aref-test 'a 'b 'c)))
  (list (aref r 0) (aref r 1) (aref r 3)))
"##,
        expect_test::expect![[r#""OK (aref-test a c)""#]],
    );
}

#[test]
fn div_cx534_record_aset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((r (record 'aset-test 1 2 3)))
  (aset r 1 99)
  (aref r 1))
"##,
        expect_test::expect![[r#""OK 99""#]],
    );
}

#[test]
fn div_cx534_type_of_numbers() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (type-of 42) (type-of 3.14) (type-of 1/3) (type-of most-positive-fixnum))
"##,
        expect_test::expect![[r#""ERR (void-variable 1/3)""#]],
    );
}

#[test]
fn div_cx534_type_of_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (type-of "hello") (type-of '(1 2)) (type-of [1 2]) (type-of nil))
"##,
        expect_test::expect![[r#""OK (string cons vector symbol)""#]],
    );
}

#[test]
fn div_cx534_cl_typep_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (cl-typep 42 'integer) (cl-typep "a" 'string) (cl-typep 3.14 'float))
"##,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn div_cx534_cl_typep_sequence() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (cl-typep '(1 2) 'list) (cl-typep [1 2] 'vector) (cl-typep "abc" 'sequence))
"##,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn div_cx534_cl_typep_number() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (cl-typep 42 'number) (cl-typep 42 'fixnum) (cl-typep (expt 2 100) 'bignum))
"##,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}

#[test]
fn div_cx534_cl_struct_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn
  (cl-defstruct cx534-point x y)
  (let ((p (make-cx534-point :x 1 :y 2)))
    (list (type-of p) (cx534-point-p p))))
"##,
        expect_test::expect![[r#""OK (cx534-point t)""#]],
    );
}

#[test]
fn div_cx534_cl_struct_constructor() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn
  (cl-defstruct (cx534-pair (:constructor cx534-make))
    (a 0) (b 0))
  (let ((p (cx534-make :a 10 :b 20)))
    (cx534-pair-a p)))
"##,
        expect_test::expect![[r#""OK 10""#]],
    );
}

#[test]
fn div_cx534_record_typeof_deep() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (type-of (current-buffer))
      (type-of (selected-window))
      (type-of (selected-frame)))
"##,
        expect_test::expect![[r#""OK (buffer window frame)""#]],
    );
}

#[test]
fn div_cx534_cl_typep_struct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn
  (cl-defstruct cx534-pt x y)
  (cl-typep (make-cx534-pt) 'cx534-pt))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx534_record_copy() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((r1 (record 'copy 1 2 3)))
  (let ((r2 (copy-sequence r1)))
    (aset r2 1 99)
    (list (aref r1 1) (aref r2 1))))
"##,
        expect_test::expect![[r#""OK (1 99)""#]],
    );
}

#[test]
fn div_cx534_cl_struct_copier() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(progn
  (cl-defstruct cx534-cpt x y)
  (let ((p (make-cx534-cpt :x 5 :y 10))
        (c (copy-cx534-cpt p)))
    (setf (cx534-cpt-x p) 50)
    (cx534-cpt-x c)))
"##,
        expect_test::expect![[r#""ERR (void-variable p)""#]],
    );
}

#[test]
fn div_cx534_record_multiple() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((r1 (record 'type1 1))
      (r2 (record 'type2 2))
      (r3 (record 'type1 3)))
  (list (recordp r1) (recordp r2) (eq (type-of r1) (type-of r3))))
"##,
        expect_test::expect![[r#""OK (t t t)""#]],
    );
}
