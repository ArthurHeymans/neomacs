//! Bytecode virtual machine — stack-based interpreter.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use smallvec::SmallVec;

use super::chunk::ByteCodeFunction;
use super::opcode::Op;
use crate::buffer::BufferManager;
use crate::emacs_core::advice::VariableWatcherList;
use crate::emacs_core::builtins;
use crate::emacs_core::coding::CodingSystemManager;
use crate::emacs_core::custom::CustomManager;
use crate::emacs_core::error::*;
use crate::emacs_core::eval::{
    ConditionFrame, Context, LispArgVec, ResumeTarget, SubrEntry, lookup_global_subr_entry,
    subr_entry_from_value,
};
use crate::emacs_core::intern::{SymId, intern, intern_uninterned, lookup_interned, resolve_sym};
use crate::emacs_core::regex::MatchData;
// storage_char_len and storage_substring no longer needed here — using emacs_char + LispString
use crate::emacs_core::value::*;
use crate::tagged::header::{SubrDispatchKind, SubrFn};
use crate::window::{FrameId, FrameManager, Window};

/// Local marker for catch/condition-case frames mirrored into the shared
/// condition runtime.
#[derive(Clone, Debug)]
enum Handler {
    /// Local marker corresponding to a catch/condition-case frame already
    /// stored in `Context.condition_stack`.
    Condition,
}

type HandlerStack = SmallVec<[Handler; 4]>;
type BindStack = SmallVec<[usize; 8]>;

use crate::emacs_core::eval::SpecBinding;

#[cold]
#[inline(never)]
fn invalid_bytecode_flow() -> Flow {
    signal("error", vec![Value::string("Invalid byte-code")])
}

#[cold]
#[inline(never)]
fn trace_invalid_bytecode_site(
    func: &ByteCodeFunction,
    reason: &str,
    pc: usize,
    frame_base: usize,
    frame_limit: usize,
    stack_len: usize,
    op: Option<&Op>,
) {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    if !*ENABLED.get_or_init(|| std::env::var_os("NEOMACS_TRACE_INVALID_BYTECODE").is_some()) {
        return;
    }

    let gnu_byte_offset = func.gnu_byte_offset_map.as_ref().and_then(|map| {
        map.iter()
            .find_map(|entry| (entry.instruction_index == pc).then_some(entry.byte_offset))
    });
    let op_window_start = pc.saturating_sub(8);
    let op_window_end = (pc + 8).min(func.ops.len());
    let op_window = func.ops[op_window_start..op_window_end]
        .iter()
        .enumerate()
        .map(|(idx, op)| format!("{}:{:?}", op_window_start + idx, op))
        .collect::<Vec<_>>()
        .join(" ");
    let raw_bytes = func.gnu_bytecode_bytes.as_ref().map(|bytes| {
        let start = gnu_byte_offset.unwrap_or(0).saturating_sub(12);
        let end = (gnu_byte_offset.unwrap_or(0) + 24).min(bytes.len());
        bytes[start..end]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    });
    tracing::error!(
        reason,
        pc,
        gnu_byte_offset,
        ?op,
        op_window,
        raw_bytes,
        stack_len,
        frame_base,
        frame_limit,
        max_stack = func.max_stack,
        ops_len = func.ops.len(),
        constants_len = func.constants.len(),
        lexical = func.lexical,
        "Invalid byte-code"
    );
}

#[derive(Clone, Copy)]
enum DirectSubrCallee {
    Symbol(SymId),
    Value(Value),
}

impl DirectSubrCallee {
    #[inline]
    fn wrong_arity_value(self) -> Value {
        match self {
            Self::Symbol(sym_id) => Value::subr_from_sym_id(sym_id),
            Self::Value(value) => value,
        }
    }
}

#[inline(always)]
fn fixnum_tagged_i64(value: Value) -> i64 {
    debug_assert!(value.is_fixnum());
    // GNU bytecode.c compares XFIXNUM values for fixnum comparison opcodes.
    // Neomacs fixnums are `(n << 2) | 2`, so the signed tagged bits preserve
    // the same total order without materializing the untagged integer.
    value.bits() as i64
}

#[inline(always)]
fn fixnum_lt(left: Value, right: Value) -> bool {
    fixnum_tagged_i64(left) < fixnum_tagged_i64(right)
}

#[inline(always)]
fn fixnum_gt(left: Value, right: Value) -> bool {
    fixnum_tagged_i64(left) > fixnum_tagged_i64(right)
}

#[inline(always)]
fn fixnum_le(left: Value, right: Value) -> bool {
    fixnum_tagged_i64(left) <= fixnum_tagged_i64(right)
}

#[inline(always)]
fn fixnum_ge(left: Value, right: Value) -> bool {
    fixnum_tagged_i64(left) >= fixnum_tagged_i64(right)
}

#[inline]
fn plus_sym_id() -> SymId {
    static PLUS: OnceLock<SymId> = OnceLock::new();
    *PLUS.get_or_init(|| intern("+"))
}

#[inline]
fn logand_sym_id() -> SymId {
    static LOGAND: OnceLock<SymId> = OnceLock::new();
    *LOGAND.get_or_init(|| intern("logand"))
}

#[inline]
fn logior_sym_id() -> SymId {
    static LOGIOR: OnceLock<SymId> = OnceLock::new();
    *LOGIOR.get_or_init(|| intern("logior"))
}

#[inline]
fn logxor_sym_id() -> SymId {
    static LOGXOR: OnceLock<SymId> = OnceLock::new();
    *LOGXOR.get_or_init(|| intern("logxor"))
}

/// The bytecode VM execution engine.
///
/// Operates on an Context's obarray and dynamic binding stack.
pub struct Vm<'a> {
    ctx: &'a mut crate::emacs_core::eval::Context,
}

// Match the evaluator's coarse stack-growth policy so deeply recursive
// bytecode/macroexpansion paths don't exhaust the native thread stack before
// `max-lisp-eval-depth` handling can fire.
const VM_STACK_RED_ZONE: usize = 128 * 1024;
const VM_STACK_SEGMENT: usize = 2 * 1024 * 1024;
const VM_STACK_GROWTH_PROBE_START_DEPTH: usize = 16;
const VM_STACK_GROWTH_PROBE_INTERVAL: usize = 16;

impl<'a> crate::emacs_core::hook_runtime::HookRuntime for Vm<'a> {
    fn hook_context(&self) -> &crate::emacs_core::eval::Context {
        &self.ctx
    }

    fn call_hook_callable(&mut self, function: Value, args: &[Value]) -> EvalResult {
        self.call_function_with_roots(function, args)
    }

    fn remove_hook_function_after_error(&mut self, hook_sym: SymId, function: Value) {
        crate::emacs_core::hook_runtime::HookRuntime::remove_hook_function_after_error(
            &mut *self.ctx,
            hook_sym,
            function,
        );
    }

    fn with_hook_root_scope<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, Flow>,
    ) -> Result<T, Flow> {
        self.with_dynamic_vm_roots(|vm| f(vm))
    }

    fn push_hook_root(&mut self, value: Value) {
        self.push_dynamic_vm_root(value);
    }
}

impl<'a> Vm<'a> {
    pub(crate) fn from_context(ctx: &'a mut crate::emacs_core::eval::Context) -> Self {
        Self { ctx }
    }

    /// Set the current depth and max_depth (inherited from the Context).
    pub fn set_depth(&mut self, depth: usize, max_depth: usize) {
        self.ctx.depth = depth;
        self.ctx.max_depth = max_depth;
    }

    /// Get the current depth (to sync back to the Context).
    pub fn get_depth(&self) -> usize {
        self.ctx.depth
    }

    #[inline(always)]
    fn with_dynamic_vm_roots<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let scope = self.ctx.save_vm_roots();
        let result = f(self);
        self.ctx.restore_vm_roots(scope);
        result
    }

    #[inline]
    fn maybe_grow_vm_stack<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let depth = self.ctx.depth;
        if depth < VM_STACK_GROWTH_PROBE_START_DEPTH
            || !depth.is_multiple_of(VM_STACK_GROWTH_PROBE_INTERVAL)
        {
            return f(self);
        }
        stacker::maybe_grow(VM_STACK_RED_ZONE, VM_STACK_SEGMENT, || f(self))
    }

    fn with_bytecode_call_depth<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, Flow>,
    ) -> Result<T, Flow> {
        self.ctx.depth += 1;
        if self.ctx.depth > self.ctx.max_depth {
            if self.ctx.max_depth < 100 {
                self.ctx.max_depth = 100;
            }
            if self.ctx.depth > self.ctx.max_depth {
                self.ctx.depth -= 1;
                return Err(signal(
                    "error",
                    vec![Value::string("Lisp nesting exceeds ‘max-lisp-eval-depth’")],
                ));
            }
        }

        let result = f(self);
        self.ctx.depth -= 1;
        result
    }

    #[inline(always)]
    fn with_vm_root_scope<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
        let scope = self.ctx.save_vm_roots();
        let result = f(self);
        self.ctx.restore_vm_roots(scope);
        result
    }

    #[inline(always)]
    fn push_dynamic_vm_root(&mut self, value: Value) {
        self.ctx.push_vm_frame_root(value);
    }

    fn cleanup_bytecode_frame(
        &mut self,
        result: EvalResult,
        condition_stack_base: usize,
        specpdl_base: usize,
        frame_base: usize,
    ) -> EvalResult {
        // GNU bytecode.c keeps a bytecode return value in `TOP` while
        // unwinding back to the caller. Neomacs uses recursive Rust frames,
        // so root the result while this frame removes condition/specpdl state
        // and truncates its bytecode stack slice.
        let root_scope = self.ctx.save_vm_roots();
        self.ctx.push_eval_result_roots(&result);
        self.ctx.truncate_condition_stack(condition_stack_base);
        self.ctx.unbind_to(specpdl_base);
        self.ctx.bc_buf.truncate(frame_base);
        self.ctx.bc_frames.pop();
        self.ctx.restore_vm_roots(root_scope);
        result
    }

    fn with_frame_roots<T>(
        &mut self,
        func: &ByteCodeFunction,
        extra: &[Value],
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        self.with_dynamic_vm_roots(|vm| {
            // The active bytecode frame already roots its constants for the
            // whole invocation; only transient values removed from bc_buf need
            // an explicit root while a nested call can GC.
            for value in extra.iter().copied() {
                vm.ctx.push_vm_frame_root(value);
            }
            f(vm)
        })
    }

    fn with_frame_arg_roots<A, T>(
        &mut self,
        func: &ByteCodeFunction,
        args: A,
        f: impl FnOnce(&mut Self, A) -> T,
    ) -> T
    where
        A: AsRef<[Value]>,
    {
        self.with_frame_roots(func, &[], |vm| {
            for value in args.as_ref().iter().copied() {
                vm.ctx.push_vm_frame_root(value);
            }
            f(vm, args)
        })
    }

    fn with_frame_call_roots<A, T>(
        &mut self,
        func: &ByteCodeFunction,
        function: Value,
        args: A,
        f: impl FnOnce(&mut Self, A) -> T,
    ) -> T
    where
        A: AsRef<[Value]>,
    {
        self.with_frame_roots(func, &[], |vm| {
            vm.ctx.push_vm_frame_root(function);
            for value in args.as_ref().iter().copied() {
                vm.ctx.push_vm_frame_root(value);
            }
            f(vm, args)
        })
    }

    fn with_macro_expansion_scope<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, Flow>,
    ) -> Result<T, Flow> {
        let state = self.ctx.begin_macro_expansion_scope();
        let result = f(self);
        self.ctx.finish_macro_expansion_scope(state);
        result
    }

    fn collect_flow_roots(flow: &Flow, out: &mut Vec<Value>) {
        match flow {
            Flow::Signal(sig) => {
                out.push(Value::from_sym_id(sig.symbol));
                out.extend(sig.data.iter().copied());
                if let Some(raw) = sig.raw_data {
                    out.push(raw);
                }
            }
            Flow::Throw { tag, value } => {
                out.push(*tag);
                out.push(*value);
            }
        }
    }

    fn result_roots(result: &EvalResult) -> Vec<Value> {
        let mut roots = Vec::new();
        match result {
            Ok(value) => roots.push(*value),
            Err(flow) => Self::collect_flow_roots(flow, &mut roots),
        }
        roots
    }

    /// Execute a bytecode function with given arguments.
    pub(crate) fn execute(&mut self, func: &ByteCodeFunction, args: Vec<Value>) -> EvalResult {
        self.execute_with_func_value(func, args, Value::NIL)
    }

    /// Execute a bytecode function, passing through the original function
    /// value for use in `wrong-number-of-arguments` error reporting.
    pub(crate) fn execute_with_func_value(
        &mut self,
        func: &ByteCodeFunction,
        args: impl Into<LispArgVec>,
        func_value: Value,
    ) -> EvalResult {
        let args = args.into();

        // Root the bytecode function's constants so they survive GC during
        // nested calls. Heap bytecode calls also root func_value below, which
        // mirrors GNU's frame-held function object; direct/manual bytecode
        // execution can pass NIL func_value, so constants still need roots.
        let result = self.maybe_grow_vm_stack(|vm| {
            vm.with_dynamic_vm_roots(|vm| {
                if func_value.is_heap_object() {
                    vm.push_dynamic_vm_root(func_value);
                }
                for value in func.constants.iter().copied() {
                    vm.push_dynamic_vm_root(value);
                }
                vm.run_frame(func, args, func_value)
            })
        });
        result
    }

    /// Resume a bytecode frame MID-FUNCTION after a precise JIT deopt: a
    /// native guard failed at `start_pc` with the live operand stack `stack`,
    /// `handlers_active` condition frames registered by this frame still on
    /// `ctx.condition_stack`, and `bind_entries` (pre-push specpdl depths,
    /// drained from the JIT bind-stack segment) as the frame's outstanding
    /// dynamic binds. Ownership of those binds/handlers transfers here: the
    /// native caller performed NO frame unwind, and this frame's cleanup uses
    /// the native frame's entry bases (`specpdl_base`/`condition_stack_base`)
    /// so every exit unwinds exactly like the original frame would have.
    ///
    /// lexenv note: deliberately NOT the run_frame LexicalEnv prologue — the
    /// native frame never switched lexenv, and the only compilable op that
    /// reads it (UnwindProtectPop) uses the identical `ctx.lexenv` expression
    /// in its shim and interpreter arm, so resumed ops behave exactly as the
    /// remaining native ops would have.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn run_resumed_frame(
        &mut self,
        func: &ByteCodeFunction,
        func_value: Value,
        start_pc: usize,
        stack: &[Value],
        handlers_active: usize,
        bind_entries: &[usize],
        specpdl_base: usize,
        condition_stack_base: usize,
    ) -> EvalResult {
        let frame_base = self.ctx.bc_buf.len();
        self.ctx.bc_frames.push(crate::emacs_core::eval::BcFrame {
            base: frame_base,
            fun: func_value,
        });
        let frame_limit = match frame_base.checked_add(func.max_stack as usize) {
            Some(limit) => limit,
            None => {
                self.ctx.bc_frames.pop();
                return Err(invalid_bytecode_flow());
            }
        };
        if self.ctx.bc_buf.capacity() < frame_limit {
            self.ctx
                .bc_buf
                .reserve_exact(frame_limit - self.ctx.bc_buf.len());
        }
        // Seed the operand stack with the native frame's live values (traced
        // from here on; the caller performed no allocation since reading them
        // out of the spill buffer).
        self.ctx.bc_buf.extend_from_slice(stack);
        let mut pc = start_pc;
        let mut handlers = HandlerStack::new();
        for _ in 0..handlers_active {
            handlers.push(Handler::Condition);
        }
        let mut bind_stack: BindStack = bind_entries.iter().copied().collect();
        let result = self.run_loop(
            func,
            frame_base,
            frame_limit,
            &mut pc,
            &mut handlers,
            &mut bind_stack,
        );
        self.cleanup_bytecode_frame(result, condition_stack_base, specpdl_base, frame_base)
    }

    fn run_frame(
        &mut self,
        func: &ByteCodeFunction,
        args: LispArgVec,
        func_value: Value,
    ) -> EvalResult {
        let condition_stack_base = self.ctx.condition_stack_len();
        let frame_base = self.ctx.bc_buf.len();
        self.ctx.bc_frames.push(crate::emacs_core::eval::BcFrame {
            base: frame_base,
            fun: func_value,
        });
        let mut pc: usize = 0;
        let mut handlers = HandlerStack::new();
        let specpdl_base = self.ctx.specpdl.len();
        let mut bind_stack = BindStack::new();

        // Unified calling convention: push args onto the stack.
        // Both NeoVM-compiled and GNU-compiled bytecode use StackRef(n)
        // for parameter access.
        let nargs = args.len();
        let n_required = func.params.required.len();
        let n_optional = func.params.optional.len();
        let has_rest = func.params.rest.is_some();
        let nonrest = n_required + n_optional;

        // GNU Emacs validates bytecode arity before pushing the frame.
        // See src/bytecode.c: the VM checks the arg descriptor and signals
        // wrong-number-of-arguments immediately instead of nil-padding missing
        // required args.
        if !(n_required <= nargs && (has_rest || nargs <= nonrest)) {
            // GNU bytecode.c signals the raw bytecode descriptor pair
            // (mandatory . nonrest), even when the descriptor has the &rest
            // bit set.  This differs intentionally from func-arity, which
            // reports `many` for the same bytecode function.
            let arity = Value::cons(
                Value::fixnum(n_required as i64),
                Value::fixnum(nonrest as i64),
            );
            self.ctx.bc_buf.truncate(frame_base);
            self.ctx.bc_frames.pop();
            return Err(signal(
                "wrong-number-of-arguments",
                vec![arity, Value::fixnum(nargs as i64)],
            ));
        }

        let frame_limit = match frame_base.checked_add(func.max_stack as usize) {
            Some(limit) => limit,
            None => {
                self.ctx.bc_buf.truncate(frame_base);
                self.ctx.bc_frames.pop();
                return Err(invalid_bytecode_flow());
            }
        };
        if self.ctx.bc_buf.capacity() < frame_limit {
            self.ctx
                .bc_buf
                .reserve_exact(frame_limit - self.ctx.bc_buf.len());
        }

        // GNU's bytecode stores lexical params at known stack positions; the
        // byte-compiler emits `byte-stack-ref` for every lexical reference,
        // so the param names are NOT looked up at runtime and don't need any
        // environment entry.  Dynamic params, on the other hand, are
        // referenced via `byte-varref` and must be specbound on the
        // function's specpdl span.  This split mirrors `byte-compile-bind`
        // in bytecomp.el and matches GNU's `funcall_lambda` (eval.c) ->
        // `exec_byte_code` (bytecode.c).  Building an intermediate
        // OrderedRuntimeBindingMap of params per call (which the previous
        // code did even for the lexical case) is dead work that dominated
        // debug-build batch-byte-compile runtime.
        let has_named_params = nonrest > 0 || has_rest;
        let params_on_stack = func.lexical
            || func.env.is_some()
            || matches!(func.arglist.kind(), ValueKind::Fixnum(_));
        if params_on_stack {
            // Lexical bytecode follows GNU bytecode.c: exec_byte_code receives
            // the encoded arg template and pushes incoming arguments into the
            // bytecode frame before executing the first instruction.
            for i in 0..nonrest {
                if self.ctx.bc_buf.len() >= frame_limit {
                    self.ctx.bc_buf.truncate(frame_base);
                    self.ctx.bc_frames.pop();
                    return Err(invalid_bytecode_flow());
                }
                if i < nargs {
                    let v = args[i];
                    if v.is_string() {
                        let ptr = v.as_string_ptr().unwrap();
                        let hdr =
                            unsafe { &(*(ptr as *const crate::tagged::header::StringObj)).header };
                        if !matches!(hdr.kind, crate::tagged::header::HeapObjectKind::String) {
                            panic!(
                                "RUN_FRAME ARG BUG: arg[{}] = {:#x} (ptr {:?}, kind={:?}) is corrupt string. \
                                 nargs={}, func has {} required, {} optional, rest={}",
                                i,
                                v.0,
                                ptr,
                                hdr.kind,
                                nargs,
                                func.params.required.len(),
                                func.params.optional.len(),
                                func.params.rest.is_some(),
                            );
                        }
                    }
                    self.ctx.bc_buf.push(v);
                } else {
                    self.ctx.bc_buf.push(Value::NIL);
                }
            }

            if has_rest {
                if self.ctx.bc_buf.len() >= frame_limit {
                    self.ctx.bc_buf.truncate(frame_base);
                    self.ctx.bc_frames.pop();
                    return Err(invalid_bytecode_flow());
                }
                let rest_list = if nargs > nonrest {
                    Value::list_from_slice(&args[nonrest..])
                } else {
                    Value::NIL
                };
                self.ctx.bc_buf.push(rest_list);
            }
        }

        if has_named_params {
            if params_on_stack {
                // Lexical bytecode functions: params live on bc_buf at the
                // bottom of the frame.  Just install the captured closure
                // env (if any) and run; the body's stack-ref opcodes find
                // the params via frame_base.
                //
                // Save/restore lexenv via specpdl (matching GNU's specbind
                // pattern), not direct save/restore. This ensures unbind_to
                // handles all LexicalEnv entries consistently.
                use crate::emacs_core::eval::SpecBinding;
                self.ctx.specpdl.push(SpecBinding::LexicalEnv {
                    old_lexenv: self.ctx.lexenv,
                });
                if let Some(env) = func.env {
                    self.ctx.lexenv = env;
                }
                let result = self.run_loop(
                    func,
                    frame_base,
                    frame_limit,
                    &mut pc,
                    &mut handlers,
                    &mut bind_stack,
                );
                return self.cleanup_bytecode_frame(
                    result,
                    condition_stack_base,
                    specpdl_base,
                    frame_base,
                );
            }

            // Dynamic bytecode functions: each param needs a specbind so
            // that varref opcodes inside the body can find it via the
            // obarray.  GNU eval.c:funcall_lambda then calls exec_byte_code
            // with zero bytecode arguments, so dynamic params must not occupy
            // bytecode stack slots.
            let mut arg_idx = 0;
            for param in &func.params.required {
                let val = if arg_idx < nargs {
                    args[arg_idx]
                } else {
                    Value::NIL
                };
                crate::emacs_core::eval::specbind_in_state(
                    &mut self.ctx.obarray,
                    &mut self.ctx.specpdl,
                    *param,
                    val,
                );
                arg_idx += 1;
            }
            for param in &func.params.optional {
                let val = if arg_idx < nargs {
                    args[arg_idx]
                } else {
                    Value::NIL
                };
                crate::emacs_core::eval::specbind_in_state(
                    &mut self.ctx.obarray,
                    &mut self.ctx.specpdl,
                    *param,
                    val,
                );
                arg_idx += 1;
            }
            if let Some(rest_name) = func.params.rest {
                let rest_list = if arg_idx < nargs {
                    Value::list_from_slice(&args[arg_idx..])
                } else {
                    Value::NIL
                };
                crate::emacs_core::eval::specbind_in_state(
                    &mut self.ctx.obarray,
                    &mut self.ctx.specpdl,
                    rest_name,
                    rest_list,
                );
            }
            let result = self.run_loop(
                func,
                frame_base,
                frame_limit,
                &mut pc,
                &mut handlers,
                &mut bind_stack,
            );
            return self.cleanup_bytecode_frame(
                result,
                condition_stack_base,
                specpdl_base,
                frame_base,
            );
        }

        // No params: set up lexenv for lexical closures/functions, then run.
        // Save/restore via specpdl, matching GNU's specbind pattern.
        {
            use crate::emacs_core::eval::SpecBinding;
            if func.env.is_some() || func.lexical {
                self.ctx.specpdl.push(SpecBinding::LexicalEnv {
                    old_lexenv: self.ctx.lexenv,
                });
                if let Some(env) = func.env {
                    self.ctx.lexenv = env;
                }
            }
        }

        let result = self.run_loop(
            func,
            frame_base,
            frame_limit,
            &mut pc,
            &mut handlers,
            &mut bind_stack,
        );
        self.cleanup_bytecode_frame(result, condition_stack_base, specpdl_base, frame_base)
    }

    fn run_loop(
        &mut self,
        func: &ByteCodeFunction,
        frame_base: usize,
        frame_limit: usize,
        pc: &mut usize,
        handlers: &mut HandlerStack,
        bind_stack: &mut BindStack,
    ) -> EvalResult {
        let ops = &func.ops;
        let constants = &func.constants;
        let ops_len = ops.len();
        let ops_ptr = ops.as_ptr();
        let mut pc_local = *pc;
        let mut quitcounter: u8 = 1;

        macro_rules! stk {
            () => {
                self.ctx.bc_buf
            };
        }

        macro_rules! stk_push {
            ($val:expr) => {{
                let v = $val;
                #[cfg(debug_assertions)]
                if v.is_string() {
                    let ptr = v.as_string_ptr().unwrap();
                    let hdr =
                        unsafe { &(*(ptr as *const crate::tagged::header::StringObj)).header };
                    if !matches!(hdr.kind, crate::tagged::header::HeapObjectKind::String) {
                        panic!(
                            "BC_BUF PUSH BUG: pushing corrupt string {:#x} (ptr {:?}, kind={:?}) \
                             at pc={}, op={:?}, bc_buf.len()={}, frame_base={}",
                            v.0,
                            ptr,
                            hdr.kind,
                            pc_local.saturating_sub(1),
                            ops.get(pc_local.saturating_sub(1)),
                            stk!().len(),
                            frame_base,
                        );
                    }
                }
                let len = self.ctx.bc_buf.len();
                if len >= frame_limit {
                    let invalid_pc = pc_local.saturating_sub(1);
                    trace_invalid_bytecode_site(
                        func,
                        "push-frame-limit",
                        invalid_pc,
                        frame_base,
                        frame_limit,
                        len,
                        ops.get(invalid_pc),
                    );
                    self.resume_nonlocal(
                        func,
                        &mut pc_local,
                        handlers,
                        bind_stack,
                        invalid_bytecode_flow(),
                    )?;
                    continue;
                }
                let stack = &mut self.ctx.bc_buf;
                debug_assert!(len < stack.capacity());
                unsafe {
                    stack.as_mut_ptr().add(len).write(v);
                    stack.set_len(len + 1);
                }
            }};
        }

        macro_rules! vm_try {
            ($expr:expr) => {{
                match $expr {
                    Ok(value) => value,
                    Err(flow) => {
                        self.resume_nonlocal(func, &mut pc_local, handlers, bind_stack, flow)?;
                        continue;
                    }
                }
            }};
        }

        macro_rules! branch_to {
            ($target:expr) => {{
                let target = $target;
                if target < pc_local {
                    quitcounter = quitcounter.wrapping_add(1);
                    if quitcounter == 0 {
                        quitcounter = 1;
                        vm_try!(self.ctx.bytecode_branch_maybe_gc_and_quit());
                    }
                }
                pc_local = target;
            }};
        }

        macro_rules! invalid_bytecode {
            ($reason:expr) => {{
                let invalid_pc = pc_local.saturating_sub(1);
                trace_invalid_bytecode_site(
                    func,
                    $reason,
                    invalid_pc,
                    frame_base,
                    frame_limit,
                    self.ctx.bc_buf.len(),
                    ops.get(invalid_pc),
                );
                self.resume_nonlocal(
                    func,
                    &mut pc_local,
                    handlers,
                    bind_stack,
                    invalid_bytecode_flow(),
                )?;
                continue;
            }};
        }

        while pc_local < ops_len {
            let op = unsafe { &*ops_ptr.add(pc_local) };
            pc_local += 1;

            match op {
                // -- Constants and stack --
                Op::Constant(idx) => {
                    let Some(value) = constants.get(*idx as usize).copied() else {
                        let invalid_pc = pc_local.saturating_sub(1);
                        trace_invalid_bytecode_site(
                            func,
                            "constant-index-out-of-range",
                            invalid_pc,
                            frame_base,
                            frame_limit,
                            self.ctx.bc_buf.len(),
                            ops.get(invalid_pc),
                        );
                        self.resume_nonlocal(
                            func,
                            &mut pc_local,
                            handlers,
                            bind_stack,
                            invalid_bytecode_flow(),
                        )?;
                        continue;
                    };
                    stk_push!(value);
                }
                Op::Nil => stk_push!(Value::NIL),
                Op::True => stk_push!(Value::T),
                Op::Pop => {
                    if stk!().is_empty() {
                        invalid_bytecode!("pop-empty-stack");
                    }
                    stk!().pop();
                }
                Op::Dup => {
                    if pc_local + 2 < ops_len {
                        let next0 = unsafe { &*ops_ptr.add(pc_local) };
                        let next1 = unsafe { &*ops_ptr.add(pc_local + 1) };
                        let next2 = unsafe { &*ops_ptr.add(pc_local + 2) };
                        if let (Op::StackRef(stack_ref), Op::Lss, Op::GotoIfNil(target)) =
                            (next0, next1, next2)
                        {
                            let len = self.ctx.bc_buf.len();
                            if len == 0 {
                                invalid_bytecode!("dup-lss-gotoifnil-empty-stack");
                            }
                            if len >= frame_limit {
                                invalid_bytecode!("dup-lss-gotoifnil-stack-at-frame-limit");
                            }

                            let top = unsafe { *self.ctx.bc_buf.get_unchecked(len - 1) };
                            let after_dup_len = len + 1;
                            let offset = 1 + *stack_ref as usize;

                            if offset > after_dup_len || after_dup_len >= frame_limit {
                                let stack = &mut self.ctx.bc_buf;
                                unsafe {
                                    stack.as_mut_ptr().add(len).write(top);
                                    stack.set_len(after_dup_len);
                                }
                                pc_local += 1;
                                invalid_bytecode!("dup-lss-gotoifnil-stackref-out-of-range");
                            }

                            let ref_index = after_dup_len - offset;
                            let ref_value = if ref_index == len {
                                top
                            } else {
                                unsafe { *self.ctx.bc_buf.get_unchecked(ref_index) }
                            };

                            if top.is_fixnum() && ref_value.is_fixnum() {
                                pc_local += 3;
                                if !fixnum_lt(top, ref_value) {
                                    branch_to!(*target as usize);
                                }
                                continue;
                            }
                        }
                    }

                    if let Some(&top) = stk!().last() {
                        stk_push!(top);
                    } else {
                        invalid_bytecode!("dup-empty-stack");
                    }
                }
                Op::StackRef(n) => {
                    let offset = 1 + *n as usize;
                    let len = stk!().len();
                    if offset <= len {
                        // Valid bytecode references an existing stack slot.
                        // Keep the hot path to one explicit check and avoid
                        // the slice indexer's second bounds check.
                        let val = unsafe { *stk!().get_unchecked(len - offset) };
                        stk_push!(val);
                    } else {
                        let invalid_pc = pc_local.saturating_sub(1);
                        trace_invalid_bytecode_site(
                            func,
                            "stack-ref-out-of-range",
                            invalid_pc,
                            frame_base,
                            frame_limit,
                            len,
                            ops.get(invalid_pc),
                        );
                        self.resume_nonlocal(
                            func,
                            &mut pc_local,
                            handlers,
                            bind_stack,
                            invalid_bytecode_flow(),
                        )?;
                        continue;
                    }
                }
                Op::StackSet(n) => {
                    let len = stk!().len();
                    if len == 0 {
                        invalid_bytecode!("stack-set-empty-stack");
                    }
                    let n = *n as usize;
                    if n == 0 {
                        stk!().pop();
                        continue;
                    }
                    if n < len {
                        let stack = &mut self.ctx.bc_buf;
                        let val = unsafe { *stack.get_unchecked(len - 1) };
                        let idx = len - 1 - n;
                        unsafe {
                            *stack.get_unchecked_mut(idx) = val;
                            stack.set_len(len - 1);
                        }
                    } else {
                        invalid_bytecode!("stack-set-out-of-range");
                    }
                }
                Op::DiscardN(raw) => {
                    let preserve_tos = (raw & 0x80) != 0;
                    let n = (raw & 0x7F) as usize;
                    if n == 0 {
                        continue;
                    }
                    let len = stk!().len();
                    if n > len {
                        invalid_bytecode!("discard-n-out-of-range");
                    }
                    let stack = &mut self.ctx.bc_buf;
                    if preserve_tos {
                        if n >= len {
                            invalid_bytecode!("discard-n-preserve-tos-out-of-range");
                        }
                        let top = unsafe { *stack.get_unchecked(len - 1) };
                        let target = len - 1 - n;
                        unsafe {
                            *stack.get_unchecked_mut(target) = top;
                        }
                    }
                    unsafe {
                        stack.set_len(len - n);
                    }
                }

                // -- Variable access --
                Op::VarRef(idx) => {
                    let name_id = sym_id_at(constants, *idx);
                    let val = vm_try!(self.fast_path_var_ref(name_id));
                    stk_push!(val);
                }
                Op::VarSet(idx) => {
                    let name_id = sym_id_at(constants, *idx);
                    let val = stk!().pop().unwrap_or(Value::NIL);
                    let extra = [val];
                    vm_try!(
                        self.with_frame_roots(func, &extra, |vm| {
                            vm.assign_var_id(name_id, val)
                        },)
                    );
                }
                Op::VarBind(idx) => {
                    // GNU bytecode.c Bvarbind: `specbind (vectorp[arg], POP);`
                    // — always a dynamic binding, no lexical fallback. The
                    // byte-compiler (bytecomp.el byte-compile-bind) emits
                    // `byte-varbind` ONLY for variables that
                    // `cconv--not-lexical-var-p` reports as dynamic — i.e.
                    // members of `byte-compile-bound-variables`, populated
                    // from the file's top-level `(defvar VAR)` declarations
                    // among other sources. Lexical `let` bindings never get
                    // a varbind opcode at all; they live on the value stack
                    // and are tracked via `byte-compile--lexical-environment`.
                    //
                    // Therefore the VM must NOT second-guess the byte-compiler
                    // by inspecting `is_special_id` / `lexenv_declares_special`
                    // at runtime. Doing so misroutes file-local-only dynamic
                    // declarations (e.g. `(defvar cconv-freevars-alist)` in
                    // cconv.el — declared special locally but not globally) to
                    // the lexenv, where they are invisible to other functions
                    // called from the let body and surface as `void-variable`.
                    let name_id = sym_id_at(constants, *idx);
                    let val = stk!().pop().unwrap_or(Value::NIL);
                    bind_stack.push(self.ctx.specpdl.len());
                    self.ctx.specbind(name_id, val);
                }
                Op::Unbind(n) => {
                    let n = *n as usize;
                    let target = if n <= bind_stack.len() {
                        let depth = bind_stack[bind_stack.len() - n];
                        bind_stack.truncate(bind_stack.len() - n);
                        depth
                    } else {
                        bind_stack.clear();
                        0
                    };
                    self.ctx.unbind_to(target);
                }

                // -- Function calls --
                Op::Call(n) => {
                    let n = *n as usize;
                    let args_start = stk!().len().saturating_sub(n);
                    let stack_after_call = args_start.saturating_sub(1);
                    let func_val = if args_start > 0 {
                        stk!()[args_start - 1]
                    } else {
                        Value::NIL
                    };
                    // JIT Phase 1: record the callee for direct-call speculation.
                    // Only NAMED (symbol) callees carry a SymId; the call-site
                    // index is `pc_local - 1` (pc was advanced past Call above).
                    // GC-safe: a SymId is a stable index, never a heap pointer.
                    #[cfg(feature = "jit")]
                    if let ValueKind::Symbol(id) = func_val.kind() {
                        func.runtime.record_call(pc_local - 1, ops_len, id);
                    }
                    // GNU `bytecode.c:Bcall` polls `maybe_quit` before
                    // entering the callee. This is observable when bytecode
                    // sets `quit-flag` immediately before a call: the callee
                    // must not run.
                    vm_try!(self.ctx.maybe_quit());
                    let writeback_names = if n > 0 && stk!()[args_start].is_string() {
                        self.writeback_mutating_callable_names(&func_val)
                    } else {
                        None
                    };
                    let writeback_args = writeback_names
                        .as_ref()
                        .map(|_| stk!()[args_start..].iter().copied().collect::<LispArgVec>());
                    let result =
                        if writeback_names.is_none() {
                            vm_try!(self.with_bytecode_call_depth(|vm| {
                                vm.call_function_from_stack_args(func_val, args_start, n, true)
                            }))
                        } else {
                            let args: LispArgVec = stk!()[args_start..].iter().copied().collect();
                            vm_try!(self.with_bytecode_call_depth(|vm| {
                                vm.call_function(func_val, args)
                            }))
                        };
                    if let (Some((called_name, alias_target)), Some(writeback_args)) =
                        (writeback_names.as_ref(), writeback_args.as_ref())
                    {
                        let root_scope = self.ctx.save_vm_roots();
                        self.push_dynamic_vm_root(result);
                        for value in writeback_args.iter().copied() {
                            self.push_dynamic_vm_root(value);
                        }
                        self.maybe_writeback_mutating_first_arg(
                            called_name,
                            *alias_target,
                            writeback_args,
                            &result,
                        );
                        self.ctx.restore_vm_roots(root_scope);
                    }
                    stk!().truncate(stack_after_call);
                    stk_push!(result);
                }
                Op::Apply(n) => {
                    let n = *n as usize;
                    vm_try!(self.ctx.maybe_quit());
                    if n == 0 {
                        let stack_after_call = stk!().len().saturating_sub(1);
                        let func_val = stk!().last().copied().unwrap_or(Value::NIL);
                        let result = vm_try!(self.call_function(func_val, LispArgVec::new()));
                        stk!().truncate(stack_after_call);
                        stk_push!(result);
                    } else {
                        let args_start = stk!().len().saturating_sub(n);
                        let stack_after_call = args_start.saturating_sub(1);
                        let func_val = if args_start > 0 {
                            stk!()[args_start - 1]
                        } else {
                            Value::NIL
                        };
                        let mut args: Vec<Value> = stk!()[args_start..].to_vec();
                        // Spread last argument
                        if let Some(last) = args.pop() {
                            let spread = list_to_vec(&last).unwrap_or_default();
                            args.extend(spread);
                        }
                        let writeback_names = if args.first().is_some_and(|value| value.is_string())
                        {
                            self.writeback_mutating_callable_names(&func_val)
                        } else {
                            None
                        };
                        let writeback_args = writeback_names.as_ref().map(|_| args.clone());
                        let result = vm_try!(self.with_frame_call_roots(
                            func,
                            func_val,
                            args,
                            |vm, args| vm.call_function(func_val, args),
                        ));
                        if let (Some((called_name, alias_target)), Some(writeback_args)) =
                            (writeback_names.as_ref(), writeback_args.as_ref())
                        {
                            let root_scope = self.ctx.save_vm_roots();
                            self.push_dynamic_vm_root(result);
                            for value in writeback_args.iter().copied() {
                                self.push_dynamic_vm_root(value);
                            }
                            self.maybe_writeback_mutating_first_arg(
                                called_name,
                                *alias_target,
                                writeback_args,
                                &result,
                            );
                            self.ctx.restore_vm_roots(root_scope);
                        }
                        stk!().truncate(stack_after_call);
                        stk_push!(result);
                    }
                }

                // -- Control flow --
                // Backward branches mirror GNU `bytecode.c:op_branch`: an
                // unsigned byte `quitcounter` is incremented only for backward
                // jumps, and `maybe_gc(); maybe_quit();` runs when it wraps.
                Op::Goto(addr) => {
                    branch_to!(*addr as usize);
                }
                Op::GotoIfNil(addr) => {
                    let stack = &mut self.ctx.bc_buf;
                    let len = stack.len();
                    if len == 0 {
                        invalid_bytecode!("goto-if-nil-empty-stack");
                    }
                    let val = unsafe { *stack.get_unchecked(len - 1) };
                    unsafe {
                        stack.set_len(len - 1);
                    }
                    if val.is_nil() {
                        branch_to!(*addr as usize);
                    }
                }
                Op::GotoIfNotNil(addr) => {
                    let stack = &mut self.ctx.bc_buf;
                    let len = stack.len();
                    if len == 0 {
                        invalid_bytecode!("goto-if-not-nil-empty-stack");
                    }
                    let val = unsafe { *stack.get_unchecked(len - 1) };
                    unsafe {
                        stack.set_len(len - 1);
                    }
                    if val.is_truthy() {
                        branch_to!(*addr as usize);
                    }
                }
                Op::GotoIfNilElsePop(addr) => {
                    let stack = &mut self.ctx.bc_buf;
                    let len = stack.len();
                    if len == 0 {
                        invalid_bytecode!("goto-if-nil-else-pop-empty-stack");
                    }
                    if unsafe { stack.get_unchecked(len - 1) }.is_nil() {
                        branch_to!(*addr as usize);
                    } else {
                        unsafe {
                            stack.set_len(len - 1);
                        }
                    }
                }
                Op::GotoIfNotNilElsePop(addr) => {
                    let stack = &mut self.ctx.bc_buf;
                    let len = stack.len();
                    if len == 0 {
                        invalid_bytecode!("goto-if-not-nil-else-pop-empty-stack");
                    }
                    if unsafe { stack.get_unchecked(len - 1) }.is_truthy() {
                        branch_to!(*addr as usize);
                    } else {
                        unsafe {
                            stack.set_len(len - 1);
                        }
                    }
                }
                Op::Switch => {
                    let jump_table = stk!().pop().unwrap_or(Value::NIL);
                    let dispatch = stk!().pop().unwrap_or(Value::NIL);

                    if !matches!(
                        jump_table.kind(),
                        ValueKind::Veclike(VecLikeType::HashTable)
                    ) {
                        self.resume_nonlocal(
                            func,
                            &mut pc_local,
                            handlers,
                            bind_stack,
                            signal(
                                "wrong-type-argument",
                                vec![Value::symbol("hash-table-p"), jump_table],
                            ),
                        )?;
                        continue;
                    }

                    let ht = jump_table.as_hash_table().unwrap();
                    let key = dispatch.to_hash_key_swp(&ht.test, self.ctx.symbols_with_pos_enabled);
                    let target = ht.data.get(&key).copied();

                    match target {
                        Some(target_val) => match target_val.kind() {
                            ValueKind::Fixnum(addr) => {
                                pc_local = vm_try!(resolve_switch_target(func, addr));
                            }
                            _ => {
                                vm_try!(Err(signal(
                                    "wrong-type-argument",
                                    vec![Value::symbol("integerp"), target_val],
                                )));
                            }
                        },
                        None => {}
                    }
                }
                Op::Return => {
                    return Ok(stk!().pop().unwrap_or(Value::NIL));
                }
                Op::SaveCurrentBuffer => {
                    if let Some(buffer_id) =
                        self.ctx.buffers.current_buffer().map(|buffer| buffer.id)
                    {
                        bind_stack.push(self.ctx.specpdl.len());
                        self.ctx
                            .specpdl
                            .push(SpecBinding::SaveCurrentBuffer { buffer_id });
                    }
                }
                Op::SaveExcursion => {
                    if let Some(count) = self.ctx.record_save_excursion() {
                        bind_stack.push(count);
                    }
                }
                Op::SaveRestriction => {
                    if let Some(saved) = self.ctx.buffers.save_current_restriction_state() {
                        bind_stack.push(self.ctx.specpdl.len());
                        self.ctx
                            .specpdl
                            .push(SpecBinding::SaveRestriction { state: saved });
                    }
                }

                Op::SaveWindowExcursion => {
                    // GNU bytecode.c Bsave_window_excursion (opcode 139):
                    // Pop body form list, evaluate with Fprogn inside
                    // a real window-configuration save/restore.
                    //
                    // GNU `src/bytecode.c:945-952`:
                    //
                    //   record_unwind_protect (restore_window_configuration,
                    //                          Fcurrent_window_configuration (Qnil));
                    //   TOP = Fprogn (TOP);
                    //   unbind_to (count1, TOP);
                    //
                    // `save-some-buffers`, `map-y-or-n-p`, and other
                    // byte-compiled Lisp still rely on this obsolete opcode.
                    // Evaluating the body without restoring the window
                    // configuration leaves minibuffer/window state corrupted.
                    let body = stk!().pop().unwrap_or(Value::NIL);
                    let progn_form = Value::cons(Value::symbol("progn"), body);
                    let saved = vm_try!(
                        crate::emacs_core::window_cmds::builtin_current_window_configuration(
                            self.ctx,
                            vec![Value::NIL],
                        )
                    );
                    let body_result = self.ctx.eval_sub(progn_form);
                    let restore_result =
                        crate::emacs_core::window_cmds::builtin_set_window_configuration(
                            self.ctx,
                            vec![saved],
                        );

                    match body_result {
                        Ok(result) => {
                            vm_try!(restore_result);
                            stk_push!(result);
                        }
                        Err(flow) => {
                            vm_try!(restore_result);
                            self.resume_nonlocal(func, &mut pc_local, handlers, bind_stack, flow)?;
                            continue;
                        }
                    }
                }

                // -- Arithmetic --
                // Inline fixnum fast paths match GNU Emacs bytecode.c design:
                // the bytecode opcode IS the contract — no override check needed.
                Op::Add => {
                    let fallback = {
                        let stack = &mut self.ctx.bc_buf;
                        let len = stack.len();
                        if len < 2 {
                            invalid_bytecode!("add-stack-underflow");
                        }
                        let b = unsafe { *stack.get_unchecked(len - 1) };
                        let a = unsafe { *stack.get_unchecked(len - 2) };
                        if a.is_fixnum() && b.is_fixnum() {
                            let av = a.xfixnum();
                            let bv = b.xfixnum();
                            let res = av + bv;
                            if res >= Value::MOST_NEGATIVE_FIXNUM
                                && res <= Value::MOST_POSITIVE_FIXNUM
                            {
                                unsafe {
                                    *stack.get_unchecked_mut(len - 2) = Value::fixnum(res);
                                    stack.set_len(len - 1);
                                }
                                None
                            } else {
                                stack.truncate(len - 2);
                                Some((a, b))
                            }
                        } else {
                            stack.truncate(len - 2);
                            Some((a, b))
                        }
                    };
                    if let Some((a, b)) = fallback {
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "+", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Sub => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    if a.is_fixnum() && b.is_fixnum() {
                        let av = a.xfixnum();
                        let bv = b.xfixnum();
                        let res = av - bv;
                        if res >= Value::MOST_NEGATIVE_FIXNUM && res <= Value::MOST_POSITIVE_FIXNUM
                        {
                            stk!()[len - 2] = Value::fixnum(res);
                            stk!().pop();
                        } else {
                            stk!().truncate(len - 2);
                            let result =
                                vm_try!(self.dispatch_vm_builtin_with_frame(func, "-", vec![a, b]));
                            stk_push!(result);
                        }
                    } else {
                        stk!().truncate(len - 2);
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "-", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Mul => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    if a.is_fixnum() && b.is_fixnum() {
                        let av = a.xfixnum();
                        let bv = b.xfixnum();
                        if let Some(res) = av.checked_mul(bv) {
                            if res >= Value::MOST_NEGATIVE_FIXNUM
                                && res <= Value::MOST_POSITIVE_FIXNUM
                            {
                                stk!()[len - 2] = Value::fixnum(res);
                                stk!().pop();
                            } else {
                                stk!().truncate(len - 2);
                                let result = vm_try!(self.dispatch_vm_builtin_with_frame(
                                    func,
                                    "*",
                                    vec![a, b]
                                ));
                                stk_push!(result);
                            }
                        } else {
                            stk!().truncate(len - 2);
                            let result =
                                vm_try!(self.dispatch_vm_builtin_with_frame(func, "*", vec![a, b]));
                            stk_push!(result);
                        }
                    } else {
                        stk!().truncate(len - 2);
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "*", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Div => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    if a.is_fixnum() && b.is_fixnum() {
                        let av = a.xfixnum();
                        let bv = b.xfixnum();
                        if bv != 0 {
                            // Emacs truncation division (towards zero), matching C semantics
                            let res = if (av < 0) != (bv < 0) && av % bv != 0 {
                                av / bv
                            } else {
                                av / bv
                            };
                            stk!()[len - 2] = Value::fixnum(res);
                            stk!().pop();
                        } else {
                            stk!().truncate(len - 2);
                            let result =
                                vm_try!(self.dispatch_vm_builtin_with_frame(func, "/", vec![a, b]));
                            stk_push!(result);
                        }
                    } else {
                        stk!().truncate(len - 2);
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "/", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Rem => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    if a.is_fixnum() && b.is_fixnum() {
                        let av = a.xfixnum();
                        let bv = b.xfixnum();
                        if bv != 0 {
                            stk!()[len - 2] = Value::fixnum(av % bv);
                            stk!().pop();
                        } else {
                            stk!().truncate(len - 2);
                            let result =
                                vm_try!(self.dispatch_vm_builtin_with_frame(func, "%", vec![a, b]));
                            stk_push!(result);
                        }
                    } else {
                        stk!().truncate(len - 2);
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "%", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Add1 => {
                    let fallback = {
                        let stack = &mut self.ctx.bc_buf;
                        let len = stack.len();
                        if len == 0 {
                            invalid_bytecode!("add1-empty-stack");
                        }
                        let top = unsafe { *stack.get_unchecked(len - 1) };
                        if top.is_fixnum() {
                            let n = top.xfixnum();
                            if n != Value::MOST_POSITIVE_FIXNUM {
                                unsafe {
                                    *stack.get_unchecked_mut(len - 1) = Value::fixnum(n + 1);
                                }
                                None
                            } else {
                                unsafe {
                                    stack.set_len(len - 1);
                                }
                                Some(top)
                            }
                        } else {
                            unsafe {
                                stack.set_len(len - 1);
                            }
                            Some(top)
                        }
                    };
                    if let Some(top) = fallback {
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "1+", vec![top]));
                        stk_push!(result);
                    }
                }
                Op::Sub1 => {
                    let top = *stk!().last().unwrap();
                    if top.is_fixnum() {
                        let n = top.xfixnum();
                        if n != Value::MOST_NEGATIVE_FIXNUM {
                            *stk!().last_mut().unwrap() = Value::fixnum(n - 1);
                        } else {
                            stk!().pop();
                            let result =
                                vm_try!(self.dispatch_vm_builtin_with_frame(func, "1-", vec![top]));
                            stk_push!(result);
                        }
                    } else {
                        stk!().pop();
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "1-", vec![top]));
                        stk_push!(result);
                    }
                }
                Op::Negate => {
                    let top = *stk!().last().unwrap();
                    if top.is_fixnum() {
                        let n = top.xfixnum();
                        if n != Value::MOST_NEGATIVE_FIXNUM {
                            *stk!().last_mut().unwrap() = Value::fixnum(-n);
                        } else {
                            stk!().pop();
                            let result =
                                vm_try!(self.dispatch_vm_builtin_with_frame(func, "-", vec![top]));
                            stk_push!(result);
                        }
                    } else {
                        stk!().pop();
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "-", vec![top]));
                        stk_push!(result);
                    }
                }

                // -- Comparison --
                // Inline fixnum fast paths match GNU Emacs bytecode.c.
                Op::Eqlsign => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    if a.is_fixnum() && b.is_fixnum() {
                        stk!()[len - 2] = if a.0 == b.0 { Value::T } else { Value::NIL };
                        stk!().pop();
                    } else {
                        stk!().truncate(len - 2);
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "=", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Gtr => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    if a.is_fixnum() && b.is_fixnum() {
                        stk!()[len - 2] = if fixnum_gt(a, b) {
                            Value::T
                        } else {
                            Value::NIL
                        };
                        stk!().pop();
                    } else {
                        stk!().truncate(len - 2);
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, ">", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Lss => {
                    let fallback = {
                        let stack = &mut self.ctx.bc_buf;
                        let len = stack.len();
                        if len < 2 {
                            invalid_bytecode!("lss-stack-underflow");
                        }
                        let b = unsafe { *stack.get_unchecked(len - 1) };
                        let a = unsafe { *stack.get_unchecked(len - 2) };
                        if a.is_fixnum() && b.is_fixnum() {
                            unsafe {
                                *stack.get_unchecked_mut(len - 2) = if fixnum_lt(a, b) {
                                    Value::T
                                } else {
                                    Value::NIL
                                };
                                stack.set_len(len - 1);
                            }
                            None
                        } else {
                            stack.truncate(len - 2);
                            Some((a, b))
                        }
                    };
                    if let Some((a, b)) = fallback {
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "<", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Leq => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    if a.is_fixnum() && b.is_fixnum() {
                        stk!()[len - 2] = if fixnum_le(a, b) {
                            Value::T
                        } else {
                            Value::NIL
                        };
                        stk!().pop();
                    } else {
                        stk!().truncate(len - 2);
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "<=", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Geq => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    if a.is_fixnum() && b.is_fixnum() {
                        stk!()[len - 2] = if fixnum_ge(a, b) {
                            Value::T
                        } else {
                            Value::NIL
                        };
                        stk!().pop();
                    } else {
                        stk!().truncate(len - 2);
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, ">=", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Max => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    if a.is_fixnum() && b.is_fixnum() {
                        stk!()[len - 2] = if fixnum_ge(a, b) { a } else { b };
                        stk!().pop();
                    } else {
                        stk!().truncate(len - 2);
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "max", vec![a, b]));
                        stk_push!(result);
                    }
                }
                Op::Min => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    if a.is_fixnum() && b.is_fixnum() {
                        stk!()[len - 2] = if fixnum_le(a, b) { a } else { b };
                        stk!().pop();
                    } else {
                        stk!().truncate(len - 2);
                        let result =
                            vm_try!(self.dispatch_vm_builtin_with_frame(func, "min", vec![a, b]));
                        stk_push!(result);
                    }
                }

                // -- List operations --
                // Inline car/cdr/car-safe/cdr-safe match GNU Emacs exactly:
                // direct cons field access, nil passthrough, error on wrong type.
                Op::Car => {
                    let top = stk!().last_mut().unwrap();
                    if top.is_cons() {
                        *top = top.cons_car();
                    } else if !top.is_nil() {
                        let val = *top;
                        stk!().pop();
                        vm_try!(Err(signal(
                            "wrong-type-argument",
                            vec![Value::symbol("listp"), val]
                        )));
                    }
                    // nil → nil: no change needed
                }
                Op::Cdr => {
                    let top = stk!().last_mut().unwrap();
                    if top.is_cons() {
                        *top = top.cons_cdr();
                    } else if !top.is_nil() {
                        let val = *top;
                        stk!().pop();
                        vm_try!(Err(signal(
                            "wrong-type-argument",
                            vec![Value::symbol("listp"), val]
                        )));
                    }
                }
                Op::CarSafe => {
                    let top = stk!().last_mut().unwrap();
                    *top = if top.is_cons() {
                        top.cons_car()
                    } else {
                        Value::NIL
                    };
                }
                Op::CdrSafe => {
                    let top = stk!().last_mut().unwrap();
                    *top = if top.is_cons() {
                        top.cons_cdr()
                    } else {
                        Value::NIL
                    };
                }
                Op::Cons => {
                    let len = stk!().len();
                    let cdr_val = stk!()[len - 1];
                    let car_val = stk!()[len - 2];
                    stk!()[len - 2] = Value::cons(car_val, cdr_val);
                    stk!().pop();
                }
                Op::List(n) => {
                    let n = *n as usize;
                    let start = stk!().len().saturating_sub(n);
                    // GNU bytecode.c:BlistN keeps operands on the bytecode
                    // stack and calls Flist(n, &TOP).  Keep the same stack
                    // rooting discipline here and build from the live slice.
                    let result = Value::list_from_slice(&stk!()[start..]);
                    stk!().truncate(start);
                    stk_push!(result);
                }
                Op::Length => {
                    let len = stk!().len();
                    let val = stk!()[len - 1];
                    stk!()[len - 1] = vm_try!(builtins::builtin_length_1(&mut *self.ctx, val));
                }
                Op::Nth => {
                    let len = stk!().len();
                    let n = stk!()[len - 2];
                    let list = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_nth_2(&mut *self.ctx, n, list));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Nthcdr => {
                    let len = stk!().len();
                    let n = stk!()[len - 2];
                    let list = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_nthcdr_2(&mut *self.ctx, n, list));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Elt => {
                    let len = stk!().len();
                    let seq = stk!()[len - 2];
                    let idx = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_elt_2(&mut *self.ctx, seq, idx));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Setcar => {
                    let len = stk!().len();
                    let cell = stk!()[len - 2];
                    let newcar = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_setcar_2(&mut *self.ctx, cell, newcar));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Setcdr => {
                    let len = stk!().len();
                    let cell = stk!()[len - 2];
                    let newcdr = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_setcdr_2(&mut *self.ctx, cell, newcdr));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Nconc => {
                    let start = stk!().len().saturating_sub(2);
                    let result = vm_try!(builtins::builtin_nconc_slice_values(&stk!()[start..]));
                    stk!().truncate(start);
                    stk_push!(result);
                }
                Op::Nreverse => {
                    let len = stk!().len();
                    let value = stk!()[len - 1];
                    stk!()[len - 1] = vm_try!(builtins::builtin_nreverse_1(&mut *self.ctx, value));
                }
                Op::Member => {
                    let len = stk!().len();
                    let elt = stk!()[len - 2];
                    let list = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_member_2(&mut *self.ctx, elt, list));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Memq => {
                    let len = stk!().len();
                    let elt = stk!()[len - 2];
                    let list = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_memq_2(&mut *self.ctx, elt, list));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Assq => {
                    let len = stk!().len();
                    let key = stk!()[len - 2];
                    let alist = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_assq_2(&mut *self.ctx, key, alist));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }

                // -- Type predicates --
                // -- Type predicates --
                // Pure inline tag checks, zero function calls. Matches GNU exactly.
                Op::Symbolp => {
                    let top = stk!().last_mut().unwrap();
                    let is_sym = top.is_symbol()
                        || (self.ctx.symbols_with_pos_enabled && top.is_symbol_with_pos());
                    *top = if is_sym { Value::T } else { Value::NIL };
                }
                Op::Consp => {
                    let top = stk!().last_mut().unwrap();
                    *top = if top.is_cons() { Value::T } else { Value::NIL };
                }
                Op::Stringp => {
                    let top = stk!().last_mut().unwrap();
                    *top = if top.is_string() {
                        Value::T
                    } else {
                        Value::NIL
                    };
                }
                Op::Listp => {
                    let top = stk!().last_mut().unwrap();
                    *top = if top.is_cons() || top.is_nil() {
                        Value::T
                    } else {
                        Value::NIL
                    };
                }
                Op::Integerp => {
                    let top = stk!().last_mut().unwrap();
                    *top = if top.is_integer() {
                        Value::T
                    } else {
                        Value::NIL
                    };
                }
                Op::Numberp => {
                    let top = stk!().last_mut().unwrap();
                    *top = if top.is_number() {
                        Value::T
                    } else {
                        Value::NIL
                    };
                }
                Op::Null | Op::Not => {
                    let top = stk!().last_mut().unwrap();
                    *top = if top.is_nil() { Value::T } else { Value::NIL };
                }
                Op::Eq => {
                    let len = stk!().len();
                    let b = stk!()[len - 1];
                    let a = stk!()[len - 2];
                    let result = if a.0 == b.0 {
                        true
                    } else if self.ctx.symbols_with_pos_enabled {
                        crate::emacs_core::value::eq_value_swp(&a, &b, true)
                    } else {
                        false
                    };
                    stk!()[len - 2] = if result { Value::T } else { Value::NIL };
                    stk!().pop();
                }
                Op::Equal => {
                    let len = stk!().len();
                    let a = stk!()[len - 2];
                    let b = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_equal_2(&mut *self.ctx, a, b));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }

                // -- String operations --
                Op::Concat(n) => {
                    let n = *n as usize;
                    let start = stk!().len().saturating_sub(n);
                    // GNU bytecode.c:BconcatN passes the stack slice directly
                    // to Fconcat instead of materializing an argument vector.
                    let result = vm_try!(builtins::builtin_concat_slice(&stk!()[start..]));
                    stk!().truncate(start);
                    stk_push!(result);
                }
                Op::Substring => {
                    let start = stk!().len().saturating_sub(3);
                    let result = vm_try!(builtins::builtin_substring_slice(&stk!()[start..]));
                    stk!().truncate(start);
                    stk_push!(result);
                }
                Op::StringEqual => {
                    let len = stk!().len();
                    let a = stk!()[len - 2];
                    let b = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_string_equal_2(&mut *self.ctx, a, b));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::StringLessp => {
                    let len = stk!().len();
                    let a = stk!()[len - 2];
                    let b = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_string_lessp_2(&mut *self.ctx, a, b));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }

                // -- Vector operations --
                Op::Aref => {
                    let len = stk!().len();
                    let array = stk!()[len - 2];
                    let index = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_aref_2(&mut *self.ctx, array, index));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Aset => {
                    let val = stk!().pop().unwrap_or(Value::NIL);
                    let idx_val = stk!().pop().unwrap_or(Value::fixnum(0));
                    let vec_val = stk!().pop().unwrap_or(Value::NIL);
                    let mut call_args = LispArgVec::new();
                    call_args.push(vec_val);
                    call_args.push(idx_val);
                    call_args.push(val);
                    let result = if let Some(result) = vm_try!(self.maybe_call_named_function_cell(
                        func,
                        "aset",
                        call_args.clone(),
                    )) {
                        result
                    } else {
                        vm_try!(builtins::builtin_aset(call_args.clone().into_vec()))
                    };
                    let root_scope = self.ctx.save_vm_roots();
                    self.push_dynamic_vm_root(result);
                    for value in call_args.iter().copied() {
                        self.push_dynamic_vm_root(value);
                    }
                    self.maybe_writeback_mutating_first_arg("aset", None, &call_args, &result);
                    self.ctx.restore_vm_roots(root_scope);
                    stk_push!(result);
                }

                // -- Symbol operations --
                Op::SymbolValue => {
                    let len = stk!().len();
                    let sym = stk!()[len - 1];
                    stk!()[len - 1] =
                        vm_try!(builtins::builtin_symbol_value_1(&mut *self.ctx, sym));
                }
                Op::SymbolFunction => {
                    let len = stk!().len();
                    let sym = stk!()[len - 1];
                    stk!()[len - 1] =
                        vm_try!(builtins::builtin_symbol_function_1(&mut *self.ctx, sym));
                }
                Op::Set => {
                    let len = stk!().len();
                    let sym = stk!()[len - 2];
                    let val = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_set_2(&mut *self.ctx, sym, val));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Fset => {
                    let len = stk!().len();
                    let sym = stk!()[len - 2];
                    let val = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_fset_2(&mut *self.ctx, sym, val));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Get => {
                    let len = stk!().len();
                    let sym = stk!()[len - 2];
                    let prop = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_get_2(&mut *self.ctx, sym, prop));
                    stk!()[len - 2] = result;
                    stk!().pop();
                }
                Op::Put => {
                    let len = stk!().len();
                    let sym = stk!()[len - 3];
                    let prop = stk!()[len - 2];
                    let val = stk!()[len - 1];
                    let result = vm_try!(builtins::builtin_put_3(&mut *self.ctx, sym, prop, val));
                    stk!().truncate(len - 3);
                    stk_push!(result);
                }

                // -- Error handling --
                Op::PushConditionCase(target) => {
                    let stack_len = stk!().len();
                    let spec_depth = self.ctx.specpdl.len();
                    let bsl = bind_stack.len();
                    let resume_id = self.ctx.allocate_resume_id();
                    handlers.push(Handler::Condition);
                    self.ctx
                        .push_condition_frame(ConditionFrame::ConditionCase {
                            conditions: Value::symbol("error"),
                            resume: ResumeTarget::VmConditionCase {
                                resume_id,
                                target: *target,
                                stack_len,
                                spec_depth,
                                bind_stack_len: bsl,
                            },
                        });
                }
                Op::PushConditionCaseRaw(target) => {
                    // GNU bytecode consumes the handler pattern operand from TOS.
                    let conditions = stk!().pop().unwrap_or(Value::NIL);
                    let stack_len = stk!().len();
                    let spec_depth = self.ctx.specpdl.len();
                    let bsl = bind_stack.len();
                    let resume_id = self.ctx.allocate_resume_id();
                    handlers.push(Handler::Condition);
                    self.ctx
                        .push_condition_frame(ConditionFrame::ConditionCase {
                            conditions,
                            resume: ResumeTarget::VmConditionCase {
                                resume_id,
                                target: *target,
                                stack_len,
                                spec_depth,
                                bind_stack_len: bsl,
                            },
                        });
                }
                Op::PushCatch(target) => {
                    let tag = stk!().pop().unwrap_or(Value::NIL);
                    let stack_len = stk!().len();
                    let spec_depth = self.ctx.specpdl.len();
                    let bsl = bind_stack.len();
                    let resume_id = self.ctx.allocate_resume_id();
                    handlers.push(Handler::Condition);
                    self.ctx.push_condition_frame(ConditionFrame::Catch {
                        tag,
                        resume: ResumeTarget::VmCatch {
                            resume_id,
                            target: *target,
                            stack_len,
                            spec_depth,
                            bind_stack_len: bsl,
                        },
                    });
                }
                Op::PopHandler => {
                    if handlers.pop().is_some() {
                        self.ctx.pop_condition_frame();
                    }
                }
                Op::UnwindProtectPop => {
                    let cleanup = stk!().pop().unwrap_or(Value::NIL);
                    bind_stack.push(self.ctx.specpdl.len());
                    self.ctx.specpdl.push(SpecBinding::UnwindProtect {
                        forms: cleanup,
                        lexenv: self.ctx.lexenv,
                    });
                }
                Op::Throw => {
                    let val = stk!().pop().unwrap_or(Value::NIL);
                    let tag = stk!().pop().unwrap_or(Value::NIL);
                    self.resume_nonlocal(
                        func,
                        &mut pc_local,
                        handlers,
                        bind_stack,
                        Flow::Throw { tag, value: val },
                    )?;
                    continue;
                }

                // -- Closure --
                Op::MakeClosure(idx) => {
                    let val = constants[*idx as usize];
                    if let Some(bc_data) = val.get_bytecode_data() {
                        let mut closure = bc_data.clone();
                        closure.env = Some(self.ctx.lexenv);
                        stk_push!(Value::make_bytecode(closure));
                    } else {
                        stk_push!(val);
                    }
                }

                // -- Builtin escape hatch --
                Op::CallBuiltin(name_idx, n) => {
                    let name_id = sym_id_at(constants, *name_idx);
                    let name = resolve_sym(name_id);
                    let n = *n as usize;
                    let args_start = stk!().len().saturating_sub(n);
                    let args: LispArgVec = stk!()[args_start..].iter().copied().collect();
                    let writeback_args = (args.first().is_some_and(|value| value.is_string())
                        && Self::mutates_first_arg_name(name))
                    .then(|| args.clone());
                    let result = if self.named_builtin_fast_path_allowed_id(name_id) {
                        vm_try!(self.dispatch_vm_builtin_with_frame(func, name, args,))
                    } else {
                        let func_val = Value::from_sym_id(name_id);
                        vm_try!(
                            self.with_frame_call_roots(func, func_val, args, |vm, args| {
                                vm.call_function(func_val, args)
                            })
                        )
                    };
                    if let Some(writeback_args) = writeback_args.as_ref() {
                        let root_scope = self.ctx.save_vm_roots();
                        self.push_dynamic_vm_root(result);
                        for value in writeback_args.iter().copied() {
                            self.push_dynamic_vm_root(value);
                        }
                        self.maybe_writeback_mutating_first_arg(
                            name,
                            None,
                            writeback_args,
                            &result,
                        );
                        self.ctx.restore_vm_roots(root_scope);
                    }
                    stk!().truncate(args_start);
                    stk_push!(result);
                    vm_try!(self.ctx.maybe_quit());
                }
                // Mirrors GNU bytecode.c inline dispatch of opcodes
                // 0140-0177 etc. — the symbol name is encoded in the
                // op, no constants-pool lookup.
                Op::CallBuiltinSym(sym, n) => {
                    let name = crate::emacs_core::intern::resolve_sym(*sym);
                    let n = *n as usize;
                    let args_start = stk!().len().saturating_sub(n);
                    let args: LispArgVec = stk!()[args_start..].iter().copied().collect();
                    let writeback_args = (args.first().is_some_and(|value| value.is_string())
                        && Self::mutates_first_arg_name(name))
                    .then(|| args.clone());
                    // GNU-parity: opcodes 0140-0177 (decode.rs:295-303)
                    // dispatch *directly* to their C implementations
                    // (bytecode.c:1412-1545), bypassing the symbol's
                    // function cell and advice table. `(advice-add
                    // 'point ...)` deliberately does not fire when
                    // bytecode calls `(point)` via Bpoint — GNU docs
                    // this as a limitation of advice on
                    // bytecode-inlined primitives. Routing these
                    // through maybe_call_named_function_cell (which
                    // consults the symbol's function cell) would make
                    // neomacs MORE advisable than GNU, breaking parity.
                    let result = vm_try!(self.dispatch_vm_builtin_with_frame(func, name, args));
                    if let Some(writeback_args) = writeback_args.as_ref() {
                        let root_scope = self.ctx.save_vm_roots();
                        self.push_dynamic_vm_root(result);
                        for value in writeback_args.iter().copied() {
                            self.push_dynamic_vm_root(value);
                        }
                        self.maybe_writeback_mutating_first_arg(
                            name,
                            None,
                            writeback_args,
                            &result,
                        );
                        self.ctx.restore_vm_roots(root_scope);
                    }
                    stk!().truncate(args_start);
                    stk_push!(result);
                    vm_try!(self.ctx.maybe_quit());
                }
            }
        }

        // Fell off the end — return TOS or nil
        *pc = pc_local;
        Ok(stk!().pop().unwrap_or(Value::NIL))
    }

    // -- Helper methods --

    #[inline(always)]
    fn mutates_first_arg_name(name: &str) -> bool {
        name == "fillarray" || name == "aset"
    }

    #[inline]
    fn writeback_mutating_callable_names(
        &self,
        func_val: &Value,
    ) -> Option<(&'static str, Option<&'static str>)> {
        match func_val.kind() {
            ValueKind::Subr(_) | ValueKind::Veclike(VecLikeType::Subr)
                if func_val.as_subr_id().is_some() =>
            {
                let id = func_val.as_subr_id().unwrap();
                let name = resolve_sym(id);
                Self::mutates_first_arg_name(name).then_some((name, None))
            }
            ValueKind::Symbol(id) => {
                let name = resolve_sym(id);
                if Self::mutates_first_arg_name(name) {
                    return Some((name, None));
                }
                let alias_target =
                    self.ctx
                        .obarray
                        .symbol_function_id(id)
                        .and_then(|bound| match bound.kind() {
                            ValueKind::Symbol(tid) => {
                                let target = resolve_sym(tid);
                                Self::mutates_first_arg_name(target).then_some(target)
                            }
                            ValueKind::Subr(_) | ValueKind::Veclike(VecLikeType::Subr) => {
                                let tid = bound.as_subr_id().unwrap();
                                let target = resolve_sym(tid);
                                Self::mutates_first_arg_name(target).then_some(target)
                            }
                            _ => None,
                        });
                alias_target.map(|target| (name, Some(target)))
            }
            _ => None,
        }
    }

    fn builtin_name_id(name: &str) -> SymId {
        lookup_interned(name).unwrap_or_else(|| intern(name))
    }

    fn named_builtin_fast_path_allowed_id(&self, id: SymId) -> bool {
        if self.ctx.compiler_function_overrides_active() {
            return false;
        }
        match self.ctx.obarray.symbol_function_id(id) {
            Some(val) => match val.kind() {
                ValueKind::Subr(_) | ValueKind::Veclike(VecLikeType::Subr) => {
                    val.as_subr_id() == Some(id)
                }
                ValueKind::Nil => true,
                _ => false,
            },
            None => true,
        }
    }

    fn maybe_call_named_function_cell(
        &mut self,
        func: &ByteCodeFunction,
        name: &str,
        args: LispArgVec,
    ) -> Result<Option<Value>, Flow> {
        let id = Self::builtin_name_id(name);
        if self.named_builtin_fast_path_allowed_id(id) {
            return Ok(None);
        }

        let func_val = Value::from_sym_id(id);
        self.with_frame_call_roots(func, func_val, args, |vm, args| {
            vm.call_function(func_val, args)
        })
        .map(Some)
    }

    fn maybe_writeback_mutating_first_arg(
        &mut self,
        called_name: &str,
        alias_target: Option<&str>,
        call_args: &[Value],
        result: &Value,
    ) {
        let mutates_fillarray =
            called_name == "fillarray" || alias_target.is_some_and(|name| name == "fillarray");
        let mutates_aset = called_name == "aset" || alias_target.is_some_and(|name| name == "aset");
        if !mutates_fillarray && !mutates_aset {
            return;
        }

        let Some(first_arg) = call_args.first() else {
            return;
        };
        if !first_arg.is_string() {
            return;
        }

        let replacement = if mutates_fillarray {
            if !result.is_string() || eq_value(first_arg, result) {
                return;
            }
            *result
        } else {
            if call_args.len() < 3 {
                return;
            }
            let Ok(updated) =
                builtins::aset_string_replacement(first_arg, &call_args[1], &call_args[2])
            else {
                return;
            };
            if eq_value(first_arg, &updated) {
                return;
            }
            updated
        };

        if crate::emacs_core::value::equal_value(first_arg, &replacement, 0) {
            return;
        }

        let mut visited = HashSet::new();
        for value in self.ctx.bc_buf.iter_mut() {
            Self::replace_alias_refs_in_value(value, first_arg, &replacement, &mut visited);
        }
        // Walk the lexenv cons alist and replace alias refs in binding values
        {
            let mut lexenv_val = self.ctx.lexenv;
            Self::replace_alias_refs_in_value(
                &mut lexenv_val,
                first_arg,
                &replacement,
                &mut visited,
            );
            self.ctx.lexenv = lexenv_val;
        }
        // dynamic stack removed — specbind writes directly to obarray
        if let Some(current_id) = self.ctx.buffers.current_buffer_id()
            && let Some(buf) = self.ctx.buffers.get_mut(current_id)
        {
            for value in buf.bound_buffer_local_values_mut() {
                Self::replace_alias_refs_in_value(value, first_arg, &replacement, &mut visited);
            }
        }

        self.ctx.obarray.for_each_value_cell_mut(|value| {
            Self::replace_alias_refs_in_value(value, first_arg, &replacement, &mut visited);
        });
    }

    fn replace_alias_refs_in_value(
        value: &mut Value,
        from: &Value,
        to: &Value,
        visited: &mut HashSet<usize>,
    ) {
        if eq_value(value, from) {
            *value = *to;
            return;
        }

        match value.kind() {
            ValueKind::Cons => {
                let key = value.bits() ^ 0x1;
                if !visited.insert(key) {
                    return;
                }
                let mut new_car = value.cons_car();
                let mut new_cdr = value.cons_cdr();
                Self::replace_alias_refs_in_value(&mut new_car, from, to, visited);
                Self::replace_alias_refs_in_value(&mut new_cdr, from, to, visited);
                value.set_car(new_car);
                value.set_cdr(new_cdr);
            }
            ValueKind::Veclike(VecLikeType::Vector) => {
                let key = value.bits() ^ 0x2;
                if !visited.insert(key) {
                    return;
                }
                let mut data = value.as_vector_data().unwrap().clone();
                for item in data.iter_mut() {
                    Self::replace_alias_refs_in_value(item, from, to, visited);
                }
                let _ = value.replace_vector_data(data);
            }
            ValueKind::Veclike(VecLikeType::HashTable) => {
                let key = value.bits() ^ 0x4;
                if !visited.insert(key) {
                    return;
                }
                let old_ptr = match from.kind() {
                    ValueKind::String => Some(from.bits()),
                    _ => None,
                };
                let new_ptr = match to.kind() {
                    ValueKind::String => Some(to.bits()),
                    _ => None,
                };
                let _ = value.with_hash_table_mut(|ht| {
                    if matches!(ht.test, HashTableTest::Eq | HashTableTest::Eql) {
                        if let (Some(old_ptr), Some(new_ptr)) = (old_ptr, new_ptr) {
                            if let Some(existing) = ht.data.remove(&HashKey::Ptr(old_ptr)) {
                                ht.data.insert(HashKey::Ptr(new_ptr), existing);
                            }
                            if ht.key_snapshots.remove(&HashKey::Ptr(old_ptr)).is_some() {
                                ht.key_snapshots.insert(HashKey::Ptr(new_ptr), *to);
                            }
                            for k in &mut ht.insertion_order {
                                if *k == HashKey::Ptr(old_ptr) {
                                    *k = HashKey::Ptr(new_ptr);
                                }
                            }
                            for k in ht.entry_slots.iter_mut().flatten() {
                                if *k == HashKey::Ptr(old_ptr) {
                                    *k = HashKey::Ptr(new_ptr);
                                }
                            }
                            if let Some(slot) = ht.entry_slot_by_key.remove(&HashKey::Ptr(old_ptr))
                            {
                                ht.entry_slot_by_key.insert(HashKey::Ptr(new_ptr), slot);
                            }
                        }
                    }
                    for item in ht.data.values_mut() {
                        Self::replace_alias_refs_in_value(item, from, to, visited);
                    }
                });
            }
            _ => {}
        }
    }

    /// GNU bytecode `Bvarref` by SymId.
    ///
    /// GNU `src/bytecode.c` reads bytecode variables with `Fsymbol_value`;
    /// it does not consult the interpreter lexical environment.  Lexical
    /// bytecode variables are compiled as stack/closure accesses instead.
    /// Fast path for variable reads matching GNU bytecode.c:626-647
    /// Bvarref: if the symbol is a plain global with a bound value,
    /// read the value cell directly without full symbolic resolution.
    fn fast_path_var_ref(&mut self, name_id: SymId) -> EvalResult {
        let ob = &self.ctx.obarray;
        let sym = ob
            .get_by_id(name_id)
            .ok_or_else(|| signal("void-variable", vec![Value::from_sym_id(name_id)]))?;
        if sym.redirect() == crate::emacs_core::symbol::SymbolRedirect::Plainval {
            // SAFETY: redirect() already confirmed Plainval, so val.plain is active
            let val = unsafe { sym.val.plain };
            if !val.is_unbound() {
                // For variables like `buffer-undo-list`: the obarray
                // default is nil but the buffer-local value (via
                // SharedUndoState / local_var_alist) is the live value.
                // Check buffer-local when the Plainval is nil.
                if !val.is_nil() {
                    return Ok(val);
                }
                if let Some(buf) = self.ctx.buffers.current_buffer() {
                    if let Some(blv) = buf.get_buffer_local_by_sym_id(name_id) {
                        if !blv.is_nil() {
                            return Ok(blv);
                        }
                    }
                }
                return Ok(val);
            }
        }
        self.lookup_var_id(name_id)
    }

    fn lookup_var_id(&mut self, name_id: SymId) -> EvalResult {
        let resolved = crate::emacs_core::builtins::symbols::resolve_variable_alias_id_in_obarray(
            &self.ctx.obarray,
            name_id,
        )?;

        // Phase 9 of the symbol-redirect refactor: if the symbol's
        // redirect tag is LOCALIZED or FORWARDED, the new redirect
        // machinery is the source of truth. Route the read through
        // `find_symbol_value_in_buffer` which will swap the BLV
        // cache for LOCALIZED and read the slot for FORWARDED.
        //
        // For PLAINVAL / VARALIAS, fall through to the PLAINVAL fast path
        // via `find_symbol_value`. With Phase B complete, every LOCALIZED
        // symbol is handled by the redirect dispatch above.
        use crate::emacs_core::symbol::SymbolRedirect;
        let redirect = self.ctx.obarray.get_by_id(resolved).map(|s| s.redirect());
        if matches!(
            redirect,
            Some(SymbolRedirect::Localized | SymbolRedirect::Forwarded)
        ) {
            let (cur_val, alist, slots_ptr, buf_id, local_flags) =
                match self.ctx.buffers.current_buffer() {
                    Some(buf) => (
                        Value::make_buffer(buf.id),
                        buf.local_var_alist,
                        Some(&buf.slots[..] as *const [Value]),
                        Some(buf.id),
                        buf.local_flags,
                    ),
                    None => (Value::NIL, Value::NIL, None, None, 0u64),
                };
            let defaults_ptr: *const [Value] =
                &self.ctx.buffers.buffer_defaults[..] as *const [Value];
            // Safety: the slots and defaults pointers are valid for
            // the duration of this call because we hold `&mut self.ctx`,
            // the buffer and BufferManager live inside `self.ctx`, and
            // `find_symbol_value_in_buffer` does not mutate the
            // buffer manager. The raw pointer dance is only needed
            // because `find_symbol_value_in_buffer` also needs
            // `&mut self.ctx.obarray` for the BLV swap-in, and the
            // borrow checker can't express "hold slices of two
            // fields while mutating a third" across the method call.
            let slots_opt: Option<&[Value]> = slots_ptr.map(|p| unsafe { &*p });
            let defaults_opt: Option<&[Value]> = Some(unsafe { &*defaults_ptr });
            if let Some(val) = self.ctx.obarray.find_symbol_value_in_buffer(
                resolved,
                buf_id,
                cur_val,
                alist,
                slots_opt,
                local_flags,
                defaults_opt,
            ) {
                // `Qunbound` from the BLV cache / alist walk marks a
                // void LOCALIZED binding for this buffer — signal
                // `void-variable` instead of returning the sentinel
                // to the caller. Mirrors GNU `Fsymbol_value` which
                // signals when `find_symbol_value` returns
                // `Qunbound`.
                if val.is_unbound() {
                    return Err(signal("void-variable", vec![Value::from_sym_id(name_id)]));
                }
                return Ok(val);
            }
        }

        // For variables like `buffer-undo-list` that are not slot-backed
        // but have per-buffer state (SharedUndoState), the obarray
        // default is nil while the buffer-local value is the live
        // undo list.  Check buffer-local before falling through to
        // the obarray default so the byte-compiled code sees the
        // correct per-buffer value.
        if let Some(buf) = self.ctx.buffers.current_buffer() {
            if let Some(val) = buf.get_buffer_local_by_sym_id(name_id) {
                if !val.is_nil() {
                    return Ok(val);
                }
            }
        }

        // GNU `bytecode.c:Bvarref` falls back to `Fsymbol_value`.
        if let Some(val) = self
            .ctx
            .visible_runtime_variable_value_by_id_resolved(resolved)
        {
            return Ok(val);
        }

        // Retry buffer-local for nil-valued defaults (e.g. unset
        // `buffer-undo-list` on a clean buffer).
        if let Some(buf) = self.ctx.buffers.current_buffer() {
            if let Some(val) = buf.get_buffer_local_by_sym_id(name_id) {
                return Ok(val);
            }
        }

        Err(signal("void-variable", vec![Value::from_sym_id(name_id)]))
    }

    /// GNU bytecode `Bvarset` by SymId.
    ///
    /// Like `Bvarref`, bytecode assignment is dynamic.  Lexical bytecode
    /// locals are stack slots, not `varset` targets.
    fn assign_var_id(&mut self, name_id: SymId, value: Value) -> Result<(), Flow> {
        let resolved = crate::emacs_core::builtins::symbols::resolve_variable_alias_id_in_obarray(
            &self.ctx.obarray,
            name_id,
        )?;

        if self.ctx.obarray.is_constant_id(resolved) {
            return Err(signal(
                "setting-constant",
                vec![Value::from_sym_id(name_id)],
            ));
        }

        // Phase 9b of the symbol-redirect refactor: for LOCALIZED
        // symbols, route the write through
        // Obarray::set_internal_localized which updates the BLV
        // cache and (for auto-create `Set` writes with
        // `local_if_set`) extends the current buffer's
        // local_var_alist. The legacy set_runtime_binding_in_state
        // path below stays populated as a fallback until Phase 10
        // deletes it.
        use crate::emacs_core::symbol::{SetInternalBind, SymbolRedirect};
        let redirect = self.ctx.obarray.get_by_id(resolved).map(|s| s.redirect());
        // Phase 10B: FORWARDED writes go to the buffer slot the
        // descriptor points at. Mirrors GNU
        // `store_symval_forwarding` for the BUFFER_OBJFWD arm
        // (`data.c:1374-1471`).
        //
        // Phase 10D: for conditional slots (`local_flags_idx >= 0`),
        // also set the per-buffer local-flags bit so subsequent reads
        // route to `slots[off]` rather than `buffer_defaults`. This
        // mirrors GNU `set_internal` SYMBOL_FORWARDED arm at
        // `data.c:1774-1786` which calls `SET_PER_BUFFER_VALUE_P`.
        if matches!(redirect, Some(SymbolRedirect::Forwarded)) {
            if let Some(buf_id) = self.ctx.buffers.current_buffer_id() {
                use crate::emacs_core::forward::{LispBufferObjFwd, LispFwdType};
                let fwd_ptr = self
                    .ctx
                    .obarray
                    .get_by_id(resolved)
                    .map(|s| unsafe { s.val.fwd });
                if let Some(fwd) = fwd_ptr {
                    // Safety: install_buffer_objfwd leaks a 'static
                    // descriptor and the symbol's redirect tag is
                    // immutable once installed.
                    let header = unsafe { &*fwd };
                    if matches!(header.ty, LispFwdType::BufferObj) {
                        let buf_fwd = unsafe { &*(fwd as *const LispBufferObjFwd) };
                        let Some(slot) =
                            crate::buffer::buffer::BufferSlot::from_u16(buf_fwd.offset)
                        else {
                            return Err(signal(
                                "error",
                                vec![Value::string("Invalid buffer slot offset")],
                            ));
                        };
                        let offset = slot.index();
                        let flags_idx = buf_fwd.local_flags_idx;
                        let slot_exists = self
                            .ctx
                            .buffers
                            .get(buf_id)
                            .is_some_and(|buf| offset < buf.slots.len());
                        if slot_exists {
                            let where_value = Value::make_buffer(buf_id);
                            self.run_variable_watchers_by_id_with_where(
                                resolved,
                                &value,
                                &Value::NIL,
                                "set",
                                &where_value,
                            )?;
                            if let Some(buf) = self.ctx.buffers.get_mut(buf_id) {
                                buf.slots[offset] = value;
                                if flags_idx >= 0 {
                                    buf.set_slot_local_flag(slot, true);
                                }
                            }
                            self.ctx.sync_cached_runtime_binding_by_id(resolved, value);
                            return Ok(());
                        }
                    }
                }
            }
        }

        if matches!(redirect, Some(SymbolRedirect::Localized)) {
            if let Some(buf_id) = self.ctx.buffers.current_buffer_id() {
                // Extract buffer state before obarray borrow.
                let (cur_val, alist) = match self.ctx.buffers.get(buf_id) {
                    Some(buf) => (Value::make_buffer(buf.id), buf.local_var_alist),
                    None => (Value::NIL, Value::NIL),
                };
                // GNU `eval.c:3559-3577 (let_shadows_buffer_binding_p)`
                // only treats SPECPDL_LET_DEFAULT for the current buffer
                // as shadowing. SPECPDL_LET_LOCAL is explicitly excluded
                // by bug#62419.
                let let_shadows = self.ctx.let_shadows_buffer_binding_p(resolved);
                let where_value = self.ctx.variable_watcher_where_for_set_by_id(resolved);
                self.run_variable_watchers_by_id_with_where(
                    resolved,
                    &value,
                    &Value::NIL,
                    "set",
                    &where_value,
                )?;
                let new_alist = self.ctx.obarray.set_internal_localized(
                    resolved,
                    value,
                    cur_val,
                    alist,
                    SetInternalBind::Set,
                    let_shadows,
                );
                // Store back the (possibly extended) alist.
                if let Some(buf) = self.ctx.buffers.get_mut(buf_id) {
                    buf.local_var_alist = new_alist;
                }
                self.ctx.sync_cached_runtime_binding_by_id(resolved, value);
                return Ok(());
            }
        }

        // Legacy path: set_runtime_binding_in_state routes to
        // either BufferLocals or the obarray value cell. Phase 10
        // deletes this call once every LOCALIZED symbol is
        // exclusively served by the new BLV path above.
        let where_value = self.ctx.variable_watcher_where_for_set_by_id(resolved);
        self.run_variable_watchers_by_id_with_where(
            resolved,
            &value,
            &Value::NIL,
            "set",
            &where_value,
        )?;
        crate::emacs_core::eval::set_runtime_binding_in_state(&mut *self.ctx, resolved, value);
        self.ctx.sync_cached_runtime_binding_by_id(resolved, value);
        Ok(())
    }

    fn lookup_var(&mut self, name: &str) -> EvalResult {
        if name.starts_with(':') {
            return Ok(Value::keyword(name));
        }

        let name_id = intern(name);
        // Match GNU eval_sub: lexical environment lookup happens before
        // alias resolution fallback.
        if let Some(val) = self.ctx.lexenv_lookup_cached_in(self.ctx.lexenv, name_id) {
            return Ok(val);
        }
        let resolved = crate::emacs_core::builtins::symbols::resolve_variable_alias_id_in_obarray(
            &self.ctx.obarray,
            name_id,
        )?;
        if resolved != name_id
            && let Some(val) = self.ctx.lexenv_lookup_cached_in(self.ctx.lexenv, resolved)
        {
            return Ok(val);
        }

        // specbind writes directly to obarray, so dynamic stack lookup is
        // no longer needed — fall through to obarray lookup.

        // GNU `bytecode.c:Bvarref` falls back to `Fsymbol_value`,
        // not the raw symbol cell. Use the shared runtime reader so
        // bytecode observes the same forwarded/localized semantics as
        // tree-walk eval.
        if let Some(val) = self
            .ctx
            .visible_runtime_variable_value_by_id_resolved(resolved)
        {
            return Ok(val);
        }

        Err(signal("void-variable", vec![Value::symbol(name)]))
    }

    fn assign_var(&mut self, name: &str, value: Value) -> Result<(), Flow> {
        let name_id = intern(name);
        let resolved = crate::emacs_core::builtins::symbols::resolve_variable_alias_id_in_obarray(
            &self.ctx.obarray,
            name_id,
        )?;
        if let Some(cell_id) = self.ctx.lexenv_assq_cached_in(self.ctx.lexenv, name_id) {
            lexenv_set(cell_id, value);
            return Ok(());
        }
        if resolved != name_id
            && let Some(cell_id) = self.ctx.lexenv_assq_cached_in(self.ctx.lexenv, resolved)
        {
            lexenv_set(cell_id, value);
            return Ok(());
        }

        // specbind writes directly to obarray, so dynamic stack mutation
        // is no longer needed — fall through to obarray write.

        if self.ctx.obarray.is_constant_id(resolved) {
            return Err(signal("setting-constant", vec![Value::symbol(name)]));
        }

        let where_value = self.ctx.variable_watcher_where_for_set_by_id(resolved);
        self.run_variable_watchers_by_id_with_where(
            resolved,
            &value,
            &Value::NIL,
            "set",
            &where_value,
        )?;
        crate::emacs_core::eval::set_runtime_binding_in_state(&mut *self.ctx, resolved, value);
        Ok(())
    }

    fn run_variable_watchers_by_id(
        &mut self,
        sym_id: SymId,
        new_value: &Value,
        old_value: &Value,
        operation: &str,
    ) -> Result<(), Flow> {
        self.run_variable_watchers_by_id_with_where(
            sym_id,
            new_value,
            old_value,
            operation,
            &Value::NIL,
        )
    }

    fn run_variable_watchers_by_id_with_where(
        &mut self,
        sym_id: SymId,
        new_value: &Value,
        old_value: &Value,
        operation: &str,
        where_value: &Value,
    ) -> Result<(), Flow> {
        if !self.ctx.watchers.has_watchers(sym_id) {
            return Ok(());
        }
        if self.ctx.active_variable_watchers.contains(&sym_id) {
            return Ok(());
        }
        let calls =
            self.ctx
                .watchers
                .notify_watchers(sym_id, new_value, old_value, operation, where_value);
        self.ctx.active_variable_watchers.insert(sym_id);
        for (callback, args) in calls {
            if let Err(err) = self.call_function_with_roots(callback, &args) {
                self.ctx.active_variable_watchers.remove(&sym_id);
                return Err(err);
            }
        }
        self.ctx.active_variable_watchers.remove(&sym_id);
        Ok(())
    }

    fn run_variable_watchers(
        &mut self,
        name: &str,
        new_value: &Value,
        old_value: &Value,
        operation: &str,
    ) -> Result<(), Flow> {
        self.run_variable_watchers_by_id(intern(name), new_value, old_value, operation)
    }

    fn run_variable_watchers_with_where(
        &mut self,
        name: &str,
        new_value: &Value,
        old_value: &Value,
        operation: &str,
        where_value: &Value,
    ) -> Result<(), Flow> {
        self.run_variable_watchers_by_id_with_where(
            intern(name),
            new_value,
            old_value,
            operation,
            where_value,
        )
    }

    fn call_function_with_roots(&mut self, function: Value, args: &[Value]) -> EvalResult {
        self.call_function(function, args.iter().copied().collect::<LispArgVec>())
    }

    #[inline]
    fn call_function1(&mut self, function: Value, arg: Value) -> EvalResult {
        let mut args = LispArgVec::new();
        args.push(arg);
        self.call_function(function, args)
    }

    #[inline]
    fn call_function2(&mut self, function: Value, arg0: Value, arg1: Value) -> EvalResult {
        let mut args = LispArgVec::new();
        args.push(arg0);
        args.push(arg1);
        self.call_function(function, args)
    }

    fn builtin_run_hooks_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::hook_runtime::run_named_hooks(self, args)
    }

    fn builtin_run_hook_with_args_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_min_args("run-hook-with-args", args, 1)?;
        crate::emacs_core::hook_runtime::run_named_hook_with_args(self, args)
    }

    fn builtin_run_hook_with_args_until_success_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_min_args("run-hook-with-args-until-success", args, 1)?;
        crate::emacs_core::hook_runtime::run_named_hook_with_args_until_success(self, args)
    }

    fn builtin_run_hook_with_args_until_failure_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_min_args("run-hook-with-args-until-failure", args, 1)?;
        crate::emacs_core::hook_runtime::run_named_hook_with_args_until_failure(self, args)
    }

    fn builtin_run_hook_wrapped_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_min_args("run-hook-wrapped", args, 2)?;
        crate::emacs_core::hook_runtime::run_named_hook_wrapped(self, args)
    }

    fn builtin_run_hook_query_error_with_timeout_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("run-hook-query-error-with-timeout", args, 1)?;
        let hook_sym = crate::emacs_core::hook_runtime::resolve_hook_symbol(&self.ctx, args[0])?;
        let hook_value = crate::emacs_core::hook_runtime::hook_value_by_id(&self.ctx, hook_sym)
            .unwrap_or(Value::NIL);
        crate::emacs_core::hook_runtime::run_hook_query_error_with_timeout(
            self, hook_sym, hook_value,
        )
    }

    fn builtin_set_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("set", args, 2)?;
        let symbol = crate::emacs_core::builtins::symbols::expect_symbol_id(&args[0])?;
        let resolved = crate::emacs_core::builtins::symbols::resolve_variable_alias_id_in_obarray(
            &self.ctx.obarray,
            symbol,
        )?;
        let value = args[1];
        if let Some(result) = crate::emacs_core::builtins::symbols::constant_set_outcome_in_obarray(
            &self.ctx.obarray,
            resolved,
            args[0],
            value,
        ) {
            return result;
        }
        let where_value = self.ctx.variable_watcher_where_for_set_by_id(resolved);
        self.run_variable_watchers_by_id_with_where(
            resolved,
            &value,
            &Value::NIL,
            "set",
            &where_value,
        )?;
        crate::emacs_core::eval::set_runtime_binding_in_state(&mut *self.ctx, resolved, value);
        Ok(value)
    }

    fn builtin_set_default_shared(&mut self, args: &[Value]) -> EvalResult {
        use crate::emacs_core::builtins::symbols::resolve_variable_alias_id_in_obarray;

        if args.len() != 2 {
            return Err(signal(
                "wrong-number-of-arguments",
                vec![
                    Value::symbol("set-default"),
                    Value::fixnum(args.len() as i64),
                ],
            ));
        }
        let symbol = match args[0].kind() {
            ValueKind::Nil => intern("nil"),
            ValueKind::T => intern("t"),
            ValueKind::Symbol(id) => id,
            _ => {
                return Err(signal(
                    "wrong-type-argument",
                    vec![Value::symbol("symbolp"), args[0]],
                ));
            }
        };
        let resolved = resolve_variable_alias_id_in_obarray(&self.ctx.obarray, symbol)?;
        if let Some(result) = crate::emacs_core::builtins::symbols::constant_set_outcome_in_obarray(
            &self.ctx.obarray,
            resolved,
            args[0],
            args[1],
        ) {
            return result;
        }
        let value = args[1];

        self.run_variable_watchers_by_id(resolved, &value, &Value::NIL, "set")?;
        // GNU PLAINVAL path: for non-LOCALIZED variables, `set-default`
        // behaves like `set` — writes to dynamic frame if let-bound.
        let is_buffer_local =
            self.ctx.obarray.get_by_id(resolved).is_some_and(|s| {
                s.redirect() == crate::emacs_core::symbol::SymbolRedirect::Localized
            });
        if !is_buffer_local {
            crate::emacs_core::eval::set_runtime_binding_in_state(&mut *self.ctx, resolved, value);
        } else {
            self.ctx.obarray.set_symbol_value_id(resolved, value);
        }

        Ok(value)
    }

    fn builtin_set_default_toplevel_value_shared(&mut self, args: &[Value]) -> EvalResult {
        let symbol = crate::emacs_core::builtins::symbols::expect_symbol_id(&args[0])?;
        let resolved = crate::emacs_core::builtins::symbols::resolve_variable_alias_id_in_obarray(
            &self.ctx.obarray,
            symbol,
        )?;
        let value = args[1];
        if let Some(result) = crate::emacs_core::builtins::symbols::constant_set_outcome_in_obarray(
            &self.ctx.obarray,
            resolved,
            args[0],
            value,
        ) {
            result?;
            return Ok(Value::NIL);
        }
        self.run_variable_watchers_by_id(resolved, &value, &Value::NIL, "set")?;
        if resolved != symbol {
            self.run_variable_watchers_by_id(resolved, &value, &Value::NIL, "set")?;
        }
        crate::emacs_core::builtins::symbols::set_default_toplevel_value_impl(
            &mut *self.ctx,
            args.to_vec(),
        )?;
        Ok(Value::NIL)
    }

    fn builtin_defalias_shared(&mut self, args: &[Value]) -> EvalResult {
        let plan = crate::emacs_core::builtins::plan_defalias_in_obarray(&self.ctx.obarray, args)?;
        let crate::emacs_core::builtins::DefaliasPlan {
            action,
            docstring,
            result,
        } = plan;
        self.ctx
            .loadhist_attach(Value::cons(Value::symbol("defun"), result));
        match action {
            crate::emacs_core::builtins::DefaliasAction::SetFunction { symbol, definition } => {
                self.ctx.obarray.set_symbol_function_id(symbol, definition);
            }
            crate::emacs_core::builtins::DefaliasAction::CallHook {
                hook,
                symbol_value,
                definition,
            } => {
                let _ = self.call_function_with_roots(hook, &[symbol_value, definition])?;
            }
        }
        if let Some(symbol) = result.as_symbol_id() {
            let definition = self
                .ctx
                .obarray
                .symbol_function_id(symbol)
                .unwrap_or(Value::NIL);
            crate::emacs_core::interactive::sync_interactive_registry_for_symbol_definition(
                &mut self.ctx.interactive,
                symbol,
                definition,
            );
        }
        if let Some(docstring) = docstring {
            crate::emacs_core::builtins::symbols::builtin_put(
                &mut *self.ctx,
                vec![result, Value::symbol("function-documentation"), docstring],
            )?;
        }
        Ok(result)
    }

    fn builtin_defvaralias_shared(&mut self, args: &[Value]) -> EvalResult {
        let state_change =
            crate::emacs_core::builtins::symbols::defvaralias_impl(&mut *self.ctx, args.to_vec())?;
        self.run_variable_watchers_by_id(
            state_change.previous_target_id,
            &state_change.base_variable,
            &Value::NIL,
            "defvaralias",
        )?;
        crate::emacs_core::builtins::symbols::install_defvaralias_state(
            &mut *self.ctx,
            &state_change,
        );
        self.ctx.watchers.clear_watchers(state_change.alias_id);
        crate::emacs_core::builtins::symbols::builtin_put(
            &mut *self.ctx,
            vec![
                args[0],
                Value::symbol("variable-documentation"),
                state_change.docstring,
            ],
        )?;
        Ok(state_change.result)
    }

    fn builtin_makunbound_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("makunbound", args, 1)?;
        let symbol = crate::emacs_core::builtins::symbols::expect_symbol_id(&args[0])?;
        let resolved = crate::emacs_core::builtins::symbols::resolve_variable_alias_id_in_obarray(
            &self.ctx.obarray,
            symbol,
        )?;
        if self.ctx.obarray.is_constant_id(resolved) {
            return Err(signal("setting-constant", vec![args[0]]));
        }
        self.run_variable_watchers_by_id(resolved, &Value::NIL, &Value::NIL, "makunbound")?;
        crate::emacs_core::eval::makunbound_runtime_binding_in_state(
            &mut self.ctx.obarray,
            &mut self.ctx.buffers,
            &self.ctx.custom,
            &[],
            resolved,
        );
        Ok(args[0])
    }

    fn builtin_make_local_variable_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::custom::builtin_make_local_variable(&mut *self.ctx, args.to_vec())
    }

    fn builtin_local_variable_p_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::custom::builtin_local_variable_p(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_local_variables_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::custom::builtin_buffer_local_variables(&mut *self.ctx, args.to_vec())
    }

    fn builtin_kill_local_variable_shared(&mut self, args: &[Value]) -> EvalResult {
        let outcome =
            crate::emacs_core::custom::builtin_kill_local_variable_impl(&mut *self.ctx, args)?;
        if outcome.removed
            && let Some(buffer_id) = outcome.buffer_id
        {
            self.run_variable_watchers_by_id_with_where(
                outcome.resolved_id,
                &Value::NIL,
                &Value::NIL,
                "makunbound",
                &Value::make_buffer(buffer_id),
            )?;
        }
        Ok(outcome.result)
    }

    fn ensure_selected_frame_id(&mut self) -> FrameId {
        crate::emacs_core::window_cmds::ensure_selected_frame_id_in_state(
            &mut self.ctx.frames,
            &mut self.ctx.buffers,
        )
    }

    fn resolve_frame_id(&mut self, arg: Option<&Value>, predicate: &str) -> Result<FrameId, Flow> {
        let Some(val) = arg else {
            return Ok(self.ensure_selected_frame_id());
        };
        match val.kind() {
            ValueKind::Nil => Ok(self.ensure_selected_frame_id()),
            ValueKind::Fixnum(n) => {
                let fid = FrameId(n as u64);
                if self.ctx.frames.get(fid).is_some() {
                    Ok(fid)
                } else {
                    Err(signal(
                        "wrong-type-argument",
                        vec![Value::symbol(predicate), Value::fixnum(n)],
                    ))
                }
            }
            ValueKind::Veclike(VecLikeType::Frame) => {
                let id = val.as_frame_id().unwrap();
                let fid = FrameId(id);
                if self.ctx.frames.get(fid).is_some() {
                    Ok(fid)
                } else {
                    Err(signal(
                        "wrong-type-argument",
                        vec![Value::symbol(predicate), *val],
                    ))
                }
            }
            _ => Err(signal(
                "wrong-type-argument",
                vec![Value::symbol(predicate), *val],
            )),
        }
    }

    fn ensure_global_keymap(&mut self) -> Value {
        if let Some(value) = self.ctx.obarray.symbol_value("global-map").copied() {
            if crate::emacs_core::keymap::is_list_keymap(&value) {
                return value;
            }
        }
        let keymap = crate::emacs_core::keymap::make_list_keymap();
        self.ctx.obarray.set_symbol_value("global-map", keymap);
        keymap
    }

    fn builtin_mapcar_fast(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("mapcar", args, 2)?;
        let func = args[0];
        let sequence = args[1];
        self.with_vm_root_scope(|vm| {
            vm.push_dynamic_vm_root(func);
            vm.push_dynamic_vm_root(sequence);
            let len = crate::emacs_core::builtins::higher_order::map_sequence_length(sequence)?;
            let mut results = Vec::new();
            let map_result = vm.mapcar1_fast(len, Some(&mut results), sequence, |vm, item| {
                vm.with_vm_root_scope(|vm| {
                    vm.push_dynamic_vm_root(item);
                    vm.call_function1(func, item)
                })
            });

            match map_result {
                Ok(_) => Ok(Value::list(results)),
                Err(flow) => Err(flow),
            }
        })
    }

    fn mapcar1_fast<F>(
        &mut self,
        len: usize,
        values: Option<&mut Vec<Value>>,
        sequence: Value,
        mut call: F,
    ) -> Result<usize, Flow>
    where
        F: FnMut(&mut Self, Value) -> EvalResult,
    {
        let mut values = values;
        match sequence.kind() {
            ValueKind::Nil => Ok(0),
            ValueKind::Cons => {
                let mut cursor = sequence;
                let mut mapped = 0usize;
                for _ in 0..len {
                    if !cursor.is_cons() {
                        return Ok(mapped);
                    }
                    self.push_dynamic_vm_root(cursor);
                    let item = cursor.cons_car();
                    self.push_dynamic_vm_root(item);
                    let value = call(self, item)?;
                    if let Some(results) = values.as_deref_mut() {
                        self.push_dynamic_vm_root(value);
                        results.push(value);
                    }
                    mapped += 1;
                    cursor = cursor.cons_cdr();
                }
                Ok(mapped)
            }
            _ => {
                for index in 0..len {
                    let item = crate::emacs_core::builtins::higher_order::map_sequence_element(
                        sequence, index,
                    )?;
                    self.push_dynamic_vm_root(item);
                    let value = self.with_vm_root_scope(|vm| {
                        vm.push_dynamic_vm_root(item);
                        call(vm, item)
                    })?;
                    if let Some(results) = values.as_deref_mut() {
                        self.push_dynamic_vm_root(value);
                        results.push(value);
                    }
                }
                Ok(len)
            }
        }
    }

    fn builtin_mapc_fast(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("mapc", args, 2)?;
        let func = args[0];
        let sequence = args[1];
        self.with_vm_root_scope(|vm| {
            vm.push_dynamic_vm_root(func);
            vm.push_dynamic_vm_root(sequence);
            let len = crate::emacs_core::builtins::higher_order::map_sequence_length(sequence)?;
            vm.mapcar1_fast(len, None, sequence, |vm, item| {
                vm.with_vm_root_scope(|vm| {
                    vm.push_dynamic_vm_root(item);
                    vm.call_function1(func, item)
                })
            })?;
            Ok(sequence)
        })
    }

    fn builtin_mapcan_fast(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("mapcan", args, 2)?;
        let func = args[0];
        let sequence = args[1];
        self.with_vm_root_scope(|vm| {
            vm.push_dynamic_vm_root(func);
            vm.push_dynamic_vm_root(sequence);
            let len = crate::emacs_core::builtins::higher_order::map_sequence_length(sequence)?;
            let mut mapped = Vec::new();
            let map_result = vm.mapcar1_fast(len, Some(&mut mapped), sequence, |vm, item| {
                vm.with_vm_root_scope(|vm| {
                    vm.push_dynamic_vm_root(item);
                    vm.call_function1(func, item)
                })
            });

            match map_result {
                Ok(_) => crate::emacs_core::builtins::builtin_nconc(mapped),
                Err(flow) => Err(flow),
            }
        })
    }

    fn builtin_mapconcat_fast(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_range_args("mapconcat", args, 2, 3)?;
        let func = args[0];
        let sequence = args[1];
        let separator = args.get(2).copied().unwrap_or_else(|| Value::string(""));
        self.with_vm_root_scope(|vm| {
            vm.push_dynamic_vm_root(func);
            vm.push_dynamic_vm_root(sequence);
            vm.push_dynamic_vm_root(separator);
            let len = crate::emacs_core::builtins::higher_order::map_sequence_length(sequence)?;
            if len == 0 {
                return Ok(Value::string(""));
            }
            let mut parts = Vec::new();
            let map_result = vm.mapcar1_fast(len, Some(&mut parts), sequence, |vm, item| {
                vm.with_vm_root_scope(|vm| {
                    vm.push_dynamic_vm_root(item);
                    vm.call_function1(func, item)
                })
            });

            match map_result {
                Ok(mapped) => {
                    let mut concat_args = Vec::with_capacity(len * 2 - 1);
                    for index in 0..len {
                        if index > 0 {
                            concat_args.push(separator);
                        }
                        concat_args.push(if index < mapped {
                            parts[index]
                        } else {
                            crate::emacs_core::builtins::higher_order::gnu_mapconcat_unfilled_slot_value()
                        });
                    }
                    crate::emacs_core::builtins::builtin_concat(concat_args)
                }
                Err(flow) => Err(flow),
            }
        })
    }

    fn builtin_sort_fast(&mut self, args: &[Value]) -> EvalResult {
        let crate::emacs_core::builtins::higher_order::SortOptions {
            key_fn,
            lessp_fn,
            reverse,
            in_place,
        } = crate::emacs_core::builtins::higher_order::parse_sort_options(args)?;
        let sequence = args[0];
        self.with_vm_root_scope(|vm| {
            vm.push_dynamic_vm_root(sequence);
            vm.push_dynamic_vm_root(key_fn);
            vm.push_dynamic_vm_root(lessp_fn);
            match sequence.kind() {
                ValueKind::Nil => Ok(Value::NIL),
                ValueKind::Cons => {
                    let mut cons_cells = Vec::new();
                    let mut values = Vec::new();
                    let mut cursor = sequence;
                    loop {
                        match cursor.kind() {
                            ValueKind::Nil => break,
                            ValueKind::Cons => {
                                let value = cursor.cons_car();
                                vm.push_dynamic_vm_root(value);
                                values.push(value);
                                cons_cells.push(cursor);
                                cursor = cursor.cons_cdr();
                            }
                            _tail => {
                                return Err(signal(
                                    "wrong-type-argument",
                                    vec![Value::symbol("listp"), cursor],
                                ));
                            }
                        }
                    }
                    let mut sorted_values =
                        crate::emacs_core::builtins::higher_order::stable_sort_values_with(
                            vm, &values, key_fn, lessp_fn, reverse,
                        )?;
                    if in_place {
                        for (cell, value) in cons_cells.iter().zip(sorted_values.into_iter()) {
                            cell.set_car(value);
                        }
                        Ok(sequence)
                    } else {
                        Ok(Value::list(std::mem::take(&mut sorted_values)))
                    }
                }
                ValueKind::Veclike(VecLikeType::Vector)
                | ValueKind::Veclike(VecLikeType::Record) => {
                    let is_record =
                        matches!(sequence.kind(), ValueKind::Veclike(VecLikeType::Record));
                    let values = if is_record {
                        sequence.as_record_data().unwrap().clone()
                    } else {
                        sequence.as_vector_data().unwrap().clone()
                    };
                    for value in values.iter().copied() {
                        vm.push_dynamic_vm_root(value);
                    }
                    let sorted_values =
                        crate::emacs_core::builtins::higher_order::stable_sort_values_with(
                            vm, &values, key_fn, lessp_fn, reverse,
                        )?;

                    if in_place {
                        if is_record {
                            let _ = sequence.replace_record_data(sorted_values);
                        } else {
                            let _ = sequence.replace_vector_data(sorted_values);
                        }
                        Ok(sequence)
                    } else if is_record {
                        Ok(Value::make_record(sorted_values))
                    } else {
                        Ok(Value::vector(sorted_values))
                    }
                }
                _other => Err(signal(
                    "wrong-type-argument",
                    vec![Value::symbol("list-or-vector-p"), sequence],
                )),
            }
        })
    }

    fn builtin_frame_list_fast(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("frame-list", args, 0)?;
        let _ = self.ensure_selected_frame_id();
        let frames = self
            .ctx
            .frames
            .frame_list()
            .into_iter()
            .map(|frame_id| Value::make_frame(frame_id.0))
            .collect();
        Ok(Value::list(frames))
    }

    fn builtin_framep_fast(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("framep", args, 1)?;
        let id = match args[0].kind() {
            ValueKind::Veclike(VecLikeType::Frame) => args[0].as_frame_id().unwrap(),
            ValueKind::Fixnum(n) => n as u64,
            _ => return Ok(Value::NIL),
        };
        let Some(frame) = self.ctx.frames.get(FrameId(id)) else {
            return Ok(Value::NIL);
        };
        Ok(frame.parameter("window-system").unwrap_or(Value::T))
    }

    fn builtin_frame_parameter_fast(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::window_cmds::builtin_frame_parameter(self.ctx, args.to_vec())
    }

    fn builtin_fboundp_fast(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::symbols::builtin_fboundp(&mut *self.ctx, args.to_vec())
    }

    fn builtin_current_indentation_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::indent::builtin_current_indentation(&mut *self.ctx, args.to_vec())
    }

    fn builtin_indent_to_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::indent::builtin_indent_to(&mut *self.ctx, args.to_vec())
    }

    fn builtin_current_column_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::indent::builtin_current_column(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_string_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_buffer_string(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_substring_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_buffer_substring(&mut *self.ctx, args.to_vec())
    }

    fn builtin_field_beginning_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_field_beginning(&mut *self.ctx, args.to_vec())
    }

    fn builtin_field_end_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_field_end(&mut *self.ctx, args.to_vec())
    }

    fn builtin_field_string_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_field_string(&mut *self.ctx, args.to_vec())
    }

    fn builtin_field_string_no_properties_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_field_string_no_properties(
            &mut *self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_constrain_to_field_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_constrain_to_field(&mut *self.ctx, args.to_vec())
    }

    fn builtin_point_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_point(&mut *self.ctx, args.to_vec())
    }

    fn builtin_accept_process_output_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::process::builtin_accept_process_output(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_list_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_buffer_list(&mut *self.ctx, args.to_vec())
    }

    fn builtin_other_buffer_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_other_buffer(&mut *self.ctx, args.to_vec())
    }

    fn builtin_generate_new_buffer_name_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_generate_new_buffer_name(&mut *self.ctx, args.to_vec())
    }

    fn builtin_get_file_buffer_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_get_file_buffer(&mut *self.ctx, args.to_vec())
    }

    fn builtin_make_indirect_buffer_shared(&mut self, args: &[Value]) -> EvalResult {
        let plan = crate::emacs_core::builtins::prepare_make_indirect_buffer_in_manager(
            &mut self.ctx.buffers,
            args.to_vec(),
        )?;
        if plan.run_clone_hook {
            self.ctx.switch_current_buffer(plan.id)?;
            let hook_sym = crate::emacs_core::hook_runtime::hook_symbol_by_name(
                &self.ctx,
                "clone-indirect-buffer-hook",
            );
            let clone_result = crate::emacs_core::hook_runtime::run_named_hook(self, hook_sym, &[]);
            if let Some(saved_id) = plan.saved_current
                && self.ctx.buffers.get(saved_id).is_some()
            {
                self.ctx.restore_current_buffer_if_live(saved_id);
            }
            clone_result?;
        }
        if !self.ctx.buffers.buffer_hooks_inhibited(plan.id) {
            let hook_sym = crate::emacs_core::hook_runtime::hook_symbol_by_name(
                &self.ctx,
                "buffer-list-update-hook",
            );
            let _ = crate::emacs_core::hook_runtime::run_named_hook(self, hook_sym, &[])?;
        }
        Ok(Value::make_buffer(plan.id))
    }

    fn builtin_kill_buffer_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_kill_buffer(&mut *self.ctx, args.to_vec())
    }

    fn builtin_current_active_maps_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::keymaps::builtin_current_active_maps_impl(&mut *self.ctx, args)
    }

    fn builtin_current_minor_mode_maps_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::keymaps::builtin_current_minor_mode_maps_impl(&*self.ctx, args)
    }

    fn builtin_map_keymap_shared(&mut self, args: &[Value], include_parents: bool) -> EvalResult {
        let (function, mut keymap) = if include_parents {
            builtins::expect_min_args("map-keymap", args, 2)?;
            builtins::expect_max_args("map-keymap", args, 3)?;
            (
                args[0],
                crate::emacs_core::keymap::get_keymap_in_runtime(
                    &mut *self.ctx,
                    &args[1],
                    true,
                    true,
                )?,
            )
        } else {
            builtins::expect_args("map-keymap-internal", args, 2)?;
            (
                args[0],
                crate::emacs_core::keymap::get_keymap_in_runtime(
                    &mut *self.ctx,
                    &args[1],
                    true,
                    true,
                )?,
            )
        };

        loop {
            let plan = crate::emacs_core::builtins::keymaps::plan_keymap_iteration(keymap);
            let parent = plan.parent;
            let bindings = plan.bindings;
            for (event, binding) in &bindings {
                let call_args = [*event, *binding];
                let _ = self.call_function_with_roots(function, &call_args)?;
            }

            if !include_parents {
                return Ok(parent);
            }
            if parent.is_nil() || !crate::emacs_core::keymap::is_list_keymap(&parent) {
                return Ok(Value::NIL);
            }
            keymap = parent;
        }
    }

    fn builtin_map_char_table_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("map-char-table", args, 2)?;
        let function = args[0];
        crate::emacs_core::chartable::for_each_char_table_mapping(&args[1], |key, value| {
            let call_args = [key, value];
            let _ = self.call_function_with_roots(function, &call_args)?;
            Ok(())
        })?;
        Ok(Value::NIL)
    }

    fn builtin_call_last_kbd_macro_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::kmacro::builtin_call_last_kbd_macro(&mut *self.ctx, args.to_vec())
    }

    fn builtin_execute_kbd_macro_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::kmacro::builtin_execute_kbd_macro(&mut *self.ctx, args.to_vec())
    }

    fn builtin_command_remapping_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::interactive::builtin_command_remapping_impl(&*self.ctx, args.to_vec())
    }

    fn builtin_key_binding_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::interactive::builtin_key_binding_impl(&mut *self.ctx, args.to_vec())
    }

    fn builtin_local_key_binding_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::interactive::builtin_local_key_binding_impl(&*self.ctx, args.to_vec())
    }

    fn builtin_minor_mode_key_binding_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::interactive::builtin_minor_mode_key_binding_impl(
            &*self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_set_buffer_multibyte_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_set_buffer_multibyte(&mut *self.ctx, args.to_vec())
    }

    fn builtin_insert_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_insert(&mut *self.ctx, args.to_vec())
    }

    fn builtin_barf_if_buffer_read_only_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_barf_if_buffer_read_only_impl(
            &*self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_insert_and_inherit_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_insert_and_inherit(&mut *self.ctx, args.to_vec())
    }

    fn builtin_insert_before_markers_and_inherit_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_insert_before_markers_and_inherit(
            &mut *self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_point_min_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_point_min(&mut *self.ctx, args.to_vec())
    }

    fn builtin_point_max_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_point_max(&mut *self.ctx, args.to_vec())
    }

    fn builtin_goto_char_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_goto_char(&mut *self.ctx, args.to_vec())
    }

    fn builtin_char_after_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_char_after(&mut *self.ctx, args.to_vec())
    }

    fn builtin_char_before_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_char_before(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_size_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_buffer_size(&mut *self.ctx, args.to_vec())
    }

    fn builtin_byte_to_position_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_byte_to_position(&mut *self.ctx, args.to_vec())
    }

    fn builtin_position_bytes_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_position_bytes(&mut *self.ctx, args.to_vec())
    }

    fn builtin_get_byte_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_get_byte(&mut *self.ctx, args.to_vec())
    }

    fn builtin_narrow_to_region_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_narrow_to_region(&mut *self.ctx, args.to_vec())
    }

    fn builtin_widen_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_widen(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_modified_p_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_buffer_modified_p(&mut *self.ctx, args.to_vec())
    }

    fn builtin_set_buffer_modified_p_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_set_buffer_modified_p(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_modified_tick_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_buffer_modified_tick(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_chars_modified_tick_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_buffer_chars_modified_tick(
            &mut *self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_insert_char_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_insert_char(&mut *self.ctx, args.to_vec())
    }

    fn builtin_insert_byte_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_insert_byte(&mut *self.ctx, args.to_vec())
    }

    fn builtin_subst_char_in_region_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_subst_char_in_region(&mut *self.ctx, args.to_vec())
    }

    fn builtin_bobp_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::navigation::builtin_bobp(&mut *self.ctx, args.to_vec())
    }

    fn builtin_eobp_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::navigation::builtin_eobp(&mut *self.ctx, args.to_vec())
    }

    fn builtin_bolp_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::navigation::builtin_bolp(&mut *self.ctx, args.to_vec())
    }

    fn builtin_eolp_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::navigation::builtin_eolp(&mut *self.ctx, args.to_vec())
    }

    fn builtin_line_beginning_position_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::navigation::builtin_line_beginning_position(
            &mut *self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_line_end_position_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::navigation::builtin_line_end_position(&mut *self.ctx, args.to_vec())
    }

    fn builtin_insert_before_markers_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_insert_before_markers(&mut *self.ctx, args.to_vec())
    }

    fn builtin_insert_buffer_substring_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_insert_buffer_substring(&mut *self.ctx, args.to_vec())
    }

    fn builtin_replace_region_contents_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_replace_region_contents(&mut *self.ctx, args.to_vec())
    }

    fn builtin_delete_char_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::editfns::builtin_delete_char(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_substring_no_properties_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::editfns::builtin_buffer_substring_no_properties(
            &*self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_following_char_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::editfns::builtin_following_char(&*self.ctx, args.to_vec())
    }

    fn builtin_preceding_char_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::editfns::builtin_preceding_char(&*self.ctx, args.to_vec())
    }

    fn builtin_delete_region_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::editfns::builtin_delete_region(&mut *self.ctx, args.to_vec())
    }

    fn builtin_compare_buffer_substrings_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_compare_buffer_substrings_with_case_fold(
            self.case_fold_search_enabled(),
            &self.ctx.buffers,
            args.to_vec(),
        )
    }

    fn builtin_delete_field_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_delete_field(&mut *self.ctx, args.to_vec())
    }

    fn builtin_delete_and_extract_region_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::editfns::builtin_delete_and_extract_region(&mut *self.ctx, args.to_vec())
    }

    fn builtin_erase_buffer_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::editfns::builtin_erase_buffer(&mut *self.ctx, args.to_vec())
    }

    fn builtin_undo_boundary_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::undo::builtin_undo_boundary(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_enable_undo_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_buffer_enable_undo(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_disable_undo_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_buffer_disable_undo(&mut *self.ctx, args.to_vec())
    }

    fn builtin_kill_all_local_variables_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_kill_all_local_variables(&mut *self.ctx, args.to_vec())
    }

    fn builtin_buffer_local_value_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_buffer_local_value(&mut *self.ctx, args.to_vec())
    }

    fn builtin_local_variable_if_set_p_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::symbols::builtin_local_variable_if_set_p(
            &mut *self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_variable_binding_locus_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::symbols::builtin_variable_binding_locus(
            &mut *self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_move_to_column_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::indent::builtin_move_to_column(&mut *self.ctx, args.to_vec())
    }

    fn case_fold_search_enabled(&mut self) -> bool {
        self.lookup_var("case-fold-search")
            .map(|value| !value.is_nil())
            .unwrap_or(true)
    }

    fn builtin_search_forward_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_search_forward_with_state(
            self.case_fold_search_enabled(),
            &mut self.ctx.buffers,
            &mut self.ctx.match_data,
            args,
        )
    }

    fn builtin_search_backward_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_search_backward_with_state(
            self.case_fold_search_enabled(),
            &mut self.ctx.buffers,
            &mut self.ctx.match_data,
            args,
        )
    }

    fn builtin_re_search_forward_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_re_search_forward_with_state(
            self.case_fold_search_enabled(),
            &mut self.ctx.buffers,
            &mut self.ctx.match_data,
            args,
        )
    }

    fn builtin_re_search_backward_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_re_search_backward_with_state(
            self.case_fold_search_enabled(),
            &mut self.ctx.buffers,
            &mut self.ctx.match_data,
            args,
        )
    }

    fn builtin_search_forward_regexp_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_search_forward_regexp_with_state(
            self.case_fold_search_enabled(),
            &mut self.ctx.buffers,
            &mut self.ctx.match_data,
            args,
        )
    }

    fn builtin_search_backward_regexp_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_search_backward_regexp_with_state(
            self.case_fold_search_enabled(),
            &mut self.ctx.buffers,
            &mut self.ctx.match_data,
            args,
        )
    }

    fn builtin_looking_at_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_looking_at_with_state(
            self.case_fold_search_enabled(),
            &self.ctx.buffers,
            &mut self.ctx.match_data,
            args,
        )
    }

    fn builtin_looking_at_p_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_looking_at_p_with_state(
            self.case_fold_search_enabled(),
            &self.ctx.buffers,
            args,
        )
    }

    fn builtin_posix_looking_at_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_posix_looking_at_with_state(
            self.case_fold_search_enabled(),
            &self.ctx.buffers,
            &mut self.ctx.match_data,
            args,
        )
    }

    fn builtin_posix_string_match_shared(&mut self, args: &[Value]) -> EvalResult {
        let case_fold = self.case_fold_search_enabled();
        let case_translation = if case_fold {
            let canon = crate::emacs_core::casetab::current_case_canon_table(&mut self.ctx)?;
            Some(crate::emacs_core::regex_emacs::CaseTranslation::from_char_table(canon))
        } else {
            None
        };
        let current_buffer = self.ctx.buffers.current_buffer();
        let syntax_table = current_buffer.map(crate::emacs_core::syntax::SyntaxTable::for_buffer);
        let category_table =
            Some(crate::emacs_core::category::active_category_table_for_buffer(current_buffer)?);
        crate::emacs_core::builtins::search::builtin_posix_string_match_with_state(
            case_fold,
            case_translation,
            syntax_table.as_ref(),
            category_table,
            &mut self.ctx.match_data,
            args,
        )
    }

    fn builtin_match_data_translate_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_match_data_translate_with_state(
            &mut self.ctx.match_data,
            args,
        )
    }

    fn builtin_replace_match_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::search::builtin_replace_match(&mut *self.ctx, args.to_vec())
    }

    fn builtin_find_charset_region_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::charset::builtin_find_charset_region(&mut *self.ctx, args.to_vec())
    }

    fn builtin_charset_after_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::charset::builtin_charset_after(&mut *self.ctx, args.to_vec())
    }

    fn builtin_compose_region_internal_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::composite::builtin_compose_region_internal(&mut *self.ctx, args.to_vec())
    }

    fn builtin_interactive_form_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("interactive-form", args, 1)?;
        let mut target = args[0];
        loop {
            match crate::emacs_core::builtins::symbols::plan_interactive_form_in_state(
                &self.ctx.obarray,
                &self.ctx.interactive,
                target,
            )? {
                crate::emacs_core::builtins::symbols::InteractiveFormPlan::Return(value) => {
                    return Ok(value);
                }
                crate::emacs_core::builtins::symbols::InteractiveFormPlan::Autoload {
                    fundef,
                    funname,
                } => {
                    let mut load_args = vec![fundef];
                    if !funname.is_nil() {
                        load_args.push(funname);
                    }
                    target = self.with_vm_root_scope(|vm| {
                        vm.push_dynamic_vm_root(target);
                        for value in args.iter().copied() {
                            vm.push_dynamic_vm_root(value);
                        }
                        for value in load_args.iter().copied() {
                            vm.push_dynamic_vm_root(value);
                        }
                        crate::emacs_core::autoload::builtin_autoload_do_load_in_vm_runtime(
                            &mut vm.ctx,
                            &load_args,
                        )
                    })?;
                }
            }
        }
    }

    fn builtin_skip_chars_forward_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::navigation::builtin_skip_chars_forward(&mut *self.ctx, args.to_vec())
    }

    fn builtin_skip_chars_backward_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::navigation::builtin_skip_chars_backward(&mut *self.ctx, args.to_vec())
    }

    fn builtin_scan_lists_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::syntax::builtin_scan_lists(&mut *self.ctx, args.to_vec())
    }

    fn builtin_scan_sexps_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::syntax::builtin_scan_sexps(&mut *self.ctx, args.to_vec())
    }

    fn visible_variable_value_or_nil(&self, name: &str) -> Value {
        let name_id = intern(name);
        if let Some(value) = self.ctx.lexenv_lookup_cached_in(self.ctx.lexenv, name_id) {
            return value;
        }
        // specbind writes directly to obarray, so no dynamic stack lookup needed.
        if let Some(buffer) = self.ctx.buffers.current_buffer()
            && let Some(binding) = buffer.get_buffer_local_binding(name)
        {
            return binding.as_value().unwrap_or(Value::NIL);
        }
        if let Some(value) = self.ctx.obarray.symbol_value(name).copied() {
            return value;
        }
        if name == "nil" {
            return Value::NIL;
        }
        if name == "t" {
            return Value::T;
        }
        Value::NIL
    }

    fn call_function(&mut self, func_val: Value, args: impl Into<LispArgVec>) -> EvalResult {
        let args = args.into();
        let bt_count = self.ctx.specpdl.len();
        self.ctx.push_backtrace_frame(func_val, &args);
        let result = self.call_function_untraced_owned(func_val, args);
        let result = self.ctx.dispatch_signal_result_if_needed(result);
        self.ctx.unbind_to_with_result(bt_count, result)
    }

    /// Read a (dynamic/global) variable for JIT code with the interpreter's
    /// `Op::VarRef` semantics — delegates to the same `fast_path_var_ref`
    /// (Plainval fast path, buffer-locals, redirects; signals `void-variable`).
    #[cfg(feature = "jit")]
    pub(crate) fn varref_for_jit(&mut self, name_id: SymId) -> EvalResult {
        self.fast_path_var_ref(name_id)
    }

    /// Assign a (dynamic/global) variable for JIT code with the interpreter's
    /// `Op::VarSet` semantics — delegates to the same `assign_var_id` (may run
    /// variable watchers, i.e. arbitrary lisp; may signal).
    #[cfg(feature = "jit")]
    pub(crate) fn varset_for_jit(&mut self, name_id: SymId, value: Value) -> Result<(), Flow> {
        self.assign_var_id(name_id, value)
    }

    /// One bytecode-level `apply` with the interpreter's `Op::Apply` semantics:
    /// spread the last argument as a list, writeback detection + after-call
    /// writeback, and the plain traced `call_function` path (`Op::Apply` has no
    /// nesting-depth guard — mirror that exactly). Used by the JIT apply shim;
    /// keep in sync with the `Op::Apply` arm of `run_loop`. The caller polls
    /// `maybe_quit` first and roots `func_val` + `raw_args` (the spread values
    /// stay reachable through the rooted list).
    #[cfg(feature = "jit")]
    /// `Op::Aset` for JIT code — the interpreter arm minus the bc-frame
    /// rooting (the JIT shim scratch-roots the operands; nested calls root
    /// their own frames): override-aware named dispatch when `aset`'s
    /// function cell was redefined, the shared `builtin_aset` otherwise, then
    /// the unconditional string-writeback pass.
    pub(crate) fn aset_for_jit(
        &mut self,
        vec_val: Value,
        idx_val: Value,
        val: Value,
    ) -> EvalResult {
        let mut call_args = LispArgVec::new();
        call_args.push(vec_val);
        call_args.push(idx_val);
        call_args.push(val);
        let id = Self::builtin_name_id("aset");
        let result = if self.named_builtin_fast_path_allowed_id(id) {
            builtins::builtin_aset(call_args.clone().into_vec())?
        } else {
            let func_val = Value::from_sym_id(id);
            self.call_function(func_val, call_args.clone())?
        };
        let root_scope = self.ctx.save_vm_roots();
        self.push_dynamic_vm_root(result);
        for value in call_args.iter().copied() {
            self.push_dynamic_vm_root(value);
        }
        self.maybe_writeback_mutating_first_arg("aset", None, &call_args, &result);
        self.ctx.restore_vm_roots(root_scope);
        Ok(result)
    }

    /// `Op::CallBuiltin` for JIT code — the interpreter arm minus the
    /// bc-frame rooting: named fast path when the symbol's function cell is
    /// unmodified, full `call_function` (override/advice) otherwise, the
    /// mutating-first-arg string writeback, and the arm's trailing quit poll.
    pub(crate) fn callbuiltin_for_jit(&mut self, name_id: SymId, args: LispArgVec) -> EvalResult {
        let name = resolve_sym(name_id);
        let writeback_args = (args.first().is_some_and(|value| value.is_string())
            && Self::mutates_first_arg_name(name))
        .then(|| args.clone());
        let result = if self.named_builtin_fast_path_allowed_id(name_id) {
            self.dispatch_vm_builtin(name, args)?
        } else {
            let func_val = Value::from_sym_id(name_id);
            self.call_function(func_val, args)?
        };
        if let Some(writeback_args) = writeback_args.as_ref() {
            let root_scope = self.ctx.save_vm_roots();
            self.push_dynamic_vm_root(result);
            for value in writeback_args.iter().copied() {
                self.push_dynamic_vm_root(value);
            }
            self.maybe_writeback_mutating_first_arg(name, None, writeback_args, &result);
            self.ctx.restore_vm_roots(root_scope);
        }
        self.ctx.maybe_quit()?;
        Ok(result)
    }

    /// `Op::CallBuiltinSym` for JIT code — ALWAYS the direct named dispatch,
    /// never the function cell (GNU parity: bytecode-inlined primitives
    /// bypass advice; see the interpreter arm's comment), plus writeback and
    /// the trailing quit poll.
    pub(crate) fn callbuiltinsym_for_jit(&mut self, sym: SymId, args: LispArgVec) -> EvalResult {
        let name = resolve_sym(sym);
        let writeback_args = (args.first().is_some_and(|value| value.is_string())
            && Self::mutates_first_arg_name(name))
        .then(|| args.clone());
        let result = self.dispatch_vm_builtin(name, args)?;
        if let Some(writeback_args) = writeback_args.as_ref() {
            let root_scope = self.ctx.save_vm_roots();
            self.push_dynamic_vm_root(result);
            for value in writeback_args.iter().copied() {
                self.push_dynamic_vm_root(value);
            }
            self.maybe_writeback_mutating_first_arg(name, None, writeback_args, &result);
            self.ctx.restore_vm_roots(root_scope);
        }
        self.ctx.maybe_quit()?;
        Ok(result)
    }

    pub(crate) fn apply_for_jit(
        &mut self,
        func_val: Value,
        mut raw_args: LispArgVec,
    ) -> EvalResult {
        if raw_args.is_empty() {
            return self.call_function(func_val, LispArgVec::new());
        }
        // Spread the last argument.
        if let Some(last) = raw_args.pop() {
            let spread = list_to_vec(&last).unwrap_or_default();
            raw_args.extend(spread);
        }
        let args = raw_args;
        let writeback_names = if args.first().is_some_and(|value| value.is_string()) {
            self.writeback_mutating_callable_names(&func_val)
        } else {
            None
        };
        let writeback_args = writeback_names.as_ref().map(|_| args.clone());
        let result = self.call_function(func_val, args)?;
        if let (Some((called_name, alias_target)), Some(writeback_args)) =
            (writeback_names.as_ref(), writeback_args.as_ref())
        {
            let root_scope = self.ctx.save_vm_roots();
            self.push_dynamic_vm_root(result);
            for value in writeback_args.iter().copied() {
                self.push_dynamic_vm_root(value);
            }
            self.maybe_writeback_mutating_first_arg(
                called_name,
                *alias_target,
                writeback_args,
                &result,
            );
            self.ctx.restore_vm_roots(root_scope);
        }
        Ok(result)
    }

    /// One bytecode-level function call with the interpreter's `Op::Call`
    /// semantics: mutating-string-arg writeback detection, the lisp-nesting
    /// depth guard, the traced `call_function` path, and the after-call
    /// writeback. Used by the JIT call shim (`jit::compile::neovm_jit_call`) so
    /// compiled code re-enters the runtime through exactly the interpreter's
    /// call path — keep in sync with the `Op::Call` arm of `run_loop` (which
    /// keeps an in-place stack-args fast path for the no-writeback case).
    ///
    /// The caller polls `maybe_quit` first (GNU `bytecode.c:Bcall` order).
    #[cfg(feature = "jit")]
    pub(crate) fn call_for_jit(&mut self, func_val: Value, args: LispArgVec) -> EvalResult {
        let writeback_names = if args.first().is_some_and(|value| value.is_string()) {
            self.writeback_mutating_callable_names(&func_val)
        } else {
            None
        };
        let writeback_args = writeback_names.as_ref().map(|_| args.clone());
        let result = self.with_bytecode_call_depth(|vm| {
            // Fast subr path: the JIT routes subr (primitive) calls — 75.4% of
            // real-elisp calls — through the interpreter's exact direct-subr
            // dispatch (`try_call_builtin_subr_from_stack_args`), skipping
            // call_function's kind resolution + wrapper. It reads its args from
            // the GC-traced `bc_buf`, so push the value args there first (which
            // also roots them across the subr call, which may GC), try it,
            // restore. Falls back to the full call_function for non-subr callees
            // (bytecode/closures/overridden cells). Same depth guard + the
            // writeback wrapper below — behaviour-preserving, faster dispatch.
            let args_start = vm.ctx.bc_buf.len();
            for &a in args.iter() {
                vm.ctx.bc_buf.push(a);
            }
            let nargs = args.len();
            match vm.try_call_builtin_subr_from_stack_args(func_val, args_start, nargs) {
                Some(result) => {
                    vm.ctx.bc_buf.truncate(args_start);
                    result
                }
                None => {
                    vm.ctx.bc_buf.truncate(args_start);
                    vm.call_function(func_val, args)
                }
            }
        })?;
        if let (Some((called_name, alias_target)), Some(writeback_args)) =
            (writeback_names.as_ref(), writeback_args.as_ref())
        {
            let root_scope = self.ctx.save_vm_roots();
            self.push_dynamic_vm_root(result);
            for value in writeback_args.iter().copied() {
                self.push_dynamic_vm_root(value);
            }
            self.maybe_writeback_mutating_first_arg(
                called_name,
                *alias_target,
                writeback_args,
                &result,
            );
            self.ctx.restore_vm_roots(root_scope);
        }
        Ok(result)
    }

    /// V3 + native-to-native speculated direct call: the caller's spec site is
    /// armed, so `callee` is the compile-time bytecode object the symbol still
    /// names, and `args_ptr` addresses `nargs` pre-marshaled argument words (the
    /// caller's native call-args slot). Resolve and cache the callee's compiled
    /// leaf in `leaf_slot`, then run it DIRECTLY under the recursion-depth
    /// guard — skipping the `funcall_general` dispatch and the compiled-cache
    /// hash lookup that `call_for_jit` would pay.
    ///
    /// When the callee is a pure pass-through for this argument count (simple
    /// fixed arity, no `&optional` nil-pad / `&rest` list), the args go
    /// STRAIGHT to the callee's native entry — no `LispArgVec`, no per-arg
    /// scratch rooting, no re-marshal (the per-call cost that dominates
    /// call-heavy compiled code). Otherwise the args are marshaled and rooted
    /// (still skipping dispatch + hash lookup). Returns `None` when the callee
    /// can't be fast-pathed (body `NotCompilable`, or an arity mismatch the
    /// strict path must signal), leaving the shim to fall back to
    /// `call_for_jit`.
    ///
    /// The recursion-depth guard is applied exactly as `call_for_jit` applies
    /// it (one increment per call) so deeply recursive compiled functions
    /// signal `max-lisp-eval-depth` instead of overflowing the native stack.
    /// The cached leaf handle is sound because the per-thread `COMPILED` cache
    /// never evicts. The native pass-through needs no arg rooting: the caller's
    /// `maybe_quit` already returned Ok (which does not collect) and nothing
    /// allocates on a lisp heap before the callee's entry reads its args.
    ///
    /// SAFETY: `args_ptr` addresses `nargs` valid tagged words (the caller's
    /// call-args slot, populated immediately before the spec shim was called).
    pub(crate) fn call_armed_callee_native(
        &mut self,
        callee: Value,
        leaf_slot: &core::sync::atomic::AtomicU64,
        args_ptr: *const i64,
        nargs: usize,
    ) -> Option<Result<Value, Flow>> {
        use core::sync::atomic::Ordering;
        let bc = callee.get_bytecode_data()?;
        let mut ptr = leaf_slot.load(Ordering::Relaxed)
            as *const crate::emacs_core::jit::compile::CompiledLeaf;
        if ptr.is_null() {
            let ctx_ptr = core::ptr::from_mut(&mut *self.ctx);
            ptr = crate::emacs_core::jit::cache::resolve_compiled_leaf_ptr(ctx_ptr, bc)?;
            leaf_slot.store(ptr as usize as u64, Ordering::Relaxed);
        }
        // SAFETY: the COMPILED cache never evicts; `ptr` names a cache-held
        // leaf, valid for this thread's lifetime.
        let leaf = unsafe { &*ptr };
        if !leaf.accepts(nargs) {
            // Wrong arg count: defer to the strict path, which signals
            // wrong-number-of-arguments exactly as the interpreter would.
            return None;
        }
        let pure = leaf.is_pure_passthrough(nargs);
        // Debug-build evidence that the fast path actually fires (vs silently
        // falling back to call_for_jit on every call).
        #[cfg(debug_assertions)]
        crate::emacs_core::jit::compile::SPEC_FAST_CALL_COUNT.fetch_add(1, Ordering::Relaxed);
        let res = self.with_bytecode_call_depth(|vm| {
            let ctx_ptr = core::ptr::from_mut(&mut *vm.ctx);
            let ran = if pure {
                // NATIVE-TO-NATIVE: pass the caller's call-args slot straight
                // through (no LispArgVec, no rooting, no re-marshal).
                crate::emacs_core::jit::cache::run_resolved_leaf_native(
                    ctx_ptr, bc, callee, leaf, args_ptr,
                )?
            } else {
                // Marshaled (callee has &optional/&rest): build + root args.
                // The spec shim's outer scratch-root scope bounds these pushes.
                let mut args = LispArgVec::new();
                for i in 0..nargs {
                    // SAFETY: args_ptr addresses `nargs` valid words.
                    let v = Value::from_bits(unsafe { *args_ptr.add(i) } as usize);
                    crate::emacs_core::eval::push_scratch_gc_root(v);
                    args.push(v);
                }
                crate::emacs_core::jit::cache::run_resolved_leaf(ctx_ptr, bc, callee, leaf, &args)?
            };
            match ran {
                Some(bits) => Ok(Value::from_bits(bits)),
                None => {
                    // Plain Deopt only arises with a null ctx (not here);
                    // defensively run the callee on the interpreter.
                    let mut args = Vec::with_capacity(nargs);
                    for i in 0..nargs {
                        // SAFETY: args_ptr addresses `nargs` valid words.
                        args.push(Value::from_bits(unsafe { *args_ptr.add(i) } as usize));
                    }
                    vm.execute_with_func_value(bc, args, callee)
                }
            }
        });
        Some(res)
    }

    fn call_function_from_stack_args(
        &mut self,
        func_val: Value,
        args_start: usize,
        nargs: usize,
        allow_direct_builtin_subr: bool,
    ) -> EvalResult {
        let bt_count = self.ctx.specpdl.len();
        if allow_direct_builtin_subr
            && let Some(result) =
                self.try_call_builtin_subr_from_stack_args(func_val, args_start, nargs)
        {
            return result;
        }
        let args = LispArgVec::from_slice(&self.ctx.bc_buf[args_start..args_start + nargs]);
        self.ctx.push_backtrace_frame(func_val, &args);
        let result = self.call_function_untraced_owned(func_val, args);
        let result = self.ctx.dispatch_signal_result_if_needed(result);
        self.ctx.unbind_to_with_result(bt_count, result)
    }

    fn call_function_untraced_owned(&mut self, func_val: Value, args: LispArgVec) -> EvalResult {
        let result = match func_val.kind() {
            // Fast path: stay in VM for bytecoded calls.
            // Matches GNU Emacs's CLOSUREP → goto setup_frame in bytecode.c.
            ValueKind::Veclike(VecLikeType::ByteCode) => {
                let bc_data = func_val.get_bytecode_data().unwrap();
                self.execute_with_func_value(bc_data, args, func_val)
            }
            // Everything else: shared dispatch via funcall_general on Context.
            // Matches GNU Emacs where exec_byte_code delegates to funcall_general.
            _ => self.ctx.funcall_general_untraced(func_val, args),
        };
        result
    }

    fn try_call_builtin_subr_from_stack_args(
        &mut self,
        func_val: Value,
        args_start: usize,
        nargs: usize,
    ) -> Option<EvalResult> {
        let (sym_id, entry, callee) = self.direct_subr_call_target(func_val)?;
        let bt_count = self.ctx.specpdl.len();
        self.ctx
            .push_backtrace_frame_from_bc_stack(func_val, args_start, nargs);
        let result = if nargs < entry.min_args as usize
            || entry.max_args.is_some_and(|max| nargs > max as usize)
        {
            Err(signal(
                "wrong-number-of-arguments",
                vec![callee.wrong_arity_value(), Value::fixnum(nargs as i64)],
            ))
        } else {
            if let Some(value) =
                self.try_dispatch_builtin_subr_fast_value_from_stack_args(sym_id, args_start, nargs)
            {
                self.ctx.pop_fast_bytecode_backtrace_frame(bt_count);
                return Some(Ok(value));
            }
            match entry.function {
                Some(function) => self
                    .dispatch_builtin_subr_from_stack_args_unchecked(function, args_start, nargs)
                    .unwrap_or_else(|| {
                        Err(signal("void-function", vec![Value::from_sym_id(sym_id)]))
                    }),
                None => Err(signal("void-function", vec![Value::from_sym_id(sym_id)])),
            }
        };
        let result = self.ctx.dispatch_signal_result_if_needed(result);
        Some(
            self.ctx
                .pop_bytecode_backtrace_frame_with_result(bt_count, result),
        )
    }

    #[inline]
    fn try_dispatch_builtin_subr_fast_value_from_stack_args(
        &self,
        sym_id: SymId,
        args_start: usize,
        nargs: usize,
    ) -> Option<Value> {
        if sym_id == plus_sym_id() {
            return self.try_fast_fixnum_add_value_from_stack_args(args_start, nargs);
        }
        if sym_id == logand_sym_id() {
            return self.try_fast_fixnum_logand_value_from_stack_args(args_start, nargs);
        }
        if sym_id == logior_sym_id() {
            return self.try_fast_fixnum_logior_value_from_stack_args(args_start, nargs);
        }
        if sym_id == logxor_sym_id() {
            return self.try_fast_fixnum_logxor_value_from_stack_args(args_start, nargs);
        }
        None
    }

    #[inline]
    fn try_fast_fixnum_add_value_from_stack_args(
        &self,
        args_start: usize,
        nargs: usize,
    ) -> Option<Value> {
        let args = &self.ctx.bc_buf;
        match nargs {
            0 => return Some(Value::fixnum(0)),
            1 => {
                let a = unsafe { args.get_unchecked(args_start) }.as_fixnum()?;
                return Some(Value::make_int(a));
            }
            2 => {
                let a = unsafe { args.get_unchecked(args_start) }.as_fixnum()?;
                let b = unsafe { args.get_unchecked(args_start + 1) }.as_fixnum()?;
                return Some(Value::make_int(a.checked_add(b)?));
            }
            3 => {
                let a = unsafe { args.get_unchecked(args_start) }.as_fixnum()?;
                let b = unsafe { args.get_unchecked(args_start + 1) }.as_fixnum()?;
                let c = unsafe { args.get_unchecked(args_start + 2) }.as_fixnum()?;
                let sum = a.checked_add(b)?;
                return Some(Value::make_int(sum.checked_add(c)?));
            }
            4 => {
                let a = unsafe { args.get_unchecked(args_start) }.as_fixnum()?;
                let b = unsafe { args.get_unchecked(args_start + 1) }.as_fixnum()?;
                let c = unsafe { args.get_unchecked(args_start + 2) }.as_fixnum()?;
                let d = unsafe { args.get_unchecked(args_start + 3) }.as_fixnum()?;
                let sum = a.checked_add(b)?;
                let sum = sum.checked_add(c)?;
                return Some(Value::make_int(sum.checked_add(d)?));
            }
            _ => {}
        }
        let mut acc = 0i64;
        for idx in 0..nargs {
            let next = unsafe { args.get_unchecked(args_start + idx) }.as_fixnum()?;
            acc = acc.checked_add(next)?;
        }
        Some(Value::make_int(acc))
    }

    #[inline]
    fn try_fast_fixnum_logand_value_from_stack_args(
        &self,
        args_start: usize,
        nargs: usize,
    ) -> Option<Value> {
        let args = &self.ctx.bc_buf;
        let mut acc = if nargs == 0 {
            -1
        } else {
            unsafe { args.get_unchecked(args_start) }.as_fixnum()?
        };
        for idx in 1..nargs {
            let next = unsafe { args.get_unchecked(args_start + idx) }.as_fixnum()?;
            acc &= next;
        }
        Some(Value::make_int(acc))
    }

    #[inline]
    fn try_fast_fixnum_logior_value_from_stack_args(
        &self,
        args_start: usize,
        nargs: usize,
    ) -> Option<Value> {
        let args = &self.ctx.bc_buf;
        let mut acc = if nargs == 0 {
            0
        } else {
            unsafe { args.get_unchecked(args_start) }.as_fixnum()?
        };
        for idx in 1..nargs {
            let next = unsafe { args.get_unchecked(args_start + idx) }.as_fixnum()?;
            acc |= next;
        }
        Some(Value::make_int(acc))
    }

    #[inline]
    fn try_fast_fixnum_logxor_value_from_stack_args(
        &self,
        args_start: usize,
        nargs: usize,
    ) -> Option<Value> {
        let args = &self.ctx.bc_buf;
        let mut acc = if nargs == 0 {
            0
        } else {
            unsafe { args.get_unchecked(args_start) }.as_fixnum()?
        };
        for idx in 1..nargs {
            let next = unsafe { args.get_unchecked(args_start + idx) }.as_fixnum()?;
            acc ^= next;
        }
        Some(Value::make_int(acc))
    }

    fn dispatch_builtin_subr_from_stack_args_unchecked(
        &mut self,
        func: SubrFn,
        args_start: usize,
        nargs: usize,
    ) -> Option<EvalResult> {
        let args = &self.ctx.bc_buf;
        macro_rules! stack_arg {
            ($idx:expr) => {{
                let idx = $idx;
                if idx < nargs {
                    unsafe { *args.get_unchecked(args_start + idx) }
                } else {
                    Value::NIL
                }
            }};
        }
        match func {
            SubrFn::A0(func) => Some(func(self.ctx)),
            SubrFn::A1(func) => {
                let arg0 = stack_arg!(0);
                Some(func(self.ctx, arg0))
            }
            SubrFn::A2(func) => {
                let arg0 = stack_arg!(0);
                let arg1 = stack_arg!(1);
                Some(func(self.ctx, arg0, arg1))
            }
            SubrFn::A3(func) => {
                let arg0 = stack_arg!(0);
                let arg1 = stack_arg!(1);
                let arg2 = stack_arg!(2);
                Some(func(self.ctx, arg0, arg1, arg2))
            }
            SubrFn::A4(func) => {
                let arg0 = stack_arg!(0);
                let arg1 = stack_arg!(1);
                let arg2 = stack_arg!(2);
                let arg3 = stack_arg!(3);
                Some(func(self.ctx, arg0, arg1, arg2, arg3))
            }
            SubrFn::A5(func) => {
                let arg0 = stack_arg!(0);
                let arg1 = stack_arg!(1);
                let arg2 = stack_arg!(2);
                let arg3 = stack_arg!(3);
                let arg4 = stack_arg!(4);
                Some(func(self.ctx, arg0, arg1, arg2, arg3, arg4))
            }
            SubrFn::A6(func) => {
                let arg0 = stack_arg!(0);
                let arg1 = stack_arg!(1);
                let arg2 = stack_arg!(2);
                let arg3 = stack_arg!(3);
                let arg4 = stack_arg!(4);
                let arg5 = stack_arg!(5);
                Some(func(self.ctx, arg0, arg1, arg2, arg3, arg4, arg5))
            }
            SubrFn::A7(func) => {
                let arg0 = stack_arg!(0);
                let arg1 = stack_arg!(1);
                let arg2 = stack_arg!(2);
                let arg3 = stack_arg!(3);
                let arg4 = stack_arg!(4);
                let arg5 = stack_arg!(5);
                let arg6 = stack_arg!(6);
                Some(func(self.ctx, arg0, arg1, arg2, arg3, arg4, arg5, arg6))
            }
            SubrFn::A8(func) => {
                let arg0 = stack_arg!(0);
                let arg1 = stack_arg!(1);
                let arg2 = stack_arg!(2);
                let arg3 = stack_arg!(3);
                let arg4 = stack_arg!(4);
                let arg5 = stack_arg!(5);
                let arg6 = stack_arg!(6);
                let arg7 = stack_arg!(7);
                Some(func(
                    self.ctx, arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7,
                ))
            }
            SubrFn::Many(func) => {
                let args = args[args_start..args_start + nargs].to_vec();
                Some(func(self.ctx, args))
            }
            SubrFn::ManySlice(func) => {
                Some(self.call_many_slice_subr_from_stack_args(func, args_start, nargs))
            }
        }
    }

    fn call_many_slice_subr_from_stack_args(
        &mut self,
        func: crate::tagged::header::SubrFnManySlice,
        args_start: usize,
        nargs: usize,
    ) -> EvalResult {
        let args = &self.ctx.bc_buf;
        match nargs {
            0 => func(self.ctx, &[]),
            1 => {
                let arg0 = args[args_start];
                func(self.ctx, &[arg0])
            }
            2 => {
                let arg0 = args[args_start];
                let arg1 = args[args_start + 1];
                func(self.ctx, &[arg0, arg1])
            }
            3 => {
                let arg0 = args[args_start];
                let arg1 = args[args_start + 1];
                let arg2 = args[args_start + 2];
                func(self.ctx, &[arg0, arg1, arg2])
            }
            4 => {
                let arg0 = args[args_start];
                let arg1 = args[args_start + 1];
                let arg2 = args[args_start + 2];
                let arg3 = args[args_start + 3];
                func(self.ctx, &[arg0, arg1, arg2, arg3])
            }
            5 => {
                let arg0 = args[args_start];
                let arg1 = args[args_start + 1];
                let arg2 = args[args_start + 2];
                let arg3 = args[args_start + 3];
                let arg4 = args[args_start + 4];
                func(self.ctx, &[arg0, arg1, arg2, arg3, arg4])
            }
            6 => {
                let arg0 = args[args_start];
                let arg1 = args[args_start + 1];
                let arg2 = args[args_start + 2];
                let arg3 = args[args_start + 3];
                let arg4 = args[args_start + 4];
                let arg5 = args[args_start + 5];
                func(self.ctx, &[arg0, arg1, arg2, arg3, arg4, arg5])
            }
            7 => {
                let arg0 = args[args_start];
                let arg1 = args[args_start + 1];
                let arg2 = args[args_start + 2];
                let arg3 = args[args_start + 3];
                let arg4 = args[args_start + 4];
                let arg5 = args[args_start + 5];
                let arg6 = args[args_start + 6];
                func(self.ctx, &[arg0, arg1, arg2, arg3, arg4, arg5, arg6])
            }
            8 => {
                let arg0 = args[args_start];
                let arg1 = args[args_start + 1];
                let arg2 = args[args_start + 2];
                let arg3 = args[args_start + 3];
                let arg4 = args[args_start + 4];
                let arg5 = args[args_start + 5];
                let arg6 = args[args_start + 6];
                let arg7 = args[args_start + 7];
                func(self.ctx, &[arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7])
            }
            _ => {
                let args = LispArgVec::from_slice(&args[args_start..args_start + nargs]);
                func(self.ctx, &args)
            }
        }
    }

    fn direct_subr_call_target(
        &self,
        func_val: Value,
    ) -> Option<(SymId, SubrEntry, DirectSubrCallee)> {
        let (sym_id, entry, callee) = match func_val.kind() {
            ValueKind::Symbol(sym_id) => {
                if self.ctx.compiler_function_overrides_active() {
                    return None;
                }
                match self.ctx.obarray.symbol_function_id(sym_id) {
                    // GNU bytecode.c:Bcall resolves a symbol's live function
                    // cell and calls SUBRP function cells directly. Use the
                    // same resolved subr object here instead of consulting the
                    // static table again on the hot path.
                    Some(value)
                        if matches!(
                            value.kind(),
                            ValueKind::Subr(_) | ValueKind::Veclike(VecLikeType::Subr)
                        ) =>
                    {
                        let (callee_sym, entry) = subr_entry_from_value(value)?;
                        (callee_sym, entry, DirectSubrCallee::Value(value))
                    }
                    Some(value) if value.is_nil() => (
                        sym_id,
                        lookup_global_subr_entry(sym_id)?,
                        DirectSubrCallee::Symbol(sym_id),
                    ),
                    None => (
                        sym_id,
                        lookup_global_subr_entry(sym_id)?,
                        DirectSubrCallee::Symbol(sym_id),
                    ),
                    _ => return None,
                }
            }
            ValueKind::Veclike(VecLikeType::Subr) | ValueKind::Subr(_) => {
                let (sym_id, entry) = subr_entry_from_value(func_val)?;
                (sym_id, entry, DirectSubrCallee::Value(func_val))
            }
            _ => return None,
        };
        if entry.dispatch_kind != SubrDispatchKind::Builtin {
            return None;
        }
        Some((sym_id, entry, callee))
    }

    /// Execute a compiled function without param binding (for inline compilation).
    fn execute_inline(&mut self, func: &ByteCodeFunction) -> EvalResult {
        let condition_stack_base = self.ctx.condition_stack_len();
        let frame_base = self.ctx.bc_buf.len();
        self.ctx.bc_frames.push(crate::emacs_core::eval::BcFrame {
            base: frame_base,
            fun: Value::NIL,
        });
        let mut pc: usize = 0;
        let mut handlers = HandlerStack::new();
        let specpdl_base = self.ctx.specpdl.len();
        let mut bind_stack = BindStack::new();
        let frame_limit = match frame_base.checked_add(func.max_stack as usize) {
            Some(limit) => limit,
            None => {
                return self.cleanup_bytecode_frame(
                    Err(invalid_bytecode_flow()),
                    condition_stack_base,
                    specpdl_base,
                    frame_base,
                );
            }
        };
        if self.ctx.bc_buf.capacity() < frame_limit {
            self.ctx
                .bc_buf
                .reserve_exact(frame_limit - self.ctx.bc_buf.len());
        }
        let result = self.run_loop(
            func,
            frame_base,
            frame_limit,
            &mut pc,
            &mut handlers,
            &mut bind_stack,
        );
        self.cleanup_bytecode_frame(result, condition_stack_base, specpdl_base, frame_base)
    }

    fn resume_nonlocal(
        &mut self,
        _func: &ByteCodeFunction,
        pc: &mut usize,
        handlers: &mut HandlerStack,
        bind_stack: &mut BindStack,
        flow: Flow,
    ) -> Result<(), Flow> {
        match flow {
            Flow::Throw { tag, value } => {
                let selected_resume = self.ctx.matching_catch_resume(&tag);
                if let Some(ResumeTarget::VmCatch {
                    target,
                    stack_len,
                    spec_depth,
                    bind_stack_len,
                    ..
                }) = unwind_handlers_to_selected_resume(
                    handlers,
                    &mut self.ctx.condition_stack,
                    selected_resume.as_ref(),
                ) {
                    let root_scope = self.ctx.save_vm_roots();
                    self.ctx.push_vm_frame_root(tag);
                    self.ctx.push_vm_frame_root(value);
                    self.ctx.unbind_to(spec_depth);
                    bind_stack.truncate(bind_stack_len);
                    self.ctx.bc_buf.truncate(stack_len);
                    self.ctx.bc_buf.push(value);
                    self.ctx.restore_vm_roots(root_scope);
                    *pc = target as usize;
                    return Ok(());
                }

                if selected_resume.is_some() {
                    return Err(Flow::Throw { tag, value });
                }
                tracing::debug!(
                    target: "neomacs::throw_on_input",
                    ?tag,
                    ?value,
                    condition_stack_len = self.ctx.condition_stack.len(),
                    handler_stack_len = handlers.len(),
                    "vm resume_nonlocal: no matching catch for throw"
                );
                Err(signal("no-catch", vec![tag, value]))
            }
            Flow::Signal(sig) => {
                if sig.symbol == intern("kill-emacs") {
                    return Err(Flow::Signal(sig));
                }
                // dispatch_signal_if_needed may call signal hooks and
                // handler-bind handlers via eval.apply(), which can trigger
                // GC.  We must root the current frame so values survive
                // collection.
                let mut sig_extra = Vec::new();
                Self::collect_flow_roots(&Flow::Signal(sig.clone()), &mut sig_extra);
                let sig = match self.with_frame_roots(_func, &sig_extra, |vm| {
                    vm.ctx.dispatch_signal_if_needed(sig)
                }) {
                    Ok(sig) => sig,
                    Err(flow) => {
                        return self.resume_nonlocal(_func, pc, handlers, bind_stack, flow);
                    }
                };
                if let Some(ResumeTarget::VmConditionCase {
                    target,
                    stack_len,
                    spec_depth,
                    bind_stack_len,
                    ..
                }) = unwind_handlers_to_selected_resume(
                    handlers,
                    &mut self.ctx.condition_stack,
                    sig.selected_resume.as_ref(),
                ) {
                    let root_scope = self.ctx.save_vm_roots();
                    self.ctx.push_vm_frame_root(Value::from_sym_id(sig.symbol));
                    for value in sig.data.iter().copied() {
                        self.ctx.push_vm_frame_root(value);
                    }
                    if let Some(raw_data) = sig.raw_data {
                        self.ctx.push_vm_frame_root(raw_data);
                    }
                    self.ctx.unbind_to(spec_depth);
                    bind_stack.truncate(bind_stack_len);
                    self.ctx.bc_buf.truncate(stack_len);
                    self.ctx.bc_buf.push(make_signal_binding_value(&sig));
                    self.ctx.restore_vm_roots(root_scope);
                    *pc = target as usize;
                    return Ok(());
                }
                Err(Flow::Signal(sig))
            }
        }
    }

    fn dispatch_vm_builtin_with_frame(
        &mut self,
        func: &ByteCodeFunction,
        name: &str,
        args: impl Into<LispArgVec>,
    ) -> EvalResult {
        let args = args.into();
        self.with_frame_arg_roots(func, args, |vm, args| {
            vm.dispatch_vm_builtin_unrooted(name, args)
        })
    }

    fn dispatch_vm_builtin(&mut self, name: &str, args: impl Into<LispArgVec>) -> EvalResult {
        self.dispatch_vm_builtin_unrooted(name, args.into())
    }

    /// Dispatch to builtin functions from the VM.
    fn dispatch_vm_builtin_unrooted(&mut self, name: &str, args: LispArgVec) -> EvalResult {
        // VM-internal bytecode operations that are not real Elisp builtins.
        match name {
            "call-interactively" => return self.builtin_call_interactively_shared(&args),
            "start-kbd-macro" => {
                return crate::emacs_core::kmacro::builtin_start_kbd_macro(
                    &mut *self.ctx,
                    args.into_vec(),
                );
            }
            "end-kbd-macro" => {
                return crate::emacs_core::kmacro::builtin_end_kbd_macro(
                    &mut *self.ctx,
                    args.into_vec(),
                );
            }
            "call-last-kbd-macro" => return self.builtin_call_last_kbd_macro_shared(&args),
            "execute-kbd-macro" => return self.builtin_execute_kbd_macro_shared(&args),
            "garbage-collect" => return self.builtin_garbage_collect_shared(&args),
            "mapatoms" => return self.builtin_mapatoms_shared(&args),
            "maphash" => return self.builtin_maphash_shared(&args),
            "store-kbd-macro-event" => {
                return crate::emacs_core::kmacro::builtin_store_kbd_macro_event(
                    &mut *self.ctx,
                    args.into_vec(),
                );
            }
            "cancel-kbd-macro-events" => {
                return crate::emacs_core::builtins::builtin_cancel_kbd_macro_events(
                    &mut *self.ctx,
                    args.into_vec(),
                );
            }
            "%%defvar" => {
                if args.len() >= 2 {
                    let sym_name = args[1].as_symbol_name().unwrap_or("nil").to_string();
                    if !self.ctx.obarray.boundp(&sym_name) {
                        self.ctx.obarray.set_symbol_value(&sym_name, args[0]);
                    }
                    self.ctx.obarray.make_special(&sym_name);
                    return Ok(Value::symbol(sym_name));
                }
                return Ok(Value::NIL);
            }
            "%%defconst" => {
                if args.len() >= 2 {
                    let sym = args[1];
                    let sym_id = sym.as_symbol_id().unwrap_or_else(|| intern("nil"));
                    self.builtin_set_default_shared(&[Value::from_sym_id(sym_id), args[0]])?;
                    self.ctx.obarray.make_special_id(sym_id);
                    self.ctx.obarray.put_property_id(
                        sym_id,
                        intern("risky-local-variable"),
                        Value::T,
                    )?;
                    return Ok(Value::from_sym_id(sym_id));
                }
                return Ok(Value::NIL);
            }
            "%%unimplemented-elc-bytecode" => {
                return Err(signal(
                    "error",
                    vec![Value::string(
                        "Compiled .elc bytecode execution is not implemented yet",
                    )],
                ));
            }
            _ => {}
        }

        // All real builtins go through funcall_general → dispatch_subr.
        // This matches GNU Emacs where the bytecode VM delegates to
        // funcall_general for everything except bytecoded closures.
        self.ctx
            .funcall_general(Value::subr_from_sym_id(Self::builtin_name_id(name)), args)
    }

    fn with_default_directory_binding<T>(
        &mut self,
        directory: &crate::heap_types::LispString,
        f: impl FnOnce(&mut Self) -> Result<T, Flow>,
    ) -> Result<T, Flow> {
        let specpdl_count = self.ctx.specpdl.len();
        crate::emacs_core::eval::specbind_in_state(
            &mut self.ctx.obarray,
            &mut self.ctx.specpdl,
            intern("default-directory"),
            Value::heap_string(directory.clone()),
        );
        let result = f(self);
        crate::emacs_core::eval::unbind_to_in_state(
            &mut self.ctx.obarray,
            &mut self.ctx.specpdl,
            specpdl_count,
        );
        result
    }

    fn builtin_documentation_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::doc::builtin_documentation_in_vm_runtime(&mut self.ctx, args.to_vec())
    }

    fn builtin_documentation_property_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::doc::builtin_documentation_property_in_vm_runtime(
            &mut self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_format_mode_line_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::xdisp::builtin_format_mode_line_in_vm_runtime(&mut self.ctx, args)
    }

    fn builtin_read_from_minibuffer_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::finish_read_from_minibuffer_in_vm_runtime(&mut self.ctx, args)
    }

    fn builtin_call_interactively_shared(&mut self, args: &[Value]) -> EvalResult {
        let mut plan = crate::emacs_core::interactive::plan_call_interactively_in_state(
            &self.ctx.obarray,
            &self.ctx.interactive,
            self.ctx.read_command_keys(),
            args,
        )?;
        if crate::emacs_core::interactive::callable_form_needs_instantiation(&plan.func) {
            plan.func = self.ctx.instantiate_callable_cons_form(plan.func)?;
        }
        self.with_vm_root_scope(|vm| {
            for value in args.iter().copied() {
                vm.push_dynamic_vm_root(value);
            }
            vm.push_dynamic_vm_root(plan.func);
            let (_function, call_args) =
                crate::emacs_core::interactive::resolve_call_interactively_target_and_args_with_vm_fallback(
                    &mut vm.ctx,
                    &mut plan,
                )?;
            for value in call_args.iter().copied() {
                vm.push_dynamic_vm_root(value);
            }
            let mut funcall_args = Vec::with_capacity(call_args.len() + 1);
            funcall_args.push(plan.invocation_function);
            funcall_args.extend(call_args);
            vm.call_function_with_roots(Value::symbol("funcall-interactively"), &funcall_args)
        })
    }

    fn builtin_assoc_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_range_args("assoc", args, 2, 3)?;
        if args.get(2).is_some_and(|value| !value.is_nil()) {
            let key = args[0];
            let list = args[1];
            let test_fn = args[2];
            return self.with_vm_root_scope(|vm| {
                vm.push_dynamic_vm_root(key);
                vm.push_dynamic_vm_root(list);
                vm.push_dynamic_vm_root(test_fn);
                let mut cursor = list;
                loop {
                    match cursor.kind() {
                        ValueKind::Nil => return Ok(Value::NIL),
                        ValueKind::Cons => {
                            let pair_car = cursor.cons_car();
                            let pair_cdr = cursor.cons_cdr();
                            if let ValueKind::Cons = pair_car.kind() {
                                let entry_key = pair_car.cons_car();
                                let matches = vm.with_vm_root_scope(|vm| {
                                    vm.push_dynamic_vm_root(cursor);
                                    vm.push_dynamic_vm_root(pair_car);
                                    vm.push_dynamic_vm_root(pair_cdr);
                                    vm.push_dynamic_vm_root(entry_key);
                                    vm.call_function2(test_fn, entry_key, key)
                                        .map(|value| value.is_truthy())
                                });
                                let matches = matches?;
                                if matches {
                                    return Ok(pair_car);
                                }
                            }
                            cursor = pair_cdr;
                        }
                        _ => {
                            return Err(signal(
                                "wrong-type-argument",
                                vec![Value::symbol("listp"), list],
                            ));
                        }
                    }
                }
            });
        }
        crate::emacs_core::builtins::builtin_assoc(&mut *self.ctx, vec![args[0], args[1]])
    }

    fn builtin_plist_member_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_range_args("plist-member", args, 2, 3)?;
        if args.get(2).is_some_and(|value| !value.is_nil()) {
            let plist = args[0];
            let prop = args[1];
            let predicate = args[2];
            return self.with_vm_root_scope(|vm| {
                vm.push_dynamic_vm_root(plist);
                vm.push_dynamic_vm_root(prop);
                vm.push_dynamic_vm_root(predicate);
                let mut cursor = plist;
                loop {
                    match cursor.kind() {
                        ValueKind::Cons => {
                            let pair_car = cursor.cons_car();
                            let pair_cdr = cursor.cons_cdr();
                            let entry_key = pair_car;
                            let matches = vm.with_vm_root_scope(|vm| {
                                vm.push_dynamic_vm_root(cursor);
                                vm.push_dynamic_vm_root(entry_key);
                                vm.push_dynamic_vm_root(pair_cdr);
                                vm.call_function2(predicate, entry_key, prop)
                                    .map(|value| value.is_truthy())
                            });
                            let matches = matches?;
                            if matches {
                                return Ok(cursor);
                            }

                            // Match GNU's `plist_member` nil-
                            // terminator rule: an unpaired last key is
                            // a valid end (return nil, not-found);
                            // only dotted tails signal plistp.
                            match pair_cdr.kind() {
                                ValueKind::Cons => {
                                    cursor = pair_cdr.cons_cdr();
                                }
                                ValueKind::Nil => {
                                    return Ok(Value::NIL);
                                }
                                _ => {
                                    return Err(signal(
                                        "wrong-type-argument",
                                        vec![Value::symbol("plistp"), plist],
                                    ));
                                }
                            }
                        }
                        ValueKind::Nil => return Ok(Value::NIL),
                        _ => {
                            return Err(signal(
                                "wrong-type-argument",
                                vec![Value::symbol("plistp"), plist],
                            ));
                        }
                    }
                }
            });
        }
        crate::emacs_core::builtins::plist_member_eq_swp(
            args.to_vec(),
            self.ctx.symbols_with_pos_enabled,
        )
    }

    fn builtin_garbage_collect_shared(&mut self, args: &[Value]) -> EvalResult {
        builtins::expect_args("garbage-collect", args, 0)?;
        self.ctx.gc_collect_exact();
        crate::emacs_core::builtins_extra::builtin_garbage_collect_stats()
    }

    fn builtin_kill_emacs_shared(&mut self, args: &[Value]) -> EvalResult {
        let request = crate::emacs_core::builtins::symbols::plan_kill_emacs_request(args)?;
        self.builtin_run_hooks_shared(&[Value::symbol("kill-emacs-hook")])?;
        self.ctx
            .request_shutdown(request.exit_code, request.restart);
        Err(signal_suppressed("kill-emacs", vec![]))
    }

    fn builtin_macroexpand_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::symbols::builtin_macroexpand_slice_with_runtime(self, args)
    }

    fn builtin_mapatoms_shared(&mut self, args: &[Value]) -> EvalResult {
        let (func, symbols) =
            crate::emacs_core::hashtab::collect_mapatoms_symbols(&self.ctx, args.to_vec())?;
        self.with_dynamic_vm_roots(|vm| {
            vm.push_dynamic_vm_root(func);
            for sym in symbols.iter().copied() {
                vm.push_dynamic_vm_root(sym);
            }
            for sym in symbols {
                vm.call_function1(func, sym)?;
            }
            Ok(Value::NIL)
        })
    }

    fn builtin_maphash_shared(&mut self, args: &[Value]) -> EvalResult {
        let (func, table) = crate::emacs_core::hashtab::validate_maphash_args(args)?;
        self.with_dynamic_vm_roots(|vm| {
            vm.push_dynamic_vm_root(func);
            vm.push_dynamic_vm_root(table);
            let mut slot = 0_usize;
            loop {
                let Some((key, value)) =
                    crate::emacs_core::hashtab::maphash_entry_at_slot(table, slot)
                else {
                    if slot >= crate::emacs_core::hashtab::maphash_slot_len(table) {
                        break;
                    }
                    slot += 1;
                    continue;
                };
                vm.push_dynamic_vm_root(key);
                vm.push_dynamic_vm_root(value);
                vm.call_function2(func, key, value)?;
                slot += 1;
            }
            Ok(Value::NIL)
        })
    }

    fn builtin_read_string_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::finish_read_string_in_vm_runtime(&mut self.ctx, args)
    }

    fn builtin_completing_read_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::validate_completing_read_arity(args)?;
        if let Some(function) = crate::emacs_core::reader::completing_read_function_value(&self.ctx)
        {
            return self.call_function(function, args.to_vec());
        }

        crate::emacs_core::reader::finish_completing_read_in_vm_runtime(&mut self.ctx, args)
    }

    fn builtin_read_buffer_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::minibuffer::builtin_read_buffer_in_runtime(self.ctx, args)?;
        let completing_args = crate::emacs_core::minibuffer::read_buffer_completing_args(
            &self.ctx.obarray,
            &self.ctx.buffers,
            args,
        );
        self.builtin_completing_read_shared(&completing_args)
    }

    fn builtin_try_completion_shared(&mut self, args: &[Value]) -> EvalResult {
        let candidates =
            crate::emacs_core::minibuffer::completion_candidates_from_collection_in_state(
                &*self.ctx, &args[1],
            )?;
        let ignore_case = self
            .ctx
            .obarray
            .symbol_value("completion-ignore-case")
            .is_some_and(|v| v.is_truthy());
        let regexps = crate::emacs_core::minibuffer::completion_regexp_lisp_list_from_obarray(
            &self.ctx.obarray,
        );
        crate::emacs_core::minibuffer::builtin_try_completion_with_candidates(
            args,
            candidates,
            ignore_case,
            &regexps,
            |function, call_args| self.call_function_with_roots(function, &call_args),
        )
    }

    fn builtin_all_completions_shared(&mut self, args: &[Value]) -> EvalResult {
        let candidates =
            crate::emacs_core::minibuffer::completion_candidates_from_collection_in_state(
                &*self.ctx, &args[1],
            )?;
        let ignore_case = self
            .ctx
            .obarray
            .symbol_value("completion-ignore-case")
            .is_some_and(|v| v.is_truthy());
        let regexps = crate::emacs_core::minibuffer::completion_regexp_lisp_list_from_obarray(
            &self.ctx.obarray,
        );
        crate::emacs_core::minibuffer::builtin_all_completions_with_candidates(
            args,
            candidates,
            ignore_case,
            &regexps,
            |function, call_args| self.call_function_with_roots(function, &call_args),
        )
    }

    fn builtin_file_name_completion_shared(&mut self, args: &[Value]) -> EvalResult {
        let needs_eval_predicate = matches!(
            args.get(2),
            Some(predicate)
                if !predicate.is_nil()
                    && !(predicate.is_symbol() || predicate.as_subr_id().is_some())
        );
        if needs_eval_predicate {
            let plan = crate::emacs_core::dired::prepare_file_name_completion_in_state(
                &self.ctx.obarray,
                &[],
                &self.ctx.buffers,
                args,
            )?;
            let predicate = args[2];
            let use_absolute_path = crate::emacs_core::dired::predicate_uses_absolute_file_argument(
                &self.ctx.obarray,
                &predicate,
            );
            let bound_directory = plan.directory.clone();
            return crate::emacs_core::dired::finish_file_name_completion_with_callable_predicate(
                use_absolute_path,
                plan.directory,
                plan.file,
                plan.completions,
                plan.ignore_case,
                |predicate_arg| {
                    self.with_default_directory_binding(&bound_directory, |vm| {
                        vm.call_function_with_roots(predicate, &[predicate_arg])
                    })
                },
            );
        }
        crate::emacs_core::dired::builtin_file_name_completion(&mut *self.ctx, args.to_vec())
    }

    fn builtin_read_command_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::minibuffer::finish_read_command_in_vm_runtime(&mut self.ctx, args)
    }

    fn builtin_read_variable_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::minibuffer::finish_read_variable_in_vm_runtime(&mut self.ctx, args)
    }

    fn builtin_test_completion_shared(&mut self, args: &[Value]) -> EvalResult {
        let candidates =
            crate::emacs_core::minibuffer::completion_candidates_from_collection_in_state(
                &*self.ctx, &args[1],
            )?;
        let ignore_case = self
            .ctx
            .obarray
            .symbol_value("completion-ignore-case")
            .is_some_and(|v| v.is_truthy());
        let regexps = crate::emacs_core::minibuffer::completion_regexp_lisp_list_from_obarray(
            &self.ctx.obarray,
        );
        crate::emacs_core::minibuffer::builtin_test_completion_with_candidates(
            args,
            candidates,
            ignore_case,
            &regexps,
            |function, call_args| self.call_function_with_roots(function, &call_args),
        )
    }

    fn builtin_input_pending_p_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::builtin_input_pending_p(&mut *self.ctx, args.to_vec())
    }

    fn builtin_discard_input_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::builtin_discard_input(&mut *self.ctx, args.to_vec())
    }

    fn builtin_current_input_mode_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::builtin_current_input_mode(&mut *self.ctx, args.to_vec())
    }

    fn builtin_set_input_mode_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::builtin_set_input_mode(&mut *self.ctx, args.to_vec())
    }

    fn builtin_set_input_interrupt_mode_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::builtin_set_input_interrupt_mode(&mut *self.ctx, args.to_vec())
    }

    fn builtin_read_char_shared(&mut self, args: &[Value]) -> EvalResult {
        if let Some(value) =
            crate::emacs_core::reader::builtin_read_char_in_runtime(self.ctx, args)?
        {
            return Ok(value);
        }
        crate::emacs_core::reader::finish_read_char_interactive_in_runtime(self.ctx, args)
    }

    fn builtin_read_from_string_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::builtin_read_from_string(&mut *self.ctx, args.to_vec())
    }

    fn builtin_read_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::builtin_read(&mut *self.ctx, args.to_vec())
    }

    fn builtin_read_event_shared(&mut self, args: &[Value]) -> EvalResult {
        if let Some(value) =
            crate::emacs_core::lread::builtin_read_event_in_runtime(self.ctx, args)?
        {
            return Ok(value);
        }
        crate::emacs_core::lread::finish_read_event_interactive_in_runtime(self.ctx, args)
    }

    fn builtin_read_char_exclusive_shared(&mut self, args: &[Value]) -> EvalResult {
        if let Some(value) =
            crate::emacs_core::lread::builtin_read_char_exclusive_in_runtime(self.ctx, args)?
        {
            return Ok(value);
        }
        crate::emacs_core::lread::finish_read_char_exclusive_interactive_in_runtime(self.ctx, args)
    }

    fn builtin_read_key_sequence_shared(&mut self, args: &[Value]) -> EvalResult {
        if let Some(value) =
            crate::emacs_core::reader::builtin_read_key_sequence_in_runtime(self.ctx, args)?
        {
            return Ok(value);
        }
        crate::emacs_core::reader::finish_read_key_sequence_interactive_in_runtime(
            self.ctx,
            crate::emacs_core::reader::read_key_sequence_options_from_args(args),
        )
    }

    fn builtin_read_key_sequence_vector_shared(&mut self, args: &[Value]) -> EvalResult {
        if let Some(value) =
            crate::emacs_core::reader::builtin_read_key_sequence_vector_in_runtime(self.ctx, args)?
        {
            return Ok(value);
        }
        crate::emacs_core::reader::finish_read_key_sequence_vector_interactive_in_runtime(
            self.ctx,
            crate::emacs_core::reader::read_key_sequence_options_from_args(args),
        )
    }

    fn builtin_recent_keys_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::keymaps::builtin_recent_keys_impl(&*self.ctx, args.to_vec())
    }

    fn builtin_current_message_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_current_message(&mut *self.ctx, args.to_vec())
    }

    fn builtin_current_case_table_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::casetab::builtin_current_case_table(&mut *self.ctx, args.to_vec())
    }

    fn builtin_standard_case_table_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::casetab::builtin_standard_case_table(&mut *self.ctx, args.to_vec())
    }

    fn builtin_set_case_table_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::casetab::builtin_set_case_table(&mut *self.ctx, args.to_vec())
    }

    fn builtin_set_standard_case_table_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::casetab::builtin_set_standard_case_table(&mut *self.ctx, args.to_vec())
    }

    fn builtin_format_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_format_wrapper_strict(&mut *self.ctx, args.to_vec())
    }

    fn builtin_format_message_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_format_message(&mut *self.ctx, args.to_vec())
    }

    fn builtin_message_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_message(&mut *self.ctx, args.to_vec())
    }

    fn builtin_message_box_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_message_box(&mut *self.ctx, args.to_vec())
    }

    fn builtin_message_or_box_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_message_or_box(&mut *self.ctx, args.to_vec())
    }

    fn builtin_make_thread_shared(&mut self, args: &[Value]) -> EvalResult {
        let (thread_id, function) =
            crate::emacs_core::threads::prepare_make_thread(&mut self.ctx.threads, args)?;
        self.ctx
            .threads
            .set_thread_current_buffer(thread_id, self.ctx.buffers.current_buffer_id());
        let runtime_state =
            crate::emacs_core::threads::enter_thread_runtime(&mut *self.ctx, thread_id)?;
        let result = self.call_function_with_roots(function, &[]);
        crate::emacs_core::threads::exit_thread_runtime(&mut *self.ctx, thread_id, runtime_state);
        crate::emacs_core::threads::finish_make_thread_result(
            &mut self.ctx.threads,
            thread_id,
            result,
        )
    }

    fn builtin_thread_join_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_thread_join(&mut *self.ctx, args.to_vec())
    }

    fn builtin_thread_yield_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_thread_yield(&mut *self.ctx, args.to_vec())
    }

    fn builtin_thread_name_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_thread_name(&mut *self.ctx, args.to_vec())
    }

    fn builtin_thread_live_p_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_thread_live_p(&mut *self.ctx, args.to_vec())
    }

    fn builtin_threadp_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_threadp(&mut *self.ctx, args.to_vec())
    }

    fn builtin_thread_signal_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_thread_signal(&mut *self.ctx, args.to_vec())
    }

    fn builtin_current_thread_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_current_thread(&mut *self.ctx, args.to_vec())
    }

    fn builtin_all_threads_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_all_threads(&mut *self.ctx, args.to_vec())
    }

    fn builtin_thread_last_error_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_thread_last_error(&mut *self.ctx, args.to_vec())
    }

    fn builtin_make_mutex_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_make_mutex(&mut *self.ctx, args.to_vec())
    }

    fn builtin_mutex_name_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_mutex_name(&mut *self.ctx, args.to_vec())
    }

    fn builtin_mutex_lock_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_mutex_lock(&mut *self.ctx, args.to_vec())
    }

    fn builtin_mutex_unlock_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_mutex_unlock(&mut *self.ctx, args.to_vec())
    }

    fn builtin_mutexp_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_mutexp(&mut *self.ctx, args.to_vec())
    }

    fn builtin_make_condition_variable_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_make_condition_variable(&mut *self.ctx, args.to_vec())
    }

    fn builtin_condition_variable_p_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_condition_variable_p(&mut *self.ctx, args.to_vec())
    }

    fn builtin_condition_name_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_condition_name(&mut *self.ctx, args.to_vec())
    }

    fn builtin_condition_mutex_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_condition_mutex(&mut *self.ctx, args.to_vec())
    }

    fn builtin_condition_wait_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_condition_wait(&mut *self.ctx, args.to_vec())
    }

    fn builtin_condition_notify_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::threads::builtin_condition_notify(&mut *self.ctx, args.to_vec())
    }

    fn builtin_princ_shared(&mut self, args: &[Value]) -> EvalResult {
        let target =
            crate::emacs_core::builtins::resolve_print_target_in_state(&*self.ctx, args.get(1));
        if crate::emacs_core::builtins::print_target_is_direct(target) {
            return crate::emacs_core::builtins::builtin_princ_impl(&mut *self.ctx, args.to_vec());
        }
        let text = crate::emacs_core::builtins::print_value_princ_in_state(&*self.ctx, &args[0]);
        crate::emacs_core::builtins::dispatch_print_callback_chars(&text, |ch| {
            self.call_function_with_roots(target, &[ch]).map(|_| ())
        })?;
        Ok(args[0])
    }

    fn builtin_prin1_shared(&mut self, args: &[Value]) -> EvalResult {
        let target =
            crate::emacs_core::builtins::resolve_print_target_in_state(&*self.ctx, args.get(1));
        if crate::emacs_core::builtins::print_target_is_direct(target) {
            return crate::emacs_core::builtins::builtin_prin1_impl(&mut *self.ctx, args.to_vec());
        }
        let text = crate::emacs_core::error::print_value_in_state(&*self.ctx, &args[0]);
        crate::emacs_core::builtins::dispatch_print_callback_chars(&text, |ch| {
            self.call_function_with_roots(target, &[ch]).map(|_| ())
        })?;
        Ok(args[0])
    }

    fn builtin_prin1_to_string_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::builtin_prin1_to_string_impl(&*self.ctx, args.to_vec())
    }

    fn builtin_print_shared(&mut self, args: &[Value]) -> EvalResult {
        let target =
            crate::emacs_core::builtins::resolve_print_target_in_state(&*self.ctx, args.get(1));
        if crate::emacs_core::builtins::print_target_is_direct(target) {
            return crate::emacs_core::builtins::builtin_print_impl(&mut *self.ctx, args.to_vec());
        }
        let text = {
            let mut text = String::new();
            text.push('\n');
            text.push_str(&crate::emacs_core::error::print_value_in_state(
                &*self.ctx, &args[0],
            ));
            text.push('\n');
            text
        };
        crate::emacs_core::builtins::dispatch_print_callback_chars(&text, |ch| {
            self.call_function_with_roots(target, &[ch]).map(|_| ())
        })?;
        Ok(args[0])
    }

    fn builtin_terpri_shared(&mut self, args: &[Value]) -> EvalResult {
        if let Some(result) =
            crate::emacs_core::builtins::builtin_terpri_impl(&mut *self.ctx, args.to_vec())?
        {
            return Ok(result);
        }
        let target =
            crate::emacs_core::builtins::resolve_print_target_in_state(&*self.ctx, args.first());
        self.call_function_with_roots(target, &[Value::fixnum('\n' as i64)])?;
        Ok(Value::T)
    }

    fn builtin_write_char_shared(&mut self, args: &[Value]) -> EvalResult {
        if let Some(result) =
            crate::emacs_core::builtins::builtin_write_char_impl(&mut *self.ctx, args.to_vec())?
        {
            return Ok(result);
        }
        let target =
            crate::emacs_core::builtins::resolve_print_target_in_state(&*self.ctx, args.get(1));
        builtins::expect_range_args("write-char", args, 1, 2)?;
        let char_code = builtins::expect_fixnum(&args[0])?;
        self.call_function_with_roots(target, &[Value::fixnum(char_code)])?;
        Ok(Value::fixnum(char_code))
    }

    fn builtin_redraw_frame_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::dispnew::pure::builtin_redraw_frame(&mut *self.ctx, args.to_vec())
    }

    fn builtin_x_get_resource_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::display::builtin_x_get_resource(&mut *self.ctx, args.to_vec())
    }

    fn builtin_x_list_fonts_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::display::builtin_x_list_fonts(&mut *self.ctx, args.to_vec())
    }

    fn builtin_x_server_vendor_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::display::builtin_x_server_vendor(&mut *self.ctx, args.to_vec())
    }

    fn builtin_xw_display_color_p_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::builtins::symbols::builtin_xw_display_color_p_ctx(
            &*self.ctx,
            args.to_vec(),
        )
    }

    fn builtin_display_color_cells_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::display::builtin_display_color_cells(&mut *self.ctx, args.to_vec())
    }

    fn builtin_tty_type_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::terminal::pure::builtin_tty_type(&mut *self.ctx, args.to_vec())
    }

    fn builtin_suspend_tty_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::terminal::pure::builtin_suspend_tty(&mut *self.ctx, args.to_vec())
    }

    fn builtin_resume_tty_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::terminal::pure::builtin_resume_tty(&mut *self.ctx, args.to_vec())
    }

    fn builtin_x_create_frame_shared(&mut self, args: &[Value]) -> EvalResult {
        tracing::debug!("builtin_x_create_frame_shared: delegating to Context");
        crate::emacs_core::window_cmds::builtin_x_create_frame(&mut *self.ctx, args.to_vec())
    }

    fn builtin_make_frame_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::window_cmds::builtin_make_frame(&mut *self.ctx, args.to_vec())
    }

    fn builtin_set_frame_height_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::window_cmds::builtin_set_frame_height(&mut *self.ctx, args.to_vec())
    }

    fn builtin_set_frame_width_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::window_cmds::builtin_set_frame_width(&mut *self.ctx, args.to_vec())
    }

    fn builtin_set_frame_size_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::window_cmds::builtin_set_frame_size(&mut *self.ctx, args.to_vec())
    }

    fn builtin_yes_or_no_p_shared(&mut self, args: &[Value]) -> EvalResult {
        crate::emacs_core::reader::finish_yes_or_no_p_in_vm_runtime(&mut self.ctx, args)
    }
}

impl<'a> crate::emacs_core::builtins::symbols::MacroexpandRuntime for Vm<'a> {
    fn symbol_function_by_id(&self, symbol: SymId) -> Option<Value> {
        crate::emacs_core::builtins::symbols::symbol_function_cell_in_obarray(
            &self.ctx.obarray,
            symbol,
        )
    }

    fn autoload_do_load_macro(&mut self, autoload: Value, head: Value) -> Result<(), Flow> {
        let args = vec![autoload, head, Value::symbol("macro")];
        let _ = self.with_vm_root_scope(|vm| {
            for value in args.iter().copied() {
                vm.push_dynamic_vm_root(value);
            }
            crate::emacs_core::autoload::builtin_autoload_do_load_in_vm_runtime(&mut vm.ctx, &args)
        })?;
        Ok(())
    }

    fn apply_macro_function(
        &mut self,
        form: Value,
        function: Value,
        args: Vec<Value>,
        environment: Option<Value>,
    ) -> Result<Value, Flow> {
        if let Some(cached) = self
            .ctx
            .lookup_runtime_macro_expansion(function, &args, environment)
        {
            return Ok(cached);
        }
        let args_for_cache = args.clone();
        let expand_start = std::time::Instant::now();
        self.with_dynamic_vm_roots(move |vm| {
            vm.push_dynamic_vm_root(form);
            vm.push_dynamic_vm_root(function);
            if let Some(environment) = environment {
                vm.push_dynamic_vm_root(environment);
            }
            for value in args.iter().copied() {
                vm.push_dynamic_vm_root(value);
            }
            // GNU `Fmacroexpand` applies macro expanders directly.  Only the
            // ordinary `eval_sub` macro-call path specbinds
            // `lexical-binding`; byte-compiled bytecomp/macroexp code depends
            // on the caller's visible dynamic value while compiling source.
            let expanded = vm.call_function(function, args)?;
            let expand_elapsed = expand_start.elapsed();
            vm.ctx.store_runtime_macro_expansion(
                form,
                function,
                &args_for_cache,
                &expanded,
                expand_elapsed,
                environment,
            );
            Ok(expanded)
        })
    }
}

impl crate::emacs_core::builtins::higher_order::SortRuntime for Vm<'_> {
    fn call_sort_function1(&mut self, function: Value, arg: Value) -> Result<Value, Flow> {
        self.with_vm_root_scope(|vm| {
            vm.push_dynamic_vm_root(arg);
            vm.call_function1(function, arg)
        })
    }

    fn call_sort_function2(
        &mut self,
        function: Value,
        arg0: Value,
        arg1: Value,
    ) -> Result<Value, Flow> {
        self.with_vm_root_scope(|vm| {
            vm.push_dynamic_vm_root(arg0);
            vm.push_dynamic_vm_root(arg1);
            vm.call_function2(function, arg0, arg1)
        })
    }

    fn root_sort_value(&mut self, value: Value) {
        self.push_dynamic_vm_root(value);
    }

    fn compare_sort_keys(
        &mut self,
        left: &Value,
        right: &Value,
    ) -> Result<std::cmp::Ordering, Flow> {
        crate::emacs_core::builtins::symbols::compare_value_lt(self.ctx, left, right)
    }
}

// -- Arithmetic helpers --

pub(crate) fn condition_frame_resume(frame: ConditionFrame) -> ResumeTarget {
    match frame {
        ConditionFrame::Catch { resume, .. } | ConditionFrame::ConditionCase { resume, .. } => {
            resume
        }
        ConditionFrame::HandlerBind { .. } | ConditionFrame::SkipConditions { .. } => {
            unreachable!("VM handler stack only mirrors catch/condition-case frames")
        }
    }
}

fn unwind_handlers_to_selected_resume(
    handlers: &mut HandlerStack,
    condition_stack: &mut Vec<ConditionFrame>,
    selected_resume: Option<&ResumeTarget>,
) -> Option<ResumeTarget> {
    while let Some(handler) = handlers.pop() {
        match handler {
            Handler::Condition => {
                let resume = condition_frame_resume(
                    condition_stack
                        .pop()
                        .expect("handler stack and condition stack diverged"),
                );
                if selected_resume.is_some_and(|selected| &resume == selected) {
                    return Some(resume);
                }
            }
        }
    }
    None
}

fn normalize_vm_builtin_error(name: &str, flow: Flow) -> Flow {
    match flow {
        Flow::Signal(mut sig) if sig.symbol_name() == "wrong-number-of-arguments" => {
            if let Some(first) = sig.data.first_mut() {
                if matches!(first.kind(), ValueKind::Symbol(id) if resolve_sym(id) == name) {
                    *first = Value::subr_from_sym_id(intern(name));
                }
            }
            Flow::Signal(sig)
        }
        other => other,
    }
}

fn resolve_switch_target(func: &ByteCodeFunction, raw_addr: i64) -> Result<usize, Flow> {
    let raw_addr = usize::try_from(raw_addr).map_err(|_| {
        signal(
            "error",
            vec![Value::string(format!(
                "invalid GNU switch target byte offset {}",
                raw_addr
            ))],
        )
    })?;

    if let Some(offset_map) = &func.gnu_byte_offset_map {
        offset_map
            .binary_search_by_key(&raw_addr, |entry| entry.byte_offset)
            .map(|index| offset_map[index].instruction_index)
            .map_err(|_| {
                signal(
                    "error",
                    vec![Value::string(format!(
                        "invalid GNU switch target byte offset {}",
                        raw_addr
                    ))],
                )
            })
    } else {
        Ok(raw_addr)
    }
}

/// Extract a `SymId` from a bytecode constants vector entry without
/// going through the global string interner.
///
/// `Op::VarRef` / `Op::VarSet` / `Op::VarBind` all reference variables
/// by index into the function's constants table.  Each constant is
/// already a `Value::Symbol(SymId)`, so we can extract the SymId via a
/// pure tag inspection.  Going through `as_symbol_name() -> &str ->
/// intern() -> SymId` instead would acquire the global interner
/// `RwLock` twice per opcode, which dominated debug-build runtime when
/// the byte-compiler iterated over hot loops.
///
/// When `read-positioning-symbols` wraps constants as symbol-with-pos,
/// we transparently unwrap to the bare symbol SymId.
fn sym_id_at(constants: &[Value], idx: u16) -> SymId {
    constants
        .get(idx as usize)
        .and_then(|v| {
            v.as_symbol_id().or_else(|| {
                v.as_symbol_with_pos_sym()
                    .and_then(|sym| sym.as_symbol_id())
            })
        })
        .unwrap_or_else(|| intern("nil"))
}
#[cfg(test)]
#[path = "vm_test.rs"]
mod tests;
