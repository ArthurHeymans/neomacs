//! Oracle parity tests for GNU equality and sxhash edge semantics.
//!
//! GNU implements `eql`, `equal`, `equal-including-properties`, and the sxhash
//! family in `src/fns.c`.  The sxhash tests assert documented equality
//! invariants rather than concrete hash numbers, because GNU does not preserve
//! hash codes across Emacs sessions.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_equal_ignores_string_properties_but_including_properties_checks_them() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((plain "abc")
      (bold (propertize "abc" 'face 'bold))
      (tagged (propertize "abc" 'tag 'source)))
  (list
   (equal plain bold)
   (equal bold tagged)
   (equal-including-properties plain bold)
   (equal-including-properties bold tagged)
   (equal-including-properties
    (propertize "abc" 'face 'bold)
    (propertize "abc" 'face 'bold))
   (equal-including-properties
    (propertize "abc" 'face 'bold 'tag 'source)
    (propertize "abc" 'tag 'source 'face 'bold))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_equal_numeric_and_bool_vector_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (eql 0.0 -0.0)
 (equal 0.0 -0.0)
 (eql (/ 0.0 0.0) (/ 0.0 0.0))
 (equal (/ 0.0 0.0) (/ 0.0 0.0))
 (let ((a (make-bool-vector 3 nil))
       (b (make-bool-vector 3 nil)))
   (aset a 1 t)
   (aset b 1 t)
   (equal a b))
 (let ((a (make-bool-vector 3 nil))
       (b (make-bool-vector 3 nil)))
   (aset a 1 t)
   (aset b 1 t)
   (aset b 2 t)
   (equal a b))
 (let ((a (make-bool-vector 4 nil))
       (b (make-bool-vector 3 nil)))
   (aset a 1 t)
   (aset b 1 t)
   (equal a b)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_equal_handles_circular_lists_and_vectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((a (list 'x))
       (b (list 'x))
       (v1 (vector 'x nil))
       (v2 (vector 'x nil)))
  (setcdr a a)
  (setcdr b b)
  (aset v1 1 v1)
  (aset v2 1 v2)
  (list
   (equal a b)
   (equal a (let ((c (list 'y))) (setcdr c c) c))
   (equal v1 v2)
   (equal v1 (let ((v (vector 'y nil))) (aset v 1 v) v))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_equal_including_properties_recurses_through_cycles() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((left-string (propertize "abc" 'face 'bold))
       (same-string (propertize "abc" 'face 'bold))
       (different-string (propertize "abc" 'face 'italic))
       (left-list (list left-string))
       (same-list (list same-string))
       (different-list (list different-string))
       (left-vector (vector left-string nil))
       (same-vector (vector same-string nil))
       (different-vector (vector different-string nil)))
  (setcdr left-list left-list)
  (setcdr same-list same-list)
  (setcdr different-list different-list)
  (aset left-vector 1 left-vector)
  (aset same-vector 1 same-vector)
  (aset different-vector 1 different-vector)
  (list
   (equal left-list different-list)
   (equal-including-properties left-list same-list)
   (equal-including-properties left-list different-list)
   (equal left-vector different-vector)
   (equal-including-properties left-vector same-vector)
   (equal-including-properties left-vector different-vector)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_sxhash_equal_invariants_for_properties_and_structures() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((a (propertize "abc" 'face 'bold))
      (b (propertize "abc" 'face 'bold))
      (c (propertize "abc" 'face 'italic))
      (x '(1 (2 . 3) [4 5]))
      (y '(1 (2 . 3) [4 5])))
  (list
   (= (sxhash-equal a) (sxhash-equal c))
   (= (sxhash-equal-including-properties a)
      (sxhash-equal-including-properties b))
   (equal-including-properties a b)
   (equal-including-properties a c)
   (= (sxhash-equal x) (sxhash-equal y))
   (= (sxhash-eql 0.0) (sxhash-eql -0.0))
   (= (sxhash-equal 0.0) (sxhash-equal -0.0))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_hash_table_tests_follow_eq_eql_equal_keys() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((eqh (make-hash-table :test 'eq))
      (eqlh (make-hash-table :test 'eql))
      (equalh (make-hash-table :test 'equal))
      (s1 (copy-sequence "key"))
      (s2 (copy-sequence "key")))
  (puthash s1 'eq-string eqh)
  (puthash 1.0 'eql-float eqlh)
  (puthash s1 'equal-string equalh)
  (list
   (gethash s2 eqh 'missing)
   (gethash 1.0 eqlh 'missing)
   (gethash (+ 0.5 0.5) eqlh 'missing)
   (gethash s2 equalh 'missing)
   (hash-table-test eqh)
   (hash-table-test eqlh)
   (hash-table-test equalh)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
