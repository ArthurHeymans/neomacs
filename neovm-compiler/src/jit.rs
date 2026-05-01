use cranelift_codegen::ir::{Signature, UserFuncName};
use cranelift_codegen::isa::CallConv;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};

use crate::clif::{ClifModuleBackend, ClifRuntimeAbi, ssa_to_clif_with_backend};
use crate::diagnostic::Diagnostic;
use crate::ids::FunctionId;
use crate::ssa::{SsaLambdaTemplate, SsaModule};
use crate::surface::SurfaceForm;

use lasso::Rodeo;

impl ClifModuleBackend for JITModule {
    fn declare_import(&mut self, name: &str, signature: &Signature) -> FuncId {
        self.declare_function(name, Linkage::Import, signature)
            .expect("JITModule::declare_function should not fail")
    }

    fn call_conv(&self) -> CallConv {
        self.isa().default_call_conv()
    }
}

pub struct JitCompiledModule {
    pub entry_code_ptr: Option<*const u8>,
    pub entry_arity: usize,
    pub functions: Vec<JitCompiledFunction>,
    pub runtime_tables: JitRuntimeTables,
    _jit: JITModule,
}

pub struct JitCompiledFunction {
    pub name: Option<String>,
    pub arity: usize,
    pub code_ptr: *const u8,
}

pub struct JitRuntimeTables {
    pub symbol_rodeo: Rodeo,
    pub string_rodeo: Rodeo,
    pub quoted_forms: Vec<SurfaceForm>,
    pub lambda_templates: Vec<SsaLambdaTemplate>,
}

pub struct JitCompileOutput {
    pub compiled: Result<JitCompiledModule, ()>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn compile_ssa_to_jit(ssa: &SsaModule) -> JitCompileOutput {
    let builder = JITBuilder::new(default_libcall_names())
        .expect("failed to create JITBuilder");
    compile_ssa_to_jit_with_builder(ssa, builder)
}

pub fn compile_ssa_to_jit_with_builder(ssa: &SsaModule, builder: JITBuilder) -> JitCompileOutput {
    let mut diagnostics = Vec::new();

    let entry_arity = ssa.entry.map_or(0, |id| {
        ssa.functions.get(id)
            .map(|f| f.lambda_list.required.len())
            .unwrap_or(0)
    });

    let jit = JITModule::new(builder);
    let mut runtime: ClifRuntimeAbi<JITModule> = ClifRuntimeAbi::from_module(jit);
    let call_conv = runtime.module().call_conv();

    // Phase 0: Register named SSA functions as local functions for direct JIT-to-JIT calls.
    // Only register functions with fixed arity (no &rest, no &optional).
    for (_fid, func) in ssa.functions.iter() {
        if let Some(name) = &func.name {
            if func.lambda_list.rest.is_none() && func.lambda_list.optional.is_empty() {
                let arity = func.lambda_list.required.len();
                runtime.register_local_function(name, arity, call_conv);
            }
        }
    }

    // Phase 1: Lower each SSA function using the shared JIT module backend
    let mut lowered: Vec<(FunctionId, Option<cranelift_codegen::ir::Function>)> = Vec::new();
    for (fid, func) in ssa.functions.iter() {
        let output = ssa_to_clif_with_backend(func, runtime);
        runtime = output.runtime;
        diagnostics.extend(output.diagnostics.iter().cloned());
        lowered.push((fid, output.function));
    }

    if diagnostics.iter().any(Diagnostic::is_error) {
        return JitCompileOutput {
            compiled: Err(()),
            diagnostics,
        };
    }

    // Extract runtime tables from the shared lowering runtime before consuming it
    let tables = runtime.extract_tables();

    // Extract the JIT module from the runtime
    let mut jit = runtime.into_module();

    // Phase 2: Declare and define local functions in the JIT module
    let mut jit_func_ids: Vec<(FunctionId, FuncId)> = Vec::new();
    for (ssa_fid, function) in &lowered {
        let Some(mut function) = function.clone() else {
            // Function has nonlocal control flow (catch/throw, condition-case, etc.)
            // and can't be JIT-compiled. It will be called through the interpreter.
            continue;
        };

        let name = ssa.functions[*ssa_fid]
            .name
            .clone()
            .unwrap_or_else(|| format!("__neovm_fn_{}", ssa_fid.as_u32()));

        let jit_fid = jit
            .declare_function(&name, Linkage::Local, &function.signature)
            .expect("JIT declare_function should not fail");

        function.name = UserFuncName::user(0, jit_fid.as_u32());

        let mut ctx = jit.make_context();
        ctx.func = function;
        jit.define_function(jit_fid, &mut ctx)
            .expect("JIT define_function should not fail");
        jit.clear_context(&mut ctx);

        jit_func_ids.push((*ssa_fid, jit_fid));
    }

    if diagnostics.iter().any(Diagnostic::is_error) {
        return JitCompileOutput {
            compiled: Err(()),
            diagnostics,
        };
    }

    // Phase 3: Finalize
    jit.finalize_definitions()
        .expect("JIT finalize_definitions should not fail");

    // Phase 4: Extract code pointers
    let mut functions = Vec::new();
    let mut entry_code_ptr = None;

    for (ssa_fid, jit_fid) in &jit_func_ids {
        let code_ptr = jit.get_finalized_function(*jit_fid);

        let name = ssa.functions[*ssa_fid].name.clone();
        let arity = ssa.functions[*ssa_fid].lambda_list.required.len();

        functions.push(JitCompiledFunction {
            name,
            arity,
            code_ptr,
        });

        if ssa.entry == Some(*ssa_fid) {
            entry_code_ptr = Some(code_ptr);
        }
    }

    JitCompileOutput {
        compiled: Ok(JitCompiledModule {
            entry_code_ptr,
            entry_arity,
            functions,
            runtime_tables: JitRuntimeTables {
                symbol_rodeo: tables.symbol_rodeo,
                string_rodeo: tables.string_rodeo,
                quoted_forms: tables.quoted_forms,
                lambda_templates: tables.lambda_templates,
            },
            _jit: jit,
        }),
        diagnostics,
    }
}
