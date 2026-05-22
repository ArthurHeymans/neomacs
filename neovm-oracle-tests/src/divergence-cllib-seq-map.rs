//! Divergence tests: cl-lib deep, seq, map functions.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_cl_loop_collect() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'cl-lib)
(cl-loop for i from 1 to 5 collect (* i i))"#,
    );
}

#[test]
fn divergence_cl_loop_sum() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'cl-lib)
(cl-loop for i from 1 to 10 sum i)"#,
    );
}

#[test]
fn divergence_cl_loop_with_hashtable() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'cl-lib)
(let ((ht (make-hash-table)))
  (puthash 'a 1 ht)
  (puthash 'b 2 ht)
  (puthash 'c 3 ht)
  (sort (cl-loop for k being the hash-keys of ht collect k)
        #'symbol<))"#,
    );
}

#[test]
fn divergence_cl_loop_destructuring() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'cl-lib)
(cl-loop for (a b) in '((1 2) (3 4) (5 6))
         collect (+ a b))"#,
    );
}

#[test]
fn divergence_cl_flet_labels() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'cl-lib)
(cl-labels ((fact (n) (if (<= n 1) 1 (* n (fact (1- n))))))
  (list (fact 5) (fact 10)))"#,
    );
}

#[test]
fn divergence_cl_macrolet() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'cl-lib)
(cl-macrolet ((my-inc (x) (list '+ x 1)))
  (list (my-inc 5) (my-inc 10)))"#,
    );
}

#[test]
fn divergence_cl_values_mv() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'cl-lib)
(multiple-value-bind (a b c) (values 1 2 3)
  (list a b c))"#,
    );
}

#[test]
fn divergence_cl_case_ecase() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'cl-lib)
(list
  (cl-case 3 (1 'one) (2 'two) (3 'three) (t 'other))
  (cl-case 'b (a 1) ((b c) 2) (t 3)))"#,
    );
}

#[test]
fn divergence_cl_typecase() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'cl-lib)
(list
  (cl-typecase 42 (string 'str) (integer 'int) (t 'other))
  (cl-typecase "hi" (string 'str) (integer 'int) (t 'other))
  (cl-typecase '(1 2) (string 'str) (integer 'int) (cons 'pair) (t 'other)))"#,
    );
}

#[test]
fn divergence_seq_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'seq)
(list
  (seq-map #'1+ '(1 2 3))
  (seq-filter #'cl-evenp '(1 2 3 4 5))
  (seq-reduce #'+ '(1 2 3 4) 0)
  (seq-find #'cl-oddp '(2 4 5 6))
  (seq-contains '(a b c) 'b)
  (seq-contains-p '(a b c) 'd))"#,
    );
}

#[test]
fn divergence_seq_sort_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'seq)
(list
  (seq-sort #'< '(3 1 4 1 5 9))
  (seq-group-by #'cl-evenp '(1 2 3 4 5 6))
  (seq-uniq '(1 2 3 2 1 4 3))
  (seq-concatenate 'vector '(a b) '(c d)))"#,
    );
}

#[test]
fn divergence_map_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(require 'map)
(let ((ht (make-hash-table)))
  (puthash 'x 1 ht)
  (puthash 'y 2 ht)
  (list (map-elt ht 'x)
        (map-keys ht)
        (map-values ht)
        (map-length ht)))"#,
    );
}
