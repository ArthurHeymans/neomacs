use super::{NoEvalPolicy, SubrArity, SubrSpec, no_eval_policy};
use crate::emacs_core::eval::Context;
use crate::emacs_core::intern::intern;
use crate::emacs_core::value::Value;

fn zero(_ctx: &mut Context) -> crate::emacs_core::error::EvalResult {
    Ok(Value::NIL)
}

fn two(_ctx: &mut Context, left: Value, _right: Value) -> crate::emacs_core::error::EvalResult {
    Ok(left)
}

#[test]
fn fixed_subr_spec_derives_maximum_arity_from_its_function_shape() {
    let spec = SubrSpec::a2("test-two", two).required_args(1);

    assert_eq!(spec.name(), "test-two");
    assert_eq!(spec.arity(), SubrArity::new(1, Some(2)));
}

#[test]
fn registering_a_spec_installs_its_declared_metadata_and_function() {
    let mut ctx = Context::new();
    ctx.register_subr(SubrSpec::a0("test-zero", zero));

    let value = ctx
        .eval_str("(list (subr-arity (symbol-function 'test-zero)) (test-zero))")
        .expect("registered subr should be callable from Lisp");

    assert_eq!(format!("{value}"), "((0 . 0) nil)");
}

#[test]
fn registered_spec_is_authoritative_even_for_a_known_compatibility_name() {
    let mut ctx = Context::new();
    ctx.register_subr(SubrSpec::a0("message", zero));

    let arity = ctx
        .eval_str("(subr-arity (symbol-function 'message))")
        .expect("subr-arity should observe the descriptor");

    assert_eq!(format!("{arity}"), "(0 . 0)");
}

#[test]
fn registering_a_spec_replaces_its_previous_no_eval_policy() {
    let mut ctx = Context::new();
    let name = "test-authoritative-no-eval-policy";
    ctx.register_subr(SubrSpec::a0(name, zero).requires_eval_state());
    assert_eq!(
        no_eval_policy(intern(name)),
        NoEvalPolicy::RequiresEvalState
    );

    ctx.register_subr(SubrSpec::a0(name, zero));
    assert_eq!(no_eval_policy(intern(name)), NoEvalPolicy::Native);
}
