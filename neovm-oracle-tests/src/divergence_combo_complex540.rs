/// Batch 540: closure, lexical-binding, function-call, apply, funcall edge.
use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn div_cx540_lexical_closure() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((lexical-binding t))
  (let ((x 5))
    (let ((f (lambda () (* x 2))))
      (funcall f))))
"##,
    );
}

#[test]
fn div_cx540_lexical_closure_mutate() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((lexical-binding t))
  (let ((x 10))
    (let ((f (lambda (n) (setq x (+ x n)))))
      (funcall f 5)
      x)))
"##,
    );
}

#[test]
fn div_cx540_apply_funcall() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (lambda (a b) (+ a b))))
  (list (funcall f 1 2) (apply f 3 4 nil)))
"##,
    );
}

#[test]
fn div_cx540_apply_partially() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (apply-partially #'+ 10)))
  (funcall f 5))
"##,
    );
}

#[test]
fn div_cx540_funcall_many() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(funcall (lambda (&rest args) (length args)) 1 2 3 4 5)
"##,
    );
}

#[test]
fn div_cx540_apply_many_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(apply #'+ 1 2 3 '(4 5 6))
"##,
    );
}

#[test]
fn div_cx540_apply_spread() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(apply #'list (append '(1 2) '(3 4)) nil)
"##,
    );
}

#[test]
fn div_cx540_closure_capture() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((lexical-binding t))
  (let ((count 0))
    (let ((f (lambda () (cl-incf count))))
      (funcall f) (funcall f) count)))
"##,
    );
}

#[test]
fn div_cx540_closure_redefine() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((lexical-binding t))
  (let ((fn (lambda (x) (* x 2))))
    (flet ((fn (x) (* x 3)))
      (funcall #'fn 5))))
"##,
    );
}

#[test]
fn div_cx540_lambda_with_doc() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (lambda (x) "docstring" (* x 2))))
  (list (documentation f) (funcall f 5)))
"##,
    );
}

#[test]
fn div_cx540_lambda_interactive() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((f (lambda (x) (interactive "p") (* x 2))))
  (commandp f))
"##,
    );
}

#[test]
fn div_cx540_closure_in_list() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(let ((lexical-binding t))
  (let ((x 42))
    (mapcar (lambda (e) (+ e x)) '(1 2 3))))
"##,
    );
}

#[test]
fn div_cx540_funcall_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(funcall '1+ 5)
"##,
    );
}

#[test]
fn div_cx540_apply_symbol() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(apply #'+ '(1 2 3))
"##,
    );
}

#[test]
fn div_cx540_funcall_no_args() {
    return_if_neovm_enable_oracle_proptest_not_set!();
    assert_oracle_parity(
        r##"(funcall (lambda () 42))
"##,
    );
}
