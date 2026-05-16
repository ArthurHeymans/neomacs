//! Oracle parity tests for GNU `letrec` macro semantics.
//!
//! GNU implements `letrec` in `lisp/subr.el`, not as a primitive special form.
//! Its macro expands initial non-recursive binders into `let*`, keeps recursive
//! binders in a `let` plus `setq` block, and accepts an omitted initializer as
//! nil via the same binder syntax as `let`/`let*`.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn oracle_letrec_macroexpansion_rewrite_shapes() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (macroexpand
  '(letrec ((neovm--lr-a 1)
            (neovm--lr-b neovm--lr-a))
     (+ neovm--lr-a neovm--lr-b)))
 (macroexpand
  '(letrec ((neovm--lr-a (lambda () (funcall neovm--lr-b)))
            (neovm--lr-b (lambda () 42)))
     (funcall neovm--lr-a)))
 (macroexpand
  '(letrec ((neovm--lr-a 1)
            (neovm--lr-b (lambda () neovm--lr-c))
            (neovm--lr-c 3))
     (list neovm--lr-a (funcall neovm--lr-b) neovm--lr-c))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_letrec_runtime_omitted_initializers_and_scope() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (letrec ((a (lambda () (funcall c)))
          (b)
          (c (lambda () b)))
   (setq b 'ok)
   (funcall a))
 (let ((events nil))
   (letrec ((a (progn (push 'init-a events) 1))
            (b (progn (push (list 'init-b a) events) (1+ a)))
            (c (lambda () (list a b events))))
     (funcall c)))
 (letrec ((a)
          (b (lambda () a)))
   (list a (funcall b))))
"#;

    assert_oracle_parity_with_bootstrap(form);
}

#[test]
fn oracle_letrec_nonrecursive_rewrite_edges() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    let form = r#"
(list
 (macroexpand '(letrec () 1 2 3))
 (macroexpand '(letrec ((neovm--lr-x 1)) neovm--lr-x))
 (macroexpand '(letrec ((neovm--lr-x 1)
                        (neovm--lr-y 2))
                 neovm--lr-y))
 (letrec () 1 2 3)
 (letrec ((x 1)) x)
 (letrec ((x 1)
          (y (+ x 2)))
   (list x y)))
"#;

    assert_oracle_parity_with_bootstrap(form);
}
