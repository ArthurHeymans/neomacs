/// Batch 524: vector operations - fill, map, map-into, reduce across types.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx524_vector_fill() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((v [1 2 3])) (fillarray v 0) v)
"##,
        expect_test::expect![[r#""OK [0 0 0]""#]],
    );
}

#[test]
fn div_cx524_vector_map() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(map 'vector #'1+ [1 2 3])
"##,
        expect_test::expect![[r#""ERR (void-function map)""#]],
    );
}

#[test]
fn div_cx524_vector_map_into() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((v (make-vector 3 0))) (map-into v 'list))
"##,
        expect_test::expect![[r#""ERR (void-function map-into)""#]],
    );
}

#[test]
fn div_cx524_vector_reduce() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-reduce #'+ [1 2 3 4])
"##,
        expect_test::expect![[r#""OK 10""#]],
    );
}

#[test]
fn div_cx524_vector_some() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-some #'numberp [1 a 3])
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx524_vector_every() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-every #'numberp [1 2 3])
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx524_vector_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-position 3 [1 2 3 4])
"##,
        expect_test::expect![[r#""OK 2""#]],
    );
}

#[test]
fn div_cx524_vector_find() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-find 3 [1 2 3 4])
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_cx524_vector_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-count 2 [1 2 2 3])
"##,
        expect_test::expect![[r#""OK 2""#]],
    );
}

#[test]
fn div_cx524_vector_subseq() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(cl-subseq [1 2 3 4 5] 1 3)
"##,
        expect_test::expect![[r#""OK [2 3]""#]],
    );
}

#[test]
fn div_cx524_vector_concatenate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(vconcat [1 2] [3 4])
"##,
        expect_test::expect![[r#""OK [1 2 3 4]""#]],
    );
}

#[test]
fn div_cx524_vector_to_string() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(concat [1 2 3])
"##,
        expect_test::expect![[r#""OK \"\u{1}\u{2}\u{3}\"""#]],
    );
}

#[test]
fn div_cx524_vector_aref_setf() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((v [nil nil nil]))
  (setf (aref v 0) 'a (aref v 1) 'b)
  v)
"##,
        expect_test::expect![[r#""OK [a b nil]""#]],
    );
}

#[test]
fn div_cx524_vector_svref() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (svref [1 2 3] 1) (svref [1 2 3] 0))
"##,
        expect_test::expect![[r#""ERR (void-function svref)""#]],
    );
}

#[test]
fn div_cx524_vector_make_with_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (make-vector 5 0) (make-vector 0 nil) (make-vector 3 'x))
"##,
        expect_test::expect![[r#""OK ([0 0 0 0 0] [] [x x x])""#]],
    );
}
