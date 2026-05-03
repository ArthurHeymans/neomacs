use std::collections::HashMap;

use lasso::Rodeo;
use neovm_compiler::diagnostic::Diagnostic;
use neovm_compiler::ids::FunctionId;
use neovm_compiler::jit::compile_ssa_to_jit_with_builder;
use neovm_compiler::regir::RegModule;
use neovm_compiler::surface::SurfaceForm;

use crate::jit_rt::JitContext;
use crate::{ExecuteResult, LispValue, Runtime};

fn register_runtime_shims(builder: &mut cranelift_jit::JITBuilder) {
    use crate::jit_rt::*;
    // SAFETY: these extern "C" functions are defined in jit_rt.rs
    builder.symbol("__neomacs_rt_cons", __neomacs_rt_cons as *const u8);
    builder.symbol("__neomacs_rt_car", __neomacs_rt_car as *const u8);
    builder.symbol("__neomacs_rt_cdr", __neomacs_rt_cdr as *const u8);
    builder.symbol(
        "__neomacs_rt_make_lexical_cell",
        __neomacs_rt_make_lexical_cell as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_lexical_cell_get",
        __neomacs_rt_lexical_cell_get as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_lexical_cell_set",
        __neomacs_rt_lexical_cell_set as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_symbol_get",
        __neomacs_rt_symbol_get as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_symbol_set",
        __neomacs_rt_symbol_set as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_bind_dynamic",
        __neomacs_rt_bind_dynamic as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_unbind_dynamic",
        __neomacs_rt_unbind_dynamic as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_string_const",
        __neomacs_rt_string_const as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_float_const",
        __neomacs_rt_float_const as *const u8,
    );
    builder.symbol("__neomacs_rt_quote", __neomacs_rt_quote as *const u8);
    builder.symbol(
        "__neomacs_rt_function_quote",
        __neomacs_rt_function_quote as *const u8,
    );
    builder.symbol("__neomacs_rt_lambda_0", __neomacs_rt_lambda_0 as *const u8);
    builder.symbol("__neomacs_rt_lambda_1", __neomacs_rt_lambda_1 as *const u8);
    builder.symbol("__neomacs_rt_lambda_2", __neomacs_rt_lambda_2 as *const u8);
    builder.symbol("__neomacs_rt_lambda_3", __neomacs_rt_lambda_3 as *const u8);
    // call_named shims (0-16)
    builder.symbol(
        "__neomacs_rt_call_named_0",
        __neomacs_rt_call_named_0 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_1",
        __neomacs_rt_call_named_1 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_2",
        __neomacs_rt_call_named_2 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_3",
        __neomacs_rt_call_named_3 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_4",
        __neomacs_rt_call_named_4 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_5",
        __neomacs_rt_call_named_5 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_6",
        __neomacs_rt_call_named_6 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_7",
        __neomacs_rt_call_named_7 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_8",
        __neomacs_rt_call_named_8 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_9",
        __neomacs_rt_call_named_9 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_10",
        __neomacs_rt_call_named_10 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_11",
        __neomacs_rt_call_named_11 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_12",
        __neomacs_rt_call_named_12 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_13",
        __neomacs_rt_call_named_13 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_14",
        __neomacs_rt_call_named_14 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_15",
        __neomacs_rt_call_named_15 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_call_named_16",
        __neomacs_rt_call_named_16 as *const u8,
    );
    // funcall shims (0-8)
    builder.symbol(
        "__neomacs_rt_funcall_0",
        __neomacs_rt_funcall_0 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_funcall_1",
        __neomacs_rt_funcall_1 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_funcall_2",
        __neomacs_rt_funcall_2 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_funcall_3",
        __neomacs_rt_funcall_3 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_funcall_4",
        __neomacs_rt_funcall_4 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_funcall_5",
        __neomacs_rt_funcall_5 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_funcall_6",
        __neomacs_rt_funcall_6 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_funcall_7",
        __neomacs_rt_funcall_7 as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_funcall_8",
        __neomacs_rt_funcall_8 as *const u8,
    );
    // apply shims
    builder.symbol("__neomacs_rt_apply_1", __neomacs_rt_apply_1 as *const u8);
    builder.symbol("__neomacs_rt_apply_2", __neomacs_rt_apply_2 as *const u8);
    builder.symbol("__neomacs_rt_apply_3", __neomacs_rt_apply_3 as *const u8);
    // exception / nonlocal control flow shims
    builder.symbol(
        "__neomacs_rt_catch_begin",
        __neomacs_rt_catch_begin as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_catch_end",
        __neomacs_rt_catch_end as *const u8,
    );
    builder.symbol("__neomacs_rt_throw", __neomacs_rt_throw as *const u8);
    builder.symbol(
        "__neomacs_rt_catch_match",
        __neomacs_rt_catch_match as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_get_throw_value",
        __neomacs_rt_get_throw_value as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_peek_throw_tag",
        __neomacs_rt_peek_throw_tag as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_check_exception",
        __neomacs_rt_check_exception as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_condition_case_begin",
        __neomacs_rt_condition_case_begin as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_condition_case_end",
        __neomacs_rt_condition_case_end as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_condition_handler_match",
        __neomacs_rt_condition_handler_match as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_condition_case_pop",
        __neomacs_rt_condition_case_pop as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_get_signal_data",
        __neomacs_rt_get_signal_data as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_unwind_protect_begin",
        __neomacs_rt_unwind_protect_begin as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_unwind_protect_cleanup_enter",
        __neomacs_rt_unwind_protect_cleanup_enter as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_unwind_protect_end",
        __neomacs_rt_unwind_protect_end as *const u8,
    );
    // GC root stack shims
    builder.symbol(
        "__neomacs_rt_push_root",
        __neomacs_rt_push_root as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_pop_roots",
        __neomacs_rt_pop_roots as *const u8,
    );
    builder.symbol(
        "__neomacs_rt_gc_safepoint",
        __neomacs_rt_gc_safepoint as *const u8,
    );
}

pub struct JitExecuteArtifact {
    pub compile: neovm_compiler::CompileArtifact,
    pub result: ExecuteResult,
    pub runtime: Runtime,
}

pub fn execute_with_jit(
    name: impl Into<String>,
    text: impl Into<String>,
    args: &[i64],
) -> JitExecuteArtifact {
    let mut session = neovm_compiler::expand::CompilerSession::new();
    execute_with_jit_session(name, text, args, &mut session)
}

pub fn execute_with_jit_session(
    name: impl Into<String>,
    text: impl Into<String>,
    args: &[i64],
    session: &mut neovm_compiler::expand::CompilerSession,
) -> JitExecuteArtifact {
    let compile = neovm_compiler::compile_source_with_session(name, text, session);
    let mut diagnostics = compile.diagnostics.clone();
    let mut value = None;
    let mut runtime = Runtime::new();

    if !diagnostics.iter().any(Diagnostic::is_error) {
        let ssa = match &compile.ssa {
            Some(ssa) => ssa,
            None => {
                diagnostics.push(Diagnostic::error("JIT execution requires SSA module"));
                return JitExecuteArtifact {
                    compile,
                    result: ExecuteResult {
                        value: None,
                        diagnostics,
                    },
                    runtime,
                };
            }
        };

        let jit_output = {
            let mut builder =
                cranelift_jit::JITBuilder::new(cranelift_module::default_libcall_names())
                    .expect("failed to create JITBuilder");
            register_runtime_shims(&mut builder);
            compile_ssa_to_jit_with_builder(ssa, builder)
        };
        diagnostics.extend(jit_output.diagnostics);

        if !diagnostics.iter().any(Diagnostic::is_error) {
            let jit_module = match jit_output.compiled {
                Ok(m) => m,
                Err(()) => {
                    diagnostics.push(Diagnostic::error("JIT compilation failed"));
                    return JitExecuteArtifact {
                        compile,
                        result: ExecuteResult {
                            value: None,
                            diagnostics,
                        },
                        runtime,
                    };
                }
            };

            // Build the RegModule for interpreter fallback calls
            let regir = match &compile.regir {
                Some(regir) => regir.clone(),
                None => {
                    diagnostics.push(Diagnostic::error(
                        "JIT execution requires RegIR module for fallback",
                    ));
                    return JitExecuteArtifact {
                        compile,
                        result: ExecuteResult {
                            value: None,
                            diagnostics,
                        },
                        runtime,
                    };
                }
            };

            let functions_by_name = functions_by_name(&regir);

            // Use interners and tables from JIT compilation (matching the Spur indices)
            let rt_tables = &jit_module.runtime_tables;
            let mut symbols = rt_tables.symbol_rodeo.clone();
            let mut strings = rt_tables.string_rodeo.clone();
            let mut quoted_forms = rt_tables.quoted_forms.clone();
            let mut lambda_templates = rt_tables.lambda_templates.clone();

            let ctx = Box::new(JitContext {
                runtime: &mut runtime as *mut Runtime,
                symbols: &mut symbols as *mut Rodeo,
                strings: &mut strings as *mut Rodeo,
                quoted_forms: &mut quoted_forms as *mut Vec<SurfaceForm>,
                lambda_templates: &mut lambda_templates
                    as *mut Vec<neovm_compiler::ssa::SsaLambdaTemplate>,
                regir: &regir as *const RegModule as *mut RegModule,
                functions_by_name: &functions_by_name as *const HashMap<String, FunctionId>
                    as *mut HashMap<String, FunctionId>,
                gc_roots: Vec::new(),
                gc_root_base: 0,
            });
            let ctx_ptr = Box::into_raw(ctx);

            // Convert args to LispValue
            let lisp_args: Vec<LispValue> = match args
                .iter()
                .map(|v| LispValue::from_fixnum(*v))
                .collect::<Option<Vec<_>>>()
            {
                Some(args) => args,
                None => {
                    diagnostics.push(Diagnostic::error("JIT args must fit in LispValue fixnums"));
                    unsafe {
                        drop(Box::from_raw(ctx_ptr));
                    }
                    return JitExecuteArtifact {
                        compile,
                        result: ExecuteResult {
                            value: None,
                            diagnostics,
                        },
                        runtime,
                    };
                }
            };

            if let Some(code_ptr) = jit_module.entry_code_ptr {
                // JIT-compiled entry: call directly via function pointer
                let result = call_jit_entry_with_ptr(
                    code_ptr,
                    jit_module.entry_arity,
                    ctx_ptr as i64,
                    &lisp_args,
                );
                value = Some(result);
                unsafe {
                    drop(Box::from_raw(ctx_ptr));
                }
            } else {
                // Entry function has nonlocal control flow and can't be JIT-compiled.
                // Fall back to the RegIR interpreter.
                unsafe {
                    drop(Box::from_raw(ctx_ptr));
                }
                let interp_result = crate::object_interp::execute_module_with_args(
                    &regir,
                    &lisp_args,
                    &mut runtime,
                );
                diagnostics.extend(interp_result.diagnostics);
                value = interp_result.value;
            }
        }
    }

    JitExecuteArtifact {
        compile,
        result: ExecuteResult { value, diagnostics },
        runtime,
    }
}

fn call_jit_entry_with_ptr(
    code_ptr: *const u8,
    arity: usize,
    vmctx: i64,
    args: &[LispValue],
) -> LispValue {
    let arg_abi: Vec<i64> = args.iter().map(|a| a.to_abi_i64()).collect();

    let result = unsafe {
        match arity {
            0 => {
                let f: extern "C" fn(i64) -> i64 = std::mem::transmute(code_ptr);
                f(vmctx)
            }
            1 => {
                let f: extern "C" fn(i64, i64) -> i64 = std::mem::transmute(code_ptr);
                f(vmctx, arg_abi[0])
            }
            2 => {
                let f: extern "C" fn(i64, i64, i64) -> i64 = std::mem::transmute(code_ptr);
                f(vmctx, arg_abi[0], arg_abi[1])
            }
            3 => {
                let f: extern "C" fn(i64, i64, i64, i64) -> i64 = std::mem::transmute(code_ptr);
                f(vmctx, arg_abi[0], arg_abi[1], arg_abi[2])
            }
            4 => {
                let f: extern "C" fn(i64, i64, i64, i64, i64) -> i64 =
                    std::mem::transmute(code_ptr);
                f(vmctx, arg_abi[0], arg_abi[1], arg_abi[2], arg_abi[3])
            }
            _ => {
                eprintln!("JIT: unsupported arity {arity}");
                return LispValue::NIL;
            }
        }
    };

    LispValue::from_abi_i64(result)
}

fn functions_by_name(module: &RegModule) -> HashMap<String, FunctionId> {
    let mut map = HashMap::new();
    if let Some(entry) = module.entry {
        if let Some(function) = module.functions.get(entry) {
            if let Some(name) = &function.name {
                map.insert(name.clone(), entry);
            }
        }
    }
    for (id, function) in module.functions.iter() {
        if let Some(name) = &function.name {
            map.entry(name.clone()).or_insert(id);
        }
    }
    map
}
