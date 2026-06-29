/// Batch 535: obarray deep, mapatoms filtered, intern-soft, unintern.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx535_obarray_make() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (make-vector 7 0)))
  (list (vectorp obs) (> (length obs) 0)))
"##,
        expect_test::expect![[r#""OK (t t)""#]],
    );
}

#[test]
fn div_cx535_intern_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (make-vector 7 0)))
  (intern "hello" obs))
"##,
        expect_test::expect![[r#""OK hello""#]],
    );
}

#[test]
fn div_cx535_intern_soft() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(list (intern-soft "nonexistent-cx535-symbol")
      (intern-soft "car"))
"##,
        expect_test::expect![[r#""OK (nil car)""#]],
    );
}

#[test]
fn div_cx535_unintern() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (make-vector 7 0)))
  (intern "test-sym" obs)
  (list (intern-soft "test-sym" obs)
        (unintern "test-sym" obs)
        (intern-soft "test-sym" obs)))
"##,
        expect_test::expect![[r#""OK (test-sym t nil)""#]],
    );
}

#[test]
fn div_cx535_mapatoms_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((count 0))
  (mapatoms (lambda (_) (setq count (1+ count))))
  (listp count))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx535_mapatoms_filter_forward() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((count 0))
  (mapatoms (lambda (s)
    (when (string-prefix-p "forward-" (symbol-name s))
      (setq count (1+ count)))))
  count)
"##,
        expect_test::expect![[r#""OK 27""#]],
    );
}

#[test]
fn div_cx535_obarray_default_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(length (obarray-default))
"##,
        expect_test::expect![[r#""ERR (void-function obarray-default)""#]],
    );
}

#[test]
fn div_cx535_obarray_make_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(length (obarray-make 100))
"##,
        expect_test::expect![[r#""ERR (wrong-type-argument sequencep #<obarray n=0>)""#]],
    );
}

#[test]
fn div_cx535_obarray_get_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (make-vector 7 0)))
  (obarray-put obs "key" 'value)
  (obarray-get obs "key"))
"##,
        expect_test::expect![[r#""ERR (wrong-number-of-arguments (2 . 2) 3)""#]],
    );
}

#[test]
fn div_cx535_obarray_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (make-vector 7 0)))
  (obarray-put obs "k" 'v)
  (obarray-remove obs "k")
  (obarray-get obs "k"))
"##,
        expect_test::expect![[r#""ERR (wrong-number-of-arguments (2 . 2) 3)""#]],
    );
}

#[test]
fn div_cx535_intern_different_obarray() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((o1 (make-vector 7 0))
      (o2 (make-vector 7 0)))
  (let ((s1 (intern "same" o1))
        (s2 (intern "same" o2)))
    (eq s1 s2)))
"##,
        expect_test::expect![[r#""OK nil""#]],
    );
}

#[test]
fn div_cx535_intern_from_fresh() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (make-vector 7 0)))
  (intern (make-string 10 65) obs))
"##,
        expect_test::expect![[r#""OK AAAAAAAAAA""#]],
    );
}

#[test]
fn div_cx535_mapatoms_partial() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((obs (obarray-make 10)))
  (intern "a" obs) (intern "b" obs) (intern "c" obs)
  (let ((count 0))
    (mapatoms (lambda (_) (setq count (1+ count))) obs)
    count))
"##,
        expect_test::expect![[r#""OK 3""#]],
    );
}

#[test]
fn div_cx535_gensym_unique() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((g1 (gensym)) (g2 (gensym))) (not (eq g1 g2)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}

#[test]
fn div_cx535_gensym_prefix() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    crate::common::assert_oracle_parity_expect(
        r##"(let ((g (gensym "CX535-")))
  (string-prefix-p "CX535-" (symbol-name g)))
"##,
        expect_test::expect![[r#""OK t""#]],
    );
}
