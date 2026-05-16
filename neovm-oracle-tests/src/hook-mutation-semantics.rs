//! Oracle parity tests for GNU `add-hook` and `remove-hook` mutation rules.
//!
//! GNU implements these helpers in `lisp/subr.el`.  These tests pin the
//! observable ordering and cleanup behavior around depth metadata, duplicate
//! detection by `equal`, and buffer-local hook bindings.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_add_hook_depth_order_and_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (defvar neomacs--oracle-hook-depth nil)
  (unwind-protect
      (progn
        (setq-default neomacs--oracle-hook-depth nil)
        (put 'neomacs--oracle-hook-depth 'hook--depth-alist nil)
        (add-hook 'neomacs--oracle-hook-depth 'zero-a)
        (add-hook 'neomacs--oracle-hook-depth 'zero-b)
        (add-hook 'neomacs--oracle-hook-depth 'pos-a 10)
        (add-hook 'neomacs--oracle-hook-depth 'pos-b 10)
        (add-hook 'neomacs--oracle-hook-depth 'neg-a -20)
        (add-hook 'neomacs--oracle-hook-depth 'legacy-depth t)
        (let* ((depth-sym (get 'neomacs--oracle-hook-depth 'hook--depth-alist))
               (depths (and depth-sym (default-value depth-sym))))
          (list
           (default-value 'neomacs--oracle-hook-depth)
           (symbolp depth-sym)
           (boundp depth-sym)
           (mapcar (lambda (fn) (alist-get fn depths :missing nil #'eq))
                   '(neg-a zero-b zero-a pos-a pos-b legacy-depth)))))
    (setq-default neomacs--oracle-hook-depth nil)
    (put 'neomacs--oracle-hook-depth 'hook--depth-alist nil)
    (makunbound 'neomacs--oracle-hook-depth)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_add_hook_duplicate_detection_uses_equal() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (defvar neomacs--oracle-hook-duplicates nil)
  (unwind-protect
      (let ((fn1 (lambda () 'same))
            (fn2 (lambda () 'same)))
        (setq-default neomacs--oracle-hook-duplicates nil)
        (put 'neomacs--oracle-hook-duplicates 'hook--depth-alist nil)
        (add-hook 'neomacs--oracle-hook-duplicates fn1 30)
        (add-hook 'neomacs--oracle-hook-duplicates fn2 -30)
        (list
         (length (default-value 'neomacs--oracle-hook-duplicates))
         (eq (car (default-value 'neomacs--oracle-hook-duplicates)) fn1)
         (eq (car (default-value 'neomacs--oracle-hook-duplicates)) fn2)))
    (setq-default neomacs--oracle-hook-duplicates nil)
    (put 'neomacs--oracle-hook-duplicates 'hook--depth-alist nil)
    (makunbound 'neomacs--oracle-hook-duplicates)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_remove_hook_removes_depth_metadata() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (defvar neomacs--oracle-hook-remove nil)
  (unwind-protect
      (progn
        (setq-default neomacs--oracle-hook-remove nil)
        (put 'neomacs--oracle-hook-remove 'hook--depth-alist nil)
        (add-hook 'neomacs--oracle-hook-remove 'keep 5)
        (add-hook 'neomacs--oracle-hook-remove 'drop 20)
        (let* ((depth-sym (get 'neomacs--oracle-hook-remove 'hook--depth-alist))
               (before (and depth-sym (default-value depth-sym))))
          (remove-hook 'neomacs--oracle-hook-remove 'drop)
          (let ((after (and depth-sym (default-value depth-sym))))
            (list
             (default-value 'neomacs--oracle-hook-remove)
             (alist-get 'keep before :missing nil #'eq)
             (alist-get 'drop before :missing nil #'eq)
             (alist-get 'keep after :missing nil #'eq)
             (alist-get 'drop after :missing nil #'eq)))))
    (setq-default neomacs--oracle-hook-remove nil)
    (put 'neomacs--oracle-hook-remove 'hook--depth-alist nil)
    (makunbound 'neomacs--oracle-hook-remove)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_remove_hook_local_binding_cleanup() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (defvar neomacs--oracle-hook-local nil)
  (unwind-protect
      (progn
        (setq-default neomacs--oracle-hook-local '(global-fn))
        (with-temp-buffer
          (list
           (local-variable-p 'neomacs--oracle-hook-local)
           (progn
             (add-hook 'neomacs--oracle-hook-local 'local-fn 10 t)
             (list neomacs--oracle-hook-local
                   (local-variable-p 'neomacs--oracle-hook-local)))
           (progn
             (remove-hook 'neomacs--oracle-hook-local 'local-fn t)
             (list neomacs--oracle-hook-local
                   (local-variable-p 'neomacs--oracle-hook-local)
                   (default-value 'neomacs--oracle-hook-local))))))
    (setq-default neomacs--oracle-hook-local nil)
    (put 'neomacs--oracle-hook-local 'hook--depth-alist nil)
    (makunbound 'neomacs--oracle-hook-local)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
