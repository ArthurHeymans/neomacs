//! Divergence tests: eval, apply, funcall edge cases.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_eval_nested_environment() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((x 'outer))
  (list x
        (let ((x 'inner))
          (eval 'x))
        (eval 'x)))"#,
    );
}

#[test]
fn divergence_funcall_interactively() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (called-interactively-p 'interactive)
  (interactive-p))"#,
    );
}

#[test]
fn divergence_function_quoting() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((fn1 (function (lambda (x) (1+ x))))
        (fn2 #'(lambda (x) (1+ x))))
  (list (funcall fn1 41)
        (funcall fn2 41)
        (functionp fn1)
        (functionp fn2)))"#,
    );
}

#[test]
fn divergence_defalias_and_fset() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defalias 'my-alias-fn #'car)
  (list (my-alias-fn '(1 2 3))
        (symbol-function 'my-alias-fn)
        (fmakunbound 'my-alias-fn)
        (fboundp 'my-alias-fn)))"#,
    );
}

#[test]
fn divergence_special_form_p() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (special-form-p (symbol-function 'if))
  (special-form-p (symbol-function 'let))
  (special-form-p (symbol-function 'condition-case))
  (special-form-p (symbol-function 'car))
  (special-form-p (symbol-function 'and)))"#,
    );
}

#[test]
fn divergence_setq_default() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defvar my-sdq-var 0)
  (setq-default my-sdq-var 50)
  (let ((buf1 (get-buffer-create " *test-sdq1*"))
        (buf2 (get-buffer-create " *test-sdq2*")))
    (unwind-protect
        (progn
          (with-current-buffer buf1
            (setq-local my-sdq-var 100))
          (list (default-value 'my-sdq-var)
                (buffer-local-value 'my-sdq-var buf1)
                (buffer-local-value 'my-sdq-var buf2)))
      (kill-buffer buf1)
      (kill-buffer buf2))))"#,
    );
}

#[test]
fn divergence_dynamic_binding_with_let() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(progn
  (defvar my-dyn-var 100)
  (let ((my-dyn-var 200))
    (list my-dyn-var
          (eval 'my-dyn-var)
          (let ((my-dyn-var 300))
            (list my-dyn-var (eval 'my-dyn-var)))))"#,
    );
}

#[test]
fn divergence_backtrace_frames() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((frames nil))
  (condition-case err
      (letrec ((f (lambda (n)
                    (if (= n 0)
                        (signal 'error "bottom")
                      (funcall f (1- n))))))
        (funcall f 3))
    (error
     (let ((bt (with-output-to-string
                  (backtrace))))
       (if (> (length bt) 0) 'has-backtrace 'no-backtrace))))"#,
    );
}

#[test]
fn divergence_obarray_intern() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((ob (make-obarray 13)))
  (intern "hello" ob)
  (intern "world" ob)
  (list (intern-soft "hello" ob)
        (intern-soft "world" ob)
        (intern-soft "missing" ob)
        (let (count)
          (mapatoms (lambda (s) (push s count)) ob)
          (length count))))"#,
    );
}

#[test]
fn divergence_unintern_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((sym (intern "test-unintern-me")))
  (list (intern-soft "test-unintern-me")
        (unintern "test-unintern-me")
        (intern-soft "test-unintern-me")))"#,
    );
}
