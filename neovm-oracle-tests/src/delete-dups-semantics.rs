//! Oracle parity tests for GNU `subr.el` `delete-dups`.
//!
//! GNU implements `delete-dups` in Lisp.  The small-list path repeatedly calls
//! `delete`, so it inherits GNU `delete`'s destructive mutation and tail-check
//! ordering on malformed input lists.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_delete_dups_small_list_is_destructive_and_keeps_first() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((first (list 'k 1))
       (second (list 'k 1))
       (third (list 'other 2))
       (fourth (list 'k 1))
       (xs (list first second third fourth))
       (result (delete-dups xs)))
  (list result
        xs
        (eq result xs)
        (eq (car result) first)
        (memq second result)
        (eq (cadr result) third)
        (memq fourth result)))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_delete_dups_small_list_mutates_before_improper_tail_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((xs (cons 'a (cons 'b (cons 'b 'tail)))))
  (list
   (condition-case err
       (delete-dups xs)
     (error (list (car err) (cdr err))))
   xs))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_delete_dups_large_list_uses_hash_path_and_keeps_first() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((first (list 'same))
       (middle (list 'same))
       (last (list 'same))
       (xs (append (list first)
                   (number-sequence 0 100)
                   (list middle last)))
       (result (delete-dups xs)))
  (list
   (eq result xs)
   (eq (car result) first)
   (memq middle result)
   (memq last result)
   (length result)
   (nth 1 result)
   (nth 101 result)
   xs))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_delete_dups_large_list_rejects_improper_tail_before_hash_walk() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((xs (append (number-sequence 0 100) 'tail)))
  (list
   (condition-case err
       (delete-dups xs)
     (error (list (car err) (cdr err))))
   xs))
"#;

    assert_oracle_parity(form);
}
