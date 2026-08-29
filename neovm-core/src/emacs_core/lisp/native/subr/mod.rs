//! Native Lisp subroutine declarations.
//!
//! A [`SubrSpec`] is the single declaration consumed by startup registration:
//! it keeps the Lisp name, native function shape, arity, dispatch kind, and
//! command metadata together.  The runtime representation remains the static
//! `SymId`-indexed registry described in the static-subr design; this module is
//! the declaration seam above it.

use super::interactive::BuiltinInteractiveSpec;
use super::intern::{SymId, intern};
use crate::tagged::header::{
    SubrDispatchKind, SubrFn, SubrFn0, SubrFn1, SubrFn2, SubrFn3, SubrFnMany, SubrFnManySlice,
};
use std::sync::{Mutex, OnceLock};

/// Lisp-visible argument-count metadata for a native subroutine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SubrArity {
    min: u16,
    max: Option<u16>,
}

impl SubrArity {
    pub(crate) const fn new(min: u16, max: Option<u16>) -> Self {
        if let Some(max) = max {
            assert!(min <= max, "subr minimum arity exceeds its maximum");
        }
        Self { min, max }
    }

    const fn fixed(count: u16) -> Self {
        Self::new(count, Some(count))
    }

    pub(crate) const fn min(self) -> u16 {
        self.min
    }

    pub(crate) const fn max(self) -> Option<u16> {
        self.max
    }
}

/// Behavior used only by tests that exercise primitives without an evaluator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NoEvalPlaceholder {
    Nil,
    FixnumZero,
    WindowLineHeight,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NoEvalPolicy {
    Native,
    RequiresEvalState,
    Placeholder(NoEvalPlaceholder),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandDefault {
    Enabled,
    Disabled,
}

static NO_EVAL_POLICIES: OnceLock<Mutex<Vec<Option<NoEvalPolicy>>>> = OnceLock::new();

fn no_eval_policies() -> &'static Mutex<Vec<Option<NoEvalPolicy>>> {
    NO_EVAL_POLICIES.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn record_no_eval_policy(name: &str, policy: NoEvalPolicy) {
    let sym_id = intern(name);
    let mut policies = no_eval_policies()
        .lock()
        .expect("subr no-eval policy registry poisoned");
    let index = sym_id.0 as usize;
    if policies.len() <= index {
        policies.resize(index + 1, None);
    }
    policies[index] = Some(policy);
}

pub(crate) fn no_eval_policy(sym_id: SymId) -> NoEvalPolicy {
    no_eval_policies()
        .lock()
        .expect("subr no-eval policy registry poisoned")
        .get(sym_id.0 as usize)
        .copied()
        .flatten()
        .unwrap_or(NoEvalPolicy::Native)
}

/// Complete startup declaration for one Rust-backed Lisp function.
#[derive(Clone, Copy)]
pub(crate) struct SubrSpec {
    name: &'static str,
    function: Option<SubrFn>,
    arity: SubrArity,
    dispatch_kind: SubrDispatchKind,
    interactive_spec: Option<BuiltinInteractiveSpec>,
    no_eval_policy: NoEvalPolicy,
    command_default: CommandDefault,
}

impl SubrSpec {
    const fn native(name: &'static str, function: SubrFn, arity: SubrArity) -> Self {
        assert!(!name.is_empty(), "a subr must have a Lisp name");
        Self {
            name,
            function: Some(function),
            arity,
            dispatch_kind: SubrDispatchKind::Builtin,
            interactive_spec: None,
            no_eval_policy: NoEvalPolicy::Native,
            command_default: CommandDefault::Enabled,
        }
    }

    pub(crate) const fn many(
        name: &'static str,
        function: SubrFnMany,
        min: u16,
        max: Option<u16>,
    ) -> Self {
        Self::native(name, SubrFn::Many(function), SubrArity::new(min, max))
    }

    pub(crate) const fn many_slice(
        name: &'static str,
        function: SubrFnManySlice,
        min: u16,
        max: Option<u16>,
    ) -> Self {
        Self::native(name, SubrFn::ManySlice(function), SubrArity::new(min, max))
    }

    pub(crate) const fn many_requires_eval_state(
        name: &'static str,
        function: SubrFnMany,
        min: u16,
        max: Option<u16>,
    ) -> Self {
        Self::many(name, function, min, max).requires_eval_state()
    }

    pub(crate) const fn many_placeholder(
        name: &'static str,
        function: SubrFnMany,
        min: u16,
        max: Option<u16>,
        placeholder: NoEvalPlaceholder,
    ) -> Self {
        Self::many(name, function, min, max).placeholder(placeholder)
    }

    pub(crate) const fn many_disabled_command(
        name: &'static str,
        function: SubrFnMany,
        min: u16,
        max: Option<u16>,
    ) -> Self {
        Self::many(name, function, min, max).disabled_command()
    }

    pub(crate) const fn a0(name: &'static str, function: SubrFn0) -> Self {
        Self::native(name, SubrFn::A0(function), SubrArity::fixed(0))
    }

    pub(crate) const fn a1(name: &'static str, function: SubrFn1) -> Self {
        Self::native(name, SubrFn::A1(function), SubrArity::fixed(1))
    }

    pub(crate) const fn a2(name: &'static str, function: SubrFn2) -> Self {
        Self::native(name, SubrFn::A2(function), SubrArity::fixed(2))
    }

    pub(crate) const fn a3(name: &'static str, function: SubrFn3) -> Self {
        Self::native(name, SubrFn::A3(function), SubrArity::fixed(3))
    }

    /// Declare how many arguments of a fixed-shape entrypoint are required.
    /// Remaining slots receive Lisp nil, matching GNU optional arguments.
    pub(crate) const fn required_args(mut self, required: u16) -> Self {
        let Some(max) = self.arity.max else {
            panic!("required_args is only valid for fixed-shape subrs");
        };
        assert!(required <= max, "required arguments exceed function shape");
        self.arity = SubrArity::new(required, Some(max));
        self
    }

    pub(crate) const fn interactive(mut self, spec: BuiltinInteractiveSpec) -> Self {
        self.interactive_spec = Some(spec);
        self
    }

    pub(crate) const fn requires_eval_state(mut self) -> Self {
        self.no_eval_policy = NoEvalPolicy::RequiresEvalState;
        self
    }

    pub(crate) const fn placeholder(mut self, placeholder: NoEvalPlaceholder) -> Self {
        self.no_eval_policy = NoEvalPolicy::Placeholder(placeholder);
        self
    }

    pub(crate) const fn disabled_command(mut self) -> Self {
        self.command_default = CommandDefault::Disabled;
        self
    }

    pub(crate) const fn evaluator(
        name: &'static str,
        arity: SubrArity,
        dispatch_kind: SubrDispatchKind,
    ) -> Self {
        assert!(
            matches!(
                dispatch_kind,
                SubrDispatchKind::ContextCallable | SubrDispatchKind::SpecialForm
            ),
            "evaluator subrs require an evaluator-owned dispatch kind"
        );
        Self {
            name,
            function: None,
            arity,
            dispatch_kind,
            interactive_spec: None,
            no_eval_policy: NoEvalPolicy::RequiresEvalState,
            command_default: CommandDefault::Enabled,
        }
    }

    pub(crate) const fn name(self) -> &'static str {
        self.name
    }

    pub(crate) const fn function(self) -> Option<SubrFn> {
        self.function
    }

    pub(crate) const fn arity(self) -> SubrArity {
        self.arity
    }

    pub(crate) const fn dispatch_kind(self) -> SubrDispatchKind {
        self.dispatch_kind
    }

    pub(crate) const fn interactive_spec(self) -> Option<BuiltinInteractiveSpec> {
        self.interactive_spec
    }

    pub(crate) const fn no_eval_policy(self) -> NoEvalPolicy {
        self.no_eval_policy
    }

    pub(crate) const fn command_default(self) -> CommandDefault {
        self.command_default
    }
}

#[cfg(test)]
mod tests;
