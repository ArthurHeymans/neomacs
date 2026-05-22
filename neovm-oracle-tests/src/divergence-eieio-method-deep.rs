//! Divergence tests: EIEIO method dispatch, class hierarchy deep.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_eieio_core() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'defclass)
  (fboundp 'make-instance)
  (fboundp 'slot-value)
  (fboundp 'setf)
  (featurep 'eieio))"#,
    );
}

#[test]
fn divergence_eieio_class_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'find-class)
  (fboundp 'class-name)
  (fboundp 'class-of)
  (fboundp 'subclassp)
  (fboundp 'object-of-class-p)
  (fboundp 'child-of-class-p))"#,
    );
}

#[test]
fn divergence_eieio_slots() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'slot-boundp)
  (fboundp 'slot-makeunbound)
  (fboundp 'slot-exists-p)
  (fboundp 'with-slots)
  (fboundp 'oset))"#,
    );
}

#[test]
fn divergence_eieio_generic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'defgeneric)
  (fboundp 'defmethod)
  (fboundp 'generic-p)
  (fboundp 'no-next-method)
  (fboundp 'call-next-method)
  (fboundp 'next-method-p))"#,
    );
}

#[test]
fn divergence_eieio_accessors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'eieio--class-precedence-c3)
  (fboundp 'eieio--c3-candidate)
  (fboundp 'eieio-class-parents)
  (fboundp 'eieio-class-children))"#,
    );
}

#[test]
fn divergence_eieio_constructor() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'initialize-instance)
  (fboundp 'shared-initialize)
  (fboundp 'clone)
  (fboundp 'object-print))"#,
    );
}

#[test]
fn divergence_eieio_method_combination() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'eieio--defmethod)
  (fboundp 'eieio--defgeneric)
  (fboundp 'eieio-method-select))"#,
    );
}

#[test]
fn divergence_cl_defstruct_compat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'cl-defstruct)
  (fboundp 'cl-struct-setf-expander)
  (featurep 'cl-lib)
  (featurep 'cl-macs))"#,
    );
}

#[test]
fn divergence_record_type_compat() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'cl--make-random-access-record)
  (fboundp 'cl--random-access-record-p)
  (recordp (record 'tag 1 2 3))
  (length (record 'tag 1 2 3))) "#,
    );
}

#[test]
fn divergence_eieio_validation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (fboundp 'eieio-validate-slot-value)
  (fboundp 'eieio-perform-slot-validation-for-default)
  (fboundp 'eieio--slot-attribute)
  (fboundp 'eieio--slot-type)) "#,
    );
}
