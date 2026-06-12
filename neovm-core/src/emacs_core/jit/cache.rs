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

use super::compile::{CompiledLeaf, compile_bytecode_function};
use crate::emacs_core::bytecode::ByteCodeFunction;
use crate::emacs_core::value::Value;

/// One thread's knowledge of a function's compiled state.
enum CacheEntry {
    /// Native code, ready to run.
    Compiled(CompiledLeaf),
    /// The body is outside the baseline JIT's supported subset; never retried.
    NotCompilable,
}

thread_local! {
    /// `compiled_id` -> compiled state, owned by and private to this thread.
    static COMPILED: RefCell<HashMap<u64, CacheEntry>> = RefCell::new(HashMap::new());
}

/// Tier-up entry point: run `func`'s body as native code if possible.
///
/// Returns `Some(bits)` with the result's raw tagged [`crate::emacs_core::value::Value`]
/// bits when native code produced a result; returns `None` when the caller must
/// fall back to the interpreter — either because the body is not compilable by
/// this tier, or because compiled code **deoptimized** (a speculation guard
/// failed).
///
/// Compiles on first use (per thread) and caches the outcome, so a
/// non-compilable body is only attempted once. Native code runs only when
/// `args.len()` matches the compiled function's arity; a mismatch returns `None`
/// so the interpreter signals the wrong-argument-count error (matching GNU).
pub fn try_run_compiled(func: &ByteCodeFunction, args: &[Value]) -> Option<usize> {
    let id = func.runtime.compiled_id_or_assign();
    COMPILED.with(|cache| {
        let mut cache = cache.borrow_mut();
        let entry = cache
            .entry(id)
            .or_insert_with(|| match compile_bytecode_function(func) {
                Ok(leaf) => CacheEntry::Compiled(leaf),
                Err(_) => CacheEntry::NotCompilable,
            });
        match entry {
            // Only run native for a valid call (matching arity); a mismatch is a
            // wrong-arg-count call the interpreter must signal.
            CacheEntry::Compiled(leaf) if args.len() == leaf.arity() => leaf.call(args),
            _ => None,
        }
    })
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
        assert_eq!(try_run_compiled(&f, &[]), Some(c.bits()));
        // Second call hits the cache; same result.
        assert_eq!(try_run_compiled(&f, &[]), Some(c.bits()));
    }

    #[test]
    fn returns_none_for_noncompilable_body() {
        // Mul is unsupported -> NotCompilable -> None (interpreter fallback).
        let f = nullary_fn(
            vec![Op::Constant(0), Op::Constant(0), Op::Mul, Op::Return],
            vec![Value::make_int(2)],
        );
        assert_eq!(try_run_compiled(&f, &[]), None);
        assert_eq!(try_run_compiled(&f, &[]), None);
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
        assert_eq!(try_run_compiled(&f, &[]), None);
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
            try_run_compiled(&f, &[Value::make_int(40), Value::make_int(2)]),
            Some(Value::make_int(42).bits())
        );
        // Wrong arity -> None (interpreter will signal wrong-number-of-arguments).
        assert_eq!(try_run_compiled(&f, &[Value::make_int(40)]), None);
    }
}
