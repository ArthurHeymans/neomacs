//! Oracle parity tests for symbol value-cell edge semantics.
//!
//! GNU implements `boundp`, `makunbound`, `symbol-value`, `set`,
//! `default-boundp`, `default-value`, and `set-default` in `src/data.c`.
//! These tests focus on constant write protection and void/default value
//! behavior around the value cell.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_symbol_value_void_and_default_void_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((sym (make-symbol "neomacs--oracle-void-symbol")))
  (list
   (boundp sym)
   (default-boundp sym)
   (condition-case err
       (symbol-value sym)
     (error (list (car err) (cdr err))))
   (condition-case err
       (default-value sym)
     (error (list (car err) (cdr err))))
   (set sym 'now-bound)
   (boundp sym)
   (default-boundp sym)
   (symbol-value sym)
   (default-value sym)
   (eq (makunbound sym) sym)
   (boundp sym)
   (default-boundp sym)
   (condition-case err
       (symbol-value sym)
     (error (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_set_and_set_default_protect_nil_and_t() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (condition-case err
     (set nil nil)
   (error (list (car err) (cdr err))))
 (condition-case err
     (set nil 'changed)
   (error (list (car err) (cdr err))))
 (condition-case err
     (set t t)
   (error (list (car err) (cdr err))))
 (condition-case err
     (set t nil)
   (error (list (car err) (cdr err))))
 (condition-case err
     (set-default nil nil)
   (error (list (car err) (cdr err))))
 (condition-case err
     (set-default t t)
   (error (list (car err) (cdr err))))
 (condition-case err
     (makunbound nil)
   (error (list (car err) (cdr err))))
 (condition-case err
     (makunbound t)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_keyword_value_cell_can_only_be_set_to_self() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((kw :neomacs-oracle-keyword-edge))
  (list
   (keywordp kw)
   (boundp kw)
   (default-boundp kw)
   (symbol-value kw)
   (default-value kw)
   ;; GNU's `set_internal' has special constant-write handling for keywords.
   (condition-case err
       (set kw kw)
     (t (list (car err) (cdr err))))
   (condition-case err
       (set-default kw kw)
     (t (list (car err) (cdr err))))
   (condition-case err
       (symbol-value kw)
     (t (list (car err) (cdr err))))
   (condition-case err
       (default-value kw)
     (t (list (car err) (cdr err))))
   (condition-case err
       (set kw 'changed)
     (t (list (car err) (cdr err))))
   (condition-case err
       (set-default kw 'changed)
     (t (list (car err) (cdr err))))
   (condition-case err
       (makunbound kw)
     (t (list (car err) (cdr err))))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_set_default_uses_default_cell_not_current_let_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((sym (make-symbol "neomacs--oracle-default-cell")))
  (set sym 'global-1)
  (list
   (symbol-value sym)
   (default-value sym)
   (let ((sym 'lexical-shadow))
     sym)
   (let ((old (symbol-value sym)))
     (let ((neomacs--oracle-dynamic-holder sym))
       (let ((neomacs--oracle-dynamic-holder 'dynamic-value))
         (set-default sym 'global-2)
         (list old
               neomacs--oracle-dynamic-holder
               (symbol-value sym)
               (default-value sym)))))
   (set-default sym 'global-3)
   (symbol-value sym)
   (default-value sym)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
