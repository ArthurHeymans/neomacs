//! Divergence tests: cl-extra, cl-seq, cl-macs deep.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_cl_remove_if() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(list
  (cl-remove-if #'cl-evenp '(1 2 3 4 5 6))
  (cl-remove-if-not #'cl-evenp '(1 2 3 4 5 6))
  (cl-remove-duplicates '(1 2 3 2 1 4)))"#,
    );
}

#[test]
fn divergence_cl_sort() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(cl-sort '(3 1 4 1 5 9 2 6) #'<)"#,
    );
}

#[test]
fn divergence_cl_subseq() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(list
  (cl-subseq '(a b c d e) 1 3)
  (cl-subseq '(a b c d e) 2)
  (cl-subseq "hello" 1 4))"#,
    );
}

#[test]
fn divergence_cl_position() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(list
  (cl-position 'b '(a b c b a))
  (cl-position 'b '(a b c b a) :from-end t)
  (cl-position 'z '(a b c)))"#,
    );
}

#[test]
fn divergence_cl_count() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(list
  (cl-count 'a '(a b a c a))
  (cl-count-if #'cl-evenp '(1 2 3 4 5 6))
  (cl-count-if-not #'cl-evenp '(1 2 3 4 5 6)))"#,
    );
}

#[test]
fn divergence_cl_reduce() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(list
  (cl-reduce #'+ '(1 2 3 4) :initial-value 10)
  (cl-reduce #'* '(1 2 3 4))
  (cl-replace (copy-sequence "abcdef") "XYZ" :start1 2))"#,
    );
}

#[test]
fn divergence_cl_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(cl-merge 'list '(1 3 5 7) '(2 4 6 8) #'<)"#,
    );
}

#[test]
fn divergence_cl_dolist_dotimes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(let ((result nil))
  (cl-dolist (x '(a b c) result)
    (push x result)))"#,
    );
}

#[test]
fn divergence_cl_destructuring_bind() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(list
  (cl-destructuring-bind (a b . c) '(1 2 3 4 5)
    (list a b c))
  (cl-destructuring-bind (&key x y) '(:x 10 :y 20)
    (list x y)))"#,
    );
}

#[test]
fn divergence_cl_the_check() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(list
  (cl-the fixnum 42)
  (cl-the string "hello")
  (cl-typep 42 'integer)
  (cl-typep "hello" 'string)
  (cl-typep 42 'string))"#,
    );
}

#[test]
fn divergence_cl_assoc_rassoc() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(require 'cl-lib)
(let ((alist '((a . 1) (b . 2) (c . 3))))
  (list (cl-assoc 'b alist)
        (cl-rassoc 3 alist)
        (cl-member 'b '(a b c))
        (cl-adjoin 'd '(a b c))))"#,
    );
}
