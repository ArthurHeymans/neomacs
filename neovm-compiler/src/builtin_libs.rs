//! Built-in macro library sources for common Elisp packages.
//!
//! These are injected into the CompilerSession at creation time so that
//! `(require 'cl-lib)` works without needing actual .el files on disk.
//! Only macro definitions are included; runtime functions are omitted because
//! the compiler only needs macro definitions at expansion time.

/// Core macros from subr.el that are always available.
pub const CORE_MACROS_SOURCE: &str = r#"
;;; core-macros.el --- Always-available macros  -*- lexical-binding: t; -*-

(defmacro when (cond &rest body)
  (list 'if cond (cons 'progn body)))

(defmacro unless (cond &rest body)
  (list 'if cond nil (cons 'progn body)))

(provide 'core-macros)
"#;

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

(defmacro cl-block (name &rest body)
  (let ((tag (if (eq name nil) '--cl-block-nil-- name)))
    (cons 'catch (cons (list 'quote tag) body))))

(defmacro cl-return (&optional value)
  (list 'throw (list 'quote '--cl-block-nil--) (or value nil)))

(defmacro cl-return-from (block &optional value)
  (list 'throw (list 'quote block) (or value nil)))

(defmacro with-mutex (mutex &rest body)
  (let ((m (make-symbol \"m\")))
    `(let ((,m ,mutex))
       (mutex-lock ,m)
       (unwind-protect (progn ,@body)
         (mutex-unlock ,m)))))

(defmacro cl-adjoin (item list &rest keys)
  (list 'if (list 'memq item list) list (list 'cons item list)))

(defmacro cl-pushnew (item place &rest keys)
  (list 'setq place (list 'cl-adjoin item place)))

(defmacro cl-first (list) (list 'car list))
(defmacro cl-second (list) (list 'cadr list))
(defmacro cl-third (list) (list 'caddr list))
(defmacro cl-fourth (list) (list 'cadddr list))

(provide 'cl-lib)
"#;
