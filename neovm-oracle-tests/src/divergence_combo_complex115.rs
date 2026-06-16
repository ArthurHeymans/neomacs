//! Complex combo batch 115 — `cl-macs` deep macros: cl-letf, cl-letf*,
//! cl-multiple-value-bind, cl-values, cl-coerce, cl-typep with custom
//! types and satisfies predicates.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx115_cl_letf_with_setf_place() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((lst (list 1 2 3))
      (vec (vector 10 20 30)))
  (cl-letf (((car lst) 99)
            ((aref vec 1) 88))
    (list lst vec (car lst) (aref vec 1))))
"##,
    );
}

#[test]
fn div_cx115_cl_letf_star_chained_bindings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((vec (vector 1 2 3)))
  (cl-letf* (((aref vec 0) 10)
             ((aref vec 1) (* (aref vec 0) 2))
             ((aref vec 2) (* (aref vec 1) 2)))
    vec))
"##,
    );
}

#[test]
fn div_cx115_cl_letf_with_symbol_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((orig-fn (symbol-function '+)))
  (cl-letf (((symbol-function '+) (lambda (&rest args) (- (apply orig-fn args)))))
    (list (+ 1 2 3) (+ 10 20) (+ 100 200))))
"##,
    );
}

#[test]
fn div_cx115_cl_letf_restores_after_throw() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((counter (list 0)))
    (catch 'done
      (cl-letf (((car counter) 100))
        (throw 'done :caught)))
    counter)
"##,
    );
}

#[test]
fn div_cx115_cl_multiple_value_bind_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-multiple-value-bind (a b c) (values 1 2 3)
  (list a b c))
"##,
    );
}

#[test]
fn div_cx115_cl_multiple_value_bind_fewer_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-multiple-value-bind (a b c) (values 1 2)
  (list a b c))
"##,
    );
}

#[test]
fn div_cx115_cl_multiple_value_bind_extra_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-multiple-value-bind (a b) (values 1 2 3 4 5)
  (list a b))
"##,
    );
}

#[test]
fn div_cx115_cl_multiple_value_list_round_trip() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-multiple-value-list (values 1 2 3))
      (cl-multiple-value-list (values))
      (cl-multiple-value-list (values :only)))
"##,
    );
}

#[test]
fn div_cx115_cl_typep_with_satisfies() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-typep 42 'integer)
      (cl-typep "x" 'string)
      (cl-typep 42 'string)
      (cl-typep '(1 2) 'cons)
      (cl-typep [1 2] 'vector)
      (cl-typep 5 '(satisfies evenp))
      (cl-typep 5 '(satisfies oddp))
      (cl-typep 5 '(integer 0 10))
      (cl-typep 11 '(integer 0 10))
      (cl-typep 5 '(or string integer))
      (cl-typep "x" '(or string integer))
      (cl-typep 'a '(member a b c))
      (cl-typep 'd '(member a b c)))
"##,
    );
}

#[test]
fn div_cx115_cl_check_type_with_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (cl-check-type 42 integer) (error :no-error))
      (condition-case e (cl-check-type "x" integer) (error (car e)))
      (condition-case e (cl-check-type 5 (integer 0 10)) (error :no-error))
      (condition-case e (cl-check-type 11 (integer 0 10)) (error (car e))))
"##,
    );
}

#[test]
fn div_cx115_cl_assert_with_message() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (condition-case e (cl-assert t) (error :err))
      (condition-case e (cl-assert nil) (error (car e)))
      (condition-case e (cl-assert (> 5 3) nil "5 should be > 3") (error :err))
      (condition-case e (cl-assert (< 5 3) nil "5 should be < 3") (error (car e))))
"##,
    );
}

#[test]
fn div_cx115_cl_ecase_and_etypecase_dispatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (cl-ecase 1 (1 :one) (2 :two) (3 :three))
      (cl-ecase 2 (1 :one) (2 :two) (3 :three))
      (condition-case e (cl-ecase 99 (1 :one) (2 :two)) (error (car e)))
      (cl-typecase 42 (integer :int) (string :str))
      (cl-typecase "x" (integer :int) (string :str))
      (condition-case e (cl-etypecase 'symbol (integer :int) (string :str)) (error (car e))))
"##,
    );
}

#[test]
fn div_cx115_cl_defstruct_with_predicate_and_included() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(cl-defstruct (neo-cx115-base (:conc-name neo-cx115-b-)) a b)
(cl-defstruct (neo-cx115-derived (:include neo-cx115-base)
                                 (:conc-name neo-cx115-d-))
  c)
(let ((b (make-neo-cx115-base :a 1 :b 2))
      (d (make-neo-cx115-derived :a 1 :b 2 :c 3)))
  (list (neo-cx115-base-p b)
        (neo-cx115-base-p d)
        (neo-cx115-derived-p b)
        (neo-cx115-derived-p d)
        (neo-cx115-b-a d)
        (neo-cx115-d-c d)
        (setf (neo-cx115-b-a d) 99)
        (neo-cx115-b-a d)))
"##,
    );
}

#[test]
fn div_cx115_cl_letf_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((counter (list 0)))
  (with-temp-buffer
    (buffer-enable-undo)
    (insert "cl-letf mega test buffer content")
    (put-text-property 1 6 'face 'bold)
    (let ((m (set-marker (make-marker) 8))
          (ov (make-overlay 4 14)))
      (overlay-put ov 'face 'italic)
      (overlay-put ov 'evaporate t)
      (narrow-to-region 2 18)
      (cl-letf (((car counter) 100))
        (let ((state (list (car counter)
                           (buffer-string)
                           (marker-position m)
                           (overlay-start ov) (overlay-end ov)
                           (text-properties-at 1))))
          (undo)
          (widen)
          (list state (car counter)
                (buffer-string) (marker-position m)
                (overlay-start ov) (overlay-end ov)
                (text-properties-at 1)))))))
"##,
    );
}
