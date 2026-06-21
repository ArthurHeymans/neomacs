//! Baseline bytecode → native lowering.
//!
//! Real compilation of neovm-core bytecode to machine code, grown as a series of
//! always-correct vertical slices. It compiles **leaf** functions with 0+
//! required (stack) arguments and arbitrary intra-function control flow: the
//! operand-stack ops, branches `{Goto, GotoIf*}`, fixnum arithmetic
//! `{+, -, *, 1+, 1-, neg}` and comparisons `{=, <, >, <=, >=}`, the
//! non-allocating type predicates `{null, not, consp, stringp, listp}`,
//! `car`/`cdr`, and the allocating `cons`. It **bails to the interpreter**
//! (returns [`CompileError`]) on anything else — `&optional`/`&rest`/dynamic
//! params, dynamic-binding bytecode, variables, function `Call`/`Apply`,
//! `switch`, `eq`/`symbolp`, `Div`/`Rem`, and non-fixnum arithmetic.
//!
//! Control flow builds a CLIF basic-block CFG (`analyze_cfg` + `lower_leaf`); the
//! operand stack flows across edges through per-slot SSA variables, so Cranelift
//! inserts the phi nodes and branches carry no explicit block arguments.
//!
//! ## Speculation + deopt
//!
//! The arithmetic ops are *speculative*: native code assumes the operands are
//! fixnums and the result stays in fixnum range — exactly the interpreter's
//! fast path (`vm.rs` `Op::Add`). Each assumption is a **guard**; if a guard
//! fails at run time the function **deoptimizes**: it returns a 0 flag and the
//! caller re-runs the body on the Tier-0 interpreter, which handles the slow
//! cases (non-numbers signal, out-of-range promotes to a bignum). Because every
//! op in the supported subset is pure (no heap writes, no calls, no side
//! effects), re-running from the start after a deopt is always correct.
//!
//! ABI: `extern "C" fn(args: *const i64, out: *mut i64) -> i64`. Reads the
//! function's fixed arguments from `args` (seeding the operand stack), returns 1
//! and writes the result's raw tagged bits through `out` on success; returns 0
//! (deopt) otherwise, leaving `out` untouched.
//!
//! Allocation (`cons`) calls a C-ABI runtime shim. Because that may trigger GC,
//! live `Value`s held across it are kept alive by pushing them onto the
//! GC-traced scratch-root stack (see the `neovm_jit_*` shims); the GC is
//! non-moving, so the JIT's SSA registers stay valid afterward without a reload.
//! No vmctx is needed yet (`cons` uses the thread-local heap directly); that
//! arrives with `Call`/`Apply`.
//!
//! The bytecode operand stack is modelled at *compile time* as a `Vec` of
//! Cranelift SSA values (abstract interpretation). A `Value` is opaque to native
//! code: it flows as its `usize` bit pattern (`i64` in CLIF), exactly as the
//! interpreter stores it.

use cranelift_codegen::ir::Value as ClifValue;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    AbiParam, Block, BlockArg, FuncRef, Function, InstBuilder, MemFlags, Signature, StackSlot,
    StackSlotData, StackSlotKind, Type, UserFuncName, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use smallvec::SmallVec;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use super::backend::BackendError;
use super::mir;
use crate::emacs_core::bytecode::chunk::GnuByteOffsetMapEntry;
use crate::emacs_core::bytecode::opcode::Op;
use crate::emacs_core::bytecode::vm::condition_frame_resume;
use crate::emacs_core::bytecode::{ByteCodeFunction, Vm};
use crate::emacs_core::error::{Flow, make_signal_binding_value, signal};
use crate::emacs_core::eval::{
    ConditionFrame, Context, LispArgVec, ResumeTarget, push_scratch_gc_root,
    restore_scratch_gc_roots, save_scratch_gc_roots,
};
use crate::emacs_core::intern::{SymId, intern};
use crate::emacs_core::symbol::Obarray;
use crate::emacs_core::value::{Value, ValueKind};
use crate::tagged::header::ConsCell;
use crate::tagged::value::{
    FIXNUM_CHECK_MASK, FIXNUM_CHECK_VALUE, FIXNUM_SHIFT, TAG_CONS, TAG_MASK, TAG_STRING, TAG_SYMBOL,
};

// ---------------------------------------------------------------------------
// Runtime shims — C-ABI functions the JIT calls for operations that allocate
// (and so may trigger GC). Live `Value`s held across such a call are kept alive
// by pushing them onto the GC-traced scratch-root stack; the GC is non-moving,
// so the JIT's SSA registers stay valid afterward (no reload). These are the
// foundation the eventual `Call`/`Apply` reuse.
// ---------------------------------------------------------------------------

/// Snapshot the scratch-root depth so it can be restored after a rooted region.
extern "C" fn neovm_jit_gc_save() -> i64 {
    save_scratch_gc_roots() as i64
}

/// Root one live `Value` (by its raw bits) across an upcoming allocation.
///
/// Only heap objects (cons/string/float/veclike incl. bignum) can be collected
/// and need stack rooting; immediates (fixnums, chars, nil/t) are never on the
/// heap, and symbols are kept live by the obarray (always a GC root), not by the
/// operand stack. Skipping those here is correct — `mark_value` would no-op on
/// them anyway — and avoids the thread-local push for the many symbol/fixnum
/// operands the JIT roots before calls. `gc_restore` truncates to the saved
/// depth, so a variable push count is fine.
extern "C" fn neovm_jit_gc_push(bits: i64) {
    let v = Value::from_bits(bits as usize);
    if v.is_heap_object() {
        push_scratch_gc_root(v);
    }
}

/// Pop the scratch roots back to a saved depth.
extern "C" fn neovm_jit_gc_restore(saved: i64) {
    restore_scratch_gc_roots(saved as usize);
}

/// Allocate `(cons car cdr)`. Roots car+cdr across the allocation itself; the
/// caller roots any *other* live values first.
extern "C" fn neovm_jit_cons(car: i64, cdr: i64) -> i64 {
    let car = Value::from_bits(car as usize);
    let cdr = Value::from_bits(cdr as usize);
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(car);
    push_scratch_gc_root(cdr);
    let result = Value::cons(car, cdr).bits() as i64;
    restore_scratch_gc_roots(saved);
    result
}

std::thread_local! {
    /// The non-local `Flow` (signal/throw/...) raised inside a runtime call made
    /// by JIT code. The call shim stashes it and returns [`STATUS_SIGNAL`]; the
    /// nearest Rust caller of the compiled function takes it and re-raises.
    /// Thread-local because compiled code and its dispatch run on one thread.
    static PENDING_FLOW: std::cell::RefCell<Option<Flow>> = const { std::cell::RefCell::new(None) };
}

/// Native return code: success, result bits written through `out`.
pub const STATUS_OK: i64 = 1;
/// Native return code: a speculation guard failed before any side effect ran —
/// rerun the body on the Tier-0 interpreter.
pub const STATUS_DEOPT: i64 = 0;
/// Native return code: a runtime call raised a non-local `Flow`; take it with
/// [`take_pending_flow`] and propagate.
pub const STATUS_SIGNAL: i64 = 2;

/// Native return code: a speculation guard failed at a PRECISE bytecode pc —
/// the live operand stack was spilled into the leaf's deopt buffer and the
/// frame's binds/handlers were left REGISTERED (no frame unwind): the caller
/// resumes the Tier-0 interpreter mid-function via `Vm::run_resumed_frame`.
/// Unlike [`STATUS_DEOPT`], this is sound even after side effects ran.
pub const STATUS_DEOPT_AT: i64 = 3;

/// Debug-build counter of speculated direct-call shim entries (test evidence
/// that `find_spec_sites` + the spec lowering actually engage).
#[cfg(debug_assertions)]
pub(crate) static SPEC_CALL_COUNT: AtomicU64 = AtomicU64::new(0);

/// Debug-build counter of V3 fast-path engagements: a speculated call that
/// ran the cached callee leaf DIRECTLY (skipping funcall dispatch + the cache
/// hash lookup), as opposed to falling back to `call_for_jit`. Test evidence
/// that the fast path actually fires instead of silently no-op'ing.
#[cfg(debug_assertions)]
pub(crate) static SPEC_FAST_CALL_COUNT: AtomicU64 = AtomicU64::new(0);

/// Take the `Flow` stashed by a shim that returned [`STATUS_SIGNAL`].
pub fn take_pending_flow() -> Option<Flow> {
    PENDING_FLOW.with(|p| p.borrow_mut().take())
}

fn stash_pending_flow(flow: Flow) {
    PENDING_FLOW.with(|p| *p.borrow_mut() = Some(flow));
}

/// Call a function from JIT code with the interpreter's `Op::Call` semantics
/// (quit poll, writeback, depth guard — see `Vm::call_for_jit`). Reads `nargs`
/// argument words from `args_ptr`; on success writes the result bits through
/// `out` and returns [`STATUS_OK`]; on a non-local exit stashes the `Flow` and
/// returns [`STATUS_SIGNAL`].
///
/// SAFETY contract with the generated code and the dispatch seam:
/// - `ctx` is the `*mut Context` the seam passed into this invocation of the
///   compiled function. The seam's `&mut Context` is dormant for the entire
///   native call (it is not touched until the compiled function returns), the
///   elisp mutator is single-threaded, and the pointer round-trips through
///   native code — so reconstructing `&mut Context` here does not create a
///   *used* aliasing `&mut`.
/// - `args_ptr` points at `nargs` valid argument words (a JIT stack slot).
/// - The generated code rooted every *other* live `Value` of its frame before
///   this call; the callee + args are rooted here, so a GC inside the callee
///   traces everything that survives the call.
extern "C" fn neovm_jit_call(
    ctx: *mut u8,
    func_bits: i64,
    args_ptr: *const i64,
    nargs: i64,
    out: *mut i64,
) -> i64 {
    let func_val = Value::from_bits(func_bits as usize);
    let nargs = nargs as usize;
    let saved = save_scratch_gc_roots();
    // The callee is not on bc_buf, so it needs an explicit scratch root across
    // the call (which may GC); the arguments go straight onto the GC-traced
    // bc_buf below, so they are rooted there — no LispArgVec, no per-arg root.
    push_scratch_gc_root(func_val);
    // SAFETY: see the function-level contract — seam-provided, dormant, single
    // mutator thread.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let status = match ctx.maybe_quit() {
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
        Ok(()) => {
            // Push the native call-args slot straight onto bc_buf (GC-traced,
            // so the args are rooted across the call); the fast subr path reads
            // them in place. Truncate back afterwards.
            let args_start = ctx.bc_buf.len();
            for i in 0..nargs {
                // SAFETY: the generated code stored exactly `nargs` argument
                // words at `args_ptr` (its call-args slot) immediately before
                // this call.
                let v = Value::from_bits(unsafe { *args_ptr.add(i) } as usize);
                ctx.bc_buf.push(v);
            }
            let mut vm = Vm::from_context(ctx);
            let res = vm.call_for_jit_stack(func_val, args_start, nargs);
            vm.bc_buf_truncate(args_start);
            match res {
                Ok(value) => {
                    // SAFETY: `out` is the generated code's result stack slot.
                    unsafe { *out = value.bits() as i64 };
                    STATUS_OK
                }
                Err(flow) => {
                    stash_pending_flow(flow);
                    STATUS_SIGNAL
                }
            }
        }
    };
    restore_scratch_gc_roots(saved);
    status
}

/// `apply` a function from JIT code with the interpreter's `Op::Apply`
/// semantics (quit poll first, last argument spread as a list, writeback, NO
/// nesting-depth guard — see `Vm::apply_for_jit`). Same SAFETY contract as
/// [`neovm_jit_call`].
extern "C" fn neovm_jit_apply(
    ctx: *mut u8,
    func_bits: i64,
    args_ptr: *const i64,
    nargs: i64,
    out: *mut i64,
) -> i64 {
    let func_val = Value::from_bits(func_bits as usize);
    let nargs = nargs as usize;
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(func_val);
    let mut args = LispArgVec::new();
    for i in 0..nargs {
        // SAFETY: the generated code stored exactly `nargs` argument words at
        // `args_ptr` (its call-args stack slot) immediately before this call.
        let v = Value::from_bits(unsafe { *args_ptr.add(i) } as usize);
        push_scratch_gc_root(v);
        args.push(v);
    }
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let status = match ctx.maybe_quit() {
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
        Ok(()) => {
            let mut vm = Vm::from_context(ctx);
            match vm.apply_for_jit(func_val, args) {
                Ok(value) => {
                    // SAFETY: `out` is the generated code's result stack slot.
                    unsafe { *out = value.bits() as i64 };
                    STATUS_OK
                }
                Err(flow) => {
                    stash_pending_flow(flow);
                    STATUS_SIGNAL
                }
            }
        }
    };
    restore_scratch_gc_roots(saved);
    status
}

/// Slow path for `eq` when the raw bits differ: only `symbols-with-pos` can
/// still make two differing values `eq`. Read-only on the Context; never
/// allocates, GCs, or signals — a plain value-returning helper.
///
/// SAFETY: same vmctx contract as [`neovm_jit_call`], but only a shared read.
extern "C" fn neovm_jit_eq_slow(ctx: *mut u8, a: i64, b: i64) -> i64 {
    let a = Value::from_bits(a as usize);
    let b = Value::from_bits(b as usize);
    // SAFETY: seam-provided dormant Context; read-only access.
    let ctx = unsafe { &*(ctx as *const Context) };
    let eq = ctx.symbols_with_pos_enabled && crate::emacs_core::value::eq_value_swp(&a, &b, true);
    (if eq {
        Value::T.bits()
    } else {
        Value::NIL.bits()
    }) as i64
}

/// Slow path for `symbolp` when the value's tag is not Symbol: only a
/// symbol-with-pos (a veclike) can still count, and only while
/// `symbols-with-pos-enabled`. Read-only; never allocates, GCs, or signals.
///
/// SAFETY: same read-only vmctx contract as [`neovm_jit_eq_slow`].
extern "C" fn neovm_jit_symbolp_slow(ctx: *mut u8, v: i64) -> i64 {
    let v = Value::from_bits(v as usize);
    // SAFETY: seam-provided dormant Context; read-only access.
    let ctx = unsafe { &*(ctx as *const Context) };
    let is_sym = ctx.symbols_with_pos_enabled && v.is_symbol_with_pos();
    (if is_sym {
        Value::T.bits()
    } else {
        Value::NIL.bits()
    }) as i64
}

/// Read a variable from JIT code (`Op::VarRef` semantics via
/// `Vm::varref_for_jit`). Writes the value through `out` and returns
/// [`STATUS_OK`], or stashes the `Flow` (e.g. `void-variable`) and returns
/// [`STATUS_SIGNAL`]. SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_varref(ctx: *mut u8, sym: i64, out: *mut i64) -> i64 {
    use crate::emacs_core::intern::SymId;
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let mut vm = Vm::from_context(ctx);
    match vm.varref_for_jit(SymId(sym as u32)) {
        Ok(value) => {
            // SAFETY: `out` is the generated code's result stack slot.
            unsafe { *out = value.bits() as i64 };
            STATUS_OK
        }
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
    }
}

/// Assign a variable from JIT code (`Op::VarSet` semantics via
/// `Vm::varset_for_jit`; may run variable watchers — arbitrary lisp). Roots the
/// value across the assignment. SAFETY: same vmctx contract as
/// [`neovm_jit_call`].
extern "C" fn neovm_jit_varset(ctx: *mut u8, sym: i64, val: i64) -> i64 {
    use crate::emacs_core::intern::SymId;
    let value = Value::from_bits(val as usize);
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(value);
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let mut vm = Vm::from_context(ctx);
    let status = match vm.varset_for_jit(SymId(sym as u32), value) {
        Ok(()) => STATUS_OK,
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
    };
    restore_scratch_gc_roots(saved);
    status
}

std::thread_local! {
    /// Per-thread analogue of the interpreter's per-frame `bind_stack`: the
    /// specpdl depth recorded before each JIT-made `varbind`, consumed by the
    /// `unbind` shim. [`CompiledLeaf::call`] truncates a frame's segment on
    /// every exit (the `cleanup_bytecode_frame` parity unwind).
    static JIT_BIND_STACK: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Dynamically bind a variable (`Op::VarBind` semantics: GNU `Bvarbind`,
/// `specbind(sym, POP)` — infallible, like the interpreter arm). Records the
/// pre-bind specpdl depth for the matching `unbind`.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_varbind(ctx: *mut u8, sym: i64, val: i64) {
    use crate::emacs_core::intern::SymId;
    let value = Value::from_bits(val as usize);
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    JIT_BIND_STACK.with(|s| s.borrow_mut().push(ctx.specpdl.len()));
    ctx.specbind(SymId(sym as u32), value);
}

/// Unbind the `n` most recent JIT-made dynamic bindings (`Op::Unbind`
/// semantics). The static bind-depth analysis guarantees `n` never exceeds this
/// frame's outstanding binds; the `min` is defensive only.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_unbind(ctx: *mut u8, n: i64) {
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let target = JIT_BIND_STACK.with(|s| {
        let mut s = s.borrow_mut();
        let take = (n as usize).min(s.len());
        if take == 0 {
            return None;
        }
        let target = s[s.len() - take];
        let new_len = s.len() - take;
        s.truncate(new_len);
        Some(target)
    });
    if let Some(target) = target {
        ctx.unbind_to(target);
    }
}

// ---------------------------------------------------------------------------
// Direct-builtin tables: the SAME typed `builtins::builtin_*` functions the
// interpreter opcode arms call, exposed to generated code through three
// arity-shaped generic shims. Single source of truth — the JIT cannot drift
// from the interpreter's semantics for these ops.
// ---------------------------------------------------------------------------

type JitBuiltin1 = fn(&mut Context, Value) -> Result<Value, Flow>;
type JitBuiltin2 = fn(&mut Context, Value, Value) -> Result<Value, Flow>;
type JitBuiltin3 = fn(&mut Context, Value, Value, Value) -> Result<Value, Flow>;

use crate::emacs_core::builtins as b;

static JIT_BUILTIN1: [JitBuiltin1; 4] = [
    b::builtin_length_1,          // 0
    b::builtin_symbol_value_1,    // 1
    b::builtin_symbol_function_1, // 2
    b::builtin_nreverse_1,        // 3
];

static JIT_BUILTIN2: [JitBuiltin2; 15] = [
    b::builtin_nth_2,          // 0
    b::builtin_nthcdr_2,       // 1
    b::builtin_elt_2,          // 2
    b::builtin_member_2,       // 3
    b::builtin_memq_2,         // 4
    b::builtin_assq_2,         // 5
    b::builtin_equal_2,        // 6
    b::builtin_setcar_2,       // 7
    b::builtin_setcdr_2,       // 8
    b::builtin_aref_2,         // 9
    b::builtin_set_2,          // 10
    b::builtin_fset_2,         // 11
    b::builtin_get_2,          // 12
    b::builtin_string_equal_2, // 13
    b::builtin_string_lessp_2, // 14
];

static JIT_BUILTIN3: [JitBuiltin3; 1] = [
    b::builtin_put_3, // 0
];

/// Slice-shaped builtins (`fn(&[Value]) -> EvalResult`, no Context) — the
/// exact functions the interpreter's `Nconc`/`Concat`/`Substring` arms call.
type JitBuiltinSlice = fn(&[Value]) -> Result<Value, Flow>;

static JIT_BUILTIN_SLICE: [JitBuiltinSlice; 3] = [
    b::builtin_nconc_slice_values, // 0
    b::builtin_concat_slice,       // 1
    b::builtin_substring_slice,    // 2
];

/// `(nargs, table_index)` for ops lowered through the slice-builtin shim.
/// `Concat`'s arity rides in the opcode; `Nconc`/`Substring` are fixed.
fn slice_builtin_spec(op: &Op) -> Option<(usize, usize)> {
    Some(match op {
        Op::Nconc => (2, 0),
        Op::Concat(n) => (*n as usize, 1),
        Op::Substring => (3, 2),
        _ => return None,
    })
}

/// `(table_arity, table_index)` for ops lowered through the generic
/// direct-builtin shims. (There is no longer a per-op "mutates" flag: every op
/// that needs runtime re-entry already sets `needs_rt`, and these ops always
/// route through the precise-deopt path, so there is nothing to poison.)
fn direct_builtin_spec(op: &Op) -> Option<(u8, usize)> {
    Some(match op {
        Op::Length => (1, 0),
        Op::SymbolValue => (1, 1),
        Op::SymbolFunction => (1, 2),
        Op::Nreverse => (1, 3),
        Op::Nth => (2, 0),
        Op::Nthcdr => (2, 1),
        Op::Elt => (2, 2),
        Op::Member => (2, 3),
        Op::Memq => (2, 4),
        Op::Assq => (2, 5),
        Op::Equal => (2, 6),
        Op::Setcar => (2, 7),
        Op::Setcdr => (2, 8),
        Op::Aref => (2, 9),
        Op::Set => (2, 10),
        Op::Fset => (2, 11),
        Op::Get => (2, 12),
        Op::StringEqual => (2, 13),
        Op::StringLessp => (2, 14),
        Op::Put => (3, 0),
        _ => return None,
    })
}

/// Call a unary direct builtin (`JIT_BUILTIN1[idx]`) — the identical function
/// the interpreter arm calls. Roots the argument across the call (builtins may
/// GC); the generated code rooted the rest of its frame.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_builtin1(ctx: *mut u8, idx: i64, a: i64, out: *mut i64) -> i64 {
    let a = Value::from_bits(a as usize);
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(a);
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let status = match JIT_BUILTIN1[idx as usize](ctx, a) {
        Ok(value) => {
            // SAFETY: `out` is the generated code's result stack slot.
            unsafe { *out = value.bits() as i64 };
            STATUS_OK
        }
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
    };
    restore_scratch_gc_roots(saved);
    status
}

/// Binary variant of [`neovm_jit_builtin1`].
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_builtin2(ctx: *mut u8, idx: i64, a: i64, b: i64, out: *mut i64) -> i64 {
    let a = Value::from_bits(a as usize);
    let b = Value::from_bits(b as usize);
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(a);
    push_scratch_gc_root(b);
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let status = match JIT_BUILTIN2[idx as usize](ctx, a, b) {
        Ok(value) => {
            // SAFETY: `out` is the generated code's result stack slot.
            unsafe { *out = value.bits() as i64 };
            STATUS_OK
        }
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
    };
    restore_scratch_gc_roots(saved);
    status
}

/// Ternary variant of [`neovm_jit_builtin1`].
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_builtin3(
    ctx: *mut u8,
    idx: i64,
    a: i64,
    b: i64,
    c: i64,
    out: *mut i64,
) -> i64 {
    let a = Value::from_bits(a as usize);
    let b = Value::from_bits(b as usize);
    let c = Value::from_bits(c as usize);
    let saved = save_scratch_gc_roots();
    push_scratch_gc_root(a);
    push_scratch_gc_root(b);
    push_scratch_gc_root(c);
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let status = match JIT_BUILTIN3[idx as usize](ctx, a, b, c) {
        Ok(value) => {
            // SAFETY: `out` is the generated code's result stack slot.
            unsafe { *out = value.bits() as i64 };
            STATUS_OK
        }
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
    };
    restore_scratch_gc_roots(saved);
    status
}

/// `Op::List`: build a list from `n` operand words (the interpreter's
/// `Value::list_from_slice` on the live stack slice). The values are rooted
/// here across the per-cell allocations; the generated code rooted the rest of
/// its frame. Infallible, context-free.
extern "C" fn neovm_jit_list(args_ptr: *const i64, nargs: i64) -> i64 {
    let nargs = nargs as usize;
    let saved = save_scratch_gc_roots();
    let mut args: SmallVec<[Value; 8]> = SmallVec::with_capacity(nargs);
    for i in 0..nargs {
        // SAFETY: the generated code stored exactly `nargs` words at
        // `args_ptr` (its call-args stack slot) immediately before this call.
        let v = Value::from_bits(unsafe { *args_ptr.add(i) } as usize);
        push_scratch_gc_root(v);
        args.push(v);
    }
    let result = Value::list_from_slice(&args).bits() as i64;
    restore_scratch_gc_roots(saved);
    result
}

/// Call a slice-shaped direct builtin (`JIT_BUILTIN_SLICE[idx]`) — the
/// identical function the interpreter arm calls (`nconc`/`concat`/
/// `substring`). Roots the operands across the call (they may allocate);
/// context-free like the interpreter's slice calls.
extern "C" fn neovm_jit_builtin_slice(
    idx: i64,
    args_ptr: *const i64,
    nargs: i64,
    out: *mut i64,
) -> i64 {
    let nargs = nargs as usize;
    let saved = save_scratch_gc_roots();
    let mut args: SmallVec<[Value; 8]> = SmallVec::with_capacity(nargs);
    for i in 0..nargs {
        // SAFETY: see neovm_jit_list — the same spill-slot contract.
        let v = Value::from_bits(unsafe { *args_ptr.add(i) } as usize);
        push_scratch_gc_root(v);
        args.push(v);
    }
    let status = match JIT_BUILTIN_SLICE[idx as usize](&args) {
        Ok(value) => {
            // SAFETY: `out` is the generated code's result stack slot.
            unsafe { *out = value.bits() as i64 };
            STATUS_OK
        }
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
    };
    restore_scratch_gc_roots(saved);
    status
}

/// Named-builtin dispatch for `Op::CallBuiltin`/`Op::CallBuiltinSym`/
/// `Op::Aset` — re-enters the runtime through the dedicated `Vm::*_for_jit`
/// helpers, which mirror the interpreter arms exactly (override-aware named
/// dispatch for CallBuiltin/Aset, advice-bypassing direct dispatch for
/// CallBuiltinSym, mutating-first-arg string writeback, trailing quit poll).
/// `variant`: 0 = CallBuiltin, 1 = CallBuiltinSym, 2 = Aset.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_named_builtin(
    ctx: *mut u8,
    variant: i64,
    sym: i64,
    args_ptr: *const i64,
    nargs: i64,
    out: *mut i64,
) -> i64 {
    let nargs = nargs as usize;
    let saved = save_scratch_gc_roots();
    let mut args = LispArgVec::new();
    for i in 0..nargs {
        // SAFETY: the generated code stored exactly `nargs` words at
        // `args_ptr` (its call-args stack slot) immediately before this call.
        let v = Value::from_bits(unsafe { *args_ptr.add(i) } as usize);
        push_scratch_gc_root(v);
        args.push(v);
    }
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let mut vm = Vm::from_context(ctx);
    let result = match variant {
        0 => vm.callbuiltin_for_jit(SymId(sym as u32), args),
        1 => vm.callbuiltinsym_for_jit(SymId(sym as u32), args),
        _ => vm.aset_for_jit(args[0], args[1], args[2]),
    };
    let status = match result {
        Ok(value) => {
            // SAFETY: `out` is the generated code's result stack slot.
            unsafe { *out = value.bits() as i64 };
            STATUS_OK
        }
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
    };
    restore_scratch_gc_roots(saved);
    status
}

/// `Op::SaveWindowExcursion` (GNU bytecode.c Bsave_window_excursion): pop the
/// body form list, evaluate `(progn . body)` inside a real
/// window-configuration save/restore — the interpreter arm 1:1, including
/// error precedence (a failed restore wins over the body's flow). The body
/// runs arbitrary lisp: everything live is rooted here, the generated code
/// rooted the rest of its frame.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_save_window_excursion(ctx: *mut u8, body: i64, out: *mut i64) -> i64 {
    use crate::emacs_core::window_cmds;
    let body = Value::from_bits(body as usize);
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let root_scope = save_scratch_gc_roots();
    push_scratch_gc_root(body);
    let progn_form = Value::cons(Value::symbol("progn"), body);
    push_scratch_gc_root(progn_form);
    let status = (|| {
        let saved = match window_cmds::builtin_current_window_configuration(ctx, vec![Value::NIL]) {
            Ok(v) => v,
            Err(flow) => {
                stash_pending_flow(flow);
                return STATUS_SIGNAL;
            }
        };
        push_scratch_gc_root(saved);
        let body_result = ctx.eval_sub(progn_form);
        if let Ok(v) = &body_result {
            push_scratch_gc_root(*v);
        }
        let restore_result = window_cmds::builtin_set_window_configuration(ctx, vec![saved]);
        match body_result {
            Ok(result) => match restore_result {
                Ok(_) => {
                    // SAFETY: `out` is the generated code's result stack slot.
                    unsafe { *out = result.bits() as i64 };
                    STATUS_OK
                }
                Err(flow) => {
                    stash_pending_flow(flow);
                    STATUS_SIGNAL
                }
            },
            Err(flow) => {
                // Interpreter parity: vm_try!(restore_result) runs first, so a
                // restore failure takes precedence over the body's flow.
                match restore_result {
                    Err(restore_flow) => stash_pending_flow(restore_flow),
                    Ok(_) => stash_pending_flow(flow),
                }
                STATUS_SIGNAL
            }
        }
    })();
    restore_scratch_gc_roots(root_scope);
    status
}

/// Speculated direct call (`Op::Call` whose callee slot provably holds a
/// constant symbol that was fbound to a bytecode object at compile time).
/// Quit poll FIRST (the interpreter's Op::Call order — quit processing can run
/// lisp, including fset), then the validity check: if `ctx.obarray`'s
/// function_epoch still equals this site's armed epoch, NO function binding
/// anywhere has changed since the binding was observed equal to `expected`,
/// so the callee object is still reachable through the obarray and calling it
/// directly is exactly equivalent to resolving the symbol — minus the
/// resolution. On an epoch move, re-validate THIS binding: unchanged -> re-arm
/// the slot and proceed direct; changed -> strict symbol call (fset/advice
/// take effect immediately, GNU default-settings parity).
/// SAFETY: same vmctx contract as [`neovm_jit_call`]; `slot` points into the
/// owning CompiledLeaf's spec_slots (alive whenever its code runs).
extern "C" fn neovm_jit_call_spec(
    ctx: *mut u8,
    sym: i64,
    expected: i64,
    slot: i64,
    args_ptr: *const i64,
    nargs: i64,
    out: *mut i64,
) -> i64 {
    // Debug-build evidence that speculation actually engages (tests assert on
    // it; release builds carry no counter).
    #[cfg(debug_assertions)]
    SPEC_CALL_COUNT.fetch_add(1, Ordering::Relaxed);
    let nargs = nargs as usize;
    let saved = save_scratch_gc_roots();
    // Build a rooted LispArgVec from the caller's call-args slot — used only by
    // the strict-call fallback paths (call_for_jit). The native-to-native fast
    // path passes `args_ptr` straight through and never materializes this.
    let read_rooted_args = || {
        let mut args = LispArgVec::new();
        for i in 0..nargs {
            // SAFETY: the generated code stored exactly `nargs` argument words
            // at `args_ptr` (its call-args slot) immediately before this call.
            let v = Value::from_bits(unsafe { *args_ptr.add(i) } as usize);
            push_scratch_gc_root(v);
            args.push(v);
        }
        args
    };
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let status = match ctx.maybe_quit() {
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
        Ok(()) => {
            // SAFETY: slot points into the executing leaf's spec_slots.
            let slot = unsafe { &*(slot as *const SpecSlot) };
            let epoch = ctx.obarray.function_epoch();
            let armed = slot.epoch.load(Ordering::Relaxed) == epoch || {
                let cur = ctx.obarray.symbol_function_id(SymId(sym as u32));
                if cur.is_some_and(|v| v.bits() as i64 == expected) {
                    slot.epoch.store(epoch, Ordering::Relaxed);
                    true
                } else {
                    // The binding changed: drop any cached callee leaf so a
                    // later re-arm can't reuse a stale callee.
                    slot.leaf.store(0, Ordering::Relaxed);
                    false
                }
            };
            // Armed: the symbol still names the compile-time bytecode object.
            // Try the fast path (cached leaf, native-to-native pass-through when
            // the callee is a pure fixed-arity match — no arg marshaling at
            // all). Fall back to the strict call on the VALUE if it can't be
            // fast-pathed (arity / not compilable). Not armed: strict call on
            // the SYMBOL (resolves the new binding — fset/advice take effect
            // immediately).
            let mut vm = Vm::from_context(ctx);
            let outcome = if armed {
                let target = Value::from_bits(expected as usize);
                push_scratch_gc_root(target);
                match vm.call_armed_callee_native(target, &slot.leaf, args_ptr, nargs) {
                    Some(res) => res,
                    None => vm.call_for_jit(target, read_rooted_args()),
                }
            } else {
                let target = Value::from_sym_id(SymId(sym as u32));
                push_scratch_gc_root(target);
                vm.call_for_jit(target, read_rooted_args())
            };
            match outcome {
                Ok(value) => {
                    // SAFETY: `out` is the generated code's result stack slot.
                    unsafe { *out = value.bits() as i64 };
                    STATUS_OK
                }
                Err(flow) => {
                    stash_pending_flow(flow);
                    STATUS_SIGNAL
                }
            }
        }
    };
    restore_scratch_gc_roots(saved);
    status
}

/// `Op::Throw`: stash `Flow::Throw{tag, value}` for the signal-exit path.
/// Compiled bodies have no local handlers (handler opcodes bail), so a throw
/// always propagates out — exactly the interpreter's `resume_nonlocal` once no
/// local handler matches. Context-free.
extern "C" fn neovm_jit_throw(tag: i64, value: i64) {
    stash_pending_flow(Flow::Throw {
        tag: Value::from_bits(tag as usize),
        value: Value::from_bits(value as usize),
    });
}

/// Slow path for `integerp` when the value isn't a fixnum: bignums are
/// veclikes, so delegate to the value layer's own predicate. Context-free,
/// pure, never allocates or signals.
extern "C" fn neovm_jit_integerp_slow(v: i64) -> i64 {
    let v = Value::from_bits(v as usize);
    (if v.is_integer() {
        Value::T.bits()
    } else {
        Value::NIL.bits()
    }) as i64
}

/// Slow path for `numberp` when the value isn't a fixnum (floats, bignums).
/// Context-free, pure, never allocates or signals.
extern "C" fn neovm_jit_numberp_slow(v: i64) -> i64 {
    let v = Value::from_bits(v as usize);
    (if v.is_number() {
        Value::T.bits()
    } else {
        Value::NIL.bits()
    }) as i64
}

/// `Op::SaveCurrentBuffer`: record the current buffer on the specpdl + the
/// bind stack, exactly like the interpreter arm (conditional + infallible).
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_save_current_buffer(ctx: *mut u8) {
    use crate::emacs_core::eval::SpecBinding;
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    if let Some(buffer_id) = ctx.buffers.current_buffer().map(|buffer| buffer.id) {
        JIT_BIND_STACK.with(|s| s.borrow_mut().push(ctx.specpdl.len()));
        ctx.specpdl
            .push(SpecBinding::SaveCurrentBuffer { buffer_id });
    }
}

/// `Op::SaveExcursion`: record point/mark/buffer via the same Context helper
/// the interpreter uses (`record_save_excursion` pushes the specpdl record and
/// returns the pre-push depth for the bind stack).
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_save_excursion(ctx: *mut u8) {
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    if let Some(count) = ctx.record_save_excursion() {
        JIT_BIND_STACK.with(|s| s.borrow_mut().push(count));
    }
}

/// `Op::SaveRestriction`: record the narrowing state, exactly like the
/// interpreter arm (conditional + infallible).
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_save_restriction(ctx: *mut u8) {
    use crate::emacs_core::eval::SpecBinding;
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    if let Some(saved) = ctx.buffers.save_current_restriction_state() {
        JIT_BIND_STACK.with(|s| s.borrow_mut().push(ctx.specpdl.len()));
        ctx.specpdl
            .push(SpecBinding::SaveRestriction { state: saved });
    }
}

/// `Op::UnwindProtectPop`: register an unwind-protect cleanup form as a
/// specpdl record (the interpreter arm mirrored 1:1 — same `SpecBinding`
/// entry, same captured lexenv). The cleanup runs whenever `unbind_to` crosses
/// it: the matching `Unbind`, or the frame unwind on any exit — shared
/// machinery with the interpreter, including the signal path.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_unwind_protect(ctx: *mut u8, forms: i64) {
    use crate::emacs_core::eval::SpecBinding;
    let forms = Value::from_bits(forms as usize);
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    JIT_BIND_STACK.with(|s| s.borrow_mut().push(ctx.specpdl.len()));
    let lexenv = ctx.lexenv;
    ctx.specpdl
        .push(SpecBinding::UnwindProtect { forms, lexenv });
}

/// `Op::PushConditionCase`: register a `condition-case` handler frame on the
/// ctx-level condition stack, mirroring the interpreter arm exactly — implicit
/// `error` conditions, a `VmConditionCase` resume carrying the bytecode target,
/// the static operand-stack depth at the push, the current specpdl depth, and
/// the current JIT bind-stack length (this frame's analogue of the
/// interpreter's frame-local `bind_stack`). Infallible.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_push_cc(ctx: *mut u8, target: i64, stack_len: i64) {
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let resume_id = ctx.allocate_resume_id();
    let bind_stack_len = JIT_BIND_STACK.with(|s| s.borrow().len());
    let spec_depth = ctx.specpdl.len();
    ctx.push_condition_frame(ConditionFrame::ConditionCase {
        conditions: Value::symbol("error"),
        resume: ResumeTarget::VmConditionCase {
            resume_id,
            target: target as u32,
            stack_len: stack_len as usize,
            spec_depth,
            bind_stack_len,
        },
    });
}

/// `Op::PushConditionCaseRaw`: like [`neovm_jit_push_cc`] but the handler
/// pattern (conditions) was popped from the operand stack by the generated
/// code and is passed in. Infallible.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_push_cc_raw(ctx: *mut u8, target: i64, stack_len: i64, conditions: i64) {
    let conditions = Value::from_bits(conditions as usize);
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let resume_id = ctx.allocate_resume_id();
    let bind_stack_len = JIT_BIND_STACK.with(|s| s.borrow().len());
    let spec_depth = ctx.specpdl.len();
    ctx.push_condition_frame(ConditionFrame::ConditionCase {
        conditions,
        resume: ResumeTarget::VmConditionCase {
            resume_id,
            target: target as u32,
            stack_len: stack_len as usize,
            spec_depth,
            bind_stack_len,
        },
    });
}

/// `Op::PushCatch`: register a `catch` frame (tag popped by the generated
/// code), mirroring the interpreter arm. Infallible.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_push_catch(ctx: *mut u8, target: i64, stack_len: i64, tag: i64) {
    let tag = Value::from_bits(tag as usize);
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let resume_id = ctx.allocate_resume_id();
    let bind_stack_len = JIT_BIND_STACK.with(|s| s.borrow().len());
    let spec_depth = ctx.specpdl.len();
    ctx.push_condition_frame(ConditionFrame::Catch {
        tag,
        resume: ResumeTarget::VmCatch {
            resume_id,
            target: target as u32,
            stack_len: stack_len as usize,
            spec_depth,
            bind_stack_len,
        },
    });
}

/// `Op::PopHandler`: drop the innermost handler frame (normal exit from a
/// protected extent). The static handler-depth analysis guarantees balance.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_pop_handler(ctx: *mut u8) {
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    ctx.pop_condition_frame();
}

/// Handler-match dispatch: called on the cold path after a runtime call inside
/// a protected extent returned [`STATUS_SIGNAL`], with `ours` = the number of
/// condition frames this *native frame* has active at the site (static). The
/// per-frame invariant "callees pop their own frames on every exit" means the
/// top `ours` frames of `ctx.condition_stack` are exactly ours.
///
/// Mirrors `Vm::resume_nonlocal` 1:1:
/// - `Throw`: select via `matching_catch_resume` (whole-stack scan, like the
///   interpreter); pop our frames innermost-first looking for the selected
///   resume. Found -> unwind (`unbind_to` to the frame's spec depth, truncate
///   the JIT bind stack), write the thrown value through `out`, and return the
///   0-based miss count `m` (0 = innermost handler matched). Selected-but-outer
///   -> all ours popped, rethrow (-1). No catch anywhere -> `no-catch` signal.
/// - `Signal`: `kill-emacs` propagates untouched (frames left for the frame
///   unwind, like the interpreter's early return). Otherwise run
///   `dispatch_signal_if_needed` (signal hooks + handler-bind — may run lisp,
///   GC, or itself raise: loop on the new flow, the interpreter's recursion),
///   then unwind to `selected_resume` among our frames; on a match the error
///   object (`make_signal_binding_value`) goes through `out`.
///
/// The generated code keeps its live operand-stack values rooted across this
/// call (the lisp run by cleanups/hooks can collect) and maps the returned
/// ordinal back to the statically known handler target.
/// SAFETY: same vmctx contract as [`neovm_jit_call`]; `out` is the generated
/// code's result stack slot.
extern "C" fn neovm_jit_match_handler(ctx: *mut u8, ours: i64, out: *mut i64) -> i64 {
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let ours = ours as usize;
    let mut flow = take_pending_flow().expect("match shim runs only after STATUS_SIGNAL");
    loop {
        match flow {
            Flow::Throw { tag, value } => {
                let Some(selected) = ctx.matching_catch_resume(&tag) else {
                    // No matching catch anywhere: unwind all our frames and
                    // propagate `no-catch` (resume_nonlocal parity).
                    for _ in 0..ours {
                        ctx.pop_condition_frame();
                    }
                    stash_pending_flow(signal("no-catch", vec![tag, value]));
                    return -1;
                };
                for m in 0..ours {
                    let frame = ctx
                        .pop_condition_frame()
                        .expect("JIT handler frames missing from condition stack");
                    let resume = condition_frame_resume(frame);
                    if resume == selected {
                        let ResumeTarget::VmCatch {
                            spec_depth,
                            bind_stack_len,
                            ..
                        } = resume
                        else {
                            unreachable!("JIT catch frame carries a VmCatch resume");
                        };
                        // unbind_to may run unwind-protect cleanups (lisp ->
                        // GC); keep the carried values alive across it.
                        let saved = save_scratch_gc_roots();
                        push_scratch_gc_root(tag);
                        push_scratch_gc_root(value);
                        ctx.unbind_to(spec_depth);
                        JIT_BIND_STACK.with(|s| s.borrow_mut().truncate(bind_stack_len));
                        restore_scratch_gc_roots(saved);
                        // SAFETY: `out` is the generated code's result slot.
                        unsafe { *out = value.bits() as i64 };
                        return m as i64;
                    }
                }
                // The selected catch belongs to an outer frame: ours are all
                // popped; rethrow for the frame unwind + outer handlers.
                stash_pending_flow(Flow::Throw { tag, value });
                return -1;
            }
            Flow::Signal(sig) => {
                if sig.symbol == intern("kill-emacs") {
                    // Interpreter parity: propagate immediately, frames left
                    // to the frame-exit truncation.
                    stash_pending_flow(Flow::Signal(sig));
                    return -1;
                }
                // Signal hooks / handler-bind handlers may run lisp and GC;
                // root the signal payload across the dispatch.
                let saved = save_scratch_gc_roots();
                push_scratch_gc_root(Value::from_sym_id(sig.symbol));
                for v in sig.data.iter().copied() {
                    push_scratch_gc_root(v);
                }
                if let Some(raw) = sig.raw_data {
                    push_scratch_gc_root(raw);
                }
                let dispatched = ctx.dispatch_signal_if_needed(sig);
                restore_scratch_gc_roots(saved);
                let sig = match dispatched {
                    Ok(sig) => sig,
                    // A hook/handler raised: restart matching on the new flow
                    // (resume_nonlocal recurses here).
                    Err(next) => {
                        flow = next;
                        continue;
                    }
                };
                let Some(selected) = sig.selected_resume.clone() else {
                    for _ in 0..ours {
                        ctx.pop_condition_frame();
                    }
                    stash_pending_flow(Flow::Signal(sig));
                    return -1;
                };
                for m in 0..ours {
                    let frame = ctx
                        .pop_condition_frame()
                        .expect("JIT handler frames missing from condition stack");
                    let resume = condition_frame_resume(frame);
                    if resume == selected {
                        let ResumeTarget::VmConditionCase {
                            spec_depth,
                            bind_stack_len,
                            ..
                        } = resume
                        else {
                            unreachable!(
                                "JIT condition-case frame carries a VmConditionCase resume"
                            );
                        };
                        // unbind_to runs cleanups and the error object below
                        // allocates: root the signal payload throughout.
                        let saved = save_scratch_gc_roots();
                        push_scratch_gc_root(Value::from_sym_id(sig.symbol));
                        for v in sig.data.iter().copied() {
                            push_scratch_gc_root(v);
                        }
                        if let Some(raw) = sig.raw_data {
                            push_scratch_gc_root(raw);
                        }
                        ctx.unbind_to(spec_depth);
                        JIT_BIND_STACK.with(|s| s.borrow_mut().truncate(bind_stack_len));
                        let binding = make_signal_binding_value(&sig);
                        restore_scratch_gc_roots(saved);
                        // SAFETY: `out` is the generated code's result slot.
                        unsafe { *out = binding.bits() as i64 };
                        return m as i64;
                    }
                }
                stash_pending_flow(Flow::Signal(sig));
                return -1;
            }
        }
    }
}

/// `Op::Switch` lookup result: the dispatch value is not in the jump table —
/// fall through (interpreter parity).
const JIT_SWITCH_MISS: i64 = -1;
/// `Op::Switch` lookup result: the table no longer matches what was compiled
/// (a value mutated to a non-fixnum); the shim stashed a signal.
const JIT_SWITCH_STALE: i64 = -2;

/// `Op::Switch`: look the dispatch value up in the (statically verified
/// compile-time constant) hash-table jump table, with the interpreter's exact
/// key semantics (`to_hash_key_swp` under the table's own test). Returns the
/// raw fixnum target address on a hit ([`JIT_SWITCH_MISS`]/[`JIT_SWITCH_STALE`]
/// otherwise); the generated code maps raw addresses onto the statically
/// resolved target blocks. Pure lookup — no allocation, no lisp.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_switch(ctx: *mut u8, dispatch: i64, table: i64) -> i64 {
    let table = Value::from_bits(table as usize);
    let dispatch = Value::from_bits(dispatch as usize);
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    let Some(ht) = table.as_hash_table() else {
        // Statically verified a hash table; only runtime mutation of the
        // constant pool itself could change that.
        stash_pending_flow(signal(
            "error",
            vec![Value::string("jit: switch jump table mutated at runtime")],
        ));
        return JIT_SWITCH_STALE;
    };
    let key = dispatch.to_hash_key_swp(&ht.test, ctx.symbols_with_pos_enabled);
    match ht.data.get(&key).copied() {
        Some(v) => match v.kind() {
            ValueKind::Fixnum(addr) if addr >= 0 => addr,
            _ => {
                stash_pending_flow(signal(
                    "error",
                    vec![Value::string("jit: switch jump table mutated at runtime")],
                ));
                JIT_SWITCH_STALE
            }
        },
        None => JIT_SWITCH_MISS,
    }
}

/// Cold path for a switch hit whose raw address is not in the statically
/// compiled target set (the jump table was mutated after compilation — code
/// the byte-compiler never produces). Stash a loud signal; the generated code
/// routes to its signal path. Context-free.
extern "C" fn neovm_jit_switch_stale() {
    stash_pending_flow(signal(
        "error",
        vec![Value::string("jit: switch jump table mutated at runtime")],
    ));
}

/// Back-edge service poll: GC safepoint + `maybe_quit`, via the same shared
/// Context helper the interpreter's `branch_to!` wrap path uses
/// (`bytecode_branch_maybe_gc_and_quit`). Generated code calls this every 255
/// backward jumps (the interpreter's u8 `quitcounter` cadence), with its live
/// operand-stack values rooted by the caller — the poll may collect.
/// SAFETY: same vmctx contract as [`neovm_jit_call`].
extern "C" fn neovm_jit_backedge(ctx: *mut u8) -> i64 {
    // SAFETY: see neovm_jit_call's function-level contract.
    let ctx = unsafe { &mut *(ctx as *mut Context) };
    match ctx.bytecode_branch_maybe_gc_and_quit() {
        Ok(()) => STATUS_OK,
        Err(flow) => {
            stash_pending_flow(flow);
            STATUS_SIGNAL
        }
    }
}

/// Why a bytecode body could not be compiled by this baseline tier.
///
/// Every variant means "stay on the Tier-0 interpreter"; none is fatal.
#[derive(Debug)]
pub enum CompileError {
    /// The function's parameter list is unsupported: `&optional`/`&rest`, or
    /// required params that are dynamically bound (not on the operand stack).
    TakesArguments,
    /// An opcode outside the supported leaf subset (coarse category for logs).
    UnsupportedOp(&'static str),
    /// The body did not end in `Return` (open block / fell off the end).
    NoReturn,
    /// A stack op referenced below the modelled operand stack.
    StackUnderflow,
    /// A `Constant`/`StackRef` operand was out of range for the pool/stack.
    BadOperand,
    /// The body is call-dominated, so native codegen would only add overhead
    /// (per-call operand GC-rooting + a runtime call shim) without an offsetting
    /// win — the baseline tier removes per-op dispatch, not call cost. Measured
    /// net-negative on real workloads; keep it on the interpreter.
    NotProfitable,
    /// The Cranelift backend failed to build or finalize the code.
    Backend(BackendError),
}

impl core::fmt::Display for CompileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CompileError::TakesArguments => write!(f, "function takes arguments"),
            CompileError::UnsupportedOp(k) => write!(f, "unsupported opcode: {k}"),
            CompileError::NoReturn => write!(f, "body does not end in Return"),
            CompileError::StackUnderflow => write!(f, "operand stack underflow"),
            CompileError::BadOperand => write!(f, "operand out of range"),
            CompileError::NotProfitable => write!(f, "call-dominated body, not JIT-profitable"),
            CompileError::Backend(e) => write!(f, "backend: {e}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// A compiled leaf function taking a fixed number of arguments.
///
/// Owns its [`JITModule`], which keeps the executable memory mapped for the
/// lifetime of this handle. The raw entry pointer makes this neither `Send` nor
/// `Sync`, which is correct — the code is tied to its owning module.
pub struct CompiledLeaf {
    /// Number of fixed slots the native code reads from the args pointer at
    /// entry: `nonrest` parameters (required + optional, nil-padded) plus one
    /// slot for the `&rest` list when present. [`call`](Self::call) normalizes
    /// an incoming argument list to exactly this many slots, mirroring the
    /// interpreter's `run_frame` frame seeding.
    arity: usize,
    /// Number of required parameters (lower bound of an acceptable call).
    required: usize,
    /// Whether the last native slot is a `&rest` list.
    has_rest: bool,
    /// Whether the body makes dynamic bindings (`varbind`/`unbind`). When set,
    /// [`call`](Self::call) restores the entry specpdl depth on every exit —
    /// the `cleanup_bytecode_frame` parity unwind — and requires a non-null
    /// vmctx.
    has_binds: bool,
    /// Precise-deopt spill buffer: a failing guard writes the live operand
    /// stack here (raw tagged bits) before returning [`STATUS_DEOPT_AT`].
    /// Untraced by design — consumed immediately after the native call
    /// returns, with no allocation in between.
    deopt_spill: Box<[core::cell::Cell<i64>]>,
    /// Precise-deopt pc/depth/handler-count cells (see [`DeoptCells`]).
    deopt_meta: Box<DeoptCells>,
    /// Per-site direct-call speculation state ([`SpecSlot`]): armed epoch +
    /// lazily-cached callee leaf pointer. Generated code holds raw pointers
    /// into this Box (stable: boxed slice, owned here, code only runs under a
    /// live Rc of this leaf).
    spec_slots: Box<[SpecSlot]>,
    /// Whether the body registers handler frames (`condition-case`/`catch`).
    /// When set, [`call`](Self::call) truncates `ctx.condition_stack` back to
    /// the entry depth on every exit (before the specpdl unwind, exactly like
    /// `cleanup_bytecode_frame` — no stale frame may be matchable while unbind
    /// cleanups run lisp) and requires a non-null vmctx.
    has_handlers: bool,
    // Field order matters for drop: `entry` points into `_module`'s memory; keep
    // `_module` alive as long as the handle exists.
    entry: *const u8,
    _module: JITModule,
}

impl core::fmt::Debug for CompiledLeaf {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // `JITModule` is not `Debug`; show only the entry pointer + arity.
        f.debug_struct("CompiledLeaf")
            .field("arity", &self.arity)
            .field("entry", &self.entry)
            .finish_non_exhaustive()
    }
}

/// Outcome of executing a compiled function.
#[derive(Debug, PartialEq, Eq)]
pub enum NativeRun {
    /// Native code produced a result (the raw tagged [`Value`] bits).
    Ok(usize),
    /// A speculation guard failed. The poisoning analysis guarantees no side
    /// effect (no runtime call) ran before any guard, so the caller can safely
    /// rerun the body on the Tier-0 interpreter. (Also the null-vmctx mapping
    /// of a precise deopt: shim-free bodies are side-effect-free by
    /// construction, so rerun-from-start stays sound for them.)
    Deopt,
    /// A guard failed at a precise bytecode pc with the live operand stack
    /// and frame state captured — resume the Tier-0 interpreter MID-FUNCTION
    /// via `Vm::run_resumed_frame`. The native call performed NO frame
    /// unwind: `binds` (pre-push specpdl depths, this frame's JIT bind-stack
    /// segment) and the `handlers` condition frames remain registered and
    /// their ownership transfers to the resumed frame, which unwinds to
    /// `spec_base`/`cond_base` (the native frame's entry bases) on exit.
    DeoptAt {
        pc: usize,
        stack: Vec<Value>,
        handlers: usize,
        binds: Vec<usize>,
        spec_base: usize,
        cond_base: usize,
    },
    /// A runtime call inside the body raised a non-local `Flow` (signal/throw);
    /// take it with [`take_pending_flow`] and propagate it.
    Signal,
}

impl CompiledLeaf {
    /// The number of fixed slots the native code reads (see the field doc).
    pub fn arity(&self) -> usize {
        self.arity
    }

    /// Whether a call with `n` arguments is valid for this function's lambda
    /// list — the same predicate the interpreter's `run_frame` arity check
    /// applies before signaling `wrong-number-of-arguments`.
    pub fn accepts(&self, n: usize) -> bool {
        let nonrest = self.arity - usize::from(self.has_rest);
        self.required <= n && (self.has_rest || n <= nonrest)
    }

    /// Execute the compiled function with `args` (which must satisfy
    /// [`accepts`](Self::accepts)).
    ///
    /// The argument list is normalized to the native frame exactly as the
    /// interpreter's `run_frame` seeds it: missing `&optional` slots are
    /// nil-padded, and with `&rest` the surplus arguments become a fresh list in
    /// the final slot (allocated here, before entering native code; the caller's
    /// rooting of `args` covers the elements, as it does for `run_frame`).
    ///
    /// `vmctx` is the `*mut Context` runtime-call shims re-enter through. It may
    /// be null **only** when the body performs no runtime re-entry (it contains
    /// no `Call`); allocation (`cons`) uses the thread-local heap and tolerates
    /// a null vmctx too.
    pub fn call(&self, vmctx: *mut u8, args: &[Value]) -> NativeRun {
        debug_assert!(self.accepts(args.len()), "compiled call arity mismatch");
        // Copy the argument bits into a contiguous i64 buffer for the native
        // ABI (no heap alloc for the common <= 8 args). A `Value` is an opaque
        // tagged word here; its `usize` bits ride unchanged in an `i64` slot.
        let nonrest = self.arity - usize::from(self.has_rest);
        let mut arg_bits: SmallVec<[i64; 8]> =
            args.iter().take(nonrest).map(|v| v.bits() as i64).collect();
        // Nil-pad missing &optional parameters.
        while arg_bits.len() < nonrest {
            arg_bits.push(Value::NIL.bits() as i64);
        }
        if self.has_rest {
            let rest = if args.len() > nonrest {
                Value::list_from_slice(&args[nonrest..])
            } else {
                Value::NIL
            };
            arg_bits.push(rest.bits() as i64);
        }
        self.invoke_native(vmctx, arg_bits.as_ptr())
    }

    /// Whether a call with `nargs` arguments needs NO argument normalization
    /// (no `&optional` nil-padding, no `&rest` list construction) — the
    /// native-to-native pre-marshaled fast path applies only then.
    pub(crate) fn is_pure_passthrough(&self, nargs: usize) -> bool {
        !self.has_rest && nargs == self.arity
    }

    /// Native-to-native fast path: invoke the body with `args_ptr` addressing
    /// EXACTLY `self.arity` pre-marshaled argument words (the caller's native
    /// call-args slot). Valid only when [`is_pure_passthrough`](Self::is_pure_passthrough)
    /// holds for the call's argument count — no nil-pad / rest-list step. Skips
    /// the `LispArgVec` build and the `arg_bits` re-marshal that [`call`](Self::call)
    /// pays, which is the per-call cost that dominates call-heavy compiled code.
    ///
    /// SAFETY: `args_ptr` must address `self.arity` valid tagged words that stay
    /// live until the native entry reads them (its first block). The spec fast
    /// path guarantees no GC safepoint runs in between: `maybe_quit` already
    /// returned `Ok` (which does not collect) and nothing allocates on a lisp
    /// heap before the entry consumes its args.
    pub(crate) fn call_premarshaled(&self, vmctx: *mut u8, args_ptr: *const i64) -> NativeRun {
        debug_assert!(!vmctx.is_null(), "native-to-native requires a Context");
        self.invoke_native(vmctx, args_ptr)
    }

    /// The post-marshaling tail shared by [`call`](Self::call) and
    /// [`call_premarshaled`](Self::call_premarshaled): invoke the native entry
    /// with `args_ptr` (exactly `self.arity` words) and handle the `STATUS_*`
    /// outcome — precise-deopt capture (no frame unwind, ownership transfers to
    /// the resumed interpreter frame) or the `cleanup_bytecode_frame`-parity
    /// frame unwind on a normal/signal exit.
    fn invoke_native(&self, vmctx: *mut u8, args_ptr: *const i64) -> NativeRun {
        let mut out: i64 = 0;
        // SAFETY: `entry` is finalized native code with ABI
        // `extern "C" fn(vmctx: *mut u8, args: *const i64, out: *mut i64) -> i64`
        // (built in `lower_leaf`): it reads `self.arity` words from `args`,
        // writes the result bits through `out` and returns STATUS_OK, or returns
        // STATUS_DEOPT/STATUS_SIGNAL without touching `out`. `_module` keeps the
        // code mapped for `&self`; `arg_bits` and `out` outlive the call; for
        // arity 0 `args` is never read; `vmctx` is only dereferenced inside the
        // call shim under its own documented contract.
        // Frame-unwind bookkeeping for dynamic bindings: record the entry
        // specpdl depth and this frame's bind-stack segment base, and restore
        // both on every exit — exactly cleanup_bytecode_frame's unconditional
        // unbind_to(specpdl_base). On a deopt this is a no-op by construction
        // (varbind poisons, so no binding can precede a deopt).
        let bind_frame = if self.has_binds {
            debug_assert!(!vmctx.is_null(), "binding bodies require a Context");
            // SAFETY: the vmctx contract (dormant seam-provided Context); only
            // a length read here.
            let spec_base = unsafe { (*(vmctx as *const Context)).specpdl.len() };
            let stack_base = JIT_BIND_STACK.with(|s| s.borrow().len());
            Some((spec_base, stack_base))
        } else {
            None
        };
        let cond_base = if self.has_handlers {
            debug_assert!(!vmctx.is_null(), "handler bodies require a Context");
            // SAFETY: as above — only a length read.
            Some(unsafe { (*(vmctx as *const Context)).condition_stack_len() })
        } else {
            None
        };
        let status = unsafe {
            let f: extern "C" fn(*mut u8, *const i64, *mut i64) -> i64 =
                core::mem::transmute(self.entry);
            f(vmctx, args_ptr, &mut out as *mut i64)
        };
        if status == STATUS_DEOPT_AT {
            // Precise deopt: NO frame unwind — the resumed interpreter frame
            // takes ownership of the registered binds/handlers and unwinds to
            // the entry bases itself on every exit. With a null vmctx (shim-
            // free test bodies — side-effect-free by construction) fall back
            // to the legacy rerun-from-start mapping.
            if vmctx.is_null() {
                return NativeRun::Deopt;
            }
            let pc = self.deopt_meta.pc.get() as usize;
            let depth = self.deopt_meta.depth.get() as usize;
            let handlers = self.deopt_meta.handlers.get() as usize;
            // No allocation happens between the native spill write and this
            // read; the caller seeds the values into the GC-traced bc_buf
            // before any elisp can run.
            let stack: Vec<Value> = (0..depth)
                .map(|j| Value::from_bits(self.deopt_spill[j].get() as usize))
                .collect();
            let binds: Vec<usize> = match bind_frame {
                Some((_, stack_base)) => JIT_BIND_STACK.with(|s| {
                    let mut s = s.borrow_mut();
                    s.split_off(stack_base)
                }),
                None => Vec::new(),
            };
            // SAFETY: dormant seam Context; length reads only.
            let spec_base = match bind_frame {
                Some((spec_base, _)) => spec_base,
                None => unsafe { (*(vmctx as *const Context)).specpdl.len() },
            };
            let cond_base = match cond_base {
                Some(base) => base,
                None => unsafe { (*(vmctx as *const Context)).condition_stack_len() },
            };
            return NativeRun::DeoptAt {
                pc,
                stack,
                handlers,
                binds,
                spec_base,
                cond_base,
            };
        }
        // cleanup_bytecode_frame parity, same order: condition frames first
        // (the specpdl unwind below can run unwind-protect cleanups — lisp
        // that must not be able to match a stale frame of this dead body),
        // then the dynamic-binding unwind. On a deopt both are exactly what
        // makes the interpreter rerun sound: the rerun re-registers them.
        if let Some(base) = cond_base {
            // SAFETY: the native call has returned; the seam's &mut Context is
            // still dormant (we are inside its dynamic extent).
            unsafe { (*(vmctx as *mut Context)).truncate_condition_stack(base) };
        }
        if let Some((spec_base, stack_base)) = bind_frame {
            JIT_BIND_STACK.with(|s| s.borrow_mut().truncate(stack_base));
            // `unbind_to` runs unwind-protect cleanups (arbitrary lisp -> GC). On
            // STATUS_OK the result lives ONLY in the local `out`, so a
            // cleanup-triggered collection would sweep it while it is the return
            // value (exact-root GC use-after-free). Root it across the unwind via
            // the scratch roots, mirroring the interpreter's
            // `unbind_to_with_result`. Signal/Throw flow components live in the
            // Context's pending state, which the unwind preserves.
            let saved_roots = if status == STATUS_OK {
                let saved = crate::emacs_core::eval::save_scratch_gc_roots();
                crate::emacs_core::eval::push_scratch_gc_root(Value::from_bits(out as usize));
                Some(saved)
            } else {
                None
            };
            // SAFETY: as above.
            unsafe { (*(vmctx as *mut Context)).unbind_to(spec_base) };
            if let Some(saved) = saved_roots {
                crate::emacs_core::eval::restore_scratch_gc_roots(saved);
            }
        }
        match status {
            STATUS_OK => NativeRun::Ok(out as usize),
            STATUS_SIGNAL => NativeRun::Signal,
            _ => NativeRun::Deopt,
        }
    }

    /// Test-only adapter: run with a null vmctx (valid because the test bodies
    /// using it perform no runtime re-entry through `Call`) and map the outcome
    /// to the legacy Option shape (`Ok -> Some(bits)`, `Deopt -> None`).
    /// A `Signal` panics — no shim-free test body can produce one.
    #[cfg(test)]
    pub(crate) fn call_for_test(&self, args: &[Value]) -> Option<usize> {
        match self.call(core::ptr::null_mut(), args) {
            NativeRun::Ok(bits) => Some(bits),
            NativeRun::Deopt | NativeRun::DeoptAt { .. } => None,
            NativeRun::Signal => panic!("unexpected STATUS_SIGNAL from a test body"),
        }
    }
}

/// Coarse opcode category for [`CompileError::UnsupportedOp`] diagnostics.
fn op_category(op: &Op) -> &'static str {
    match op {
        Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem | Op::Add1 | Op::Sub1 | Op::Negate => {
            "arithmetic"
        }
        Op::Call(_) | Op::Apply(_) => "call",
        Op::VarRef(_) | Op::VarSet(_) | Op::VarBind(_) | Op::Unbind(_) => "variable",
        Op::Goto(_)
        | Op::GotoIfNil(_)
        | Op::GotoIfNotNil(_)
        | Op::GotoIfNilElsePop(_)
        | Op::GotoIfNotNilElsePop(_)
        | Op::Switch => "control-flow",
        Op::StackSet(_) | Op::DiscardN(_) => "stack-mutate",
        _ => "other",
    }
}

/// Resolve the `SymId` a `VarRef`/`VarSet` operand names, at compile time —
/// mirrors the interpreter's `sym_id_at` (symbol or symbol-with-pos), except
/// that exotic constants bail to the interpreter instead of falling back to
/// `nil`.
fn const_sym_id(constants: &[Value], idx: u16) -> Result<u32, CompileError> {
    let v = constants
        .get(idx as usize)
        .ok_or(CompileError::BadOperand)?;
    v.as_symbol_id()
        .or_else(|| v.as_symbol_with_pos_sym().and_then(|s| s.as_symbol_id()))
        .map(|id| id.0)
        .ok_or(CompileError::BadOperand)
}

/// True iff this function's parameters are pushed onto the operand stack at
/// entry (so the body's `StackRef` opcodes reach them) — mirrors the
/// interpreter's `params_on_stack` in `vm.rs` `run_frame`. Dynamic-binding
/// bytecode binds params via `varref` instead and is not supported here.
fn params_on_stack(f: &ByteCodeFunction) -> bool {
    f.lexical
        || f.env.is_some()
        || matches!(
            f.arglist.kind(),
            crate::emacs_core::value::ValueKind::Fixnum(_)
        )
}

/// Compile a [`ByteCodeFunction`] whose parameters live on the operand stack
/// (lexical bytecode); otherwise bail.
///
/// `&optional` and `&rest` are supported: the native frame has one slot per
/// non-rest parameter plus one for the rest list, and [`CompiledLeaf::call`]
/// normalizes each incoming argument list to that frame (nil-padding, rest-list
/// construction) exactly as the interpreter's `run_frame` seeds it.
/// Dynamic-binding bytecode (params bound via `varbind`, not on the stack)
/// still bails.
pub fn compile_bytecode_function(f: &ByteCodeFunction) -> Result<CompiledLeaf, CompileError> {
    compile_bytecode_function_with(f, None)
}

/// [`compile_bytecode_function`] with the compiling thread's obarray for
/// direct-call speculation.
/// Profiling chokepoint (env-gated, zero cost when off): record the op-mix of
/// every distinct bytecode function the JIT attempts to compile, so a real
/// workload can be characterized — is hot elisp arithmetic-heavy (unboxing
/// helps), call-heavy (inlining helps), or dispatch/alloc-bound (an MIR tier
/// helps little)? Set `NEOVM_JIT_PROFILE=<path>` to append one CSV row per
/// function. Used to justify (or not) the optimizing Tier-2 investment.
fn jit_profile_path() -> Option<&'static str> {
    use std::sync::OnceLock;
    static PATH: OnceLock<Option<String>> = OnceLock::new();
    PATH.get_or_init(|| std::env::var("NEOVM_JIT_PROFILE").ok())
        .as_deref()
}

/// Verification harness (J0): when `NEOVM_JIT_FORCE_DEOPT=1`, EVERY speculation
/// guard (`emit_guard`) is forced to fail, so every guarded native fast path
/// takes its deopt path instead. Running the full suite with this on (ideally
/// with `NEOVM_JIT_THRESHOLD=1` so every function compiles) exercises every deopt
/// site and must produce results identical to the interpreter — the JIT analogue
/// of `NEOVM_GC_STRESS`/`gc_stress`. Catches deopt-frame-reconstruction bugs (the
/// riskiest part of speculation) before the optimizing Tier-2 adds more guards.
fn jit_force_deopt() -> bool {
    use std::sync::OnceLock;
    static FORCE: OnceLock<bool> = OnceLock::new();
    *FORCE.get_or_init(|| std::env::var("NEOVM_JIT_FORCE_DEOPT").as_deref() == Ok("1"))
}

fn jit_profile_emit(f: &ByteCodeFunction, obarray: Option<&Obarray>, compiled: bool) {
    let Some(path) = jit_profile_path() else {
        return;
    };
    let ops = &f.ops;
    let mut arith = 0u32;
    let mut calls = 0u32;
    let mut alloc = 0u32;
    let mut listops = 0u32;
    let mut varops = 0u32;
    let mut preds = 0u32;
    let mut backedges = 0u32;
    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::Rem
            | Op::Add1
            | Op::Sub1
            | Op::Negate
            | Op::Max
            | Op::Min
            | Op::Eqlsign
            | Op::Lss
            | Op::Gtr
            | Op::Leq
            | Op::Geq => arith += 1,
            Op::Call(_) | Op::Apply(_) | Op::CallBuiltin(..) | Op::CallBuiltinSym(..) => calls += 1,
            Op::Cons | Op::List(_) | Op::Concat(_) | Op::Nconc => alloc += 1,
            Op::Car | Op::Cdr | Op::CarSafe | Op::CdrSafe => listops += 1,
            Op::VarRef(_) | Op::VarSet(_) | Op::VarBind(_) | Op::Unbind(_) => varops += 1,
            Op::Null
            | Op::Not
            | Op::Consp
            | Op::Stringp
            | Op::Listp
            | Op::Symbolp
            | Op::Integerp
            | Op::Numberp => preds += 1,
            Op::Goto(t)
            | Op::GotoIfNil(t)
            | Op::GotoIfNotNil(t)
            | Op::GotoIfNilElsePop(t)
            | Op::GotoIfNotNilElsePop(t) => {
                if (*t as usize) <= i {
                    backedges += 1;
                }
            }
            _ => {}
        }
    }
    // Inlinable call sites: those whose callee is a constant symbol currently
    // fbound to a BYTECODE object (the only directly-inlinable target) — the
    // SAME shape `find_spec_sites` detects. `calls - inlinable` are subr /
    // dynamic / non-bytecode callees inlining can't directly take. This sizes
    // inlining's TRUE surface (vs the call-bearing upper bound).
    let arity =
        f.params.required.len() + f.params.optional.len() + usize::from(f.params.rest.is_some());
    let inlinable = match obarray {
        Some(ob) => analyze_cfg(ops, &f.constants, f.gnu_byte_offset_map.as_deref(), arity)
            .map(|cfg| find_spec_sites(ops, &f.constants, &cfg.leaders, ob).len())
            .unwrap_or(0),
        None => 0,
    };
    let line = format!(
        "{},{},{},{},{},{},{},{},{},{},{}\n",
        ops.len(),
        arith,
        calls,
        alloc,
        listops,
        varops,
        preds,
        backedges,
        u8::from(backedges > 0),
        u8::from(compiled),
        inlinable,
    );
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(line.as_bytes());
    }
}

#[cfg(test)]
thread_local! {
    /// Per-thread override for the profitability gate, set by tests that need to
    /// compile a deliberately call-dominated body to exercise the call/spec
    /// machinery (which production would correctly decline to compile).
    static PROFIT_GATE_TEST_OVERRIDE: std::cell::Cell<Option<bool>> =
        const { std::cell::Cell::new(None) };
}

/// Force the profitability gate on/off on the current thread (tests only).
#[cfg(test)]
pub(crate) fn force_profit_gate_for_test(on: bool) {
    PROFIT_GATE_TEST_OVERRIDE.with(|c| c.set(Some(on)));
}

/// Is the JIT profitability gate enabled? Default yes; `NEOVM_JIT_PROFIT=off`
/// disables it, so the gate can be A/B-measured against the old behavior in a
/// single build.
fn jit_profit_gate_on() -> bool {
    #[cfg(test)]
    if let Some(o) = PROFIT_GATE_TEST_OVERRIDE.with(|c| c.get()) {
        return o;
    }
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("NEOVM_JIT_PROFIT").as_deref() != Ok("off"))
}

/// Decide whether a bytecode body is worth compiling.
///
/// The baseline tier only removes per-op interpreter *dispatch*. A function call
/// costs MORE in native code than in the VM — each call GC-roots its live
/// operands and trampolines through a runtime shim (`neovm_jit_gc_push` +
/// `neovm_jit_call`). So a call-dominated body pays that overhead with nothing
/// to offset it: measured ~32% SLOWER on real workloads (byte-compilation,
/// font-lock), where ~36 of 48 tiered bodies had zero arithmetic and ~10 calls
/// each — pure call/control code the native frame can only shuffle, not speed
/// up. Compile only when arithmetic is not outnumbered by calls; the genuine win
/// shape (hot arithmetic/control loops — the 7x microbenchmark) clears this, and
/// call-free bodies always pass (`0 <= 0`).
fn body_is_jit_profitable(ops: &[Op]) -> bool {
    if !jit_profit_gate_on() {
        return true;
    }
    let mut arith = 0u32;
    let mut calls = 0u32;
    for op in ops {
        match op {
            Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::Rem
            | Op::Add1
            | Op::Sub1
            | Op::Negate
            | Op::Max
            | Op::Min
            | Op::Eqlsign
            | Op::Lss
            | Op::Gtr
            | Op::Leq
            | Op::Geq => arith += 1,
            Op::Call(_) | Op::Apply(_) | Op::CallBuiltin(..) | Op::CallBuiltinSym(..) => calls += 1,
            _ => {}
        }
    }
    calls <= arith
}

pub fn compile_bytecode_function_with(
    f: &ByteCodeFunction,
    obarray: Option<&Obarray>,
) -> Result<CompiledLeaf, CompileError> {
    let result = compile_bytecode_function_inner(f, obarray);
    if jit_profile_path().is_some() {
        jit_profile_emit(f, obarray, result.is_ok());
    }
    result
}

fn compile_bytecode_function_inner(
    f: &ByteCodeFunction,
    obarray: Option<&Obarray>,
) -> Result<CompiledLeaf, CompileError> {
    let required = f.params.required.len();
    let nonrest = required + f.params.optional.len();
    let has_rest = f.params.rest.is_some();
    let native_arity = nonrest + usize::from(has_rest);
    if native_arity > 0 && !params_on_stack(f) {
        // Params are dynamically bound, not on the stack — `StackRef` would not
        // find them.
        return Err(CompileError::TakesArguments);
    }
    if !body_is_jit_profitable(&f.ops) {
        // Call-dominated body: native codegen would only add rooting + call-shim
        // overhead. Keep it on the interpreter (cached as NotCompilable).
        return Err(CompileError::NotProfitable);
    }
    let mut leaf = lower_leaf_full(
        &f.ops,
        &f.constants,
        native_arity,
        f.gnu_byte_offset_map.as_deref(),
        obarray,
    )?;
    leaf.required = required;
    leaf.has_rest = has_rest;
    Ok(leaf)
}

/// Emit a speculation guard.
///
/// If `cond` (an `i8` boolean from `icmp`) is false, branch to the shared deopt
/// block — created lazily on first use; otherwise fall through into a fresh,
/// sealed continuation block. On return, the builder is positioned in the
/// continuation so lowering continues on the success path.
fn emit_guard(fb: &mut FunctionBuilder, deopt: Block, cond: ClifValue) {
    // J0 verification harness: force every guard to fail so the deopt path is
    // always taken (see `jit_force_deopt`). A constant-false condition makes
    // `brif` unconditionally branch to `deopt`.
    let cond = if jit_force_deopt() {
        let ty = fb.func.dfg.value_type(cond);
        fb.ins().iconst(ty, 0)
    } else {
        cond
    };
    let cont = fb.create_block();
    fb.ins().brif(cond, cont, &[], deopt, &[]);
    fb.switch_to_block(cont);
    // `cont`'s only predecessor is the guard branch just emitted.
    fb.seal_block(cont);
}

/// True if `v` is a compile-time fixnum constant — an `iconst` whose immediate
/// already carries the fixnum tag bits. A runtime fixnum guard on such a value
/// is provably unnecessary (it is the same fixnum on every path), so
/// [`guard_fixnum`] can skip it. This is the safe, dataflow-free subset of
/// redundant-guard elimination: constant operands of arithmetic/comparison are
/// pervasive (`(+ i 1)`, `(< i n)`, `(1+ i)`), and a fixnum `iconst` dominates
/// every use, so eliding its guard cannot change any result or deopt.
fn is_fixnum_const(fb: &FunctionBuilder, v: ClifValue) -> bool {
    use cranelift_codegen::ir::{InstructionData, Opcode, ValueDef};
    if let ValueDef::Result(inst, _) = fb.func.dfg.value_def(v) {
        if let InstructionData::UnaryImm {
            opcode: Opcode::Iconst,
            imm,
        } = fb.func.dfg.insts[inst]
        {
            return (imm.bits() & FIXNUM_CHECK_MASK as i64) == FIXNUM_CHECK_VALUE as i64;
        }
    }
    false
}

/// True if `v` is provably a fixnum at this point — a fixnum constant
/// ([`is_fixnum_const`]) OR the output of [`retag_fixnum`], i.e.
/// `bor_imm(ishl_imm(_, k>=FIXNUM_SHIFT), FIXNUM_CHECK_VALUE)`, whose low tag
/// bits are exactly `0b10`. In either case a fixnum guard on `v` would always
/// pass, so it can be elided. The retag case extends redundant-guard elimination
/// to chained arithmetic WITHIN a block: the range-checked, retagged inner result
/// of `(+ (+ a b) c)` / `(< (1+ i) n)` is re-guarded for nothing. (Sound even if
/// some non-retag op produced the same bit pattern — any value with low bits
/// `0b10` passes the guard. opt_level=none keeps the instruction sequence stable.)
fn is_known_fixnum(fb: &FunctionBuilder, v: ClifValue) -> bool {
    use cranelift_codegen::ir::{InstructionData, Opcode, ValueDef};
    if is_fixnum_const(fb, v) {
        return true;
    }
    let ValueDef::Result(bor, _) = fb.func.dfg.value_def(v) else {
        return false;
    };
    let InstructionData::BinaryImm64 {
        opcode: Opcode::BorImm,
        imm,
        arg,
    } = fb.func.dfg.insts[bor]
    else {
        return false;
    };
    if imm.bits() != FIXNUM_CHECK_VALUE as i64 {
        return false;
    }
    // The bor operand must clear the low FIXNUM_SHIFT bits (a left shift by at
    // least FIXNUM_SHIFT), so `v`'s low two bits are exactly the fixnum tag.
    let ValueDef::Result(shl, _) = fb.func.dfg.value_def(arg) else {
        return false;
    };
    matches!(
        fb.func.dfg.insts[shl],
        InstructionData::BinaryImm64 {
            opcode: Opcode::IshlImm,
            imm: shift,
            ..
        } if shift.bits() >= FIXNUM_SHIFT as i64
    )
}

/// Guard that `v` is a fixnum (`(v & 0b11) == 0b10`), deopting otherwise.
fn guard_fixnum(fb: &mut FunctionBuilder, deopt: Block, v: ClifValue, known: &HashSet<ClifValue>) {
    // Redundant-guard elimination: a value provably a fixnum needs no runtime
    // guard. Within-block: a fixnum constant or range-checked+retagged arithmetic
    // result ([`is_known_fixnum`]). Cross-block: an operand the dataflow analysis
    // proved fixnum at this block's entry ([`compute_known_fixnum_slots`], seeded
    // into `known` by `lower_leaf_full`).
    if is_known_fixnum(fb, v) || known.contains(&v) {
        return;
    }
    let tag = fb.ins().band_imm(v, FIXNUM_CHECK_MASK as i64);
    let is_fix = fb
        .ins()
        .icmp_imm(IntCC::Equal, tag, FIXNUM_CHECK_VALUE as i64);
    emit_guard(fb, deopt, is_fix);
}

/// Retag an untagged i64 `n` as a fixnum `Value`: `(n << 2) | 2`.
fn retag_fixnum(fb: &mut FunctionBuilder, n: ClifValue) -> ClifValue {
    let shifted = fb.ins().ishl_imm(n, FIXNUM_SHIFT as i64);
    fb.ins().bor_imm(shifted, FIXNUM_CHECK_VALUE as i64)
}

/// Lower a fixnum-fast-path binary op (`Add`/`Sub`) with the exact parity the
/// interpreter uses (`vm.rs` `Op::Add`): require both operands be fixnums and
/// the result be in fixnum range, else deopt. Returns the tagged-fixnum result.
fn lower_fixnum_binop(
    fb: &mut FunctionBuilder,
    deopt: Block,
    is_sub: bool,
    a: ClifValue,
    b: ClifValue,
    known: &HashSet<ClifValue>,
) -> ClifValue {
    guard_fixnum(fb, deopt, a, known);
    guard_fixnum(fb, deopt, b, known);

    // Untag (arithmetic shift right by 2 == GNU XFIXNUM), compute, range-check.
    let av = fb.ins().sshr_imm(a, FIXNUM_SHIFT as i64);
    let bv = fb.ins().sshr_imm(b, FIXNUM_SHIFT as i64);
    // Operands are <= 61-bit, so the i64 result cannot overflow; a fixnum-range
    // check is sufficient and matches the interpreter exactly.
    let res = if is_sub {
        fb.ins().isub(av, bv)
    } else {
        fb.ins().iadd(av, bv)
    };

    // Guard: MOST_NEGATIVE_FIXNUM <= res <= MOST_POSITIVE_FIXNUM.
    let ge_lo = fb.ins().icmp_imm(
        IntCC::SignedGreaterThanOrEqual,
        res,
        Value::MOST_NEGATIVE_FIXNUM,
    );
    let le_hi = fb.ins().icmp_imm(
        IntCC::SignedLessThanOrEqual,
        res,
        Value::MOST_POSITIVE_FIXNUM,
    );
    let in_range = fb.ins().band(ge_lo, le_hi);
    emit_guard(fb, deopt, in_range);

    retag_fixnum(fb, res)
}

/// A fixnum-fast-path unary opcode.
#[derive(Clone, Copy)]
enum UnaryKind {
    /// `1+`: n -> n + 1.
    Add1,
    /// `1-`: n -> n - 1.
    Sub1,
    /// unary `-`: n -> -n.
    Negate,
}

/// Lower a fixnum-fast-path unary op with exact interpreter parity (`vm.rs`
/// `Op::Add1`/`Op::Sub1`/`Op::Negate`): require a fixnum operand whose result
/// stays in range, else deopt. The single out-of-range input per op is the
/// boundary fixnum, so the interpreter's `n != BOUND` guard is reproduced
/// exactly rather than a post-compute range check.
fn lower_fixnum_unop(
    fb: &mut FunctionBuilder,
    deopt: Block,
    kind: UnaryKind,
    a: ClifValue,
    known: &HashSet<ClifValue>,
) -> ClifValue {
    guard_fixnum(fb, deopt, a, known);
    let n = fb.ins().sshr_imm(a, FIXNUM_SHIFT as i64);

    // The only input that leaves fixnum range is the op's boundary value.
    let bound = match kind {
        UnaryKind::Add1 => Value::MOST_POSITIVE_FIXNUM,
        UnaryKind::Sub1 | UnaryKind::Negate => Value::MOST_NEGATIVE_FIXNUM,
    };
    let in_range = fb.ins().icmp_imm(IntCC::NotEqual, n, bound);
    emit_guard(fb, deopt, in_range);

    let res = match kind {
        UnaryKind::Add1 => fb.ins().iadd_imm(n, 1),
        UnaryKind::Sub1 => fb.ins().iadd_imm(n, -1),
        UnaryKind::Negate => fb.ins().ineg(n),
    };
    retag_fixnum(fb, res)
}

/// Lower a fixnum numeric comparison (`=`/`<`/`>`/`<=`/`>=`) with exact
/// interpreter parity (`vm.rs` `Op::Lss` &c.): require both operands be fixnums
/// else deopt, then select `t`/`nil` from the comparison — no branch needed.
fn lower_fixnum_compare(
    fb: &mut FunctionBuilder,
    deopt: Block,
    cc: IntCC,
    a: ClifValue,
    b: ClifValue,
    known: &HashSet<ClifValue>,
) -> ClifValue {
    guard_fixnum(fb, deopt, a, known);
    guard_fixnum(fb, deopt, b, known);
    let av = fb.ins().sshr_imm(a, FIXNUM_SHIFT as i64);
    let bv = fb.ins().sshr_imm(b, FIXNUM_SHIFT as i64);
    let cond = fb.ins().icmp(cc, av, bv);
    let t = fb.ins().iconst(types::I64, Value::T.bits() as i64);
    let nil = fb.ins().iconst(types::I64, Value::NIL.bits() as i64);
    fb.ins().select(cond, t, nil)
}

/// Lower a fixnum multiply with exact interpreter parity (`vm.rs` `Op::Mul`):
/// both operands fixnums and the exact product in fixnum range, else deopt.
///
/// Operands are <= 61-bit so the product is <= 122-bit; widening to `i128` makes
/// it exact, then a single range check covers both i64 overflow and
/// fixnum-range overflow at once.
fn lower_fixnum_mul(
    fb: &mut FunctionBuilder,
    deopt: Block,
    a: ClifValue,
    b: ClifValue,
    known: &HashSet<ClifValue>,
) -> ClifValue {
    guard_fixnum(fb, deopt, a, known);
    guard_fixnum(fb, deopt, b, known);
    let av = fb.ins().sshr_imm(a, FIXNUM_SHIFT as i64);
    let bv = fb.ins().sshr_imm(b, FIXNUM_SHIFT as i64);

    let a128 = fb.ins().sextend(types::I128, av);
    let b128 = fb.ins().sextend(types::I128, bv);
    let prod = fb.ins().imul(a128, b128);

    let lo = fb.ins().iconst(types::I64, Value::MOST_NEGATIVE_FIXNUM);
    let hi = fb.ins().iconst(types::I64, Value::MOST_POSITIVE_FIXNUM);
    let lo128 = fb.ins().sextend(types::I128, lo);
    let hi128 = fb.ins().sextend(types::I128, hi);
    let ge = fb.ins().icmp(IntCC::SignedGreaterThanOrEqual, prod, lo128);
    let le = fb.ins().icmp(IntCC::SignedLessThanOrEqual, prod, hi128);
    let in_range = fb.ins().band(ge, le);
    emit_guard(fb, deopt, in_range);

    let res = fb.ins().ireduce(types::I64, prod);
    retag_fixnum(fb, res)
}

/// Lower fixnum `/` or `%` with exact interpreter parity (`vm.rs`
/// `Op::Div`/`Op::Rem`): both operands fixnums and the divisor nonzero, else
/// deopt (the interpreter's `/` builtin signals arith-error on zero). Rust and
/// CLIF `sdiv`/`srem` both truncate toward zero, matching the interpreter; the
/// operands are <= 61-bit so the i64 ops cannot trap, and the interpreter's
/// `Value::fixnum` retag of `MOST_NEGATIVE_FIXNUM / -1` (a wrap) produces the
/// same bits as our retag, so no extra range guard is needed for parity.
fn lower_fixnum_divrem(
    fb: &mut FunctionBuilder,
    deopt: Block,
    is_rem: bool,
    a: ClifValue,
    b: ClifValue,
    known: &HashSet<ClifValue>,
) -> ClifValue {
    guard_fixnum(fb, deopt, a, known);
    guard_fixnum(fb, deopt, b, known);
    let bv = fb.ins().sshr_imm(b, FIXNUM_SHIFT as i64);
    let nonzero = fb.ins().icmp_imm(IntCC::NotEqual, bv, 0);
    emit_guard(fb, deopt, nonzero);
    let av = fb.ins().sshr_imm(a, FIXNUM_SHIFT as i64);
    let res = if is_rem {
        fb.ins().srem(av, bv)
    } else {
        fb.ins().sdiv(av, bv)
    };
    retag_fixnum(fb, res)
}

/// A non-allocating unary type/nil predicate. Inspects only the tagged bits;
/// never dereferences the value, allocates, or deopts.
#[derive(Clone, Copy)]
enum PredKind {
    /// `null`/`not`: value is nil.
    Null,
    /// `consp`: value is a cons.
    Consp,
    /// `stringp`: value is a string.
    Stringp,
    /// `listp`: value is nil or a cons.
    Listp,
}

/// Lower a type/nil predicate to `t`/`nil` via `select` (no branch, no deopt —
/// it matches the interpreter for any value by inspecting the tag bits).
fn lower_predicate(fb: &mut FunctionBuilder, kind: PredKind, a: ClifValue) -> ClifValue {
    let cond = match kind {
        PredKind::Null => fb.ins().icmp_imm(IntCC::Equal, a, Value::NIL.bits() as i64),
        PredKind::Consp => {
            let tag = fb.ins().band_imm(a, TAG_MASK as i64);
            fb.ins().icmp_imm(IntCC::Equal, tag, TAG_CONS as i64)
        }
        PredKind::Stringp => {
            let tag = fb.ins().band_imm(a, TAG_MASK as i64);
            fb.ins().icmp_imm(IntCC::Equal, tag, TAG_STRING as i64)
        }
        PredKind::Listp => {
            let is_nil = fb.ins().icmp_imm(IntCC::Equal, a, Value::NIL.bits() as i64);
            let tag = fb.ins().band_imm(a, TAG_MASK as i64);
            let is_cons = fb.ins().icmp_imm(IntCC::Equal, tag, TAG_CONS as i64);
            fb.ins().bor(is_nil, is_cons)
        }
    };
    let t = fb.ins().iconst(types::I64, Value::T.bits() as i64);
    let nil = fb.ins().iconst(types::I64, Value::NIL.bits() as i64);
    fb.ins().select(cond, t, nil)
}

/// Lower `car`/`cdr` (and the `-safe` variants) with exact interpreter parity:
/// a cons yields the loaded field; otherwise plain car/cdr yields nil for nil
/// and deopts for anything else (the interpreter signals
/// `wrong-type-argument`), while car-safe/cdr-safe yield nil for ANY non-cons
/// (total, no deopt). Non-allocating; reading a cons field needs no SATB
/// barrier (the barrier is on writes), and there is no GC safepoint here.
fn lower_car_cdr(
    fb: &mut FunctionBuilder,
    deopt: Option<Block>,
    is_cdr: bool,
    safe: bool,
    a: ClifValue,
) -> ClifValue {
    let tag = fb.ins().band_imm(a, TAG_MASK as i64);
    let is_cons = fb.ins().icmp_imm(IntCC::Equal, tag, TAG_CONS as i64);
    if !safe {
        let is_nil = fb.ins().icmp_imm(IntCC::Equal, a, Value::NIL.bits() as i64);
        let valid = fb.ins().bor(is_cons, is_nil);
        emit_guard(
            fb,
            deopt.expect("guarded car/cdr lowers with a deopt site"),
            valid,
        );
    }

    // Branch: cons -> load the field; nil -> nil. The result flows through a
    // fresh SSA variable (Cranelift inserts the phi at the merge).
    let res = fb.declare_var(types::I64);
    let cons_blk = fb.create_block();
    let nil_blk = fb.create_block();
    let merge = fb.create_block();
    fb.ins().brif(is_cons, cons_blk, &[], nil_blk, &[]);

    fb.switch_to_block(cons_blk);
    let ptr = fb.ins().band_imm(a, !(TAG_MASK as i64));
    let offset = if is_cdr {
        core::mem::offset_of!(ConsCell, cdr_or_next)
    } else {
        core::mem::offset_of!(ConsCell, car)
    };
    let field = fb
        .ins()
        .load(types::I64, MemFlags::trusted(), ptr, offset as i32);
    fb.def_var(res, field);
    fb.ins().jump(merge, &[]);

    fb.switch_to_block(nil_blk);
    if safe {
        // -safe variants: ANY non-cons yields nil.
        let nil = fb.ins().iconst(types::I64, Value::NIL.bits() as i64);
        fb.def_var(res, nil);
    } else {
        fb.def_var(res, a); // nil -> nil (a already holds nil, guarded above)
    }
    fb.ins().jump(merge, &[]);

    fb.switch_to_block(merge);
    fb.use_var(res)
}

/// Lower a no-argument straight-line leaf body. Thin wrapper over [`lower_leaf`]
/// kept for the existing call sites/tests.
pub fn lower_nullary_leaf(ops: &[Op], constants: &[Value]) -> Result<CompiledLeaf, CompileError> {
    lower_leaf(ops, constants, 0)
}

/// Get MIR value `v` as a RAW (untagged) fixnum i64 for arithmetic. If `cval_raw`
/// marks it already raw (a prior fixnum arithmetic result or fixnum constant in
/// this block), use it directly — no re-guard, no re-untag (the unboxing fast
/// path: chained fixnum arithmetic stays raw). Otherwise guard it is a fixnum
/// (deopt else) and untag.
fn mir_as_raw(
    fb: &mut FunctionBuilder,
    cval: &[Option<ClifValue>],
    cval_raw: &[bool],
    v: mir::MirValue,
    deopt: Block,
) -> Result<ClifValue, CompileError> {
    let i = v.0 as usize;
    let cv = cval[i].ok_or(CompileError::BadOperand)?;
    if cval_raw[i] {
        Ok(cv)
    } else {
        guard_fixnum(fb, deopt, cv, &HashSet::new());
        Ok(fb.ins().sshr_imm(cv, FIXNUM_SHIFT as i64))
    }
}

/// Get MIR value `v` as a TAGGED `Value` (for boundaries: returns, predicates,
/// car/cdr, cross-block block args). Retags a raw fixnum; passes a tagged value
/// through unchanged.
fn mir_as_tagged(
    fb: &mut FunctionBuilder,
    cval: &[Option<ClifValue>],
    cval_raw: &[bool],
    v: mir::MirValue,
) -> Result<ClifValue, CompileError> {
    let i = v.0 as usize;
    let cv = cval[i].ok_or(CompileError::BadOperand)?;
    if cval_raw[i] {
        Ok(retag_fixnum(fb, cv))
    } else {
        Ok(cv)
    }
}

/// Raw fixnum add/sub: operands and result are untagged i64 (no untag/retag), with
/// the interpreter's fixnum-range check (deopt on overflow). The unboxed analogue
/// of [`lower_fixnum_binop`].
fn raw_fixnum_addsub(
    fb: &mut FunctionBuilder,
    deopt: Block,
    is_sub: bool,
    av: ClifValue,
    bv: ClifValue,
) -> ClifValue {
    let res = if is_sub {
        fb.ins().isub(av, bv)
    } else {
        fb.ins().iadd(av, bv)
    };
    let ge_lo = fb.ins().icmp_imm(
        IntCC::SignedGreaterThanOrEqual,
        res,
        Value::MOST_NEGATIVE_FIXNUM,
    );
    let le_hi = fb
        .ins()
        .icmp_imm(IntCC::SignedLessThanOrEqual, res, Value::MOST_POSITIVE_FIXNUM);
    let in_range = fb.ins().band(ge_lo, le_hi);
    emit_guard(fb, deopt, in_range);
    res
}

/// Raw fixnum 1+/1-/negate: untagged in, untagged out, with the interpreter's
/// boundary check (deopt on the single out-of-range input). Unboxed analogue of
/// [`lower_fixnum_unop`].
fn raw_fixnum_unop(
    fb: &mut FunctionBuilder,
    deopt: Block,
    kind: UnaryKind,
    av: ClifValue,
) -> ClifValue {
    let bound = match kind {
        UnaryKind::Add1 => Value::MOST_POSITIVE_FIXNUM,
        UnaryKind::Sub1 | UnaryKind::Negate => Value::MOST_NEGATIVE_FIXNUM,
    };
    let in_range = fb.ins().icmp_imm(IntCC::NotEqual, av, bound);
    emit_guard(fb, deopt, in_range);
    match kind {
        UnaryKind::Add1 => fb.ins().iadd_imm(av, 1),
        UnaryKind::Sub1 => fb.ins().iadd_imm(av, -1),
        UnaryKind::Negate => fb.ins().ineg(av),
    }
}

/// **MIR Tier-2, Phase 4b (pure subset).** Lower a [`mir::MirFunction`] to a
/// [`CompiledLeaf`] by driving CLIF emission from the MIR instead of a bytecode
/// walk — the first proof that the bytecode→MIR→CLIF pipeline produces runnable
/// native code. Scoped to the *pure* op subset (arithmetic / comparisons /
/// type predicates / car-cdr / stack — no calls, cons, eq, or other shim-using
/// ops; those and precise-deopt framestates come in follow-up increments), so
/// no vmctx is needed and a failing guard can rerun the interpreter from the
/// start (sound: a pure body has no side effect before any guard).
///
/// Uses CLIF **block parameters** as the SSA phis — each MIR block becomes a
/// CLIF block whose params are its entry operand stack, and terminator edges
/// pass the live stack as block arguments. Behaviour-neutral: not wired into
/// the live compile pipeline; validated only by differential tests against the
/// interpreter.
pub(crate) fn lower_mir_pure(m: &mir::MirFunction) -> Result<CompiledLeaf, CompileError> {
    use mir::{BinKind, CmpKind, MirOp, MirTerm, PredKind as MP, UnaryKind as MU};

    let builder = JITBuilder::new(default_libcall_names())
        .map_err(|e| CompileError::Backend(BackendError::ModuleInit(e.to_string())))?;
    let mut module = JITModule::new(builder);
    let call_conv = module.target_config().default_call_conv;
    let ptr_ty = module.target_config().pointer_type();

    // ABI identical to lower_leaf: fn(vmctx, args, out) -> status.
    let mut sig = Signature::new(call_conv);
    sig.params.push(AbiParam::new(ptr_ty)); // vmctx (unused for pure)
    sig.params.push(AbiParam::new(ptr_ty)); // args
    sig.params.push(AbiParam::new(ptr_ty)); // out
    sig.returns.push(AbiParam::new(types::I64));

    let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig.clone());
    let mut fbctx = FunctionBuilderContext::new();
    {
        let mut fb = FunctionBuilder::new(&mut func, &mut fbctx);

        // One CLIF block per MIR block, params = the MIR block's params.
        let clif_blocks: Vec<Block> = m
            .blocks
            .iter()
            .map(|blk| {
                let cb = fb.create_block();
                for _ in &blk.params {
                    fb.append_block_param(cb, types::I64);
                }
                cb
            })
            .collect();

        // Map every MIR value to its CLIF value (filled in dominance order: a
        // single forward pass works because the MIR is SSA and block params
        // carry all cross-block values).
        let mut cval: Vec<Option<ClifValue>> = vec![None; m.value_types.len()];
        // Per-value form: true if `cval` holds an UNTAGGED raw fixnum (unboxing).
        // Fixnum arithmetic results + fixnum constants stay raw WITHIN a block (no
        // intermediate retag/untag/re-guard); boundaries (returns, predicates,
        // car/cdr, cross-block args) retag. Block params/args + non-fixnum values
        // are tagged (false) — no raw phis (the simpler, sound scope).
        let mut cval_raw: Vec<bool> = vec![false; m.value_types.len()];

        // Shared deopt landing block: pure bodies rerun the interpreter from the
        // start (STATUS_DEOPT), created lazily on the first guard.
        let mut deopt: Option<Block> = None;

        // Function-entry block: stash the out pointer + load args, jump into MIR
        // block 0 passing the args as block params.
        let entry = fb.create_block();
        fb.append_block_params_for_function_params(entry);
        fb.switch_to_block(entry);
        let args_ptr = fb.block_params(entry)[1];
        let out_ptr = fb.block_params(entry)[2];
        let arg_vals: Vec<BlockArg> = (0..m.arity)
            .map(|i| {
                let v = fb
                    .ins()
                    .load(types::I64, MemFlags::trusted(), args_ptr, (i * 8) as i32);
                BlockArg::Value(v)
            })
            .collect();
        fb.ins().jump(clif_blocks[0], &arg_vals);

        for (bi, blk) in m.blocks.iter().enumerate() {
            let cb = clif_blocks[bi];
            fb.switch_to_block(cb);
            // Bind this block's params to the CLIF block params.
            let bp = fb.block_params(cb).to_vec();
            for (p, &cv) in blk.params.iter().zip(bp.iter()) {
                cval[p.0 as usize] = Some(cv);
            }

            for inst in &blk.insts {
                let r = inst.result.0 as usize;
                match &inst.op {
                    MirOp::Arg(_) => {
                        // The param already holds the argument (bound above).
                    }
                    MirOp::Const(v) => {
                        if (v.bits() & FIXNUM_CHECK_MASK) == FIXNUM_CHECK_VALUE {
                            // Fixnum constant -> keep raw (untagged integer).
                            cval[r] =
                                Some(fb.ins().iconst(types::I64, (v.bits() as i64) >> FIXNUM_SHIFT));
                            cval_raw[r] = true;
                        } else {
                            cval[r] = Some(fb.ins().iconst(types::I64, v.bits() as i64));
                        }
                    }
                    MirOp::Bin(kind, a, b) => {
                        let is_sub = match kind {
                            BinKind::Add => false,
                            BinKind::Sub => true,
                            // Mul/Div/Rem/Max/Min need their own helpers / the
                            // shim path; deferred past the pure subset.
                            _ => return Err(CompileError::UnsupportedOp("mir-pure-binop")),
                        };
                        let d = *deopt.get_or_insert_with(|| fb.create_block());
                        let av = mir_as_raw(&mut fb, &cval, &cval_raw, *a, d)?;
                        let bv = mir_as_raw(&mut fb, &cval, &cval_raw, *b, d)?;
                        cval[r] = Some(raw_fixnum_addsub(&mut fb, d, is_sub, av, bv));
                        cval_raw[r] = true;
                    }
                    MirOp::Unary(kind, a) => {
                        let k = match kind {
                            MU::Add1 => UnaryKind::Add1,
                            MU::Sub1 => UnaryKind::Sub1,
                            MU::Negate => UnaryKind::Negate,
                        };
                        let d = *deopt.get_or_insert_with(|| fb.create_block());
                        let av = mir_as_raw(&mut fb, &cval, &cval_raw, *a, d)?;
                        cval[r] = Some(raw_fixnum_unop(&mut fb, d, k, av));
                        cval_raw[r] = true;
                    }
                    MirOp::Cmp(kind, a, b) => {
                        let cc = match kind {
                            CmpKind::NumEq => IntCC::Equal,
                            CmpKind::Lt => IntCC::SignedLessThan,
                            CmpKind::Gt => IntCC::SignedGreaterThan,
                            CmpKind::Le => IntCC::SignedLessThanOrEqual,
                            CmpKind::Ge => IntCC::SignedGreaterThanOrEqual,
                        };
                        let d = *deopt.get_or_insert_with(|| fb.create_block());
                        let av = mir_as_raw(&mut fb, &cval, &cval_raw, *a, d)?;
                        let bv = mir_as_raw(&mut fb, &cval, &cval_raw, *b, d)?;
                        let cond = fb.ins().icmp(cc, av, bv);
                        let t = fb.ins().iconst(types::I64, Value::T.bits() as i64);
                        let nil = fb.ins().iconst(types::I64, Value::NIL.bits() as i64);
                        cval[r] = Some(fb.ins().select(cond, t, nil));
                    }
                    MirOp::Pred(kind, a) => {
                        let k = match kind {
                            MP::Null | MP::Not => PredKind::Null,
                            MP::Consp => PredKind::Consp,
                            MP::Stringp => PredKind::Stringp,
                            MP::Listp => PredKind::Listp,
                            // Symbolp/Integerp/Numberp use shims; deferred.
                            _ => return Err(CompileError::UnsupportedOp("mir-pure-pred")),
                        };
                        let a = mir_as_tagged(&mut fb, &cval, &cval_raw, *a)?;
                        cval[r] = Some(lower_predicate(&mut fb, k, a));
                    }
                    MirOp::CarCdr { cdr, safe, arg } => {
                        let d = if *safe {
                            None
                        } else {
                            Some(*deopt.get_or_insert_with(|| fb.create_block()))
                        };
                        let a = mir_as_tagged(&mut fb, &cval, &cval_raw, *arg)?;
                        cval[r] = Some(lower_car_cdr(&mut fb, d, *cdr, *safe, a));
                    }
                    // Shim-using ops (calls / cons / eq / opaque): deferred.
                    MirOp::Eq(..) | MirOp::Cons(..) | MirOp::Opaque { .. } => {
                        return Err(CompileError::UnsupportedOp("mir-pure-shim-op"));
                    }
                }
            }

            // Terminator.
            match &blk.term {
                MirTerm::Return(v) => {
                    let rv = mir_as_tagged(&mut fb, &cval, &cval_raw, *v)?;
                    let out = out_ptr;
                    fb.ins().store(MemFlags::trusted(), rv, out, 0);
                    let ok = fb.ins().iconst(types::I64, STATUS_OK);
                    fb.ins().return_(&[ok]);
                }
                MirTerm::Goto { target, args } => {
                    // Cross-block args are tagged (block params are tagged).
                    let mut a: Vec<BlockArg> = Vec::with_capacity(args.len());
                    for v in args {
                        a.push(BlockArg::Value(mir_as_tagged(&mut fb, &cval, &cval_raw, *v)?));
                    }
                    fb.ins().jump(clif_blocks[target.0 as usize], &a);
                }
                MirTerm::Branch {
                    cond,
                    on_nil,
                    taken,
                    taken_args,
                    fallthrough,
                    fallthrough_args,
                    ..
                } => {
                    let c = mir_as_tagged(&mut fb, &cval, &cval_raw, *cond)?;
                    let is_nil = fb.ins().icmp_imm(IntCC::Equal, c, Value::NIL.bits() as i64);
                    let mut ta: Vec<BlockArg> = Vec::with_capacity(taken_args.len());
                    for v in taken_args {
                        ta.push(BlockArg::Value(mir_as_tagged(&mut fb, &cval, &cval_raw, *v)?));
                    }
                    let mut fa: Vec<BlockArg> = Vec::with_capacity(fallthrough_args.len());
                    for v in fallthrough_args {
                        fa.push(BlockArg::Value(mir_as_tagged(&mut fb, &cval, &cval_raw, *v)?));
                    }
                    let tb = clif_blocks[taken.0 as usize];
                    let fbk = clif_blocks[fallthrough.0 as usize];
                    // brif takes the `then` block when the condition is true.
                    if *on_nil {
                        fb.ins().brif(is_nil, tb, &ta, fbk, &fa);
                    } else {
                        fb.ins().brif(is_nil, fbk, &fa, tb, &ta);
                    }
                }
            }
        }

        if let Some(db) = deopt {
            fb.switch_to_block(db);
            let code = fb.ins().iconst(types::I64, STATUS_DEOPT);
            fb.ins().return_(&[code]);
        }

        fb.seal_all_blocks();
        fb.finalize();
    }

    let fid = module
        .declare_function("__neovm_mir_leaf", Linkage::Local, &sig)
        .map_err(|e| CompileError::Backend(BackendError::Define(e.to_string())))?;
    let mut ctx = module.make_context();
    ctx.func = func;
    module
        .define_function(fid, &mut ctx)
        .map_err(|e| CompileError::Backend(BackendError::Define(e.to_string())))?;
    module.clear_context(&mut ctx);
    module
        .finalize_definitions()
        .map_err(|e| CompileError::Backend(BackendError::Finalize(e.to_string())))?;
    let entry = module.get_finalized_function(fid);

    Ok(CompiledLeaf {
        arity: m.arity,
        required: m.arity,
        has_rest: false,
        has_binds: false,
        has_handlers: false,
        spec_slots: Box::from([]),
        deopt_spill: Box::from([]),
        deopt_meta: Box::new(DeoptCells {
            pc: core::cell::Cell::new(0),
            depth: core::cell::Cell::new(0),
            handlers: core::cell::Cell::new(0),
        }),
        entry,
        _module: module,
    })
}

/// Per-function runtime-call machinery: shim references plus the vmctx variable
/// and the scratch stack slots `Call` spills through. Present only when the body
/// re-enters the runtime (`Cons` / `Call`).
struct RtCtx {
    refs: RtRefs,
    /// The `*mut Context` function parameter, carried in an SSA variable so any
    /// block can read it.
    vmctx_var: Variable,
    /// Pointer type of the target (for `stack_addr`).
    ptr_ty: Type,
    /// Spill buffer for outgoing call arguments (max `Call` nargs in the body).
    call_args_slot: StackSlot,
    /// 8-byte result slot the call shim writes through.
    call_result_slot: StackSlot,
}

/// Callable references to every runtime shim, declared into one function.
struct RtRefs {
    gc_save: FuncRef,
    gc_push: FuncRef,
    gc_restore: FuncRef,
    cons: FuncRef,
    call: FuncRef,
    apply: FuncRef,
    eq_slow: FuncRef,
    symbolp_slow: FuncRef,
    varref: FuncRef,
    varset: FuncRef,
    varbind: FuncRef,
    unbind: FuncRef,
    backedge: FuncRef,
    save_current_buffer: FuncRef,
    save_excursion: FuncRef,
    save_restriction: FuncRef,
    unwind_protect: FuncRef,
    throw_flow: FuncRef,
    integerp_slow: FuncRef,
    numberp_slow: FuncRef,
    builtin1: FuncRef,
    builtin2: FuncRef,
    builtin3: FuncRef,
    push_cc: FuncRef,
    push_cc_raw: FuncRef,
    push_catch: FuncRef,
    pop_handler: FuncRef,
    match_handler: FuncRef,
    switch_lookup: FuncRef,
    switch_stale: FuncRef,
    list: FuncRef,
    builtin_slice: FuncRef,
    named_builtin: FuncRef,
    save_window_excursion: FuncRef,
    call_spec: FuncRef,
}

/// Declare the runtime-shim imports into `module`/`func` and return the callable
/// refs. The matching addresses are registered on the `JITBuilder` in
/// [`lower_leaf`] via `builder.symbol(...)`.
fn declare_rt_refs(
    module: &mut JITModule,
    func: &mut Function,
    call_conv: cranelift_codegen::isa::CallConv,
    ptr_ty: Type,
) -> Result<RtRefs, CompileError> {
    let i64t = types::I64;
    let mut sig_ret = Signature::new(call_conv); // () -> i64
    sig_ret.returns.push(AbiParam::new(i64t));
    let mut sig_arg = Signature::new(call_conv); // (i64) -> ()
    sig_arg.params.push(AbiParam::new(i64t));
    let mut sig_cons = Signature::new(call_conv); // (i64, i64) -> i64
    sig_cons.params.push(AbiParam::new(i64t));
    sig_cons.params.push(AbiParam::new(i64t));
    sig_cons.returns.push(AbiParam::new(i64t));
    // (vmctx, func_bits, args_ptr, nargs, out_ptr) -> status
    let mut sig_call = Signature::new(call_conv);
    sig_call.params.push(AbiParam::new(ptr_ty));
    sig_call.params.push(AbiParam::new(i64t));
    sig_call.params.push(AbiParam::new(ptr_ty));
    sig_call.params.push(AbiParam::new(i64t));
    sig_call.params.push(AbiParam::new(ptr_ty));
    sig_call.returns.push(AbiParam::new(i64t));
    // (vmctx, a, b) -> t/nil bits
    let mut sig_eq = Signature::new(call_conv);
    sig_eq.params.push(AbiParam::new(ptr_ty));
    sig_eq.params.push(AbiParam::new(i64t));
    sig_eq.params.push(AbiParam::new(i64t));
    sig_eq.returns.push(AbiParam::new(i64t));
    // (vmctx, v) -> t/nil bits
    let mut sig_symp = Signature::new(call_conv);
    sig_symp.params.push(AbiParam::new(ptr_ty));
    sig_symp.params.push(AbiParam::new(i64t));
    sig_symp.returns.push(AbiParam::new(i64t));

    let declare = |module: &mut JITModule, name: &str, sig: &Signature| {
        module
            .declare_function(name, Linkage::Import, sig)
            .map_err(|e| CompileError::Backend(BackendError::Define(e.to_string())))
    };

    let save_id = declare(module, "neovm_jit_gc_save", &sig_ret)?;
    let push_id = declare(module, "neovm_jit_gc_push", &sig_arg)?;
    let restore_id = declare(module, "neovm_jit_gc_restore", &sig_arg)?;
    let cons_id = declare(module, "neovm_jit_cons", &sig_cons)?;
    let call_id = declare(module, "neovm_jit_call", &sig_call)?;
    let apply_id = declare(module, "neovm_jit_apply", &sig_call)?;
    let eq_id = declare(module, "neovm_jit_eq_slow", &sig_eq)?;
    let symp_id = declare(module, "neovm_jit_symbolp_slow", &sig_symp)?;
    // (vmctx, sym_id, out_ptr) -> status
    let mut sig_varref = Signature::new(call_conv);
    sig_varref.params.push(AbiParam::new(ptr_ty));
    sig_varref.params.push(AbiParam::new(i64t));
    sig_varref.params.push(AbiParam::new(ptr_ty));
    sig_varref.returns.push(AbiParam::new(i64t));
    // (vmctx, sym_id, val) -> status
    let mut sig_varset = Signature::new(call_conv);
    sig_varset.params.push(AbiParam::new(ptr_ty));
    sig_varset.params.push(AbiParam::new(i64t));
    sig_varset.params.push(AbiParam::new(i64t));
    sig_varset.returns.push(AbiParam::new(i64t));
    let varref_id = declare(module, "neovm_jit_varref", &sig_varref)?;
    let varset_id = declare(module, "neovm_jit_varset", &sig_varset)?;
    // (vmctx, sym_id, val) -> ()  — specbind is infallible.
    let mut sig_varbind = Signature::new(call_conv);
    sig_varbind.params.push(AbiParam::new(ptr_ty));
    sig_varbind.params.push(AbiParam::new(i64t));
    sig_varbind.params.push(AbiParam::new(i64t));
    // (vmctx, n) -> ()  — unbind_to is infallible.
    let mut sig_unbind = Signature::new(call_conv);
    sig_unbind.params.push(AbiParam::new(ptr_ty));
    sig_unbind.params.push(AbiParam::new(i64t));
    let varbind_id = declare(module, "neovm_jit_varbind", &sig_varbind)?;
    let unbind_id = declare(module, "neovm_jit_unbind", &sig_unbind)?;
    // (vmctx) -> status
    let mut sig_backedge = Signature::new(call_conv);
    sig_backedge.params.push(AbiParam::new(ptr_ty));
    sig_backedge.returns.push(AbiParam::new(i64t));
    let backedge_id = declare(module, "neovm_jit_backedge", &sig_backedge)?;
    // (vmctx) -> ()  — the infallible Save* records.
    let mut sig_save = Signature::new(call_conv);
    sig_save.params.push(AbiParam::new(ptr_ty));
    let scb_id = declare(module, "neovm_jit_save_current_buffer", &sig_save)?;
    let sexc_id = declare(module, "neovm_jit_save_excursion", &sig_save)?;
    let sres_id = declare(module, "neovm_jit_save_restriction", &sig_save)?;
    // (vmctx, forms) -> ()  — unwind-protect record (infallible).
    let up_id = declare(module, "neovm_jit_unwind_protect", &sig_unbind)?;
    // (tag, value) -> ()  — context-free Flow stash.
    let mut sig_throw = Signature::new(call_conv);
    sig_throw.params.push(AbiParam::new(i64t));
    sig_throw.params.push(AbiParam::new(i64t));
    let throw_id = declare(module, "neovm_jit_throw", &sig_throw)?;
    // (v) -> t/nil bits  — context-free predicates.
    let mut sig_pred1 = Signature::new(call_conv);
    sig_pred1.params.push(AbiParam::new(i64t));
    sig_pred1.returns.push(AbiParam::new(i64t));
    let intp_id = declare(module, "neovm_jit_integerp_slow", &sig_pred1)?;
    let nump_id = declare(module, "neovm_jit_numberp_slow", &sig_pred1)?;
    // (vmctx, idx, a[, b[, c]], out_ptr) -> status — generic direct builtins.
    let mut sig_b1 = Signature::new(call_conv);
    sig_b1.params.push(AbiParam::new(ptr_ty));
    sig_b1.params.push(AbiParam::new(i64t));
    sig_b1.params.push(AbiParam::new(i64t));
    sig_b1.params.push(AbiParam::new(ptr_ty));
    sig_b1.returns.push(AbiParam::new(i64t));
    let mut sig_b2 = sig_b1.clone();
    sig_b2.params.insert(3, AbiParam::new(i64t));
    let mut sig_b3 = sig_b2.clone();
    sig_b3.params.insert(4, AbiParam::new(i64t));
    let b1_id = declare(module, "neovm_jit_builtin1", &sig_b1)?;
    let b2_id = declare(module, "neovm_jit_builtin2", &sig_b2)?;
    let b3_id = declare(module, "neovm_jit_builtin3", &sig_b3)?;
    // (vmctx, target, stack_len) -> ()  — condition-case push (infallible).
    let mut sig_pcc = Signature::new(call_conv);
    sig_pcc.params.push(AbiParam::new(ptr_ty));
    sig_pcc.params.push(AbiParam::new(i64t));
    sig_pcc.params.push(AbiParam::new(i64t));
    // (vmctx, target, stack_len, conditions/tag) -> ()
    let mut sig_pcc_raw = sig_pcc.clone();
    sig_pcc_raw.params.push(AbiParam::new(i64t));
    let pcc_id = declare(module, "neovm_jit_push_cc", &sig_pcc)?;
    let pcc_raw_id = declare(module, "neovm_jit_push_cc_raw", &sig_pcc_raw)?;
    let pcatch_id = declare(module, "neovm_jit_push_catch", &sig_pcc_raw)?;
    let pop_handler_id = declare(module, "neovm_jit_pop_handler", &sig_save)?;
    // (vmctx, ours, out_ptr) -> matched ordinal or -1.
    let match_id = declare(module, "neovm_jit_match_handler", &sig_varref)?;
    // (vmctx, dispatch, table) -> raw target addr / miss / stale.
    let switch_id = declare(module, "neovm_jit_switch", &sig_eq)?;
    // () -> ()  — stash the stale-table signal.
    let sig_void = Signature::new(call_conv);
    let switch_stale_id = declare(module, "neovm_jit_switch_stale", &sig_void)?;
    // (args_ptr, nargs) -> list bits  — infallible n-ary list builder.
    let mut sig_list = Signature::new(call_conv);
    sig_list.params.push(AbiParam::new(ptr_ty));
    sig_list.params.push(AbiParam::new(i64t));
    sig_list.returns.push(AbiParam::new(i64t));
    let list_id = declare(module, "neovm_jit_list", &sig_list)?;
    // (idx, args_ptr, nargs, out_ptr) -> status  — slice-shaped builtins.
    let mut sig_slice = Signature::new(call_conv);
    sig_slice.params.push(AbiParam::new(i64t));
    sig_slice.params.push(AbiParam::new(ptr_ty));
    sig_slice.params.push(AbiParam::new(i64t));
    sig_slice.params.push(AbiParam::new(ptr_ty));
    sig_slice.returns.push(AbiParam::new(i64t));
    let slice_id = declare(module, "neovm_jit_builtin_slice", &sig_slice)?;
    // (vmctx, variant, sym, args_ptr, nargs, out_ptr) -> status.
    let mut sig_named = Signature::new(call_conv);
    sig_named.params.push(AbiParam::new(ptr_ty));
    sig_named.params.push(AbiParam::new(i64t));
    sig_named.params.push(AbiParam::new(i64t));
    sig_named.params.push(AbiParam::new(ptr_ty));
    sig_named.params.push(AbiParam::new(i64t));
    sig_named.params.push(AbiParam::new(ptr_ty));
    sig_named.returns.push(AbiParam::new(i64t));
    let named_id = declare(module, "neovm_jit_named_builtin", &sig_named)?;
    // (vmctx, body, out_ptr) -> status.
    let swe_id = declare(module, "neovm_jit_save_window_excursion", &sig_varref)?;
    // (vmctx, sym, expected, slot_ptr, args_ptr, nargs, out_ptr) -> status.
    let mut sig_spec = Signature::new(call_conv);
    sig_spec.params.push(AbiParam::new(ptr_ty));
    sig_spec.params.push(AbiParam::new(i64t));
    sig_spec.params.push(AbiParam::new(i64t));
    sig_spec.params.push(AbiParam::new(i64t));
    sig_spec.params.push(AbiParam::new(ptr_ty));
    sig_spec.params.push(AbiParam::new(i64t));
    sig_spec.params.push(AbiParam::new(ptr_ty));
    sig_spec.returns.push(AbiParam::new(i64t));
    let call_spec_id = declare(module, "neovm_jit_call_spec", &sig_spec)?;

    Ok(RtRefs {
        gc_save: module.declare_func_in_func(save_id, func),
        gc_push: module.declare_func_in_func(push_id, func),
        gc_restore: module.declare_func_in_func(restore_id, func),
        cons: module.declare_func_in_func(cons_id, func),
        call: module.declare_func_in_func(call_id, func),
        apply: module.declare_func_in_func(apply_id, func),
        eq_slow: module.declare_func_in_func(eq_id, func),
        symbolp_slow: module.declare_func_in_func(symp_id, func),
        varref: module.declare_func_in_func(varref_id, func),
        varset: module.declare_func_in_func(varset_id, func),
        varbind: module.declare_func_in_func(varbind_id, func),
        unbind: module.declare_func_in_func(unbind_id, func),
        backedge: module.declare_func_in_func(backedge_id, func),
        save_current_buffer: module.declare_func_in_func(scb_id, func),
        save_excursion: module.declare_func_in_func(sexc_id, func),
        save_restriction: module.declare_func_in_func(sres_id, func),
        unwind_protect: module.declare_func_in_func(up_id, func),
        throw_flow: module.declare_func_in_func(throw_id, func),
        integerp_slow: module.declare_func_in_func(intp_id, func),
        numberp_slow: module.declare_func_in_func(nump_id, func),
        builtin1: module.declare_func_in_func(b1_id, func),
        builtin2: module.declare_func_in_func(b2_id, func),
        builtin3: module.declare_func_in_func(b3_id, func),
        push_cc: module.declare_func_in_func(pcc_id, func),
        push_cc_raw: module.declare_func_in_func(pcc_raw_id, func),
        push_catch: module.declare_func_in_func(pcatch_id, func),
        pop_handler: module.declare_func_in_func(pop_handler_id, func),
        match_handler: module.declare_func_in_func(match_id, func),
        switch_lookup: module.declare_func_in_func(switch_id, func),
        switch_stale: module.declare_func_in_func(switch_stale_id, func),
        list: module.declare_func_in_func(list_id, func),
        builtin_slice: module.declare_func_in_func(slice_id, func),
        named_builtin: module.declare_func_in_func(named_id, func),
        save_window_excursion: module.declare_func_in_func(swe_id, func),
        call_spec: module.declare_func_in_func(call_spec_id, func),
    })
}

/// The per-leaf cells a precise-deopt exit writes through before returning
/// [`STATUS_DEOPT_AT`]: the failing op's bytecode index, the live operand
/// stack depth (the values themselves go to the spill buffer), and the number
/// of condition frames this frame had registered at that point. `Cell` makes
/// the native interior writes legal; the mutator is single-threaded and the
/// values are consumed immediately after the native call returns.
pub(crate) struct DeoptCells {
    pub(crate) pc: core::cell::Cell<i64>,
    pub(crate) depth: core::cell::Cell<i64>,
    pub(crate) handlers: core::cell::Cell<i64>,
}

/// A precise-deopt exit block queued at a guard-emitting op: created (and
/// targeted by that op's guards) during lowering, filled after the bytecode
/// block terminates. Captures the op's index and the operand stack snapshot
/// from BEFORE the op popped its operands — the interpreter reruns the
/// failing op itself.
struct PendingDeopt {
    block: Block,
    pc: usize,
    handlers_len: usize,
    stack: Vec<ClifValue>,
}

/// Queue (and return) the precise-deopt block for the guard-emitting op at
/// bytecode index `pc`, capturing the pre-op operand stack.
fn deopt_site(
    fb: &mut FunctionBuilder,
    pc: usize,
    handlers_len: usize,
    stack: &[ClifValue],
    pending: &mut Vec<PendingDeopt>,
) -> Block {
    let block = fb.create_block();
    pending.push(PendingDeopt {
        block,
        pc,
        handlers_len,
        stack: stack.to_vec(),
    });
    block
}

/// Raw addresses of the leaf's deopt cells + spill buffer, baked into the
/// generated code as immediates (the owning Boxes are address-stable and
/// outlive every execution of the code).
#[derive(Clone, Copy)]
struct DeoptRefs {
    spill_base: i64,
    meta_pc: i64,
    meta_depth: i64,
    meta_handlers: i64,
}

/// Fill the precise-deopt blocks queued within one bytecode block: spill the
/// captured live stack, record pc/depth/handler-count, and return
/// [`STATUS_DEOPT_AT`].
fn emit_pending_deopts(fb: &mut FunctionBuilder, refs: DeoptRefs, pending: &mut Vec<PendingDeopt>) {
    for pd in pending.drain(..) {
        fb.switch_to_block(pd.block);
        fb.seal_block(pd.block);
        let base = fb.ins().iconst(types::I64, refs.spill_base);
        for (j, &v) in pd.stack.iter().enumerate() {
            fb.ins().store(MemFlags::trusted(), v, base, (j * 8) as i32);
        }
        let pc_v = fb.ins().iconst(types::I64, pd.pc as i64);
        let a = fb.ins().iconst(types::I64, refs.meta_pc);
        fb.ins().store(MemFlags::trusted(), pc_v, a, 0);
        let depth_v = fb.ins().iconst(types::I64, pd.stack.len() as i64);
        let a = fb.ins().iconst(types::I64, refs.meta_depth);
        fb.ins().store(MemFlags::trusted(), depth_v, a, 0);
        let h_v = fb.ins().iconst(types::I64, pd.handlers_len as i64);
        let a = fb.ins().iconst(types::I64, refs.meta_handlers);
        fb.ins().store(MemFlags::trusted(), h_v, a, 0);
        let code = fb.ins().iconst(types::I64, STATUS_DEOPT_AT);
        fb.ins().return_(&[code]);
    }
}

/// A handler-dispatch block queued at a `STATUS_SIGNAL` site inside a
/// protected extent: created (and branched to) at the site, filled after the
/// current bytecode block terminates by [`emit_pending_dispatches`]. Carries
/// the static handler list active at the site and the live operand-stack
/// snapshot (the site's SSA values dominate the dispatch block — it is their
/// only successor on the signal edge).
struct PendingDispatch {
    block: Block,
    handlers: Vec<HandlerStatic>,
    stack: Vec<ClifValue>,
}

/// Where a `STATUS_SIGNAL` site should branch: with no active handlers, the
/// shared signal-exit block (today's behavior); inside a protected extent, a
/// per-site dispatch block that will call the match shim.
fn signal_target_for_site(
    fb: &mut FunctionBuilder,
    signal_exit: &mut Option<Block>,
    handlers: &[HandlerStatic],
    pending: &mut Vec<PendingDispatch>,
    stack: &[ClifValue],
) -> Block {
    if handlers.is_empty() {
        return *signal_exit.get_or_insert_with(|| fb.create_block());
    }
    let block = fb.create_block();
    pending.push(PendingDispatch {
        block,
        handlers: handlers.to_vec(),
        stack: stack.to_vec(),
    });
    block
}

/// Fill the dispatch blocks queued by [`signal_target_for_site`] within one
/// bytecode block (called after its terminator, when the builder can switch
/// blocks). Each dispatch: root the live operand stack (the match shim can run
/// lisp — unwind-protect cleanups, handler-bind handlers, signal hooks — and
/// GC), call the match shim, and map the returned ordinal (`m` misses from the
/// innermost handler; -1 = propagate) onto the statically known handler
/// targets: re-materialize the handler's entry stack (the current model values
/// below its push depth + the error value the shim wrote through the result
/// slot) and jump to its block.
fn emit_pending_dispatches(
    fb: &mut FunctionBuilder,
    rt: &RtCtx,
    signal_exit: &mut Option<Block>,
    vars: &[Variable],
    block_for: &HashMap<usize, Block>,
    pending: &mut Vec<PendingDispatch>,
) -> Result<(), CompileError> {
    for pd in pending.drain(..) {
        fb.switch_to_block(pd.block);
        fb.seal_block(pd.block);
        let saved = if pd.stack.is_empty() {
            None
        } else {
            let c = fb.ins().call(rt.refs.gc_save, &[]);
            let s = fb.inst_results(c)[0];
            for &v in pd.stack.iter() {
                fb.ins().call(rt.refs.gc_push, &[v]);
            }
            Some(s)
        };
        let vmctx = fb.use_var(rt.vmctx_var);
        let ours = fb.ins().iconst(types::I64, pd.handlers.len() as i64);
        let out_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_result_slot, 0);
        let call = fb
            .ins()
            .call(rt.refs.match_handler, &[vmctx, ours, out_addr]);
        let idx = fb.inst_results(call)[0];
        if let Some(s) = saved {
            fb.ins().call(rt.refs.gc_restore, &[s]);
        }
        // Compare chain over the (small) static handler list: shim ordinal
        // m counts misses from the top, so m maps to handlers[len-1-m].
        let k = pd.handlers.len();
        for m in 0..k {
            let (target, push_depth) = pd.handlers[k - 1 - m];
            if push_depth > pd.stack.len() {
                // The byte-compiler keeps the operand stack at or above the
                // protected base inside the extent; anything else is exotic —
                // bail to the interpreter.
                return Err(CompileError::UnsupportedOp("handler-depth"));
            }
            let hit = fb.create_block();
            let next = fb.create_block();
            let is_m = fb.ins().icmp_imm(IntCC::Equal, idx, m as i64);
            fb.ins().brif(is_m, hit, &[], next, &[]);
            fb.switch_to_block(hit);
            fb.seal_block(hit);
            for (j, &v) in pd.stack.iter().take(push_depth).enumerate() {
                fb.def_var(vars[j], v);
            }
            let err = fb.ins().stack_load(types::I64, rt.call_result_slot, 0);
            fb.def_var(vars[push_depth], err);
            fb.ins().jump(block_for[&target], &[]);
            fb.switch_to_block(next);
            fb.seal_block(next);
        }
        let se = *signal_exit.get_or_insert_with(|| fb.create_block());
        fb.ins().jump(se, &[]);
    }
    Ok(())
}

/// Lower one non-control-flow opcode, updating the compile-time operand `stack`
/// (the live CLIF SSA values within the current basic block). Terminators
/// (`Return`/`Goto`/`GotoIf*`) are handled by the block lowerer before this.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn lower_simple_op(
    fb: &mut FunctionBuilder,
    pc: usize,
    deopt_sites: &mut Vec<PendingDeopt>,
    signal_exit: &mut Option<Block>,
    constants: &[Value],
    stack: &mut Vec<ClifValue>,
    rt: Option<&RtCtx>,
    handlers: &[HandlerStatic],
    pending: &mut Vec<PendingDispatch>,
    spec: Option<(u32, u64, i64)>,
    op: &Op,
    // Cross-block known-fixnum operand values at this block (seeded by
    // `lower_leaf_full` from `compute_known_fixnum_slots`); `guard_fixnum` elides
    // guards for members.
    known: &HashSet<ClifValue>,
) -> Result<(), CompileError> {
    match op {
        Op::Constant(idx) => {
            let v = constants
                .get(*idx as usize)
                .ok_or(CompileError::BadOperand)?;
            stack.push(fb.ins().iconst(types::I64, v.bits() as i64));
        }
        Op::Nil => stack.push(fb.ins().iconst(types::I64, Value::NIL.bits() as i64)),
        Op::True => stack.push(fb.ins().iconst(types::I64, Value::T.bits() as i64)),
        Op::Pop => {
            stack.pop().ok_or(CompileError::StackUnderflow)?;
        }
        Op::Dup => {
            let top = *stack.last().ok_or(CompileError::StackUnderflow)?;
            stack.push(top);
        }
        Op::StackRef(n) => {
            // 0 = top of stack, 1 = one below, ...
            let n = *n as usize;
            let idx = stack
                .len()
                .checked_sub(1 + n)
                .ok_or(CompileError::StackUnderflow)?;
            stack.push(stack[idx]);
        }
        Op::StackSet(n) => {
            // Assign TOS into the slot N below TOS, then pop TOS (N = 0 == pop).
            let n = *n as usize;
            let top = stack.pop().ok_or(CompileError::StackUnderflow)?;
            if n != 0 {
                let idx = stack
                    .len()
                    .checked_sub(n)
                    .ok_or(CompileError::StackUnderflow)?;
                stack[idx] = top;
            }
        }
        Op::DiscardN(raw) => {
            // Low 7 bits: count to discard. High bit: keep TOS in the last kept
            // slot before discarding. Pure operand-stack manipulation.
            let preserve_tos = (*raw & 0x80) != 0;
            let n = (*raw & 0x7F) as usize;
            if n != 0 {
                let len = stack.len();
                if preserve_tos {
                    let target = len.checked_sub(1 + n).ok_or(CompileError::StackUnderflow)?;
                    stack[target] = stack[len - 1];
                } else if n > len {
                    return Err(CompileError::StackUnderflow);
                }
                stack.truncate(len - n);
            }
        }
        Op::Add | Op::Sub => {
            let dsite = deopt_site(fb, pc, handlers.len(), stack, deopt_sites);
            let b = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let is_sub = matches!(op, Op::Sub);
            stack.push(lower_fixnum_binop(fb, dsite, is_sub, a, b, known));
        }
        Op::Mul => {
            let dsite = deopt_site(fb, pc, handlers.len(), stack, deopt_sites);
            let b = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            stack.push(lower_fixnum_mul(fb, dsite, a, b, known));
        }
        Op::Div | Op::Rem => {
            let dsite = deopt_site(fb, pc, handlers.len(), stack, deopt_sites);
            let b = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let is_rem = matches!(op, Op::Rem);
            stack.push(lower_fixnum_divrem(fb, dsite, is_rem, a, b, known));
        }
        Op::Eq => {
            // Bit-equal -> t natively; differing bits -> the read-only slow-path
            // shim (only symbols-with-pos can make differing bits eq).
            let rt = rt.ok_or(CompileError::UnsupportedOp("eq"))?;
            let b = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let res = fb.declare_var(types::I64);
            let fast = fb.create_block();
            let slow = fb.create_block();
            let merge = fb.create_block();
            let same = fb.ins().icmp(IntCC::Equal, a, b);
            fb.ins().brif(same, fast, &[], slow, &[]);

            fb.switch_to_block(fast);
            fb.seal_block(fast);
            let t = fb.ins().iconst(types::I64, Value::T.bits() as i64);
            fb.def_var(res, t);
            fb.ins().jump(merge, &[]);

            fb.switch_to_block(slow);
            fb.seal_block(slow);
            let vmctx = fb.use_var(rt.vmctx_var);
            let call = fb.ins().call(rt.refs.eq_slow, &[vmctx, a, b]);
            let slow_res = fb.inst_results(call)[0];
            fb.def_var(res, slow_res);
            fb.ins().jump(merge, &[]);

            fb.switch_to_block(merge);
            fb.seal_block(merge);
            stack.push(fb.use_var(res));
        }
        Op::Symbolp => {
            // Symbol tag -> t natively (nil/t are symbols); otherwise the
            // read-only slow-path shim (symbol-with-pos while enabled).
            let rt = rt.ok_or(CompileError::UnsupportedOp("symbolp"))?;
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let res = fb.declare_var(types::I64);
            let fast = fb.create_block();
            let slow = fb.create_block();
            let merge = fb.create_block();
            let tag = fb.ins().band_imm(a, TAG_MASK as i64);
            let is_sym = fb.ins().icmp_imm(IntCC::Equal, tag, TAG_SYMBOL as i64);
            fb.ins().brif(is_sym, fast, &[], slow, &[]);

            fb.switch_to_block(fast);
            fb.seal_block(fast);
            let t = fb.ins().iconst(types::I64, Value::T.bits() as i64);
            fb.def_var(res, t);
            fb.ins().jump(merge, &[]);

            fb.switch_to_block(slow);
            fb.seal_block(slow);
            let vmctx = fb.use_var(rt.vmctx_var);
            let call = fb.ins().call(rt.refs.symbolp_slow, &[vmctx, a]);
            let slow_res = fb.inst_results(call)[0];
            fb.def_var(res, slow_res);
            fb.ins().jump(merge, &[]);

            fb.switch_to_block(merge);
            fb.seal_block(merge);
            stack.push(fb.use_var(res));
        }
        Op::Add1 | Op::Sub1 | Op::Negate => {
            let dsite = deopt_site(fb, pc, handlers.len(), stack, deopt_sites);
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let kind = match op {
                Op::Add1 => UnaryKind::Add1,
                Op::Sub1 => UnaryKind::Sub1,
                Op::Negate => UnaryKind::Negate,
                _ => unreachable!("matched Add1/Sub1/Negate above"),
            };
            stack.push(lower_fixnum_unop(fb, dsite, kind, a, known));
        }
        Op::Eqlsign | Op::Lss | Op::Gtr | Op::Leq | Op::Geq => {
            let dsite = deopt_site(fb, pc, handlers.len(), stack, deopt_sites);
            let b = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let cc = match op {
                Op::Eqlsign => IntCC::Equal,
                Op::Lss => IntCC::SignedLessThan,
                Op::Gtr => IntCC::SignedGreaterThan,
                Op::Leq => IntCC::SignedLessThanOrEqual,
                Op::Geq => IntCC::SignedGreaterThanOrEqual,
                _ => unreachable!("matched comparison ops above"),
            };
            stack.push(lower_fixnum_compare(fb, dsite, cc, a, b, known));
        }
        Op::Null | Op::Not | Op::Consp | Op::Stringp | Op::Listp => {
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let kind = match op {
                Op::Null | Op::Not => PredKind::Null,
                Op::Consp => PredKind::Consp,
                Op::Stringp => PredKind::Stringp,
                Op::Listp => PredKind::Listp,
                _ => unreachable!("matched predicate ops above"),
            };
            stack.push(lower_predicate(fb, kind, a));
        }
        Op::Car | Op::Cdr => {
            let dsite = deopt_site(fb, pc, handlers.len(), stack, deopt_sites);
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let is_cdr = matches!(op, Op::Cdr);
            stack.push(lower_car_cdr(fb, Some(dsite), is_cdr, false, a));
        }
        Op::CarSafe | Op::CdrSafe => {
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let is_cdr = matches!(op, Op::CdrSafe);
            stack.push(lower_car_cdr(fb, None, is_cdr, true, a));
        }
        Op::Max | Op::Min => {
            // Both fixnum -> keep the original tagged operand selected by the
            // untagged comparison (exact interpreter parity: fixnum_ge ->
            // a-else-b for max, fixnum_le -> a-else-b for min); otherwise deopt
            // to the interpreter's number-coercing builtin.
            let dsite = deopt_site(fb, pc, handlers.len(), stack, deopt_sites);
            let b = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            guard_fixnum(fb, dsite, a, known);
            guard_fixnum(fb, dsite, b, known);
            let av = fb.ins().sshr_imm(a, FIXNUM_SHIFT as i64);
            let bv = fb.ins().sshr_imm(b, FIXNUM_SHIFT as i64);
            let cc = if matches!(op, Op::Max) {
                IntCC::SignedGreaterThanOrEqual
            } else {
                IntCC::SignedLessThanOrEqual
            };
            let keep_a = fb.ins().icmp(cc, av, bv);
            stack.push(fb.ins().select(keep_a, a, b));
        }
        Op::Integerp | Op::Numberp => {
            // Fixnum tag -> t natively; anything else (bignum/float/non-number)
            // through the context-free slow shim.
            let rt = rt.ok_or(CompileError::UnsupportedOp("predicate"))?;
            let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let shim = if matches!(op, Op::Integerp) {
                rt.refs.integerp_slow
            } else {
                rt.refs.numberp_slow
            };
            let res = fb.declare_var(types::I64);
            let fast = fb.create_block();
            let slow = fb.create_block();
            let merge = fb.create_block();
            let tagbits = fb.ins().band_imm(a, FIXNUM_CHECK_MASK as i64);
            let is_fix = fb
                .ins()
                .icmp_imm(IntCC::Equal, tagbits, FIXNUM_CHECK_VALUE as i64);
            fb.ins().brif(is_fix, fast, &[], slow, &[]);

            fb.switch_to_block(fast);
            fb.seal_block(fast);
            let t = fb.ins().iconst(types::I64, Value::T.bits() as i64);
            fb.def_var(res, t);
            fb.ins().jump(merge, &[]);

            fb.switch_to_block(slow);
            fb.seal_block(slow);
            let call = fb.ins().call(shim, &[a]);
            let slow_res = fb.inst_results(call)[0];
            fb.def_var(res, slow_res);
            fb.ins().jump(merge, &[]);

            fb.switch_to_block(merge);
            fb.seal_block(merge);
            stack.push(fb.use_var(res));
        }
        Op::VarRef(idx) => {
            // Read through the runtime's variable machinery (buffer-locals,
            // redirects); can signal void-variable. Reads are idempotent, so
            // this neither poisons nor guards.
            let rt = rt.ok_or(CompileError::UnsupportedOp("variable"))?;
            let sym = const_sym_id(constants, *idx)?;
            // Root live stack values: variable access may allocate.
            let saved = if stack.is_empty() {
                None
            } else {
                let c = fb.ins().call(rt.refs.gc_save, &[]);
                let s = fb.inst_results(c)[0];
                for &v in stack.iter() {
                    fb.ins().call(rt.refs.gc_push, &[v]);
                }
                Some(s)
            };
            let vmctx = fb.use_var(rt.vmctx_var);
            let sym_v = fb.ins().iconst(types::I64, sym as i64);
            let out_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_result_slot, 0);
            let call = fb.ins().call(rt.refs.varref, &[vmctx, sym_v, out_addr]);
            let status = fb.inst_results(call)[0];
            if let Some(s) = saved {
                fb.ins().call(rt.refs.gc_restore, &[s]);
            }
            let se = signal_target_for_site(fb, signal_exit, handlers, pending, stack);
            let cont = fb.create_block();
            let ok = fb.ins().icmp_imm(IntCC::Equal, status, STATUS_OK);
            fb.ins().brif(ok, cont, &[], se, &[]);
            fb.switch_to_block(cont);
            fb.seal_block(cont);
            let result = fb.ins().stack_load(types::I64, rt.call_result_slot, 0);
            stack.push(result);
        }
        Op::VarSet(idx) => {
            // Assign through the runtime (may run variable watchers — arbitrary
            // lisp — and signal). A side effect: poisons later guards.
            let rt = rt.ok_or(CompileError::UnsupportedOp("variable"))?;
            let sym = const_sym_id(constants, *idx)?;
            let val = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let saved = if stack.is_empty() {
                None
            } else {
                let c = fb.ins().call(rt.refs.gc_save, &[]);
                let s = fb.inst_results(c)[0];
                for &v in stack.iter() {
                    fb.ins().call(rt.refs.gc_push, &[v]);
                }
                Some(s)
            };
            let vmctx = fb.use_var(rt.vmctx_var);
            let sym_v = fb.ins().iconst(types::I64, sym as i64);
            let call = fb.ins().call(rt.refs.varset, &[vmctx, sym_v, val]);
            let status = fb.inst_results(call)[0];
            if let Some(s) = saved {
                fb.ins().call(rt.refs.gc_restore, &[s]);
            }
            let se = signal_target_for_site(fb, signal_exit, handlers, pending, stack);
            let cont = fb.create_block();
            let ok = fb.ins().icmp_imm(IntCC::Equal, status, STATUS_OK);
            fb.ins().brif(ok, cont, &[], se, &[]);
            fb.switch_to_block(cont);
            fb.seal_block(cont);
        }
        Op::Call(n) | Op::Apply(n) => {
            // `rt` is always present here (`needs_rt` includes Call/Apply).
            // Stack: [func a1 .. aN] -> [result], mirroring the interpreter's
            // Op::Call / Op::Apply; the two differ only in which shim runs
            // (apply spreads its last argument inside the runtime).
            let rt = rt.ok_or(CompileError::UnsupportedOp("call"))?;
            let shim = if matches!(op, Op::Apply(_)) {
                rt.refs.apply
            } else {
                rt.refs.call
            };
            let n = *n as usize;
            if stack.len() < n + 1 {
                return Err(CompileError::StackUnderflow);
            }
            let args_at = stack.len() - n;
            // Spill the args into the call buffer for the shim.
            for (i, &v) in stack[args_at..].iter().enumerate() {
                fb.ins().stack_store(v, rt.call_args_slot, (i * 8) as i32);
            }
            let func_val = stack[args_at - 1];
            stack.truncate(args_at - 1);
            // Root every value that stays live across the call (the callee +
            // args are rooted by the shim; the constants are rooted by the
            // dispatch seam via the executing function).
            let saved = if stack.is_empty() {
                None
            } else {
                let c = fb.ins().call(rt.refs.gc_save, &[]);
                let s = fb.inst_results(c)[0];
                for &v in stack.iter() {
                    fb.ins().call(rt.refs.gc_push, &[v]);
                }
                Some(s)
            };
            let vmctx = fb.use_var(rt.vmctx_var);
            let args_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_args_slot, 0);
            let out_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_result_slot, 0);
            let n_val = fb.ins().iconst(types::I64, n as i64);
            // Speculated direct call: the callee slot holds a constant symbol
            // whose compile-time binding was a bytecode object — call through
            // the epoch-validated spec shim instead (Apply never speculates).
            let spec = spec.filter(|_| matches!(op, Op::Call(_)));
            let call = if let Some((sym, expected, slot_ptr)) = spec {
                let sym_v = fb.ins().iconst(types::I64, sym as i64);
                let exp_v = fb.ins().iconst(types::I64, expected as i64);
                let slot_v = fb.ins().iconst(types::I64, slot_ptr);
                fb.ins().call(
                    rt.refs.call_spec,
                    &[vmctx, sym_v, exp_v, slot_v, args_addr, n_val, out_addr],
                )
            } else {
                fb.ins()
                    .call(shim, &[vmctx, func_val, args_addr, n_val, out_addr])
            };
            let status = fb.inst_results(call)[0];
            if let Some(s) = saved {
                fb.ins().call(rt.refs.gc_restore, &[s]);
            }
            // STATUS_OK -> continue with the result; anything else from the call
            // shim is STATUS_SIGNAL -> propagate via the shared signal block.
            let se = signal_target_for_site(fb, signal_exit, handlers, pending, stack);
            let cont = fb.create_block();
            let ok = fb.ins().icmp_imm(IntCC::Equal, status, STATUS_OK);
            fb.ins().brif(ok, cont, &[], se, &[]);
            fb.switch_to_block(cont);
            fb.seal_block(cont);
            let result = fb.ins().stack_load(types::I64, rt.call_result_slot, 0);
            stack.push(result);
        }
        Op::Cons => {
            // `rt` is always present here: analyze_cfg accepts Cons only when the
            // function declares the shims (see `needs_rt` in lower_leaf).
            let rt = rt.ok_or(CompileError::UnsupportedOp("cons"))?;
            let cdr = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let car = stack.pop().ok_or(CompileError::StackUnderflow)?;
            // The cons shim roots car+cdr across the allocation; we must root any
            // *other* live operand-stack values too (none in the common case).
            let result = if stack.is_empty() {
                let call = fb.ins().call(rt.refs.cons, &[car, cdr]);
                fb.inst_results(call)[0]
            } else {
                let saved = {
                    let c = fb.ins().call(rt.refs.gc_save, &[]);
                    fb.inst_results(c)[0]
                };
                for &v in stack.iter() {
                    fb.ins().call(rt.refs.gc_push, &[v]);
                }
                let call = fb.ins().call(rt.refs.cons, &[car, cdr]);
                let r = fb.inst_results(call)[0];
                fb.ins().call(rt.refs.gc_restore, &[saved]);
                r
            };
            stack.push(result);
        }
        Op::VarBind(idx) => {
            // GNU Bvarbind: specbind(sym, POP) — infallible, no status branch.
            // The shim records the pre-bind specpdl depth; the frame unwind in
            // CompiledLeaf::call restores the entry base on every exit.
            let rt = rt.ok_or(CompileError::UnsupportedOp("variable"))?;
            let sym = const_sym_id(constants, *idx)?;
            let val = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let vmctx = fb.use_var(rt.vmctx_var);
            let sym_v = fb.ins().iconst(types::I64, sym as i64);
            // The shim runs variable watchers (arbitrary lisp -> GC). `val` is
            // rooted by `specbind` inside the shim, but the remaining operand
            // stack lives only in Cranelift registers — root it across the call
            // (mirrors VarRef/VarSet). This is an exact-root GC: a live Value
            // unrooted across a GC-capable call is a use-after-free.
            let saved = if stack.is_empty() {
                None
            } else {
                let c = fb.ins().call(rt.refs.gc_save, &[]);
                let s = fb.inst_results(c)[0];
                for &v in stack.iter() {
                    fb.ins().call(rt.refs.gc_push, &[v]);
                }
                Some(s)
            };
            fb.ins().call(rt.refs.varbind, &[vmctx, sym_v, val]);
            if let Some(s) = saved {
                fb.ins().call(rt.refs.gc_restore, &[s]);
            }
        }
        Op::Unbind(n) => {
            // Unbind the N most recent dynamic bindings — infallible; the
            // static bind-depth analysis guarantees balance.
            let rt = rt.ok_or(CompileError::UnsupportedOp("variable"))?;
            let vmctx = fb.use_var(rt.vmctx_var);
            let n_v = fb.ins().iconst(types::I64, *n as i64);
            // The shim runs unwind-protect cleanups (arbitrary lisp -> GC); root
            // the whole live operand stack across the call.
            let saved = if stack.is_empty() {
                None
            } else {
                let c = fb.ins().call(rt.refs.gc_save, &[]);
                let s = fb.inst_results(c)[0];
                for &v in stack.iter() {
                    fb.ins().call(rt.refs.gc_push, &[v]);
                }
                Some(s)
            };
            fb.ins().call(rt.refs.unbind, &[vmctx, n_v]);
            if let Some(s) = saved {
                fb.ins().call(rt.refs.gc_restore, &[s]);
            }
        }
        Op::SaveCurrentBuffer | Op::SaveExcursion | Op::SaveRestriction => {
            // Infallible specpdl records (the interpreter arms mirrored in the
            // shims); restored by the matching Unbind or the frame unwind.
            let rt = rt.ok_or(CompileError::UnsupportedOp("variable"))?;
            let shim = match op {
                Op::SaveCurrentBuffer => rt.refs.save_current_buffer,
                Op::SaveExcursion => rt.refs.save_excursion,
                Op::SaveRestriction => rt.refs.save_restriction,
                _ => unreachable!("matched Save* above"),
            };
            let vmctx = fb.use_var(rt.vmctx_var);
            fb.ins().call(shim, &[vmctx]);
        }
        Op::UnwindProtectPop => {
            // Pop the cleanup form and register the unwind-protect record
            // (infallible; the cleanup runs via the shared unbind machinery).
            let rt = rt.ok_or(CompileError::UnsupportedOp("variable"))?;
            let forms = stack.pop().ok_or(CompileError::StackUnderflow)?;
            let vmctx = fb.use_var(rt.vmctx_var);
            fb.ins().call(rt.refs.unwind_protect, &[vmctx, forms]);
        }
        Op::SaveWindowExcursion => {
            // Evaluate the popped body under a window-configuration
            // save/restore via the shim (interpreter arm parity).
            let rt = rt.ok_or(CompileError::UnsupportedOp("builtin"))?;
            let body = stack.pop().ok_or(CompileError::StackUnderflow)?;
            // Root remaining live values: the body runs arbitrary lisp.
            let saved = if stack.is_empty() {
                None
            } else {
                let c = fb.ins().call(rt.refs.gc_save, &[]);
                let s = fb.inst_results(c)[0];
                for &v in stack.iter() {
                    fb.ins().call(rt.refs.gc_push, &[v]);
                }
                Some(s)
            };
            let vmctx = fb.use_var(rt.vmctx_var);
            let out_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_result_slot, 0);
            let call = fb
                .ins()
                .call(rt.refs.save_window_excursion, &[vmctx, body, out_addr]);
            let status = fb.inst_results(call)[0];
            if let Some(s) = saved {
                fb.ins().call(rt.refs.gc_restore, &[s]);
            }
            let se = signal_target_for_site(fb, signal_exit, handlers, pending, stack);
            let cont = fb.create_block();
            let ok = fb.ins().icmp_imm(IntCC::Equal, status, STATUS_OK);
            fb.ins().brif(ok, cont, &[], se, &[]);
            fb.switch_to_block(cont);
            fb.seal_block(cont);
            let result = fb.ins().stack_load(types::I64, rt.call_result_slot, 0);
            stack.push(result);
        }
        Op::CallBuiltin(..) | Op::CallBuiltinSym(..) | Op::Aset => {
            // Named-builtin escape hatch + aset: route through the
            // Vm::*_for_jit helpers mirroring the interpreter arms
            // (override-aware / advice-bypassing / writeback / quit poll).
            let rt = rt.ok_or(CompileError::UnsupportedOp("builtin"))?;
            let (variant, sym, nargs): (i64, u32, usize) = match op {
                Op::CallBuiltin(name_idx, n) => {
                    (0, const_sym_id(constants, *name_idx)?, *n as usize)
                }
                Op::CallBuiltinSym(sym, n) => (1, sym.0, *n as usize),
                Op::Aset => (2, 0, 3),
                _ => unreachable!("matched named-builtin ops above"),
            };
            if stack.len() < nargs {
                return Err(CompileError::StackUnderflow);
            }
            let at = stack.len() - nargs;
            for (i, &v) in stack[at..].iter().enumerate() {
                fb.ins().stack_store(v, rt.call_args_slot, (i * 8) as i32);
            }
            stack.truncate(at);
            // Root remaining live values (arbitrary lisp may run; the shim
            // roots the operands themselves).
            let saved = if stack.is_empty() {
                None
            } else {
                let c = fb.ins().call(rt.refs.gc_save, &[]);
                let s = fb.inst_results(c)[0];
                for &v in stack.iter() {
                    fb.ins().call(rt.refs.gc_push, &[v]);
                }
                Some(s)
            };
            let vmctx = fb.use_var(rt.vmctx_var);
            let variant_v = fb.ins().iconst(types::I64, variant);
            let sym_v = fb.ins().iconst(types::I64, sym as i64);
            let args_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_args_slot, 0);
            let n_val = fb.ins().iconst(types::I64, nargs as i64);
            let out_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_result_slot, 0);
            let call = fb.ins().call(
                rt.refs.named_builtin,
                &[vmctx, variant_v, sym_v, args_addr, n_val, out_addr],
            );
            let status = fb.inst_results(call)[0];
            if let Some(s) = saved {
                fb.ins().call(rt.refs.gc_restore, &[s]);
            }
            let se = signal_target_for_site(fb, signal_exit, handlers, pending, stack);
            let cont = fb.create_block();
            let ok = fb.ins().icmp_imm(IntCC::Equal, status, STATUS_OK);
            fb.ins().brif(ok, cont, &[], se, &[]);
            fb.switch_to_block(cont);
            fb.seal_block(cont);
            let result = fb.ins().stack_load(types::I64, rt.call_result_slot, 0);
            stack.push(result);
        }
        Op::List(n) => {
            // N-ary list builder — infallible allocation through the shim
            // (the interpreter's Value::list_from_slice on the stack slice).
            let rt = rt.ok_or(CompileError::UnsupportedOp("builtin"))?;
            let n = *n as usize;
            if stack.len() < n {
                return Err(CompileError::StackUnderflow);
            }
            let at = stack.len() - n;
            for (i, &v) in stack[at..].iter().enumerate() {
                fb.ins().stack_store(v, rt.call_args_slot, (i * 8) as i32);
            }
            stack.truncate(at);
            // Root remaining live values (the allocation may GC; the shim
            // roots the operands themselves).
            let saved = if stack.is_empty() {
                None
            } else {
                let c = fb.ins().call(rt.refs.gc_save, &[]);
                let s = fb.inst_results(c)[0];
                for &v in stack.iter() {
                    fb.ins().call(rt.refs.gc_push, &[v]);
                }
                Some(s)
            };
            let args_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_args_slot, 0);
            let n_val = fb.ins().iconst(types::I64, n as i64);
            let call = fb.ins().call(rt.refs.list, &[args_addr, n_val]);
            let result = fb.inst_results(call)[0];
            if let Some(s) = saved {
                fb.ins().call(rt.refs.gc_restore, &[s]);
            }
            stack.push(result);
        }
        other => {
            // Slice-shaped builtins (nconc/concat/substring): spill the
            // operands and call the generic slice shim with the table index
            // baked in — the SAME builtins::*_slice function the interpreter
            // arm calls.
            if let Some((nargs, idx)) = slice_builtin_spec(other) {
                let rt = rt.ok_or(CompileError::UnsupportedOp("builtin"))?;
                if stack.len() < nargs {
                    return Err(CompileError::StackUnderflow);
                }
                let at = stack.len() - nargs;
                for (i, &v) in stack[at..].iter().enumerate() {
                    fb.ins().stack_store(v, rt.call_args_slot, (i * 8) as i32);
                }
                stack.truncate(at);
                let saved = if stack.is_empty() {
                    None
                } else {
                    let c = fb.ins().call(rt.refs.gc_save, &[]);
                    let s = fb.inst_results(c)[0];
                    for &v in stack.iter() {
                        fb.ins().call(rt.refs.gc_push, &[v]);
                    }
                    Some(s)
                };
                let idx_v = fb.ins().iconst(types::I64, idx as i64);
                let args_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_args_slot, 0);
                let n_val = fb.ins().iconst(types::I64, nargs as i64);
                let out_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_result_slot, 0);
                let call = fb
                    .ins()
                    .call(rt.refs.builtin_slice, &[idx_v, args_addr, n_val, out_addr]);
                let status = fb.inst_results(call)[0];
                if let Some(s) = saved {
                    fb.ins().call(rt.refs.gc_restore, &[s]);
                }
                let se = signal_target_for_site(fb, signal_exit, handlers, pending, stack);
                let cont = fb.create_block();
                let ok = fb.ins().icmp_imm(IntCC::Equal, status, STATUS_OK);
                fb.ins().brif(ok, cont, &[], se, &[]);
                fb.switch_to_block(cont);
                fb.seal_block(cont);
                let result = fb.ins().stack_load(types::I64, rt.call_result_slot, 0);
                stack.push(result);
                return Ok(());
            }
            // Direct-builtin ops: pop the operands, root the rest of the live
            // frame, and call the arity-shaped generic shim with the table
            // index baked in — the shim invokes the SAME builtins::* function
            // the interpreter arm calls.
            let Some((arity, idx)) = direct_builtin_spec(other) else {
                return Err(CompileError::UnsupportedOp(op_category(other)));
            };
            let rt = rt.ok_or(CompileError::UnsupportedOp("builtin"))?;
            let arity = arity as usize;
            if stack.len() < arity {
                return Err(CompileError::StackUnderflow);
            }
            let at = stack.len() - arity;
            let operands: Vec<ClifValue> = stack[at..].to_vec();
            stack.truncate(at);
            // Root remaining live values (the builtin may allocate/GC; the
            // shim roots the operands themselves).
            let saved = if stack.is_empty() {
                None
            } else {
                let c = fb.ins().call(rt.refs.gc_save, &[]);
                let s = fb.inst_results(c)[0];
                for &v in stack.iter() {
                    fb.ins().call(rt.refs.gc_push, &[v]);
                }
                Some(s)
            };
            let vmctx = fb.use_var(rt.vmctx_var);
            let idx_v = fb.ins().iconst(types::I64, idx as i64);
            let out_addr = fb.ins().stack_addr(rt.ptr_ty, rt.call_result_slot, 0);
            let shim = match arity {
                1 => rt.refs.builtin1,
                2 => rt.refs.builtin2,
                _ => rt.refs.builtin3,
            };
            let mut call_args = vec![vmctx, idx_v];
            call_args.extend(operands);
            call_args.push(out_addr);
            let call = fb.ins().call(shim, &call_args);
            let status = fb.inst_results(call)[0];
            if let Some(s) = saved {
                fb.ins().call(rt.refs.gc_restore, &[s]);
            }
            let se = signal_target_for_site(fb, signal_exit, handlers, pending, stack);
            let cont = fb.create_block();
            let ok = fb.ins().icmp_imm(IntCC::Equal, status, STATUS_OK);
            fb.ins().brif(ok, cont, &[], se, &[]);
            fb.switch_to_block(cont);
            fb.seal_block(cont);
            let result = fb.ins().stack_load(types::I64, rt.call_result_slot, 0);
            stack.push(result);
        }
    }
    Ok(())
}

/// Minimum operand-stack depth a simple op requires, and its net depth change.
/// `Err` for anything outside the supported simple subset.
pub(crate) fn simple_effect(op: &Op) -> Result<(usize, i64), CompileError> {
    if let Some((arity, _)) = direct_builtin_spec(op) {
        // N operands -> one result.
        return Ok((arity as usize, 1 - arity as i64));
    }
    if let Some((nargs, _)) = slice_builtin_spec(op) {
        return Ok((nargs, 1 - nargs as i64));
    }
    Ok(match op {
        Op::List(n) => (*n as usize, 1 - *n as i64),
        Op::CallBuiltin(_, n) | Op::CallBuiltinSym(_, n) => (*n as usize, 1 - *n as i64),
        Op::Aset => (3, -2),
        Op::SaveWindowExcursion => (1, 0),
        Op::Constant(_) | Op::Nil | Op::True => (0, 1),
        Op::StackRef(n) => (*n as usize + 1, 1),
        Op::StackSet(n) => (*n as usize + 1, -1),
        Op::DiscardN(raw) => {
            let n = (*raw & 0x7F) as usize;
            let needs = if (*raw & 0x80) != 0 && n > 0 {
                n + 1
            } else {
                n
            };
            (needs, -(n as i64))
        }
        Op::Dup => (1, 1),
        Op::Pop => (1, -1),
        Op::Add
        | Op::Sub
        | Op::Mul
        | Op::Div
        | Op::Rem
        | Op::Eq
        | Op::Eqlsign
        | Op::Lss
        | Op::Gtr
        | Op::Leq
        | Op::Geq => (2, -1),
        Op::Add1 | Op::Sub1 | Op::Negate => (1, 0),
        Op::Null | Op::Not | Op::Consp | Op::Stringp | Op::Listp | Op::Symbolp => (1, 0),
        Op::Integerp | Op::Numberp => (1, 0),
        Op::Car | Op::Cdr | Op::CarSafe | Op::CdrSafe => (1, 0),
        Op::Max | Op::Min => (2, -1),
        Op::Cons => (2, -1),
        // [func a1 .. aN] -> [result]
        Op::Call(n) | Op::Apply(n) => (*n as usize + 1, -(*n as i64)),
        Op::VarRef(_) => (0, 1),
        Op::VarSet(_) => (1, -1),
        Op::VarBind(_) => (1, -1),
        Op::Unbind(_) => (0, 0),
        Op::SaveCurrentBuffer | Op::SaveExcursion | Op::SaveRestriction => (0, 0),
        Op::UnwindProtectPop => (1, -1),
        other => return Err(CompileError::UnsupportedOp(op_category(other))),
    })
}

/// A statically tracked active handler: `(handler target instruction, operand
/// stack depth at the push)`. The list at any program point is the stack of
/// `PushConditionCase`/`PushCatch` frames not yet popped, outermost first.
type HandlerStatic = (usize, usize);

/// A speculated direct-call site: an `Op::Call` whose callee slot provably
/// holds the constant symbol `sym`, fbound at compile time to the bytecode
/// object `expected_bits`. `slot` indexes the leaf's armed-epoch slots.
struct SpecSite {
    sym: u32,
    expected_bits: u64,
    slot: usize,
}

/// Per-site speculation state, baked into generated code by raw address and
/// read by `neovm_jit_call_spec`. `epoch` is the obarray `function_epoch` at
/// which this site's callee binding was last validated. `leaf` lazily caches a
/// `*const CompiledLeaf` (as `usize` bits; 0 = none) for the armed callee, so
/// repeat calls skip the compiled-cache hash lookup (the V3 fast path). The
/// leaf pointer is cleared whenever revalidation fails (the binding changed),
/// and is sound while set because the per-thread `COMPILED` cache never evicts.
/// `repr(C)` pins the field order the baked pointer arithmetic relies on.
#[repr(C)]
pub(crate) struct SpecSlot {
    epoch: AtomicU64,
    leaf: AtomicU64,
}

/// Find direct-call speculation sites: the byte-compiler's standard call
/// shape `Constant(f) arg-push* Call(n)` where every op between the callee
/// push and its call only PUSHES new slots (Constant/Nil/True/Dup/StackRef —
/// the callee slot can't be rewritten), no jump target lands inside the
/// window, and `f` is currently fbound to a BYTECODE object.
///
/// Bytecode callees only for now: an epoch-equal check on a bytecode binding
/// proves it still names the same immutable bytecode object. (Subr-entry
/// rewrites — `register_global_subr_entry` — now also bump function_epoch via
/// `defsubr_with_entry`, so extending speculation to subr bindings is
/// unlocked; it just isn't implemented or measured yet.)
fn find_spec_sites(
    ops: &[Op],
    constants: &[Value],
    leaders: &[usize],
    obarray: &Obarray,
) -> HashMap<usize, SpecSite> {
    let mut sites = HashMap::new();
    let mut next_slot = 0usize;
    'outer: for i in 0..ops.len() {
        let Op::Constant(cidx) = &ops[i] else {
            continue;
        };
        let Some(sym_val) = constants.get(*cidx as usize) else {
            continue;
        };
        let Some(sym_id) = sym_val.as_symbol_id() else {
            continue;
        };
        let mut pushes = 0usize;
        let mut j = i + 1;
        let call_idx = loop {
            if j >= ops.len() || leaders.binary_search(&j).is_ok() {
                continue 'outer;
            }
            match ops[j] {
                Op::Constant(_) | Op::Nil | Op::True | Op::Dup | Op::StackRef(_) => {
                    pushes += 1;
                    j += 1;
                }
                Op::Call(n) if n as usize == pushes => break j,
                _ => continue 'outer,
            }
        };
        let Some(binding) = obarray.symbol_function_id(sym_id) else {
            continue;
        };
        if binding.get_bytecode_data().is_none() {
            continue;
        }
        sites.insert(
            call_idx,
            SpecSite {
                sym: sym_id.0,
                expected_bits: binding.bits() as u64,
                slot: next_slot,
            },
        );
        next_slot += 1;
    }
    sites
}

/// Resolve the static target set of the `Op::Switch` at `i`: the byte
/// compiler always pushes the jump table as a constant immediately before the
/// switch, so require `ops[i-1]` to be that `Constant`, the constant to be a
/// hash table, and every table value to be a fixnum address resolving (through
/// the GNU byte-offset map when present) to an in-range instruction index.
/// Returns deduplicated `(raw address, instruction index)` pairs; anything
/// else bails to the interpreter.
fn switch_static_targets(
    ops: &[Op],
    constants: &[Value],
    offset_map: Option<&[GnuByteOffsetMapEntry]>,
    i: usize,
) -> Result<Vec<(i64, usize)>, CompileError> {
    let table = match i.checked_sub(1).map(|p| &ops[p]) {
        Some(Op::Constant(idx)) => constants
            .get(*idx as usize)
            .ok_or(CompileError::BadOperand)?,
        _ => return Err(CompileError::UnsupportedOp("switch-dynamic")),
    };
    let Some(ht) = table.as_hash_table() else {
        return Err(CompileError::UnsupportedOp("switch-dynamic"));
    };
    let mut out: Vec<(i64, usize)> = Vec::with_capacity(ht.data.len());
    for v in ht.data.values() {
        let ValueKind::Fixnum(raw) = v.kind() else {
            return Err(CompileError::UnsupportedOp("switch-dynamic"));
        };
        let raw_addr = usize::try_from(raw).map_err(|_| CompileError::BadOperand)?;
        let target = match offset_map {
            Some(map) => map
                .binary_search_by_key(&raw_addr, |e| e.byte_offset)
                .map(|k| map[k].instruction_index)
                .map_err(|_| CompileError::BadOperand)?,
            None => raw_addr,
        };
        if target >= ops.len() {
            return Err(CompileError::BadOperand);
        }
        if !out.iter().any(|&(r, _)| r == raw) {
            out.push((raw, target));
        }
    }
    Ok(out)
}

/// Basic-block analysis: sorted block leaders, the operand-stack depth at each
/// block's entry, the active-handler stack at each block's entry, the resolved
/// static target sets of every `Op::Switch`, and the max depth seen at any
/// block boundary.
pub(crate) struct Cfg {
    pub(crate) leaders: Vec<usize>,
    pub(crate) entry_depth: HashMap<usize, usize>,
    pub(crate) entry_handlers: HashMap<usize, Vec<HandlerStatic>>,
    pub(crate) switch_targets: HashMap<usize, Vec<(i64, usize)>>,
    pub(crate) max_depth: usize,
}

/// Record that `target` is entered with stack depth `d`, outstanding dynamic
/// bind count `binds`, and active handler stack `handlers`, scheduling it for
/// analysis on first sight. Depth, bind count, and handler stack must be
/// non-negative and consistent across all paths (the byte-compiler guarantees
/// a single static value per program point), so each block is analyzed once.
fn push_succ(
    entry_depth: &mut HashMap<usize, usize>,
    entry_binds: &mut HashMap<usize, usize>,
    entry_handlers: &mut HashMap<usize, Vec<HandlerStatic>>,
    work: &mut Vec<usize>,
    target: usize,
    d: i64,
    binds: usize,
    handlers: &[HandlerStatic],
) -> Result<(), CompileError> {
    if d < 0 {
        return Err(CompileError::StackUnderflow);
    }
    let d = d as usize;
    match entry_depth.get(&target) {
        Some(&existing) if existing != d => {
            Err(CompileError::UnsupportedOp("inconsistent stack depth"))
        }
        Some(_) => {
            if entry_binds.get(&target).copied().unwrap_or(0) != binds {
                return Err(CompileError::UnsupportedOp("inconsistent bind depth"));
            }
            if entry_handlers
                .get(&target)
                .is_none_or(|existing| existing != handlers)
            {
                return Err(CompileError::UnsupportedOp("inconsistent handler stack"));
            }
            Ok(())
        }
        None => {
            entry_depth.insert(target, d);
            entry_binds.insert(target, binds);
            entry_handlers.insert(target, handlers.to_vec());
            work.push(target);
            Ok(())
        }
    }
}

/// Partition `ops` into basic blocks and compute the operand-stack depth at each
/// block boundary, validating that every op is supported, jump targets are in
/// range, depth never underflows, and every path ends in `Return`.
pub(crate) fn analyze_cfg(
    ops: &[Op],
    constants: &[Value],
    offset_map: Option<&[GnuByteOffsetMapEntry]>,
    arity: usize,
) -> Result<Cfg, CompileError> {
    let n = ops.len();
    if n == 0 {
        return Err(CompileError::NoReturn);
    }

    // 1. Block leaders: index 0, every jump target, and every index following a
    //    branch/goto/return.
    let mut leader_set: BTreeSet<usize> = BTreeSet::new();
    let mut switch_targets: HashMap<usize, Vec<(i64, usize)>> = HashMap::new();
    leader_set.insert(0);
    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::Switch => {
                // Resolve the static target set now (bails for non-constant
                // tables); every target is a leader, plus the miss
                // fall-through.
                let targets = switch_static_targets(ops, constants, offset_map, i)?;
                for &(_, t) in &targets {
                    leader_set.insert(t);
                }
                if i + 1 < n {
                    leader_set.insert(i + 1);
                }
                switch_targets.insert(i, targets);
            }
            Op::Goto(t)
            | Op::GotoIfNil(t)
            | Op::GotoIfNotNil(t)
            | Op::GotoIfNilElsePop(t)
            | Op::GotoIfNotNilElsePop(t) => {
                let t = *t as usize;
                if t >= n {
                    return Err(CompileError::BadOperand);
                }
                leader_set.insert(t);
                if i + 1 < n {
                    leader_set.insert(i + 1);
                }
            }
            // Handler pushes end their block (the lowering emits an anchor
            // edge to the handler target) and make the target a leader.
            Op::PushConditionCase(t) | Op::PushConditionCaseRaw(t) | Op::PushCatch(t) => {
                let t = *t as usize;
                if t >= n {
                    return Err(CompileError::BadOperand);
                }
                leader_set.insert(t);
                if i + 1 < n {
                    leader_set.insert(i + 1);
                }
            }
            Op::Return | Op::Throw => {
                if i + 1 < n {
                    leader_set.insert(i + 1);
                }
            }
            _ => {}
        }
    }
    let leaders: Vec<usize> = leader_set.into_iter().collect();
    let next_leader = |idx: usize| leaders.iter().copied().find(|&l| l > idx).unwrap_or(n);

    // 2. Propagate entry depths over the CFG (worklist). Guards deopt at a
    // PRECISE pc (the interpreter resumes mid-function with the live state),
    // so side effects before a guard are fine — no poisoning dimension.
    let mut entry_depth: HashMap<usize, usize> = HashMap::new();
    let mut entry_binds: HashMap<usize, usize> = HashMap::new();
    let mut entry_handlers: HashMap<usize, Vec<HandlerStatic>> = HashMap::new();
    entry_depth.insert(0, arity);
    entry_binds.insert(0, 0);
    entry_handlers.insert(0, Vec::new());
    let mut work = vec![0usize];
    let mut max_depth = arity;

    while let Some(l) = work.pop() {
        let mut cur = entry_depth[&l] as i64;
        let mut binds = entry_binds.get(&l).copied().unwrap_or(0);
        let mut handlers = entry_handlers.get(&l).cloned().unwrap_or_default();
        let end = next_leader(l);
        let mut terminated = false;
        for op in &ops[l..end] {
            // The terminator (if any) is always the last op of the block.
            match op {
                Op::Return => {
                    if cur < 1 {
                        return Err(CompileError::StackUnderflow);
                    }
                    // Returning with outstanding binds is fine: the frame
                    // unwind in CompiledLeaf::call unbinds to the entry base,
                    // exactly like cleanup_bytecode_frame.
                    terminated = true;
                    break;
                }
                Op::Throw => {
                    // [tag value] -> non-local exit; a terminator for compiled
                    // code (no local handlers exist — handler opcodes bail).
                    if cur < 2 {
                        return Err(CompileError::StackUnderflow);
                    }
                    terminated = true;
                    break;
                }
                Op::Goto(t) => {
                    push_succ(
                        &mut entry_depth,
                        &mut entry_binds,
                        &mut entry_handlers,
                        &mut work,
                        *t as usize,
                        cur,
                        binds,
                        &handlers,
                    )?;
                    terminated = true;
                    break;
                }
                Op::GotoIfNil(t) | Op::GotoIfNotNil(t) => {
                    if cur < 1 {
                        return Err(CompileError::StackUnderflow);
                    }
                    cur -= 1; // pop the condition
                    push_succ(
                        &mut entry_depth,
                        &mut entry_binds,
                        &mut entry_handlers,
                        &mut work,
                        *t as usize,
                        cur,
                        binds,
                        &handlers,
                    )?;
                    // fall-through
                    push_succ(
                        &mut entry_depth,
                        &mut entry_binds,
                        &mut entry_handlers,
                        &mut work,
                        end,
                        cur,
                        binds,
                        &handlers,
                    )?;
                    terminated = true;
                    break;
                }
                Op::GotoIfNilElsePop(t) | Op::GotoIfNotNilElsePop(t) => {
                    if cur < 1 {
                        return Err(CompileError::StackUnderflow);
                    }
                    // The jump preserves TOS (depth cur); the fall-through pops it.
                    push_succ(
                        &mut entry_depth,
                        &mut entry_binds,
                        &mut entry_handlers,
                        &mut work,
                        *t as usize,
                        cur,
                        binds,
                        &handlers,
                    )?;
                    push_succ(
                        &mut entry_depth,
                        &mut entry_binds,
                        &mut entry_handlers,
                        &mut work,
                        end,
                        cur - 1,
                        binds,
                        &handlers,
                    )?;
                    terminated = true;
                    break;
                }
                Op::Switch => {
                    // [dispatch table] -> jump to a static target or fall
                    // through on a miss. The target set was resolved in the
                    // leader pass.
                    if end >= n {
                        return Err(CompileError::NoReturn);
                    }
                    if cur < 2 {
                        return Err(CompileError::StackUnderflow);
                    }
                    cur -= 2;
                    // The Switch is the last op of its block (pass 1 made the
                    // following index a leader), so `end` is the fall-through.
                    let i = end - 1;
                    for &(_, t) in switch_targets.get(&i).expect("resolved in pass 1") {
                        push_succ(
                            &mut entry_depth,
                            &mut entry_binds,
                            &mut entry_handlers,
                            &mut work,
                            t,
                            cur,
                            binds,
                            &handlers,
                        )?;
                    }
                    push_succ(
                        &mut entry_depth,
                        &mut entry_binds,
                        &mut entry_handlers,
                        &mut work,
                        end,
                        cur,
                        binds,
                        &handlers,
                    )?;
                    terminated = true;
                    break;
                }
                Op::PushConditionCase(t) | Op::PushConditionCaseRaw(t) | Op::PushCatch(t) => {
                    if end >= n {
                        // A push as the final op would fall off the end.
                        return Err(CompileError::NoReturn);
                    }
                    // Raw/Catch consume the conditions/tag operand first.
                    if !matches!(op, Op::PushConditionCase(_)) {
                        if cur < 1 {
                            return Err(CompileError::StackUnderflow);
                        }
                        cur -= 1;
                    }
                    // Handler edge: entered with the push-time stack plus the
                    // error value, the handler stack as of BEFORE this push
                    // (the matched frame and everything above it were popped
                    // by the unwind), and the push-time bind count (the catch
                    // restored the specpdl/bind state).
                    push_succ(
                        &mut entry_depth,
                        &mut entry_binds,
                        &mut entry_handlers,
                        &mut work,
                        *t as usize,
                        cur + 1,
                        binds,
                        &handlers,
                    )?;
                    if cur as usize + 1 > max_depth {
                        max_depth = cur as usize + 1;
                    }
                    handlers.push((*t as usize, cur as usize));
                    // Fall-through edge: same stack, handler now active.
                    push_succ(
                        &mut entry_depth,
                        &mut entry_binds,
                        &mut entry_handlers,
                        &mut work,
                        end,
                        cur,
                        binds,
                        &handlers,
                    )?;
                    terminated = true;
                    break;
                }
                Op::PopHandler => {
                    // Normal exit from a protected extent: drop the innermost
                    // static handler. No stack effect; non-poisoning (the pop
                    // is a silent registration change — a deopt-rerun re-pushes
                    // and re-pops it after the frame unwind truncated ours).
                    if handlers.pop().is_none() {
                        return Err(CompileError::UnsupportedOp("unbalanced-pophandler"));
                    }
                }
                other => {
                    let (needs, delta) = simple_effect(other)?;
                    if cur < needs as i64 {
                        return Err(CompileError::StackUnderflow);
                    }
                    cur += delta;
                    if cur as usize > max_depth {
                        max_depth = cur as usize;
                    }
                    match other {
                        Op::VarBind(_)
                        | Op::SaveCurrentBuffer
                        | Op::SaveExcursion
                        | Op::SaveRestriction
                        | Op::UnwindProtectPop => binds += 1,
                        Op::Unbind(un) => {
                            let un = *un as usize;
                            if un > binds {
                                // Unbinding more than this function bound —
                                // bail to the interpreter (its bind_stack
                                // saturation handles it).
                                return Err(CompileError::UnsupportedOp("unbalanced-unbind"));
                            }
                            binds -= un;
                        }
                        _ => {}
                    }
                }
            }
        }
        if !terminated {
            // Block falls through into the next leader (guaranteed to exist and
            // be < n; a block running off the end with no Return is invalid).
            if end >= n {
                return Err(CompileError::NoReturn);
            }
            push_succ(
                &mut entry_depth,
                &mut entry_binds,
                &mut entry_handlers,
                &mut work,
                end,
                cur,
                binds,
                &handlers,
            )?;
        }
    }

    for &d in entry_depth.values() {
        max_depth = max_depth.max(d);
    }
    Ok(Cfg {
        leaders,
        entry_depth,
        entry_handlers,
        switch_targets,
        max_depth,
    })
}

/// Apply one non-terminator op's effect to the known-fixnum operand-stack model
/// `k` (parallel to the real operand stack: `k[i]` is `true` iff position `i` is
/// PROVABLY a fixnum). Returns `Err(())` for any op this analysis does not model
/// precisely, so the caller bails the whole function (conservative — no guard is
/// elided). Fixnum constants and fixnum arithmetic results are `true`;
/// StackRef/Dup/StackSet/DiscardN move bits; everything else is `false`.
fn apply_known_fixnum_op(op: &Op, constants: &[Value], k: &mut Vec<bool>) -> Result<(), ()> {
    match op {
        Op::Constant(idx) => {
            let is_fix = constants
                .get(*idx as usize)
                .map(|v| (v.bits() & FIXNUM_CHECK_MASK) == FIXNUM_CHECK_VALUE)
                .unwrap_or(false);
            k.push(is_fix);
        }
        Op::Nil | Op::True => k.push(false),
        Op::StackRef(j) => {
            let n = *j as usize;
            let v = *k.get(k.len().checked_sub(1 + n).ok_or(())?).ok_or(())?;
            k.push(v);
        }
        Op::Dup => {
            let v = *k.last().ok_or(())?;
            k.push(v);
        }
        Op::StackSet(j) => {
            let n = *j as usize;
            let v = k.pop().ok_or(())?;
            if n >= 1 {
                let idx = k.len().checked_sub(n).ok_or(())?;
                k[idx] = v;
            }
        }
        Op::Pop => {
            k.pop().ok_or(())?;
        }
        Op::DiscardN(raw) => {
            let n = (*raw & 0x7F) as usize;
            let preserve = (*raw & 0x80) != 0 && n > 0;
            if preserve {
                let tos = k.pop().ok_or(())?;
                let keep = k.len().checked_sub(n).ok_or(())?;
                k.truncate(keep);
                k.push(tos);
            } else {
                let keep = k.len().checked_sub(n).ok_or(())?;
                k.truncate(keep);
            }
        }
        // Fixnum arithmetic: the result is range-checked + retagged -> fixnum.
        Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem | Op::Max | Op::Min => {
            k.pop().ok_or(())?;
            k.pop().ok_or(())?;
            k.push(true);
        }
        Op::Add1 | Op::Sub1 | Op::Negate => {
            k.pop().ok_or(())?;
            k.push(true);
        }
        // Operand-consuming ops whose result is NOT a known fixnum: pop `needs`
        // (== the operands consumed for these) and push the results as unknown.
        // `simple_effect` is authoritative for the depth change.
        Op::Eq
        | Op::Eqlsign
        | Op::Lss
        | Op::Gtr
        | Op::Leq
        | Op::Geq
        | Op::Null
        | Op::Not
        | Op::Consp
        | Op::Stringp
        | Op::Listp
        | Op::Symbolp
        | Op::Integerp
        | Op::Numberp
        | Op::Car
        | Op::Cdr
        | Op::CarSafe
        | Op::CdrSafe
        | Op::Cons
        | Op::Aset
        | Op::List(_)
        | Op::Call(_)
        | Op::Apply(_)
        | Op::CallBuiltin(..)
        | Op::CallBuiltinSym(..)
        | Op::VarRef(_)
        | Op::VarSet(_)
        | Op::VarBind(_)
        | Op::Unbind(_)
        | Op::UnwindProtectPop
        | Op::SaveCurrentBuffer
        | Op::SaveExcursion
        | Op::SaveRestriction => {
            let (needs, delta) = simple_effect(op).map_err(|_| ())?;
            // These ops (unlike StackRef/Dup/StackSet/DiscardN) consume exactly
            // their top `needs` operands, so popping `needs` keeps `k` aligned.
            for _ in 0..needs {
                k.pop().ok_or(())?;
            }
            let pushes = needs as i64 + delta;
            for _ in 0..pushes.max(0) {
                k.push(false);
            }
        }
        // Direct/slice builtin dispatch (e.g. `1+`-as-subr): pop nargs, push one
        // unknown result. Detected the same way `simple_effect` does.
        other if direct_builtin_spec(other).is_some() || slice_builtin_spec(other).is_some() => {
            let (needs, delta) = simple_effect(other).map_err(|_| ())?;
            for _ in 0..needs {
                k.pop().ok_or(())?;
            }
            let pushes = needs as i64 + delta;
            for _ in 0..pushes.max(0) {
                k.push(false);
            }
        }
        // Anything else (Switch/handler ops/unmodeled) -> bail.
        _ => return Err(()),
    }
    Ok(())
}

/// **Cross-block redundant-guard elimination — the analysis (UNWIRED).**
///
/// Forward dataflow fixpoint over the CFG: for each block leader, the operand-
/// stack SLOTS provably fixnum at block entry. A slot is known-fixnum at entry
/// iff it is known-fixnum on EVERY predecessor edge (meet = AND); loops need the
/// fixpoint (the back-edge induction value depends on the slot's own bit). It is
/// a MUST analysis, so non-entry blocks start at TOP (all-`true`) and are
/// narrowed by predecessors; the entry block starts all-`false` (args untyped).
///
/// Conservative: returns an EMPTY map (no elision anywhere) for any function
/// containing an op this analysis does not model precisely (Switch, catch/
/// condition-case handlers, ...). NOT yet wired into `lower_leaf_full`; the
/// integration that consumes this is a follow-up. `cfg` must come from
/// [`analyze_cfg`] on the same `ops`.
fn compute_known_fixnum_slots(
    ops: &[Op],
    constants: &[Value],
    cfg: &Cfg,
) -> HashMap<usize, Vec<bool>> {
    let n = ops.len();
    let next_leader = |idx: usize| cfg.leaders.iter().copied().find(|&l| l > idx).unwrap_or(n);
    let empty = HashMap::new();

    // in[leader] = known-fixnum bits at block entry. Entry (0) is all-false;
    // every other block starts at TOP for the AND fixpoint.
    let mut in_sets: HashMap<usize, Vec<bool>> = HashMap::new();
    for &l in &cfg.leaders {
        let d = cfg.entry_depth.get(&l).copied().unwrap_or(0);
        in_sets.insert(l, vec![l != 0; d]);
    }

    // AND a predecessor contribution into a successor's in-set; report narrowing.
    fn meet(into: &mut [bool], contrib: &[bool]) -> bool {
        let mut changed = false;
        for (slot, &c) in into.iter_mut().zip(contrib.iter()) {
            if *slot && !c {
                *slot = false;
                changed = true;
            }
        }
        changed
    }

    let mut iterate = true;
    while iterate {
        iterate = false;
        for &l in &cfg.leaders {
            let mut k = in_sets[&l].clone();
            let end = next_leader(l);
            let mut edges: Vec<(usize, Vec<bool>)> = Vec::new();
            let mut terminated = false;
            for op in &ops[l..end] {
                match op {
                    Op::Return | Op::Throw => {
                        terminated = true;
                        break;
                    }
                    Op::Goto(t) => {
                        edges.push((*t as usize, k.clone()));
                        terminated = true;
                        break;
                    }
                    Op::GotoIfNil(t) | Op::GotoIfNotNil(t) => {
                        if k.pop().is_none() {
                            return empty;
                        }
                        edges.push((*t as usize, k.clone()));
                        edges.push((end, k.clone()));
                        terminated = true;
                        break;
                    }
                    Op::GotoIfNilElsePop(t) | Op::GotoIfNotNilElsePop(t) => {
                        // The jump preserves TOS; the fall-through pops it.
                        edges.push((*t as usize, k.clone()));
                        let mut ft = k.clone();
                        if ft.pop().is_none() {
                            return empty;
                        }
                        edges.push((end, ft));
                        terminated = true;
                        break;
                    }
                    other => {
                        if apply_known_fixnum_op(other, constants, &mut k).is_err() {
                            // Unmodeled op (Switch / handler / ...): bail entirely.
                            return empty;
                        }
                    }
                }
            }
            if !terminated {
                if end >= n {
                    return empty;
                }
                edges.push((end, k.clone()));
            }
            for (t, contrib) in &edges {
                if let Some(into) = in_sets.get_mut(t) {
                    if meet(into, contrib) {
                        iterate = true;
                    }
                }
            }
        }
    }
    in_sets
}

/// Write the live operand `stack` back into the slot variables so a successor
/// block can read it (the variable/SSA machinery inserts the needed phis).
fn write_stack_to_vars(fb: &mut FunctionBuilder, vars: &[Variable], stack: &[ClifValue]) {
    for (k, &v) in stack.iter().enumerate() {
        fb.def_var(vars[k], v);
    }
}

/// Emit a backward jump with the interpreter's `branch_to!` parity: bump the
/// u8 quit counter; on every wrap (each 255th backward jump — counter resets to
/// 1, exactly like the interpreter) root the live operand stack and call the
/// back-edge service poll (GC safepoint + `maybe_quit`), propagating a signaled
/// `Flow` via the shared signal-exit block. The caller has already written the
/// operand stack to `vars` (the target's entry state).
#[allow(clippy::too_many_arguments)]
fn emit_backedge_jump(
    fb: &mut FunctionBuilder,
    rt: &RtCtx,
    counter_slot: StackSlot,
    signal_exit: &mut Option<Block>,
    vars: &[Variable],
    target_depth: usize,
    target_block: Block,
    handlers: &[HandlerStatic],
    pending: &mut Vec<PendingDispatch>,
) {
    let c = fb.ins().stack_load(types::I64, counter_slot, 0);
    let c1 = fb.ins().iadd_imm(c, 1);
    let c1m = fb.ins().band_imm(c1, 0xFF);
    fb.ins().stack_store(c1m, counter_slot, 0);
    let wrapped = fb.ins().icmp_imm(IntCC::Equal, c1m, 0);
    let poll = fb.create_block();
    fb.ins().brif(wrapped, poll, &[], target_block, &[]);

    fb.switch_to_block(poll);
    fb.seal_block(poll);
    let one = fb.ins().iconst(types::I64, 1);
    fb.ins().stack_store(one, counter_slot, 0);
    // The live operand stack at the jump (already written to vars): rooted
    // across the poll, and the handler-entry snapshot if a quit signal lands
    // in a protected extent (condition-case catching `quit` around a loop).
    let vals: Vec<ClifValue> = (0..target_depth).map(|k| fb.use_var(vars[k])).collect();
    let saved = if vals.is_empty() {
        None
    } else {
        let call = fb.ins().call(rt.refs.gc_save, &[]);
        let s = fb.inst_results(call)[0];
        for &v in vals.iter() {
            fb.ins().call(rt.refs.gc_push, &[v]);
        }
        Some(s)
    };
    let vmctx = fb.use_var(rt.vmctx_var);
    let call = fb.ins().call(rt.refs.backedge, &[vmctx]);
    let status = fb.inst_results(call)[0];
    if let Some(s) = saved {
        fb.ins().call(rt.refs.gc_restore, &[s]);
    }
    let se = signal_target_for_site(fb, signal_exit, handlers, pending, &vals);
    let ok = fb.ins().icmp_imm(IntCC::Equal, status, STATUS_OK);
    fb.ins().brif(ok, target_block, &[], se, &[]);
}

/// Lower a leaf bytecode body taking `arity` fixed arguments to native code.
///
/// Handles arbitrary intra-function control flow (`Goto`/`GotoIf*`) by building a
/// CLIF basic-block CFG: each bytecode basic block becomes a CLIF block, and the
/// operand stack flows across edges through per-slot SSA variables (Cranelift
/// inserts the phis). The `arity` arguments are loaded and seed the bottom of the
/// stack (arg0 deepest), exactly as the interpreter's `run_frame` pushes them.
pub fn lower_leaf(
    ops: &[Op],
    constants: &[Value],
    arity: usize,
) -> Result<CompiledLeaf, CompileError> {
    lower_leaf_with_map(ops, constants, arity, None)
}

/// [`lower_leaf`] with the function's GNU byte-offset map, needed to resolve
/// `Op::Switch` jump-table addresses to instruction indices (GNU bytecode
/// stores byte offsets; natively compiled chunks store indices directly).
pub fn lower_leaf_with_map(
    ops: &[Op],
    constants: &[Value],
    arity: usize,
    offset_map: Option<&[GnuByteOffsetMapEntry]>,
) -> Result<CompiledLeaf, CompileError> {
    lower_leaf_full(ops, constants, arity, offset_map, None)
}

/// [`lower_leaf_with_map`] plus the compiling thread's obarray, enabling
/// direct-call speculation (constant-symbol callees bound to bytecode get
/// epoch-validated direct calls; see [`find_spec_sites`]).
pub fn lower_leaf_full(
    ops: &[Op],
    constants: &[Value],
    arity: usize,
    offset_map: Option<&[GnuByteOffsetMapEntry]>,
    obarray: Option<&Obarray>,
) -> Result<CompiledLeaf, CompileError> {
    let cfg = analyze_cfg(ops, constants, offset_map, arity)?;
    // Cross-block redundant-guard elimination: per-block-entry known-fixnum slots
    // (empty if the function has an op the analysis doesn't model -> no elision).
    let known_fixnum_slots = compute_known_fixnum_slots(ops, constants, &cfg);
    let n = ops.len();
    // Direct-call speculation sites + their armed-epoch slots. The Box's heap
    // storage is address-stable: slot pointers are baked into the generated
    // code as immediates and the Box moves into the CompiledLeaf at the end.
    let (spec_sites, spec_slots): (HashMap<usize, SpecSite>, Box<[SpecSlot]>) = match obarray {
        Some(ob) => {
            let sites = find_spec_sites(ops, constants, &cfg.leaders, ob);
            let slots: Box<[SpecSlot]> = (0..sites.len())
                .map(|_| SpecSlot {
                    epoch: AtomicU64::new(0),
                    leaf: AtomicU64::new(0),
                })
                .collect();
            // Arm every slot with the epoch the bindings were observed at; any
            // bump before first execution self-heals via shim re-validation.
            let epoch = ob.function_epoch();
            for site in sites.values() {
                slots[site.slot].epoch.store(epoch, Ordering::Relaxed);
            }
            (sites, slots)
        }
        None => (HashMap::new(), Box::from([])),
    };
    // Precise-deopt buffers: live operand-stack spill (max depth) + the
    // pc/depth/handler-count cells. Address-stable Boxes owned by the leaf;
    // generated code writes through baked raw addresses.
    let deopt_spill: Box<[core::cell::Cell<i64>]> = (0..cfg.max_depth)
        .map(|_| core::cell::Cell::new(0))
        .collect();
    let deopt_meta: Box<DeoptCells> = Box::new(DeoptCells {
        pc: core::cell::Cell::new(0),
        depth: core::cell::Cell::new(0),
        handlers: core::cell::Cell::new(0),
    });

    // Baseline tier runs Cranelift at the default opt_level="none": its job is
    // FAST compilation (low tier-up latency; the soak compiles every function).
    // Measured opt_level="speed" (2026-06-13): no runtime win on fib (call-
    // bound) or the arithmetic loop, because Cranelift sees our tagged Values
    // as opaque i64 — it can't unbox, drop fixnum guards, or reason about lisp
    // effects. The real headroom is semantic (unboxing/inlining), which needs
    // an MIR-level optimizing Tier-2; opt_level="speed" belongs there, not at
    // this tier where it would only cost compile time.
    let mut builder = JITBuilder::new(default_libcall_names())
        .map_err(|e| CompileError::Backend(BackendError::ModuleInit(e.to_string())))?;
    builder.symbol("neovm_jit_gc_save", neovm_jit_gc_save as *const u8);
    builder.symbol("neovm_jit_gc_push", neovm_jit_gc_push as *const u8);
    builder.symbol("neovm_jit_gc_restore", neovm_jit_gc_restore as *const u8);
    builder.symbol("neovm_jit_cons", neovm_jit_cons as *const u8);
    builder.symbol("neovm_jit_call", neovm_jit_call as *const u8);
    builder.symbol("neovm_jit_apply", neovm_jit_apply as *const u8);
    builder.symbol("neovm_jit_eq_slow", neovm_jit_eq_slow as *const u8);
    builder.symbol(
        "neovm_jit_symbolp_slow",
        neovm_jit_symbolp_slow as *const u8,
    );
    builder.symbol("neovm_jit_varref", neovm_jit_varref as *const u8);
    builder.symbol("neovm_jit_varset", neovm_jit_varset as *const u8);
    builder.symbol("neovm_jit_varbind", neovm_jit_varbind as *const u8);
    builder.symbol("neovm_jit_unbind", neovm_jit_unbind as *const u8);
    builder.symbol("neovm_jit_backedge", neovm_jit_backedge as *const u8);
    builder.symbol(
        "neovm_jit_save_current_buffer",
        neovm_jit_save_current_buffer as *const u8,
    );
    builder.symbol(
        "neovm_jit_save_excursion",
        neovm_jit_save_excursion as *const u8,
    );
    builder.symbol(
        "neovm_jit_save_restriction",
        neovm_jit_save_restriction as *const u8,
    );
    builder.symbol("neovm_jit_throw", neovm_jit_throw as *const u8);
    builder.symbol(
        "neovm_jit_integerp_slow",
        neovm_jit_integerp_slow as *const u8,
    );
    builder.symbol(
        "neovm_jit_numberp_slow",
        neovm_jit_numberp_slow as *const u8,
    );
    builder.symbol(
        "neovm_jit_unwind_protect",
        neovm_jit_unwind_protect as *const u8,
    );
    builder.symbol("neovm_jit_builtin1", neovm_jit_builtin1 as *const u8);
    builder.symbol("neovm_jit_builtin2", neovm_jit_builtin2 as *const u8);
    builder.symbol("neovm_jit_builtin3", neovm_jit_builtin3 as *const u8);
    builder.symbol("neovm_jit_push_cc", neovm_jit_push_cc as *const u8);
    builder.symbol("neovm_jit_push_cc_raw", neovm_jit_push_cc_raw as *const u8);
    builder.symbol("neovm_jit_push_catch", neovm_jit_push_catch as *const u8);
    builder.symbol("neovm_jit_pop_handler", neovm_jit_pop_handler as *const u8);
    builder.symbol(
        "neovm_jit_match_handler",
        neovm_jit_match_handler as *const u8,
    );
    builder.symbol("neovm_jit_switch", neovm_jit_switch as *const u8);
    builder.symbol(
        "neovm_jit_switch_stale",
        neovm_jit_switch_stale as *const u8,
    );
    builder.symbol("neovm_jit_list", neovm_jit_list as *const u8);
    builder.symbol(
        "neovm_jit_builtin_slice",
        neovm_jit_builtin_slice as *const u8,
    );
    builder.symbol(
        "neovm_jit_named_builtin",
        neovm_jit_named_builtin as *const u8,
    );
    builder.symbol(
        "neovm_jit_save_window_excursion",
        neovm_jit_save_window_excursion as *const u8,
    );
    builder.symbol("neovm_jit_call_spec", neovm_jit_call_spec as *const u8);
    let mut module = JITModule::new(builder);
    let call_conv = module.target_config().default_call_conv;
    let ptr_ty = module.target_config().pointer_type();
    // Backward jumps need the back-edge service poll (GC safepoint + quit),
    // mirroring the interpreter's branch_to! wrap path. Switch targets count:
    // a jump-table edge can also close a loop.
    let has_backedge = ops.iter().enumerate().any(|(i, o)| match o {
        Op::Goto(t)
        | Op::GotoIfNil(t)
        | Op::GotoIfNotNil(t)
        | Op::GotoIfNilElsePop(t)
        | Op::GotoIfNotNilElsePop(t) => (*t as usize) <= i,
        _ => false,
    }) || cfg
        .switch_targets
        .iter()
        .any(|(i, ts)| ts.iter().any(|&(_, t)| t <= *i));
    // Eq/Symbolp need vmctx for their symbols-with-pos slow-path shims;
    // VarRef/VarSet/VarBind/Unbind re-enter the runtime's variable machinery;
    // back-edges poll through vmctx.
    let needs_rt = has_backedge
        || ops.iter().any(|o| {
            direct_builtin_spec(o).is_some()
                || slice_builtin_spec(o).is_some()
                || matches!(
                    o,
                    Op::List(_)
                        | Op::CallBuiltin(..)
                        | Op::CallBuiltinSym(..)
                        | Op::Aset
                        | Op::SaveWindowExcursion
                )
                || matches!(
                    o,
                    Op::Cons
                        | Op::Call(_)
                        | Op::Apply(_)
                        | Op::Eq
                        | Op::Symbolp
                        | Op::VarRef(_)
                        | Op::VarSet(_)
                        | Op::VarBind(_)
                        | Op::Unbind(_)
                        | Op::SaveCurrentBuffer
                        | Op::SaveExcursion
                        | Op::SaveRestriction
                        | Op::UnwindProtectPop
                        | Op::Throw
                        | Op::Integerp
                        | Op::Numberp
                        | Op::PushConditionCase(_)
                        | Op::PushConditionCaseRaw(_)
                        | Op::PushCatch(_)
                        | Op::PopHandler
                        | Op::Switch
                )
        });

    // ABI: fn(vmctx: *mut Context, args: *const i64, out: *mut i64) -> i64.
    // Reads `arity` argument words from `args`; returns STATUS_OK + writes the
    // result bits via `out` on success, STATUS_DEOPT on a failed guard, or
    // STATUS_SIGNAL when a runtime call raised a Flow (stashed for
    // `take_pending_flow`). `vmctx` is only used by runtime-call shims.
    let mut sig = Signature::new(call_conv);
    sig.params.push(AbiParam::new(ptr_ty)); // vmctx
    sig.params.push(AbiParam::new(ptr_ty)); // args
    sig.params.push(AbiParam::new(ptr_ty)); // out
    sig.returns.push(AbiParam::new(types::I64));

    let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig.clone());
    let mut fbctx = FunctionBuilderContext::new();
    {
        let mut fb = FunctionBuilder::new(&mut func, &mut fbctx);

        // Declare the runtime-call machinery into this function if the body
        // re-enters the runtime (`Cons` / `Call`).
        let rt = if needs_rt {
            let refs = declare_rt_refs(&mut module, fb.func, call_conv, ptr_ty)?;
            let vmctx_var = fb.declare_var(ptr_ty);
            let max_call_args = ops
                .iter()
                .filter_map(|o| match o {
                    Op::Call(n) | Op::Apply(n) | Op::List(n) | Op::Concat(n) => Some(*n as usize),
                    Op::CallBuiltin(_, n) | Op::CallBuiltinSym(_, n) => Some(*n as usize),
                    Op::Nconc => Some(2),
                    Op::Substring | Op::Aset => Some(3),
                    _ => None,
                })
                .max()
                .unwrap_or(0);
            let call_args_slot = fb.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (max_call_args.max(1) * 8) as u32,
                3,
            ));
            let call_result_slot =
                fb.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
            Some(RtCtx {
                refs,
                vmctx_var,
                ptr_ty,
                call_args_slot,
                call_result_slot,
            })
        } else {
            None
        };

        // SSA variables: one I64 slot per operand-stack position (carries the
        // stack across block edges), plus one for the out pointer (used by
        // `Return` in any block).
        let vars: Vec<Variable> = (0..cfg.max_depth)
            .map(|_| fb.declare_var(types::I64))
            .collect();
        let out_var = fb.declare_var(ptr_ty);

        // One CLIF block per bytecode basic block.
        let block_for: HashMap<usize, Block> = cfg
            .leaders
            .iter()
            .map(|&l| (l, fb.create_block()))
            .collect();
        // Shared deopt landing block, created lazily on the first guard.
        let deopt_refs = DeoptRefs {
            spill_base: deopt_spill.as_ptr() as i64,
            meta_pc: &deopt_meta.pc as *const core::cell::Cell<i64> as i64,
            meta_depth: &deopt_meta.depth as *const core::cell::Cell<i64> as i64,
            meta_handlers: &deopt_meta.handlers as *const core::cell::Cell<i64> as i64,
        };
        // Shared signal-propagation block (returns STATUS_SIGNAL), created
        // lazily by the first `Call` lowering.
        let mut signal_exit: Option<Block> = None;
        // Backward-jump quit counter (the interpreter's u8 `quitcounter`), kept
        // in a stack slot so every block can bump it.
        let backedge_counter: Option<StackSlot> = has_backedge.then(|| {
            fb.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3))
        });

        // Function-entry block: stash vmctx + the out pointer, load args into
        // the slot variables, then jump into bytecode block 0.
        let entry = fb.create_block();
        fb.append_block_params_for_function_params(entry);
        fb.switch_to_block(entry);
        let vmctx_param = fb.block_params(entry)[0];
        let args_ptr = fb.block_params(entry)[1];
        let out_ptr = fb.block_params(entry)[2];
        if let Some(rt) = &rt {
            fb.def_var(rt.vmctx_var, vmctx_param);
        }
        fb.def_var(out_var, out_ptr);
        if let Some(slot) = backedge_counter {
            // The interpreter starts quitcounter at 1.
            let one = fb.ins().iconst(types::I64, 1);
            fb.ins().stack_store(one, slot, 0);
        }
        for i in 0..arity {
            let v = fb
                .ins()
                .load(types::I64, MemFlags::trusted(), args_ptr, (i * 8) as i32);
            fb.def_var(vars[i], v);
        }
        fb.ins().jump(block_for[&0], &[]);

        let next_leader = |idx: usize| cfg.leaders.iter().copied().find(|&l| l > idx).unwrap_or(n);

        for &l in &cfg.leaders {
            let blk = block_for[&l];
            fb.switch_to_block(blk);
            // Materialize the incoming operand stack from the slot variables.
            let depth = cfg.entry_depth[&l];
            let mut stack: Vec<ClifValue> = (0..depth).map(|k| fb.use_var(vars[k])).collect();
            // Cross-block known-fixnum operands at this block's entry: each slot
            // the dataflow analysis proved fixnum maps to its just-materialized
            // ClifValue. StackRef/Dup keep the same ClifValue, so the set stays
            // valid as the block runs; `guard_fixnum` elides guards for members.
            let known_fixnum: HashSet<ClifValue> = known_fixnum_slots
                .get(&l)
                .map(|slots| {
                    slots
                        .iter()
                        .enumerate()
                        .filter_map(|(k, &is_fix)| (is_fix).then(|| stack.get(k).copied()).flatten())
                        .collect()
                })
                .unwrap_or_default();
            // Active handler frames at block entry (static), kept in sync as
            // PopHandler ops run; signal sites inside a protected extent queue
            // a dispatch block here, filled after the block's terminator.
            let mut handlers: Vec<HandlerStatic> =
                cfg.entry_handlers.get(&l).cloned().unwrap_or_default();
            let mut pending: Vec<PendingDispatch> = Vec::new();
            let mut pending_deopt: Vec<PendingDeopt> = Vec::new();

            let end = next_leader(l);
            let mut terminated = false;
            for (off, op) in ops[l..end].iter().enumerate() {
                let i = l + off;
                match op {
                    Op::Return => {
                        let result = stack.pop().ok_or(CompileError::StackUnderflow)?;
                        let out = fb.use_var(out_var);
                        fb.ins().store(MemFlags::trusted(), result, out, 0);
                        let one = fb.ins().iconst(types::I64, 1);
                        fb.ins().return_(&[one]);
                        terminated = true;
                        break;
                    }
                    Op::Throw => {
                        // Stash Flow::Throw{tag, value} and exit via the signal
                        // path; inside a protected extent that path is the
                        // handler dispatch (a same-function `catch` is caught
                        // natively via the match shim).
                        let value = stack.pop().ok_or(CompileError::StackUnderflow)?;
                        let tag = stack.pop().ok_or(CompileError::StackUnderflow)?;
                        let rt = rt.as_ref().ok_or(CompileError::UnsupportedOp("throw"))?;
                        fb.ins().call(rt.refs.throw_flow, &[tag, value]);
                        let se = signal_target_for_site(
                            &mut fb,
                            &mut signal_exit,
                            &handlers,
                            &mut pending,
                            &stack,
                        );
                        fb.ins().jump(se, &[]);
                        terminated = true;
                        break;
                    }
                    Op::Goto(t) => {
                        write_stack_to_vars(&mut fb, &vars, &stack);
                        let tu = *t as usize;
                        if tu <= i {
                            // Backward jump: bump the quit counter and poll on
                            // wrap, exactly like the interpreter's branch_to!.
                            let (rt, slot) = (
                                rt.as_ref().expect("backedge implies rt"),
                                backedge_counter.expect("backedge implies counter"),
                            );
                            emit_backedge_jump(
                                &mut fb,
                                rt,
                                slot,
                                &mut signal_exit,
                                &vars,
                                cfg.entry_depth[&tu],
                                block_for[&tu],
                                &handlers,
                                &mut pending,
                            );
                        } else {
                            fb.ins().jump(block_for[&tu], &[]);
                        }
                        terminated = true;
                        break;
                    }
                    Op::GotoIfNil(t) | Op::GotoIfNotNil(t) => {
                        let cond = stack.pop().ok_or(CompileError::StackUnderflow)?;
                        write_stack_to_vars(&mut fb, &vars, &stack);
                        let is_nil =
                            fb.ins()
                                .icmp_imm(IntCC::Equal, cond, Value::NIL.bits() as i64);
                        let tu = *t as usize;
                        let mut target = block_for[&tu];
                        let fallthrough = block_for[&(i + 1)];
                        let backedge = (tu <= i).then(|| fb.create_block());
                        if let Some(tramp) = backedge {
                            target = tramp;
                        }
                        // brif takes the `then` block when the condition is true.
                        if matches!(op, Op::GotoIfNil(_)) {
                            fb.ins().brif(is_nil, target, &[], fallthrough, &[]);
                        } else {
                            fb.ins().brif(is_nil, fallthrough, &[], target, &[]);
                        }
                        if let Some(tramp) = backedge {
                            // Taken-edge trampoline carrying the back-edge poll.
                            fb.switch_to_block(tramp);
                            fb.seal_block(tramp);
                            let (rt, slot) = (
                                rt.as_ref().expect("backedge implies rt"),
                                backedge_counter.expect("backedge implies counter"),
                            );
                            emit_backedge_jump(
                                &mut fb,
                                rt,
                                slot,
                                &mut signal_exit,
                                &vars,
                                cfg.entry_depth[&tu],
                                block_for[&tu],
                                &handlers,
                                &mut pending,
                            );
                        }
                        terminated = true;
                        break;
                    }
                    Op::GotoIfNilElsePop(t) | Op::GotoIfNotNilElsePop(t) => {
                        // Peek the condition without popping; write the FULL stack
                        // (cond on top) to vars. The jump-taken successor reads it
                        // all (depth D); the fall-through (depth D-1) ignores the
                        // top slot — implementing the "ElsePop".
                        let cond = *stack.last().ok_or(CompileError::StackUnderflow)?;
                        write_stack_to_vars(&mut fb, &vars, &stack);
                        let is_nil =
                            fb.ins()
                                .icmp_imm(IntCC::Equal, cond, Value::NIL.bits() as i64);
                        let tu = *t as usize;
                        let mut target = block_for[&tu];
                        let fallthrough = block_for[&(i + 1)];
                        let backedge = (tu <= i).then(|| fb.create_block());
                        if let Some(tramp) = backedge {
                            target = tramp;
                        }
                        if matches!(op, Op::GotoIfNilElsePop(_)) {
                            fb.ins().brif(is_nil, target, &[], fallthrough, &[]);
                        } else {
                            fb.ins().brif(is_nil, fallthrough, &[], target, &[]);
                        }
                        if let Some(tramp) = backedge {
                            fb.switch_to_block(tramp);
                            fb.seal_block(tramp);
                            let (rt, slot) = (
                                rt.as_ref().expect("backedge implies rt"),
                                backedge_counter.expect("backedge implies counter"),
                            );
                            emit_backedge_jump(
                                &mut fb,
                                rt,
                                slot,
                                &mut signal_exit,
                                &vars,
                                cfg.entry_depth[&tu],
                                block_for[&tu],
                                &handlers,
                                &mut pending,
                            );
                        }
                        terminated = true;
                        break;
                    }
                    Op::Switch => {
                        // [dispatch table] -> shim lookup (the interpreter's
                        // exact hash-key semantics) returning the raw fixnum
                        // address; map it onto the statically resolved targets
                        // with a compare chain. Miss -> fall through. A raw
                        // address outside the static set or a mutated table ->
                        // loud signal (out-of-contract self-modification).
                        let rt_ref = rt.as_ref().ok_or(CompileError::UnsupportedOp("switch"))?;
                        let table = stack.pop().ok_or(CompileError::StackUnderflow)?;
                        let dispatch = stack.pop().ok_or(CompileError::StackUnderflow)?;
                        write_stack_to_vars(&mut fb, &vars, &stack);
                        let vmctx = fb.use_var(rt_ref.vmctx_var);
                        let call = fb
                            .ins()
                            .call(rt_ref.refs.switch_lookup, &[vmctx, dispatch, table]);
                        let addr = fb.inst_results(call)[0];
                        let targets = cfg.switch_targets.get(&i).expect("resolved in analyze");
                        let sig = signal_target_for_site(
                            &mut fb,
                            &mut signal_exit,
                            &handlers,
                            &mut pending,
                            &stack,
                        );
                        let fall = block_for[&(i + 1)];
                        // miss -> fall through
                        let miss = fb.ins().icmp_imm(IntCC::Equal, addr, JIT_SWITCH_MISS);
                        let chain = fb.create_block();
                        fb.ins().brif(miss, fall, &[], chain, &[]);
                        fb.switch_to_block(chain);
                        fb.seal_block(chain);
                        // stale (-2): the shim stashed the flow already.
                        let stale = fb.ins().icmp_imm(IntCC::Equal, addr, JIT_SWITCH_STALE);
                        let mut cur_blk = fb.create_block();
                        fb.ins().brif(stale, sig, &[], cur_blk, &[]);
                        for &(raw, target) in targets {
                            fb.switch_to_block(cur_blk);
                            fb.seal_block(cur_blk);
                            let next = fb.create_block();
                            let hit = fb.ins().icmp_imm(IntCC::Equal, addr, raw);
                            if target <= i {
                                // Backward jump-table edge: poll through a
                                // trampoline, exactly like Goto back-edges.
                                let tramp = fb.create_block();
                                fb.ins().brif(hit, tramp, &[], next, &[]);
                                fb.switch_to_block(tramp);
                                fb.seal_block(tramp);
                                let (rt_b, slot) = (
                                    rt.as_ref().expect("backedge implies rt"),
                                    backedge_counter.expect("backedge implies counter"),
                                );
                                emit_backedge_jump(
                                    &mut fb,
                                    rt_b,
                                    slot,
                                    &mut signal_exit,
                                    &vars,
                                    cfg.entry_depth[&target],
                                    block_for[&target],
                                    &handlers,
                                    &mut pending,
                                );
                            } else {
                                fb.ins().brif(hit, block_for[&target], &[], next, &[]);
                            }
                            cur_blk = next;
                        }
                        // Exhausted: a hit whose address is not in the static
                        // set — stash the stale-table signal and propagate.
                        fb.switch_to_block(cur_blk);
                        fb.seal_block(cur_blk);
                        fb.ins().call(rt_ref.refs.switch_stale, &[]);
                        fb.ins().jump(sig, &[]);
                        terminated = true;
                        break;
                    }
                    Op::PushConditionCase(t) | Op::PushConditionCaseRaw(t) | Op::PushCatch(t) => {
                        // Register the handler frame via the shim (interpreter
                        // arm parity), then end the block with an "anchor"
                        // edge: a never-taken branch to the handler target
                        // that (a) guarantees the target block always has a
                        // Cranelift predecessor with every entry var defined
                        // (its real entries are the runtime match dispatches)
                        // and (b) falls through to the protected body.
                        let rt_ref = rt.as_ref().ok_or(CompileError::UnsupportedOp("handler"))?;
                        let tu = *t as usize;
                        let vmctx = fb.use_var(rt_ref.vmctx_var);
                        let t_v = fb.ins().iconst(types::I64, tu as i64);
                        match op {
                            Op::PushConditionCase(_) => {
                                let d_v = fb.ins().iconst(types::I64, stack.len() as i64);
                                fb.ins().call(rt_ref.refs.push_cc, &[vmctx, t_v, d_v]);
                            }
                            Op::PushConditionCaseRaw(_) => {
                                let conditions = stack.pop().ok_or(CompileError::StackUnderflow)?;
                                let d_v = fb.ins().iconst(types::I64, stack.len() as i64);
                                fb.ins()
                                    .call(rt_ref.refs.push_cc_raw, &[vmctx, t_v, d_v, conditions]);
                            }
                            Op::PushCatch(_) => {
                                let tag = stack.pop().ok_or(CompileError::StackUnderflow)?;
                                let d_v = fb.ins().iconst(types::I64, stack.len() as i64);
                                fb.ins()
                                    .call(rt_ref.refs.push_catch, &[vmctx, t_v, d_v, tag]);
                            }
                            _ => unreachable!("matched Push* above"),
                        }
                        write_stack_to_vars(&mut fb, &vars, &stack);
                        // Placeholder error-value slot for the never-taken
                        // anchor edge (real entries define it from the shim).
                        let nil = fb.ins().iconst(types::I64, Value::NIL.bits() as i64);
                        fb.def_var(vars[stack.len()], nil);
                        let never = fb.ins().iconst(types::I8, 0);
                        fb.ins()
                            .brif(never, block_for[&tu], &[], block_for[&(i + 1)], &[]);
                        terminated = true;
                        break;
                    }
                    Op::PopHandler => {
                        // Normal exit from the protected extent: drop the
                        // runtime frame and the static tracking entry.
                        let rt_ref = rt.as_ref().ok_or(CompileError::UnsupportedOp("handler"))?;
                        let vmctx = fb.use_var(rt_ref.vmctx_var);
                        fb.ins().call(rt_ref.refs.pop_handler, &[vmctx]);
                        handlers
                            .pop()
                            .ok_or(CompileError::UnsupportedOp("unbalanced-pophandler"))?;
                    }
                    other => {
                        let spec = spec_sites.get(&i).map(|site| {
                            (
                                site.sym,
                                site.expected_bits,
                                &spec_slots[site.slot] as *const SpecSlot as i64,
                            )
                        });
                        lower_simple_op(
                            &mut fb,
                            i,
                            &mut pending_deopt,
                            &mut signal_exit,
                            constants,
                            &mut stack,
                            rt.as_ref(),
                            &handlers,
                            &mut pending,
                            spec,
                            other,
                            &known_fixnum,
                        )?
                    }
                }
            }
            if !terminated {
                // Fall through into the next leader block (analyze guaranteed it
                // exists and is < n).
                write_stack_to_vars(&mut fb, &vars, &stack);
                fb.ins().jump(block_for[&end], &[]);
            }
            // Fill the precise-deopt exit blocks queued by this block's guards.
            emit_pending_deopts(&mut fb, deopt_refs, &mut pending_deopt);
            // Fill the handler-dispatch blocks queued by this block's signal
            // sites (the builder can switch blocks now that it's terminated).
            if !pending.is_empty() {
                let rt_ref = rt.as_ref().expect("pending dispatches imply rt");
                emit_pending_dispatches(
                    &mut fb,
                    rt_ref,
                    &mut signal_exit,
                    &vars,
                    &block_for,
                    &mut pending,
                )?;
            }
        }

        // Terminate the shared signal block (return STATUS_SIGNAL) iff used.
        if let Some(sb) = signal_exit {
            fb.switch_to_block(sb);
            let code = fb.ins().iconst(types::I64, STATUS_SIGNAL);
            fb.ins().return_(&[code]);
        }

        fb.seal_all_blocks();
        fb.finalize();
    }

    let fid = module
        .declare_function("__neovm_jit_leaf", Linkage::Local, &sig)
        .map_err(|e| CompileError::Backend(BackendError::Define(e.to_string())))?;
    let mut ctx = module.make_context();
    ctx.func = func;
    module
        .define_function(fid, &mut ctx)
        .map_err(|e| CompileError::Backend(BackendError::Define(e.to_string())))?;
    module.clear_context(&mut ctx);

    module
        .finalize_definitions()
        .map_err(|e| CompileError::Backend(BackendError::Finalize(e.to_string())))?;

    let entry = module.get_finalized_function(fid);
    Ok(CompiledLeaf {
        arity,
        // Plain fixed-arity defaults; compile_bytecode_function overrides for
        // &optional/&rest lambda lists.
        required: arity,
        has_rest: false,
        has_binds: ops.iter().any(|o| {
            matches!(
                o,
                Op::VarBind(_)
                    | Op::Unbind(_)
                    | Op::SaveCurrentBuffer
                    | Op::SaveExcursion
                    | Op::SaveRestriction
                    | Op::UnwindProtectPop
            )
        }),
        has_handlers: ops.iter().any(|o| {
            matches!(
                o,
                Op::PushConditionCase(_) | Op::PushConditionCaseRaw(_) | Op::PushCatch(_)
            )
        }),
        spec_slots,
        deopt_spill,
        deopt_meta,
        entry,
        _module: module,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emacs_core::value::LambdaParams;

    fn nullary() -> ByteCodeFunction {
        ByteCodeFunction::new(LambdaParams {
            required: Vec::new(),
            optional: Vec::new(),
            rest: None,
        })
    }

    #[test]
    fn compiles_constant_return() {
        // (lambda () 42)  ==  [Constant(0), Return], constants = [42]
        let c = Value::make_int(42);
        let leaf = lower_nullary_leaf(&[Op::Constant(0), Op::Return], &[c]).unwrap();
        assert_eq!(leaf.call_for_test(&[]), Some(c.bits()));
    }

    #[test]
    fn is_fixnum_const_detects_fixnum_constants_for_guard_elision() {
        // Redundant-guard elimination: a fixnum `iconst` is provably a fixnum, so
        // guard_fixnum elides its runtime guard; a symbol (nil) constant and a
        // computed value are NOT fixnum constants and keep their guards.
        let mut func = Function::with_name_signature(
            UserFuncName::user(0, 0),
            Signature::new(cranelift_codegen::isa::CallConv::SystemV),
        );
        let mut fbctx = FunctionBuilderContext::new();
        let mut fb = FunctionBuilder::new(&mut func, &mut fbctx);
        let block = fb.create_block();
        fb.switch_to_block(block);
        fb.seal_block(block);
        let fixnum = fb.ins().iconst(types::I64, Value::make_int(7).bits() as i64);
        let nil = fb.ins().iconst(types::I64, Value::NIL.bits() as i64);
        let sum = fb.ins().iadd(fixnum, fixnum);
        assert!(is_fixnum_const(&fb, fixnum), "a fixnum iconst is a fixnum constant");
        assert!(!is_fixnum_const(&fb, nil), "nil (symbol tag) is not a fixnum");
        assert!(!is_fixnum_const(&fb, sum), "an iadd result is not a constant");

        // is_known_fixnum additionally recognizes a retag_fixnum output (a
        // range-checked arithmetic result), eliding the re-guard on chained
        // arithmetic; a bare untagged iadd is not recognized.
        let shifted = fb.ins().ishl_imm(sum, FIXNUM_SHIFT as i64);
        let retagged = fb.ins().bor_imm(shifted, FIXNUM_CHECK_VALUE as i64);
        assert!(is_known_fixnum(&fb, retagged), "retag_fixnum output is a known fixnum");
        assert!(is_known_fixnum(&fb, fixnum), "a fixnum constant is a known fixnum");
        assert!(!is_known_fixnum(&fb, sum), "a bare iadd is not a known fixnum");
        assert!(!is_known_fixnum(&fb, nil), "nil is not a known fixnum");
    }

    fn known_fixnum_at(ops: &[Op], constants: &[Value], leader: usize) -> Option<Vec<bool>> {
        let cfg = analyze_cfg(ops, constants, None, 0).unwrap();
        compute_known_fixnum_slots(ops, constants, &cfg).get(&leader).cloned()
    }

    #[test]
    fn cross_block_known_fixnum_propagates_meets_and_loops() {
        // Forward: a fixnum constant flows across a Goto into its successor block.
        let ops = [Op::Constant(0), Op::Goto(2), Op::Return];
        assert_eq!(
            known_fixnum_at(&ops, &[Value::make_int(7)], 2),
            Some(vec![true]),
            "fixnum constant is known-fixnum across a Goto"
        );
        // A non-fixnum constant is NOT known-fixnum across the edge.
        assert_eq!(
            known_fixnum_at(&ops, &[Value::NIL], 2),
            Some(vec![false]),
            "nil is not a known fixnum across a Goto"
        );

        // Merge narrows: fixnum on the then-path, non-fixnum on the else-path.
        let diamond = [
            Op::Constant(0),  // 0: condition
            Op::GotoIfNil(4), // 1: pop, branch to else(4) or fall to then(2)
            Op::Constant(1),  // 2: then -> fixnum
            Op::Goto(5),      // 3
            Op::Constant(2),  // 4: else -> nil (leader); falls through to 5
            Op::Return,       // 5: merge (leader)
        ];
        let cs = [Value::make_int(0), Value::make_int(9), Value::NIL];
        assert_eq!(
            known_fixnum_at(&diamond, &cs, 5),
            Some(vec![false]),
            "merge of fixnum and non-fixnum is not known-fixnum"
        );

        // THE TARGET: a loop induction variable (i=0; while i<10: i=1+i) is
        // proven fixnum at the loop head across the back-edge (the fixpoint).
        let loop_ops = [
            Op::Constant(0),   // 0: i = 0
            Op::StackRef(0),   // 1: loop head (back-edge target): push i
            Op::Constant(1),   // 2: push limit 10
            Op::Lss,           // 3: i < 10
            Op::GotoIfNil(9),  // 4: pop; exit -> 9
            Op::StackRef(0),   // 5: body: push i
            Op::Add1,          // 6: 1+ i
            Op::StackSet(1),   // 7: i = 1+ i
            Op::Goto(1),       // 8: back-edge
            Op::Return,        // 9: exit
        ];
        let lc = [Value::make_int(0), Value::make_int(10)];
        assert_eq!(
            known_fixnum_at(&loop_ops, &lc, 1),
            Some(vec![true]),
            "loop induction variable is known-fixnum at the loop head"
        );
    }

    #[test]
    fn compiles_nil_and_true() {
        assert_eq!(
            lower_nullary_leaf(&[Op::Nil, Op::Return], &[])
                .unwrap()
                .call_for_test(&[]),
            Some(Value::NIL.bits())
        );
        assert_eq!(
            lower_nullary_leaf(&[Op::True, Op::Return], &[])
                .unwrap()
                .call_for_test(&[]),
            Some(Value::T.bits())
        );
    }

    #[test]
    fn dup_and_pop_select_the_right_value() {
        // [Const(0), Const(1), Dup, Pop, Return] -> top is constants[1]
        let a = Value::make_int(7);
        let b = Value::make_int(9);
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::Dup,
                Op::Pop,
                Op::Return,
            ],
            &[a, b],
        )
        .unwrap();
        assert_eq!(leaf.call_for_test(&[]), Some(b.bits()));
    }

    #[test]
    fn stackref_reaches_below_top() {
        // [Const(0), Const(1), StackRef(1), Return] -> pushes a copy of a, returns a
        let a = Value::make_int(100);
        let b = Value::make_int(200);
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::StackRef(1),
                Op::Return,
            ],
            &[a, b],
        )
        .unwrap();
        assert_eq!(leaf.call_for_test(&[]), Some(a.bits()));
    }

    #[test]
    fn compiles_fixnum_add() {
        // (+ 40 2) -> 42, all fixnums in range
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Add, Op::Return],
            &[Value::make_int(40), Value::make_int(2)],
        )
        .unwrap();
        assert_eq!(leaf.call_for_test(&[]), Some(Value::make_int(42).bits()));
    }

    #[test]
    fn compiles_fixnum_sub_including_negative() {
        // (- 3 10) -> -7
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Sub, Op::Return],
            &[Value::make_int(3), Value::make_int(10)],
        )
        .unwrap();
        assert_eq!(leaf.call_for_test(&[]), Some(Value::make_int(-7).bits()));
    }

    #[test]
    fn add_overflowing_fixnum_range_deopts() {
        // MOST_POSITIVE_FIXNUM + 1 leaves fixnum range -> deopt (None), so the
        // interpreter can promote to a bignum.
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Add, Op::Return],
            &[
                Value::make_int(Value::MOST_POSITIVE_FIXNUM),
                Value::make_int(1),
            ],
        )
        .unwrap();
        assert_eq!(leaf.call_for_test(&[]), None);
    }

    #[test]
    fn add_non_fixnum_operand_deopts() {
        // a = fixnum 5, b = nil -> not both fixnums -> deopt.
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Nil, Op::Add, Op::Return],
            &[Value::make_int(5)],
        )
        .unwrap();
        assert_eq!(leaf.call_for_test(&[]), None);
    }

    #[test]
    fn add_then_sub_chain() {
        // ((1 + 2) - 4) = -1
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::Add,
                Op::Constant(2),
                Op::Sub,
                Op::Return,
            ],
            &[Value::make_int(1), Value::make_int(2), Value::make_int(4)],
        )
        .unwrap();
        assert_eq!(leaf.call_for_test(&[]), Some(Value::make_int(-1).bits()));
    }

    #[test]
    fn compiles_unary_fixnum_ops() {
        // 1+ 41 -> 42
        let add1 = lower_nullary_leaf(
            &[Op::Constant(0), Op::Add1, Op::Return],
            &[Value::make_int(41)],
        )
        .unwrap();
        assert_eq!(add1.call_for_test(&[]), Some(Value::make_int(42).bits()));

        // 1- 43 -> 42
        let sub1 = lower_nullary_leaf(
            &[Op::Constant(0), Op::Sub1, Op::Return],
            &[Value::make_int(43)],
        )
        .unwrap();
        assert_eq!(sub1.call_for_test(&[]), Some(Value::make_int(42).bits()));

        // - 42 -> -42
        let neg = lower_nullary_leaf(
            &[Op::Constant(0), Op::Negate, Op::Return],
            &[Value::make_int(42)],
        )
        .unwrap();
        assert_eq!(neg.call_for_test(&[]), Some(Value::make_int(-42).bits()));
    }

    #[test]
    fn unary_boundary_inputs_deopt() {
        // 1+ MOST_POSITIVE -> overflow -> deopt
        let add1 = lower_nullary_leaf(
            &[Op::Constant(0), Op::Add1, Op::Return],
            &[Value::make_int(Value::MOST_POSITIVE_FIXNUM)],
        )
        .unwrap();
        assert_eq!(add1.call_for_test(&[]), None);

        // 1- MOST_NEGATIVE -> underflow -> deopt
        let sub1 = lower_nullary_leaf(
            &[Op::Constant(0), Op::Sub1, Op::Return],
            &[Value::make_int(Value::MOST_NEGATIVE_FIXNUM)],
        )
        .unwrap();
        assert_eq!(sub1.call_for_test(&[]), None);

        // - MOST_NEGATIVE -> +MOST_POSITIVE+1 out of range -> deopt
        let neg = lower_nullary_leaf(
            &[Op::Constant(0), Op::Negate, Op::Return],
            &[Value::make_int(Value::MOST_NEGATIVE_FIXNUM)],
        )
        .unwrap();
        assert_eq!(neg.call_for_test(&[]), None);
    }

    #[test]
    fn unary_on_non_fixnum_deopts() {
        // 1+ t -> not a fixnum -> deopt
        let leaf = lower_nullary_leaf(&[Op::True, Op::Add1, Op::Return], &[]).unwrap();
        assert_eq!(leaf.call_for_test(&[]), None);
    }

    #[test]
    fn compiles_fixnum_comparisons() {
        fn cmp(ops: &[Op], a: i64, b: i64) -> Option<usize> {
            lower_nullary_leaf(ops, &[Value::make_int(a), Value::make_int(b)])
                .unwrap()
                .call_for_test(&[])
        }
        let t = Some(Value::T.bits());
        let nil = Some(Value::NIL.bits());
        assert_eq!(
            cmp(
                &[Op::Constant(0), Op::Constant(1), Op::Lss, Op::Return],
                3,
                5
            ),
            t
        );
        assert_eq!(
            cmp(
                &[Op::Constant(0), Op::Constant(1), Op::Lss, Op::Return],
                5,
                3
            ),
            nil
        );
        assert_eq!(
            cmp(
                &[Op::Constant(0), Op::Constant(1), Op::Gtr, Op::Return],
                5,
                3
            ),
            t
        );
        assert_eq!(
            cmp(
                &[Op::Constant(0), Op::Constant(1), Op::Leq, Op::Return],
                4,
                4
            ),
            t
        );
        assert_eq!(
            cmp(
                &[Op::Constant(0), Op::Constant(1), Op::Geq, Op::Return],
                4,
                5
            ),
            nil
        );
        assert_eq!(
            cmp(
                &[Op::Constant(0), Op::Constant(1), Op::Eqlsign, Op::Return],
                7,
                7
            ),
            t
        );
        assert_eq!(
            cmp(
                &[Op::Constant(0), Op::Constant(1), Op::Eqlsign, Op::Return],
                7,
                8
            ),
            nil
        );
    }

    #[test]
    fn comparison_on_non_fixnum_deopts() {
        // (< 1 nil) -> nil isn't a fixnum -> deopt.
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Nil, Op::Lss, Op::Return],
            &[Value::make_int(1)],
        )
        .unwrap();
        assert_eq!(leaf.call_for_test(&[]), None);
    }

    #[test]
    fn compiles_if_branch() {
        // (lambda (x) (if x 1 2)):
        //  0 StackRef(0); 1 GotoIfNil(4); 2 Constant(0=>1); 3 Return;
        //  4 Constant(1=>2); 5 Return
        let f = lower_leaf(
            &[
                Op::StackRef(0),
                Op::GotoIfNil(4),
                Op::Constant(0),
                Op::Return,
                Op::Constant(1),
                Op::Return,
            ],
            &[Value::make_int(1), Value::make_int(2)],
            1,
        )
        .unwrap();
        assert_eq!(
            f.call_for_test(&[Value::T]),
            Some(Value::make_int(1).bits())
        );
        assert_eq!(
            f.call_for_test(&[Value::make_int(99)]),
            Some(Value::make_int(1).bits())
        );
        assert_eq!(
            f.call_for_test(&[Value::NIL]),
            Some(Value::make_int(2).bits())
        );
    }

    #[test]
    fn compiles_goto_if_not_nil() {
        // jumps to the second arm when the arg is non-nil.
        let f = lower_leaf(
            &[
                Op::StackRef(0),
                Op::GotoIfNotNil(4),
                Op::Constant(0),
                Op::Return,
                Op::Constant(1),
                Op::Return,
            ],
            &[Value::make_int(1), Value::make_int(2)],
            1,
        )
        .unwrap();
        assert_eq!(
            f.call_for_test(&[Value::NIL]),
            Some(Value::make_int(1).bits())
        );
        assert_eq!(
            f.call_for_test(&[Value::T]),
            Some(Value::make_int(2).bits())
        );
    }

    #[test]
    fn compiles_goto_if_nil_else_pop() {
        // (lambda (x) (and x 7)) shape:
        //  0 StackRef(0); 1 GotoIfNilElsePop(3); 2 Constant(0=>7); 3 Return
        // x nil  -> jump keeping x -> return x (nil);
        // x else -> pop x, push 7 -> return 7.  A join with differing stacks (phi).
        let f = lower_leaf(
            &[
                Op::StackRef(0),
                Op::GotoIfNilElsePop(3),
                Op::Constant(0),
                Op::Return,
            ],
            &[Value::make_int(7)],
            1,
        )
        .unwrap();
        assert_eq!(
            f.call_for_test(&[Value::make_int(5)]),
            Some(Value::make_int(7).bits())
        );
        assert_eq!(f.call_for_test(&[Value::NIL]), Some(Value::NIL.bits()));
    }

    #[test]
    fn compiles_unconditional_goto() {
        //  0 Goto(1); 1 Constant(0=>5); 2 Return
        let f = lower_leaf(
            &[Op::Goto(1), Op::Constant(0), Op::Return],
            &[Value::make_int(5)],
            0,
        )
        .unwrap();
        assert_eq!(f.call_for_test(&[]), Some(Value::make_int(5).bits()));
    }

    #[test]
    fn jit_matches_interpreter_on_if_branch() {
        use crate::emacs_core::bytecode::Vm;
        use crate::emacs_core::eval::Context;
        let ops = [
            Op::StackRef(0),
            Op::GotoIfNil(4),
            Op::Constant(0),
            Op::Return,
            Op::Constant(1),
            Op::Return,
        ];
        let constants = [Value::make_int(10), Value::make_int(20)];
        for arg in [Value::T, Value::NIL, Value::make_int(3)] {
            let mut eval = Context::new_minimal_vm_harness();
            let mut f = ByteCodeFunction::new(LambdaParams {
                required: vec![crate::emacs_core::intern::SymId(1)],
                optional: Vec::new(),
                rest: None,
            });
            f.lexical = true;
            f.ops = ops.to_vec();
            f.constants = constants.to_vec();
            f.max_stack = 16;
            let want = {
                let mut vm = Vm::from_context(&mut eval);
                vm.execute(&f, vec![arg]).expect("interp runs if").bits()
            };
            let got = lower_leaf(&ops, &constants, 1)
                .unwrap()
                .call_for_test(&[arg]);
            assert_eq!(
                got,
                Some(want),
                "if-branch mismatch for arg bits {}",
                arg.bits()
            );
            // Also via the typed-MIR Tier-2 path (probe lower_mir_pure control flow).
            if let Ok(mir) = mir::build_mir(&ops, &constants, 1) {
                if let Ok(mleaf) = lower_mir_pure(&mir) {
                    let ctx_ptr = &mut eval as *mut Context as *mut u8;
                    if let NativeRun::Ok(bits) = mleaf.call(ctx_ptr, &[arg]) {
                        assert_eq!(
                            bits, want,
                            "MIR if-branch mismatch for arg bits {}",
                            arg.bits()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn compiles_stackset() {
        // (lambda (a) (setq a (1+ a)) a):
        //  0 StackRef(0); 1 Add1; 2 StackSet(1); 3 StackRef(0); 4 Return
        let f = lower_leaf(
            &[
                Op::StackRef(0),
                Op::Add1,
                Op::StackSet(1),
                Op::StackRef(0),
                Op::Return,
            ],
            &[],
            1,
        )
        .unwrap();
        assert_eq!(
            f.call_for_test(&[Value::make_int(41)]),
            Some(Value::make_int(42).bits())
        );
    }

    #[test]
    fn compiles_discardn() {
        let consts = &[
            Value::make_int(10),
            Value::make_int(20),
            Value::make_int(30),
        ];
        // Non-preserve: push 10,20,30; discard top 2 -> 10.
        let np = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::Constant(2),
                Op::DiscardN(2),
                Op::Return,
            ],
            consts,
        )
        .unwrap();
        assert_eq!(np.call_for_test(&[]), Some(Value::make_int(10).bits()));
        // Preserve TOS: push 10,20,30; discardN(2 | 0x80) keeps 30 -> 30.
        let pr = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::Constant(2),
                Op::DiscardN(0x82),
                Op::Return,
            ],
            consts,
        )
        .unwrap();
        assert_eq!(pr.call_for_test(&[]), Some(Value::make_int(30).bits()));
    }

    #[test]
    fn compiles_countdown_loop_matches_interpreter() {
        use crate::emacs_core::bytecode::Vm;
        use crate::emacs_core::eval::Context;
        // (lambda (n) (while (> n 0) (setq n (1- n))) n) -> 0. A back-edge loop:
        //  0 StackRef(0); 1 Constant(0=>0); 2 Gtr; 3 GotoIfNil(8);
        //  4 StackRef(0); 5 Sub1; 6 StackSet(1); 7 Goto(0);
        //  8 StackRef(0); 9 Return
        let ops = [
            Op::StackRef(0),
            Op::Constant(0),
            Op::Gtr,
            Op::GotoIfNil(8),
            Op::StackRef(0),
            Op::Sub1,
            Op::StackSet(1),
            Op::Goto(0),
            Op::StackRef(0),
            Op::Return,
        ];
        let constants = [Value::make_int(0)];
        for n in [0i64, 1, 4, 9] {
            let mut eval = Context::new_minimal_vm_harness();
            let mut f = ByteCodeFunction::new(LambdaParams {
                required: vec![crate::emacs_core::intern::SymId(1)],
                optional: Vec::new(),
                rest: None,
            });
            f.lexical = true;
            f.ops = ops.to_vec();
            f.constants = constants.to_vec();
            f.max_stack = 16;
            let want = {
                let mut vm = Vm::from_context(&mut eval);
                vm.execute(&f, vec![Value::make_int(n)])
                    .expect("interp loop")
                    .bits()
            };
            let got = lower_leaf(&ops, &constants, 1)
                .unwrap()
                .call_for_test(&[Value::make_int(n)]);
            assert_eq!(got, Some(want), "loop mismatch for n={n}");
            assert_eq!(
                got,
                Some(Value::make_int(0).bits()),
                "countdown should reach 0 (n={n})"
            );
            // Also via the typed-MIR Tier-2 path (probe lower_mir_pure loops/back-edges).
            if let Ok(mir) = mir::build_mir(&ops, &constants, 1) {
                if let Ok(mleaf) = lower_mir_pure(&mir) {
                    let ctx_ptr = &mut eval as *mut Context as *mut u8;
                    if let NativeRun::Ok(bits) = mleaf.call(ctx_ptr, &[Value::make_int(n)]) {
                        assert_eq!(bits, want, "MIR loop mismatch for n={n}");
                    }
                }
            }
        }
    }

    #[test]
    fn mir_merge_phi_matches_interpreter() {
        use crate::emacs_core::bytecode::Vm;
        use crate::emacs_core::eval::Context;
        // (lambda (c) (1+ (if c 10 20))) — a diamond whose then/else values merge
        // at a common block (a phi), consumed by Add1. Tests build_mir's merge-phi.
        let ops = [
            Op::StackRef(0),  // 0: cond
            Op::GotoIfNil(4), // 1: pop; else->4, fall to then->2
            Op::Constant(0),  // 2: then: 10
            Op::Goto(5),      // 3
            Op::Constant(1),  // 4: else: 20 (leader); falls through to 5
            Op::Add1,         // 5: merge: 1+ phi (leader)
            Op::Return,       // 6
        ];
        let constants = [Value::make_int(10), Value::make_int(20)];
        for c in [Value::T, Value::NIL] {
            let mut eval = Context::new_minimal_vm_harness();
            let mut f = ByteCodeFunction::new(LambdaParams {
                required: vec![crate::emacs_core::intern::SymId(1)],
                optional: Vec::new(),
                rest: None,
            });
            f.lexical = true;
            f.ops = ops.to_vec();
            f.constants = constants.to_vec();
            f.max_stack = 16;
            let want = {
                let mut vm = Vm::from_context(&mut eval);
                vm.execute(&f, vec![c]).expect("interp diamond").bits()
            };
            if let Ok(mir) = mir::build_mir(&ops, &constants, 1) {
                if let Ok(mleaf) = lower_mir_pure(&mir) {
                    let ctx_ptr = &mut eval as *mut Context as *mut u8;
                    if let NativeRun::Ok(bits) = mleaf.call(ctx_ptr, &[c]) {
                        assert_eq!(
                            bits, want,
                            "MIR merge-phi mismatch for cond bits {}",
                            c.bits()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn mir_multi_phi_merge_matches_interpreter() {
        use crate::emacs_core::bytecode::Vm;
        use crate::emacs_core::eval::Context;
        // A diamond where BOTH branches leave TWO values on the stack, so the
        // merge needs TWO phis; then Sub consumes them. Compares MIR to the
        // interpreter (ground truth) — no manual expected value.
        let ops = [
            Op::StackRef(0),  // 0: cond, depth 2
            Op::GotoIfNil(5), // 1: pop; else->5, fall->2  (depth 1)
            Op::Constant(0),  // 2: then: 10  (depth 2)
            Op::Constant(1),  // 3:        20 (depth 3)
            Op::Goto(7),      // 4: -> merge(7)
            Op::Constant(1),  // 5: else: 20 (depth 2) [leader]
            Op::Constant(0),  // 6:        10 (depth 3)  falls to 7
            Op::Sub,          // 7: merge: two phis -> Sub (depth 2) [leader]
            Op::Return,       // 8
        ];
        let constants = [Value::make_int(10), Value::make_int(20)];
        for c in [Value::T, Value::NIL] {
            let mut eval = Context::new_minimal_vm_harness();
            let mut f = ByteCodeFunction::new(LambdaParams {
                required: vec![crate::emacs_core::intern::SymId(1)],
                optional: Vec::new(),
                rest: None,
            });
            f.lexical = true;
            f.ops = ops.to_vec();
            f.constants = constants.to_vec();
            f.max_stack = 16;
            let want = {
                let mut vm = Vm::from_context(&mut eval);
                vm.execute(&f, vec![c]).expect("interp multi-phi").bits()
            };
            if let Ok(mir) = mir::build_mir(&ops, &constants, 1) {
                if let Ok(mleaf) = lower_mir_pure(&mir) {
                    let ctx_ptr = &mut eval as *mut Context as *mut u8;
                    if let NativeRun::Ok(bits) = mleaf.call(ctx_ptr, &[c]) {
                        assert_eq!(
                            bits, want,
                            "MIR multi-phi-merge mismatch for cond bits {}",
                            c.bits()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn backedge_polls_quit_like_the_interpreter() {
        use crate::emacs_core::bytecode::Vm;
        use crate::emacs_core::eval::Context;
        // Countdown loop with enough iterations (> 255 backward jumps) for the
        // u8 quit counter to wrap and trigger the back-edge service poll.
        let ops = [
            Op::StackRef(0),
            Op::Constant(0),
            Op::Gtr,
            Op::GotoIfNil(8),
            Op::StackRef(0),
            Op::Sub1,
            Op::StackSet(1),
            Op::Goto(0),
            Op::StackRef(0),
            Op::Return,
        ];
        let constants = [Value::make_int(0)];
        let mut ev = Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut Context as *mut u8;
        let leaf = lower_leaf(&ops, &constants, 1).unwrap();

        // Flag clear: the loop runs to completion natively (polls return OK).
        assert_eq!(
            leaf.call(ctx_ptr, &[Value::make_int(1000)]),
            NativeRun::Ok(Value::make_int(0).bits())
        );

        // Flag set: the wrap poll must signal quit out of native code...
        ev.set_quit_flag_value(Value::T);
        assert_eq!(
            leaf.call(ctx_ptr, &[Value::make_int(1000)]),
            NativeRun::Signal,
            "C-g must interrupt a compiled loop"
        );
        assert!(take_pending_flow().is_some(), "quit Flow stashed");

        // ...exactly like the interpreter on the same body (the poll clears the
        // flag, so re-set it for the oracle run).
        ev.set_quit_flag_value(Value::T);
        let mut f = ByteCodeFunction::new(LambdaParams {
            required: vec![crate::emacs_core::intern::SymId(1)],
            optional: Vec::new(),
            rest: None,
        });
        f.lexical = true;
        f.ops = ops.to_vec();
        f.constants = constants.to_vec();
        f.max_stack = 16;
        let interp = {
            let mut vm = Vm::from_context(&mut ev);
            vm.execute(&f, vec![Value::make_int(1000)])
        };
        assert!(interp.is_err(), "interpreter quits on the same loop");

        // Flag cleared by the quit: the loop completes again.
        assert_eq!(
            leaf.call(ctx_ptr, &[Value::make_int(1000)]),
            NativeRun::Ok(Value::make_int(0).bits())
        );
    }

    #[test]
    fn compiles_save_excursion_with_unwind_semantics() {
        use crate::emacs_core::eval::Context;
        let mut ev = Context::new();
        let ctx_ptr = &mut ev as *mut Context as *mut u8;
        ev.eval_str(r#"(insert "hello world")"#).expect("insert");
        let specpdl_before = ev.specpdl.len();
        let constants = [
            Value::symbol("goto-char"),
            Value::make_int(1),
            Value::symbol("point"),
        ];

        // Balanced: (save-excursion (goto-char 1)) then (point) — restored.
        let balanced = lower_nullary_leaf(
            &[
                Op::SaveExcursion,
                Op::Constant(0),
                Op::Constant(1),
                Op::Call(1),
                Op::Pop,
                Op::Unbind(1),
                Op::Constant(2),
                Op::Call(0),
                Op::Return,
            ],
            &constants,
        )
        .unwrap();
        assert_eq!(
            balanced.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(12).bits()),
            "point must be restored by the Unbind"
        );
        assert_eq!(ev.specpdl.len(), specpdl_before);

        // Early return with the record dangling: the frame unwind restores it.
        let dangling = lower_nullary_leaf(
            &[
                Op::SaveExcursion,
                Op::Constant(0),
                Op::Constant(1),
                Op::Call(1),
                Op::Return,
            ],
            &constants,
        )
        .unwrap();
        assert_eq!(
            dangling.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(1).bits())
        );
        assert_eq!(ev.specpdl.len(), specpdl_before, "frame unwind pops record");
        let point_now =
            lower_nullary_leaf(&[Op::Constant(2), Op::Call(0), Op::Return], &constants).unwrap();
        assert_eq!(
            point_now.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(12).bits()),
            "point must be restored by the frame unwind too"
        );

        // SaveCurrentBuffer / SaveRestriction: records create + frame-unwind
        // cleanly (same shim/record machinery; arms mirrored 1:1).
        for op in [Op::SaveCurrentBuffer, Op::SaveRestriction] {
            let mech = lower_nullary_leaf(&[op, Op::Nil, Op::Return], &[]).unwrap();
            assert_eq!(mech.call(ctx_ptr, &[]), NativeRun::Ok(Value::NIL.bits()));
            assert_eq!(ev.specpdl.len(), specpdl_before);
        }

        // Precise deopt: a guard after the Save* record compiles and runs
        // (a failing guard would resume the interpreter mid-frame with the
        // record still registered).
        let after = lower_nullary_leaf(
            &[Op::SaveExcursion, Op::Constant(1), Op::Add1, Op::Return],
            &constants,
        )
        .expect("guard after a side effect compiles under precise deopt");
        match after.call(ctx_ptr, &[]) {
            NativeRun::Ok(_) => {}
            other => panic!("guard-after-save must run, got {other:?}"),
        }
        assert_eq!(ev.specpdl.len(), specpdl_before);
    }

    #[test]
    fn compiles_trivial_natives_carsafe_maxmin_throw_numpreds() {
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        let t = NativeRun::Ok(Value::T.bits());
        let nil = NativeRun::Ok(Value::NIL.bits());
        let run1 = |op: Op, v: Value, ctx: *mut u8| {
            lower_nullary_leaf(&[Op::Constant(0), op, Op::Return], &[v])
                .unwrap()
                .call(ctx, &[])
        };

        // car-safe / cdr-safe: total — non-cons (incl. fixnums) -> nil, no deopt.
        let cons = Value::cons(Value::make_int(3), Value::make_int(4));
        assert_eq!(
            run1(Op::CarSafe, cons, ctx_ptr),
            NativeRun::Ok(Value::make_int(3).bits())
        );
        assert_eq!(
            run1(Op::CdrSafe, cons, ctx_ptr),
            NativeRun::Ok(Value::make_int(4).bits())
        );
        assert_eq!(run1(Op::CarSafe, Value::make_int(9), ctx_ptr), nil);
        assert_eq!(run1(Op::CdrSafe, Value::T, ctx_ptr), nil);
        assert_eq!(run1(Op::CarSafe, Value::NIL, ctx_ptr), nil);

        // max / min: fixnum fast path keeps the original tagged operand;
        // non-fixnum deopts to the interpreter's coercing builtin.
        let run2 = |op: Op, a: Value, b: Value, ctx: *mut u8| {
            lower_nullary_leaf(&[Op::Constant(0), Op::Constant(1), op, Op::Return], &[a, b])
                .unwrap()
                .call(ctx, &[])
        };
        assert_eq!(
            run2(Op::Max, Value::make_int(3), Value::make_int(7), ctx_ptr),
            NativeRun::Ok(Value::make_int(7).bits())
        );
        assert_eq!(
            run2(Op::Max, Value::make_int(-3), Value::make_int(-7), ctx_ptr),
            NativeRun::Ok(Value::make_int(-3).bits())
        );
        assert_eq!(
            run2(Op::Min, Value::make_int(3), Value::make_int(7), ctx_ptr),
            NativeRun::Ok(Value::make_int(3).bits())
        );
        // Non-fixnum operand: precise deopt at the Max op with the operands
        // still on the captured stack.
        match run2(Op::Max, Value::make_float(1.5), Value::make_int(7), ctx_ptr) {
            NativeRun::DeoptAt { pc, stack, .. } => {
                assert_eq!(pc, 2, "deopt at the Max op");
                assert_eq!(stack[1], Value::make_int(7));
            }
            other => panic!("expected a precise deopt, got {other:?}"),
        }

        // integerp / numberp: fixnum natively; float/bignum via the slow shim.
        assert_eq!(run1(Op::Integerp, Value::make_int(5), ctx_ptr), t);
        assert_eq!(run1(Op::Integerp, Value::make_float(1.5), ctx_ptr), nil);
        assert_eq!(run1(Op::Integerp, Value::T, ctx_ptr), nil);
        assert_eq!(run1(Op::Numberp, Value::make_int(5), ctx_ptr), t);
        assert_eq!(run1(Op::Numberp, Value::make_float(1.5), ctx_ptr), t);
        assert_eq!(run1(Op::Numberp, Value::NIL, ctx_ptr), nil);

        // throw: stashes Flow::Throw and exits via the signal path.
        let tag = Value::symbol("jit-throw-tag");
        let thrown = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Throw],
            &[tag, Value::make_int(42)],
        )
        .unwrap();
        assert_eq!(thrown.call(ctx_ptr, &[]), NativeRun::Signal);
        match take_pending_flow().expect("throw Flow stashed") {
            Flow::Throw {
                tag: got_tag,
                value,
            } => {
                assert_eq!(got_tag, tag);
                assert_eq!(value, Value::make_int(42));
            }
            other => panic!("expected Flow::Throw, got {other:?}"),
        }
    }

    #[test]
    fn compiles_direct_builtin_ops() {
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        let ok_int = |n: i64| NativeRun::Ok(Value::make_int(n).bits());
        let run = |ops: &[Op], consts: &[Value], ctx: *mut u8| {
            lower_nullary_leaf(ops, consts).unwrap().call(ctx, &[])
        };

        // length
        let list = Value::cons(
            Value::make_int(1),
            Value::cons(
                Value::make_int(2),
                Value::cons(Value::make_int(3), Value::NIL),
            ),
        );
        assert_eq!(
            run(&[Op::Constant(0), Op::Length, Op::Return], &[list], ctx_ptr),
            ok_int(3)
        );

        // nth: (nth 1 '(1 2 3)) = 2 — operand order matches the arm (n, list).
        assert_eq!(
            run(
                &[Op::Constant(0), Op::Constant(1), Op::Nth, Op::Return],
                &[Value::make_int(1), list],
                ctx_ptr
            ),
            ok_int(2)
        );

        // memq: (memq 'b '(a b c)) -> the tail whose car is 'b.
        let (a, bsym, c) = (
            Value::symbol("jit-memq-a"),
            Value::symbol("jit-memq-b"),
            Value::symbol("jit-memq-c"),
        );
        let abc = Value::cons(a, Value::cons(bsym, Value::cons(c, Value::NIL)));
        let NativeRun::Ok(tail) = run(
            &[Op::Constant(0), Op::Constant(1), Op::Memq, Op::Return],
            &[bsym, abc],
            ctx_ptr,
        ) else {
            panic!("memq must succeed");
        };
        assert_eq!(Value::from_bits(tail).cons_car(), bsym);

        // equal on structurally-equal fresh lists -> t.
        let l1 = Value::cons(
            Value::make_int(1),
            Value::cons(Value::make_int(2), Value::NIL),
        );
        let l2 = Value::cons(
            Value::make_int(1),
            Value::cons(Value::make_int(2), Value::NIL),
        );
        assert_eq!(
            run(
                &[Op::Constant(0), Op::Constant(1), Op::Equal, Op::Return],
                &[l1, l2],
                ctx_ptr
            ),
            NativeRun::Ok(Value::T.bits())
        );

        // setcar mutates through the SATB-barriered builtin; result = new car.
        let cell = Value::cons(Value::make_int(10), Value::make_int(20));
        assert_eq!(
            run(
                &[Op::Constant(0), Op::Constant(1), Op::Setcar, Op::Return],
                &[cell, Value::make_int(99)],
                ctx_ptr
            ),
            ok_int(99)
        );
        assert_eq!(cell.cons_car(), Value::make_int(99), "mutation visible");

        // Precise deopt: a guard after the mutation compiles and runs —
        // (1+ (setcar cell 1)) = 2 with the mutation visible.
        assert_eq!(
            run(
                &[
                    Op::Constant(0),
                    Op::Constant(1),
                    Op::Setcar,
                    Op::Add1,
                    Op::Return,
                ],
                &[cell, Value::make_int(1)],
                ctx_ptr
            ),
            ok_int(2)
        );
        assert_eq!(cell.cons_car(), Value::make_int(1), "mutation visible");

        // symbol-value: live read + void-variable signal.
        let var = Value::symbol("jit-bw-var");
        let crate::emacs_core::value::ValueKind::Symbol(var_id) = var.kind() else {
            panic!("symbol expected");
        };
        ev.obarray.set_symbol_value_id(var_id, Value::make_int(5));
        assert_eq!(
            run(
                &[Op::Constant(0), Op::SymbolValue, Op::Return],
                &[var],
                ctx_ptr
            ),
            ok_int(5)
        );
        let unbound = Value::symbol("jit-bw-unbound");
        assert_eq!(
            run(
                &[Op::Constant(0), Op::SymbolValue, Op::Return],
                &[unbound],
                ctx_ptr
            ),
            NativeRun::Signal
        );
        assert!(take_pending_flow().is_some());

        // put / get round-trip on a plist.
        let psym = Value::symbol("jit-bw-plist");
        let prop = Value::symbol("jit-bw-prop");
        assert_eq!(
            run(
                &[
                    Op::Constant(0),
                    Op::Constant(1),
                    Op::Constant(2),
                    Op::Put,
                    Op::Return,
                ],
                &[psym, prop, Value::make_int(7)],
                ctx_ptr
            ),
            ok_int(7)
        );
        assert_eq!(
            run(
                &[Op::Constant(0), Op::Constant(1), Op::Get, Op::Return],
                &[psym, prop],
                ctx_ptr
            ),
            ok_int(7)
        );

        // aref on a string; string-equal.
        let s = Value::string("abc");
        assert_eq!(
            run(
                &[Op::Constant(0), Op::Constant(1), Op::Aref, Op::Return],
                &[s, Value::make_int(1)],
                ctx_ptr
            ),
            ok_int('b' as i64)
        );
        let s2 = Value::string("abc");
        assert_eq!(
            run(
                &[
                    Op::Constant(0),
                    Op::Constant(1),
                    Op::StringEqual,
                    Op::Return
                ],
                &[s, s2],
                ctx_ptr
            ),
            NativeRun::Ok(Value::T.bits())
        );
    }

    #[test]
    fn compiles_unwind_protect_pop() {
        use crate::emacs_core::eval::Context;
        let mut ev = Context::new();
        let ctx_ptr = &mut ev as *mut Context as *mut u8;
        // NOTE: the opcode's operand is a LIST of cleanup forms (sf_progn_value),
        // exactly what the byte-compiler pushes for (unwind-protect BODY FORMS..).
        let cleanup = ev
            .eval_str("'((setq jit-up-ran t))")
            .expect("cleanup forms");
        ev.eval_str("(setq jit-up-ran nil)").expect("flag init");
        let specpdl_before = ev.specpdl.len();
        let consts = [
            cleanup,
            Value::make_int(7),
            Value::symbol("jit-up-no-such-fn"),
        ];

        // Balanced: the matching Unbind runs the cleanup.
        let balanced = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::UnwindProtectPop,
                Op::Constant(1),
                Op::Unbind(1),
                Op::Return,
            ],
            &consts,
        )
        .unwrap();
        assert_eq!(
            balanced.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(7).bits())
        );
        assert_eq!(
            ev.eval_str("jit-up-ran").unwrap(),
            Value::T,
            "cleanup ran on the balanced path"
        );
        assert_eq!(ev.specpdl.len(), specpdl_before);

        // Signal inside the protected extent: the frame unwind runs the cleanup.
        ev.eval_str("(setq jit-up-ran nil)").expect("flag reset");
        let signaled = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::UnwindProtectPop,
                Op::Constant(2),
                Op::Call(0),
                Op::Return,
            ],
            &consts,
        )
        .unwrap();
        assert_eq!(signaled.call(ctx_ptr, &[]), NativeRun::Signal);
        assert!(take_pending_flow().is_some());
        assert_eq!(
            ev.eval_str("jit-up-ran").unwrap(),
            Value::T,
            "cleanup ran on the signal path"
        );
        assert_eq!(ev.specpdl.len(), specpdl_before);
    }

    /// MIR Tier-2 Phase 4b: a pure body lowered bytecode→MIR→CLIF produces the
    /// SAME native result as the interpreter — the first end-to-end proof of the
    /// MIR pipeline.
    #[test]
    fn mir_pure_lowering_matches_interpreter() {
        use crate::emacs_core::bytecode::ByteCodeFunction;
        use crate::emacs_core::value::LambdaParams;

        let cases: Vec<(Vec<Op>, Vec<Value>, usize, Vec<Value>)> = vec![
            // (lambda (a b) (+ a b)) on (40, 2) -> 42.
            (
                vec![Op::StackRef(1), Op::StackRef(1), Op::Add, Op::Return],
                vec![],
                2,
                vec![Value::make_int(40), Value::make_int(2)],
            ),
            // (lambda (n) (if (< n 2) n (1- n))) — branch + arithmetic.
            (
                vec![
                    Op::StackRef(0),
                    Op::Constant(0),
                    Op::Lss,
                    Op::GotoIfNil(6),
                    Op::StackRef(0),
                    Op::Return,
                    Op::StackRef(0),
                    Op::Sub1,
                    Op::Return,
                ],
                vec![Value::make_int(2)],
                1,
                vec![Value::make_int(9)],
            ),
            // Pure countdown loop: (lambda (n) (let ((acc 0)) (while (> n 0)
            // (setq acc (+ acc n)) (setq n (1- n))) acc)).
            (
                vec![
                    Op::Constant(0),   // 0  acc=0      [n 0]
                    Op::StackRef(1),   // 1  [n acc n]   <- head
                    Op::Constant(0),   // 2  0
                    Op::Gtr,           // 3  [n acc c]
                    Op::GotoIfNil(13), // 4  [n acc]
                    Op::StackRef(1),   // 5  n
                    Op::StackRef(1),   // 6  acc
                    Op::Add,           // 7  acc'
                    Op::StackSet(1),   // 8  [n acc']
                    Op::StackRef(1),   // 9  n
                    Op::Sub1,          // 10 n-1
                    Op::StackSet(2),   // 11 [n-1 acc']
                    Op::Goto(1),       // 12 backedge
                    Op::StackRef(0),   // 13 [n acc acc]
                    Op::Return,        // 14
                ],
                vec![Value::make_int(0)],
                1,
                vec![Value::make_int(10)],
            ),
        ];

        for (ops, constants, arity, args) in cases {
            let mir = mir::build_mir(&ops, &constants, arity).expect("MIR builds");
            let leaf = lower_mir_pure(&mir).expect("MIR lowers (pure subset)");

            // Interpreter oracle.
            let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
            let mut f = ByteCodeFunction::new(LambdaParams {
                required: (1..=arity)
                    .map(|i| crate::emacs_core::intern::SymId(i as u32))
                    .collect(),
                optional: Vec::new(),
                rest: None,
            });
            f.lexical = true;
            f.ops = ops.clone();
            f.constants = constants.clone();
            f.max_stack = 32;
            let want = {
                let mut vm = Vm::from_context(&mut ev);
                vm.execute(&f, args.clone()).expect("interpreter runs")
            };

            match leaf.call_for_test(&args) {
                Some(bits) => assert_eq!(
                    Value::from_bits(bits),
                    want,
                    "MIR-lowered native result must equal the interpreter for {ops:?}"
                ),
                None => panic!("MIR-lowered pure body deopted unexpectedly for {ops:?}"),
            }
        }
    }

    /// A pure-arithmetic guard deopts cleanly (non-fixnum input) — same as the
    /// baseline tier, since the pure subset reruns the interpreter from start.
    #[test]
    fn mir_pure_lowering_deopts_on_nonfixnum() {
        // (lambda (a b) (+ a b)) called with a string -> the fixnum guard fails.
        let ops = vec![Op::StackRef(1), Op::StackRef(1), Op::Add, Op::Return];
        let mir = mir::build_mir(&ops, &[], 2).expect("builds");
        let leaf = lower_mir_pure(&mir).expect("lowers");
        assert_eq!(
            leaf.call_for_test(&[Value::string("x"), Value::make_int(2)]),
            None,
            "non-fixnum operand deopts (rerun-from-start)"
        );
    }

    /// Shim-using ops (a call) are out of the pure-subset scope: bail.
    #[test]
    fn mir_pure_lowering_bails_on_calls() {
        // (lambda () (foo)) — has a Call (opaque) -> pure lowering refuses.
        let ops = vec![Op::Constant(0), Op::Call(0), Op::Return];
        let mir = mir::build_mir(&ops, &[Value::symbol("foo")], 0).expect("MIR builds");
        assert!(matches!(
            lower_mir_pure(&mir),
            Err(CompileError::UnsupportedOp("mir-pure-shim-op"))
        ));
    }

    #[test]
    fn bails_on_unsupported_op() {
        // MakeClosure (closure construction) is not in the supported subset ->
        // refuse, do not miscompile.
        let err = lower_nullary_leaf(
            &[Op::Nil, Op::Nil, Op::MakeClosure(0), Op::Nil, Op::Return],
            &[Value::NIL],
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::UnsupportedOp("other")));
        // A Switch whose jump table is not a compile-time constant bails too
        // (the byte compiler always emits Constant(table) right before it).
        let err = lower_nullary_leaf(&[Op::Nil, Op::Nil, Op::Switch, Op::Nil, Op::Return], &[])
            .unwrap_err();
        assert!(matches!(err, CompileError::UnsupportedOp("switch-dynamic")));
    }

    #[test]
    fn list_and_slice_builtins_run_natively() {
        use crate::emacs_core::print::print_value;
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;

        // (list 1 2 3)
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::Constant(2),
                Op::List(3),
                Op::Return,
            ],
            &[Value::make_int(1), Value::make_int(2), Value::make_int(3)],
        )
        .expect("list body compiles");
        let NativeRun::Ok(bits) = leaf.call(ctx_ptr, &[]) else {
            panic!("native list failed");
        };
        assert_eq!(print_value(&Value::from_bits(bits)), "(1 2 3)");

        // (concat "foo" "bar")
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Concat(2), Op::Return],
            &[Value::string("foo"), Value::string("bar")],
        )
        .expect("concat body compiles");
        let NativeRun::Ok(bits) = leaf.call(ctx_ptr, &[]) else {
            panic!("native concat failed");
        };
        assert_eq!(print_value(&Value::from_bits(bits)), "\"foobar\"");

        // (substring "hello" 1 3)
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::Constant(2),
                Op::Substring,
                Op::Return,
            ],
            &[
                Value::string("hello"),
                Value::make_int(1),
                Value::make_int(3),
            ],
        )
        .expect("substring body compiles");
        let NativeRun::Ok(bits) = leaf.call(ctx_ptr, &[]) else {
            panic!("native substring failed");
        };
        assert_eq!(print_value(&Value::from_bits(bits)), "\"el\"");

        // (nconc (list 1 2) (list 3)) — built natively end-to-end.
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::List(2),
                Op::Constant(2),
                Op::List(1),
                Op::Nconc,
                Op::Return,
            ],
            &[Value::make_int(1), Value::make_int(2), Value::make_int(3)],
        )
        .expect("nconc body compiles");
        let NativeRun::Ok(bits) = leaf.call(ctx_ptr, &[]) else {
            panic!("native nconc failed");
        };
        assert_eq!(print_value(&Value::from_bits(bits)), "(1 2 3)");

        // Signal path: (substring 5 0 1) is a wrong-type-argument.
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::Constant(2),
                Op::Substring,
                Op::Return,
            ],
            &[Value::make_int(5), Value::make_int(0), Value::make_int(1)],
        )
        .expect("substring body compiles");
        assert_eq!(leaf.call(ctx_ptr, &[]), NativeRun::Signal);
        let flow = take_pending_flow().expect("signal stashed");
        match flow {
            Flow::Signal(sig) => assert_eq!(sig.symbol_name(), "wrong-type-argument"),
            other => panic!("expected wrong-type-argument, got {other:?}"),
        }
    }

    #[test]
    fn named_builtin_ops_run_natively() {
        // CallBuiltin/CallBuiltinSym need the full runtime's subr resolution
        // (covered by the eval_test seam differential); Aset's fast path runs
        // against the minimal harness.
        use crate::emacs_core::print::print_value;
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;

        // Aset: mutate a constant vector natively, read back.
        let vec = Value::vector(vec![Value::make_int(0), Value::make_int(0)]);
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0), // v
                Op::Constant(1), // 1
                Op::Constant(2), // 99
                Op::Aset,
                Op::Return,
            ],
            &[vec, Value::make_int(1), Value::make_int(99)],
        )
        .expect("aset body compiles");
        assert_eq!(
            leaf.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(99).bits())
        );
        assert_eq!(print_value(&vec), "[0 99]");

        // Signal path: (aset 5 0 1) is a wrong-type-argument.
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::Constant(2),
                Op::Aset,
                Op::Return,
            ],
            &[Value::make_int(5), Value::make_int(0), Value::make_int(1)],
        )
        .expect("aset body compiles");
        assert_eq!(leaf.call(ctx_ptr, &[]), NativeRun::Signal);
        let _ = take_pending_flow().expect("signal stashed");
    }

    #[test]
    fn switch_jump_table_dispatches_natively() {
        // Mirror vm_switch_branches_using_hash_table_jump_table: a constant
        // eq jump table {foo -> byte offset 8} resolving through the GNU
        // byte-offset map to instruction 5. Hit -> 20, miss -> 10.
        use crate::emacs_core::value::HashTableTest;
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        let table = Value::hash_table(HashTableTest::Eq);
        let _ = table.with_hash_table_mut(|ht| {
            let key = Value::symbol("jit-sw-foo").to_hash_key(&ht.test);
            ht.data.insert(key.clone(), Value::fixnum(8));
            ht.key_snapshots
                .insert(key.clone(), Value::symbol("jit-sw-foo"));
            ht.insertion_order.push(key);
        });
        let map = vec![GnuByteOffsetMapEntry::new(8, 5)];
        let leaf = lower_leaf_with_map(
            &[
                Op::StackRef(0), // [x x]
                Op::Constant(0), // [x x table]
                Op::Switch,      // [x], jump or fall through
                Op::Constant(1), // miss: 10
                Op::Return,
                Op::Constant(2), // 5: hit: 20
                Op::Return,
            ],
            &[table, Value::make_int(10), Value::make_int(20)],
            1,
            Some(&map),
        )
        .expect("switch body compiles");
        let hit = leaf.call(ctx_ptr, &[Value::symbol("jit-sw-foo")]);
        assert_eq!(hit, NativeRun::Ok(Value::make_int(20).bits()));
        let miss = leaf.call(ctx_ptr, &[Value::symbol("jit-sw-bar")]);
        assert_eq!(miss, NativeRun::Ok(Value::make_int(10).bits()));
    }

    #[test]
    fn handler_analysis_bails_on_unbalanced_pophandler() {
        // PopHandler with no statically active handler frame.
        let err = lower_nullary_leaf(&[Op::PopHandler, Op::Nil, Op::Return], &[]).unwrap_err();
        assert!(matches!(
            err,
            CompileError::UnsupportedOp("unbalanced-pophandler")
        ));
    }

    #[test]
    fn handler_body_compiles_and_runs_catch_throw_natively() {
        // (catch 'tag (throw 'tag 42)) — the throw is caught by this same
        // frame's PushCatch via the match shim, natively.
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        let tag = Value::symbol("jit-unit-tag");
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),  // 'tag
                Op::PushCatch(5), // frame, handler target 5
                Op::Constant(0),  // 'tag
                Op::Constant(1),  // 42
                Op::Throw,
                Op::Return, // 5: handler entry [thrown]
            ],
            &[tag, Value::make_int(42)],
        )
        .expect("handler body compiles");
        let base = ev.condition_stack.len();
        match leaf.call(ctx_ptr, &[]) {
            NativeRun::Ok(bits) => {
                assert_eq!(Value::from_bits(bits), Value::make_int(42));
            }
            other => panic!("expected native catch, got {other:?}"),
        }
        assert_eq!(ev.condition_stack.len(), base, "frame popped by the catch");
    }

    #[test]
    fn handler_frames_unwound_on_propagation() {
        // (catch 'a (throw 'b 1)) — no frame matches: the flow propagates as
        // STATUS_SIGNAL (no-catch) and our registered frame is unwound.
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),  // 'a
                Op::PushCatch(5), // frame, handler target 5
                Op::Constant(1),  // 'b
                Op::Constant(2),  // 1
                Op::Throw,
                Op::Return, // 5: handler (reachable only via the frame)
            ],
            &[
                Value::symbol("jit-unit-a"),
                Value::symbol("jit-unit-b"),
                Value::make_int(1),
            ],
        )
        .expect("handler body compiles");
        let base = ev.condition_stack.len();
        assert_eq!(leaf.call(ctx_ptr, &[]), NativeRun::Signal);
        let flow = take_pending_flow().expect("no-catch flow stashed");
        match flow {
            Flow::Signal(sig) => assert_eq!(sig.symbol_name(), "no-catch"),
            other => panic!("expected no-catch signal, got {other:?}"),
        }
        assert_eq!(ev.condition_stack.len(), base, "frames unwound");
    }

    #[test]
    fn compiles_varref_and_varset() {
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        let var = Value::symbol("jit-test-dynvar");
        let crate::emacs_core::value::ValueKind::Symbol(var_id) = var.kind() else {
            panic!("symbol expected");
        };
        ev.obarray.set_symbol_value_id(var_id, Value::make_int(33));

        // VarRef reads the live value.
        let read = lower_nullary_leaf(&[Op::VarRef(0), Op::Return], &[var]).unwrap();
        assert_eq!(
            read.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(33).bits())
        );

        // VarSet stores; read back through the runtime.
        let write = lower_nullary_leaf(
            &[Op::Constant(1), Op::VarSet(0), Op::Nil, Op::Return],
            &[var, Value::make_int(44)],
        )
        .unwrap();
        assert_eq!(write.call(ctx_ptr, &[]), NativeRun::Ok(Value::NIL.bits()));
        assert_eq!(
            read.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(44).bits()),
            "VarSet must be visible to a subsequent VarRef"
        );

        // Reading an unbound variable signals (void-variable) -> Signal.
        let unbound = Value::symbol("jit-test-unbound-var");
        let bad = lower_nullary_leaf(&[Op::VarRef(0), Op::Return], &[unbound]).unwrap();
        assert_eq!(bad.call(ctx_ptr, &[]), NativeRun::Signal);
        assert!(take_pending_flow().is_some());
    }

    #[test]
    fn compiles_varbind_unbind_with_full_unwind_semantics() {
        use crate::emacs_core::bytecode::Vm;
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        let var = Value::symbol("jit-test-bind-var");
        let crate::emacs_core::value::ValueKind::Symbol(var_id) = var.kind() else {
            panic!("symbol expected");
        };
        ev.obarray.set_symbol_value_id(var_id, Value::make_int(99));
        let read = lower_nullary_leaf(&[Op::VarRef(0), Op::Return], &[var]).unwrap();
        let global_now = |ev: &mut crate::emacs_core::eval::Context| {
            let p = ev as *mut crate::emacs_core::eval::Context as *mut u8;
            match read.call(p, &[]) {
                NativeRun::Ok(bits) => Value::from_bits(bits),
                other => panic!("global read failed: {other:?}"),
            }
        };

        // Balanced let: bind 5, read it, unbind, return. Matches the
        // interpreter on the same body.
        let ops = [
            Op::Constant(1), // 5
            Op::VarBind(0),
            Op::VarRef(0),
            Op::Unbind(1),
            Op::Return,
        ];
        let consts = [var, Value::make_int(5)];
        let balanced = lower_nullary_leaf(&ops, &consts).unwrap();
        assert_eq!(
            balanced.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(5).bits())
        );
        assert_eq!(global_now(&mut ev), Value::make_int(99), "binding popped");
        let interp = {
            let mut f = ByteCodeFunction::new(LambdaParams {
                required: Vec::new(),
                optional: Vec::new(),
                rest: None,
            });
            f.lexical = true;
            f.ops = ops.to_vec();
            f.constants = consts.to_vec();
            f.max_stack = 16;
            let mut vm = Vm::from_context(&mut ev);
            vm.execute(&f, vec![]).expect("interp runs let")
        };
        assert_eq!(interp, Value::make_int(5), "interpreter agrees");
        assert_eq!(global_now(&mut ev), Value::make_int(99));

        // Early return with the binding still active: the frame unwind must
        // restore the global (cleanup_bytecode_frame parity).
        let early = lower_nullary_leaf(
            &[Op::Constant(1), Op::VarBind(0), Op::True, Op::Return],
            &consts,
        )
        .unwrap();
        assert_eq!(early.call(ctx_ptr, &[]), NativeRun::Ok(Value::T.bits()));
        assert_eq!(
            global_now(&mut ev),
            Value::make_int(99),
            "early return must unwind the dangling binding"
        );

        // Signal inside the dynamic extent: the binding must also unwind.
        let sig = lower_nullary_leaf(
            &[
                Op::Constant(1),
                Op::VarBind(0),
                Op::Constant(2), // undefined function symbol
                Op::Call(0),
                Op::Return,
            ],
            &[
                var,
                Value::make_int(5),
                Value::symbol("jit-bind-no-such-fn"),
            ],
        )
        .unwrap();
        assert_eq!(sig.call(ctx_ptr, &[]), NativeRun::Signal);
        assert!(take_pending_flow().is_some());
        assert_eq!(
            global_now(&mut ev),
            Value::make_int(99),
            "signal must unwind the dangling binding"
        );
    }

    #[test]
    fn guard_after_varbind_and_unbalanced_unbind_bail() {
        // Precise deopt: a guard after a binding compiles (a failing guard
        // transfers the bind to the resumed interpreter frame).
        lower_nullary_leaf(
            &[
                Op::Constant(1),
                Op::VarBind(0),
                Op::Constant(1),
                Op::Add1,
                Op::Return,
            ],
            &[Value::symbol("jit-test-bind-poison"), Value::make_int(1)],
        )
        .expect("guard after a binding compiles under precise deopt");

        // Unbinding more than this function bound bails to the interpreter.
        let err = lower_nullary_leaf(&[Op::Unbind(1), Op::Nil, Op::Return], &[]).unwrap_err();
        assert!(matches!(
            err,
            CompileError::UnsupportedOp("unbalanced-unbind")
        ));
    }

    #[test]
    fn guard_after_varset_compiles_and_runs() {
        // Precise deopt: a guard after an assignment compiles and runs; the
        // assignment is NOT replayed on a later deopt (resume is mid-frame).
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(1),
                Op::VarSet(0),
                Op::Constant(1),
                Op::Add1,
                Op::Return,
            ],
            &[Value::symbol("jit-test-poison-var"), Value::make_int(1)],
        )
        .expect("guard after an assignment compiles under precise deopt");
        assert_eq!(
            leaf.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(2).bits())
        );
    }

    #[test]
    fn compiles_fixnum_mul() {
        let mul = |a: i64, b: i64| {
            lower_nullary_leaf(
                &[Op::Constant(0), Op::Constant(1), Op::Mul, Op::Return],
                &[Value::make_int(a), Value::make_int(b)],
            )
            .unwrap()
            .call_for_test(&[])
        };
        assert_eq!(mul(6, 7), Some(Value::make_int(42).bits()));
        assert_eq!(mul(-6, 7), Some(Value::make_int(-42).bits()));
        assert_eq!(mul(0, 12345), Some(Value::make_int(0).bits()));
        // Product overflowing fixnum range -> deopt.
        assert_eq!(mul(Value::MOST_POSITIVE_FIXNUM, 2), None);
        assert_eq!(mul(1 << 40, 1 << 40), None); // 2^80, way out of range
    }

    #[test]
    fn mul_non_fixnum_deopts() {
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Nil, Op::Mul, Op::Return],
            &[Value::make_int(5)],
        )
        .unwrap();
        assert_eq!(leaf.call_for_test(&[]), None);
    }

    #[test]
    fn compiles_type_predicates() {
        // Inspects only tag bits; never dereferences, so heap values needn't be
        // kept alive (no GC safepoint in the JIT call).
        fn pred(op: Op, v: Value) -> Option<usize> {
            lower_nullary_leaf(&[Op::Constant(0), op, Op::Return], &[v])
                .unwrap()
                .call_for_test(&[])
        }
        let t = Some(Value::T.bits());
        let nil = Some(Value::NIL.bits());
        let cons = Value::cons(Value::make_int(1), Value::make_int(2));
        let s = Value::string("hi");

        // null / not: only nil is null; fixnum 0 is NOT nil.
        assert_eq!(pred(Op::Null, Value::NIL), t);
        assert_eq!(pred(Op::Null, Value::make_int(0)), nil);
        assert_eq!(pred(Op::Not, Value::T), nil);
        assert_eq!(pred(Op::Not, Value::NIL), t);
        // consp
        assert_eq!(pred(Op::Consp, cons), t);
        assert_eq!(pred(Op::Consp, Value::NIL), nil);
        assert_eq!(pred(Op::Consp, Value::make_int(5)), nil);
        // stringp
        assert_eq!(pred(Op::Stringp, s), t);
        assert_eq!(pred(Op::Stringp, Value::make_int(5)), nil);
        // listp: nil or cons
        assert_eq!(pred(Op::Listp, cons), t);
        assert_eq!(pred(Op::Listp, Value::NIL), t);
        assert_eq!(pred(Op::Listp, Value::make_int(5)), nil);
    }

    #[test]
    fn compiles_car_cdr() {
        // No GC safepoint in the JIT call, so the cons local stays alive across it.
        let cons = Value::cons(Value::make_int(11), Value::make_int(22));
        let car_ops = [Op::Constant(0), Op::Car, Op::Return];
        let cdr_ops = [Op::Constant(0), Op::Cdr, Op::Return];

        // car/cdr of a cons load the fields; differential vs the interpreter.
        // Direct value assertions, not an interp differential: interp_nullary
        // builds a Context whose heap is installed as the thread-local TAGGED_HEAP
        // and left dangling on drop, which would crash the later cons allocation.
        // car/cdr correctness is fully pinned by the expected values here.
        assert_eq!(
            lower_nullary_leaf(&car_ops, &[cons])
                .unwrap()
                .call_for_test(&[]),
            Some(Value::make_int(11).bits())
        );
        assert_eq!(
            lower_nullary_leaf(&cdr_ops, &[cons])
                .unwrap()
                .call_for_test(&[]),
            Some(Value::make_int(22).bits())
        );

        // car/cdr of nil -> nil.
        assert_eq!(
            lower_nullary_leaf(&car_ops, &[Value::NIL])
                .unwrap()
                .call_for_test(&[]),
            Some(Value::NIL.bits())
        );
        assert_eq!(
            lower_nullary_leaf(&cdr_ops, &[Value::NIL])
                .unwrap()
                .call_for_test(&[]),
            Some(Value::NIL.bits())
        );

        // car of a non-list -> deopt (interpreter signals wrong-type-argument).
        assert_eq!(
            lower_nullary_leaf(&car_ops, &[Value::make_int(5)])
                .unwrap()
                .call_for_test(&[]),
            None
        );

        // Chained: (car (cdr (11 22))) = 22.
        let list = Value::cons(
            Value::make_int(11),
            Value::cons(Value::make_int(22), Value::NIL),
        );
        let cadr =
            lower_nullary_leaf(&[Op::Constant(0), Op::Cdr, Op::Car, Op::Return], &[list]).unwrap();
        assert_eq!(cadr.call_for_test(&[]), Some(Value::make_int(22).bits()));
    }

    #[test]
    fn compiles_cons() {
        // (cons 1 2): allocates a cons cell. No GC between the call and the deref
        // (nothing allocates), so the fresh cons stays valid.
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Cons, Op::Return],
            &[Value::make_int(1), Value::make_int(2)],
        )
        .unwrap();
        let cell = Value::from_bits(leaf.call_for_test(&[]).expect("cons runs"));
        assert!(cell.is_cons());
        assert_eq!(cell.cons_car(), Value::make_int(1));
        assert_eq!(cell.cons_cdr(), Value::make_int(2));
    }

    #[test]
    fn compiles_nested_cons_list() {
        // (cons 7 (cons 8 nil)) = (7 8). The inner cons leaves 7 live below it on
        // the operand stack, exercising the gc_push rooting path.
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0),
                Op::Constant(1),
                Op::Nil,
                Op::Cons,
                Op::Cons,
                Op::Return,
            ],
            &[Value::make_int(7), Value::make_int(8)],
        )
        .unwrap();
        let result = Value::from_bits(leaf.call_for_test(&[]).expect("nested cons runs"));
        assert_eq!(result.cons_car(), Value::make_int(7));
        let tail = result.cons_cdr();
        assert!(tail.is_cons());
        assert_eq!(tail.cons_car(), Value::make_int(8));
        assert!(tail.cons_cdr().is_nil());
    }

    /// Build a harness Context with `name` bound to a lexical one-arg bytecode
    /// callee `(lambda (y) (1+ y))`, returning (ctx, callee symbol Value).
    fn harness_with_inc_callee(name: &str) -> (crate::emacs_core::eval::Context, Value) {
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let sym_val = Value::symbol(name);
        let crate::emacs_core::value::ValueKind::Symbol(sym_id) = sym_val.kind() else {
            panic!("Value::symbol must produce a symbol");
        };
        let mut callee = ByteCodeFunction::new(LambdaParams {
            required: vec![crate::emacs_core::intern::SymId(1)],
            optional: Vec::new(),
            rest: None,
        });
        callee.lexical = true;
        callee.ops = vec![Op::StackRef(0), Op::Add1, Op::Return];
        callee.max_stack = 16;
        ev.obarray
            .set_symbol_function_id(sym_id, Value::make_bytecode(callee));
        (ev, sym_val)
    }

    #[test]
    fn compiles_call_to_bytecode_callee() {
        // (lambda () (callee 41)) where callee = (lambda (y) (1+ y)).
        // The native code re-enters the runtime through the call shim; the
        // callee runs on the interpreter and the result flows back.
        let (mut ev, sym_val) = harness_with_inc_callee("jit-test-inc-callee");
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Call(1), Op::Return],
            &[sym_val, Value::make_int(41)],
        )
        .unwrap();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        assert_eq!(
            leaf.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(42).bits())
        );
    }

    #[test]
    fn call_with_live_values_below_roots_and_returns() {
        // (lambda () (let ((keep 7)) (+0-guard-free use of keep after a call)).
        // Body: push keep=7, push sym, push 41, Call(1) -> keep stays live below
        // the call (exercises the gc_save/gc_push rooting path), then combine:
        // [keep, result] -> StackSet(1) folds result into keep slot -> Return.
        let (mut ev, sym_val) = harness_with_inc_callee("jit-test-inc-callee-2");
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(2), // keep = 7
                Op::Constant(0), // sym
                Op::Constant(1), // 41
                Op::Call(1),     // -> [keep, 42]
                Op::StackSet(1), // -> [42]
                Op::Return,
            ],
            &[sym_val, Value::make_int(41), Value::make_int(7)],
        )
        .unwrap();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        assert_eq!(
            leaf.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(42).bits())
        );
    }

    #[test]
    fn call_signal_propagates() {
        // Calling an unbound function must surface as NativeRun::Signal with the
        // Flow stashed for the caller — not a deopt, not a crash.
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let sym_val = Value::symbol("jit-test-no-such-function");
        let leaf =
            lower_nullary_leaf(&[Op::Constant(0), Op::Call(0), Op::Return], &[sym_val]).unwrap();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        assert_eq!(leaf.call(ctx_ptr, &[]), NativeRun::Signal);
        assert!(
            take_pending_flow().is_some(),
            "STATUS_SIGNAL must stash the Flow"
        );
    }

    #[test]
    fn guard_after_call_deopts_without_replaying_the_call() {
        // THE precise-deopt capability test: a guard after a side-effecting
        // call compiles; when it fails, the interpreter resumes AT the guard
        // op — the call's side effect happened exactly once (rerun-from-start
        // would have replayed it). Full Context: the resumed 1+ promotes to a
        // bignum through the real builtin dispatch.
        let mut ev = crate::emacs_core::eval::Context::new();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        // Callee (lambda (x) (setcar CELL (1+ (car CELL))) x): observable
        // side effect (counter cons), returns its argument unchanged.
        let cell = Value::cons(Value::make_int(0), Value::NIL);
        let sym_val = Value::symbol("jit-test-effect-callee");
        let crate::emacs_core::value::ValueKind::Symbol(sym_id) = sym_val.kind() else {
            panic!("symbol expected");
        };
        let mut callee = ByteCodeFunction::new(LambdaParams {
            required: vec![crate::emacs_core::intern::SymId(1)],
            optional: Vec::new(),
            rest: None,
        });
        callee.lexical = true;
        callee.ops = vec![
            Op::Constant(0), // CELL
            Op::Constant(0), // CELL
            Op::Car,
            Op::Add1,
            Op::Setcar,
            Op::Pop,
            Op::StackRef(0),
            Op::Return,
        ];
        callee.constants = vec![cell];
        callee.max_stack = 16;
        ev.obarray
            .set_symbol_function_id(sym_id, Value::make_bytecode(callee));

        // Caller: (1+ (callee MOST-POSITIVE-FIXNUM)) — the 1+ guard fails
        // AFTER the call ran.
        let ops = vec![
            Op::Constant(0), // 'callee
            Op::Constant(1), // MOST_POSITIVE
            Op::Call(1),
            Op::Add1, // pc 3: deopts (overflow)
            Op::Return,
        ];
        let constants = vec![sym_val, Value::make_int(Value::MOST_POSITIVE_FIXNUM)];
        let mut f = ByteCodeFunction::new(LambdaParams {
            required: Vec::new(),
            optional: Vec::new(),
            rest: None,
        });
        f.lexical = true;
        f.ops = ops.clone();
        f.constants = constants.clone();
        f.max_stack = 16;
        let leaf = lower_nullary_leaf(&ops, &constants).expect("guard after call compiles now");
        let native = match leaf.call(ctx_ptr, &[]) {
            NativeRun::DeoptAt {
                pc,
                stack,
                handlers,
                binds,
                spec_base,
                cond_base,
            } => {
                assert_eq!(pc, 3, "deopt at the 1+ after the call");
                assert_eq!(
                    cell.cons_car(),
                    Value::make_int(1),
                    "the call's side effect ran exactly once before the deopt"
                );
                let mut vm = Vm::from_context(&mut ev);
                vm.run_resumed_frame(
                    &f,
                    Value::NIL,
                    pc,
                    &stack,
                    handlers,
                    &binds,
                    spec_base,
                    cond_base,
                )
                .expect("resume computes the bignum")
            }
            other => panic!("expected a precise deopt after the call, got {other:?}"),
        };
        assert_eq!(
            cell.cons_car(),
            Value::make_int(1),
            "resume must NOT replay the call"
        );
        // Differential: the pure interpreter on the same body (fresh counter
        // state) computes the same bignum and also increments exactly once.
        b::builtin_setcar_2(&mut ev, cell, Value::make_int(0)).expect("reset counter");
        let interp = {
            let mut vm = Vm::from_context(&mut ev);
            vm.execute(&f, vec![]).expect("interpreter computes")
        };
        assert_eq!(
            crate::emacs_core::print::print_value(&native),
            crate::emacs_core::print::print_value(&interp),
            "resume result must equal the interpreter's"
        );
        assert_eq!(cell.cons_car(), Value::make_int(1));
    }

    #[test]
    fn guard_before_call_compiles_and_deopts_cleanly() {
        // Guards strictly before the first call are fine: a deopt there reruns
        // the interpreter with no side effect having happened.
        let (mut ev, sym_val) = harness_with_inc_callee("jit-test-inc-callee-3");
        let ops = [
            Op::Constant(0), // sym
            Op::Constant(1), // n
            Op::Add1,        // guard BEFORE the call
            Op::Call(1),
            Op::Return,
        ];
        // In-range: runs natively end-to-end: (1+ 40) = 41 -> callee -> 42.
        let leaf = lower_nullary_leaf(&ops, &[sym_val, Value::make_int(40)]).unwrap();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        assert_eq!(
            leaf.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(42).bits())
        );
        // Boundary input: the pre-call guard now deopts PRECISELY at the 1+
        // op (pc 2) with the pre-op stack captured — the resume would rerun
        // exactly that op on the interpreter.
        let leaf2 = lower_nullary_leaf(
            &ops,
            &[sym_val, Value::make_int(Value::MOST_POSITIVE_FIXNUM)],
        )
        .unwrap();
        match leaf2.call(ctx_ptr, &[]) {
            NativeRun::DeoptAt {
                pc,
                stack,
                handlers,
                binds,
                ..
            } => {
                assert_eq!(pc, 2, "deopt at the Add1 op");
                assert_eq!(stack.len(), 2, "pre-op stack: [callee-sym, arg]");
                assert_eq!(stack[1], Value::make_int(Value::MOST_POSITIVE_FIXNUM));
                assert_eq!(handlers, 0);
                assert!(binds.is_empty());
            }
            other => panic!("expected a precise deopt, got {other:?}"),
        }
    }

    #[test]
    fn compiles_fixnum_div_rem() {
        let run = |op: Op, a: i64, b: i64| {
            lower_nullary_leaf(
                &[Op::Constant(0), Op::Constant(1), op, Op::Return],
                &[Value::make_int(a), Value::make_int(b)],
            )
            .unwrap()
            .call_for_test(&[])
        };
        // Truncation toward zero, matching the interpreter / C.
        assert_eq!(run(Op::Div, 42, 5), Some(Value::make_int(8).bits()));
        assert_eq!(run(Op::Div, -42, 5), Some(Value::make_int(-8).bits()));
        assert_eq!(run(Op::Div, 42, -5), Some(Value::make_int(-8).bits()));
        assert_eq!(run(Op::Rem, 42, 5), Some(Value::make_int(2).bits()));
        assert_eq!(run(Op::Rem, -42, 5), Some(Value::make_int(-2).bits()));
        // Zero divisor -> deopt (interpreter signals arith-error).
        assert_eq!(run(Op::Div, 1, 0), None);
        assert_eq!(run(Op::Rem, 1, 0), None);
        // Non-fixnum operand -> deopt.
        let nf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Nil, Op::Div, Op::Return],
            &[Value::make_int(4)],
        )
        .unwrap();
        assert_eq!(nf.call_for_test(&[]), None);
    }

    #[test]
    fn div_wrap_case_matches_interpreter() {
        // MOST_NEGATIVE_FIXNUM / -1 wraps through the interpreter's retag; the
        // JIT's sdiv + retag must produce the identical bits.
        let ops = [Op::Constant(0), Op::Constant(1), Op::Div, Op::Return];
        let consts = [
            Value::make_int(Value::MOST_NEGATIVE_FIXNUM),
            Value::make_int(-1),
        ];
        let want = interp_nullary(&ops, &consts).bits();
        let got = lower_nullary_leaf(&ops, &consts)
            .unwrap()
            .call_for_test(&[]);
        assert_eq!(got, Some(want));
    }

    #[test]
    fn compiles_eq_and_symbolp() {
        // One live Context for the vmctx-reading slow paths (symbols-with-pos
        // is disabled by default, so differing bits -> nil).
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        let sym_a = Value::symbol("jit-eq-sym-a");
        let s = Value::string("eq-str");

        let eq2 = |a: Value, b: Value, ctx: *mut u8| {
            lower_nullary_leaf(
                &[Op::Constant(0), Op::Constant(1), Op::Eq, Op::Return],
                &[a, b],
            )
            .unwrap()
            .call(ctx, &[])
        };
        let t = NativeRun::Ok(Value::T.bits());
        let nil = NativeRun::Ok(Value::NIL.bits());
        // Identical bits -> t (fast path, no shim).
        assert_eq!(eq2(Value::make_int(7), Value::make_int(7), ctx_ptr), t);
        assert_eq!(eq2(sym_a, sym_a, ctx_ptr), t);
        assert_eq!(eq2(Value::NIL, Value::NIL, ctx_ptr), t);
        // Differing bits -> slow shim -> nil (swp disabled).
        assert_eq!(eq2(Value::make_int(7), Value::make_int(8), ctx_ptr), nil);
        assert_eq!(eq2(sym_a, Value::make_int(7), ctx_ptr), nil);

        let symp = |v: Value, ctx: *mut u8| {
            lower_nullary_leaf(&[Op::Constant(0), Op::Symbolp, Op::Return], &[v])
                .unwrap()
                .call(ctx, &[])
        };
        // Symbol tag -> t natively (nil and t are symbols).
        assert_eq!(symp(sym_a, ctx_ptr), t);
        assert_eq!(symp(Value::NIL, ctx_ptr), t);
        assert_eq!(symp(Value::T, ctx_ptr), t);
        // Non-symbol -> slow shim -> nil (swp disabled).
        assert_eq!(symp(Value::make_int(5), ctx_ptr), nil);
        assert_eq!(symp(s, ctx_ptr), nil);
    }

    #[test]
    fn compiles_apply_with_spread() {
        // (apply 'inc (list 41)) -> 42: the last argument spreads as the list.
        let (mut ev, sym_val) = harness_with_inc_callee("jit-test-inc-apply");
        let arg_list = Value::cons(Value::make_int(41), Value::NIL);
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Apply(1), Op::Return],
            &[sym_val, arg_list],
        )
        .unwrap();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        assert_eq!(
            leaf.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(42).bits())
        );
    }

    #[test]
    fn compiles_apply_with_leading_args() {
        // (apply 'add2 40 (list 2)) -> 42: leading args + spread tail.
        let mut ev = crate::emacs_core::eval::Context::new_minimal_vm_harness();
        let sym_val = Value::symbol("jit-test-add2-apply");
        let crate::emacs_core::value::ValueKind::Symbol(sym_id) = sym_val.kind() else {
            panic!("symbol expected");
        };
        let mut callee = ByteCodeFunction::new(LambdaParams {
            required: vec![
                crate::emacs_core::intern::SymId(1),
                crate::emacs_core::intern::SymId(2),
            ],
            optional: Vec::new(),
            rest: None,
        });
        callee.lexical = true;
        callee.ops = vec![Op::StackRef(1), Op::StackRef(1), Op::Add, Op::Return];
        callee.max_stack = 16;
        ev.obarray
            .set_symbol_function_id(sym_id, Value::make_bytecode(callee));

        let tail = Value::cons(Value::make_int(2), Value::NIL);
        let leaf = lower_nullary_leaf(
            &[
                Op::Constant(0), // sym
                Op::Constant(1), // 40
                Op::Constant(2), // (2)
                Op::Apply(2),
                Op::Return,
            ],
            &[sym_val, Value::make_int(40), tail],
        )
        .unwrap();
        let ctx_ptr = &mut ev as *mut crate::emacs_core::eval::Context as *mut u8;
        assert_eq!(
            leaf.call(ctx_ptr, &[]),
            NativeRun::Ok(Value::make_int(42).bits())
        );
    }

    #[test]
    fn bails_on_missing_return() {
        let err = lower_nullary_leaf(&[Op::Nil], &[]).unwrap_err();
        assert!(matches!(err, CompileError::NoReturn));
    }

    #[test]
    fn bails_on_argument_taking_function() {
        let mut f = nullary();
        f.params.required.push(crate::emacs_core::intern::SymId(1));
        f.ops = vec![Op::Nil, Op::Return];
        let err = compile_bytecode_function(&f).unwrap_err();
        assert!(matches!(err, CompileError::TakesArguments));
    }

    #[test]
    fn bails_on_stack_underflow() {
        let err = lower_nullary_leaf(&[Op::Return], &[]).unwrap_err();
        assert!(matches!(err, CompileError::StackUnderflow));
    }

    #[test]
    fn compile_bytecode_function_handles_nullary_leaf() {
        let mut f = nullary();
        let c = Value::make_int(123);
        f.constants = vec![c];
        f.ops = vec![Op::Constant(0), Op::Return];
        let leaf = compile_bytecode_function(&f).unwrap();
        assert_eq!(leaf.call_for_test(&[]), Some(c.bits()));
    }

    #[test]
    fn one_arg_identity_and_increment() {
        // (lambda (x) x)
        let id = lower_leaf(&[Op::StackRef(0), Op::Return], &[], 1).unwrap();
        assert_eq!(id.arity(), 1);
        assert_eq!(
            id.call_for_test(&[Value::make_int(7)]),
            Some(Value::make_int(7).bits())
        );
        // (lambda (x) (1+ x))
        let inc = lower_leaf(&[Op::StackRef(0), Op::Add1, Op::Return], &[], 1).unwrap();
        assert_eq!(
            inc.call_for_test(&[Value::make_int(41)]),
            Some(Value::make_int(42).bits())
        );
    }

    #[test]
    fn two_arg_addition_preserves_args_via_stackref() {
        // (lambda (a b) (+ a b)); each StackRef(1) reaches an original arg as the
        // model stack grows: seed [a,b] -> push a -> push b -> Add -> a+b.
        let add = lower_leaf(
            &[Op::StackRef(1), Op::StackRef(1), Op::Add, Op::Return],
            &[],
            2,
        )
        .unwrap();
        assert_eq!(
            add.call_for_test(&[Value::make_int(40), Value::make_int(2)]),
            Some(Value::make_int(42).bits())
        );
        // A non-fixnum argument makes the speculative Add deopt.
        assert_eq!(add.call_for_test(&[Value::make_int(40), Value::NIL]), None);
    }

    #[test]
    fn compile_bytecode_function_accepts_required_args_when_lexical() {
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
        let leaf = compile_bytecode_function(&f).unwrap();
        assert_eq!(leaf.arity(), 2);
        assert_eq!(
            leaf.call_for_test(&[Value::make_int(1), Value::make_int(41)]),
            Some(Value::make_int(42).bits())
        );
    }

    #[test]
    fn compile_bytecode_function_bails_on_dynamic_params() {
        // Required params but dynamic binding (not lexical, arglist not a
        // fixnum) -> params are not on the stack -> bail.
        let mut dynp = ByteCodeFunction::new(LambdaParams {
            required: vec![crate::emacs_core::intern::SymId(1)],
            optional: Vec::new(),
            rest: None,
        });
        dynp.lexical = false;
        dynp.ops = vec![Op::StackRef(0), Op::Return];
        assert!(!params_on_stack(&dynp));
        assert!(matches!(
            compile_bytecode_function(&dynp),
            Err(CompileError::TakesArguments)
        ));
    }

    #[test]
    fn compiles_optional_params_with_nil_padding() {
        // (lambda (a &optional b) b): frame = [a, b]; missing b is nil-padded.
        let mut f = ByteCodeFunction::new(LambdaParams {
            required: vec![crate::emacs_core::intern::SymId(1)],
            optional: vec![crate::emacs_core::intern::SymId(2)],
            rest: None,
        });
        f.lexical = true;
        f.ops = vec![Op::StackRef(0), Op::Return]; // top of frame = b
        f.max_stack = 16;
        let leaf = compile_bytecode_function(&f).unwrap();
        assert!(leaf.accepts(1) && leaf.accepts(2));
        assert!(!leaf.accepts(0) && !leaf.accepts(3));
        // One arg: b is nil.
        assert_eq!(
            leaf.call(core::ptr::null_mut(), &[Value::make_int(5)]),
            NativeRun::Ok(Value::NIL.bits())
        );
        // Two args: b is supplied.
        assert_eq!(
            leaf.call(
                core::ptr::null_mut(),
                &[Value::make_int(5), Value::make_int(6)]
            ),
            NativeRun::Ok(Value::make_int(6).bits())
        );
    }

    #[test]
    fn compiles_rest_param_as_list() {
        // (lambda (&rest xs) xs): frame = [xs]; surplus args become a list.
        let mut f = ByteCodeFunction::new(LambdaParams {
            required: Vec::new(),
            optional: Vec::new(),
            rest: Some(crate::emacs_core::intern::SymId(1)),
        });
        f.lexical = true;
        f.ops = vec![Op::StackRef(0), Op::Return];
        f.max_stack = 16;
        let leaf = compile_bytecode_function(&f).unwrap();
        assert!(leaf.accepts(0) && leaf.accepts(5));
        // No args: xs = nil.
        assert_eq!(
            leaf.call(core::ptr::null_mut(), &[]),
            NativeRun::Ok(Value::NIL.bits())
        );
        // Two args: xs = (10 20).
        let NativeRun::Ok(bits) = leaf.call(
            core::ptr::null_mut(),
            &[Value::make_int(10), Value::make_int(20)],
        ) else {
            panic!("rest call must succeed");
        };
        let xs = Value::from_bits(bits);
        assert_eq!(xs.cons_car(), Value::make_int(10));
        assert_eq!(xs.cons_cdr().cons_car(), Value::make_int(20));
        assert!(xs.cons_cdr().cons_cdr().is_nil());
    }

    /// Run a nullary body through the Tier-0 interpreter (the correctness
    /// oracle) and return its result.
    fn interp_nullary(ops: &[Op], constants: &[Value]) -> Value {
        use crate::emacs_core::bytecode::Vm;
        use crate::emacs_core::eval::Context;
        let mut eval = Context::new_minimal_vm_harness();
        let mut f = nullary();
        f.ops = ops.to_vec();
        f.constants = constants.to_vec();
        f.max_stack = 16;
        let mut vm = Vm::from_context(&mut eval);
        vm.execute(&f, vec![]).expect("interpreter runs the body")
    }

    #[test]
    fn jit_matches_interpreter_on_supported_bodies() {
        // The ultimate parity proof: when the JIT compiles a body and does not
        // deopt, its result must be bit-identical to the interpreter's.
        let cases: &[(&[Op], &[Value])] = &[
            (&[Op::Constant(0), Op::Return], &[Value::make_int(42)]),
            (&[Op::Nil, Op::Return], &[]),
            (&[Op::True, Op::Return], &[]),
            (
                &[Op::Constant(0), Op::Constant(1), Op::Add, Op::Return],
                &[Value::make_int(40), Value::make_int(2)],
            ),
            (
                &[Op::Constant(0), Op::Constant(1), Op::Sub, Op::Return],
                &[Value::make_int(3), Value::make_int(10)],
            ),
            (
                &[Op::Constant(0), Op::Constant(1), Op::Mul, Op::Return],
                &[Value::make_int(-6), Value::make_int(7)],
            ),
            (&[Op::Nil, Op::Null, Op::Return], &[]),
            (
                &[Op::Constant(0), Op::Null, Op::Return],
                &[Value::make_int(0)],
            ),
            (
                &[Op::Constant(0), Op::Consp, Op::Return],
                &[Value::make_int(5)],
            ),
            (
                &[Op::Constant(0), Op::Listp, Op::Return],
                &[Value::make_int(5)],
            ),
            (
                &[Op::Constant(0), Op::Add1, Op::Return],
                &[Value::make_int(41)],
            ),
            (
                &[Op::Constant(0), Op::Sub1, Op::Return],
                &[Value::make_int(43)],
            ),
            (
                &[Op::Constant(0), Op::Negate, Op::Return],
                &[Value::make_int(42)],
            ),
            (
                &[
                    Op::Constant(0),
                    Op::Constant(1),
                    Op::Add,
                    Op::Constant(2),
                    Op::Sub,
                    Op::Return,
                ],
                &[Value::make_int(1), Value::make_int(2), Value::make_int(4)],
            ),
            (
                &[Op::Constant(0), Op::Constant(1), Op::Lss, Op::Return],
                &[Value::make_int(3), Value::make_int(5)],
            ),
            (
                &[Op::Constant(0), Op::Constant(1), Op::Gtr, Op::Return],
                &[Value::make_int(3), Value::make_int(5)],
            ),
            (
                &[Op::Constant(0), Op::Constant(1), Op::Eqlsign, Op::Return],
                &[Value::make_int(5), Value::make_int(5)],
            ),
            (
                &[
                    Op::Constant(0),
                    Op::Constant(1),
                    Op::Constant(2),
                    Op::DiscardN(2),
                    Op::Return,
                ],
                &[
                    Value::make_int(10),
                    Value::make_int(20),
                    Value::make_int(30),
                ],
            ),
            (
                &[
                    Op::Constant(0),
                    Op::Constant(1),
                    Op::Constant(2),
                    Op::DiscardN(0x82),
                    Op::Return,
                ],
                &[
                    Value::make_int(10),
                    Value::make_int(20),
                    Value::make_int(30),
                ],
            ),
        ];
        for (i, (ops, consts)) in cases.iter().enumerate() {
            let want = interp_nullary(ops, consts).bits();
            let got = lower_nullary_leaf(ops, consts).unwrap().call_for_test(&[]);
            assert_eq!(got, Some(want), "JIT/interpreter mismatch on case {i}");
        }
    }

    #[test]
    fn jit_matches_interpreter_with_args() {
        use crate::emacs_core::bytecode::Vm;
        use crate::emacs_core::eval::Context;
        // (lambda (a b) (+ a b)), lexical.
        let ops = [Op::StackRef(1), Op::StackRef(1), Op::Add, Op::Return];
        let args = [Value::make_int(40), Value::make_int(2)];

        let mut eval = Context::new_minimal_vm_harness();
        let mut f = ByteCodeFunction::new(LambdaParams {
            required: vec![
                crate::emacs_core::intern::SymId(1),
                crate::emacs_core::intern::SymId(2),
            ],
            optional: Vec::new(),
            rest: None,
        });
        f.lexical = true;
        f.ops = ops.to_vec();
        f.max_stack = 16;
        let want = {
            let mut vm = Vm::from_context(&mut eval);
            vm.execute(&f, args.to_vec())
                .expect("interpreter runs")
                .bits()
        };

        let got = lower_leaf(&ops, &[], 2).unwrap().call_for_test(&args);
        assert_eq!(got, Some(want), "JIT must match the interpreter with args");
    }

    // Note: the JIT's deopt *boundary* (out-of-range -> None) is covered by
    // `add_overflowing_fixnum_range_deopts` and `unary_boundary_inputs_deopt`.
    // A differential check against the interpreter's bignum-promotion path is
    // intentionally omitted here because `new_minimal_vm_harness` does not wire
    // the full `+`/bignum builtins (it signals on that fallback), so it cannot
    // serve as the oracle for the slow path.

    /// Phase-8 micro-benchmark: the hot fixnum countdown loop, Tier 0 vs JIT.
    /// `#[ignore]`d (timing does not belong in CI); run explicitly, in release:
    /// `cargo nextest run --cargo-profile release --features jit --run-ignored all jit_bench`
    #[test]
    #[ignore = "manual perf measurement; run in release"]
    fn jit_bench_countdown_loop() {
        use crate::emacs_core::bytecode::Vm;
        use crate::emacs_core::eval::Context;
        use std::time::Instant;

        // (lambda (n) (while (> n 0) (setq n (1- n))) n)
        let ops = [
            Op::StackRef(0),
            Op::Constant(0),
            Op::Gtr,
            Op::GotoIfNil(8),
            Op::StackRef(0),
            Op::Sub1,
            Op::StackSet(1),
            Op::Goto(0),
            Op::StackRef(0),
            Op::Return,
        ];
        let constants = [Value::make_int(0)];
        let iters: i64 = 3_000_000;
        let calls = 5;

        let mut ev = Context::new_minimal_vm_harness();

        // Tier 0.
        let mut f = ByteCodeFunction::new(LambdaParams {
            required: vec![crate::emacs_core::intern::SymId(1)],
            optional: Vec::new(),
            rest: None,
        });
        f.lexical = true;
        f.ops = ops.to_vec();
        f.constants = constants.to_vec();
        f.max_stack = 16;
        let t0 = Instant::now();
        for _ in 0..calls {
            let mut vm = Vm::from_context(&mut ev);
            let r = vm.execute(&f, vec![Value::make_int(iters)]).unwrap();
            assert_eq!(r, Value::make_int(0));
        }
        let interp = t0.elapsed();

        // JIT.
        let leaf = lower_leaf(&ops, &constants, 1).unwrap();
        let ctx_ptr = &mut ev as *mut Context as *mut u8;
        let t1 = Instant::now();
        for _ in 0..calls {
            assert_eq!(
                leaf.call(ctx_ptr, &[Value::make_int(iters)]),
                NativeRun::Ok(Value::make_int(0).bits())
            );
        }
        let jit = t1.elapsed();

        eprintln!(
            "[jit-bench] countdown {iters}x{calls}: interp {interp:?}  jit {jit:?}  speedup {:.1}x",
            interp.as_secs_f64() / jit.as_secs_f64()
        );
    }

    /// Differential fuzzing (the Phase-9 discipline, brought forward): generate
    /// seeded random straight-line bodies over the supported non-allocating op
    /// subset, run each through BOTH tiers, and hold the tiering contract:
    /// - `Ok(bits)`  -> the interpreter must produce exactly those bits;
    /// - `Deopt`     -> the seam reruns the interpreter (sound by the poisoning
    ///                  analysis), so any interpreter outcome is acceptable;
    /// - `Signal`    -> the interpreter must also signal.
    #[test]
    fn fuzz_straightline_bodies_match_interpreter() {
        use crate::emacs_core::bytecode::Vm;
        use crate::emacs_core::eval::Context;

        // Deterministic xorshift64* — no external randomness (reproducible; on
        // failure the seed in the assert message reproduces the body).
        fn next(state: &mut u64) -> u64 {
            let mut x = *state;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            *state = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }

        let mut ev = Context::new_minimal_vm_harness();
        let ctx_ptr = &mut ev as *mut Context as *mut u8;

        // Constant pool: small fixnums, the fixnum boundaries, nil and t —
        // enough to hit fast paths, deopt boundaries, and type guards. No heap
        // values, so Ok-results compare exactly by bits.
        let constants: Vec<Value> = vec![
            Value::make_int(0),
            Value::make_int(1),
            Value::make_int(-1),
            Value::make_int(2),
            Value::make_int(3),
            Value::make_int(Value::MOST_POSITIVE_FIXNUM),
            Value::make_int(Value::MOST_NEGATIVE_FIXNUM),
            Value::NIL,
            Value::T,
        ];

        for seed in 1u64..=600 {
            let mut rng = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let len = 1 + (next(&mut rng) % 18) as usize;
            let mut ops: Vec<Op> = Vec::with_capacity(len + 2);
            let mut depth: usize = 0;
            for _ in 0..len {
                let r = (next(&mut rng) % 100) as usize;
                let op = if depth == 0 || r < 30 {
                    // Pushes (always valid).
                    match next(&mut rng) % 3 {
                        0 => Op::Nil,
                        1 => Op::True,
                        _ => Op::Constant((next(&mut rng) % constants.len() as u64) as u16),
                    }
                } else if depth >= 2 && r < 60 {
                    // Binary ops.
                    match next(&mut rng) % 11 {
                        0 => Op::Add,
                        1 => Op::Sub,
                        2 => Op::Mul,
                        3 => Op::Div,
                        4 => Op::Rem,
                        5 => Op::Eqlsign,
                        6 => Op::Lss,
                        7 => Op::Gtr,
                        8 => Op::Leq,
                        9 => Op::Geq,
                        _ => Op::Eq,
                    }
                } else if r < 85 {
                    // Unary ops (depth >= 1).
                    match next(&mut rng) % 10 {
                        0 => Op::Add1,
                        1 => Op::Sub1,
                        2 => Op::Negate,
                        3 => Op::Null,
                        4 => Op::Not,
                        5 => Op::Consp,
                        6 => Op::Stringp,
                        7 => Op::Listp,
                        8 => Op::Symbolp,
                        _ => Op::Dup,
                    }
                } else {
                    // Stack shuffles.
                    match next(&mut rng) % 3 {
                        0 => Op::Dup,
                        1 => Op::StackRef((next(&mut rng) % depth as u64) as u16),
                        _ if depth >= 2 => {
                            Op::StackSet(1 + (next(&mut rng) % (depth as u64 - 1)) as u16)
                        }
                        _ => Op::Pop,
                    }
                };
                let (needs, delta) = simple_effect(&op).expect("generator emits supported ops");
                if depth < needs {
                    continue; // skip an op the current depth can't support
                }
                depth = (depth as i64 + delta) as usize;
                ops.push(op);
            }
            if depth == 0 {
                ops.push(Op::Constant(0));
            }
            ops.push(Op::Return);

            // Tier 0 (oracle).
            let mut f = ByteCodeFunction::new(LambdaParams {
                required: Vec::new(),
                optional: Vec::new(),
                rest: None,
            });
            f.lexical = true;
            f.ops = ops.clone();
            f.constants = constants.clone();
            f.max_stack = 64;
            let interp = {
                let mut vm = Vm::from_context(&mut ev);
                vm.execute(&f, vec![])
            };

            // JIT.
            let leaf = lower_leaf(&ops, &constants, 0)
                .unwrap_or_else(|e| panic!("seed {seed}: body must compile, got {e}: {ops:?}"));
            match leaf.call(ctx_ptr, &[]) {
                NativeRun::Ok(bits) => {
                    let want = interp.as_ref().unwrap_or_else(|e| {
                        panic!("seed {seed}: JIT Ok but interpreter erred ({e:?}): {ops:?}")
                    });
                    assert_eq!(
                        bits,
                        want.bits(),
                        "seed {seed}: JIT/interpreter mismatch on {ops:?}"
                    );
                }
                NativeRun::Deopt => {
                    // The seam reruns the interpreter; nothing further to hold.
                }
                NativeRun::DeoptAt {
                    pc,
                    stack,
                    handlers,
                    binds,
                    spec_base,
                    cond_base,
                } => {
                    // Precise deopt: resume mid-function and the result must
                    // match the pure-interpreter run exactly.
                    let mut vm = crate::emacs_core::bytecode::Vm::from_context(&mut ev);
                    let resumed = vm.run_resumed_frame(
                        &f,
                        Value::NIL,
                        pc,
                        &stack,
                        handlers,
                        &binds,
                        spec_base,
                        cond_base,
                    );
                    match (&resumed, &interp) {
                        (Ok(got), Ok(want)) => assert_eq!(
                            got.bits(),
                            want.bits(),
                            "seed {seed}: resume/interpreter mismatch on {ops:?}"
                        ),
                        (Err(_), Err(_)) => {}
                        other => panic!(
                            "seed {seed}: resume/interpreter outcome mismatch {other:?}: {ops:?}"
                        ),
                    }
                }
                NativeRun::Signal => {
                    let _ = take_pending_flow();
                    assert!(
                        interp.is_err(),
                        "seed {seed}: JIT signaled but interpreter succeeded: {ops:?}"
                    );
                }
            }

            // Also exercise the typed-MIR Tier-2 path (build_mir + lower_mir_pure)
            // on the same body, skipping bodies the pure subset bails on. Localizes
            // lower_mir_pure miscompiles (the module-test failures under MIR wiring).
            if let Ok(mir) = mir::build_mir(&ops, &constants, 0) {
                if let Ok(mleaf) = lower_mir_pure(&mir) {
                    match mleaf.call(ctx_ptr, &[]) {
                        NativeRun::Ok(bits) => {
                            if let Ok(want) = &interp {
                                assert_eq!(
                                    bits,
                                    want.bits(),
                                    "seed {seed}: MIR/interpreter mismatch on {ops:?}"
                                );
                            }
                        }
                        NativeRun::Deopt | NativeRun::DeoptAt { .. } => {}
                        NativeRun::Signal => {
                            let _ = take_pending_flow();
                        }
                    }
                }
            }
        }
    }
}
