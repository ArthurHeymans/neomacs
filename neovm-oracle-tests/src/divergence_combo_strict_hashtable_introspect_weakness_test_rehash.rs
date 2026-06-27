//! Strict combo oracle probes, batch 89: hash-table introspection accessors
//! (weakness, test, rehash-size, rehash-threshold, size) and hash-table
//! prin1 form comparison.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_q3_hashtable_weakness_and_test() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (hash-table-weakness (make-hash-table :weakness 'key))
      (hash-table-weakness (make-hash-table :weakness 'value))
      (hash-table-weakness (make-hash-table :weakness 'key-and-value))
      (hash-table-weakness (make-hash-table :weakness 'key-or-value))
      (hash-table-weakness (make-hash-table))
      (hash-table-test (make-hash-table :test 'equal))
      (hash-table-test (make-hash-table :test 'eq)))
"##,
    );
}

#[test]
fn div_q3_hashtable_rehash_and_size() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (hash-table-rehash-size (make-hash-table :rehash-size 2.0))
      (hash-table-rehash-size (make-hash-table :rehash-size 10))
      (hash-table-rehash-threshold (make-hash-table :rehash-threshold 0.8))
      (hash-table-size (make-hash-table :size 50))
      (hash-table-size (make-hash-table))
      (>= (hash-table-size (make-hash-table :size 100)) 50))
"##,
    );
}

#[test]
fn div_q3_hashtable_prin1_form() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((h (make-hash-table :test 'equal)))
  (puthash 'a 1 h)
  (puthash 'b 2 h)
  (format "%S" h))
"##,
    );
}
