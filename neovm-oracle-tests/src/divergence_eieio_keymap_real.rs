//! Divergence tests: real EIEIO/object behavioral differences.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_defclass_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (defclass test-person-xxx ()
    ((name :initarg :name :accessor test-person-name-xxx)
     (age :initarg :age :accessor test-person-age-xxx :initform 0))
    \"A test person class.\")
  (let ((p (test-person-xxx \"Alice\" :name \"Alice\" :age 30)))
    (list (test-person-name-xxx p)
          (test-person-age-xxx p)
          (slot-value p 'name)
          (slot-value p 'age)
          (class-name (eieio-object-class p))))) ",
    );
}

#[test]
fn divergence_defclass_inheritance() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (defclass test-animal-xxx ()
    ((sound :initarg :sound :accessor test-animal-sound-xxx :initform \"\")))
  (defclass test-dog-xxx (test-animal-xxx)
    ((breed :initarg :breed :accessor test-dog-breed-xxx)))
  (let ((d (test-dog-xxx \"Rex\" :sound \"woof\" :breed \"labrador\")))
    (list (test-animal-sound-xxx d)
          (test-dog-breed-xxx d)
          (child-of-class-p (eieio-object-class d) 'test-animal-xxx)
          (same-class-p d 'test-dog-xxx)))) ",
    );
}

#[test]
fn divergence_defclass_method_dispatch() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (defclass test-shape-xxx () ())
  (defclass test-circle-xxx (test-shape-xxx) ((radius :initarg :radius)))
  (cl-defgeneric test-area-xxx (obj) \"Calculate area.\")
  (cl-defmethod test-area-xxx ((obj test-circle-xxx))
    (* float-pi (expt (slot-value obj 'radius) 2)))
  (let ((c (test-circle-xxx \"c\" :radius 5)))
    (list (> (test-area-xxx c) 0)
          (< (abs (- (test-area-xxx c) 78.5398)) 0.001)))) ",
    );
}

#[test]
fn divergence_eieio_oset() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (defclass test-box-xxx ()
    ((width :initarg :width :initform 0)
     (height :initarg :height :initform 0)))
  (let ((b (test-box-xxx \"b\" :width 10 :height 20)))
    (list (slot-value b 'width)
          (slot-value b 'height))
    (oset b width 99)
    (list (slot-value b 'width)
          (slot-value b 'height)))) ",
    );
}

#[test]
fn divergence_eieio_object_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn
  (defclass test-item-xxx ()
    ((label :initarg :label :initform \"\")))
  (let ((i (test-item-xxx \"i\" :label \"hello\")))
    (list (eieio-object-p i)
          (eieio-object-class-name i)
          (stringp (object-name i))
          (string-match \"test-item\" (object-name i))
          (not (eieio-object-p 42))
          (not (eieio-object-p nil))))) ",
    );
}

#[test]
fn divergence_keymap_basic_ops() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((map (make-sparse-keymap)))
  (define-key map \"a\" 'insert-char)
  (define-key map \"b\" 'forward-char)
  (list (keymapp map)
        (lookup-key map \"a\")
        (lookup-key map \"b\")
        (lookup-key map \"c\")
        (length map))) ",
    );
}

#[test]
fn divergence_keymap_prefix() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((map (make-sparse-keymap))
        (prefix (make-sparse-keymap)))
  (define-key prefix \"a\" 'beginning-of-line)
  (define-key map \"\\C-c\" prefix)
  (list (keymapp map)
        (lookup-key map \"\\C-ca\")
        (lookup-key map \"\\C-cb\")
        (keymapp (lookup-key map \"\\C-c\")))) ",
    );
}

#[test]
fn divergence_keymap_parent() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((parent (make-sparse-keymap))
        (child (make-sparse-keymap)))
  (define-key parent \"x\" 'exchange-point-and-mark)
  (set-keymap-parent child parent)
  (list (lookup-key child \"x\")
        (eq (keymap-parent child) parent)
        (lookup-key parent \"x\"))) ",
    );
}

#[test]
fn divergence_where_is_internal() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(let ((map (make-sparse-keymap)))
  (define-key map \"\\C-x\\C-f\" 'find-file)
  (define-key map \"\\C-x\\C-s\" 'save-buffer)
  (list (where-is-internal 'find-file map)
        (where-is-internal 'save-buffer map)
        (where-is-internal 'nonexistent-cmd-xxx map))) ",
    );
}

#[test]
fn divergence_key_description() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(list
  (key-description [?a])
  (key-description [?\\C-x ?\\C-f])
  (key-description [?\\M-x])
  (key-description [?\t])
  (key-description [return])) ",
    );
}
