//! Core reimplemented-function divergence probes.
//!
//! Functions commonly divergent when reimplemented from scratch: sort stability,
//! copy-tree depth (vector copying), mapcar/mapc over vectors, nreverse on
//! vectors, read of circular/shared structures, let-bound special var dynamics,
//! default-value/setq-default, indirect-variable aliasing, makunbound effects.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_aco_sort_stability() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(sort (copy-sequence '((1 . :a) (1 . :b) (2 . :c) (1 . :d)))
      (lambda (x y) (< (car x) (car y))))
"##,
    );
}

#[test]
fn div_aco_sort_stability_strings() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(sort (copy-sequence '("a3" "b1" "a1" "b2" "a2"))
      (lambda (x y) (string< (substring x 0 1) (substring y 0 1))))
"##,
    );
}

#[test]
fn div_aco_copy_tree_deep_vectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // copy-tree with vecp=t must deep-copy vectors.
    assert_oracle_parity(
        r##"
(let* ((v (vector 1 (vector 2 3)))
       (c (copy-tree v t)))
  (aset (aref c 1) 0 99)
  v)
"##,
    );
}

#[test]
fn div_aco_copy_tree_shallow_vectors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // copy-tree nil vecp shares vectors.
    assert_oracle_parity(
        r##"
(let* ((v (vector 1 (vector 2 3)))
       (c (copy-tree v)))
  (aset (aref c 1) 0 99)
  v)
"##,
    );
}

#[test]
fn div_aco_mapcar_over_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(mapcar #'identity [1 2 3])"##);
}

#[test]
fn div_aco_mapc_return_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((acc nil))
  (mapc (lambda (x) (push x acc)) '(1 2 3))
  acc)
"##,
    );
}

#[test]
fn div_aco_nreverse_vector() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(nreverse (copy-sequence [1 2 3 4]))"##);
}

#[test]
fn div_aco_read_circular_label() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(r##"(car (read-from-string "#1=(a . #1#)"))"##);
}

#[test]
fn div_aco_read_shared_label() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((x (car (read-from-string "(a #1=(b) c #1#)"))))
  (eq (nth 1 x) (nth 3 x)))
"##,
    );
}

#[test]
fn div_aco_read_struct_syntax() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(car (read-from-string (prin1-to-string #s(foo 1 2 3))))
"##,
    );
}

#[test]
fn div_aco_default_value_setq_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((neo-test-var 'original))
  (setq-default neo-test-var 'defaulted)
  (prog1 (default-value 'neo-test-var)
    (setq-default neo-test-var nil)))
"##,
    );
}

#[test]
fn div_aco_indirect_variable_alias() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(defvar neo-alias-target 42)
(defvaralias 'neo-alias 'neo-alias-target)
(list (indirect-variable 'neo-alias)
      neo-alias
      (setq neo-alias 99)
      neo-alias-target)
"##,
    );
}

#[test]
fn div_aco_makunbound_and_boundp() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(let ((neo-tmp 'x))
  (list (boundp 'neo-tmp)
        (makunbound 'neo-tmp)
        (boundp 'neo-tmp)))
"##,
    );
}

#[test]
fn div_aco_let_special_var_dynamic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    // A dynamically-let-bound var: set on the symbol sees the let value.
    assert_oracle_parity(
        r##"
(defvar neo-dyn 'outer)
(list (let ((neo-dyn 'inner)) (set 'neo-dyn 'modified) neo-dyn)
      neo-dyn)
"##,
    );
}
