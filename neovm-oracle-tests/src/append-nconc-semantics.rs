//! Oracle parity tests for GNU `append`/`nconc` edge semantics.
//!
//! GNU implements `append` through `concat_to_list` in `src/fns.c`: every
//! argument except the last is copied into fresh cons cells, while the last
//! argument is used directly as the final tail.  GNU `nconc` mutates preceding
//! list arguments and permits a non-list final argument.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_append_copies_prefix_and_shares_final_tail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((prefix (list 'a 'b))
       (tail (list 'c 'd))
       (result (append prefix tail)))
  (setcar prefix 'changed-prefix)
  (setcar tail 'changed-tail)
  (list result
        prefix
        tail
        (eq (nthcdr 2 result) tail)
        (eq result prefix)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_append_sequence_arguments_and_dotted_tail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((bv (make-bool-vector 4 nil)))
  (aset bv 1 t)
  (aset bv 3 t)
  (list
   (append "a中" [x y] bv '(tail))
   (append nil nil 'final-atom)
   (append "ab" 'tail)
   (condition-case err
       (append '(a b . c) '(tail))
     (error (list (car err) (cdr err))))
   (condition-case err
       (append 42 '(tail))
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_nconc_mutates_prefix_and_shares_tail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((a (list 'a1 'a2))
       (b (list 'b1 'b2))
       (first a)
       (result (nconc a b)))
  (setcar b 'changed-b)
  (list result
        first
        b
        (eq result first)
        (eq (nthcdr 2 result) b)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_nconc_nil_arguments_and_dotted_tail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((a (list 'a))
       (b (list 'b))
       (with-middle-nil (nconc a nil b))
       (dotted (nconc (list 'x 'y) 'tail)))
  (list with-middle-nil
        (eq with-middle-nil a)
        dotted
        (nconc nil nil 'final)
        (condition-case err
            (nconc 'not-last (list 'tail))
          (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_nconc_overwrites_dotted_nonfinal_tail() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((dotted (cons 'head 'old-tail))
       (tail (list 'new-tail))
       (result (nconc dotted tail)))
  (list result
        dotted
        tail
        (eq result dotted)
        (eq (cdr result) tail)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
