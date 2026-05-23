//! Deep combo: defmacro + eval-and-compile + eval-when-compile + macroexpand.
//! Tests macro expansion semantics with compile-time evaluation.

use super::common::assert_oracle_parity;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn deficiency_defmacro_basic_expansion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defmacro my-add (a b)\n\
         (list '+ a b))\n\
         (list (my-add 3 4)\n\
         (macroexpand '(my-add 3 4))))",
    );
}

#[test]
fn deficiency_defmacro_with_gensym() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defmacro my-swap (a b)\n\
         (let ((tmp (make-symbol \"tmp\")))\n\
         (list 'let (list (list tmp a))\n\
         (list 'setq a b)\n\
         (list 'setq b tmp))))\n\
         (let ((x 10) (y 20))\n\
         (my-swap x y)\n\
         (list x y)))",
    );
}

#[test]
fn deficiency_defmacro_nested_quasiquote() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defmacro my-let1 (var val &rest body)\n\
         (list 'let (list (list var val)) (cons 'progn body)))\n\
         (my-let1 x 42\n\
         (+ x 1)\n\
         (* x 2)))",
    );
}

#[test]
fn deficiency_eval_and_compile_defines_at_compile() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (eval-and-compile\n\
         (defvar eac-val 42))\n\
         (list eac-val\n\
         (boundp 'eac-val)))",
    );
}

#[test]
fn deficiency_macroexpand_does_not_eval() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defmacro identity-macro (x)\n\
         x)\n\
         (let ((expanded (macroexpand '(identity-macro (+ 1 2)))))\n\
         (list expanded (equal expanded '(+ 1 2)))))",
    );
}

#[test]
fn deficiency_defmacro_with_body_wrapping() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defmacro with-gensym (name &rest body)\n\
         (declare (indent 1))\n\
         (list 'let (list (list name '(gensym)))\n\
         (cons 'progn body)))\n\
         (with-gensym sym\n\
         (list sym (symbolp sym))))",
    );
}

#[test]
fn deficiency_macro_expansion_recursive() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defmacro triple (x)\n\
         (list '+ x (list '+ x x)))\n\
         (list (triple 5)\n\
         (macroexpand '(triple 5))))",
    );
}

#[test]
fn deficiency_defmacro_with_destructuring() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defmacro my-destructure ((a b) &rest body)\n\
         (list 'let (list (list a 1) (list b 2))\n\
         (cons 'progn body)))\n\
         (my-destructure (x y)\n\
         (list x y (+ x y))))",
    );
}

#[test]
fn deficiency_macroexpand_all_nested() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defmacro add1 (x) (list '1+ x))\n\
         (let ((form '(progn (add1 5) (add1 (add1 3)))))\n\
         (macroexpand-all form)))",
    );
}

#[test]
fn deficiency_eval_when_compile_side_effects() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity(
        "(progn\n\
         (defvar ewc-check nil)\n\
         (eval-when-compile\n\
         (setq ewc-check 'compiled))\n\
         (list ewc-check\n\
         (boundp 'ewc-check)))",
    );
}
