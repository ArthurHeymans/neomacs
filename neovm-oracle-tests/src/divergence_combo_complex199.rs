//! Complex combo batch 199 — `eieio` `cl-defstruct` interop with
//! `cl-typep`, `cl-coerce`, `print-object`, slot documentation, and
//! `change-class` migration.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx199_eieio_change_class_migration() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'eieio)
      (defclass neo-cx199-v1 () ((x :initarg :x :initform 0)))
      (defclass neo-cx199-v2 () ((x :initarg :x :initform 0)
                                 (y :initarg :y :initform 0)))
      (let ((inst (make-instance 'neo-cx199-v1 :x 10)))
        (change-class inst 'neo-cx199-v2 :y 20)
        (list (slot-value inst 'x)
              (slot-value inst 'y)
              (class-of inst))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx199_eieio_print_object_custom_rendering() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'eieio)
      (defclass neo-cx199-po ()
        ((name :initarg :name :initform "anon")))
      (cl-defmethod cl-print-object ((o neo-cx199-po) stream)
        (princ (format "#<PO:%s>" (slot-value o 'name)) stream)
        o)
      (let ((inst (make-instance 'neo-cx199-po :name "alpha")))
        (list (let ((print-circle t)) (prin1-to-string inst))
              (let ((print-circle nil)) (prin1-to-string inst))
              (format "%S" inst)
              (format "%s" inst))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx199_eieio_cl_typep_with_eieio_classes() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'eieio)
      (defclass neo-cx199-base () ())
      (defclass neo-cx199-derived (neo-cx199-base) ())
      (let ((b (make-instance 'neo-cx199-base))
            (d (make-instance 'neo-cx199-derived)))
        (list (cl-typep b 'neo-cx199-base)
              (cl-typep b 'neo-cx199-derived)
              (cl-typep d 'neo-cx199-base)
              (cl-typep d 'neo-cx199-derived)
              (cl-typep d 'standard-object)
              (cl-typep d 'integer)
              (cl-typep 42 'integer))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx199_eieio_initialize_instance_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'eieio)
      (defclass neo-cx199-init ()
        ((computed :initform nil)))
      (cl-defmethod initialize-instance :after ((o neo-cx199-init) &rest _)
        (oset o computed :init-ran))
      (let ((inst (make-instance 'neo-cx199-init)))
        (slot-value inst 'computed)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx199_eieio_slot_boundp_and_makunbound() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'eieio)
      (defclass neo-cx199-sb ()
        ((x :initarg :x :initform unbound)
         (y :initarg :y :initform 0)))
      (let ((inst (make-instance 'neo-cx199-sb)))
        (list (slot-boundp inst 'x)
              (slot-boundp inst 'y)
              (slot-value inst 'y)
              (condition-case err (slot-value inst 'x) (error (car err)))
              (slot-makeunbound inst 'y)
              (slot-boundp inst 'y))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx199_eieio_with_slots_and_accessors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'eieio)
      (defclass neo-cx199-ws ()
        ((a :initarg :a :initform 0 :reader neo-cx199-get-a)
         (b :initarg :b :initform 0 :accessor neo-cx199-b)))
      (let ((inst (make-instance 'neo-cx199-ws :a 1 :b 2)))
        (list (neo-cx199-get-a inst)
              (neo-cx199-b inst)
              (setf (neo-cx199-b inst) 99)
              (neo-cx199-b inst)
              (slot-value inst 'a))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx199_eieio_object_of_class_p_predicate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'eieio)
      (defclass neo-cx199-root () ())
      (defclass neo-cx199-child (neo-cx199-root) ())
      (let ((root (make-instance 'neo-cx199-root))
            (child (make-instance 'neo-cx199-child)))
        (list (object-of-class-p root 'neo-cx199-root)
              (object-of-class-p root 'neo-cx199-child)
              (object-of-class-p child 'neo-cx199-root)
              (object-of-class-p child 'neo-cx199-child)
              (same-class-p root 'neo-cx199-root)
              (same-class-p child 'neo-cx199-root))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx199_eieio_class_parents_and_children_query() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'eieio)
      (defclass neo-cx199-p1 () ())
      (defclass neo-cx199-p2 () ())
      (defclass neo-cx199-c (neo-cx199-p1 neo-cx199-p2) ())
      (list (eieio-class-parents 'neo-cx199-c)
            (eieio-class-parents 'neo-cx199-p1)
            (eieio-class-children 'neo-cx199-p1)))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx199_eieio_class_definition_with_documentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'eieio)
      (defclass neo-cx199-doc ()
        ((x :initarg :x :initform 0
            :documentation "The X slot."))
        "Top-level class documentation.")
      (let ((inst (make-instance 'neo-cx199-doc :x 42)))
        (list (documentation-property 'neo-cx199-doc 'structure-documentation)
              (slot-value inst 'x))))
  (error (list :errored (car e))))
"##,
    );
}

#[test]
fn div_cx199_eieio_with_marker_overlay_undo_narrow_mega() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"
(condition-case e
    (progn
      (require 'eieio)
      (defclass neo-cx199-mega ()
        ((value :initarg :value :initform 0)
         (name :initarg :name :initform "anon")))
      (cl-defmethod neo-cx199-touch :before ((o neo-cx199-mega) v)
        (oset o value (+ (slot-value o 'value) v)))
      (cl-defmethod neo-cx199-touch ((o neo-cx199-mega) v)
        (oset o value (* (slot-value o 'value) v))
        (slot-value o 'value))
      (let ((inst (make-instance 'neo-cx199-mega :value 1 :name "test")))
        (with-temp-buffer
          (buffer-enable-undo)
          (insert "EIEIO mega interop test buffer content")
          (put-text-property 1 6 'face 'bold)
          (let ((m (set-marker (make-marker) 8))
                (ov (make-overlay 4 14)))
            (overlay-put ov 'face 'italic)
            (overlay-put ov 'evaporate t)
            (narrow-to-region 2 18)
            (let ((r (neo-cx199-touch inst 5)))
              (let ((state (list r (slot-value inst 'value) (slot-value inst 'name)
                                 (cl-typep inst 'neo-cx199-mega)
                                 (buffer-string)
                                 (marker-position m)
                                 (overlay-start ov) (overlay-end ov)
                                 (text-properties-at 1))))
                (undo)
                (widen)
                (list state (buffer-string) (marker-position m)
                      (overlay-start ov) (overlay-end ov)
                      (text-properties-at 1))))))))
  (error (list :errored (car e))))
"##,
    );
}
