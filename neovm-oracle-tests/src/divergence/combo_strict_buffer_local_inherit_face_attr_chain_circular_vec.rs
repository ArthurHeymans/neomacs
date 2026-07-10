//! Strict combo oracle probes, batch 121: buffer-local variable inheritance
//! via make-indirect-buffer, face-attribute inheritance chain, circular
//! vector read/write, and print-escape-multibyte on exotic chars.
//! Uses assert_oracle_parity_expect format.

use crate::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_t5_buffer_local_inheritance_indirect() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(
        r####"
(let* ((base (get-buffer-create " *probe-bli-base*"))
       (ind (make-indirect-buffer base " *probe-bli-ind*")))
  (unwind-protect
      (progn
        (with-current-buffer base
          (setq-local probe-bli-var 'base-value)
          (setq-local tab-width 4))
        (list (buffer-local-value 'probe-bli-var base)
              (buffer-local-value 'probe-bli-var ind)
              (buffer-local-value 'tab-width base)
              (buffer-local-value 'tab-width ind)
              (with-current-buffer ind
                (setq-local probe-bli-var 'ind-value)
                (buffer-local-value 'probe-bli-var base))
              (local-variable-p 'probe-bli-var base)
              (local-variable-p 'probe-bli-var ind)
              (eq (buffer-base-buffer ind) base)))
    (kill-buffer ind)
    (kill-buffer base)))
"####,
        expect,
    );
}

#[test]
fn div_t5_face_attribute_inheritance_chain() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(
        r####"
(let ((probe-face (make-face 'probe-inherit-face)))
  (set-face-attribute 'probe-inherit-face nil
                      :family "Monospace" :height 120 :weight 'bold)
  (let ((child-face (make-face 'probe-child-face)))
    (set-face-attribute 'probe-child-face nil :inherit 'probe-inherit-face)
    (list (face-attribute 'probe-inherit-face :family nil 'default)
          (face-attribute 'probe-inherit-face :weight nil 'default)
          (face-attribute 'probe-child-face :family nil 'default)
          (face-attribute 'probe-child-face :weight nil 'default)
          (face-attribute 'probe-child-face :height nil 'default)
          (face-attribute 'probe-child-face :inherit nil 'default)
          (face-all-attributes 'probe-child-face nil))))
"####,
        expect,
    );
}

#[test]
fn div_t5_circular_vector_read_write() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(
        r####"
(let* ((v (vector 1 2 3))
       (circular (read (let ((print-circle t)) (prin1-to-string
                         (progn (aset v 1 v) v))))))
  (list (aref circular 0)
        (eq (aref circular 1) circular)
        (aref (aref circular 1) 0)
        (aref (aref (aref circular 1) 1) 0)))
"####,
        expect,
    );
}

#[test]
fn div_t5_print_escape_multibyte_exotic() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(
        r####"
(list (let ((print-escape-multibyte t)) (prin1-to-string "café日本"))
      (let ((print-escape-multibyte nil)) (prin1-to-string "café日本"))
      (let ((print-escape-multibyte t)) (prin1-to-string (string 128 255 256)))
      (let ((print-escape-nonascii t)) (prin1-to-string "café"))
      (let ((print-escape-multibyte t) (print-escape-nonascii t))
        (prin1-to-string (string 233 345 65))))
"####,
        expect,
    );
}

#[test]
fn div_t5_setf_generalized_places_combo() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    let expect = expect_test::expect![[r#""""#]];
    crate::common::assert_oracle_parity_expect(
        r####"
(let ((lst (list 1 2 3))
      (vec [10 20 30])
      (sym (gensym)))
  (list (progn (setf (nth 1 lst) 'changed) lst)
        (progn (setf (aref vec 2) 'changed) vec)
        (progn (setf (get sym 'prop) 'val) (get sym 'prop))
        (progn (setf (car lst) 'first) lst)
        (progn (setf (cdr lst) '(rest)) lst)
        (progn (cl-rotatef (nth 0 lst) (nth 1 lst)) lst)
        (progn (cl-shiftf (nth 0 lst) (nth 1 lst) 'shifted) lst)))
"####,
        expect,
    );
}
