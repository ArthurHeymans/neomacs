use std::collections::HashMap;

use lasso::{Key, Rodeo, Spur};
use neovm_compiler::ids::FunctionId;
use neovm_compiler::regir::RegModule;
use neovm_compiler::ssa::SsaLambdaTemplate;
use neovm_compiler::surface::SurfaceForm;

use crate::{LispValue, Runtime};

#[repr(C)]
pub struct JitContext {
    pub runtime: *mut Runtime,
    pub symbols: *mut Rodeo,
    pub strings: *mut Rodeo,
    pub quoted_forms: *mut Vec<SurfaceForm>,
    pub lambda_templates: *mut Vec<SsaLambdaTemplate>,
    pub regir: *mut RegModule,
    pub functions_by_name: *mut HashMap<String, FunctionId>,
    pub gc_roots: Vec<LispValue>,
    pub gc_root_base: usize,
}

macro_rules! jit_shim {
    ($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? { $($body:tt)* }) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name($($arg: $ty),*) $(-> $ret)? {
            unsafe { $($body)* }
        }
    };
}

macro_rules! ctx_and_rt {
    ($vmctx:ident, $ctx:ident, $rt:ident) => {
        let $ctx = &mut *($vmctx as *mut JitContext);
        let $rt = &mut *$ctx.runtime;
    };
}

macro_rules! resolve_sym {
    ($ctx:expr, $idx:expr) => {{
        let rodeo = &*$ctx.symbols;
        let spur = Spur::try_from_usize($idx as usize).expect("invalid symbol index");
        rodeo.resolve(&spur)
    }};
}

macro_rules! resolve_str {
    ($ctx:expr, $idx:expr) => {{
        let rodeo = &*$ctx.strings;
        let spur = Spur::try_from_usize($idx as usize).expect("invalid string index");
        rodeo.resolve(&spur)
    }};
}

// --- Core heap operations ---

jit_shim!(__neomacs_rt_cons(vmctx: i64, car: i64, cdr: i64) -> i64 {
    ctx_and_rt!(vmctx, ctx, rt);
    rt.cons(LispValue::from_abi_i64(car), LispValue::from_abi_i64(cdr)).to_abi_i64()
});

jit_shim!(__neomacs_rt_car(vmctx: i64, pair: i64) -> i64 {
    ctx_and_rt!(vmctx, ctx, rt);
    let result = rt.car(LispValue::from_abi_i64(pair));
    match result { Ok(v) => v.to_abi_i64(), Err(e) => { eprintln!("JIT rt error: {e}"); LispValue::NIL.to_abi_i64() } }
});

jit_shim!(__neomacs_rt_cdr(vmctx: i64, pair: i64) -> i64 {
    ctx_and_rt!(vmctx, ctx, rt);
    let result = rt.cdr(LispValue::from_abi_i64(pair));
    match result { Ok(v) => v.to_abi_i64(), Err(e) => { eprintln!("JIT rt error: {e}"); LispValue::NIL.to_abi_i64() } }
});

jit_shim!(__neomacs_rt_make_lexical_cell(vmctx: i64, value: i64) -> i64 {
    ctx_and_rt!(vmctx, ctx, rt);
    rt.lexical_cell(LispValue::from_abi_i64(value)).to_abi_i64()
});

jit_shim!(__neomacs_rt_lexical_cell_get(vmctx: i64, cell: i64) -> i64 {
    ctx_and_rt!(vmctx, ctx, rt);
    match rt.lexical_cell_get(LispValue::from_abi_i64(cell)) {
        Ok(v) => v.to_abi_i64(),
        Err(e) => { eprintln!("JIT rt error: {e}"); LispValue::NIL.to_abi_i64() }
    }
});

jit_shim!(__neomacs_rt_lexical_cell_set(vmctx: i64, cell: i64, value: i64) -> i64 {
    ctx_and_rt!(vmctx, ctx, rt);
    match rt.lexical_cell_set(LispValue::from_abi_i64(cell), LispValue::from_abi_i64(value)) {
        Ok(v) => v.to_abi_i64(),
        Err(e) => { eprintln!("JIT rt error: {e}"); LispValue::NIL.to_abi_i64() }
    }
});

// --- Symbol operations ---

jit_shim!(__neomacs_rt_symbol_get(vmctx: i64, symbol_index: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let name = resolve_sym!(ctx, symbol_index);
    let rt = &mut *ctx.runtime;
    let symbol = rt.intern(name);
    match rt.symbol_value(symbol) { Ok(v) => v.to_abi_i64(), Err(e) => { eprintln!("JIT rt error: {e}"); LispValue::NIL.to_abi_i64() } }
});

jit_shim!(__neomacs_rt_symbol_set(vmctx: i64, symbol_index: i64, value: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let name = resolve_sym!(ctx, symbol_index);
    let rt = &mut *ctx.runtime;
    let symbol = rt.intern(name);
    match rt.set_symbol_value(symbol, LispValue::from_abi_i64(value)) { Ok(v) => v.to_abi_i64(), Err(e) => { eprintln!("JIT rt error: {e}"); LispValue::NIL.to_abi_i64() } }
});

jit_shim!(__neomacs_rt_bind_dynamic(vmctx: i64, symbol_index: i64, value: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let name = resolve_sym!(ctx, symbol_index);
    let rt = &mut *ctx.runtime;
    if let Err(e) = rt.bind_dynamic_by_name(name, LispValue::from_abi_i64(value)) { eprintln!("JIT rt error: {e}"); }
    0
});

jit_shim!(__neomacs_rt_unbind_dynamic(vmctx: i64, count: i64) -> i64 {
    ctx_and_rt!(vmctx, ctx, rt);
    if let Err(e) = rt.unbind_dynamic(count as usize) { eprintln!("JIT rt error: {e}"); }
    0
});

// --- String/Float ---

jit_shim!(__neomacs_rt_string_const(vmctx: i64, string_index: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let value = resolve_str!(ctx, string_index);
    let rt = &mut *ctx.runtime;
    rt.string(value).to_abi_i64()
});

jit_shim!(__neomacs_rt_float_const(vmctx: i64, bits: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let rt = &mut *ctx.runtime;
    let value = f64::from_bits(bits as u64);
    rt.float(value).to_abi_i64()
});

// --- Quote / Lambda ---

jit_shim!(__neomacs_rt_quote(vmctx: i64, form_index: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let forms = &*ctx.quoted_forms;
    let rt = &mut *ctx.runtime;
    match forms.get(form_index as usize) {
        Some(form) => surface_form_to_lisp(rt, form).to_abi_i64(),
        None => { eprintln!("JIT: quote index {form_index} OOB"); LispValue::NIL.to_abi_i64() }
    }
});

jit_shim!(__neomacs_rt_function_quote(vmctx: i64, fn_index: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let forms = &*ctx.quoted_forms;
    let rt = &mut *ctx.runtime;
    match forms.get(fn_index as usize) {
        Some(form) => surface_form_to_lisp(rt, form).to_abi_i64(),
        None => { eprintln!("JIT: fn-quote index {fn_index} OOB"); LispValue::NIL.to_abi_i64() }
    }
});

jit_shim!(__neomacs_rt_lambda_0(vmctx: i64, template_index: i64) -> i64 {
    make_lambda(vmctx, template_index, &[])
});

jit_shim!(__neomacs_rt_lambda_1(vmctx: i64, template_index: i64, capture0: i64) -> i64 {
    make_lambda(vmctx, template_index, &[capture0])
});

jit_shim!(__neomacs_rt_lambda_2(vmctx: i64, template_index: i64, capture0: i64, capture1: i64) -> i64 {
    make_lambda(vmctx, template_index, &[capture0, capture1])
});

jit_shim!(__neomacs_rt_lambda_3(vmctx: i64, template_index: i64, capture0: i64, capture1: i64, capture2: i64) -> i64 {
    make_lambda(vmctx, template_index, &[capture0, capture1, capture2])
});

unsafe fn make_lambda(vmctx: i64, template_index: i64, captures: &[i64]) -> i64 {
    unsafe {
        let ctx = &mut *(vmctx as *mut JitContext);
        let templates = &*ctx.lambda_templates;
        let rt = &mut *ctx.runtime;
        match templates.get(template_index as usize) {
            Some(template) => {
                let capture_values: Vec<LispValue> = captures
                    .iter()
                    .map(|c| LispValue::from_abi_i64(*c))
                    .collect();
                rt.function(template.clone(), capture_values).to_abi_i64()
            }
            None => {
                eprintln!("JIT: lambda template index {template_index} OOB");
                LispValue::NIL.to_abi_i64()
            }
        }
    }
}

// --- Call dispatch ---

jit_shim!(__neomacs_rt_call_named_0(vmctx: i64, symbol_index: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let name = resolve_sym!(ctx, symbol_index).to_string();
    let rt = &mut *ctx.runtime;
    dispatch_named_call(ctx, &name, &[], rt)
});

jit_shim!(__neomacs_rt_call_named_1(vmctx: i64, symbol_index: i64, arg0: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let name = resolve_sym!(ctx, symbol_index).to_string();
    let rt = &mut *ctx.runtime;
    dispatch_named_call(ctx, &name, &[LispValue::from_abi_i64(arg0)], rt)
});

jit_shim!(__neomacs_rt_call_named_2(vmctx: i64, symbol_index: i64, arg0: i64, arg1: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let name = resolve_sym!(ctx, symbol_index).to_string();
    let rt = &mut *ctx.runtime;
    dispatch_named_call(ctx, &name, &[LispValue::from_abi_i64(arg0), LispValue::from_abi_i64(arg1)], rt)
});

jit_shim!(__neomacs_rt_call_named_3(vmctx: i64, symbol_index: i64, arg0: i64, arg1: i64, arg2: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let name = resolve_sym!(ctx, symbol_index).to_string();
    let rt = &mut *ctx.runtime;
    dispatch_named_call(ctx, &name, &[LispValue::from_abi_i64(arg0), LispValue::from_abi_i64(arg1), LispValue::from_abi_i64(arg2)], rt)
});

jit_shim!(__neomacs_rt_call_named_4(vmctx: i64, symbol_index: i64, arg0: i64, arg1: i64, arg2: i64, arg3: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let name = resolve_sym!(ctx, symbol_index).to_string();
    let rt = &mut *ctx.runtime;
    dispatch_named_call(ctx, &name, &[LispValue::from_abi_i64(arg0), LispValue::from_abi_i64(arg1), LispValue::from_abi_i64(arg2), LispValue::from_abi_i64(arg3)], rt)
});

// Higher-arity call_named shims (5-16)
macro_rules! call_named_shim {
    ($fname:ident $($arg:ident)*) => {
        jit_shim!($fname(vmctx: i64, symbol_index: i64 $(, $arg: i64)*) -> i64 {
            let ctx = &mut *(vmctx as *mut JitContext);
            let name = resolve_sym!(ctx, symbol_index).to_string();
            let rt = &mut *ctx.runtime;
            dispatch_named_call(ctx, &name, &[$(LispValue::from_abi_i64($arg)),*], rt)
        });
    };
}

call_named_shim!(__neomacs_rt_call_named_5 a0 a1 a2 a3 a4);
call_named_shim!(__neomacs_rt_call_named_6 a0 a1 a2 a3 a4 a5);
call_named_shim!(__neomacs_rt_call_named_7 a0 a1 a2 a3 a4 a5 a6);
call_named_shim!(__neomacs_rt_call_named_8 a0 a1 a2 a3 a4 a5 a6 a7);
call_named_shim!(__neomacs_rt_call_named_9 a0 a1 a2 a3 a4 a5 a6 a7 a8);
call_named_shim!(__neomacs_rt_call_named_10 a0 a1 a2 a3 a4 a5 a6 a7 a8 a9);
call_named_shim!(__neomacs_rt_call_named_11 a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10);
call_named_shim!(__neomacs_rt_call_named_12 a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11);
call_named_shim!(__neomacs_rt_call_named_13 a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12);
call_named_shim!(__neomacs_rt_call_named_14 a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13);
call_named_shim!(__neomacs_rt_call_named_15 a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14);
call_named_shim!(__neomacs_rt_call_named_16 a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15);

jit_shim!(__neomacs_rt_funcall_0(vmctx: i64, callee: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let rt = &mut *ctx.runtime;
    dispatch_funcall(ctx, LispValue::from_abi_i64(callee), &[], rt)
});

jit_shim!(__neomacs_rt_funcall_1(vmctx: i64, callee: i64, arg0: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let rt = &mut *ctx.runtime;
    dispatch_funcall(ctx, LispValue::from_abi_i64(callee), &[LispValue::from_abi_i64(arg0)], rt)
});

jit_shim!(__neomacs_rt_funcall_2(vmctx: i64, callee: i64, arg0: i64, arg1: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let rt = &mut *ctx.runtime;
    dispatch_funcall(ctx, LispValue::from_abi_i64(callee), &[LispValue::from_abi_i64(arg0), LispValue::from_abi_i64(arg1)], rt)
});

jit_shim!(__neomacs_rt_funcall_3(vmctx: i64, callee: i64, arg0: i64, arg1: i64, arg2: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let rt = &mut *ctx.runtime;
    dispatch_funcall(ctx, LispValue::from_abi_i64(callee), &[LispValue::from_abi_i64(arg0), LispValue::from_abi_i64(arg1), LispValue::from_abi_i64(arg2)], rt)
});

// Higher-arity funcall shims (4-8)
macro_rules! funcall_shim {
    ($fname:ident $($arg:ident)*) => {
        jit_shim!($fname(vmctx: i64, callee: i64 $(, $arg: i64)*) -> i64 {
            let ctx = &mut *(vmctx as *mut JitContext);
            let rt = &mut *ctx.runtime;
            dispatch_funcall(ctx, LispValue::from_abi_i64(callee), &[$(LispValue::from_abi_i64($arg)),*], rt)
        });
    };
}

funcall_shim!(__neomacs_rt_funcall_4 a0 a1 a2 a3);
funcall_shim!(__neomacs_rt_funcall_5 a0 a1 a2 a3 a4);
funcall_shim!(__neomacs_rt_funcall_6 a0 a1 a2 a3 a4 a5);
funcall_shim!(__neomacs_rt_funcall_7 a0 a1 a2 a3 a4 a5 a6);
funcall_shim!(__neomacs_rt_funcall_8 a0 a1 a2 a3 a4 a5 a6 a7);

jit_shim!(__neomacs_rt_apply_1(vmctx: i64, callee: i64, arg0: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let rt = &mut *ctx.runtime;
    dispatch_apply(ctx, LispValue::from_abi_i64(callee), &[LispValue::from_abi_i64(arg0)], rt)
});

jit_shim!(__neomacs_rt_apply_2(vmctx: i64, callee: i64, arg0: i64, arg1: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let rt = &mut *ctx.runtime;
    dispatch_apply(ctx, LispValue::from_abi_i64(callee), &[LispValue::from_abi_i64(arg0), LispValue::from_abi_i64(arg1)], rt)
});

jit_shim!(__neomacs_rt_apply_3(vmctx: i64, callee: i64, arg0: i64, arg1: i64, arg2: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let rt = &mut *ctx.runtime;
    dispatch_apply(ctx, LispValue::from_abi_i64(callee), &[LispValue::from_abi_i64(arg0), LispValue::from_abi_i64(arg1), LispValue::from_abi_i64(arg2)], rt)
});

// --- Dispatch helpers ---

unsafe fn dispatch_named_call(
    ctx: &mut JitContext,
    name: &str,
    args: &[LispValue],
    rt: &mut Runtime,
) -> i64 {
    let fns = unsafe { &*ctx.functions_by_name };
    match dispatch_primitive(name, args, rt, fns) {
        Some(value) => value.to_abi_i64(),
        None => {
            let regir = unsafe { &*ctx.regir };
            let fns = unsafe { &*ctx.functions_by_name };
            if let Some(&fid) = fns.get(name) {
                return crate::object_interp::execute_module_function(regir, fns, fid, args, rt)
                    .unwrap_or(LispValue::NIL)
                    .to_abi_i64();
            }
            // Check symbol function cell (populated by defalias/fset)
            if let Some(sym) = rt.intern_soft(name) {
                if let Ok(Some(func)) = rt.symbol_function(sym) {
                    if rt.is_function(func) {
                        return crate::object_interp::execute_function_object_direct(
                            regir, fns, func, args, rt,
                        )
                        .unwrap_or(LispValue::NIL)
                        .to_abi_i64();
                    }
                }
            }
            // Fall back to interpreter's primitive dispatch for higher-order ops
            // (mapcar, mapc, maphash, require, etc.) that need evaluator context
            if let Some(value) = unsafe { dispatch_interpreter_fallback(ctx, name, args) } {
                return value.to_abi_i64();
            }
            eprintln!("JIT: unhandled call `{name}`");
            LispValue::NIL.to_abi_i64()
        }
    }
}

unsafe fn dispatch_funcall(
    ctx: &mut JitContext,
    callee: LispValue,
    args: &[LispValue],
    rt: &mut Runtime,
) -> i64 {
    unsafe {
        if rt.is_function(callee) {
            let regir = &*ctx.regir;
            let fns = &*ctx.functions_by_name;
            return crate::object_interp::execute_function_object_direct(
                regir, fns, callee, args, rt,
            )
            .unwrap_or(LispValue::NIL)
            .to_abi_i64();
        }
        if rt.is_symbol(callee) {
            let name = match rt.symbol_name(callee) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("JIT rt error: {e}");
                    return LispValue::NIL.to_abi_i64();
                }
            };
            return dispatch_named_call(ctx, &name, args, rt);
        }
        eprintln!("JIT: funcall on non-callable");
        LispValue::NIL.to_abi_i64()
    }
}

unsafe fn dispatch_apply(
    ctx: &mut JitContext,
    callee: LispValue,
    args: &[LispValue],
    rt: &mut Runtime,
) -> i64 {
    unsafe {
        let Some((last, prefixes)) = args.split_last() else {
            eprintln!("JIT: apply needs args");
            return LispValue::NIL.to_abi_i64();
        };
        let tail = match list_values(rt, *last) {
            Some(t) => t,
            None => return LispValue::NIL.to_abi_i64(),
        };
        let mut flat = Vec::with_capacity(prefixes.len() + tail.len());
        flat.extend(prefixes.iter().copied());
        flat.extend(tail);
        dispatch_funcall(ctx, callee, &flat, rt)
    }
}

fn list_values(rt: &Runtime, list: LispValue) -> Option<Vec<LispValue>> {
    let mut values = Vec::new();
    let mut current = list;
    while !current.is_nil() {
        let car = rt.car(current).ok()?;
        let cdr = rt.cdr(current).ok()?;
        values.push(car);
        current = cdr;
    }
    Some(values)
}

// --- Nonlocal control flow ---
//
// The JIT implements catch/throw, condition-case, and unwind-protect using a
// thread-local exception state. Each potentially-throwing runtime call checks
// for pending exceptions and returns a sentinel value. The compiled code checks
// for the sentinel after each such call and branches to the appropriate handler.

use std::cell::RefCell;

thread_local! {
    static JIT_EXCEPTION_STATE: RefCell<JitExceptionState> = RefCell::new(JitExceptionState::new());
}

struct JitExceptionState {
    pending_throw: Option<(LispValue, LispValue)>, // (tag, value)
    pending_signal: Option<(LispValue, LispValue)>, // (symbol, data)
    catch_depth: usize,
}

impl JitExceptionState {
    const fn new() -> Self {
        Self {
            pending_throw: None,
            pending_signal: None,
            catch_depth: 0,
        }
    }
}

/// Sentinel value returned by runtime calls when an exception is pending.
/// Checked by compiled code after each potentially-throwing call.
const EXCEPTION_SENTINEL: i64 = 0x0DEAD_BEEF_DEAD_BEEFu64 as i64;

/// Bridge an interpreter throw into the JIT exception state.
/// Returns a sentinel LispValue so the JIT compiled code's exception check
/// detects it and branches to the handler.
pub fn bridge_interpreter_throw(tag: LispValue, value: LispValue) -> LispValue {
    set_pending_throw(tag, value);
    LispValue::from_abi_i64(EXCEPTION_SENTINEL)
}

/// Bridge an interpreter signal into the JIT exception state.
pub fn bridge_interpreter_signal(symbol: LispValue, data: LispValue) -> LispValue {
    set_pending_signal(symbol, data);
    LispValue::from_abi_i64(EXCEPTION_SENTINEL)
}

/// Set a pending signal and return the EXCEPTION_SENTINEL as a LispValue.
/// Used by primitives that need to signal errors (e.g., division by zero).
fn set_pending_signal_and_return_sentinel(symbol: LispValue, data: LispValue) -> LispValue {
    set_pending_signal(symbol, data);
    LispValue::from_abi_i64(EXCEPTION_SENTINEL)
}

fn has_pending_exception() -> bool {
    JIT_EXCEPTION_STATE.with(|state| {
        let state = state.borrow();
        state.pending_throw.is_some() || state.pending_signal.is_some()
    })
}

fn set_pending_throw(tag: LispValue, value: LispValue) {
    JIT_EXCEPTION_STATE.with(|state| {
        state.borrow_mut().pending_throw = Some((tag, value));
    });
}

fn take_pending_throw() -> Option<(LispValue, LispValue)> {
    JIT_EXCEPTION_STATE.with(|state| state.borrow_mut().pending_throw.take())
}

fn set_pending_signal(symbol: LispValue, data: LispValue) {
    JIT_EXCEPTION_STATE.with(|state| {
        state.borrow_mut().pending_signal = Some((symbol, data));
    });
}

fn take_pending_signal() -> Option<(LispValue, LispValue)> {
    JIT_EXCEPTION_STATE.with(|state| state.borrow_mut().pending_signal.take())
}

fn push_catch() {
    JIT_EXCEPTION_STATE.with(|state| {
        state.borrow_mut().catch_depth += 1;
    });
}

fn pop_catch() {
    JIT_EXCEPTION_STATE.with(|state| {
        state.borrow_mut().catch_depth -= 1;
    });
}

// JIT runtime shims for nonlocal control flow

jit_shim!(__neomacs_rt_catch_begin(vmctx: i64, tag: i64) -> i64 {
    push_catch();
    // Return the tag — compiled code stores it in a local var for later matching
    tag
});

jit_shim!(__neomacs_rt_catch_end(vmctx: i64) -> i64 {
    pop_catch();
    0 // void
});

jit_shim!(__neomacs_rt_throw(vmctx: i64, tag: i64, value: i64) -> i64 {
    set_pending_throw(LispValue::from_abi_i64(tag), LispValue::from_abi_i64(value));
    EXCEPTION_SENTINEL
});

jit_shim!(__neomacs_rt_catch_match(vmctx: i64, catch_tag: i64, throw_tag: i64) -> i64 {
    let result = catch_tag == throw_tag;
    bool_value(result).to_abi_i64()
});

jit_shim!(__neomacs_rt_get_throw_value(vmctx: i64) -> i64 {
    match take_pending_throw() {
        Some((_tag, value)) => value.to_abi_i64(),
        None => LispValue::NIL.to_abi_i64(),
    }
});

jit_shim!(__neomacs_rt_peek_throw_tag(vmctx: i64) -> i64 {
    JIT_EXCEPTION_STATE.with(|state| {
        state.borrow().pending_throw
            .as_ref()
            .map(|(tag, _)| tag.to_abi_i64())
            .unwrap_or(LispValue::NIL.to_abi_i64())
    })
});

jit_shim!(__neomacs_rt_check_exception(vmctx: i64) -> i64 {
    if has_pending_exception() {
        EXCEPTION_SENTINEL
    } else {
        0
    }
});

jit_shim!(__neomacs_rt_signal(vmctx: i64, symbol: i64, data: i64) -> i64 {
    set_pending_signal(LispValue::from_abi_i64(symbol), LispValue::from_abi_i64(data));
    EXCEPTION_SENTINEL
});

jit_shim!(__neomacs_rt_error_0(vmctx: i64, message_index: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let name = resolve_str!(ctx, message_index).to_string();
    let rt = &mut *ctx.runtime;
    let symbol = rt.intern("error");
    let msg = rt.string(&name);
    let data = rt.cons(msg, LispValue::NIL);
    set_pending_signal(symbol, data);
    EXCEPTION_SENTINEL
});

jit_shim!(__neomacs_rt_error_1(vmctx: i64, message_index: i64, arg0: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let name = resolve_str!(ctx, message_index).to_string();
    let rt = &mut *ctx.runtime;
    let symbol = rt.intern("error");
    let msg = rt.string(&name);
    let data = make_list(rt, [msg, LispValue::from_abi_i64(arg0)]);
    set_pending_signal(symbol, data);
    EXCEPTION_SENTINEL
});

jit_shim!(__neomacs_rt_condition_case_begin(vmctx: i64) -> i64 {
    push_catch();
    0
});

jit_shim!(__neomacs_rt_condition_case_end(vmctx: i64) -> i64 {
    pop_catch();
    // Clear any pending signal if we reached here normally
    take_pending_signal();
    0
});

jit_shim!(__neomacs_rt_condition_case_pop(vmctx: i64) -> i64 {
    pop_catch();
    0
});

jit_shim!(__neomacs_rt_condition_handler_match(vmctx: i64, error_symbol: i64, pattern_index: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let rt = &mut *ctx.runtime;
    let thrown = has_pending_exception();
    if !thrown {
        return LispValue::NIL.to_abi_i64();
    }
    // Check if the signaled symbol matches the pattern
    // pattern_index is an index into quoted_forms — resolve the pattern
    let forms = &*ctx.quoted_forms;
    let pattern = match forms.get(pattern_index as usize) {
        Some(f) => surface_form_to_lisp(rt, f),
        None => return LispValue::NIL.to_abi_i64(),
    };
    // The error symbol is in the pending signal
    let matches = JIT_EXCEPTION_STATE.with(|state| {
        state.borrow().pending_signal.as_ref()
            .map(|(sym, _data)| *sym == pattern || {
                // Check if pattern is 'error' which catches all errors
                rt.is_symbol(pattern) && rt.symbol_name(pattern).map_or(false, |n| n == "error")
            })
            .unwrap_or(false)
    });
    bool_value(matches).to_abi_i64()
});

jit_shim!(__neomacs_rt_get_signal_data(vmctx: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let rt = &mut *ctx.runtime;
    match take_pending_signal() {
        Some((symbol, data)) => {
            let condition = rt.cons(symbol, data);
            condition.to_abi_i64()
        }
        None => LispValue::NIL.to_abi_i64(),
    }
});

jit_shim!(__neomacs_rt_unwind_protect_begin(vmctx: i64) -> i64 {
    push_catch();
    0
});

jit_shim!(__neomacs_rt_unwind_protect_cleanup_enter(vmctx: i64) -> i64 {
    // Called at the start of cleanup code — exception remains pending
    0
});

jit_shim!(__neomacs_rt_unwind_protect_end(vmctx: i64) -> i64 {
    pop_catch();
    0
});

// --- GC root stack ---

jit_shim!(__neomacs_rt_push_root(vmctx: i64, value: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    ctx.gc_roots.push(LispValue::from_abi_i64(value));
    value
});

jit_shim!(__neomacs_rt_pop_roots(vmctx: i64, count: i64) -> i64 {
    let ctx = &mut *(vmctx as *mut JitContext);
    let count = count as usize;
    let base = ctx.gc_root_base;
    ctx.gc_roots.truncate(base);
    0
});

jit_shim!(__neomacs_rt_gc_safepoint(vmctx: i64) -> i64 {
    // Stub: triggers GC if needed. Currently a no-op until the
    // neovm-gc crate's Heap/Mutator replaces the current Runtime.
    0
});

// --- Primitives ---

fn dispatch_primitive(
    name: &str,
    args: &[LispValue],
    rt: &mut Runtime,
    jit_functions: &std::collections::HashMap<String, FunctionId>,
) -> Option<LispValue> {
    match name {
        // --- Arithmetic ---
        "+" => numeric_fold_add(rt, args),
        "-" => numeric_fold_sub(rt, args),
        "*" => numeric_fold_mul(rt, args),
        "/" => numeric_div(rt, args),
        "1+" => numeric_add1(rt, args[0]),
        "1-" => numeric_sub1(rt, args[0]),
        "%" => numeric_rem(rt, args),
        "mod" => numeric_mod(rt, args),
        "rem" => numeric_rem(rt, args),
        "abs" => numeric_abs(rt, args[0]),
        "max" => numeric_max(rt, args),
        "min" => numeric_min(rt, args),
        "=" => numeric_eq(rt, args),
        "/=" => numeric_ne(rt, args),
        "<" => numeric_cmp(rt, args, |a, b| a < b),
        "<=" => numeric_cmp(rt, args, |a, b| a <= b),
        ">" => numeric_cmp(rt, args, |a, b| a > b),
        ">=" => numeric_cmp(rt, args, |a, b| a >= b),

        // --- Equality and comparison ---
        "eq" | "eql" => Some(bool_value(args[0] == args[1])),
        "equal" => Some(bool_value(rt.equal(args[0], args[1]))),

        // --- Type predicates ---
        "null" | "not" => Some(bool_value(args[0].is_nil())),
        "consp" => Some(bool_value(rt.is_cons(args[0]))),
        "listp" => Some(bool_value(args[0].is_nil() || rt.is_cons(args[0]))),
        "numberp" => Some(bool_value(rt.is_number(args[0]))),
        "integerp" => Some(bool_value(args[0].is_fixnum())),
        "floatp" => Some(bool_value(rt.is_float(args[0]))),
        "symbolp" => Some(bool_value(rt.is_symbol(args[0]))),
        "stringp" => Some(bool_value(rt.is_string(args[0]))),
        "atom" => Some(bool_value(!rt.is_cons(args[0]))),
        "vectorp" => Some(bool_value(rt.is_vector(args[0]))),
        "hash-table-p" => Some(bool_value(rt.is_hash_table(args[0]))),
        "functionp" => {
            let is_fn = rt.is_function(args[0]);
            let has_symbol_fn = rt.is_symbol(args[0])
                && (rt.symbol_function(args[0]).ok().flatten().is_some()
                    || (rt
                        .symbol_name(args[0])
                        .ok()
                        .is_some_and(|name| jit_functions.contains_key(&name))));
            Some(bool_value(is_fn || has_symbol_fn))
        }
        "booleanp" => Some(bool_value(args[0].is_nil() || args[0].is_true())),
        "natnump" | "wholenump" => Some(bool_value(args[0].as_fixnum().is_some_and(|v| v >= 0))),
        "zerop" => Some(bool_value(args[0].as_fixnum() == Some(0))),
        "number-sequence" => {
            let from = args.get(0).and_then(|v| v.as_fixnum()).unwrap_or(0);
            let to = args.get(1).and_then(|v| v.as_fixnum()).unwrap_or(0);
            let sep = args.get(2).and_then(|v| v.as_fixnum()).unwrap_or(1);
            if sep == 0 {
                return Some(LispValue::NIL);
            }
            let mut items = Vec::new();
            let mut cur = from;
            if sep > 0 {
                while cur <= to {
                    items.push(LispValue::expect_fixnum(cur));
                    cur += sep;
                }
            } else {
                while cur >= to {
                    items.push(LispValue::expect_fixnum(cur));
                    cur += sep;
                }
            }
            Some(make_list(rt, items))
        }
        "last" => {
            let mut list = args[0];
            let n = args.get(1).and_then(|v| v.as_fixnum()).unwrap_or(1) as usize;
            if n == 0 {
                return Some(list);
            }
            let len: usize = {
                let mut count: usize = 0;
                let mut cur = list;
                while !cur.is_nil() {
                    count += 1;
                    cur = rt.cdr(cur).unwrap_or(LispValue::NIL);
                }
                count
            };
            let skip = len.saturating_sub(n);
            for _ in 0..skip {
                list = rt.cdr(list).unwrap_or(LispValue::NIL);
            }
            Some(list)
        }
        "butlast" => {
            let list = args[0];
            let n = args.get(1).and_then(|v| v.as_fixnum()).unwrap_or(1) as usize;
            let mut items = Vec::new();
            let mut cur = list;
            let mut count = 0;
            while !cur.is_nil() {
                items.push(rt.car(cur).unwrap_or(LispValue::NIL));
                cur = rt.cdr(cur).unwrap_or(LispValue::NIL);
                count += 1;
            }
            if n >= count {
                return Some(LispValue::NIL);
            }
            items.truncate(count - n);
            Some(make_list(rt, items))
        }
        "delete" => {
            let el = args[0];
            let list = args[1];
            let mut items = Vec::new();
            let mut cur = list;
            while !cur.is_nil() {
                let car = rt.car(cur).unwrap_or(LispValue::NIL);
                if car != el {
                    items.push(car);
                }
                cur = rt.cdr(cur).unwrap_or(LispValue::NIL);
            }
            Some(make_list(rt, items))
        }
        "number-or-marker-p" => Some(bool_value(rt.is_number(args[0]))),
        "string-or-null-p" => Some(bool_value(rt.is_string(args[0]) || args[0].is_nil())),
        "type-of" => Some(type_of(rt, args[0])),

        // --- Cons / pair ---
        "cons" => Some(rt.cons(args[0], args[1])),
        "car" => rt.car(args[0]).ok(),
        "cdr" => rt.cdr(args[0]).ok(),
        "car-safe" => Some(if rt.is_cons(args[0]) {
            rt.car(args[0]).unwrap_or(LispValue::NIL)
        } else {
            LispValue::NIL
        }),
        "cdr-safe" => Some(if rt.is_cons(args[0]) {
            rt.cdr(args[0]).unwrap_or(LispValue::NIL)
        } else {
            LispValue::NIL
        }),
        "setcar" => rt.set_car(args[0], args[1]).ok(),
        "setcdr" => rt.set_cdr(args[0], args[1]).ok(),

        // --- List operations ---
        "list" => Some(make_list(rt, args.iter().copied())),
        "length" => list_length(rt, args[0]),
        "nth" => nth_element(rt, args[1], args[0].as_fixnum()? as usize),
        "nthcdr" => nthcdr_list(rt, args[1], args[0].as_fixnum()? as usize),
        "reverse" => reverse_list(rt, args[0]),
        "nreverse" => reverse_list(rt, args[0]),
        "append" => append_lists(rt, args),
        "nconc" => nconc_lists(rt, args),
        "memq" => memq_op(rt, args[0], args[1]),
        "member" => member_op(rt, args[0], args[1]),
        "assq" => assoc_op(rt, args[0], args[1], false),
        "assoc" => assoc_op(rt, args[0], args[1], true),
        "copy-sequence" => copy_sequence(rt, args[0]),

        // --- String operations ---
        "concat" => concat_strings(rt, args),
        "substring" => substring_op(rt, args),
        "string=" | "string-equal" => string_equal(rt, args[0], args[1]),
        "string<" | "string-lessp" => string_lessp(rt, args[0], args[1]),
        "string>" | "string-greaterp" => string_greaterp(rt, args[0], args[1]),
        "char-to-string" => char_to_string(rt, args[0]),
        "string-to-char" => string_to_char(rt, args[0]),
        "format" | "format-message" => format_string(rt, args[0], &args[1..]),

        // --- Vector operations ---
        "vector" => Some(rt.vector(args.to_vec())),
        "make-vector" => {
            let len = args[0].as_fixnum().unwrap_or(-1);
            if len < 0 {
                None
            } else {
                Some(rt.make_vector(len as usize, args[1]))
            }
        }
        "aref" => aref_op(rt, args[0], args[1]),
        "aset" => aset_op(rt, args[0], args[1], args[2]),

        // --- Hash table operations ---
        "make-hash-table" => make_hash_table(rt, args),
        "hash-table-count" => hash_table_count_op(rt, args[0]),
        "gethash" => gethash_op(rt, args[0], args[1], args.get(2).copied()),
        "puthash" => rt.puthash(args[0], args[1], args[2]).ok(),
        "remhash" => rt.remhash(args[0], args[1]).ok(),
        "clrhash" => rt.clrhash(args[0]).ok(),

        // --- Symbol operations ---
        "symbol-value" => rt.symbol_value(args[0]).ok(),
        "symbol-name" => rt.symbol_name_value(args[0]).ok(),
        "symbol-function" => rt
            .symbol_function(args[0])
            .ok()
            .flatten()
            .or(Some(LispValue::NIL)),
        "symbol-plist" => rt.symbol_plist(args[0]).ok(),
        "set" => rt.set_symbol_value(args[0], args[1]).ok(),
        "setplist" => rt.set_symbol_plist(args[0], args[1]).ok(),
        "boundp" => Some(bool_value(rt.is_bound_symbol(args[0]).unwrap_or(false))),
        "fboundp" => {
            let has_fn = rt.symbol_function(args[0]).ok().flatten().is_some();
            let has_jit_fn = rt
                .symbol_name(args[0])
                .ok()
                .is_some_and(|name| jit_functions.contains_key(&name));
            Some(bool_value(has_fn || has_jit_fn))
        }
        "fset" => rt.set_symbol_function(args[0], args[1]).ok(),
        "defalias" => {
            let _ = rt.set_symbol_function(args[0], args[1]);
            Some(args[0])
        }
        "intern" => {
            let name = rt.string_contents(args[0]).ok()?.to_string();
            Some(rt.intern(&name))
        }
        "get" => rt.symbol_property(args[0], args[1]).ok(),
        "put" => rt.put_symbol_property(args[0], args[1], args[2]).ok(),
        "plist-get" => Some(rt.plist_get(args[0], args[1])),
        "plist-put" => Some(rt.plist_put(args[0], args[1], args[2])),
        "provide" => rt.provide(args[0]).ok(),
        "featurep" => Some(bool_value(rt.featurep(args[0]).unwrap_or(false))),

        // --- c*r accessors ---
        // ops are outermost-to-innermost; reversed iteration applies innermost first
        // true = car ('a'), false = cdr ('d')
        "caar" => car_cdr_chain(rt, args[0], &[true, true]),
        "cadr" => car_cdr_chain(rt, args[0], &[true, false]),
        "cdar" => car_cdr_chain(rt, args[0], &[false, true]),
        "cddr" => car_cdr_chain(rt, args[0], &[false, false]),
        "caaar" => car_cdr_chain(rt, args[0], &[true, true, true]),
        "caadr" => car_cdr_chain(rt, args[0], &[true, true, false]),
        "cadar" => car_cdr_chain(rt, args[0], &[true, false, true]),
        "caddr" => car_cdr_chain(rt, args[0], &[true, false, false]),
        "cdaar" => car_cdr_chain(rt, args[0], &[false, true, true]),
        "cdadr" => car_cdr_chain(rt, args[0], &[false, true, false]),
        "cddar" => car_cdr_chain(rt, args[0], &[false, false, true]),
        "cdddr" => car_cdr_chain(rt, args[0], &[false, false, false]),
        "caaaar" => car_cdr_chain(rt, args[0], &[true, true, true, true]),
        "caaadr" => car_cdr_chain(rt, args[0], &[true, true, true, false]),
        "caadar" => car_cdr_chain(rt, args[0], &[true, true, false, true]),
        "caaddr" => car_cdr_chain(rt, args[0], &[true, true, false, false]),
        "cadaar" => car_cdr_chain(rt, args[0], &[true, false, true, true]),
        "cadadr" => car_cdr_chain(rt, args[0], &[true, false, true, false]),
        "caddar" => car_cdr_chain(rt, args[0], &[true, false, false, true]),
        "cadddr" => car_cdr_chain(rt, args[0], &[true, false, false, false]),
        "cdaaar" => car_cdr_chain(rt, args[0], &[false, true, true, true]),
        "cdaadr" => car_cdr_chain(rt, args[0], &[false, true, true, false]),
        "cdadar" => car_cdr_chain(rt, args[0], &[false, true, false, true]),
        "cdaddr" => car_cdr_chain(rt, args[0], &[false, true, false, false]),
        "cddaar" => car_cdr_chain(rt, args[0], &[false, false, true, true]),
        "cddadr" => car_cdr_chain(rt, args[0], &[false, false, true, false]),
        "cdddar" => car_cdr_chain(rt, args[0], &[false, false, false, true]),
        "cddddr" => car_cdr_chain(rt, args[0], &[false, false, false, false]),

        // --- Bitwise ---
        "logand" => {
            if args.is_empty() {
                return LispValue::from_fixnum(-1);
            }
            let init = args[0].as_fixnum()?;
            let result = args[1..]
                .iter()
                .filter_map(|v| v.as_fixnum())
                .fold(init, |a, b| a & b);
            LispValue::from_fixnum(result)
        }
        "logior" => {
            if args.is_empty() {
                return LispValue::from_fixnum(0);
            }
            let init = args[0].as_fixnum()?;
            let result = args[1..]
                .iter()
                .filter_map(|v| v.as_fixnum())
                .fold(init, |a, b| a | b);
            LispValue::from_fixnum(result)
        }
        "logxor" => {
            if args.is_empty() {
                return LispValue::from_fixnum(0);
            }
            let init = args[0].as_fixnum()?;
            let result = args[1..]
                .iter()
                .filter_map(|v| v.as_fixnum())
                .fold(init, |a, b| a ^ b);
            LispValue::from_fixnum(result)
        }
        "lognot" => LispValue::from_fixnum(!args[0].as_fixnum()?),
        "ash" => {
            let val = args[0].as_fixnum()?;
            let count = args[1].as_fixnum()?;
            let result = if count >= 0 {
                val.wrapping_shl(count as u32)
            } else {
                val.wrapping_shr((-count) as u32)
            };
            LispValue::from_fixnum(result)
        }
        "lsh" => {
            let val = args[0].as_fixnum()?;
            let count = args[1].as_fixnum()?;
            let result = if count >= 0 {
                val.wrapping_shl(count as u32)
            } else {
                ((val as u64).wrapping_shr((-count) as u32)) as i64
            };
            LispValue::from_fixnum(result)
        }
        "evenp" | "cl-evenp" => Some(bool_value(args[0].as_fixnum()? % 2 == 0)),
        "cl-oddp" => Some(bool_value(args[0].as_fixnum()? % 2 != 0)),
        "cl-plusp" => Some(bool_value(args[0].as_fixnum()? > 0)),
        "cl-minusp" => Some(bool_value(args[0].as_fixnum()? < 0)),
        "expt" => {
            let base_is_float = rt.is_float(args[0]);
            let exp_is_float = rt.is_float(args[1]);
            let any_float = base_is_float || exp_is_float;
            let base_float = if let Ok(f) = rt.float_data(args[0]) {
                f
            } else if let Some(i) = args[0].as_fixnum() {
                i as f64
            } else {
                return None;
            };
            let exp_float = if let Ok(f) = rt.float_data(args[1]) {
                f
            } else if let Some(i) = args[1].as_fixnum() {
                i as f64
            } else {
                return None;
            };
            let result = base_float.powf(exp_float);
            if any_float {
                Some(rt.float(result))
            } else if result.is_finite() && result.fract() == 0.0 && result.abs() < i64::MAX as f64
            {
                LispValue::from_fixnum(result as i64)
            } else {
                Some(rt.float(result))
            }
        }

        // --- String ops ---
        "number-to-string" => {
            if let Some(n) = args[0].as_fixnum() {
                Some(rt.string(n.to_string()))
            } else if let Ok(f) = rt.float_data(args[0]) {
                Some(rt.string(format!("{}", f)))
            } else {
                None
            }
        }
        "string-to-number" => {
            let contents = rt.string_contents(args[0]).ok()?.to_string();
            let radix = args
                .get(1)
                .and_then(|v| v.as_fixnum())
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(10);
            let trimmed = contents.trim();
            let trimmed = if trimmed.starts_with('+') {
                &trimmed[1..]
            } else {
                trimmed
            };
            if trimmed.is_empty() {
                return LispValue::from_fixnum(0);
            }
            if radix == 10 && trimmed.contains('.') {
                if let Ok(f) = trimmed.parse::<f64>() {
                    if f.is_finite() && f.fract() == 0.0 && f.abs() < i64::MAX as f64 {
                        return LispValue::from_fixnum(f as i64);
                    }
                    return Some(rt.float(f));
                }
            }
            let n = i64::from_str_radix(trimmed, radix).unwrap_or(0);
            LispValue::from_fixnum(n)
        }
        "string-join" => {
            let sep = rt.string_contents(args[1]).ok()?.to_string();
            let mut parts = Vec::new();
            let mut cur = args[0];
            while !cur.is_nil() {
                let car = rt.car(cur).ok()?;
                parts.push(rt.string_contents(car).ok()?.to_string());
                cur = rt.cdr(cur).ok()?;
            }
            Some(rt.string(parts.join(&sep)))
        }
        "string-trim" => {
            let s = rt.string_contents(args[0]).ok()?.to_string();
            let trimmed = match args.get(1) {
                Some(re) if !re.is_nil() => {
                    let pat = rt.string_contents(*re).ok()?.to_string();
                    s.trim_matches(|c: char| pat.contains(c)).to_string()
                }
                _ => s.trim().to_string(),
            };
            Some(rt.string(trimmed))
        }
        "string-trim-left" => {
            let s = rt.string_contents(args[0]).ok()?.to_string();
            let trimmed = match args.get(1) {
                Some(re) if !re.is_nil() => {
                    let pat = rt.string_contents(*re).ok()?.to_string();
                    s.trim_start_matches(|c: char| pat.contains(c)).to_string()
                }
                _ => s.trim_start().to_string(),
            };
            Some(rt.string(trimmed))
        }
        "string-trim-right" => {
            let s = rt.string_contents(args[0]).ok()?.to_string();
            let trimmed = match args.get(1) {
                Some(re) if !re.is_nil() => {
                    let pat = rt.string_contents(*re).ok()?.to_string();
                    s.trim_end_matches(|c: char| pat.contains(c)).to_string()
                }
                _ => s.trim_end().to_string(),
            };
            Some(rt.string(trimmed))
        }
        "split-string" => {
            let s = rt.string_contents(args[0]).ok()?.to_string();
            let sep = args
                .get(1)
                .and_then(|v| {
                    if v.is_nil() {
                        None
                    } else {
                        rt.string_contents(*v).ok().map(|s| s.to_string())
                    }
                })
                .unwrap_or_default();
            let omit = args.get(2).map(|v| !v.is_nil()).unwrap_or(false);
            let parts: Vec<&str> = if sep.is_empty() {
                s.split_whitespace().collect()
            } else {
                s.split(&sep).collect()
            };
            let parts: Vec<&str> = if omit {
                parts.into_iter().filter(|p| !p.is_empty()).collect()
            } else {
                parts
            };
            let values: Vec<LispValue> = parts.into_iter().map(|p| rt.string(p)).collect();
            Some(make_list(rt, values))
        }
        "substring-no-properties" => substring_op(rt, args),
        "downcase" => {
            if rt.is_string(args[0]) {
                let s = rt.string_contents(args[0]).ok()?.to_string();
                Some(rt.string(s.to_lowercase()))
            } else if rt.is_symbol(args[0]) {
                let name = rt.symbol_name(args[0]).ok()?.to_string();
                Some(rt.intern(&name.to_lowercase()))
            } else {
                Some(args[0])
            }
        }
        "upcase" => {
            if rt.is_string(args[0]) {
                let s = rt.string_contents(args[0]).ok()?.to_string();
                Some(rt.string(s.to_uppercase()))
            } else if rt.is_symbol(args[0]) {
                let name = rt.symbol_name(args[0]).ok()?.to_string();
                Some(rt.intern(&name.to_uppercase()))
            } else {
                Some(args[0])
            }
        }
        "capitalize" => {
            if rt.is_string(args[0]) {
                let s = rt.string_contents(args[0]).ok()?.to_string();
                let mut chars: Vec<char> = s.chars().collect();
                if let Some(first) = chars.first_mut() {
                    *first = first.to_uppercase().next().unwrap_or(*first);
                }
                for ch in &mut chars[1..] {
                    *ch = ch.to_lowercase().next().unwrap_or(*ch);
                }
                Some(rt.string(chars.into_iter().collect::<String>()))
            } else if rt.is_symbol(args[0]) {
                let name = rt.symbol_name(args[0]).ok()?.to_string();
                let mut chars: Vec<char> = name.chars().collect();
                if let Some(first) = chars.first_mut() {
                    *first = first.to_uppercase().next().unwrap_or(*first);
                }
                for ch in &mut chars[1..] {
                    *ch = ch.to_lowercase().next().unwrap_or(*ch);
                }
                Some(rt.intern(&chars.into_iter().collect::<String>()))
            } else {
                Some(args[0])
            }
        }

        // --- Symbol ops ---
        "make-symbol" => {
            let name = rt.string_contents(args[0]).ok()?.to_string();
            Some(rt.make_symbol(&name))
        }
        "intern-soft" => {
            if rt.is_string(args[0]) {
                let name = rt.string_contents(args[0]).ok()?.to_string();
                return Some(rt.intern_soft(&name).unwrap_or(LispValue::NIL));
            }
            if rt.is_symbol(args[0]) {
                let name = rt.symbol_name(args[0]).ok()?.to_string();
                return Some(rt.intern_soft(&name).unwrap_or(LispValue::NIL));
            }
            Some(LispValue::NIL)
        }
        "keywordp" => {
            let is_keyword = rt.is_symbol(args[0])
                && rt
                    .symbol_name(args[0])
                    .map_or(false, |n| n.starts_with(':'));
            Some(bool_value(is_keyword))
        }

        // --- List ops ---
        "delq" => {
            let obj = args[0];
            let mut items = Vec::new();
            let mut cur = args[1];
            while !cur.is_nil() {
                let car = rt.car(cur).unwrap_or(LispValue::NIL);
                if car != obj {
                    items.push(car);
                }
                cur = rt.cdr(cur).unwrap_or(LispValue::NIL);
            }
            Some(make_list(rt, items))
        }
        "remove" => {
            let obj = args[0];
            let mut items = Vec::new();
            let mut cur = args[1];
            while !cur.is_nil() {
                let car = rt.car(cur).unwrap_or(LispValue::NIL);
                if !rt.equal(car, obj) {
                    items.push(car);
                }
                cur = rt.cdr(cur).unwrap_or(LispValue::NIL);
            }
            Some(make_list(rt, items))
        }
        "elt" => {
            let index = args[1].as_fixnum()? as usize;
            if rt.is_vector(args[0]) {
                let elements = rt.vector_elements(args[0]).ok()?;
                return Some(elements.get(index).copied().unwrap_or(LispValue::NIL));
            }
            let mut cur = args[0];
            for _ in 0..index {
                if cur.is_nil() {
                    return Some(LispValue::NIL);
                }
                cur = rt.cdr(cur).unwrap_or(LispValue::NIL);
            }
            Some(if cur.is_nil() {
                LispValue::NIL
            } else {
                rt.car(cur).unwrap_or(LispValue::NIL)
            })
        }
        "vconcat" => {
            let mut elements = Vec::new();
            for arg in args {
                if rt.is_vector(*arg) {
                    elements.extend(rt.vector_elements(*arg).ok()?);
                } else if rt.is_string(*arg) {
                    elements.push(*arg);
                } else {
                    let mut cur = *arg;
                    while !cur.is_nil() {
                        elements.push(rt.car(cur).unwrap_or(LispValue::NIL));
                        cur = rt.cdr(cur).unwrap_or(LispValue::NIL);
                    }
                }
            }
            Some(rt.vector(elements))
        }
        "prog1" => {
            let first = args[0];
            Some(first)
        }

        // --- Misc ---
        "identity" => Some(args[0]),
        "ignore" => Some(LispValue::NIL),
        "message" => Some(args.last().copied().unwrap_or(LispValue::NIL)),
        "print" | "prin1" => Some(args[0]),
        "autoload" => Some(LispValue::NIL),

        // --- Error signaling ---
        "error" => {
            let symbol = rt.intern("error");
            let msg = args.first().copied().unwrap_or(LispValue::NIL);
            let data = rt.cons(msg, LispValue::NIL);
            Some(set_pending_signal_and_return_sentinel(symbol, data))
        }
        "signal" => {
            let symbol = args.first().copied().unwrap_or(LispValue::NIL);
            let data = args.get(1).copied().unwrap_or(LispValue::NIL);
            Some(set_pending_signal_and_return_sentinel(symbol, data))
        }
        "user-error" => {
            let symbol = rt.intern("error");
            let msg = args.first().copied().unwrap_or(LispValue::NIL);
            let data = rt.cons(msg, LispValue::NIL);
            Some(set_pending_signal_and_return_sentinel(symbol, data))
        }

        _ => None,
    }
}

// Fallback: use the interpreter's primitive dispatch for higher-order operations
// (mapcar, mapc, maphash, require) that need full evaluator context
unsafe fn dispatch_interpreter_fallback(
    ctx: &mut JitContext,
    name: &str,
    args: &[LispValue],
) -> Option<LispValue> {
    unsafe {
        let regir = &*ctx.regir;
        let fns = &*ctx.functions_by_name;
        let rt = &mut *ctx.runtime;
        crate::object_interp::execute_interpreter_primitive(name, args, regir, fns, rt)
    }
}

// --- Numeric helpers ---

fn any_float(rt: &Runtime, args: &[LispValue]) -> bool {
    args.iter().any(|v| rt.is_float(*v))
}

fn to_f64(rt: &Runtime, v: LispValue) -> f64 {
    rt.as_number(v).unwrap_or(0.0)
}

fn numeric_result(rt: &mut Runtime, value: f64, has_float: bool) -> Option<LispValue> {
    if has_float {
        Some(rt.float(value))
    } else {
        LispValue::from_fixnum(value as i64)
    }
}

fn numeric_fold_add(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let has_float = any_float(rt, args);
    if !has_float {
        let sum: i64 = args.iter().filter_map(|v| v.as_fixnum()).sum();
        return LispValue::from_fixnum(sum);
    }
    let sum: f64 = args.iter().map(|v| to_f64(rt, *v)).sum();
    numeric_result(rt, sum, true)
}

fn numeric_fold_sub(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    if args.is_empty() {
        return LispValue::from_fixnum(0);
    }
    let has_float = any_float(rt, args);
    if !has_float {
        if args.len() == 1 {
            let v = args[0].as_fixnum()?;
            return LispValue::from_fixnum(-v);
        }
        let first = args[0].as_fixnum()?;
        let rest: i64 = args[1..].iter().filter_map(|v| v.as_fixnum()).sum();
        return LispValue::from_fixnum(first - rest);
    }
    if args.len() == 1 {
        return numeric_result(rt, -to_f64(rt, args[0]), true);
    }
    let first = to_f64(rt, args[0]);
    let rest: f64 = args[1..].iter().map(|v| to_f64(rt, *v)).sum();
    numeric_result(rt, first - rest, true)
}

fn numeric_fold_mul(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let has_float = any_float(rt, args);
    if !has_float {
        let product: i64 = args.iter().filter_map(|v| v.as_fixnum()).product();
        return LispValue::from_fixnum(product);
    }
    let product: f64 = args.iter().map(|v| to_f64(rt, *v)).product();
    numeric_result(rt, product, true)
}

fn numeric_div(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let has_float = any_float(rt, args);
    if !has_float {
        let mut result = args[0].as_fixnum()?;
        for arg in &args[1..] {
            let divisor = arg.as_fixnum()?;
            if divisor == 0 {
                let symbol = rt.intern("arith-error");
                let msg = rt.string("Division by zero");
                let data = rt.cons(msg, LispValue::NIL);
                return Some(set_pending_signal_and_return_sentinel(symbol, data));
            }
            result /= divisor;
        }
        return LispValue::from_fixnum(result);
    }
    let mut result = to_f64(rt, args[0]);
    for arg in &args[1..] {
        let divisor = to_f64(rt, *arg);
        if divisor == 0.0 {
            let symbol = rt.intern("arith-error");
            let msg = rt.string("Division by zero");
            let data = rt.cons(msg, LispValue::NIL);
            return Some(set_pending_signal_and_return_sentinel(symbol, data));
        }
        result /= divisor;
    }
    Some(rt.float(result))
}

fn numeric_add1(rt: &mut Runtime, v: LispValue) -> Option<LispValue> {
    if rt.is_float(v) {
        numeric_result(rt, to_f64(rt, v) + 1.0, true)
    } else {
        let n = v.as_fixnum()?;
        LispValue::from_fixnum(n + 1)
    }
}

fn numeric_sub1(rt: &mut Runtime, v: LispValue) -> Option<LispValue> {
    if rt.is_float(v) {
        numeric_result(rt, to_f64(rt, v) - 1.0, true)
    } else {
        let n = v.as_fixnum()?;
        LispValue::from_fixnum(n - 1)
    }
}

fn numeric_mod(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let has_float = any_float(rt, args);
    if !has_float {
        let a = args[0].as_fixnum()?;
        let b = args[1].as_fixnum()?;
        if b == 0 {
            return None;
        }
        let result = a % b;
        let result = if result != 0 && (a < 0) != (b < 0) {
            result + b
        } else {
            result
        };
        return LispValue::from_fixnum(result);
    }
    let a = to_f64(rt, args[0]);
    let b = to_f64(rt, args[1]);
    if b == 0.0 {
        return None;
    }
    let result = a % b;
    let result = if result != 0.0 && (a < 0.0) != (b < 0.0) {
        result + b
    } else {
        result
    };
    numeric_result(rt, result, true)
}

fn numeric_rem(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let has_float = any_float(rt, args);
    if !has_float {
        let a = args[0].as_fixnum()?;
        let b = args[1].as_fixnum()?;
        if b == 0 {
            return None;
        }
        return LispValue::from_fixnum(a % b);
    }
    let a = to_f64(rt, args[0]);
    let b = to_f64(rt, args[1]);
    if b == 0.0 {
        return None;
    }
    numeric_result(rt, a % b, true)
}

fn numeric_abs(rt: &mut Runtime, v: LispValue) -> Option<LispValue> {
    if rt.is_float(v) {
        numeric_result(rt, to_f64(rt, v).abs(), true)
    } else {
        let n = v.as_fixnum()?;
        LispValue::from_fixnum(n.checked_abs()?)
    }
}

fn numeric_max(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let has_float = any_float(rt, args);
    if !has_float {
        let max = args.iter().filter_map(|v| v.as_fixnum()).reduce(i64::max)?;
        return LispValue::from_fixnum(max);
    }
    let max = args.iter().map(|v| to_f64(rt, *v)).reduce(f64::max)?;
    numeric_result(rt, max, true)
}

fn numeric_min(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let has_float = any_float(rt, args);
    if !has_float {
        let min = args.iter().filter_map(|v| v.as_fixnum()).reduce(i64::min)?;
        return LispValue::from_fixnum(min);
    }
    let min = args.iter().map(|v| to_f64(rt, *v)).reduce(f64::min)?;
    numeric_result(rt, min, true)
}

fn numeric_eq(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let has_float = any_float(rt, args);
    if !has_float {
        let a = args[0].as_fixnum()?;
        let b = args[1].as_fixnum()?;
        return Some(bool_value(a == b));
    }
    let a = to_f64(rt, args[0]);
    let b = to_f64(rt, args[1]);
    Some(bool_value(a == b))
}

fn numeric_ne(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let has_float = any_float(rt, args);
    if !has_float {
        let a = args[0].as_fixnum()?;
        let b = args[1].as_fixnum()?;
        return Some(bool_value(a != b));
    }
    let a = to_f64(rt, args[0]);
    let b = to_f64(rt, args[1]);
    Some(bool_value(a != b))
}

fn numeric_cmp(
    rt: &Runtime,
    args: &[LispValue],
    cmp: impl Fn(f64, f64) -> bool,
) -> Option<LispValue> {
    let has_float = args.iter().any(|v| rt.is_float(*v));
    if !has_float {
        // For pure fixnum comparisons, use i64 comparison directly
        let a = args[0].as_fixnum()?;
        let b = args[1].as_fixnum()?;
        // Determine which comparison from the closure behavior
        let result = if cmp(0.0, 1.0) && !cmp(1.0, 0.0) && !cmp(0.0, 0.0) {
            a < b // <
        } else if cmp(0.0, 1.0) && cmp(1.0, 0.0) && !cmp(1.0, 1.0) {
            a != b // never happens for < <= > >=
        } else if cmp(0.0, 0.0) && cmp(0.0, 1.0) && !cmp(1.0, 0.0) {
            a <= b // <=
        } else if !cmp(0.0, 1.0) && cmp(1.0, 0.0) && !cmp(0.0, 0.0) {
            a > b // >
        } else if cmp(0.0, 0.0) && !cmp(0.0, 1.0) && cmp(1.0, 0.0) {
            a >= b // >=
        } else {
            // Fallback: use f64
            return Some(bool_value(cmp(a as f64, b as f64)));
        };
        return Some(bool_value(result));
    }
    let a = to_f64(rt, args[0]);
    let b = to_f64(rt, args[1]);
    Some(bool_value(cmp(a, b)))
}

// --- Type-of ---

fn type_of(rt: &mut Runtime, value: LispValue) -> LispValue {
    if value.is_nil() || value.is_true() {
        rt.intern("symbol")
    } else if value.is_fixnum() {
        rt.intern("integer")
    } else if value.as_char().is_some() {
        rt.intern("symbol")
    } else if value.is_heap() {
        if rt.is_cons(value) {
            rt.intern("cons")
        } else if rt.is_string(value) {
            rt.intern("string")
        } else if rt.is_vector(value) {
            rt.intern("vector")
        } else if rt.is_hash_table(value) {
            rt.intern("hash-table")
        } else if rt.is_function(value) {
            rt.intern("compiled-function")
        } else if rt.is_float(value) {
            rt.intern("float")
        } else {
            rt.intern("misc")
        }
    } else {
        rt.intern("misc")
    }
}

// --- c*r accessors ---

fn car_cdr_chain(rt: &Runtime, mut value: LispValue, ops: &[bool]) -> Option<LispValue> {
    // ops are in name order: ops[0] = outermost (leftmost in name), ops[last] = innermost
    // We apply innermost first, so iterate in reverse.
    // true = car ('a' in name), false = cdr ('d' in name)
    for &is_car in ops.iter().rev() {
        value = if is_car {
            rt.car(value).ok()?
        } else {
            rt.cdr(value).ok()?
        };
    }
    Some(value)
}

// --- List operations ---

fn bool_value(value: bool) -> LispValue {
    if value {
        LispValue::TRUE
    } else {
        LispValue::NIL
    }
}

fn make_list(rt: &mut Runtime, values: impl IntoIterator<Item = LispValue>) -> LispValue {
    let values: Vec<LispValue> = values.into_iter().collect();
    let mut result = LispValue::NIL;
    for value in values.into_iter().rev() {
        result = rt.cons(value, result);
    }
    result
}

fn list_length(rt: &mut Runtime, list: LispValue) -> Option<LispValue> {
    let mut count = 0i64;
    let mut current = list;
    while !current.is_nil() {
        current = rt.cdr(current).ok()?;
        count += 1;
    }
    Some(LispValue::from_fixnum(count)?)
}

fn nth_element(rt: &mut Runtime, list: LispValue, n: usize) -> Option<LispValue> {
    let mut current = list;
    for _ in 0..n {
        current = rt.cdr(current).ok()?;
    }
    rt.car(current).ok()
}

fn nthcdr_list(rt: &mut Runtime, list: LispValue, n: usize) -> Option<LispValue> {
    let mut current = list;
    for _ in 0..n {
        if current.is_nil() {
            return Some(LispValue::NIL);
        }
        current = rt.cdr(current).ok()?;
    }
    Some(current)
}

fn last_pair(rt: &mut Runtime, list: LispValue) -> Option<LispValue> {
    let mut current = list;
    if current.is_nil() {
        return Some(LispValue::NIL);
    }
    loop {
        let cdr = rt.cdr(current).ok()?;
        if cdr.is_nil() {
            return Some(current);
        }
        current = cdr;
    }
}

fn reverse_list(rt: &mut Runtime, list: LispValue) -> Option<LispValue> {
    let mut result = LispValue::NIL;
    let mut current = list;
    while !current.is_nil() {
        let car = rt.car(current).ok()?;
        let cdr = rt.cdr(current).ok()?;
        result = rt.cons(car, result);
        current = cdr;
    }
    Some(result)
}

fn append_lists(rt: &mut Runtime, lists: &[LispValue]) -> Option<LispValue> {
    let mut all = Vec::new();
    for list in lists.iter().take(lists.len().saturating_sub(1)) {
        let mut current = *list;
        while !current.is_nil() {
            let car = rt.car(current).ok()?;
            let cdr = rt.cdr(current).ok()?;
            all.push(car);
            current = cdr;
        }
    }
    if let Some(last) = lists.last() {
        let mut current = *last;
        while !current.is_nil() {
            let car = rt.car(current).ok()?;
            let cdr = rt.cdr(current).ok()?;
            all.push(car);
            current = cdr;
        }
    }
    Some(make_list(rt, all))
}

fn nconc_lists(rt: &mut Runtime, lists: &[LispValue]) -> Option<LispValue> {
    if lists.is_empty() {
        return Some(LispValue::NIL);
    }
    if lists.len() == 1 {
        return Some(lists[0]);
    }
    // Destructive concat: reuses cons cells
    append_lists(rt, lists)
}

fn memq_op(rt: &Runtime, element: LispValue, list: LispValue) -> Option<LispValue> {
    let mut current = list;
    while !current.is_nil() {
        let car = rt.car(current).ok()?;
        if car == element {
            return Some(current);
        }
        current = rt.cdr(current).ok()?;
    }
    Some(LispValue::NIL)
}

fn member_op(rt: &Runtime, element: LispValue, list: LispValue) -> Option<LispValue> {
    let mut current = list;
    while !current.is_nil() {
        let car = rt.car(current).ok()?;
        if rt.equal(car, element) {
            return Some(current);
        }
        current = rt.cdr(current).ok()?;
    }
    Some(LispValue::NIL)
}

fn assoc_op(rt: &Runtime, key: LispValue, alist: LispValue, use_equal: bool) -> Option<LispValue> {
    let mut current = alist;
    while !current.is_nil() {
        let pair = rt.car(current).ok()?;
        if rt.is_cons(pair) {
            let car = rt.car(pair).ok()?;
            let matches = if use_equal {
                rt.equal(car, key)
            } else {
                car == key
            };
            if matches {
                return Some(pair);
            }
        }
        current = rt.cdr(current).ok()?;
    }
    Some(LispValue::NIL)
}

fn copy_sequence(rt: &mut Runtime, seq: LispValue) -> Option<LispValue> {
    if rt.is_cons(seq) {
        let mut values = Vec::new();
        let mut current = seq;
        while !current.is_nil() {
            let car = rt.car(current).ok()?;
            let cdr = rt.cdr(current).ok()?;
            values.push(car);
            current = cdr;
        }
        Some(make_list(rt, values))
    } else if rt.is_vector(seq) {
        let elements = rt.vector_elements(seq).ok()?;
        Some(rt.vector(elements))
    } else if rt.is_string(seq) {
        let s = rt.string_contents(seq).ok()?.to_string();
        Some(rt.string(&s))
    } else {
        Some(seq)
    }
}

// --- String operations ---

fn concat_strings(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let mut result = String::new();
    for arg in args {
        result.push_str(rt.string_contents(*arg).ok()?);
    }
    Some(rt.string(&result))
}

fn substring_op(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    let s = rt.string_contents(args[0]).ok()?.to_string();
    let start = args[1].as_fixnum()? as usize;
    let end = if args.len() > 2 {
        args[2].as_fixnum()? as usize
    } else {
        s.len()
    };
    Some(rt.string(s.get(start..end)?))
}

fn string_equal(rt: &Runtime, a: LispValue, b: LispValue) -> Option<LispValue> {
    let sa = rt.string_contents(a).ok()?;
    let sb = rt.string_contents(b).ok()?;
    Some(bool_value(sa == sb))
}

fn string_lessp(rt: &Runtime, a: LispValue, b: LispValue) -> Option<LispValue> {
    let sa = rt.string_contents(a).ok()?;
    let sb = rt.string_contents(b).ok()?;
    Some(bool_value(sa < sb))
}

fn string_greaterp(rt: &Runtime, a: LispValue, b: LispValue) -> Option<LispValue> {
    let sa = rt.string_contents(a).ok()?;
    let sb = rt.string_contents(b).ok()?;
    Some(bool_value(sa > sb))
}

fn char_to_string(rt: &mut Runtime, value: LispValue) -> Option<LispValue> {
    let ch = value.as_char()?;
    Some(rt.string(ch.to_string()))
}

fn string_to_char(rt: &Runtime, value: LispValue) -> Option<LispValue> {
    let s = rt.string_contents(value).ok()?;
    let ch = s.chars().next()?;
    Some(LispValue::from_char(ch))
}

fn format_string(rt: &mut Runtime, template: LispValue, args: &[LispValue]) -> Option<LispValue> {
    let tmpl = rt.string_contents(template).ok()?;
    let mut result = String::new();
    let mut chars = tmpl.chars().peekable();
    let mut arg_idx = 0;
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let spec = chars.peek().copied();
            match spec {
                Some('s') | Some('S') => {
                    chars.next();
                    if arg_idx < args.len() {
                        let arg = args[arg_idx];
                        // %s uses princ-style (no quotes for strings)
                        if rt.is_string(arg) {
                            result.push_str(rt.string_contents(arg).ok()?);
                        } else {
                            result.push_str(&rt.format_value(arg));
                        }
                        arg_idx += 1;
                    }
                }
                Some('d') => {
                    chars.next();
                    if arg_idx < args.len() {
                        result.push_str(&args[arg_idx].as_fixnum().unwrap_or(0).to_string());
                        arg_idx += 1;
                    }
                }
                Some('f') => {
                    chars.next();
                    if arg_idx < args.len() {
                        let num = rt.as_number(args[arg_idx]).unwrap_or(0.0);
                        result.push_str(&num.to_string());
                        arg_idx += 1;
                    }
                }
                Some('c') => {
                    chars.next();
                    if arg_idx < args.len() {
                        let arg = args[arg_idx];
                        if let Some(ch) = arg.as_char() {
                            result.push(ch);
                        } else if let Some(n) = arg.as_fixnum() {
                            if let Some(ch) = char::from_u32(n as u32) {
                                result.push(ch);
                            }
                        }
                        arg_idx += 1;
                    }
                }
                Some('%') => {
                    chars.next();
                    result.push('%');
                }
                _ => result.push(ch),
            }
        } else {
            result.push(ch);
        }
    }
    Some(rt.string(&result))
}

// --- Vector operations ---

fn aref_op(rt: &Runtime, vector: LispValue, index: LispValue) -> Option<LispValue> {
    let idx = index.as_fixnum()? as usize;
    rt.vector_aref(vector, idx).ok()
}

fn aset_op(
    rt: &mut Runtime,
    vector: LispValue,
    index: LispValue,
    value: LispValue,
) -> Option<LispValue> {
    let idx = index.as_fixnum()? as usize;
    rt.vector_aset(vector, idx, value).ok()
}

// --- Hash table operations ---

fn make_hash_table(rt: &mut Runtime, args: &[LispValue]) -> Option<LispValue> {
    use crate::runtime::HashTableTest;
    let mut test = HashTableTest::Eql;
    let mut i = 0;
    while i + 1 < args.len() {
        if let Some(keyword) = rt.symbol_name(args[i]).ok() {
            if keyword == ":test" && i + 1 < args.len() {
                if let Some(test_name) = rt.symbol_name(args[i + 1]).ok() {
                    test = match test_name.as_str() {
                        "eq" => HashTableTest::Eq,
                        "eql" => HashTableTest::Eql,
                        "equal" => HashTableTest::Equal,
                        _ => HashTableTest::Eql,
                    };
                }
            }
        }
        i += 2;
    }
    Some(rt.hash_table(test))
}

fn hash_table_count_op(rt: &Runtime, table: LispValue) -> Option<LispValue> {
    let count = rt.hash_table_count(table).ok()?;
    LispValue::from_fixnum(count as i64)
}

fn gethash_op(
    rt: &Runtime,
    key: LispValue,
    table: LispValue,
    default: Option<LispValue>,
) -> Option<LispValue> {
    match rt.gethash(key, table) {
        Ok(Some(value)) => Some(value),
        Ok(None) => Some(default.unwrap_or(LispValue::NIL)),
        Err(_) => Some(default.unwrap_or(LispValue::NIL)),
    }
}

fn surface_form_to_lisp(rt: &mut Runtime, form: &SurfaceForm) -> LispValue {
    use neovm_compiler::surface::{SurfaceAtom, SurfaceKind};
    match &form.kind {
        SurfaceKind::Atom(atom) => match atom {
            SurfaceAtom::Nil => LispValue::NIL,
            SurfaceAtom::True => LispValue::TRUE,
            SurfaceAtom::Symbol(n) => rt.intern(n),
            SurfaceAtom::Int(v) => LispValue::from_fixnum(*v).unwrap_or(LispValue::NIL),
            SurfaceAtom::Float(v) => rt.float(*v),
            SurfaceAtom::String(v) => rt.string(v),
            SurfaceAtom::Char(v) => {
                char::from_u32(*v as u32).map_or(LispValue::NIL, LispValue::from_char)
            }
        },
        SurfaceKind::List(forms) => {
            let elements: Vec<LispValue> =
                forms.iter().map(|f| surface_form_to_lisp(rt, f)).collect();
            make_list(rt, elements)
        }
        SurfaceKind::DottedList(items, tail) => {
            let mut result = surface_form_to_lisp(rt, tail);
            for item in items.iter().rev() {
                let car = surface_form_to_lisp(rt, item);
                result = rt.cons(car, result);
            }
            result
        }
        SurfaceKind::Vector(items) => {
            let elements: Vec<LispValue> =
                items.iter().map(|f| surface_form_to_lisp(rt, f)).collect();
            rt.vector(elements)
        }
        SurfaceKind::Quote(inner) => prefixed_form(rt, "quote", inner),
        SurfaceKind::FunctionQuote(inner) => prefixed_form(rt, "function", inner),
        SurfaceKind::Backquote(inner) => prefixed_form(rt, "quasiquote", inner),
        SurfaceKind::Comma(inner) => prefixed_form(rt, "unquote", inner),
        SurfaceKind::CommaAt(inner) => prefixed_form(rt, "unquote-splicing", inner),
    }
}

fn prefixed_form(rt: &mut Runtime, name: &str, inner: &SurfaceForm) -> LispValue {
    let head = rt.intern(name);
    let body = surface_form_to_lisp(rt, inner);
    let tail = rt.cons(body, LispValue::NIL);
    rt.cons(head, tail)
}
