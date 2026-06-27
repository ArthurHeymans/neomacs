//! Strict combo oracle probes, batch 28: subr-x threading macros
//! (thread-first/last), when-let*/if-let*/let-alist, subr-x string ops,
//! cl-letf and setf on generalized places, and hash-table subr-x ops.
//!
//! Tests are parity locks unless annotated with a surfaced divergence.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_g3_threading_macros() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (thread-first 5 (1+) (1+))
      (thread-last 1 (+ 2) (* 3))
      (thread-first " abc " string-trim)
      (thread-first '(1 2 3) (mapcar #'1+) (length)))
"##,
    );
}

#[test]
fn div_g3_when_if_let_let_alist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (when-let* ((a 1) (b 2)) (+ a b))
      (when-let* ((a 1) (b nil)) (+ a b))
      (if-let* ((a 1) (b 2)) (+ a b) 'no)
      (if-let* ((a 1) (b nil)) (+ a b) 'no)
      (let-alist '((a . 1) (b . (2 . 3))) (list .a .b)))
"##,
    );
}

#[test]
fn div_g3_subr_x_string_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(list (string-blank-p "   ")
      (string-blank-p " x ")
      (string-empty-p "")
      (string-empty-p "x")
      (string-prefix-p "foo" "foobar")
      (string-suffix-p "bar" "foobar")
      (string-join '("a" "b" "c") "-")
      (string-trim "  abc  ")
      (string-trim-left "  abc")
      (string-trim-right "abc  ")
      (string-remove-prefix "foo" "foobar")
      (string-remove-suffix "bar" "foobar")
      (string-split "a,b,c" ","))
"##,
    );
}

#[test]
fn div_g3_cl_letf_and_setf_places() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((x (list 1 2 3))
      (v (vector 1 2 3))
      (sym (gensym)))
  (list (cl-letf (((car x) 9)) x)
        (cl-letf (((nth 1 x) 8)) (nth 1 x))
        (cl-letf (((aref v 0) 7)) v)
        (progn (put sym 'k 1) (setf (get sym 'k) 99) (get sym 'k))
        (let ((lst (list 1 2 3)))
          (cl-rotatef (car lst) (cadr lst))
          lst)))
"##,
    );
}

#[test]
fn div_g3_hash_table_subr_x_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((h (make-hash-table :test 'equal)))
  (puthash 'a 1 h)
  (puthash 'b 2 h)
  (puthash 'c 3 h)
  (let (sum)
    (maphash (lambda (_k val) (setq sum (+ sum val))) h)
    (list (sort (hash-table-keys h) #'symbol<)
          (sort (hash-table-values h) #'<)
          (hash-table-count h)
          sum)))
"##,
    );
}
