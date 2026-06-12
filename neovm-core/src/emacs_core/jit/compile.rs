//! Baseline bytecode → native lowering (Phase 3b).
//!
//! The first *real* compilation of neovm-core bytecode to machine code. It is
//! deliberately a tiny, always-correct vertical slice: it compiles only
//! **no-argument, straight-line, leaf** functions whose body uses the pure
//! operand-stack opcodes `{Constant, Nil, True, Pop, Dup, StackRef, Return}`,
//! and it **bails to the interpreter** (returns [`CompileError`]) on anything
//! else — arguments, control flow, arithmetic, variable access, calls,
//! allocation. None of the supported ops touch the heap, allocate, call back
//! into the runtime, or cross a GC safepoint, so this tier needs *none* of the
//! runtime-ABI / GC-stackmap / deopt machinery yet. Those arrive in later
//! increments, each gated the same speculative way: compile what is provably
//! safe, fall back otherwise.
//!
//! The bytecode operand stack is modelled at *compile time* as a `Vec` of
//! Cranelift SSA values (abstract interpretation): stack-only ops (`Dup`,
//! `StackRef`, `Pop`) just rearrange that vector and emit no code, while
//! `Constant`/`Nil`/`True` emit an `iconst` of the value's raw tagged bits and
//! `Return` terminates the block. A `Value` is opaque to native code here — it
//! flows as its `usize` bit pattern (`i64` in CLIF), exactly as the interpreter
//! stores it.

use cranelift_codegen::ir::Value as ClifValue;
use cranelift_codegen::ir::{AbiParam, Function, InstBuilder, Signature, UserFuncName, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

use super::backend::BackendError;
use crate::emacs_core::bytecode::ByteCodeFunction;
use crate::emacs_core::bytecode::opcode::Op;
use crate::emacs_core::value::Value;

/// Why a bytecode body could not be compiled by this baseline tier.
///
/// Every variant means "stay on the Tier-0 interpreter"; none is fatal. Matched
/// exhaustively, with no catch-all, per the JIT subsystem's completeness rule.
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

/// A compiled nullary leaf function: native code returning the raw tagged bits
/// of the resulting [`Value`].
///
/// Owns its [`JITModule`], which keeps the executable memory mapped for the
/// lifetime of this handle. The raw entry pointer makes this neither `Send` nor
/// `Sync`, which is correct — the code is tied to its owning module.
pub struct CompiledLeaf {
    // Field order matters for drop: `entry` is just a pointer into `_module`'s
    // memory; keep `_module` alive as long as the handle exists.
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
    /// Execute the compiled function and return the result's raw tagged bits.
    ///
    /// Reconstruct a [`Value`] from the result with `Value::from_bits`-equivalent
    /// logic at the call site (the interpreter compares via [`Value::bits`]).
    pub fn call(&self) -> usize {
        // SAFETY: `entry` points at finalized native code with ABI
        // `extern "C" fn() -> i64` (a nullary signature returning one i64, built
        // below). `_module` owns and keeps that code mapped for `&self`.
        unsafe {
            let f: extern "C" fn() -> i64 = core::mem::transmute(self.entry);
            f() as usize
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

/// Lower a straight-line, no-argument, leaf bytecode body to native code.
///
/// `ops` must end in `Return` and use only the supported pure-stack opcodes.
pub fn lower_nullary_leaf(ops: &[Op], constants: &[Value]) -> Result<CompiledLeaf, CompileError> {
    let builder = JITBuilder::new(default_libcall_names())
        .map_err(|e| CompileError::Backend(BackendError::ModuleInit(e.to_string())))?;
    let mut module = JITModule::new(builder);
    let call_conv = module.target_config().default_call_conv;

    // Native signature: () -> i64 (the result's raw tagged bits).
    let mut sig = Signature::new(call_conv);
    sig.returns.push(AbiParam::new(types::I64));

    let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig.clone());
    let mut fbctx = FunctionBuilderContext::new();
    let mut returned = false;
    {
        let mut fb = FunctionBuilder::new(&mut func, &mut fbctx);
        let block = fb.create_block();
        fb.switch_to_block(block);
        fb.seal_block(block);

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
                Op::Return => {
                    let result = stack.pop().ok_or(CompileError::StackUnderflow)?;
                    fb.ins().return_(&[result]);
                    returned = true;
                    // Anything after Return is unreachable dead code; stop here so
                    // we keep a single, well-terminated block.
                    break;
                }
                other => return Err(CompileError::UnsupportedOp(op_category(other))),
            }
        }

        if !returned {
            // Abandon the half-built function; nothing was finalized.
            return Err(CompileError::NoReturn);
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
        assert_eq!(
            leaf.call(),
            c.bits(),
            "native result must equal the constant"
        );
    }

    #[test]
    fn compiles_nil_and_true() {
        assert_eq!(
            lower_nullary_leaf(&[Op::Nil, Op::Return], &[])
                .unwrap()
                .call(),
            Value::NIL.bits()
        );
        assert_eq!(
            lower_nullary_leaf(&[Op::True, Op::Return], &[])
                .unwrap()
                .call(),
            Value::T.bits()
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
        assert_eq!(leaf.call(), b.bits());
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
        assert_eq!(leaf.call(), a.bits());
    }

    #[test]
    fn bails_on_arithmetic() {
        // Add is out of the supported subset -> must refuse, not miscompile.
        let err = lower_nullary_leaf(
            &[Op::Constant(0), Op::Constant(0), Op::Add, Op::Return],
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
        assert_eq!(leaf.call(), c.bits());
    }
}
