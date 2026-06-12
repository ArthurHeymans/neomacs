//! Baseline bytecode → native lowering (Phase 3b/3c).
//!
//! The first *real* compilation of neovm-core bytecode to machine code. It is
//! deliberately a small, always-correct vertical slice: it compiles only
//! **no-argument, straight-line, leaf** functions whose body uses the pure
//! operand-stack opcodes `{Constant, Nil, True, Pop, Dup, StackRef, Return}`
//! plus the fixnum fast paths `{Add, Sub}`, and it **bails to the interpreter**
//! (returns [`CompileError`]) on anything else — arguments, control flow, other
//! arithmetic, variable access, calls, allocation.
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
//! ABI: `extern "C" fn(out: *mut i64) -> i64`. Returns 1 and writes the result's
//! raw tagged bits through `out` on success; returns 0 (deopt) otherwise,
//! leaving `out` untouched. None of the supported ops allocate or cross a GC
//! safepoint, so this tier still needs none of the runtime-ABI / GC-stackmap
//! machinery — that arrives when calls and allocation are lowered.
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
    /// The function takes arguments; only nullary functions are handled yet.
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

/// A compiled nullary leaf function.
///
/// Owns its [`JITModule`], which keeps the executable memory mapped for the
/// lifetime of this handle. The raw entry pointer makes this neither `Send` nor
/// `Sync`, which is correct — the code is tied to its owning module.
pub struct CompiledLeaf {
    // Field order matters for drop: `entry` points into `_module`'s memory; keep
    // `_module` alive as long as the handle exists.
    entry: *const u8,
    _module: JITModule,
}

impl core::fmt::Debug for CompiledLeaf {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // `JITModule` is not `Debug`; show only the entry pointer.
        f.debug_struct("CompiledLeaf")
            .field("entry", &self.entry)
            .finish_non_exhaustive()
    }
}

impl CompiledLeaf {
    /// Execute the compiled function.
    ///
    /// Returns `Some(bits)` with the result's raw tagged [`Value`] bits on
    /// success, or `None` if the native code **deoptimized** (a speculation
    /// guard failed) and the caller must fall back to the interpreter.
    pub fn call(&self) -> Option<usize> {
        let mut out: i64 = 0;
        // SAFETY: `entry` is finalized native code with ABI
        // `extern "C" fn(*mut i64) -> i64` (built in `lower_nullary_leaf`): it
        // writes the result bits through the out-pointer and returns 1 on
        // success, or returns 0 without touching `out` on deopt. `_module` keeps
        // the code mapped for `&self`, and the `out` local outlives the call.
        let ok = unsafe {
            let f: extern "C" fn(*mut i64) -> i64 = core::mem::transmute(self.entry);
            f(&mut out as *mut i64)
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

/// True iff the lambda list binds no parameters at all.
fn is_nullary(f: &ByteCodeFunction) -> bool {
    f.params.required.is_empty() && f.params.optional.is_empty() && f.params.rest.is_none()
}

/// Compile a whole [`ByteCodeFunction`] if it is a nullary leaf; otherwise bail.
pub fn compile_bytecode_function(f: &ByteCodeFunction) -> Result<CompiledLeaf, CompileError> {
    if !is_nullary(f) {
        return Err(CompileError::TakesArguments);
    }
    lower_nullary_leaf(&f.ops, &f.constants)
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
    // Guard: both operands are fixnums  ((v & 0b11) == 0b10).
    let a_tag = fb.ins().band_imm(a, FIXNUM_CHECK_MASK as i64);
    let a_fix = fb
        .ins()
        .icmp_imm(IntCC::Equal, a_tag, FIXNUM_CHECK_VALUE as i64);
    let b_tag = fb.ins().band_imm(b, FIXNUM_CHECK_MASK as i64);
    let b_fix = fb
        .ins()
        .icmp_imm(IntCC::Equal, b_tag, FIXNUM_CHECK_VALUE as i64);
    let both = fb.ins().band(a_fix, b_fix);
    emit_guard(fb, deopt, both);

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

    // Retag: (res << 2) | 2.
    let shifted = fb.ins().ishl_imm(res, FIXNUM_SHIFT as i64);
    fb.ins().bor_imm(shifted, FIXNUM_CHECK_VALUE as i64)
}

/// Lower a straight-line, no-argument, leaf bytecode body to native code.
///
/// `ops` must end in `Return` and use only the supported subset.
pub fn lower_nullary_leaf(ops: &[Op], constants: &[Value]) -> Result<CompiledLeaf, CompileError> {
    let builder = JITBuilder::new(default_libcall_names())
        .map_err(|e| CompileError::Backend(BackendError::ModuleInit(e.to_string())))?;
    let mut module = JITModule::new(builder);
    let call_conv = module.target_config().default_call_conv;
    let ptr_ty = module.target_config().pointer_type();

    // ABI: fn(out: *mut i64) -> i64.  Returns 1 + writes result bits via `out`
    // on success; returns 0 (deopt) otherwise.
    let mut sig = Signature::new(call_conv);
    sig.params.push(AbiParam::new(ptr_ty));
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
        let out_ptr = fb.block_params(entry)[0];

        // The shared deopt landing block, created lazily on the first guard.
        let mut deopt: Option<Block> = None;
        // Compile-time model of the bytecode operand stack.
        let mut stack: Vec<ClifValue> = Vec::with_capacity(8);

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
        assert_eq!(leaf.call(), Some(c.bits()));
    }

    #[test]
    fn compiles_nil_and_true() {
        assert_eq!(
            lower_nullary_leaf(&[Op::Nil, Op::Return], &[])
                .unwrap()
                .call(),
            Some(Value::NIL.bits())
        );
        assert_eq!(
            lower_nullary_leaf(&[Op::True, Op::Return], &[])
                .unwrap()
                .call(),
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
        assert_eq!(leaf.call(), Some(b.bits()));
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
        assert_eq!(leaf.call(), Some(a.bits()));
    }

    #[test]
    fn compiles_fixnum_add() {
        // (+ 40 2) -> 42, all fixnums in range
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Add, Op::Return],
            &[Value::make_int(40), Value::make_int(2)],
        )
        .unwrap();
        assert_eq!(leaf.call(), Some(Value::make_int(42).bits()));
    }

    #[test]
    fn compiles_fixnum_sub_including_negative() {
        // (- 3 10) -> -7
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(1), Op::Sub, Op::Return],
            &[Value::make_int(3), Value::make_int(10)],
        )
        .unwrap();
        assert_eq!(leaf.call(), Some(Value::make_int(-7).bits()));
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
        assert_eq!(leaf.call(), None);
    }

    #[test]
    fn add_non_fixnum_operand_deopts() {
        // a = fixnum 5, b = nil -> not both fixnums -> deopt.
        let leaf = lower_nullary_leaf(
            &[Op::Constant(0), Op::Nil, Op::Add, Op::Return],
            &[Value::make_int(5)],
        )
        .unwrap();
        assert_eq!(leaf.call(), None);
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
        assert_eq!(leaf.call(), Some(Value::make_int(-1).bits()));
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
        assert_eq!(leaf.call(), Some(c.bits()));
    }
}
