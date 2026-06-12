//! Baseline bytecode → native lowering.
//!
//! Real compilation of neovm-core bytecode to machine code, grown as a series of
//! always-correct vertical slices. It compiles **straight-line leaf** functions
//! with 0+ required (stack) arguments, whose body uses the pure operand-stack
//! opcodes `{Constant, Nil, True, Pop, Dup, StackRef, Return}` plus the fixnum
//! fast paths `{Add, Sub, 1+, 1-, unary -}`, and it **bails to the interpreter**
//! (returns [`CompileError`]) on anything else — `&optional`/`&rest` params,
//! dynamic binding, control flow, other arithmetic, variables, calls, allocation.
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
//! (deopt) otherwise, leaving `out` untouched. None of the supported ops allocate
//! or cross a GC safepoint, so this tier still needs none of the runtime-ABI /
//! GC-stackmap machinery — that arrives when calls and allocation are lowered.
//!
//! The bytecode operand stack is modelled at *compile time* as a `Vec` of
//! Cranelift SSA values (abstract interpretation). A `Value` is opaque to native
//! code: it flows as its `usize` bit pattern (`i64` in CLIF), exactly as the
//! interpreter stores it.

use cranelift_codegen::ir::Value as ClifValue;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    AbiParam, Block, Function, InstBuilder, MemFlags, Signature, UserFuncName, types,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use smallvec::SmallVec;

use super::backend::BackendError;
use crate::emacs_core::bytecode::ByteCodeFunction;
use crate::emacs_core::bytecode::opcode::Op;
use crate::emacs_core::value::Value;
use crate::tagged::value::{FIXNUM_CHECK_MASK, FIXNUM_CHECK_VALUE, FIXNUM_SHIFT};

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
    /// Number of fixed arguments the native code reads from the args pointer at
    /// entry; the caller must pass exactly this many.
    arity: usize,
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

impl CompiledLeaf {
    /// The number of fixed arguments this compiled function expects.
    pub fn arity(&self) -> usize {
        self.arity
    }

    /// Execute the compiled function with `args` (whose length must equal
    /// [`arity`](Self::arity)).
    ///
    /// Returns `Some(bits)` with the result's raw tagged [`Value`] bits on
    /// success, or `None` if the native code **deoptimized** (a speculation
    /// guard failed) and the caller must fall back to the interpreter.
    pub fn call(&self, args: &[Value]) -> Option<usize> {
        debug_assert_eq!(args.len(), self.arity, "compiled call arity mismatch");
        // Copy the argument bits into a contiguous i64 buffer for the native
        // ABI (no heap alloc for the common <= 8 args). A `Value` is an opaque
        // tagged word here; its `usize` bits ride unchanged in an `i64` slot.
        let arg_bits: SmallVec<[i64; 8]> = args.iter().map(|v| v.bits() as i64).collect();
        let mut out: i64 = 0;
        // SAFETY: `entry` is finalized native code with ABI
        // `extern "C" fn(args: *const i64, out: *mut i64) -> i64` (built in
        // `lower_leaf`): it reads `self.arity` words from `args`, writes the
        // result bits through `out`, and returns 1 on success or 0 (without
        // touching `out`) on deopt. `_module` keeps the code mapped for `&self`;
        // `arg_bits` and `out` outlive the call; for arity 0 `args` is never read.
        let ok = unsafe {
            let f: extern "C" fn(*const i64, *mut i64) -> i64 = core::mem::transmute(self.entry);
            f(arg_bits.as_ptr(), &mut out as *mut i64)
        };
        (ok != 0).then_some(out as usize)
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

/// Compile a [`ByteCodeFunction`] if it is a leaf with only required (stack)
/// parameters; otherwise bail.
///
/// Supports 0+ required args (placed on the operand stack at entry, exactly as
/// the interpreter does). Optional / `&rest` parameters — which need nil-padding
/// and rest-list construction — are not handled yet, nor is dynamic-binding
/// bytecode (whose params are not on the stack).
pub fn compile_bytecode_function(f: &ByteCodeFunction) -> Result<CompiledLeaf, CompileError> {
    if !f.params.optional.is_empty() || f.params.rest.is_some() {
        return Err(CompileError::TakesArguments);
    }
    let arity = f.params.required.len();
    if arity > 0 && !params_on_stack(f) {
        // Required params are dynamically bound, not on the stack — `StackRef`
        // would not find them.
        return Err(CompileError::TakesArguments);
    }
    lower_leaf(&f.ops, &f.constants, arity)
}

/// Emit a speculation guard.
///
/// If `cond` (an `i8` boolean from `icmp`) is false, branch to the shared deopt
/// block — created lazily on first use; otherwise fall through into a fresh,
/// sealed continuation block. On return, the builder is positioned in the
/// continuation so lowering continues on the success path.
fn emit_guard(fb: &mut FunctionBuilder, deopt: &mut Option<Block>, cond: ClifValue) {
    let db = match *deopt {
        Some(b) => b,
        None => {
            let b = fb.create_block();
            *deopt = Some(b);
            b
        }
    };
    let cont = fb.create_block();
    fb.ins().brif(cond, cont, &[], db, &[]);
    fb.switch_to_block(cont);
    // `cont`'s only predecessor is the guard branch just emitted.
    fb.seal_block(cont);
}

/// Guard that `v` is a fixnum (`(v & 0b11) == 0b10`), deopting otherwise.
fn guard_fixnum(fb: &mut FunctionBuilder, deopt: &mut Option<Block>, v: ClifValue) {
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
    deopt: &mut Option<Block>,
    is_sub: bool,
    a: ClifValue,
    b: ClifValue,
) -> ClifValue {
    guard_fixnum(fb, deopt, a);
    guard_fixnum(fb, deopt, b);

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
    deopt: &mut Option<Block>,
    kind: UnaryKind,
    a: ClifValue,
) -> ClifValue {
    guard_fixnum(fb, deopt, a);
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

/// Lower a no-argument straight-line leaf body. Thin wrapper over [`lower_leaf`]
/// kept for the existing call sites/tests.
pub fn lower_nullary_leaf(ops: &[Op], constants: &[Value]) -> Result<CompiledLeaf, CompileError> {
    lower_leaf(ops, constants, 0)
}

/// Lower a straight-line leaf bytecode body taking `arity` fixed arguments.
///
/// `ops` must end in `Return` and use only the supported subset. The `arity`
/// arguments are loaded from the args pointer and seed the bottom of the operand
/// stack (arg0 deepest), exactly as the interpreter's `run_frame` pushes them —
/// so the body's `StackRef` opcodes reach them.
pub fn lower_leaf(
    ops: &[Op],
    constants: &[Value],
    arity: usize,
) -> Result<CompiledLeaf, CompileError> {
    let builder = JITBuilder::new(default_libcall_names())
        .map_err(|e| CompileError::Backend(BackendError::ModuleInit(e.to_string())))?;
    let mut module = JITModule::new(builder);
    let call_conv = module.target_config().default_call_conv;
    let ptr_ty = module.target_config().pointer_type();

    // ABI: fn(args: *const i64, out: *mut i64) -> i64. Reads `arity` argument
    // words from `args`; returns 1 + writes result bits via `out` on success, or
    // returns 0 (deopt) otherwise.
    let mut sig = Signature::new(call_conv);
    sig.params.push(AbiParam::new(ptr_ty)); // args
    sig.params.push(AbiParam::new(ptr_ty)); // out
    sig.returns.push(AbiParam::new(types::I64));

    let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig.clone());
    let mut fbctx = FunctionBuilderContext::new();
    let mut returned = false;
    {
        let mut fb = FunctionBuilder::new(&mut func, &mut fbctx);
        let entry = fb.create_block();
        fb.append_block_params_for_function_params(entry);
        fb.switch_to_block(entry);
        fb.seal_block(entry);
        let args_ptr = fb.block_params(entry)[0];
        let out_ptr = fb.block_params(entry)[1];

        // The shared deopt landing block, created lazily on the first guard.
        let mut deopt: Option<Block> = None;
        // Compile-time model of the bytecode operand stack, seeded with the
        // incoming arguments at the bottom of the frame (arg0 deepest), matching
        // the interpreter's run_frame.
        let mut stack: Vec<ClifValue> = Vec::with_capacity(arity + 8);
        for i in 0..arity {
            let v = fb
                .ins()
                .load(types::I64, MemFlags::trusted(), args_ptr, (i * 8) as i32);
            stack.push(v);
        }

        for op in ops {
            match op {
                Op::Constant(idx) => {
                    let v = constants
                        .get(*idx as usize)
                        .ok_or(CompileError::BadOperand)?;
                    stack.push(fb.ins().iconst(types::I64, v.bits() as i64));
                }
                Op::Nil => {
                    stack.push(fb.ins().iconst(types::I64, Value::NIL.bits() as i64));
                }
                Op::True => {
                    stack.push(fb.ins().iconst(types::I64, Value::T.bits() as i64));
                }
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
                Op::Add | Op::Sub => {
                    let b = stack.pop().ok_or(CompileError::StackUnderflow)?;
                    let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
                    let is_sub = matches!(op, Op::Sub);
                    let tagged = lower_fixnum_binop(&mut fb, &mut deopt, is_sub, a, b);
                    stack.push(tagged);
                }
                Op::Add1 | Op::Sub1 | Op::Negate => {
                    let a = stack.pop().ok_or(CompileError::StackUnderflow)?;
                    let kind = match op {
                        Op::Add1 => UnaryKind::Add1,
                        Op::Sub1 => UnaryKind::Sub1,
                        Op::Negate => UnaryKind::Negate,
                        _ => unreachable!("matched Add1/Sub1/Negate above"),
                    };
                    let tagged = lower_fixnum_unop(&mut fb, &mut deopt, kind, a);
                    stack.push(tagged);
                }
                Op::Return => {
                    let result = stack.pop().ok_or(CompileError::StackUnderflow)?;
                    fb.ins().store(MemFlags::trusted(), result, out_ptr, 0);
                    let one = fb.ins().iconst(types::I64, 1);
                    fb.ins().return_(&[one]);
                    returned = true;
                    // Anything after Return is unreachable dead code; stop here.
                    break;
                }
                other => return Err(CompileError::UnsupportedOp(op_category(other))),
            }
        }

        if !returned {
            // Abandon the half-built function; nothing was finalized.
            return Err(CompileError::NoReturn);
        }

        // Terminate the shared deopt block (return 0) iff any guard used it.
        if let Some(db) = deopt {
            fb.switch_to_block(db);
            fb.seal_block(db);
            let zero = fb.ins().iconst(types::I64, 0);
            fb.ins().return_(&[zero]);
        }

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
        assert_eq!(leaf.call(&[]), Some(c.bits()));
    }

    #[test]
    fn compiles_nil_and_true() {
        assert_eq!(
            lower_nullary_leaf(&[Op::Nil, Op::Return], &[])
                .unwrap()
                .call(&[]),
            Some(Value::NIL.bits())
        );
        assert_eq!(
            lower_nullary_leaf(&[Op::True, Op::Return], &[])
                .unwrap()
                .call(&[]),
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
        assert_eq!(leaf.call(&[]), Some(b.bits()));
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
        assert_eq!(leaf.call(&[]), Some(a.bits()));
    }

    #[test]
    fn compiles_fixnum_add() {
        // (+ 40 2) -> 42, all fixnums in range
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Add, Op::Return],
            &[Value::make_int(40), Value::make_int(2)],
        )
        .unwrap();
        assert_eq!(leaf.call(&[]), Some(Value::make_int(42).bits()));
    }

    #[test]
    fn compiles_fixnum_sub_including_negative() {
        // (- 3 10) -> -7
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Sub, Op::Return],
            &[Value::make_int(3), Value::make_int(10)],
        )
        .unwrap();
        assert_eq!(leaf.call(&[]), Some(Value::make_int(-7).bits()));
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
        assert_eq!(leaf.call(&[]), None);
    }

    #[test]
    fn add_non_fixnum_operand_deopts() {
        // a = fixnum 5, b = nil -> not both fixnums -> deopt.
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Nil, Op::Add, Op::Return],
            &[Value::make_int(5)],
        )
        .unwrap();
        assert_eq!(leaf.call(&[]), None);
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
        assert_eq!(leaf.call(&[]), Some(Value::make_int(-1).bits()));
    }

    #[test]
    fn compiles_unary_fixnum_ops() {
        // 1+ 41 -> 42
        let add1 = lower_nullary_leaf(
            &[Op::Constant(0), Op::Add1, Op::Return],
            &[Value::make_int(41)],
        )
        .unwrap();
        assert_eq!(add1.call(&[]), Some(Value::make_int(42).bits()));

        // 1- 43 -> 42
        let sub1 = lower_nullary_leaf(
            &[Op::Constant(0), Op::Sub1, Op::Return],
            &[Value::make_int(43)],
        )
        .unwrap();
        assert_eq!(sub1.call(&[]), Some(Value::make_int(42).bits()));

        // - 42 -> -42
        let neg = lower_nullary_leaf(
            &[Op::Constant(0), Op::Negate, Op::Return],
            &[Value::make_int(42)],
        )
        .unwrap();
        assert_eq!(neg.call(&[]), Some(Value::make_int(-42).bits()));
    }

    #[test]
    fn unary_boundary_inputs_deopt() {
        // 1+ MOST_POSITIVE -> overflow -> deopt
        let add1 = lower_nullary_leaf(
            &[Op::Constant(0), Op::Add1, Op::Return],
            &[Value::make_int(Value::MOST_POSITIVE_FIXNUM)],
        )
        .unwrap();
        assert_eq!(add1.call(&[]), None);

        // 1- MOST_NEGATIVE -> underflow -> deopt
        let sub1 = lower_nullary_leaf(
            &[Op::Constant(0), Op::Sub1, Op::Return],
            &[Value::make_int(Value::MOST_NEGATIVE_FIXNUM)],
        )
        .unwrap();
        assert_eq!(sub1.call(&[]), None);

        // - MOST_NEGATIVE -> +MOST_POSITIVE+1 out of range -> deopt
        let neg = lower_nullary_leaf(
            &[Op::Constant(0), Op::Negate, Op::Return],
            &[Value::make_int(Value::MOST_NEGATIVE_FIXNUM)],
        )
        .unwrap();
        assert_eq!(neg.call(&[]), None);
    }

    #[test]
    fn unary_on_non_fixnum_deopts() {
        // 1+ t -> not a fixnum -> deopt
        let leaf = lower_nullary_leaf(&[Op::True, Op::Add1, Op::Return], &[]).unwrap();
        assert_eq!(leaf.call(&[]), None);
    }

    #[test]
    fn bails_on_unsupported_arithmetic() {
        // Mul is not in the supported subset -> refuse, do not miscompile.
        let err = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(0), Op::Mul, Op::Return],
            &[Value::make_int(1)],
        )
        .unwrap_err();
        assert!(matches!(err, CompileError::UnsupportedOp("arithmetic")));
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
        assert_eq!(leaf.call(&[]), Some(c.bits()));
    }

    #[test]
    fn one_arg_identity_and_increment() {
        // (lambda (x) x)
        let id = lower_leaf(&[Op::StackRef(0), Op::Return], &[], 1).unwrap();
        assert_eq!(id.arity(), 1);
        assert_eq!(
            id.call(&[Value::make_int(7)]),
            Some(Value::make_int(7).bits())
        );
        // (lambda (x) (1+ x))
        let inc = lower_leaf(&[Op::StackRef(0), Op::Add1, Op::Return], &[], 1).unwrap();
        assert_eq!(
            inc.call(&[Value::make_int(41)]),
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
            add.call(&[Value::make_int(40), Value::make_int(2)]),
            Some(Value::make_int(42).bits())
        );
        // A non-fixnum argument makes the speculative Add deopt.
        assert_eq!(add.call(&[Value::make_int(40), Value::NIL]), None);
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
            leaf.call(&[Value::make_int(1), Value::make_int(41)]),
            Some(Value::make_int(42).bits())
        );
    }

    #[test]
    fn compile_bytecode_function_bails_on_optional_rest_or_dynamic_params() {
        // &optional -> bail (nil-padding not handled).
        let mut opt = ByteCodeFunction::new(LambdaParams {
            required: Vec::new(),
            optional: vec![crate::emacs_core::intern::SymId(1)],
            rest: None,
        });
        opt.lexical = true;
        assert!(matches!(
            compile_bytecode_function(&opt),
            Err(CompileError::TakesArguments)
        ));

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
        ];
        for (i, (ops, consts)) in cases.iter().enumerate() {
            let want = interp_nullary(ops, consts).bits();
            let got = lower_nullary_leaf(ops, consts).unwrap().call(&[]);
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

        let got = lower_leaf(&ops, &[], 2).unwrap().call(&args);
        assert_eq!(got, Some(want), "JIT must match the interpreter with args");
    }

    // Note: the JIT's deopt *boundary* (out-of-range -> None) is covered by
    // `add_overflowing_fixnum_range_deopts` and `unary_boundary_inputs_deopt`.
    // A differential check against the interpreter's bignum-promotion path is
    // intentionally omitted here because `new_minimal_vm_harness` does not wire
    // the full `+`/bignum builtins (it signals on that fallback), so it cannot
    // serve as the oracle for the slow path.
}
