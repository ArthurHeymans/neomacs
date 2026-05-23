//! Deep combo: eieio class + inheritance + method dispatch + slot access.
//! Tests object system basics with class hierarchies.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn deficiency_defclass_basic_slots() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\n         (defclass my-point ()\n\n         ((x :initarg :x :initform 0 :accessor get-x)\n\n         (y :initarg :y :initform 0 :accessor get-y)))\n\n         (let ((p (my-point :x 10 :y 20)))\n\n         (list (get-x p) (get-y p)\n\n         (slot-value p 'x) (slot-value p 'y))))",
    );