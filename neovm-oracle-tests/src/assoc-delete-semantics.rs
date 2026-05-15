//! Oracle parity tests for GNU `subr.el` destructive alist deletion.
//!
//! GNU implements `assoc-delete-all`, `assq-delete-all`, and
//! `rassq-delete-all` in Lisp.  Their semantics are intentionally destructive,
//! skip non-cons elements, and differ by equality predicate.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_prop_assoc_delete_all_removes_equal_keys_destructively() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((second (cons "keep" 2))
       (tail (list (cons "drop" 3) 'loose second (cons "drop" 4)))
       (alist (cons (cons "drop" 1) tail))
       (result (assoc-delete-all (copy-sequence "drop") alist)))
  (list result
        (eq result tail)
        (memq second result)
        alist
        tail))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_assoc_delete_all_custom_test_and_non_cons_elements() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((alist (list 'loose
                    (cons "alpha" 1)
                    42
                    (cons "ALPHA" 2)
                    (cons "beta" 3)
                    (cons "alpha" 4)))
       (result (assoc-delete-all "alpha" alist #'string-equal-ignore-case)))
  (list result
        (eq result alist)
        alist))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_assoc_delete_all_improper_alist_mutates_before_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let ((alist (cons (cons 'keep 1)
                   (cons (cons 'drop 2) 'tail))))
  (list
   (condition-case err
       (assoc-delete-all 'drop alist)
     (error (list (car err) (cdr err))))
   alist))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_assq_delete_all_uses_eq_not_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((key (list 'k))
       (equal-key (list 'k))
       (alist (list (cons key 1)
                    (cons equal-key 2)
                    (cons key 3)
                    'loose
                    (cons (list 'k) 4)))
       (result (assq-delete-all key alist)))
  (list result
        (equal (delq nil (mapcar (lambda (elt) (and (consp elt) (car elt))) result))
               (list equal-key (list 'k)))
        alist))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_prop_rassq_delete_all_removes_eq_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(let* ((value (list 'v))
       (equal-value (list 'v))
       (tail (list (cons 'b equal-value)
                   'loose
                   (cons 'c value)
                   (cons 'd 4)))
       (alist (cons (cons 'a value) tail))
       (result (rassq-delete-all value alist)))
  (list result
        (eq result tail)
        alist
        tail))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
