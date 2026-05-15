//! Oracle parity tests for GNU symbol property semantics.
//!
//! GNU implements `symbol-plist` and `setplist` in `src/data.c`, while `get`
//! and `put` are in `src/fns.c`.  These tests cover the symbol-specific layer
//! on top of plist handling, including `overriding-plist-environment`.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_symbol_plist_returns_live_property_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((sym (make-symbol "neomacs-oracle-live-plist")))
  (setplist sym (list 'a 1 'b 2))
  (let ((plist (symbol-plist sym)))
    (setcar (cdr plist) 11)
    (setcdr (cddr plist) (list 'c 3))
    (list
     (get sym 'a)
     (get sym 'b)
     (get sym 'c)
     (eq plist (symbol-plist sym))
     (symbol-plist sym))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_setplist_accepts_malformed_plist_and_put_validates_when_needed() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((sym (make-symbol "neomacs-oracle-malformed-plist")))
  (setplist sym '(a 1 b . bad-tail))
  (list
   (get sym 'a)
   (get sym 'b)
   (get sym 'missing)
   (put sym 'b 22)
   (get sym 'b)
   (condition-case err
       (put sym 'c 3)
     (error (list (car err) (cdr err))))
   (symbol-plist sym)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_get_uses_overriding_plist_environment_only_for_non_nil_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((sym (make-symbol "neomacs-oracle-override")))
  (put sym 'a 1)
  (put sym 'b 2)
  (put sym 'c nil)
  (let ((overriding-plist-environment (list (list sym 'a 10 'b nil 'c 30))))
    (list
     (get sym 'a)
     (get sym 'b)
     (get sym 'c)
     (get sym 'missing)
     (symbol-plist sym))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_symbol_property_type_errors() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (condition-case err
     (symbol-plist "not-symbol")
   (error (list (car err) (cdr err))))
 (condition-case err
     (setplist 42 nil)
   (error (list (car err) (cdr err))))
 (condition-case err
     (get '(not . symbol) 'a)
   (error (list (car err) (cdr err))))
 (condition-case err
     (put nil 'a 1)
   (error (list (car err) (cdr err)))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
