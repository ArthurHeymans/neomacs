//! Lisp interface to the typed renderer-effect registry.

use super::*;
use crate::emacs_core::effect_profile::{
    EffectScope, effect_name_from_lisp, effect_operations_from_lisp, effect_set_operation_from_lisp,
};
use crate::emacs_core::error::{expect_args, expect_args_range, expect_min_args};
use neomacs_display_protocol::{EffectOperation, EffectValue, MotionPolicy, VisualConfig};

fn effect_error(function: &str, message: impl std::fmt::Display) -> Flow {
    signal(
        "error",
        vec![Value::string(format!("{function}: {message}"))],
    )
}

fn publish_visual_config(
    eval: &mut crate::emacs_core::eval::Context,
    function: &str,
    updated: VisualConfig,
) -> EvalResult {
    if let Some(host) = eval.display_host.as_mut() {
        host.set_visual_config(updated.clone())
            .map_err(|error| effect_error(function, error))?;
    }
    eval.visual_config = updated;
    Ok(Value::NIL)
}

fn apply_operations(
    eval: &mut crate::emacs_core::eval::Context,
    function: &str,
    base: &VisualConfig,
    operations: &[EffectOperation],
) -> EvalResult {
    let updated = base
        .apply_effects(operations)
        .map_err(|error| effect_error(function, error))?;
    publish_visual_config(eval, function, updated)
}

pub(crate) fn builtin_neomacs_effect_set(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_min_args("neomacs-effect-set", &args, 1)?;
    let operation = effect_set_operation_from_lisp(args[0], &args[1..], EffectScope::All)
        .map_err(|error| effect_error("neomacs-effect-set", error))?;
    let base = eval.visual_config.clone();
    apply_operations(eval, "neomacs-effect-set", &base, &[operation])
}

pub(crate) fn builtin_neomacs_effect_get(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("neomacs-effect-get", &args, 1)?;
    let effect = effect_name_from_lisp(args[0], EffectScope::All)
        .map_err(|error| effect_error("neomacs-effect-get", error))?;
    let properties = eval
        .visual_config
        .effect_values(&effect)
        .map_err(|error| effect_error("neomacs-effect-get", error))?;
    let mut result = Vec::with_capacity(properties.len() * 2);
    for (name, value) in properties {
        result.push(Value::keyword(format!(":{name}")));
        result.push(value_to_lisp(value));
    }
    Ok(Value::list(result))
}

/// Set or query the accessibility motion policy.
///
/// (neomacs-motion-policy) returns the current policy symbol; with 'full or
/// 'reduced it publishes a new policy, and the render thread gates cursor
/// motion, size transitions, buffer transitions, and every effect with an
/// enabled property accordingly.
pub(crate) fn builtin_neomacs_motion_policy(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("neomacs-motion-policy", &args, 0, 1)?;
    let current = eval.visual_config.motion;
    let Some(arg) = args.first() else {
        return Ok(Value::symbol(match current {
            MotionPolicy::Full => "full",
            MotionPolicy::Reduced => "reduced",
        }));
    };
    let policy = match arg.as_symbol_name() {
        Some("full") => MotionPolicy::Full,
        Some("reduced") => MotionPolicy::Reduced,
        _ => {
            return Err(effect_error(
                "neomacs-motion-policy",
                "expected 'full or 'reduced",
            ));
        }
    };
    let mut updated = eval.visual_config.clone();
    updated.motion = policy;
    publish_visual_config(eval, "neomacs-motion-policy", updated)?;
    Ok(Value::symbol(match current {
        MotionPolicy::Full => "full",
        MotionPolicy::Reduced => "reduced",
    }))
}

pub(crate) fn builtin_neomacs_effect_reset(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("neomacs-effect-reset", &args, 1)?;
    let effect = effect_name_from_lisp(args[0], EffectScope::All)
        .map_err(|error| effect_error("neomacs-effect-reset", error))?;
    let base = eval.visual_config.clone();
    apply_operations(
        eval,
        "neomacs-effect-reset",
        &base,
        &[EffectOperation::reset(effect)],
    )
}

pub(crate) fn builtin_neomacs_effects_apply(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args("neomacs-effects-apply", &args, 1)?;
    let operations = effect_operations_from_lisp(args[0], EffectScope::All)
        .map_err(|error| effect_error("neomacs-effects-apply", error))?;
    apply_operations(
        eval,
        "neomacs-effects-apply",
        &VisualConfig::default(),
        &operations,
    )
}

pub(crate) fn builtin_neomacs_effect_names(
    eval: &mut crate::emacs_core::eval::Context,
    args: Vec<Value>,
) -> EvalResult {
    expect_args_range("neomacs-effect-names", &args, 0, 1)?;
    let names = match args.first() {
        None => eval.visual_config.effect_names(),
        Some(value) if value.is_nil() => eval.visual_config.effect_names(),
        Some(value) => match value.as_symbol_name() {
            Some("shader") => eval.visual_config.effects.effect_names(),
            Some("cursor") => eval
                .visual_config
                .effects
                .effect_names()
                .into_iter()
                .filter(|name| name.starts_with("cursor-"))
                .collect(),
            Some("behavior") => {
                let shader_names = eval.visual_config.effects.effect_names();
                eval.visual_config
                    .effect_names()
                    .into_iter()
                    .filter(|name| !shader_names.contains(name))
                    .collect()
            }
            _ => {
                return Err(effect_error(
                    "neomacs-effect-names",
                    "scope must be nil, `shader`, `cursor`, or `behavior`",
                ));
            }
        },
    };
    Ok(Value::list(names.into_iter().map(Value::symbol).collect()))
}

fn value_to_lisp(value: EffectValue) -> Value {
    match value {
        EffectValue::Bool(false) => Value::NIL,
        EffectValue::Bool(true) => Value::T,
        EffectValue::Integer(value) => Value::fixnum(value),
        EffectValue::Number(value) => Value::make_float(value),
        EffectValue::Symbol(value) => Value::symbol(value),
        EffectValue::String(value) => Value::string(value),
        EffectValue::List(values) => Value::list(values.into_iter().map(value_to_lisp).collect()),
    }
}
