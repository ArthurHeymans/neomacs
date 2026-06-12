//! Per-thread compiled-code cache and the baseline JIT tier-up entry point.
//!
//! The dispatch seam (`eval.rs`) calls [`try_run_compiled`] once a function is
//! hot ([`super::Plan::Compiled`]). Compiled code is cached **per thread**,
//! keyed by the function's stable [`super::Runtime::compiled_id`]:
//!
//! - A [`CompiledLeaf`] owns executable memory and a raw code pointer, so it is
//!   `!Send + !Sync`. Keeping it thread-local means it is never shared across
//!   threads — sound by construction, and a fine fit for elisp's overwhelmingly
//!   single-threaded execution. (Each thread that runs a function hot enough
//!   compiles its own copy; in practice that is just the main thread.)
//! - The id is monotonic and never reused, so a function that is GC'd (freeing
//!   the memory its compiled code baked constant pointers into) can never have
//!   its stale cache entry looked up again — even after the non-moving GC reuses
//!   its heap address, the new function there gets a *new* id. Stale entries for
//!   dead functions linger until thread exit (a bounded leak), never a
//!   use-after-free.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::compile::{CompiledLeaf, NativeRun, compile_bytecode_function, take_pending_flow};
use crate::emacs_core::bytecode::ByteCodeFunction;
use crate::emacs_core::error::Flow;
use crate::emacs_core::eval::Context;
use crate::emacs_core::value::Value;

/// One thread's knowledge of a function's compiled state.
enum CacheEntry {
    /// Native code, ready to run. `Rc` so execution can happen *outside* the
    /// cache borrow — compiled code can `Call` back into elisp, and a hot callee
    /// re-enters this cache (a nested `borrow_mut` would panic).
    Compiled(Rc<CompiledLeaf>),
    /// The body is outside the baseline JIT's supported subset; never retried.
    NotCompilable,
}

thread_local! {
    /// `compiled_id` -> compiled state, owned by and private to this thread.
    static COMPILED: RefCell<HashMap<u64, CacheEntry>> = RefCell::new(HashMap::new());
}

/// Tier-up entry point: run `func`'s body as native code if possible.
///
/// - `Ok(Some(bits))` — native code produced the result (raw tagged bits).
/// - `Ok(None)` — fall back to the Tier-0 interpreter: the body is not
///   compilable by this tier, the arity didn't match (the interpreter must
///   signal wrong-number-of-arguments), or compiled code **deoptimized**. A
///   deopt can only happen before any side effect (the guard-after-call
///   poisoning analysis rejects everything else), so rerunning is sound.
/// - `Err(flow)` — a runtime call inside native code raised a non-local exit;
///   propagate it.
///
/// `ctx` is the `Context` the dispatch seam is executing in; runtime-call shims
/// re-enter elisp through it. Compiles on first use (per thread) and caches the
/// outcome, so a non-compilable body is only attempted once.
/// Debug aid: when `NEOVM_JIT_MAX_ID` is set, only functions whose
/// `compiled_id` is <= it run natively — bisecting a misbehaving compiled
/// function out of a workload (ids are assigned in first-hot order, so this is
/// a clean prefix bisection).
fn max_compiled_id() -> u64 {
    use std::sync::OnceLock;
    static MAX: OnceLock<u64> = OnceLock::new();
    *MAX.get_or_init(|| {
        std::env::var("NEOVM_JIT_MAX_ID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(u64::MAX)
    })
}

pub fn try_run_compiled(
    ctx: *mut Context,
    func: &ByteCodeFunction,
    args: &[Value],
) -> Result<Option<usize>, Flow> {
    let id = func.runtime.compiled_id_or_assign();
    if id > max_compiled_id() {
        return Ok(None);
    }
    // Debug aid: dump the body of one compiled function by id.
    {
        use std::sync::OnceLock;
        static DEBUG_ID: OnceLock<Option<u64>> = OnceLock::new();
        let dbg = *DEBUG_ID.get_or_init(|| {
            std::env::var("NEOVM_JIT_DEBUG_ID")
                .ok()
                .and_then(|s| s.parse().ok())
        });
        if dbg == Some(id) {
            let consts: Vec<String> = func
                .constants
                .iter()
                .map(crate::emacs_core::print::print_value)
                .collect();
            eprintln!(
                "[jit-debug] id={id} args={} ops={:?} constants={consts:?}",
                args.len(),
                func.ops,
            );
        }
    }
    let leaf: Option<Rc<CompiledLeaf>> = COMPILED.with(|cache| {
        let mut cache = cache.borrow_mut();
        match cache
            .entry(id)
            .or_insert_with(|| match compile_bytecode_function(func) {
                Ok(leaf) => CacheEntry::Compiled(Rc::new(leaf)),
                Err(_) => CacheEntry::NotCompilable,
            }) {
            // Only run native for a valid call (lambda-list range); a mismatch
            // is a wrong-arg-count call the interpreter must signal.
            CacheEntry::Compiled(leaf) if leaf.accepts(args.len()) => Some(Rc::clone(leaf)),
            _ => None,
        }
    });
    // Execute OUTSIDE the cache borrow (see `CacheEntry::Compiled`).
    match leaf {
        None => Ok(None),
        Some(leaf) => match leaf.call(ctx as *mut u8, args) {
            NativeRun::Ok(bits) => Ok(Some(bits)),
            NativeRun::Deopt => Ok(None),
            NativeRun::Signal => Err(take_pending_flow()
                .expect("STATUS_SIGNAL from compiled code implies a stashed Flow")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emacs_core::bytecode::opcode::Op;
    use crate::emacs_core::value::{LambdaParams, Value};

    fn nullary_fn(ops: Vec<Op>, constants: Vec<Value>) -> ByteCodeFunction {
        let mut f = ByteCodeFunction::new(LambdaParams {
            required: Vec::new(),
            optional: Vec::new(),
            rest: None,
        });
        f.ops = ops;
        f.constants = constants;
        f.max_stack = 16;
        f
    }

    #[test]
    fn runs_compilable_nullary_leaf() {
        let c = Value::make_int(42);
        let f = nullary_fn(vec![Op::Constant(0), Op::Return], vec![c]);
        // First call compiles + caches; result is the constant's bits.
        assert_eq!(
            try_run_compiled(std::ptr::null_mut(), &f, &[]).unwrap(),
            Some(c.bits())
        );
        // Second call hits the cache; same result.
        assert_eq!(
            try_run_compiled(std::ptr::null_mut(), &f, &[]).unwrap(),
            Some(c.bits())
        );
    }

    #[test]
    fn returns_none_for_noncompilable_body() {
        // Switch is unsupported -> NotCompilable -> None (interpreter fallback).
        let f = nullary_fn(
            vec![Op::Nil, Op::Nil, Op::Switch, Op::Nil, Op::Return],
            vec![],
        );
        assert_eq!(
            try_run_compiled(std::ptr::null_mut(), &f, &[]).unwrap(),
            None
        );
        assert_eq!(
            try_run_compiled(std::ptr::null_mut(), &f, &[]).unwrap(),
            None
        );
    }

    #[test]
    fn deopt_returns_none() {
        // MOST_POSITIVE + 1 overflows fixnum range -> native deopts -> None.
        let f = nullary_fn(
            vec![Op::Constant(0), Op::Constant(1), Op::Add, Op::Return],
            vec![
                Value::make_int(Value::MOST_POSITIVE_FIXNUM),
                Value::make_int(1),
            ],
        );
        assert_eq!(
            try_run_compiled(std::ptr::null_mut(), &f, &[]).unwrap(),
            None
        );
    }

    #[test]
    fn assigns_stable_unique_ids() {
        let f1 = nullary_fn(vec![Op::Nil, Op::Return], vec![]);
        let f2 = nullary_fn(vec![Op::Nil, Op::Return], vec![]);
        let a = f1.runtime.compiled_id_or_assign();
        let a_again = f1.runtime.compiled_id_or_assign();
        let b = f2.runtime.compiled_id_or_assign();
        assert_eq!(a, a_again, "id is stable per function");
        assert_ne!(a, b, "distinct functions get distinct ids");
        assert_ne!(a, 0, "0 is reserved for unassigned");
    }

    #[test]
    fn runs_with_args_and_rejects_arity_mismatch() {
        // (lambda (a b) (+ a b)), lexical so params are on the stack.
        let mut f = ByteCodeFunction::new(LambdaParams {
            required: vec![
                crate::emacs_core::intern::SymId(1),
                crate::emacs_core::intern::SymId(2),
            ],
            optional: Vec::new(),
            rest: None,
        });
        f.lexical = true;
        f.ops = vec![Op::StackRef(1), Op::StackRef(1), Op::Add, Op::Return];
        f.max_stack = 16;
        // Correct arity -> native result.
        assert_eq!(
            try_run_compiled(
                std::ptr::null_mut(),
                &f,
                &[Value::make_int(40), Value::make_int(2)]
            )
            .unwrap(),
            Some(Value::make_int(42).bits())
        );
        // Wrong arity -> None (interpreter will signal wrong-number-of-arguments).
        assert_eq!(
            try_run_compiled(std::ptr::null_mut(), &f, &[Value::make_int(40)]).unwrap(),
            None
        );
    }
}
