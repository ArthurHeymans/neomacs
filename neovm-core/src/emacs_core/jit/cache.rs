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
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::compile::{CompiledLeaf, NativeRun, compile_bytecode_function_with, take_pending_flow};
use crate::emacs_core::bytecode::ByteCodeFunction;
use crate::emacs_core::error::Flow;
use crate::emacs_core::eval::Context;
use crate::emacs_core::intern::SymId;
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

    /// Precise inline-dependency REVERSE map: callee `SymId` -> the set of caller
    /// `compiled_id`s that INLINED it. Populated at compile-miss (the `or_insert_with`
    /// closures register `leaf.inline_deps()`); consulted by `evict_inline_dependents`
    /// when a function is redefined, to evict exactly the affected callers EARLY. The
    /// coarse `inline_epoch`-vs-live-epoch backstop in `try_run_compiled` remains the
    /// correctness floor regardless — this map is a pure churn-reduction optimization.
    /// Same thread/scope as COMPILED (its values are only meaningful as COMPILED keys).
    static INLINE_DEPS: RefCell<HashMap<SymId, HashSet<u64>>> = RefCell::new(HashMap::new());

    /// The tagged-heap identity the cached leaves were compiled against. The JIT
    /// cache is thread-local, but every leaf's reloc vector + baked addresses
    /// reference the heap live at compile time. If the thread's heap is replaced
    /// (a pdump load / in-process image reload / cache-replay test), the whole
    /// cache is stale — detected lazily by identity in `sync_cache_to_current_heap`
    /// and cleared before any stale reloc value is traced or run.
    static COMPILED_HEAP: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// Register a freshly-compiled leaf's inlined-callee deps into the reverse map.
/// Called ONLY from the cache compile-miss path (the `or_insert_with` closures), so
/// it runs once per compile, never on the hot dispatch path.
fn register_inline_deps(id: u64, leaf: &CompiledLeaf) {
    for &sym in leaf.inline_deps() {
        INLINE_DEPS.with(|m| m.borrow_mut().entry(sym).or_default().insert(id));
    }
}

/// Precise invalidation: function `sym` was just redefined — evict the JIT cache
/// entries of every caller that INLINED it, so each re-JITs against the new
/// definition on its next call. The coarse `inline_epoch`-vs-live-epoch backstop in
/// [`try_run_compiled`] ALSO catches them lazily; this removes the affected callers
/// EAGERLY while leaving unrelated callers cached (no per-redefinition re-JIT churn).
///
/// MUST be called OUTSIDE any `COMPILED`/`INLINE_DEPS` borrow (the redefinition path
/// in symbol.rs is) — it takes the two thread_local borrows itself, separately and
/// briefly. Idempotent: an absent/already-evicted id is a no-op. Compiled-id never
/// reuses, so a stale id in a dep set just removes nothing.
pub(crate) fn evict_inline_dependents(sym: SymId) {
    let Some(dependents) = INLINE_DEPS.with(|m| m.borrow_mut().remove(&sym)) else {
        return;
    };
    COMPILED.with(|cache| {
        let mut cache = cache.borrow_mut();
        for id in dependents {
            // Disjointness (spec-slot pointer safety): only INLINED-into caller
            // leaves are ever in a dep set, and resolve_compiled_leaf_ptr refuses to
            // cache an inlined leaf's pointer in a spec slot — so evicting one here
            // can never dangle a baked SpecSlot.leaf raw pointer.
            debug_assert!(
                !matches!(cache.get(&id), Some(CacheEntry::Compiled(l)) if l.inline_epoch().is_none()),
                "precise eviction must only touch inlined leaves (spec-slot pointer safety)"
            );
            cache.remove(&id);
        }
    });
}

/// Test-only: is a compiled leaf currently cached for `id` on this thread?
#[cfg(test)]
pub(crate) fn is_compiled_for_test(id: u64) -> bool {
    COMPILED.with(|c| matches!(c.borrow().get(&id), Some(CacheEntry::Compiled(_))))
}

/// Test-only: whether the cached leaf for `id` is AOT-backed (served from a
/// loaded `.so`, NOT JIT-compiled). Proves the AOT cache consult engaged.
#[cfg(test)]
pub(crate) fn cached_leaf_is_aot_for_test(id: u64) -> Option<bool> {
    COMPILED.with(|c| match c.borrow().get(&id) {
        Some(CacheEntry::Compiled(leaf)) => Some(leaf.is_aot_backed()),
        _ => None,
    })
}

/// Test-only: how many callers are recorded as inlining `sym`.
#[cfg(test)]
pub(crate) fn inline_dependent_count_for_test(sym: SymId) -> usize {
    INLINE_DEPS.with(|m| m.borrow().get(&sym).map_or(0, |s| s.len()))
}

/// Collect, as GC roots, the heap-object constants every currently-cached compiled
/// leaf loads through its reloc vector (R1a). Generated code holds NO heap-pointer
/// immediate — only an index into the leaf's `reloc_data` — so without this a
/// constant referenced solely by live native code could be swept. Walking COMPILED
/// keeps it precise: an evicted leaf drops out automatically (no stale roots).
/// Clear the cache if the thread's tagged heap was replaced since the cache was
/// built (a pdump load / in-process image reload / cache-replay test): the cached
/// leaves' reloc vectors + baked addresses point into the now-gone heap, so they
/// must neither be traced nor run. Detected by heap identity — one thread-local
/// load + compare on the common no-change path; clears only on an actual change.
fn sync_cache_to_current_heap() {
    let cur = crate::tagged::gc::current_tagged_heap_identity();
    let changed = COMPILED_HEAP.with(|h| {
        if h.get() != cur {
            h.set(cur);
            true
        } else {
            false
        }
    });
    if changed {
        clear();
    }
}

pub(crate) fn collect_jit_reloc_gc_roots(roots: &mut Vec<Value>) {
    sync_cache_to_current_heap();
    COMPILED.with(|c| {
        for entry in c.borrow().values() {
            if let CacheEntry::Compiled(leaf) = entry {
                roots.extend_from_slice(leaf.reloc_values());
            }
        }
    });
}

/// Drop all compiled state on this thread. Called when a pdump load replaces the
/// runtime image (and thus the heap that every cached leaf's reloc vector + baked
/// addresses reference) — so every cached leaf is now stale and must neither be run
/// nor GC-traced. No-op at the single startup load (the cache is empty then); it
/// matters when a process reloads an image in-place (e.g. the pdump round-trip
/// tests), where leaving stale leaves cached makes R1a's reloc roots trace
/// freed/reused memory.
pub(crate) fn clear() {
    COMPILED.with(|c| c.borrow_mut().clear());
    INLINE_DEPS.with(|m| m.borrow_mut().clear());
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
    func_value: Value,
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
        // SAFETY: the seam-provided Context is dormant for the whole native
        // dispatch (see neovm_jit_call's contract); a shared read of its obarray
        // for compile-time speculation. A null ctx (shim-free test bodies) just
        // disables speculation.
        let obarray = (!ctx.is_null()).then(|| unsafe { &(*ctx).obarray });
        // Re-JIT a STALE INLINED leaf: if it inlined a callee and the obarray's
        // function_epoch has since moved, a callee it inlined may have been
        // redefined — drop the entry so it recompiles below (no stale inline runs).
        let stale = matches!(
            cache.get(&id),
            Some(CacheEntry::Compiled(l))
                if l.inline_epoch().is_some()
                    && l.inline_epoch() != obarray.map(|ob| ob.function_epoch())
        );
        if stale {
            cache.remove(&id);
        }
        match cache.entry(id).or_insert_with(|| {
            // R1c-6: consult AOT FIRST (additive — a miss/error falls through to
            // the JIT below, leaving JIT behavior unchanged). An AOT hit is a
            // PRE-WARMED leaf: native code already on disk, no JIT compile. Only
            // the required-only subset the AOT emitter supports is eligible
            // (no &optional/&rest — matches the MIR pure path's arity seeding).
            if super::aot::aot_enabled()
                && func.params.optional.is_empty()
                && func.params.rest.is_none()
            {
                let native_arity = func.params.required.len();
                if let Some(leaf) =
                    super::aot::try_load_leaf(&func.ops, &func.constants, native_arity)
                {
                    // AOT leaves never inline → no inline deps to register. Their
                    // reloc consts are rooted via the COMPILED walk (R1c-8).
                    return CacheEntry::Compiled(Rc::new(leaf));
                }
            }
            match compile_bytecode_function_with(func, obarray) {
                Ok(leaf) => {
                    // Compile-only (this closure runs solely on a cache miss): record
                    // the precise inline deps so a later redefinition evicts this leaf.
                    register_inline_deps(id, &leaf);
                    CacheEntry::Compiled(Rc::new(leaf))
                }
                Err(_) => CacheEntry::NotCompilable,
            }
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
        Some(leaf) => run_resolved_leaf(ctx, func, func_value, &leaf, args),
    }
}

/// A stable raw pointer to the compiled leaf for `func` (compiling it on first
/// use), or `None` if the body is `NotCompilable`. The pointer stays valid for
/// the thread's lifetime: the per-thread `COMPILED` cache never evicts, so the
/// owning `Rc<CompiledLeaf>` (and the heap box it points at) outlive every use.
/// Used by the V3 speculated-call fast path to cache a callee leaf handle in a
/// spec slot, skipping the cache hash lookup on subsequent calls.
pub(crate) fn resolve_compiled_leaf_ptr(
    ctx: *mut Context,
    func: &ByteCodeFunction,
) -> Option<*const CompiledLeaf> {
    let id = func.runtime.compiled_id_or_assign();
    if id > max_compiled_id() {
        return None;
    }
    COMPILED.with(|cache| {
        let mut cache = cache.borrow_mut();
        match cache.entry(id).or_insert_with(|| {
            // SAFETY: same dormant-Context contract as try_run_compiled.
            let obarray = (!ctx.is_null()).then(|| unsafe { &(*ctx).obarray });
            match compile_bytecode_function_with(func, obarray) {
                Ok(leaf) => {
                    // Compile-only (this closure runs solely on a cache miss): record
                    // the precise inline deps so a later redefinition evicts this leaf.
                    register_inline_deps(id, &leaf);
                    CacheEntry::Compiled(Rc::new(leaf))
                }
                Err(_) => CacheEntry::NotCompilable,
            }
        }) {
            // INLINED leaves must NOT be fast-path-cached in a spec slot: their
            // validity depends on an inlined callee's epoch, which the caller's
            // spec guard doesn't check. Force them through try_run_compiled (which
            // re-JITs on a stale epoch). Non-inlined leaves keep the stable-pointer
            // fast path (they are never epoch-stale, so the cache never evicts them).
            CacheEntry::Compiled(leaf) if leaf.inline_epoch().is_none() => Some(Rc::as_ptr(leaf)),
            _ => None,
        }
    })
}

/// Run an already-resolved `leaf` (the caller validated arity) with the full
/// `NativeRun` outcome handling — including precise-deopt resume via
/// `run_resumed_frame`. Shared by `try_run_compiled` and the V3 fast path so
/// both have byte-identical deopt/signal semantics. Same return shape as
/// `try_run_compiled`: `Ok(Some(bits))` success, `Ok(None)` fall-back, `Err`
/// on a non-local flow.
pub(crate) fn run_resolved_leaf(
    ctx: *mut Context,
    func: &ByteCodeFunction,
    func_value: Value,
    leaf: &CompiledLeaf,
    args: &[Value],
) -> Result<Option<usize>, Flow> {
    finish_native_run(ctx, func, func_value, leaf.call(ctx as *mut u8, args))
}

/// Native-to-native variant of [`run_resolved_leaf`]: `args_ptr` addresses
/// exactly `leaf.arity` pre-marshaled argument words (the caller's native
/// call-args slot), and the leaf is a pure pass-through (no nil-pad / rest).
/// Skips the `LispArgVec` build and the `arg_bits` re-marshal entirely — the
/// per-call cost the call-heavy benchmark is dominated by.
///
/// SAFETY: see [`CompiledLeaf::call_premarshaled`] — `args_ptr` must address
/// `leaf.arity` live words with no GC safepoint before the native entry reads
/// them (the spec fast path's `maybe_quit`-returned-Ok window).
pub(crate) fn run_resolved_leaf_native(
    ctx: *mut Context,
    func: &ByteCodeFunction,
    func_value: Value,
    leaf: &CompiledLeaf,
    args_ptr: *const i64,
) -> Result<Option<usize>, Flow> {
    finish_native_run(
        ctx,
        func,
        func_value,
        leaf.call_premarshaled(ctx as *mut u8, args_ptr),
    )
}

/// Map a [`NativeRun`] outcome to the `try_run_compiled` return shape, resuming
/// the interpreter mid-frame on a precise deopt. Shared by both resolved-leaf
/// runners so marshaled and native-to-native calls have identical semantics.
fn finish_native_run(
    ctx: *mut Context,
    func: &ByteCodeFunction,
    func_value: Value,
    outcome: NativeRun,
) -> Result<Option<usize>, Flow> {
    match outcome {
        NativeRun::Ok(bits) => Ok(Some(bits)),
        NativeRun::Deopt => Ok(None),
        NativeRun::DeoptAt {
            pc,
            stack,
            handlers,
            binds,
            spec_base,
            cond_base,
        } => {
            if ctx.is_null() {
                // call() maps null-vmctx deopts to Deopt; defensive only.
                return Ok(None);
            }
            // Precise deopt: resume the Tier-0 interpreter mid-function with
            // the live stack and the (still registered) frame state.
            // SAFETY: the seam-provided &mut Context is dormant during the
            // native call — the same contract every runtime shim uses.
            let ctx = unsafe { &mut *ctx };
            let mut vm = crate::emacs_core::bytecode::Vm::from_context(ctx);
            vm.run_resumed_frame(
                func, func_value, pc, &stack, handlers, &binds, spec_base, cond_base,
            )
            .map(|v| Some(v.bits()))
        }
        NativeRun::Signal => {
            Err(take_pending_flow()
                .expect("STATUS_SIGNAL from compiled code implies a stashed Flow"))
        }
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
            try_run_compiled(std::ptr::null_mut(), &f, Value::NIL, &[]).unwrap(),
            Some(c.bits())
        );
        // Second call hits the cache; same result.
        assert_eq!(
            try_run_compiled(std::ptr::null_mut(), &f, Value::NIL, &[]).unwrap(),
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
            try_run_compiled(std::ptr::null_mut(), &f, Value::NIL, &[]).unwrap(),
            None
        );
        assert_eq!(
            try_run_compiled(std::ptr::null_mut(), &f, Value::NIL, &[]).unwrap(),
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
            try_run_compiled(std::ptr::null_mut(), &f, Value::NIL, &[]).unwrap(),
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
                Value::NIL,
                &[Value::make_int(40), Value::make_int(2)]
            )
            .unwrap(),
            Some(Value::make_int(42).bits())
        );
        // Wrong arity -> None (interpreter will signal wrong-number-of-arguments).
        assert_eq!(
            try_run_compiled(std::ptr::null_mut(), &f, Value::NIL, &[Value::make_int(40)]).unwrap(),
            None
        );
    }
}
