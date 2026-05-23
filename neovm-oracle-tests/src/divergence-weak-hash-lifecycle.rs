//! Divergence tests: weak hash tables, finalizers, and object lifecycle.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_weak_hash_table_key() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((ht (make-hash-table :weakness 'key)))
  (list (hash-table-weakness ht)
        (eq (hash-table-weakness ht) 'key)))"#,
    );
}

#[test]
fn divergence_weak_hash_table_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((ht (make-hash-table :weakness 'value)))
  (list (hash-table-weakness ht)
        (eq (hash-table-weakness ht) 'value)))"#,
    );
}

#[test]
fn divergence_weak_hash_table_kv() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((ht (make-hash-table :weakness 'key-or-value)))
  (list (hash-table-weakness ht)
        (memq (hash-table-weakness ht) '(key value key-or-value key-and-value))))"#,
    );
}

#[test]
fn divergence_hash_table_equality() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((ht1 (make-hash-table :test 'eq))
        (ht2 (make-hash-table :test 'equal)))
  (list (hash-table-test ht1)
        (hash-table-test ht2)
        (eq (hash-table-test ht1) 'eq)
        (eq (hash-table-test ht2) 'equal)))"#,
    );
}

#[test]
fn divergence_finalizer_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (fboundp 'make-finalizer)
  (fboundp 'finalizerp)
  (fboundp 'set-finalizer))"#,
    );
}

#[test]
fn divergence_cons_indirect() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((x (list 1 2 3)))
  (list (consp x)
        (car-safe x)
        (cdr-safe x)
        (car (cons 1 2))
        (cdr (cons 1 2))))"#,
    );
}

#[test]
fn divergence_list_accessors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((lst '(a b c d e)))
  (list (nth 0 lst)
        (nth 2 lst)
        (nth 4 lst)
        (nthcdr 2 lst)
        (last lst)
        (butlast lst 2)
        (safe-length lst)))"#,
    );
}

#[test]
fn divergence_number_operations() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (max 1 2 3 4 5)
  (min 1 2 3 4 5)
  (+ 1 2 3 4 5)
  (* 1 2 3 4 5)
  (- 100 1 2 3)
  (/ 100 3)
  (% 100 3)
  (expt 2 10))"#,
    );
}

#[test]
fn divergence_string_make_multibyte() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((us (string ?a ?b ?c))
         (ms (string-to-multibyte us)))
  (list (multibyte-string-p us)
        (multibyte-string-p ms)
        (string= us ms)
        (string-bytes ms)))"#,
    );
}

#[test]
fn divergence_string_make_unibyte_edge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let* ((ms "abc")
         (us (string-make-unibyte ms)))
  (list (multibyte-string-p ms)
        (multibyte-string-p us)
        (string-bytes us)
        (string= ms us)))"#,
    );
}
