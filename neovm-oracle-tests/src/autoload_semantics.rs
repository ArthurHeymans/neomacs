//! Oracle parity tests for GNU autoload semantics.
//!
//! GNU implements `autoload` and `autoload-do-load` in `src/eval.c`;
//! `autoloadp` is Lisp in `lisp/subr.el` and is exactly an `eq` check against
//! `(car-safe OBJECT)`.  These tests cover the user-visible function-cell
//! shape and error ordering without depending on loading real files.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_autoloadp_uses_interned_autoload_car_safe_semantics() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((uninterned (make-symbol "autoload")))
  (list
   (autoloadp '(autoload "file" "doc" t nil))
   (autoloadp '(autoload . dotted-tail))
   (autoloadp (cons 'autoload nil))
   (autoloadp (cons uninterned nil))
   (autoloadp nil)
   (autoloadp 42)
   (autoloadp "autoload")))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_autoload_preserves_existing_real_definition_and_replaces_autoloads() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(unwind-protect
    (progn
      (fset 'neomacs--oracle-autoload-target
            (lambda () 'real-definition))
      (let ((real-cell (symbol-function 'neomacs--oracle-autoload-target)))
        (list
         (autoload 'neomacs--oracle-autoload-target
           "ignored-file" "Ignored doc." t)
         (eq (symbol-function 'neomacs--oracle-autoload-target) real-cell)
         (neomacs--oracle-autoload-target)
         (fmakunbound 'neomacs--oracle-autoload-target)
         (autoload 'neomacs--oracle-autoload-target
           "first-file" "First doc." nil 'macro)
         (symbol-function 'neomacs--oracle-autoload-target)
         (autoload 'neomacs--oracle-autoload-target
           "second-file" "Second doc." t 'keymap)
         (symbol-function 'neomacs--oracle-autoload-target))))
  (when (fboundp 'neomacs--oracle-autoload-target)
    (fmakunbound 'neomacs--oracle-autoload-target)))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_autoload_argument_errors_and_function_cell_state() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(unwind-protect
    (list
     (condition-case err
         (autoload 42 "file")
       (error (list (car err) (cdr err))))
     (condition-case err
         (autoload 'neomacs--oracle-autoload-bad-file 42)
       (error (list (car err) (cdr err))))
     (fboundp 'neomacs--oracle-autoload-bad-file)
     (autoload 'neomacs--oracle-autoload-good
       "good-file" nil '(mode-a mode-b) t)
     (let ((cell (symbol-function 'neomacs--oracle-autoload-good)))
       (list
        (autoloadp cell)
        (nth 1 cell)
        (nth 2 cell)
        (nth 3 cell)
        (nth 4 cell))))
  (dolist (sym '(neomacs--oracle-autoload-bad-file
                 neomacs--oracle-autoload-good))
    (when (fboundp sym)
      (fmakunbound sym))))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_autoload_do_load_macro_only_ordering_without_file_load() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((function-autoload '(autoload "missing-function-file" nil nil nil))
      (macro-autoload '(autoload "missing-macro-file" nil nil macro))
      (t-autoload '(autoload "missing-t-file" nil nil t)))
  (list
   (autoload-do-load 17 'ignored 'macro)
   (eq (autoload-do-load function-autoload 42 'macro)
       function-autoload)
   (condition-case err
       (autoload-do-load macro-autoload 42 'macro)
     (error (list (car err) (cdr err))))
   (condition-case err
       (autoload-do-load t-autoload 42 'macro)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_autoload_do_load_macro_only_requires_literal_macro_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((function-autoload '(autoload "missing-function-file" nil nil nil)))
  (list
   (condition-case err
       (autoload-do-load function-autoload 42 t)
     (error (list (car err) (cdr err))))
   (condition-case err
       (autoload-do-load function-autoload 42 'not-macro)
     (error (list (car err) (cdr err))))
   (condition-case err
       (autoload-do-load function-autoload 42 17)
     (error (list (car err) (cdr err)))))))
"#;

    assert_oracle_parity(form);
}
