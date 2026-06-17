//! Complex combo batch 374 — `hash-table`/`obarray`/`sxhash` ultimate:
//! eq/eql/equal test variants, all weakness kinds after GC, maphash/remhash,
//! copy independence, sxhash consistency, clrhash, resize, custom test.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx374_hash_table_eq_eql_equal_with_edge_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((s1 "hello") (s2 (copy-sequence "hello"))
       (n1 1.0) (n2 1.0) (sym1 'k) (sym2 'k))
  (list
   (let ((ht (make-hash-table :test 'eq)))    (puthash s1 :v ht) (gethash s2 ht))
   (let ((ht (make-hash-table :test 'eql)))   (puthash s1 :v ht) (gethash s2 ht))
   (let ((ht (make-hash-table :test 'equal))) (puthash s1 :v ht) (gethash s2 ht))
   (let ((ht (make-hash-table :test 'eq)))    (puthash n1 :v ht) (gethash n2 ht))
   (let ((ht (make-hash-table :test 'eql)))   (puthash n1 :v ht) (gethash n2 ht))
   (let ((ht (make-hash-table :test 'eq)))    (puthash sym1 :v ht) (gethash sym2 ht))))
"##,
    )
}

#[test]
fn div_cx374_hash_table_weakness_all_kinds_after_gc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let (results)
  (dolist (w '(key value key-and-value key-or-value))
    (let ((ht (make-hash-table :weakness w :test 'eq)))
      (dotimes (i 5) (puthash (cons i nil) (cons (* i 10) nil) ht))
      (let ((before (hash-table-count ht)))
        (garbage-collect)
        (push (list w before (hash-table-count ht)) results))))
  (nreverse results))
"##,
    )
}

#[test]
fn div_cx374_hash_table_maphash_remhash_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :test 'equal))
      (collected nil))
  (dotimes (i 10) (puthash i (* i i) ht))
  (remhash 3 ht)
  (remhash 7 ht)
  (maphash (lambda (k v) (push (cons k v) collected)) ht)
  (list (hash-table-count ht)
        (sort collected (lambda (a b) (< (car a) (car b))))))
"##,
    )
}

#[test]
fn div_cx374_hash_table_copy_clrhash_resize() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :test 'equal)))
  (puthash :a 1 ht)
  (puthash :b 2 ht)
  (let ((ht2 (copy-hash-table ht)))
    (puthash :c 3 ht2)
    (clrhash ht)
    (list (hash-table-count ht) (hash-table-count ht2)
          (gethash :a ht :missing) (gethash :a ht2)
          (gethash :c ht2)
          (eq ht ht2))))
"##,
    )
}

#[test]
fn div_cx374_sxhash_equal_consistency_all_types() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let* ((s1 "hello") (s2 (copy-sequence "hello"))
       (v1 [1 2 3]) (v2 (copy-sequence [1 2 3]))
       (l1 '(1 (2 3) (4 5))) (l2 (copy-tree '(1 (2 3) (4 5)))))
  (list (= (sxhash-equal s1) (sxhash-equal s2))
        (= (sxhash-equal v1) (sxhash-equal v2))
        (= (sxhash-equal l1) (sxhash-equal l2))
        (integerp (sxhash-eq 'sym))
        (integerp (sxhash-eql 1.5))
        (= (sxhash-eql 1) (sxhash-eql 1.0))))
"##,
    )
}

#[test]
fn div_cx374_hash_table_resize_threshold_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :test 'equal :size 16 :rehash-size 2.0 :rehash-threshold 0.7)))
  (dotimes (i 30) (puthash i (* i 10) ht))
  (list (hash-table-size ht) (hash-table-count ht)
        (hash-table-rehash-size ht) (hash-table-rehash-threshold ht)
        (hash-table-test ht) (hash-table-weakness ht)))
"##,
    )
}

#[test]
fn div_cx374_obarray_intern_unintern_reuse_slot() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ob (make-obarray 31)))
  (intern "alpha" ob)
  (intern "beta" ob)
  (intern "gamma" ob)
  (let ((before (hash-table-count ob)))
    (unintern "beta" ob)
    (let ((after-unintern (hash-table-count ob)))
      (intern "delta" ob)
      (list before after-unintern (hash-table-count ob)
            (intern-soft "alpha" ob) (intern-soft "beta" ob)
            (intern-soft "delta" ob)))))
"##,
    )
}

#[test]
fn div_cx374_hash_table_custom_test_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (let ((ht (make-hash-table :test (lambda (a b) (eq (car-safe a) (car-safe b)))
                                  :hash (lambda (k) (sxhash (car-safe k))))))
      (puthash '(:a) :v1 ht)
      (puthash '(:b) :v2 ht)
      (puthash '(:a 99) :v3 ht)
      (list (hash-table-count ht)
            (gethash '(:a) ht)
            (gethash '(:a 99) ht)
            (gethash '(:b) ht)))
  (error (list :errored (car e))))
"##,
    )
}

#[test]
fn div_cx374_hash_table_count_after_many_remhash() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :test 'equal)))
  (dotimes (i 20) (puthash i (* i i) ht))
  (remhash 0 ht)
  (remhash 5 ht)
  (remhash 10 ht)
  (remhash 15 ht)
  (remhash 99 ht)
  (list (hash-table-count ht)
        (gethash 0 ht) (gethash 5 ht) (gethash 3 ht)
        (gethash 99 ht :missing)))
"##,
    )
}

#[test]
fn div_cx374_hash_table_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((ht (make-hash-table :test 'equal)))
  (dotimes (i 5) (puthash (cons i :key) (* i i) ht))
  (let ((rec (record 'neo-cx374-mega :a :b :c)))
    (with-temp-buffer
      (buffer-enable-undo)
      (insert "0123456789ABCDEF")
      (put-text-property 1 6 'face 'bold)
      (let ((m (set-marker (make-marker) 8))
            (ov (make-overlay 4 12)))
        (overlay-put ov 'face 'italic)
        (overlay-put ov 'evaporate t)
        (narrow-to-region 3 14)
        (remhash (cons 2 :key) ht)
        (aset rec 2 (hash-table-count ht))
        (let ((state (list (hash-table-count ht)
                           (aref rec 0) (aref rec 1) (aref rec 2) (aref rec 3)
                           (marker-position m)
                           (overlay-start ov) (overlay-end ov)
                           (buffer-string)
                           (text-properties-at 1))))
          (undo)
          (widen()
          (list state (buffer-string) (marker-position m)
                (overlay-start ov) (overlay-end ov)
                (text-properties-at 1)))))))
"##,
    )
}
