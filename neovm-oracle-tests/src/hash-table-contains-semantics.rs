//! Oracle parity tests for GNU `subr.el` `hash-table-contains-p`.
//!
//! GNU implements this helper with a private sentinel around `gethash`, so it
//! observes key presence independently of whether the stored value is nil.

use super::common::{
    assert_oracle_parity_with_bootstrap, return_if_neovm_enable_oracle_proptest_not_set,
};

#[test]
fn oracle_prop_gnu_hash_table_contains_p_distinguishes_nil_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((eq-table (make-hash-table :test 'eq))
      (equal-table (make-hash-table :test 'equal))
      (case-fold-search nil))
  (puthash 'present nil eq-table)
  (puthash 'false-value nil eq-table)
  (puthash 'true-value t eq-table)
  (puthash "alpha" nil equal-table)
  (puthash "beta" 0 equal-table)
  (puthash '(nested key) 'payload equal-table)
  (list
   (hash-table-contains-p 'present eq-table)
   (gethash 'present eq-table)
   (hash-table-contains-p 'missing eq-table)
   (gethash 'missing eq-table)
   (hash-table-contains-p 'false-value eq-table)
   (hash-table-contains-p 'true-value eq-table)
   (hash-table-contains-p (copy-sequence "alpha") equal-table)
   (gethash (copy-sequence "alpha") equal-table)
   (hash-table-contains-p "ALPHA" equal-table)
   (hash-table-contains-p '(nested key) equal-table)
   (hash-table-contains-p '(nested . key) equal-table)
   (hash-table-count eq-table)
   (hash-table-count equal-table)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_gnu_hash_table_contains_p_argument_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(mapcar
 (lambda (thunk)
   (condition-case err
       (funcall thunk)
     (error (list (car err) (cadr err)))))
 (list
  (lambda () (hash-table-contains-p 'k nil))
  (lambda () (hash-table-contains-p 'k 'not-a-table))
  (lambda () (hash-table-contains-p))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
