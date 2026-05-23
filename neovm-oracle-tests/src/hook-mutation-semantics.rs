//! Oracle parity tests for GNU `add-hook` and `remove-hook` mutation rules.
//!
//! GNU implements these helpers in `lisp/subr.el`.  These tests pin the
//! observable ordering and cleanup behavior around depth metadata, duplicate
//! detection by `equal`, and buffer-local hook bindings.

use super::common::assert_oracle_parity;
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

    assert_oracle_parity(form);
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

    assert_oracle_parity(form);
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

    assert_oracle_parity(form);
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

    assert_oracle_parity(form);
}

#[test]
fn oracle_add_hook_coerces_single_function_values() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (defvar neomacs--oracle-hook-single nil)
  (unwind-protect
      (progn
        (setq-default neomacs--oracle-hook-single 'existing-fn)
        (put 'neomacs--oracle-hook-single 'hook--depth-alist nil)
        (list
         (add-hook 'neomacs--oracle-hook-single 'new-fn)
         (default-value 'neomacs--oracle-hook-single)
         (progn
           (setq-default neomacs--oracle-hook-single
                         (lambda () 'lambda-value))
           (add-hook 'neomacs--oracle-hook-single 'after-lambda 20)
           (default-value 'neomacs--oracle-hook-single))))
    (setq-default neomacs--oracle-hook-single nil)
    (put 'neomacs--oracle-hook-single 'hook--depth-alist nil)
    (makunbound 'neomacs--oracle-hook-single)))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_remove_hook_local_without_local_binding_is_noop() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (defvar neomacs--oracle-hook-local-noop nil)
  (unwind-protect
      (progn
        (setq-default neomacs--oracle-hook-local-noop '(global-fn))
        (with-temp-buffer
          (list
           (local-variable-p 'neomacs--oracle-hook-local-noop)
           (remove-hook 'neomacs--oracle-hook-local-noop 'global-fn t)
           (local-variable-p 'neomacs--oracle-hook-local-noop)
           (symbol-value 'neomacs--oracle-hook-local-noop)
           (default-value 'neomacs--oracle-hook-local-noop))))
    (setq-default neomacs--oracle-hook-local-noop nil)
    (put 'neomacs--oracle-hook-local-noop 'hook--depth-alist nil)
    (makunbound 'neomacs--oracle-hook-local-noop)))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_add_hook_detects_legacy_local_hook_binding() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (defvar neomacs--oracle-hook-legacy-local nil)
  (unwind-protect
      (progn
        (setq-default neomacs--oracle-hook-legacy-local '(global-fn))
        (with-temp-buffer
          (make-local-variable 'neomacs--oracle-hook-legacy-local)
          (setq neomacs--oracle-hook-legacy-local '(legacy-local-fn))
          (list
           (add-hook 'neomacs--oracle-hook-legacy-local 'new-local-fn 5)
           neomacs--oracle-hook-legacy-local
           (default-value 'neomacs--oracle-hook-legacy-local)
           (local-variable-p 'neomacs--oracle-hook-legacy-local))))
    (setq-default neomacs--oracle-hook-legacy-local nil)
    (put 'neomacs--oracle-hook-legacy-local 'hook--depth-alist nil)
    (makunbound 'neomacs--oracle-hook-legacy-local)))
"#;

    assert_oracle_parity(form);
}

#[test]
fn oracle_add_hook_local_permanent_hook_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(progn
  (defvar neomacs--oracle-hook-permanent nil)
  (unwind-protect
      (progn
        (setq-default neomacs--oracle-hook-permanent '(global-fn))
        (put 'neomacs--oracle-hook-permanent 'hook--depth-alist nil)
        (put 'neomacs--oracle-hook-permanent 'permanent-local nil)
        (put 'neomacs--oracle-hook-permanent-fn 'permanent-local-hook t)
        (with-temp-buffer
          (list
           (add-hook 'neomacs--oracle-hook-permanent
                     'neomacs--oracle-hook-permanent-fn
                     7
                     t)
           neomacs--oracle-hook-permanent
           (local-variable-p 'neomacs--oracle-hook-permanent)
           (get 'neomacs--oracle-hook-permanent 'permanent-local)
           (let* ((depth-sym
                   (get 'neomacs--oracle-hook-permanent 'hook--depth-alist))
                  (local-depths (and depth-sym (symbol-value depth-sym))))
             (list (local-variable-p depth-sym)
                   (alist-get 'neomacs--oracle-hook-permanent-fn
                              local-depths :missing nil #'eq)))
           (default-value 'neomacs--oracle-hook-permanent))))
    (setq-default neomacs--oracle-hook-permanent nil)
    (put 'neomacs--oracle-hook-permanent 'hook--depth-alist nil)
    (put 'neomacs--oracle-hook-permanent 'permanent-local nil)
    (put 'neomacs--oracle-hook-permanent-fn 'permanent-local-hook nil)
    (makunbound 'neomacs--oracle-hook-permanent)))
"#;

    assert_oracle_parity(form);
}
