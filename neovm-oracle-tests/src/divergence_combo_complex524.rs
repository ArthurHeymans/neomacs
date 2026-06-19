/// Batch 524: vector operations - fill, map, map-into, reduce across types.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx524_vector_fill() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((v [1 2 3])) (fillarray v 0) v)
"##,
    );
}

#[test]
fn div_cx524_vector_map() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(map 'vector #'1+ [1 2 3])
"##,
    );
}

#[test]
fn div_cx524_vector_map_into() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((v (make-vector 3 0))) (map-into v 'list))
"##,
    );
}

#[test]
fn div_cx524_vector_reduce() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-reduce #'+ [1 2 3 4])
"##,
    );
}

#[test]
fn div_cx524_vector_some() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-some #'numberp [1 a 3])
"##,
    );
}

#[test]
fn div_cx524_vector_every() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-every #'numberp [1 2 3])
"##,
    );
}

#[test]
fn div_cx524_vector_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-position 3 [1 2 3 4])
"##,
    );
}

#[test]
fn div_cx524_vector_find() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-find 3 [1 2 3 4])
"##,
    );
}

#[test]
fn div_cx524_vector_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-count 2 [1 2 2 3])
"##,
    );
}

#[test]
fn div_cx524_vector_subseq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(cl-subseq [1 2 3 4 5] 1 3)
"##,
    );
}

#[test]
fn div_cx524_vector_concatenate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(vconcat [1 2] [3 4])
"##,
    );
}

#[test]
fn div_cx524_vector_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(concat [1 2 3])
"##,
    );
}

#[test]
fn div_cx524_vector_aref_setf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((v [nil nil nil]))
  (setf (aref v 0) 'a (aref v 1) 'b)
  v)
"##,
    );
}

#[test]
fn div_cx524_vector_svref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (svref [1 2 3] 1) (svref [1 2 3] 0))
"##,
    );
}

#[test]
fn div_cx524_vector_make_with_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(list (make-vector 5 0) (make-vector 0 nil) (make-vector 3 'x))
"##,
    );
}
