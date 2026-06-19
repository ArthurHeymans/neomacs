/// Batch 525: bool-vector operations - all ops on all vector types.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx525_bool_vector_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (bool-vector t nil t) (bool-vector nil t nil))
"##,
    );
}

#[test]
fn div_cx525_bool_vector_count_pop() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((bv (bool-vector t nil t nil t)))
  (list (bool-vector-count-population bv)
        (bool-vector-count-consecutive bv t 0)
        (bool-vector-count-consecutive bv nil 1)))
"##,
    );
}

#[test]
fn div_cx525_bool_vector_union() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((a (bool-vector t nil t))
      (b (bool-vector nil t nil))
      (c (bool-vector nil nil nil)))
  (bool-vector-union a b c)
  c)
"##,
    );
}

#[test]
fn div_cx525_bool_vector_intersection() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((a (bool-vector t nil t))
      (b (bool-vector nil t nil))
      (c (bool-vector nil nil nil)))
  (bool-vector-intersection a b c)
  c)
"##,
    );
}

#[test]
fn div_cx525_bool_vector_difference() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((a (bool-vector t nil t))
      (b (bool-vector nil t nil))
      (c (bool-vector nil nil nil)))
  (bool-vector-difference a b c)
  c)
"##,
    );
}

#[test]
fn div_cx525_bool_vector_exclusive_or() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((a (bool-vector t nil t))
      (b (bool-vector nil t nil))
      (c (bool-vector nil nil nil)))
  (bool-vector-exclusive-or a b c)
  c)
"##,
    );
}

#[test]
fn div_cx525_bool_vector_subsetp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((a (bool-vector t nil t))
      (b (bool-vector t nil t))
      (c (bool-vector t t t)))
  (list (bool-vector-subsetp a b) (bool-vector-subsetp a c)))
"##,
    );
}

#[test]
fn div_cx525_bool_vector_not() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((a (bool-vector t nil t))
      (c (bool-vector nil nil nil)))
  (bool-vector-not a c)
  c)
"##,
    );
}

#[test]
fn div_cx525_bool_vector_make() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (make-bool-vector 5 t) (make-bool-vector 3 nil) (bool-vector t t nil))
"##,
    );
}

#[test]
fn div_cx525_bool_vector_aref_aset() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((bv (make-bool-vector 5 nil)))
  (aset bv 2 t)
  (list (aref bv 0) (aref bv 2) (aref bv 4)))
"##,
    );
}

#[test]
fn div_cx525_bool_vector_different_lengths() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(condition-case e
    (let ((a (bool-vector t)) (b (bool-vector nil nil)) (c (bool-vector nil)))
      (bool-vector-union a b c))
  (error (car e)))
"##,
    );
}

#[test]
fn div_cx525_bool_vector_empty() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (make-bool-vector 0 t) (bool-vector-count-population (make-bool-vector 0 nil)))
"##,
    );
}

#[test]
fn div_cx525_bool_vector_all_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((bv (make-bool-vector 10 nil)))
  (bool-vector-count-population bv))
"##,
    );
}

#[test]
fn div_cx525_bool_vector_all_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((bv (make-bool-vector 10 t)))
  (bool-vector-count-consecutive bv t 0))
"##,
    );
}

#[test]
fn div_cx525_bool_vector_long() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((bv (make-bool-vector 100 nil)))
  (dotimes (i 100) (when (zerop (mod i 3)) (aset bv i t)))
  (bool-vector-count-population bv))
"##,
    );
}
