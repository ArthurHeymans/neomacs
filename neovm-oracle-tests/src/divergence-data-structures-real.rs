//! Divergence tests: real data structure behavioral differences.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_hash_table_equality() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((h1 (make-hash-table :test 'equal))
        (h2 (make-hash-table :test 'equal)))
  (puthash \"a\" 1 h1)
  (puthash \"b\" 2 h1)
  (puthash \"a\" 1 h2)
  (puthash \"b\" 2 h2)
  (list (equal (hash-table-count h1) (hash-table-count h2))
        (= (gethash \"a\" h1) (gethash \"a\" h2))
        (= (gethash \"b\" h1) (gethash \"b\" h2))
        (null (gethash \"c\" h1))
        (gethash \"c\" h1 42)
        (remhash \"a\" h1)
        (hash-table-count h1)
        (hash-table-count h2))) ",
    );
}

#[test]
fn divergence_alist_operations_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((alist '((a . 1) (b . 2) (c . 3))))
  (setf (alist-get 'b alist) 20)
  (setf (alist-get 'd alist) 40)
  (list (alist-get 'a alist)
        (alist-get 'b alist)
        (alist-get 'c alist)
        (alist-get 'd alist)
        (alist-get 'e alist)
        (alist-get 'e alist 'missing)
        (assoc 'b alist)
        (length alist))) ",
    );
}

#[test]
fn divergence_plist_operations_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((pl nil))
  (setq pl (plist-put pl 'x 10))
  (setq pl (plist-put pl 'y 20))
  (setq pl (plist-put pl 'z 30))
  (list (plist-get pl 'x)
        (plist-get pl 'y)
        (plist-get pl 'z)
        (plist-get pl 'w)
        (plist-member pl 'y)
        (length pl)
        (plist-get (plist-put pl 'x 99) 'x))) ",
    );
}

#[test]
fn divergence_vector_operations_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((v [10 20 30 40 50]))
  (aset v 2 99)
  (list (aref v 0) (aref v 1) (aref v 2) (aref v 3) (aref v 4)
        (length v)
        (vconcat v [60 70])
        (append v nil)
        (substring v 1 3))) ",
    );
}

#[test]
fn divergence_bool_vector_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((bv (make-bool-vector 8 nil)))
  (aset bv 0 t)
  (aset bv 3 t)
  (aset bv 7 t)
  (list (aref bv 0) (aref bv 1) (aref bv 3) (aref bv 7)
        (bool-vector-count-matches bv t)
        (bool-vector-count-matches bv nil)
        (bool-vector-not bv))) ",
    );
}

#[test]
fn divergence_char_table_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((ct (make-char-table 'syntax-table nil)))
  (set-char-table-range ct ?A 'letter)
  (set-char-table-range ct ?a 'letter)
  (set-char-table-range ct ?0 'digit)
  (list (aref ct ?A)
        (aref ct ?a)
        (aref ct ?0)
        (aref ct ?+)
        (char-table-range ct ?A)
        (char-table-p ct)
        (char-table-subtype ct))) ",
    );
}

#[test]
fn divergence_record_vs_vector_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((r (record 'point 10 20))
        (v [point 10 20]))
  (list (recordp r)
        (recordp v)
        (vectorp r)
        (vectorp v)
        (aref r 0) (aref r 1) (aref r 2)
        (aref v 0) (aref v 1) (aref v 2)
        (length r)
        (length v)
        (equal (cdr r) (cdr v)))) ",
    );
}

#[test]
fn divergence_sequence_ops_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (seq-map #'1+ '(1 2 3))
  (seq-filter #'cl-evenp '(1 2 3 4 5))
  (seq-reduce #'+ '(1 2 3 4) 0)
  (seq-find #'cl-oddp '(2 4 5 6))
  (seq-contains '(1 2 3) 2)
  (seq-contains '(1 2 3) 4)
  (seq-length '(a b c))
  (seq-into '(1 2 3) 'vector)) ",
    );
}

#[test]
fn divergence_weak_hash_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((ht (make-hash-table :test 'equal :weakness 'key)))
  (puthash \"foo\" 42 ht)
  (list (hash-table-weakness ht)
        (hash-table-test ht)
        (gethash \"foo\" ht)
        (hash-table-count ht)
        (eq (hash-table-weakness ht) 'key))) ",
    );
}

#[test]
fn divergence_sort_stable_real() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((data '((3 . \"c\") (1 . \"a\") (2 . \"b\") (1 . \"a2\"))))
  (let ((sorted (copy-sequence data)))
    (setq sorted (sort sorted (lambda (a b) (< (car a) (car b)))))
    (list (mapcar #'car sorted)
          (mapcar #'cdr sorted)))) ",
    );
}
