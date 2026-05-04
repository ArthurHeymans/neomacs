//! Built-in macro library sources for common Elisp packages.
//!
//! These are injected into the CompilerSession at creation time so that
//! `(require 'cl-lib)` works without needing actual .el files on disk.
//! Only macro definitions are included; runtime functions are omitted because
//! the compiler only needs macro definitions at expansion time.

/// Minimal cl-lib macro definitions.
pub const CL_LIB_SOURCE: &str = r#"
;;; cl-lib.el --- Common Lisp extensions  -*- lexical-binding: t; -*-

(defmacro cl-incf (place &optional x)
  (list 'setq place (list '+ place (or x 1))))

(defmacro cl-decf (place &optional x)
  (list 'setq place (list '- place (or x 1))))

(defmacro cl-push (x place)
  (list 'setq place (list 'cons x place)))

(defmacro cl-pop (place)
  (let ((v (make-symbol "--cl-pop--")))
    (list 'let (list (list v place))
          (list 'prog1 (list 'car v)
                (list 'setq place (list 'cdr v))))))

(defmacro cl-rotatef (&rest args)
  (if (null args) nil
    (if (null (cdr args))
        (car args)
      (let ((tmp (make-symbol "--cl-rotatef--")))
        (list 'let (list (list tmp (car args)))
              (list 'setq (car args) (cadr args))
              (list 'setq (cadr args) tmp))))))

(defmacro cl-shiftf (&rest args)
  (if (null (cdr args)) nil
    (if (null (cddr args))
        (list 'prog1 (car args)
              (list 'setq (car args) (cadr args)))
      (list 'prog1 (car args)
            (list 'setq (car args) (cadr args))
            (cons 'cl-shiftf (cdr args))))))

(defmacro cl-assert (form &rest _)
  form)

(defmacro cl-check-type (form &rest _)
  form)

(defmacro cl-defun (name args &rest body)
  (cons 'defun (cons name (cons args body))))

(defmacro cl-return (&optional value)
  (list 'cl-return-from nil value))

(provide 'cl-lib)
"#;
