use super::{NativeFn, NoEvalPolicy, SubrArity, SubrSpec, no_eval_policy};
use crate::emacs_core::eval::Context;
use crate::emacs_core::intern::intern;
use crate::emacs_core::value::Value;
use crate::tagged::header::SubrFn;

fn zero(_ctx: &mut Context) -> crate::emacs_core::error::EvalResult {
    Ok(Value::NIL)
}

fn two(_ctx: &mut Context, left: Value, _right: Value) -> crate::emacs_core::error::EvalResult {
    Ok(left)
}

fn vector(_ctx: &mut Context, arguments: Vec<Value>) -> crate::emacs_core::error::EvalResult {
    Ok(arguments.into_iter().next().unwrap_or(Value::NIL))
}

#[test]
fn vector_abi_does_not_imply_unbounded_lisp_arity() {
    let spec = SubrSpec::new(
        "test-fixed-vector",
        NativeFn::ContextVec(vector),
        SubrArity::new(1, Some(2)),
    );

    assert!(matches!(spec.function(), Some(SubrFn::Many(_))));
    assert_eq!(spec.arity(), SubrArity::new(1, Some(2)));
}

#[test]
fn native_function_shape_is_independent_of_lisp_arity() {
    let spec = SubrSpec::new(
        "test-optional-two-slot",
        NativeFn::Context2(two),
        SubrArity::new(1, Some(2)),
    );

    assert!(matches!(spec.function(), Some(SubrFn::A2(_))));
    assert_eq!(spec.arity(), SubrArity::new(1, Some(2)));
}

#[test]
#[should_panic(expected = "subr maximum arity must match its native function slots")]
fn fixed_slot_shape_rejects_a_different_maximum_arity() {
    let _ = SubrSpec::new(
        "test-invalid-two-slot",
        NativeFn::Context2(two),
        SubrArity::new(1, Some(1)),
    );
}

#[test]
fn registering_a_spec_installs_its_declared_metadata_and_function() {
    let mut ctx = Context::new();
    ctx.register_subr(SubrSpec::new(
        "test-zero",
        NativeFn::Context0(zero),
        SubrArity::new(0, Some(0)),
    ));

    let value = ctx
        .eval_str("(list (subr-arity (symbol-function 'test-zero)) (test-zero))")
        .expect("registered subr should be callable from Lisp");

    assert_eq!(format!("{value}"), "((0 . 0) nil)");
}

#[test]
fn registered_spec_is_authoritative_even_for_a_known_compatibility_name() {
    let mut ctx = Context::new();
    ctx.register_subr(SubrSpec::new(
        "message",
        NativeFn::Context0(zero),
        SubrArity::new(0, Some(0)),
    ));

    let arity = ctx
        .eval_str("(subr-arity (symbol-function 'message))")
        .expect("subr-arity should observe the descriptor");

    assert_eq!(format!("{arity}"), "(0 . 0)");
}

#[test]
fn registering_a_spec_replaces_its_previous_no_eval_policy() {
    let mut ctx = Context::new();
    let name = "test-authoritative-no-eval-policy";
    ctx.register_subr(
        SubrSpec::new(name, NativeFn::Context0(zero), SubrArity::new(0, Some(0)))
            .requires_eval_state(),
    );
    assert_eq!(
        no_eval_policy(intern(name)),
        NoEvalPolicy::RequiresEvalState
    );

    ctx.register_subr(SubrSpec::new(
        name,
        NativeFn::Context0(zero),
        SubrArity::new(0, Some(0)),
    ));
    assert_eq!(no_eval_policy(intern(name)), NoEvalPolicy::Native);
}
