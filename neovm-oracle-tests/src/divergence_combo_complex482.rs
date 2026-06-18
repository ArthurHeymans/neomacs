/// Batch 482: eieio-persist, eieio-named, eieio-speedbar, eieio-custom, eieio-datatype.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx482_eieio_persist() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio-base)
  (list (boundp 'eieio-persist-version) (fboundp 'eieio-persist-read)))
"##,
    );
}

#[test]
fn div_cx482_eieio_singleton() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio)
  (defclass neo-cx482-singleton () ()
    (:allow-nil-initform t))
  (fboundp 'oref-default))
"##,
    );
}

#[test]
fn div_cx482_eieio_with_slots() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio)
  (defclass neo-cx482-ws () ((x :initarg :x) (y :initarg :y)))
  (let ((obj (make-instance 'neo-cx482-ws :x 10 :y 20)))
    (with-slots (x y) obj (+ x y))))
"##,
    );
}

#[test]
fn div_cx482_eieio_with_accessors() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio)
  (defclass neo-cx482-wa () ((val :initarg :val :accessor neo-cx482-val)))
  (let ((obj (make-instance 'neo-cx482-wa :val 42)))
    (neo-cx482-val obj)))
"##,
    );
}

#[test]
fn div_cx482_eieio_default_init() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio)
  (defclass neo-cx482-di () ((x :initform 10)))
  (let ((obj (make-instance 'neo-cx482-di)))
    (oref obj x)))
"##,
    );
}

#[test]
fn div_cx482_eieio_shared_init() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio)
  (defclass neo-cx482-si () ((x :initform (list 1 2 3) :allocation :class)))
  (let ((o1 (make-instance 'neo-cx482-si))
        (o2 (make-instance 'neo-cx482-si)))
    (list (oref o1 x) (oref o2 x))))
"##,
    );
}

#[test]
fn div_cx482_eieio_protected() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio)
  (defclass neo-cx482-prot () ((x :initarg :x :protection :protected)))
  (let ((obj (make-instance 'neo-cx482-prot :x 5)))
    (oref obj x)))
"##,
    );
}

#[test]
fn div_cx482_eieio_custom() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio-custom)
  (list (fboundp 'eieio-customize-object) (boundp 'eieio-custom-version)))
"##,
    );
}

#[test]
fn div_cx482_eieio_datatype() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio-datatype)
  (list (fboundp 'eieio-datatype-encode) (fboundp 'eieio-datatype-decode)))
"##,
    );
}

#[test]
fn div_cx482_eieio_list_tree() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio-speedbar)
  (list (fboundp 'eieio-speedbar-create) (boundp 'eieio-speedbar-version)))
"##,
    );
}

#[test]
fn div_cx482_eieio_multiple_construct() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio)
  (defclass neo-cx482-mc () ((x :initarg :x)))
  (let ((objs (cl-loop for i from 1 to 3 collect (make-instance 'neo-cx482-mc :x (* i 10)))))
    (mapcar (lambda (o) (oref o x)) objs)))
"##,
    );
}

#[test]
fn div_cx482_eieio_initarg_validate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio)
  (defclass neo-cx482-iv () ((x :initarg :x :type integer)))
  (list (fboundp 'oref) (fboundp 'oset)))
"##,
    );
}

#[test]
fn div_cx482_eieio_named_object() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio-named)
  (list (fboundp 'eieio-named-version) (boundp 'eieio-named-version)))
"##,
    );
}

#[test]
fn div_cx482_eieio_group() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio-group)
  (list (fboundp 'eieio-group-version) (boundp 'eieio-group-version)))
"##,
    );
}

#[test]
fn div_cx482_eieio_base() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(progn (require 'eieio-base)
  (list (boundp 'eieio-base-version) (fboundp 'eieio-persist-make-object)))
"##,
    );
}
