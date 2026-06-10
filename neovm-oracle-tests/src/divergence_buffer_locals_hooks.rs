//! Divergence tests: buffer locals deep - permanent locals, hooks, kill-all-locals.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_make_variable_buffer_local() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar my-perm-bl 0)
  (make-variable-buffer-local 'my-perm-bl)
  (setq my-perm-bl 42)
  (let ((buf (generate-new-buffer " *perm-bl-test*")))
    (with-current-buffer buf
      (list my-perm-bl
            (default-value 'my-perm-bl)))
    (kill-buffer buf)))"#,
    );
}

#[test]
fn divergence_buffer_local_which_file() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar my-blf-test 0)
  (make-local-variable 'my-blf-test)
  (setq my-blf-test 99)
  (list (mapcar (lambda (entry)
                  (if (eq (car-safe entry) 'buffer-display-time)
                      (list 'buffer-display-time
                            (and (consp (cdr entry))
                                 (integerp (cadr entry))
                                 (integerp (caddr entry))
                                 (integerp (cadddr entry)))))
                    entry))
                (buffer-local-variables))
        (assq 'my-blf-test (buffer-local-variables))))"#,
    );
}

#[test]
fn divergence_kill_all_local_variables() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar my-kalv-test 0)
  (make-local-variable 'my-kalv-test)
  (setq my-kalv-test 99)
  (kill-all-local-variables)
  (list my-kalv-test
        (local-variable-p 'my-kalv-test)
        (default-value 'my-kalv-test)))"#,
    );
}

#[test]
fn divergence_default_value_vssetq_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar my-dv-vs 0)
  (setq-default my-dv-vs 10)
  (make-local-variable 'my-dv-vs)
  (setq my-dv-vs 20)
  (list my-dv-vs
        (default-value 'my-dv-vs)
        (setq-default my-dv-vs 30)
        (default-value 'my-dv-vs)))"#,
    );
}

#[test]
fn divergence_set_default_to_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar my-sdn-test 0)
  (setq-default my-sdn-test nil)
  (list (default-value 'my-sdn-test)
        my-sdn-test
        (boundp 'my-sdn-test)))"#,
    );
}

#[test]
fn divergence_buffer_local_force() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar my-blf-var 0)
  (make-variable-buffer-local 'my-blf-var)
  (setq my-blf-var 1)
  (let ((buf (generate-new-buffer " *blf-test*")))
    (with-current-buffer buf
      (setq my-blf-var 2))
    (prog1
        (list (buffer-local-value 'my-blf-var (current-buffer))
              (buffer-local-value 'my-blf-var buf)
              (default-value 'my-blf-var))
      (kill-buffer buf))))"#,
    );
}

#[test]
fn divergence_hook_run_functions() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar my-hook-result nil)
  (add-hook 'my-test-hook-xyz (lambda () (push 1 my-hook-result)))
  (add-hook 'my-test-hook-xyz (lambda () (push 2 my-hook-result)))
  (run-hooks 'my-test-hook-xyz)
  my-hook-result)"#,
    );
}

#[test]
fn divergence_hook_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(progn
  (defvar my-hook-remove-result nil)
  (let ((fn (lambda () (push 'a my-hook-remove-result))))
    (add-hook 'my-test-hook-rm fn)
    (remove-hook 'my-test-hook-rm fn)
    (run-hooks 'my-test-hook-rm)
    my-hook-remove-result))"#,
    );
}

#[test]
fn divergence_change_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'before-change-functions)
  (boundp 'after-change-functions)
  (boundp 'first-change-hook)
  (listp before-change-functions)
  (listp after-change-functions))"#,
    );
}

#[test]
fn divergence_find_file_hooks() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        r#"(list
  (boundp 'find-file-hook)
  (listp find-file-hook)
  (boundp 'kill-buffer-hook)
  (listp kill-buffer-hook))"#,
    );
}
