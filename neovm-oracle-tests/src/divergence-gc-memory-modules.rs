//! Divergence tests: gc, memory info, dump-emacs, dynamic-modules.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_gc_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'garbage-collect)
  (fboundp 'memory-info)
  (fboundp 'memory-use-counts)
  (consp (garbage-collect))
  (consp (memory-use-counts)))"#,
    );
}

#[test]
fn divergence_gc_cons_threshold() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (integerp gc-cons-threshold)
  (> gc-cons-threshold 0)
  (integerp gc-cons-percentage)
  (>= gc-cons-percentage 0.0))"#,
    );
}

#[test]
fn divergence_gc_elapsed() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (integerp gcs-done)
  (>= gcs-done 0)
  (numberp gc-elapsed)
  (>= gc-elapsed 0))"#,
    );
}

#[test]
fn divergence_dynamic_modules() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (featurep 'dynamic-modules)
  (fboundp 'module-load)
  (fboundp 'load))"#,
    );
}

#[test]
fn divergence_dump_emacs() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'dump-emacs)
  (fboundp 'dump-mode)
  (fboundp 'pdumper-load))"#,
    );
}

#[test]
fn divergence_pure_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'purecopy)
  (fboundp 'make-pure-string)
  (fboundp 'pure?)
  (listp pure-bytes-used))"#,
    );
}

#[test]
fn divergence_memory_limits() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (integerp most-positive-fixnum)
  (integerp most-negative-fixnum)
  (> most-positive-fixnum 0)
  (< most-negative-fixnum 0)
  (= (1+ most-positive-fixnum) most-negative-fixnum))"#,
    );
}

#[test]
fn divergence_bool_vector_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((a (make-bool-vector 8 t))
        (b (make-bool-vector 8 nil)))
  (aset b 0 t)
  (aset b 3 t)
  (aset b 7 t)
  (list (bool-vector-count-matches a t)
        (bool-vector-count-matches b t)
        (bool-vector-count-matches a nil)
        (bool-vector-count-matches b nil)))"#,
    );
}

#[test]
fn divergence_record_type_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((r (record 'cl-struct-tag 1 2 3)))
  (list (recordp r)
        (length r)
        (aref r 0)
        (aref r 1)
        (aref r 2)
        (aref r 3)))"#,
    );
}

#[test]
fn divergence_compiled_function_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (compiled-function-p (symbol-function 'car))
  (compiled-function-p (lambda (x) x))
  (compiled-function-p 'not-a-function-xyz)
  (subrp (symbol-function 'car)))"#,
    );
}
