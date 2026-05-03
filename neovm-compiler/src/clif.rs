use std::collections::HashMap;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    self, AbiParam, BlockArg, FuncRef, Function, InstBuilder, Signature, UserFuncName, types,
};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings;
use cranelift_codegen::verifier::verify_function;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{FuncId, Linkage, ModuleDeclarations};
use lasso::{Key, Rodeo, Spur};

use crate::diagnostic::Diagnostic;
use crate::ids::{BlockId, FunctionId, PrimaryMap, SafepointId, ValueId};
use crate::liveness::SsaSafepointLiveness;
use crate::ssa::{SsaConst, SsaFunction, SsaInstKind, SsaLambdaTemplate, SsaModule, SsaTerminator};
use crate::surface::SurfaceForm;

const TAG_BITS: u32 = 3;
const FIXNUM_TAG: i64 = 0b000;
const CHAR_TAG: i64 = 0b010;
const SPECIAL_TAG: i64 = 0b110;
const NIL_BITS: i64 = SPECIAL_TAG; // 6
const TRUE_BITS: i64 = (1 << TAG_BITS) | SPECIAL_TAG; // 14
const FIXNUM_MIN: i64 = i64::MIN >> TAG_BITS;
const FIXNUM_MAX: i64 = i64::MAX >> TAG_BITS;

#[derive(Clone, Debug)]
pub struct ClifRuntimeTables {
    pub symbol_rodeo: Rodeo,
    pub string_rodeo: Rodeo,
    pub quoted_forms: Vec<SurfaceForm>,
    pub lambda_templates: Vec<SsaLambdaTemplate>,
}

pub struct ClifModuleLowerOutput {
    pub functions: PrimaryMap<FunctionId, ClifLowerOutput>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct ClifLowerOutput<M: ClifModuleBackend = ModuleDeclarations> {
    pub function: Option<Function>,
    pub runtime: ClifRuntimeAbi<M>,
    pub safepoints: ClifSafepointTable,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn ssa_to_clif(function: &SsaFunction) -> ClifLowerOutput {
    ClifLowerer::<ModuleDeclarations>::new(function).lower()
}

pub fn ssa_to_clif_with_backend<M: ClifModuleBackend>(
    function: &SsaFunction,
    runtime: ClifRuntimeAbi<M>,
) -> ClifLowerOutput<M> {
    ClifLowerer::with_runtime(function, runtime).lower()
}

pub fn ssa_module_to_clif(module: &SsaModule) -> ClifModuleLowerOutput {
    let mut functions = PrimaryMap::new();
    let mut diagnostics = Vec::new();
    for (_, function) in module.functions.iter() {
        let lowered = ssa_to_clif(function);
        diagnostics.extend(lowered.diagnostics.iter().cloned());
        functions.push(lowered);
    }
    ClifModuleLowerOutput {
        functions,
        diagnostics,
    }
}

pub fn verify_clif(function: &Function) -> Vec<Diagnostic> {
    let flags = settings::Flags::new(settings::builder());
    verify_function(function, &flags)
        .err()
        .map(|error| vec![Diagnostic::error(error.to_string())])
        .unwrap_or_default()
}

pub fn dump_clif(function: &Function) -> String {
    format!("{}", function.display())
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClifSafepointTable {
    pub entries: PrimaryMap<SafepointId, ClifSafepoint>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClifSafepoint {
    pub call_inst: ir::Inst,
    pub kind: ClifRuntimeCallKind,
    pub live_roots: Vec<ClifLiveRoot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClifLiveRoot {
    pub ssa_value: ValueId,
    pub clif_value: ir::Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClifRuntimeCallKind {
    CallNamed { name: String, arity: usize },
    Funcall { arity: usize },
    Apply { arity: usize },
    SymbolGet { name: String },
    SymbolSet { name: String },
    BindDynamic { name: String },
    UnbindDynamic { count: usize },
    StringConst { value: String },
    FloatConst { bits: u64 },
    Quote { index: usize },
    FunctionQuote { index: usize },
    Lambda { index: usize, capture_count: usize },
    MakeLexicalCell,
    LexicalCellGet,
    LexicalCellSet,
    Cons,
    Car,
    Cdr,
    CatchBegin,
    CatchEnd,
    Throw,
    PeekThrowTag,
    GetThrowValue,
    CheckException,
    ConditionCaseBegin,
    ConditionCaseEnd,
    UnwindProtectBegin,
    UnwindProtectCleanupEnter,
    UnwindProtectEnd,
}

/// Abstraction over Cranelift module backends for declaring imported functions.
///
/// Implemented for `ModuleDeclarations` (IR dumping/tests) and `cranelift_jit::JITModule`
/// (native JIT compilation).
pub trait ClifModuleBackend {
    fn declare_import(&mut self, name: &str, signature: &Signature) -> FuncId;
    fn call_conv(&self) -> CallConv;
}

impl ClifModuleBackend for ModuleDeclarations {
    fn declare_import(&mut self, name: &str, signature: &Signature) -> FuncId {
        self.declare_function(name, Linkage::Import, signature)
            .expect("ModuleDeclarations::declare_function should not fail")
            .0
    }

    fn call_conv(&self) -> CallConv {
        CallConv::SystemV
    }
}

pub struct ClifRuntimeAbi<M: ClifModuleBackend = ModuleDeclarations> {
    module: M,
    symbols: Rodeo,
    strings: Rodeo,
    quoted_forms: Vec<SurfaceForm>,
    lambda_templates: Vec<SsaLambdaTemplate>,
    call_named_by_arity: HashMap<usize, FuncId>,
    funcall_by_arity: HashMap<usize, FuncId>,
    apply_by_arity: HashMap<usize, FuncId>,
    symbol_get: Option<FuncId>,
    symbol_set: Option<FuncId>,
    bind_dynamic: Option<FuncId>,
    unbind_dynamic: Option<FuncId>,
    string_const: Option<FuncId>,
    float_const: Option<FuncId>,
    quote: Option<FuncId>,
    function_quote: Option<FuncId>,
    lambda_by_capture_count: HashMap<usize, FuncId>,
    make_lexical_cell: Option<FuncId>,
    lexical_cell_get: Option<FuncId>,
    lexical_cell_set: Option<FuncId>,
    cons: Option<FuncId>,
    car: Option<FuncId>,
    cdr: Option<FuncId>,
    local_functions: HashMap<String, FuncId>,
    imported_names: Vec<String>,
    exception_funcs: HashMap<String, FuncId>,
}

impl<M: ClifModuleBackend + Default> Default for ClifRuntimeAbi<M> {
    fn default() -> Self {
        Self {
            module: M::default(),
            symbols: Rodeo::default(),
            strings: Rodeo::default(),
            quoted_forms: Vec::new(),
            lambda_templates: Vec::new(),
            call_named_by_arity: HashMap::new(),
            funcall_by_arity: HashMap::new(),
            apply_by_arity: HashMap::new(),
            symbol_get: None,
            symbol_set: None,
            bind_dynamic: None,
            unbind_dynamic: None,
            string_const: None,
            float_const: None,
            quote: None,
            function_quote: None,
            lambda_by_capture_count: HashMap::new(),
            make_lexical_cell: None,
            lexical_cell_get: None,
            lexical_cell_set: None,
            cons: None,
            car: None,
            cdr: None,
            local_functions: HashMap::new(),
            imported_names: Vec::new(),
            exception_funcs: HashMap::new(),
        }
    }
}

impl<M: ClifModuleBackend> ClifRuntimeAbi<M> {
    pub fn from_module(module: M) -> Self {
        Self {
            module,
            symbols: Rodeo::default(),
            strings: Rodeo::default(),
            quoted_forms: Vec::new(),
            lambda_templates: Vec::new(),
            call_named_by_arity: HashMap::new(),
            funcall_by_arity: HashMap::new(),
            apply_by_arity: HashMap::new(),
            symbol_get: None,
            symbol_set: None,
            bind_dynamic: None,
            unbind_dynamic: None,
            string_const: None,
            float_const: None,
            quote: None,
            function_quote: None,
            lambda_by_capture_count: HashMap::new(),
            make_lexical_cell: None,
            lexical_cell_get: None,
            lexical_cell_set: None,
            cons: None,
            car: None,
            cdr: None,
            local_functions: HashMap::new(),
            imported_names: Vec::new(),
            exception_funcs: HashMap::new(),
        }
    }

    pub fn into_module(self) -> M {
        self.module
    }

    pub fn extract_tables(&self) -> ClifRuntimeTables {
        ClifRuntimeTables {
            symbol_rodeo: self.symbols.clone(),
            string_rodeo: self.strings.clone(),
            quoted_forms: self.quoted_forms.clone(),
            lambda_templates: self.lambda_templates.clone(),
        }
    }

    /// Register a module-local function for direct JIT-to-JIT calls.
    /// The `declared_name` is the name used when declaring/defining the function in the backend module.
    pub fn register_local_function(&mut self, declared_name: &str, arity: usize, call_conv: CallConv) {
        let mut sig = Signature::new(call_conv);
        sig.params.push(AbiParam::new(types::I64)); // vmctx
        for _ in 0..arity {
            sig.params.push(AbiParam::new(types::I64));
        }
        sig.returns.push(AbiParam::new(types::I64));
        let id = self.module.declare_import(declared_name, &sig);
        self.local_functions.insert(declared_name.to_string(), id);
    }

    /// Get the FuncId for a registered local function, if any.
    pub fn local_function(&self, name: &str) -> Option<FuncId> {
        self.local_functions.get(name).copied()
    }

    pub fn module(&self) -> &M {
        &self.module
    }

    pub fn intern_symbol(&mut self, name: &str) -> Spur {
        self.symbols.get_or_intern(name)
    }

    pub fn symbol_key(&self, name: &str) -> Option<Spur> {
        self.symbols.get(name)
    }

    pub fn resolve_symbol(&self, symbol: Spur) -> &str {
        self.symbols.resolve(&symbol)
    }

    pub fn intern_string(&mut self, value: &str) -> Spur {
        self.strings.get_or_intern(value)
    }

    pub fn string_key(&self, value: &str) -> Option<Spur> {
        self.strings.get(value)
    }

    pub fn resolve_string(&self, string: Spur) -> &str {
        self.strings.resolve(&string)
    }

    pub fn intern_quoted_form(&mut self, form: SurfaceForm) -> usize {
        if let Some(index) = self
            .quoted_forms
            .iter()
            .position(|existing| existing == &form)
        {
            return index;
        }
        let index = self.quoted_forms.len();
        self.quoted_forms.push(form);
        index
    }

    pub fn quoted_forms(&self) -> &[SurfaceForm] {
        &self.quoted_forms
    }

    pub fn intern_lambda_template(&mut self, template: SsaLambdaTemplate) -> usize {
        if let Some(index) = self
            .lambda_templates
            .iter()
            .position(|existing| existing == &template)
        {
            return index;
        }
        let index = self.lambda_templates.len();
        self.lambda_templates.push(template);
        index
    }

    pub fn lambda_templates(&self) -> &[SsaLambdaTemplate] {
        &self.lambda_templates
    }

    pub fn imported_function_names(&self) -> Vec<&str> {
        self.imported_names.iter().map(|s| s.as_str()).collect()
    }

    fn exception_func(
        &mut self,
        name: &str,
        num_extra_args: usize,
        call_conv: CallConv,
    ) -> Result<RuntimeFuncImport, String> {
        if let Some(id) = self.exception_funcs.get(name).copied() {
            let mut sig = Signature::new(call_conv);
            sig.params.push(AbiParam::new(types::I64));
            for _ in 0..num_extra_args {
                sig.params.push(AbiParam::new(types::I64));
            }
            sig.returns.push(AbiParam::new(types::I64));
            return Ok(RuntimeFuncImport { id, signature: sig });
        }
        let mut sig = Signature::new(call_conv);
        sig.params.push(AbiParam::new(types::I64)); // vmctx
        for _ in 0..num_extra_args {
            sig.params.push(AbiParam::new(types::I64));
        }
        sig.returns.push(AbiParam::new(types::I64));
        let full_name = format!("__neomacs_rt_{name}");
        let id = self.module.declare_import(&full_name, &sig);
        self.imported_names.push(full_name);
        self.exception_funcs.insert(name.to_string(), id);
        Ok(RuntimeFuncImport { id, signature: sig })
    }

    fn call_named(
        &mut self,
        arity: usize,
        call_conv: CallConv,
    ) -> Result<RuntimeFuncImport, String> {
        if let Some(id) = self.call_named_by_arity.get(&arity).copied() {
            return Ok(RuntimeFuncImport {
                id,
                signature: call_named_signature(arity, call_conv),
            });
        }

        let name = format!("__neomacs_rt_call_named_{arity}");
        let signature = call_named_signature(arity, call_conv);
        let id = self.module.declare_import(&name, &signature);
        self.imported_names.push(name.clone());
        self.call_named_by_arity.insert(arity, id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn funcall(&mut self, arity: usize, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        if let Some(id) = self.funcall_by_arity.get(&arity).copied() {
            return Ok(RuntimeFuncImport {
                id,
                signature: indirect_call_signature(arity, call_conv),
            });
        }

        let name = format!("__neomacs_rt_funcall_{arity}");
        let signature = indirect_call_signature(arity, call_conv);
        let id = self.module.declare_import(&name, &signature);
        self.imported_names.push(name.clone());
        self.funcall_by_arity.insert(arity, id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn apply(&mut self, arity: usize, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        if let Some(id) = self.apply_by_arity.get(&arity).copied() {
            return Ok(RuntimeFuncImport {
                id,
                signature: indirect_call_signature(arity, call_conv),
            });
        }

        let name = format!("__neomacs_rt_apply_{arity}");
        let signature = indirect_call_signature(arity, call_conv);
        let id = self.module.declare_import(&name, &signature);
        self.imported_names.push(name.clone());
        self.apply_by_arity.insert(arity, id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn symbol_get(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = symbol_get_signature(call_conv);
        if let Some(id) = self.symbol_get {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self
            .module
            .declare_import("__neomacs_rt_symbol_get", &signature);
        self.imported_names.push("__neomacs_rt_symbol_get".to_string());
        self.symbol_get = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn symbol_set(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = symbol_set_signature(call_conv);
        if let Some(id) = self.symbol_set {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_symbol_set", &signature);
        self.imported_names.push("__neomacs_rt_symbol_set".to_string());
        self.symbol_set = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn bind_dynamic(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = bind_dynamic_signature(call_conv);
        if let Some(id) = self.bind_dynamic {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_bind_dynamic", &signature);
        self.imported_names.push("__neomacs_rt_bind_dynamic".to_string());
        self.bind_dynamic = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn unbind_dynamic(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = unbind_dynamic_signature(call_conv);
        if let Some(id) = self.unbind_dynamic {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_unbind_dynamic", &signature);
        self.imported_names.push("__neomacs_rt_unbind_dynamic".to_string());
        self.unbind_dynamic = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn string_const(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = indexed_runtime_signature(call_conv);
        if let Some(id) = self.string_const {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_string_const", &signature);
        self.imported_names.push("__neomacs_rt_string_const".to_string());
        self.string_const = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn float_const(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = indexed_runtime_signature(call_conv);
        if let Some(id) = self.float_const {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_float_const", &signature);
        self.imported_names.push("__neomacs_rt_float_const".to_string());
        self.float_const = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn quote(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = indexed_runtime_signature(call_conv);
        if let Some(id) = self.quote {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_quote", &signature);
        self.imported_names.push("__neomacs_rt_quote".to_string());
        self.quote = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn function_quote(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = indexed_runtime_signature(call_conv);
        if let Some(id) = self.function_quote {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_function_quote", &signature);
        self.imported_names.push("__neomacs_rt_function_quote".to_string());
        self.function_quote = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn lambda(
        &mut self,
        capture_count: usize,
        call_conv: CallConv,
    ) -> Result<RuntimeFuncImport, String> {
        let signature = lambda_signature(capture_count, call_conv);
        if let Some(id) = self.lambda_by_capture_count.get(&capture_count).copied() {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let name = format!("__neomacs_rt_lambda_{capture_count}");
        let id = self.module.declare_import(&name, &signature);
        self.imported_names.push(name.clone());
        self.lambda_by_capture_count.insert(capture_count, id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn make_lexical_cell(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = unary_runtime_signature(call_conv);
        if let Some(id) = self.make_lexical_cell {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self
            .module
            .declare_import("__neomacs_rt_make_lexical_cell", &signature);
        self.imported_names.push("__neomacs_rt_make_lexical_cell".to_string());
        self.make_lexical_cell = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn lexical_cell_get(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = unary_runtime_signature(call_conv);
        if let Some(id) = self.lexical_cell_get {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_lexical_cell_get", &signature);
        self.imported_names.push("__neomacs_rt_lexical_cell_get".to_string());
        self.lexical_cell_get = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn lexical_cell_set(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = binary_runtime_signature(call_conv);
        if let Some(id) = self.lexical_cell_set {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_lexical_cell_set", &signature);
        self.imported_names.push("__neomacs_rt_lexical_cell_set".to_string());
        self.lexical_cell_set = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn cons(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = binary_runtime_signature(call_conv);
        if let Some(id) = self.cons {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_cons", &signature);
        self.imported_names.push("__neomacs_rt_cons".to_string());
        self.cons = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn car(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = unary_runtime_signature(call_conv);
        if let Some(id) = self.car {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_car", &signature);
        self.imported_names.push("__neomacs_rt_car".to_string());
        self.car = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn cdr(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = unary_runtime_signature(call_conv);
        if let Some(id) = self.cdr {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let id = self.module.declare_import("__neomacs_rt_cdr", &signature);
        self.imported_names.push("__neomacs_rt_cdr".to_string());
        self.cdr = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }
}

struct RuntimeFuncImport {
    id: FuncId,
    signature: Signature,
}

fn call_named_signature(arity: usize, call_conv: CallConv) -> Signature {
    let mut signature = Signature::new(call_conv);
    signature.params.push(AbiParam::new(types::I64)); // vmctx
    signature.params.push(AbiParam::new(types::I64)); // interned function symbol
    for _ in 0..arity {
        signature.params.push(AbiParam::new(types::I64));
    }
    signature.returns.push(AbiParam::new(types::I64));
    signature
}

fn indirect_call_signature(arity: usize, call_conv: CallConv) -> Signature {
    let mut signature = Signature::new(call_conv);
    signature.params.push(AbiParam::new(types::I64)); // vmctx
    signature.params.push(AbiParam::new(types::I64)); // callee
    for _ in 0..arity {
        signature.params.push(AbiParam::new(types::I64));
    }
    signature.returns.push(AbiParam::new(types::I64));
    signature
}

fn symbol_get_signature(call_conv: CallConv) -> Signature {
    let mut signature = Signature::new(call_conv);
    signature.params.push(AbiParam::new(types::I64)); // vmctx
    signature.params.push(AbiParam::new(types::I64)); // interned variable symbol
    signature.returns.push(AbiParam::new(types::I64));
    signature
}

fn symbol_set_signature(call_conv: CallConv) -> Signature {
    let mut signature = Signature::new(call_conv);
    signature.params.push(AbiParam::new(types::I64)); // vmctx
    signature.params.push(AbiParam::new(types::I64)); // interned variable symbol
    signature.params.push(AbiParam::new(types::I64)); // value
    signature.returns.push(AbiParam::new(types::I64));
    signature
}

fn bind_dynamic_signature(call_conv: CallConv) -> Signature {
    let mut signature = Signature::new(call_conv);
    signature.params.push(AbiParam::new(types::I64)); // vmctx
    signature.params.push(AbiParam::new(types::I64)); // interned variable symbol
    signature.params.push(AbiParam::new(types::I64)); // value
    signature
}

fn unbind_dynamic_signature(call_conv: CallConv) -> Signature {
    let mut signature = Signature::new(call_conv);
    signature.params.push(AbiParam::new(types::I64)); // vmctx
    signature.params.push(AbiParam::new(types::I64)); // binding count
    signature
}

fn indexed_runtime_signature(call_conv: CallConv) -> Signature {
    let mut signature = Signature::new(call_conv);
    signature.params.push(AbiParam::new(types::I64)); // vmctx
    signature.params.push(AbiParam::new(types::I64)); // compiler-owned table index/bits
    signature.returns.push(AbiParam::new(types::I64));
    signature
}

fn unary_runtime_signature(call_conv: CallConv) -> Signature {
    let mut signature = Signature::new(call_conv);
    signature.params.push(AbiParam::new(types::I64)); // vmctx
    signature.params.push(AbiParam::new(types::I64)); // value/cell
    signature.returns.push(AbiParam::new(types::I64));
    signature
}

fn binary_runtime_signature(call_conv: CallConv) -> Signature {
    let mut signature = Signature::new(call_conv);
    signature.params.push(AbiParam::new(types::I64)); // vmctx
    signature.params.push(AbiParam::new(types::I64)); // cell
    signature.params.push(AbiParam::new(types::I64)); // value
    signature.returns.push(AbiParam::new(types::I64));
    signature
}

fn lambda_signature(capture_count: usize, call_conv: CallConv) -> Signature {
    let mut signature = indexed_runtime_signature(call_conv);
    for _ in 0..capture_count {
        signature.params.push(AbiParam::new(types::I64));
    }
    signature
}

struct ClifLowerer<'a, M: ClifModuleBackend = ModuleDeclarations> {
    ssa: &'a SsaFunction,
    runtime: ClifRuntimeAbi<M>,
    safepoints: ClifSafepointTable,
    diagnostics: Vec<Diagnostic>,
}

impl<'a, M: ClifModuleBackend> ClifLowerer<'a, M> {
    fn new(ssa: &'a SsaFunction) -> Self where M: Default {
        Self {
            ssa,
            runtime: ClifRuntimeAbi::default(),
            safepoints: ClifSafepointTable::default(),
            diagnostics: Vec::new(),
        }
    }

    fn with_runtime(ssa: &'a SsaFunction, runtime: ClifRuntimeAbi<M>) -> Self {
        Self {
            ssa,
            runtime,
            safepoints: ClifSafepointTable::default(),
            diagnostics: Vec::new(),
        }
    }

    fn lower(mut self) -> ClifLowerOutput<M> {
        let call_conv = self.runtime.module.call_conv();

        let Some(entry) = self.ssa.entry else {
            self.diagnostics.push(Diagnostic::error(
                "Cranelift lowering requires an entry block",
            ));
            return self.finish(None);
        };
        let Some(entry_block) = self.ssa.blocks.get(entry) else {
            self.diagnostics.push(Diagnostic::error(
                "Cranelift lowering entry block is missing",
            ));
            return self.finish(None);
        };

        let mut signature = Signature::new(call_conv);
        signature.params.push(AbiParam::new(types::I64)); // vmctx
        for _ in &entry_block.params {
            signature.params.push(AbiParam::new(types::I64));
        }
        signature.returns.push(AbiParam::new(types::I64));

        let mut function = Function::with_name_signature(UserFuncName::user(0, 0), signature);
        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut function, &mut builder_context);
        let mut block_map = HashMap::new();
        let mut value_map = HashMap::new();
        let mut entry_vmctx = None;

        for (block_id, _) in self.ssa.blocks.iter() {
            let clif_block = builder.create_block();
            block_map.insert(block_id, clif_block);
        }

        for (block_id, block) in self.ssa.blocks.iter() {
            let clif_block = block_map[&block_id];
            if block_id == entry {
                let vmctx = builder.append_block_param(clif_block, types::I64);
                entry_vmctx = Some(vmctx);
                for ssa_param in &block.params {
                    let clif_param = builder.append_block_param(clif_block, types::I64);
                    value_map.insert(*ssa_param, clif_param);
                }
            } else {
                for ssa_param in &block.params {
                    let clif_param = builder.append_block_param(clif_block, types::I64);
                    value_map.insert(*ssa_param, clif_param);
                }
            }
        }

        let liveness = SsaSafepointLiveness::compute(self.ssa);
        let (state_diagnostics, safepoints) = {
            let mut state = ClifBlockLowerer {
                builder,
                block_map,
                value_map,
                lexical_vars: HashMap::new(),
                runtime_func_refs: HashMap::new(),
                vmctx: None,
                runtime: &mut self.runtime,
                safepoints: ClifSafepointTable::default(),
                safepoint_liveness: liveness,
                current_inst: None,
                call_conv,
                diagnostics: Vec::new(),
                exception_handlers: Vec::new(),
                ended_handler_count: 0,
                catch_result_value: None,
            };
            state.vmctx = entry_vmctx;

            for (block_id, block) in self.ssa.blocks.iter() {
                let clif_block = state.block_map[&block_id];
                state.builder.switch_to_block(clif_block);
                for (inst_index, inst) in block.instructions.iter().enumerate() {
                    state.current_inst = Some((block_id, inst_index));
                    state.lower_inst(inst);
                }
                state.current_inst = None;
                state.lower_terminator(&block.terminator);
            }

            state.builder.seal_all_blocks();
            state.builder.finalize();
            (state.diagnostics, state.safepoints)
        };
        self.diagnostics.extend(state_diagnostics);
        self.safepoints = safepoints;
        if !self.diagnostics.is_empty() {
            return self.finish(None);
        }

        let clif_diagnostics = verify_clif(&function);
        if !clif_diagnostics.is_empty() {
            self.diagnostics.extend(clif_diagnostics);
            return self.finish(None);
        }

        self.finish(Some(function))
    }

    fn finish(self, function: Option<Function>) -> ClifLowerOutput<M> {
        ClifLowerOutput {
            function,
            runtime: self.runtime,
            safepoints: self.safepoints,
            diagnostics: self.diagnostics,
        }
    }
}

struct ClifBlockLowerer<'a, M: ClifModuleBackend> {
    builder: FunctionBuilder<'a>,
    block_map: HashMap<BlockId, ir::Block>,
    value_map: HashMap<ValueId, ir::Value>,
    lexical_vars: HashMap<String, Variable>,
    runtime_func_refs: HashMap<FuncId, FuncRef>,
    vmctx: Option<ir::Value>,
    runtime: &'a mut ClifRuntimeAbi<M>,
    safepoints: ClifSafepointTable,
    safepoint_liveness: SsaSafepointLiveness,
    current_inst: Option<(BlockId, usize)>,
    call_conv: CallConv,
    diagnostics: Vec<Diagnostic>,
    exception_handlers: Vec<ExceptionHandler>,
    /// Counts CatchEnd/ConditionCaseEnd/UnwindProtectEnd instructions processed.
    /// Used to compute handler indices without popping the handler stack.
    ended_handler_count: usize,
    catch_result_value: Option<ir::Value>,
}

const EXCEPTION_SENTINEL: i64 = 0x0DEAD_BEEF_DEAD_BEEFu64 as i64;

#[derive(Clone, Copy)]
struct ExceptionHandler {
    handler_block: ir::Block,
    kind: ExceptionHandlerKind,
}

#[derive(Clone, Copy)]
enum ExceptionHandlerKind {
    Catch {
        catch_tag: ir::Value,
        continuation_block: ir::Block,
    },
    ConditionCase {
        continuation_block: ir::Block,
    },
    UnwindProtect {
        continuation_block: ir::Block,
        normal_block: ir::Block,
    },
}

impl<M: ClifModuleBackend> ClifBlockLowerer<'_, M> {
    fn lower_inst(&mut self, inst: &crate::ssa::SsaInst) {
        match &inst.kind {
            SsaInstKind::Const(value) => {
                let Some(result) = inst.result else {
                    self.error("constant instruction has no result");
                    return;
                };
                let immediate = match value {
                    SsaConst::Nil => NIL_BITS,
                    SsaConst::True => TRUE_BITS,
                    SsaConst::Int(value) => {
                        if !(FIXNUM_MIN..=FIXNUM_MAX).contains(value) {
                            self.error(format!("integer constant {value} requires bignum support"));
                            return;
                        }
                        (*value << TAG_BITS as i64) | FIXNUM_TAG
                    }
                    SsaConst::Char(value) => ((*value as i64) << TAG_BITS) | CHAR_TAG,
                    SsaConst::Float(value) => {
                        let bits = value.to_bits();
                        let Some(func_ref) = self.float_const_ref() else {
                            return;
                        };
                        let Some(value) = self.emit_indexed_runtime_call(
                            func_ref,
                            bits as i64,
                            ClifRuntimeCallKind::FloatConst { bits },
                        ) else {
                            return;
                        };
                        self.value_map.insert(result, value);
                        return;
                    }
                    SsaConst::String(value) => {
                        let string = self.runtime.intern_string(value).into_usize() as i64;
                        let Some(func_ref) = self.string_const_ref() else {
                            return;
                        };
                        let Some(value) = self.emit_indexed_runtime_call(
                            func_ref,
                            string,
                            ClifRuntimeCallKind::StringConst {
                                value: value.clone(),
                            },
                        ) else {
                            return;
                        };
                        self.value_map.insert(result, value);
                        return;
                    }
                    SsaConst::Symbol(name) => {
                        let symbol = self.runtime.intern_symbol(name);
                        let Some(func_ref) = self.symbol_get_ref() else {
                            return;
                        };
                        let symbol_idx =
                            self.builder.ins().iconst(types::I64, symbol.into_usize() as i64);
                        let Some(value) = self.emit_runtime_call(
                            func_ref,
                            &[symbol_idx],
                            ClifRuntimeCallKind::SymbolGet {
                                name: name.clone(),
                            },
                        ) else {
                            return;
                        };
                        self.value_map.insert(result, value);
                        return;
                    }
                    SsaConst::Value(cv) => {
                        let value = self.materialize_compile_value(cv);
                        self.value_map.insert(result, value);
                        return;
                    }
                };
                let clif_value = self.builder.ins().iconst(types::I64, immediate);
                self.value_map.insert(result, clif_value);
            }
            SsaInstKind::Quote(form) => {
                let Some(result) = inst.result else {
                    self.error("quote instruction has no result");
                    return;
                };
                let index = self.runtime.intern_quoted_form(form.clone());
                let Some(func_ref) = self.quote_ref() else {
                    return;
                };
                let Some(value) = self.emit_indexed_runtime_call(
                    func_ref,
                    index as i64,
                    ClifRuntimeCallKind::Quote { index },
                ) else {
                    return;
                };
                self.value_map.insert(result, value);
            }
            SsaInstKind::FunctionQuote(form) => {
                let Some(result) = inst.result else {
                    self.error("function quote instruction has no result");
                    return;
                };
                let index = self.runtime.intern_quoted_form(form.clone());
                let Some(func_ref) = self.function_quote_ref() else {
                    return;
                };
                let Some(value) = self.emit_indexed_runtime_call(
                    func_ref,
                    index as i64,
                    ClifRuntimeCallKind::FunctionQuote { index },
                ) else {
                    return;
                };
                self.value_map.insert(result, value);
            }
            SsaInstKind::Lambda { template, captures } => {
                let Some(result) = inst.result else {
                    self.error("lambda instruction has no result");
                    return;
                };
                let index = self.runtime.intern_lambda_template(template.clone());
                let capture_values = self.value_args(captures);
                if capture_values.len() != captures.len() {
                    return;
                }
                let Some(func_ref) = self.lambda_ref(capture_values.len()) else {
                    return;
                };
                let Some(value) = self.emit_indexed_runtime_call_with_args(
                    func_ref,
                    index as i64,
                    &capture_values,
                    ClifRuntimeCallKind::Lambda {
                        index,
                        capture_count: capture_values.len(),
                    },
                ) else {
                    return;
                };
                self.value_map.insert(result, value);
            }
            SsaInstKind::LexicalGet(name) => {
                let Some(result) = inst.result else {
                    self.error("lexical get instruction has no result");
                    return;
                };
                let Some(var) = self.lexical_var(name) else {
                    self.error(format!(
                        "unknown lexical binding `{name}` in Cranelift lowering"
                    ));
                    return;
                };
                let value = self.builder.use_var(var);
                self.value_map.insert(result, value);
            }
            SsaInstKind::BindLexical { name, value } => {
                let Some(value) = self.value(*value) else {
                    return;
                };
                self.def_lexical(name, value);
            }
            SsaInstKind::LexicalSet { name, value } => {
                let Some(result) = inst.result else {
                    self.error("lexical set instruction has no result");
                    return;
                };
                let Some(value) = self.value(*value) else {
                    return;
                };
                let Some(var) = self.lexical_var(name) else {
                    self.error(format!(
                        "unknown lexical binding `{name}` in Cranelift lowering"
                    ));
                    return;
                };
                self.builder.def_var(var, value);
                self.value_map.insert(result, value);
            }
            SsaInstKind::MakeLexicalCell { initial } => {
                let Some(result) = inst.result else {
                    self.error("make lexical cell instruction has no result");
                    return;
                };
                let Some(initial) = self.value(*initial) else {
                    return;
                };
                let Some(func_ref) = self.make_lexical_cell_ref() else {
                    return;
                };
                let Some(value) = self.emit_runtime_call(
                    func_ref,
                    &[initial],
                    ClifRuntimeCallKind::MakeLexicalCell,
                ) else {
                    return;
                };
                self.value_map.insert(result, value);
            }
            SsaInstKind::LexicalCellGet { cell } => {
                let Some(result) = inst.result else {
                    self.error("lexical cell get instruction has no result");
                    return;
                };
                let Some(cell) = self.value(*cell) else {
                    return;
                };
                let Some(func_ref) = self.lexical_cell_get_ref() else {
                    return;
                };
                let Some(value) =
                    self.emit_runtime_call(func_ref, &[cell], ClifRuntimeCallKind::LexicalCellGet)
                else {
                    return;
                };
                self.value_map.insert(result, value);
            }
            SsaInstKind::LexicalCellSet { cell, value } => {
                let Some(result) = inst.result else {
                    self.error("lexical cell set instruction has no result");
                    return;
                };
                let Some(cell) = self.value(*cell) else {
                    return;
                };
                let Some(value) = self.value(*value) else {
                    return;
                };
                let Some(func_ref) = self.lexical_cell_set_ref() else {
                    return;
                };
                let Some(result_value) = self.emit_runtime_call(
                    func_ref,
                    &[cell, value],
                    ClifRuntimeCallKind::LexicalCellSet,
                ) else {
                    return;
                };
                self.value_map.insert(result, result_value);
            }
            SsaInstKind::DeclareSpecial(_) => {}
            SsaInstKind::SymbolGet(name) => {
                let Some(result) = inst.result else {
                    self.error("symbol get instruction has no result");
                    return;
                };
                let Some(value) =
                    self.emit_symbol_access_runtime_call(RuntimeImportKind::SymbolGet, name, &[])
                else {
                    return;
                };
                self.value_map.insert(result, value);
            }
            SsaInstKind::SymbolSet { name, value } => {
                let Some(result) = inst.result else {
                    self.error("symbol set instruction has no result");
                    return;
                };
                let Some(value) = self.value(*value) else {
                    return;
                };
                let Some(value) = self.emit_symbol_access_runtime_call(
                    RuntimeImportKind::SymbolSet,
                    name,
                    &[value],
                ) else {
                    return;
                };
                self.value_map.insert(result, value);
            }
            SsaInstKind::BindDynamic { name, value } => {
                let Some(value) = self.value(*value) else {
                    return;
                };
                let Some(func_ref) = self.bind_dynamic_ref() else {
                    return;
                };
                let Some(()) = self.emit_symbol_runtime_void_call_with_kind(
                    func_ref,
                    name,
                    &[value],
                    ClifRuntimeCallKind::BindDynamic { name: name.clone() },
                ) else {
                    return;
                };
            }
            SsaInstKind::UnbindDynamic { count } => {
                let Some(func_ref) = self.unbind_dynamic_ref() else {
                    return;
                };
                let Some(()) = self.emit_indexed_runtime_void_call(
                    func_ref,
                    *count as i64,
                    ClifRuntimeCallKind::UnbindDynamic { count: *count },
                ) else {
                    return;
                };
            }
            SsaInstKind::CallNamed { name, args } => {
                let Some(result) = inst.result else {
                    self.error("named call instruction has no result");
                    return;
                };
                let expected_arity = args.len();
                let args = self.value_args(args);
                if args.len() != expected_arity {
                    return;
                }
                match self.lower_pure_integer_call(name, &args) {
                    PrimitiveCallLowering::Value(value) => {
                        self.value_map.insert(result, value);
                        return;
                    }
                    PrimitiveCallLowering::Unknown => {}
                    PrimitiveCallLowering::Error => return,
                }
                match self.lower_pair_runtime_call(name, &args) {
                    PrimitiveCallLowering::Value(value) => {
                        self.value_map.insert(result, value);
                        return;
                    }
                    PrimitiveCallLowering::Unknown => {}
                    PrimitiveCallLowering::Error => return,
                }
                // Try direct JIT-to-JIT call for module-local functions
                if let Some(local_fid) = self.runtime.local_function(name) {
                    let Some(vmctx) = self.vmctx else {
                        self.error("local function call requires vmctx");
                        return;
                    };
                    let arity = args.len();
                    let mut sig = Signature::new(self.call_conv);
                    sig.params.push(AbiParam::new(types::I64)); // vmctx
                    for _ in 0..arity {
                        sig.params.push(AbiParam::new(types::I64));
                    }
                    sig.returns.push(AbiParam::new(types::I64));
                    let Some(func_ref) = self.func_ref_for_id(local_fid, sig) else {
                        return;
                    };
                    let mut call_args = vec![vmctx];
                    call_args.extend(args.iter().copied());
                    let inst = self.builder.ins().call(func_ref, &call_args);
                    let result_value = self.builder.inst_results(inst)[0];
                    let Some(checked) = self.emit_exception_check(result_value) else {
                        return;
                    };
                    self.value_map.insert(result, checked);
                    return;
                }
                let Some(func_ref) = self.call_named_ref(args.len()) else {
                    return;
                };
                let Some(result_value) = self.emit_symbol_runtime_call(func_ref, name, &args)
                else {
                    return;
                };
                let Some(checked) = self.emit_exception_check(result_value) else {
                    return;
                };
                self.value_map.insert(result, checked);
            }
            SsaInstKind::Funcall { callee, args } => {
                let Some(result) = inst.result else {
                    self.error("funcall instruction has no result");
                    return;
                };
                let Some(callee) = self.value(*callee) else {
                    return;
                };
                let args = self.value_args(args);
                let Some(func_ref) = self.funcall_ref(args.len()) else {
                    return;
                };
                let Some(result_value) = self.emit_indirect_runtime_call(func_ref, callee, &args)
                else {
                    return;
                };
                let Some(checked) = self.emit_exception_check(result_value) else {
                    return;
                };
                self.value_map.insert(result, checked);
            }
            SsaInstKind::Apply { callee, args } => {
                let Some(result) = inst.result else {
                    self.error("apply instruction has no result");
                    return;
                };
                let Some(callee) = self.value(*callee) else {
                    return;
                };
                let args = self.value_args(args);
                let Some(func_ref) = self.apply_ref(args.len()) else {
                    return;
                };
                let Some(result_value) = self.emit_indirect_runtime_call_with_kind(
                    func_ref,
                    callee,
                    &args,
                    ClifRuntimeCallKind::Apply { arity: args.len() },
                ) else {
                    return;
                };
                let Some(checked) = self.emit_exception_check(result_value) else {
                    return;
                };
                self.value_map.insert(result, checked);
            }
            SsaInstKind::CatchBegin { tag } => {
                let Some(tag_value) = self.value(*tag) else { return };
                let Some(func_ref) = self.exception_func_ref("catch_begin", 1) else {
                    return;
                };
                let Some(_) = self.emit_runtime_call(
                    func_ref,
                    &[tag_value],
                    ClifRuntimeCallKind::CatchBegin,
                ) else {
                    return;
                };
                let handler_block = self.builder.create_block();
                let continuation_block = self.builder.create_block();
                self.exception_handlers.push(ExceptionHandler {
                    handler_block,
                    kind: ExceptionHandlerKind::Catch {
                        catch_tag: tag_value,
                        continuation_block,
                    },
                });
            }
            SsaInstKind::CatchEnd { body_result } => {
                // Compute handler index from the top of the stack, accounting
                // for previous CatchEnd/ConditionCaseEnd/UnwindProtectEnd
                // instructions that have been processed.  Don't pop the stack
                // so that Throw in loop bodies can still find their handler.
                let Some(handler_idx) =
                    self.exception_handlers.len().checked_sub(self.ended_handler_count + 1)
                else {
                    self.error("CatchEnd without CatchBegin");
                    return;
                };
                self.ended_handler_count += 1;
                let Some(handler) = self.exception_handlers.get(handler_idx).cloned() else {
                    self.error("CatchEnd handler index out of range");
                    return;
                };
                let Some(func_ref) = self.exception_func_ref("catch_end", 0) else {
                    return;
                };
                let Some(_) = self.emit_runtime_call(
                    func_ref,
                    &[],
                    ClifRuntimeCallKind::CatchEnd,
                ) else {
                    return;
                };
                match handler.kind {
                    ExceptionHandlerKind::Catch {
                        catch_tag,
                        continuation_block,
                    } => {
                        // Add a block param for the catch result value
                        let catch_result =
                            self.builder.append_block_param(continuation_block, types::I64);
                        // Normal path: body completed normally, pass the body result
                        let normal_val = match body_result.and_then(|v| self.value(v)) {
                            Some(v) => v,
                            None => self.builder.ins().iconst(types::I64, NIL_BITS),
                        };
                        self.builder
                            .ins()
                            .jump(continuation_block, &[BlockArg::Value(normal_val)]);
                        self.builder.switch_to_block(handler.handler_block);
                        let Some(peek_ref) =
                            self.exception_func_ref("peek_throw_tag", 0)
                        else {
                            return;
                        };
                        let Some(throw_tag) = self.emit_runtime_call(
                            peek_ref,
                            &[],
                            ClifRuntimeCallKind::PeekThrowTag,
                        ) else {
                            return;
                        };
                        let tags_match = self
                            .builder
                            .ins()
                            .icmp(IntCC::Equal, catch_tag, throw_tag);
                        let match_block = self.builder.create_block();
                        let rethrow_block = self.builder.create_block();
                        self.builder.ins().brif(
                            tags_match,
                            match_block,
                            &[],
                            rethrow_block,
                            &[],
                        );
                        self.builder.switch_to_block(match_block);
                        let Some(get_val_ref) =
                            self.exception_func_ref("get_throw_value", 0)
                        else {
                            return;
                        };
                        let Some(throw_value) = self.emit_runtime_call(
                            get_val_ref,
                            &[],
                            ClifRuntimeCallKind::GetThrowValue,
                        ) else {
                            return;
                        };
                        let Some(catch_end_ref) =
                            self.exception_func_ref("catch_end", 0)
                        else {
                            return;
                        };
                        self.emit_runtime_call(
                            catch_end_ref,
                            &[],
                            ClifRuntimeCallKind::CatchEnd,
                        );
                        self.builder.ins().jump(continuation_block, &[BlockArg::Value(throw_value)]);
                        self.builder.switch_to_block(rethrow_block);
                        let Some(catch_end_ref2) =
                            self.exception_func_ref("catch_end", 0)
                        else {
                            return;
                        };
                        self.emit_runtime_call(
                            catch_end_ref2,
                            &[],
                            ClifRuntimeCallKind::CatchEnd,
                        );
                        // Rethrow to the outer handler (one level up).
                        // handler_idx is the index of THIS handler; the outer
                        // handler is at handler_idx - 1.
                        if let Some(outer_idx) = handler_idx.checked_sub(1) {
                            if let Some(outer) = self.exception_handlers.get(outer_idx) {
                                self.builder.ins().jump(outer.handler_block, &[]);
                            } else {
                                let sentinel =
                                    self.builder.ins().iconst(types::I64, EXCEPTION_SENTINEL);
                                self.builder.ins().return_(&[sentinel]);
                            }
                        } else {
                            let sentinel =
                                self.builder.ins().iconst(types::I64, EXCEPTION_SENTINEL);
                            self.builder.ins().return_(&[sentinel]);
                        }
                        // Don't seal blocks here — defer to seal_all_blocks.
                        // Throw in later blocks may need to add predecessors
                        // to the handler_block.
                        self.builder.switch_to_block(continuation_block);
                        // Map CatchEnd's result to the catch_result block parameter
                        if let Some(vid) = inst.result {
                            self.value_map.insert(vid, catch_result);
                        }
                        // Also remap body_result if present (for legacy code that
                        // references the body's value directly)
                        if let Some(vid) = body_result {
                            self.value_map.insert(*vid, catch_result);
                        }
                        self.catch_result_value = Some(catch_result);
                    }
                    _ => {
                        self.error("CatchEnd matched non-Catch handler");
                    }
                }
            }
            SsaInstKind::Throw { tag, value } => {
                let Some(tag_value) = self.value(*tag) else { return };
                let Some(value_value) = self.value(*value) else { return };
                let Some(func_ref) = self.exception_func_ref("throw", 2) else {
                    return;
                };
                let Some(result) = self.emit_runtime_call(
                    func_ref,
                    &[tag_value, value_value],
                    ClifRuntimeCallKind::Throw,
                ) else {
                    return;
                };
                if let Some(outer) = self.exception_handlers.last() {
                    self.builder
                        .ins()
                        .jump(outer.handler_block, &[]);
                } else {
                    self.builder.ins().return_(&[result]);
                }
                let unreachable_block = self.builder.create_block();
                self.builder.switch_to_block(unreachable_block);
            }
            SsaInstKind::ConditionCaseBegin { .. } => {
                let Some(func_ref) =
                    self.exception_func_ref("condition_case_begin", 0)
                else {
                    return;
                };
                let Some(_) = self.emit_runtime_call(
                    func_ref,
                    &[],
                    ClifRuntimeCallKind::ConditionCaseBegin,
                ) else {
                    return;
                };
                let handler_block = self.builder.create_block();
                let continuation_block = self.builder.create_block();
                self.exception_handlers.push(ExceptionHandler {
                    handler_block,
                    kind: ExceptionHandlerKind::ConditionCase { continuation_block },
                });
            }
            SsaInstKind::ConditionCaseHandler { pattern } => {
                let _ = pattern;
            }
            SsaInstKind::ConditionCaseHandlerResult { value } => {
                let _ = value;
            }
            SsaInstKind::ConditionCaseEnd { .. } => {
                let Some(handler_idx) =
                    self.exception_handlers.len().checked_sub(self.ended_handler_count + 1)
                else {
                    self.error("ConditionCaseEnd without ConditionCaseBegin");
                    return;
                };
                self.ended_handler_count += 1;
                let Some(handler) = self.exception_handlers.get(handler_idx).cloned() else {
                    self.error("ConditionCaseEnd handler index out of range");
                    return;
                };
                let Some(func_ref) =
                    self.exception_func_ref("condition_case_end", 0)
                else {
                    return;
                };
                let Some(_) = self.emit_runtime_call(
                    func_ref,
                    &[],
                    ClifRuntimeCallKind::ConditionCaseEnd,
                ) else {
                    return;
                };
                match handler.kind {
                    ExceptionHandlerKind::ConditionCase { continuation_block } => {
                        self.builder.ins().jump(continuation_block, &[]);
                        self.builder.switch_to_block(handler.handler_block);
                        if let Some(outer_idx) = handler_idx.checked_sub(1) {
                            if let Some(outer) = self.exception_handlers.get(outer_idx) {
                                self.builder.ins().jump(outer.handler_block, &[]);
                            } else {
                                let sentinel =
                                    self.builder.ins().iconst(types::I64, EXCEPTION_SENTINEL);
                                self.builder.ins().return_(&[sentinel]);
                            }
                        } else {
                            let sentinel =
                                self.builder.ins().iconst(types::I64, EXCEPTION_SENTINEL);
                            self.builder.ins().return_(&[sentinel]);
                        }
                        // Don't seal — defer to seal_all_blocks.
                        self.builder.switch_to_block(continuation_block);
                    }
                    _ => {
                        self.error("ConditionCaseEnd matched non-ConditionCase handler");
                    }
                }
            }
            SsaInstKind::UnwindProtectBegin => {
                let Some(func_ref) =
                    self.exception_func_ref("unwind_protect_begin", 0)
                else {
                    return;
                };
                let Some(_) = self.emit_runtime_call(
                    func_ref,
                    &[],
                    ClifRuntimeCallKind::UnwindProtectBegin,
                ) else {
                    return;
                };
                let handler_block = self.builder.create_block();
                let continuation_block = self.builder.create_block();
                let normal_block = self.builder.create_block();
                self.exception_handlers.push(ExceptionHandler {
                    handler_block,
                    kind: ExceptionHandlerKind::UnwindProtect {
                        continuation_block,
                        normal_block,
                    },
                });
            }
            SsaInstKind::UnwindProtectCleanup => {
                let handler = self
                    .exception_handlers
                    .last()
                    .expect("UnwindProtectCleanup without UnwindProtectBegin");
                if let ExceptionHandlerKind::UnwindProtect { normal_block, .. } =
                    &handler.kind
                {
                    let nb = *normal_block;
                    self.builder.ins().jump(nb, &[]);
                    self.builder.switch_to_block(nb);
                }
                let Some(func_ref) =
                    self.exception_func_ref("unwind_protect_cleanup_enter", 0)
                else {
                    return;
                };
                self.emit_runtime_call(
                    func_ref,
                    &[],
                    ClifRuntimeCallKind::UnwindProtectCleanupEnter,
                );
            }
            SsaInstKind::UnwindProtectEnd => {
                let Some(handler_idx) =
                    self.exception_handlers.len().checked_sub(self.ended_handler_count + 1)
                else {
                    self.error("UnwindProtectEnd without UnwindProtectBegin");
                    return;
                };
                self.ended_handler_count += 1;
                let Some(handler) = self.exception_handlers.get(handler_idx).cloned() else {
                    self.error("UnwindProtectEnd handler index out of range");
                    return;
                };
                let Some(func_ref) =
                    self.exception_func_ref("unwind_protect_end", 0)
                else {
                    return;
                };
                self.emit_runtime_call(
                    func_ref,
                    &[],
                    ClifRuntimeCallKind::UnwindProtectEnd,
                );
                match handler.kind {
                    ExceptionHandlerKind::UnwindProtect {
                        continuation_block,
                        normal_block,
                    } => {
                        let Some(check_ref) =
                            self.exception_func_ref("check_exception", 0)
                        else {
                            return;
                        };
                        let Some(check_result) = self.emit_runtime_call(
                            check_ref,
                            &[],
                            ClifRuntimeCallKind::CheckException,
                        ) else {
                            return;
                        };
                        let sentinel =
                            self.builder.ins().iconst(types::I64, EXCEPTION_SENTINEL);
                        let is_exception = self
                            .builder
                            .ins()
                            .icmp(IntCC::Equal, check_result, sentinel);
                        let no_exception_block = self.builder.create_block();
                        if let Some(outer_idx) = handler_idx.checked_sub(1) {
                            if let Some(outer) = self.exception_handlers.get(outer_idx) {
                                self.builder.ins().brif(
                                    is_exception,
                                    outer.handler_block,
                                    &[],
                                    no_exception_block,
                                    &[],
                                );
                            } else {
                                let sentinel_v =
                                    self.builder.ins().iconst(types::I64, EXCEPTION_SENTINEL);
                                self.builder.ins().return_(&[sentinel_v]);
                            }
                        } else {
                            self.builder.ins().brif(
                                is_exception,
                                continuation_block,
                                &[],
                                no_exception_block,
                                &[],
                            );
                        }
                        self.builder.switch_to_block(no_exception_block);
                        self.builder.ins().jump(continuation_block, &[]);
                        // Don't seal — defer to seal_all_blocks.
                        self.builder.switch_to_block(continuation_block);
                    }
                    _ => {
                        self.error("UnwindProtectEnd matched non-UnwindProtect handler");
                    }
                }
            }
        }
    }

    fn lower_terminator(&mut self, terminator: &SsaTerminator) {
        match terminator {
            SsaTerminator::Return(value) => {
                let value = match value {
                    Some(vid) => self.value(*vid),
                    None => self.catch_result_value.take(),
                }
                .unwrap_or_else(|| self.builder.ins().iconst(types::I64, 0));
                self.builder.ins().return_(&[value]);
            }
            SsaTerminator::Jump { target, args } => {
                let args = self.values(args);
                let Some(target) = self.block(*target) else {
                    return;
                };
                self.builder.ins().jump(target, &args);
            }
            SsaTerminator::BranchIfNil {
                test,
                then_target,
                then_args,
                else_target,
                else_args,
            } => {
                let Some(test) = self.value(*test) else {
                    return;
                };
                let is_nil = self.builder.ins().icmp_imm(IntCC::Equal, test, NIL_BITS);
                let then_args = self.values(then_args);
                let else_args = self.values(else_args);
                let Some(then_target) = self.block(*then_target) else {
                    return;
                };
                let Some(else_target) = self.block(*else_target) else {
                    return;
                };
                self.builder
                    .ins()
                    .brif(is_nil, then_target, &then_args, else_target, &else_args);
            }
            SsaTerminator::Unreachable => {
                self.builder.ins().trap(ir::TrapCode::unwrap_user(1));
            }
        }
    }

    fn values(&mut self, values: &[ValueId]) -> Vec<BlockArg> {
        self.value_args(values)
            .into_iter()
            .map(BlockArg::Value)
            .collect()
    }

    fn value_args(&mut self, values: &[ValueId]) -> Vec<ir::Value> {
        values
            .iter()
            .filter_map(|value| self.value(*value))
            .collect()
    }

    /// After a potentially-throwing call, check for exception sentinel.
    /// If sentinel detected, branch to current exception handler.
    fn emit_exception_check(&mut self, call_result: ir::Value) -> Option<ir::Value> {
        if self.exception_handlers.is_empty() {
            return Some(call_result);
        }
        let handler_block = self.exception_handlers.last().unwrap().handler_block;
        let normal_block = self.builder.create_block();
        let normal_result = self.builder.append_block_param(normal_block, types::I64);
        let sentinel = self.builder.ins().iconst(types::I64, EXCEPTION_SENTINEL);
        let is_sentinel = self
            .builder
            .ins()
            .icmp(IntCC::Equal, call_result, sentinel);
        self.builder.ins().brif(
            is_sentinel,
            handler_block,
            &[],
            normal_block,
            &[BlockArg::Value(call_result)],
        );
        self.builder.switch_to_block(normal_block);
        Some(normal_result)
    }

    fn lower_pure_integer_call(&mut self, name: &str, args: &[ir::Value]) -> PrimitiveCallLowering {
        match name {
            "null" | "not" => {
                let Some(value) = args.first() else {
                    return PrimitiveCallLowering::Error;
                };
                let is_nil = self.builder.ins().icmp_imm(IntCC::Equal, *value, NIL_BITS);
                PrimitiveCallLowering::Value(self.bool_to_lisp_value(is_nil))
            }
            "eq" | "eql" => {
                if args.len() != 2 {
                    return PrimitiveCallLowering::Unknown;
                }
                let is_eq = self.builder.ins().icmp(IntCC::Equal, args[0], args[1]);
                PrimitiveCallLowering::Value(self.bool_to_lisp_value(is_eq))
            }
            "integerp" => {
                let Some(value) = args.first() else {
                    return PrimitiveCallLowering::Error;
                };
                let tag = self.builder.ins().band_imm(*value, 7);
                let is_fix = self.builder.ins().icmp_imm(IntCC::Equal, tag, FIXNUM_TAG);
                PrimitiveCallLowering::Value(self.bool_to_lisp_value(is_fix))
            }
            "+" | "-" | "*" => self.lower_fixnum_arithmetic(name, args),
            "1+" | "1-" => {
                let Some(value) = args.first() else {
                    return PrimitiveCallLowering::Error;
                };
                let delta: i64 = if name == "1+" { 8 } else { -8 };
                let result = self.builder.ins().iadd_imm(*value, delta);
                PrimitiveCallLowering::Value(result)
            }
            "logand" | "logior" | "logxor" => self.lower_fixnum_bitwise(name, args),
            "=" | "<" | ">" | "<=" | ">=" => self.lower_fixnum_comparison(name, args),
            _ => PrimitiveCallLowering::Unknown,
        }
    }

    fn bool_to_lisp_value(&mut self, condition: ir::Value) -> ir::Value {
        let true_value = self.builder.ins().iconst(types::I64, TRUE_BITS);
        let false_value = self.builder.ins().iconst(types::I64, NIL_BITS);
        self.builder
            .ins()
            .select(condition, true_value, false_value)
    }

    fn is_fixnum(&mut self, value: ir::Value) -> ir::Value {
        let tag = self.builder.ins().band_imm(value, 7);
        self.builder.ins().icmp_imm(IntCC::Equal, tag, FIXNUM_TAG)
    }

    fn untag_fixnum(&mut self, value: ir::Value) -> ir::Value {
        self.builder.ins().ushr_imm(value, TAG_BITS as i64)
    }

    /// Inline fixnum arithmetic with runtime fallback for non-fixnum args.
    /// For + and -, tagged arithmetic works directly since both tags are 0.
    /// For *, one operand must be untagged first.
    fn lower_fixnum_arithmetic(&mut self, name: &str, args: &[ir::Value]) -> PrimitiveCallLowering {
        if args.len() != 2 {
            return PrimitiveCallLowering::Unknown;
        }

        let a = args[0];
        let b = args[1];

        let a_fix = self.is_fixnum(a);
        let b_fix = self.is_fixnum(b);
        let both_fix = self.builder.ins().band(a_fix, b_fix);

        let inline_block = self.builder.create_block();
        let fallback_block = self.builder.create_block();
        let merge_block = self.builder.create_block();
        let merge_result = self.builder.append_block_param(merge_block, types::I64);

        self.builder
            .ins()
            .brif(both_fix, inline_block, &[], fallback_block, &[]);

        self.builder.switch_to_block(inline_block);
        let inline_result = match name {
            "+" => self.builder.ins().iadd(a, b),
            "-" => self.builder.ins().isub(a, b),
            "*" => {
                let au = self.untag_fixnum(a);
                self.builder.ins().imul(au, b)
            }
            _ => unreachable!(),
        };
        self.builder.ins().jump(merge_block, &[BlockArg::Value(inline_result)]);

        self.builder.switch_to_block(fallback_block);
        let Some(func_ref) = self.call_named_ref(args.len()) else {
            return PrimitiveCallLowering::Error;
        };
        let Some(fallback_val) = self.emit_symbol_runtime_call(func_ref, name, args) else {
            return PrimitiveCallLowering::Error;
        };
        self.builder.ins().jump(merge_block, &[BlockArg::Value(fallback_val)]);

        self.builder.switch_to_block(merge_block);
        self.builder.seal_block(inline_block);
        self.builder.seal_block(fallback_block);
        self.builder.seal_block(merge_block);

        PrimitiveCallLowering::Value(merge_result)
    }

    /// Inline fixnum comparison with runtime fallback for float promotion.
    /// Tagged fixnum comparison order is preserved since tag bits are 0.
    fn lower_fixnum_comparison(&mut self, name: &str, args: &[ir::Value]) -> PrimitiveCallLowering {
        if args.len() != 2 {
            return PrimitiveCallLowering::Unknown;
        }

        let a = args[0];
        let b = args[1];

        let a_fix = self.is_fixnum(a);
        let b_fix = self.is_fixnum(b);
        let both_fix = self.builder.ins().band(a_fix, b_fix);

        let inline_block = self.builder.create_block();
        let fallback_block = self.builder.create_block();
        let merge_block = self.builder.create_block();
        let merge_result = self.builder.append_block_param(merge_block, types::I64);

        self.builder
            .ins()
            .brif(both_fix, inline_block, &[], fallback_block, &[]);

        self.builder.switch_to_block(inline_block);
        let cc = match name {
            "=" => IntCC::Equal,
            "<" => IntCC::SignedLessThan,
            ">" => IntCC::SignedGreaterThan,
            "<=" => IntCC::SignedLessThanOrEqual,
            ">=" => IntCC::SignedGreaterThanOrEqual,
            _ => unreachable!(),
        };
        let cmp = self.builder.ins().icmp(cc, a, b);
        let inline_val = self.bool_to_lisp_value(cmp);
        self.builder.ins().jump(merge_block, &[BlockArg::Value(inline_val)]);

        self.builder.switch_to_block(fallback_block);
        let Some(func_ref) = self.call_named_ref(2) else {
            return PrimitiveCallLowering::Error;
        };
        let Some(fallback_val) = self.emit_symbol_runtime_call(func_ref, name, args) else {
            return PrimitiveCallLowering::Error;
        };
        self.builder.ins().jump(merge_block, &[BlockArg::Value(fallback_val)]);

        self.builder.switch_to_block(merge_block);
        self.builder.seal_block(inline_block);
        self.builder.seal_block(fallback_block);
        self.builder.seal_block(merge_block);

        PrimitiveCallLowering::Value(merge_result)
    }

    /// Inline fixnum bitwise ops: untag both operands, apply the op, retag result.
    fn lower_fixnum_bitwise(&mut self, name: &str, args: &[ir::Value]) -> PrimitiveCallLowering {
        if args.len() != 2 {
            return PrimitiveCallLowering::Unknown;
        }

        let a = args[0];
        let b = args[1];

        let a_fix = self.is_fixnum(a);
        let b_fix = self.is_fixnum(b);
        let both_fix = self.builder.ins().band(a_fix, b_fix);

        let inline_block = self.builder.create_block();
        let fallback_block = self.builder.create_block();
        let merge_block = self.builder.create_block();
        let merge_result = self.builder.append_block_param(merge_block, types::I64);

        self.builder
            .ins()
            .brif(both_fix, inline_block, &[], fallback_block, &[]);

        self.builder.switch_to_block(inline_block);
        let au = self.untag_fixnum(a);
        let bu = self.untag_fixnum(b);
        let result = match name {
            "logand" => self.builder.ins().band(au, bu),
            "logior" => self.builder.ins().bor(au, bu),
            "logxor" => self.builder.ins().bxor(au, bu),
            _ => unreachable!(),
        };
        let tagged = self.builder.ins().ishl_imm(result, TAG_BITS as i64);
        self.builder.ins().jump(merge_block, &[BlockArg::Value(tagged)]);

        self.builder.switch_to_block(fallback_block);
        let Some(func_ref) = self.call_named_ref(2) else {
            return PrimitiveCallLowering::Error;
        };
        let Some(fallback_val) = self.emit_symbol_runtime_call(func_ref, name, args) else {
            return PrimitiveCallLowering::Error;
        };
        self.builder.ins().jump(merge_block, &[BlockArg::Value(fallback_val)]);

        self.builder.switch_to_block(merge_block);
        self.builder.seal_block(inline_block);
        self.builder.seal_block(fallback_block);
        self.builder.seal_block(merge_block);

        PrimitiveCallLowering::Value(merge_result)
    }

    fn lower_pair_runtime_call(&mut self, name: &str, args: &[ir::Value]) -> PrimitiveCallLowering {
        match (name, args) {
            ("cons", [car, cdr]) => {
                let Some(func_ref) = self.cons_ref() else {
                    return PrimitiveCallLowering::Error;
                };
                let Some(value) =
                    self.emit_runtime_call(func_ref, &[*car, *cdr], ClifRuntimeCallKind::Cons)
                else {
                    return PrimitiveCallLowering::Error;
                };
                PrimitiveCallLowering::Value(value)
            }
            ("car", [pair]) => {
                let Some(func_ref) = self.car_ref() else {
                    return PrimitiveCallLowering::Error;
                };
                let Some(value) =
                    self.emit_runtime_call(func_ref, &[*pair], ClifRuntimeCallKind::Car)
                else {
                    return PrimitiveCallLowering::Error;
                };
                PrimitiveCallLowering::Value(value)
            }
            ("cdr", [pair]) => {
                let Some(func_ref) = self.cdr_ref() else {
                    return PrimitiveCallLowering::Error;
                };
                let Some(value) =
                    self.emit_runtime_call(func_ref, &[*pair], ClifRuntimeCallKind::Cdr)
                else {
                    return PrimitiveCallLowering::Error;
                };
                PrimitiveCallLowering::Value(value)
            }
            ("cons" | "car" | "cdr", _) => PrimitiveCallLowering::Unknown,
            _ => PrimitiveCallLowering::Unknown,
        }
    }

    fn value(&mut self, value: ValueId) -> Option<ir::Value> {
        let Some(value) = self.value_map.get(&value).copied() else {
            self.error(format!("unknown SSA value {value:?} in Cranelift lowering"));
            return None;
        };
        Some(value)
    }

    fn block(&mut self, block: BlockId) -> Option<ir::Block> {
        let Some(block) = self.block_map.get(&block).copied() else {
            self.error(format!("unknown SSA block {block:?} in Cranelift lowering"));
            return None;
        };
        Some(block)
    }

    fn def_lexical(&mut self, name: &str, value: ir::Value) {
        let var = if let Some(var) = self.lexical_vars.get(name).copied() {
            var
        } else {
            let var = self.builder.declare_var(types::I64);
            self.builder.declare_var_needs_stack_map(var);
            self.lexical_vars.insert(name.to_string(), var);
            var
        };
        self.builder.def_var(var, value);
    }

    fn lexical_var(&self, name: &str) -> Option<Variable> {
        self.lexical_vars.get(name).copied()
    }

    fn error(&mut self, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(message));
    }

    fn call_named_ref(&mut self, arity: usize) -> Option<FuncRef> {
        let import = match self.runtime.call_named(arity, self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!("failed to declare Cranelift runtime call: {error}"));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn symbol_get_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.symbol_get(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift symbol get runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn symbol_set_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.symbol_set(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift symbol set runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn bind_dynamic_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.bind_dynamic(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift dynamic bind runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn unbind_dynamic_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.unbind_dynamic(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift dynamic unbind runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn funcall_ref(&mut self, arity: usize) -> Option<FuncRef> {
        let import = match self.runtime.funcall(arity, self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift funcall runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn apply_ref(&mut self, arity: usize) -> Option<FuncRef> {
        let import = match self.runtime.apply(arity, self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift apply runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn string_const_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.string_const(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift string constant runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn float_const_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.float_const(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift float constant runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn quote_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.quote(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift quote runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn function_quote_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.function_quote(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift function quote runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn lambda_ref(&mut self, capture_count: usize) -> Option<FuncRef> {
        let import = match self.runtime.lambda(capture_count, self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift lambda runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn make_lexical_cell_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.make_lexical_cell(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift lexical cell allocation runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn lexical_cell_get_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.lexical_cell_get(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift lexical cell get runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn lexical_cell_set_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.lexical_cell_set(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift lexical cell set runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn cons_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.cons(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift cons runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn car_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.car(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift car runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn cdr_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.cdr(self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!(
                    "failed to declare Cranelift cdr runtime call: {error}"
                ));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn exception_func_ref(&mut self, name: &str, num_extra_args: usize) -> Option<FuncRef> {
        let import = match self.runtime.exception_func(name, num_extra_args, self.call_conv) {
            Ok(import) => import,
            Err(error) => {
                self.error(format!("failed to declare exception runtime call: {error}"));
                return None;
            }
        };
        self.runtime_func_ref(import)
    }

    fn materialize_compile_value(&mut self, cv: &crate::compile_value::CompileValue) -> ir::Value {
        use crate::compile_value::CompileValue;
        match cv {
            CompileValue::Nil => self.builder.ins().iconst(types::I64, NIL_BITS),
            CompileValue::Bool(true) => self.builder.ins().iconst(types::I64, TRUE_BITS),
            CompileValue::Bool(false) => self.builder.ins().iconst(types::I64, NIL_BITS),
            CompileValue::Int(n) => {
                if !(FIXNUM_MIN..=FIXNUM_MAX).contains(n) {
                    self.error(format!("compile value integer {n} requires bignum support"));
                    return self.builder.ins().iconst(types::I64, NIL_BITS);
                }
                self.builder.ins().iconst(types::I64, (*n << TAG_BITS as i64) | FIXNUM_TAG)
            }
            CompileValue::Char(c) => {
                self.builder.ins().iconst(types::I64, ((*c as i64) << TAG_BITS) | CHAR_TAG)
            }
            CompileValue::Float(f) => {
                let bits = f.to_bits();
                let Some(func_ref) = self.float_const_ref() else {
                    return self.builder.ins().iconst(types::I64, NIL_BITS);
                };
                let Some(value) = self.emit_indexed_runtime_call(
                    func_ref,
                    bits as i64,
                    ClifRuntimeCallKind::FloatConst { bits },
                ) else {
                    return self.builder.ins().iconst(types::I64, NIL_BITS);
                };
                value
            }
            CompileValue::String(s) => {
                let string = self.runtime.intern_string(s).into_usize() as i64;
                let Some(func_ref) = self.string_const_ref() else {
                    return self.builder.ins().iconst(types::I64, NIL_BITS);
                };
                let Some(value) = self.emit_indexed_runtime_call(
                    func_ref,
                    string,
                    ClifRuntimeCallKind::StringConst { value: s.clone() },
                ) else {
                    return self.builder.ins().iconst(types::I64, NIL_BITS);
                };
                value
            }
            CompileValue::Symbol(name) => {
                let symbol = self.runtime.intern_symbol(name);
                let Some(func_ref) = self.symbol_get_ref() else {
                    return self.builder.ins().iconst(types::I64, NIL_BITS);
                };
                let symbol_idx =
                    self.builder.ins().iconst(types::I64, symbol.into_usize() as i64);
                let Some(value) = self.emit_runtime_call(
                    func_ref,
                    &[symbol_idx],
                    ClifRuntimeCallKind::SymbolGet { name: name.clone() },
                ) else {
                    return self.builder.ins().iconst(types::I64, NIL_BITS);
                };
                value
            }
            CompileValue::Cons { car, cdr } => {
                let car_val = self.materialize_compile_value(car);
                let cdr_val = self.materialize_compile_value(cdr);
                let Some(func_ref) = self.cons_ref() else {
                    return self.builder.ins().iconst(types::I64, NIL_BITS);
                };
                let Some(value) = self.emit_runtime_call(
                    func_ref,
                    &[car_val, cdr_val],
                    ClifRuntimeCallKind::Cons,
                ) else {
                    return self.builder.ins().iconst(types::I64, NIL_BITS);
                };
                value
            }
            CompileValue::Vector(items) => {
                // Build vector as a list of cons cells for now
                // A proper vector runtime function would be better
                let mut result = self.builder.ins().iconst(types::I64, NIL_BITS);
                for item in items.iter().rev() {
                    let item_val = self.materialize_compile_value(item);
                    let Some(func_ref) = self.cons_ref() else {
                        return self.builder.ins().iconst(types::I64, NIL_BITS);
                    };
                    let Some(value) = self.emit_runtime_call(
                        func_ref,
                        &[item_val, result],
                        ClifRuntimeCallKind::Cons,
                    ) else {
                        return self.builder.ins().iconst(types::I64, NIL_BITS);
                    };
                    result = value;
                }
                result
            }
        }
    }

    fn runtime_func_ref(&mut self, import: RuntimeFuncImport) -> Option<FuncRef> {
        self.func_ref_for_id(import.id, import.signature)
    }

    fn func_ref_for_id(&mut self, id: FuncId, signature: Signature) -> Option<FuncRef> {
        if let Some(func_ref) = self.runtime_func_refs.get(&id).copied() {
            return Some(func_ref);
        }

        let signature = self.builder.import_signature(signature);
        let user_name = self
            .builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: id.as_u32(),
            });
        let func_ref = self.builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(user_name),
            signature,
            colocated: false,
            patchable: false,
        });
        self.runtime_func_refs.insert(id, func_ref);
        Some(func_ref)
    }

    fn emit_runtime_call(
        &mut self,
        func_ref: FuncRef,
        args: &[ir::Value],
        kind: ClifRuntimeCallKind,
    ) -> Option<ir::Value> {
        let Some(vmctx) = self.vmctx else {
            self.error("runtime call lowering requires a vmctx parameter");
            return None;
        };
        let mut call_args = Vec::with_capacity(args.len() + 1);
        call_args.push(vmctx);
        call_args.extend_from_slice(args);
        let call = self.builder.ins().call(func_ref, &call_args);
        self.record_safepoint(call, kind);
        let Some(result) = self.builder.inst_results(call).first().copied() else {
            self.error("runtime call produced no result");
            return None;
        };
        Some(result)
    }

    fn emit_symbol_access_runtime_call(
        &mut self,
        kind: RuntimeImportKind,
        name: &str,
        args: &[ir::Value],
    ) -> Option<ir::Value> {
        let func_ref = match kind {
            RuntimeImportKind::SymbolGet => self.symbol_get_ref(),
            RuntimeImportKind::SymbolSet => self.symbol_set_ref(),
        }?;
        let call_kind = match kind {
            RuntimeImportKind::SymbolGet => ClifRuntimeCallKind::SymbolGet {
                name: name.to_string(),
            },
            RuntimeImportKind::SymbolSet => ClifRuntimeCallKind::SymbolSet {
                name: name.to_string(),
            },
        };
        self.emit_symbol_runtime_call_with_kind(func_ref, name, args, call_kind)
    }

    fn emit_symbol_runtime_call(
        &mut self,
        func_ref: FuncRef,
        symbol_name: &str,
        args: &[ir::Value],
    ) -> Option<ir::Value> {
        self.emit_symbol_runtime_call_with_kind(
            func_ref,
            symbol_name,
            args,
            ClifRuntimeCallKind::CallNamed {
                name: symbol_name.to_string(),
                arity: args.len(),
            },
        )
    }

    fn emit_symbol_runtime_call_with_kind(
        &mut self,
        func_ref: FuncRef,
        symbol_name: &str,
        args: &[ir::Value],
        kind: ClifRuntimeCallKind,
    ) -> Option<ir::Value> {
        let Some(vmctx) = self.vmctx else {
            self.error("runtime call lowering requires a vmctx parameter");
            return None;
        };
        let symbol = self.runtime.intern_symbol(symbol_name);
        let symbol = self
            .builder
            .ins()
            .iconst(types::I64, symbol.into_usize() as i64);
        let mut call_args = Vec::with_capacity(args.len() + 2);
        call_args.push(vmctx);
        call_args.push(symbol);
        call_args.extend_from_slice(args);
        let call = self.builder.ins().call(func_ref, &call_args);
        self.record_safepoint(call, kind);
        let Some(result) = self.builder.inst_results(call).first().copied() else {
            self.error("runtime call produced no result");
            return None;
        };
        Some(result)
    }

    fn emit_symbol_runtime_void_call_with_kind(
        &mut self,
        func_ref: FuncRef,
        symbol_name: &str,
        args: &[ir::Value],
        kind: ClifRuntimeCallKind,
    ) -> Option<()> {
        let Some(vmctx) = self.vmctx else {
            self.error("runtime call lowering requires a vmctx parameter");
            return None;
        };
        let symbol = self.runtime.intern_symbol(symbol_name);
        let symbol = self
            .builder
            .ins()
            .iconst(types::I64, symbol.into_usize() as i64);
        let mut call_args = Vec::with_capacity(args.len() + 2);
        call_args.push(vmctx);
        call_args.push(symbol);
        call_args.extend_from_slice(args);
        let call = self.builder.ins().call(func_ref, &call_args);
        self.record_safepoint(call, kind);
        Some(())
    }

    fn emit_indirect_runtime_call(
        &mut self,
        func_ref: FuncRef,
        callee: ir::Value,
        args: &[ir::Value],
    ) -> Option<ir::Value> {
        self.emit_indirect_runtime_call_with_kind(
            func_ref,
            callee,
            args,
            ClifRuntimeCallKind::Funcall { arity: args.len() },
        )
    }

    fn emit_indirect_runtime_call_with_kind(
        &mut self,
        func_ref: FuncRef,
        callee: ir::Value,
        args: &[ir::Value],
        kind: ClifRuntimeCallKind,
    ) -> Option<ir::Value> {
        let Some(vmctx) = self.vmctx else {
            self.error("indirect runtime call lowering requires a vmctx parameter");
            return None;
        };
        let mut call_args = Vec::with_capacity(args.len() + 2);
        call_args.push(vmctx);
        call_args.push(callee);
        call_args.extend_from_slice(args);
        let call = self.builder.ins().call(func_ref, &call_args);
        self.record_safepoint(call, kind);
        let Some(result) = self.builder.inst_results(call).first().copied() else {
            self.error("indirect runtime call produced no result");
            return None;
        };
        Some(result)
    }

    fn emit_indexed_runtime_call(
        &mut self,
        func_ref: FuncRef,
        value: i64,
        kind: ClifRuntimeCallKind,
    ) -> Option<ir::Value> {
        let Some(vmctx) = self.vmctx else {
            self.error("indexed runtime call lowering requires a vmctx parameter");
            return None;
        };
        let value = self.builder.ins().iconst(types::I64, value);
        let call = self.builder.ins().call(func_ref, &[vmctx, value]);
        self.record_safepoint(call, kind);
        let Some(result) = self.builder.inst_results(call).first().copied() else {
            self.error("indexed runtime call produced no result");
            return None;
        };
        Some(result)
    }

    fn emit_indexed_runtime_call_with_args(
        &mut self,
        func_ref: FuncRef,
        value: i64,
        args: &[ir::Value],
        kind: ClifRuntimeCallKind,
    ) -> Option<ir::Value> {
        let Some(vmctx) = self.vmctx else {
            self.error("indexed runtime call lowering requires a vmctx parameter");
            return None;
        };
        let value = self.builder.ins().iconst(types::I64, value);
        let mut call_args = Vec::with_capacity(args.len() + 2);
        call_args.push(vmctx);
        call_args.push(value);
        call_args.extend_from_slice(args);
        let call = self.builder.ins().call(func_ref, &call_args);
        self.record_safepoint(call, kind);
        let Some(result) = self.builder.inst_results(call).first().copied() else {
            self.error("indexed runtime call produced no result");
            return None;
        };
        Some(result)
    }

    fn emit_indexed_runtime_void_call(
        &mut self,
        func_ref: FuncRef,
        value: i64,
        kind: ClifRuntimeCallKind,
    ) -> Option<()> {
        let Some(vmctx) = self.vmctx else {
            self.error("indexed runtime call lowering requires a vmctx parameter");
            return None;
        };
        let value = self.builder.ins().iconst(types::I64, value);
        let call = self.builder.ins().call(func_ref, &[vmctx, value]);
        self.record_safepoint(call, kind);
        Some(())
    }

    fn record_safepoint(&mut self, call_inst: ir::Inst, kind: ClifRuntimeCallKind) {
        let Some((block, inst)) = self.current_inst else {
            self.error("safepoint recorded outside an SSA instruction");
            return;
        };
        let mut live_roots = Vec::new();
        let roots = self.safepoint_liveness.roots_for(block, inst).to_vec();
        for ssa_value in roots {
            let Some(clif_value) = self.value_map.get(&ssa_value).copied() else {
                self.error(format!(
                    "safepoint references unknown SSA value {ssa_value:?}"
                ));
                continue;
            };
            self.builder.declare_value_needs_stack_map(clif_value);
            live_roots.push(ClifLiveRoot {
                ssa_value,
                clif_value,
            });
        }
        live_roots.sort_by_key(|root| root.ssa_value);
        self.safepoints.entries.push(ClifSafepoint {
            call_inst,
            kind,
            live_roots,
        });
    }
}

enum RuntimeImportKind {
    SymbolGet,
    SymbolSet,
}

enum PrimitiveCallLowering {
    Value(ir::Value),
    Unknown,
    Error,
}

#[cfg(test)]
mod tests {
    use crate::clif::{ClifRuntimeCallKind, dump_clif, ssa_module_to_clif, ssa_to_clif};
    use crate::compile_source;
    use crate::ids::PrimaryMap;
    use crate::lower::{hir_to_ssa, hir_to_ssa_module, lambda_template_to_ssa};
    use crate::ssa::{SsaBlock, SsaCaptureMode, SsaFunction, SsaLambdaCapture, SsaTerminator};
    use crate::verify::verify_ssa;

    fn capture_names(captures: &[SsaLambdaCapture]) -> Vec<&str> {
        captures
            .iter()
            .map(|capture| capture.name.as_str())
            .collect()
    }

    #[test]
    fn lowers_constant_return_to_cranelift_ir() {
        let artifact = compile_source(
            "constant.el",
            ";;; -*- lexical-binding: t; -*-\n(defun forty-two () 42)",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert_eq!(clif.safepoints.entries.len(), 0);
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("iconst.i64 336")); // 42 << 3 (tagged fixnum)
        assert!(dump.contains("return"));
    }

    #[test]
    fn lowers_ssa_module_to_clif_functions() {
        let artifact = compile_source(
            "module.el",
            ";;; -*- lexical-binding: t; -*-\n(defun a (x) x)\n(defun b (y) (+ y 1))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa_module = hir_to_ssa_module(&hir);
        assert_eq!(ssa_module.diagnostics, Vec::new());

        let clif_module = ssa_module_to_clif(&ssa_module.value);
        assert_eq!(clif_module.diagnostics, Vec::new());
        assert_eq!(clif_module.functions.len(), 2);
        for (_, function) in clif_module.functions.iter() {
            assert!(function.function.is_some());
        }
    }

    #[test]
    fn lowers_unreachable_terminator_to_trap() {
        let mut blocks = PrimaryMap::new();
        let entry = blocks.push(SsaBlock {
            params: Vec::new(),
            instructions: Vec::new(),
            terminator: SsaTerminator::Unreachable,
        });
        let ssa = SsaFunction {
            name: Some("trap-only".to_string()),
            lambda_list: Default::default(),
            values: PrimaryMap::new(),
            blocks,
            entry: Some(entry),
        };

        let clif = ssa_to_clif(&ssa);
        assert_eq!(clif.diagnostics, Vec::new());
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("trap user1"));
    }

    #[test]
    fn lowers_string_constant_to_runtime_materialization() {
        let artifact = compile_source(
            "string.el",
            ";;; -*- lexical-binding: t; -*-\n(defun stringy () \"hello\")",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert_eq!(
            clif.runtime
                .string_key("hello")
                .map(|string| clif.runtime.resolve_string(string)),
            Some("hello")
        );
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_string_const")
        );
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert!(matches!(
            &safepoint.kind,
            ClifRuntimeCallKind::StringConst { value } if value == "hello"
        ));
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn lowers_float_constant_to_runtime_materialization() {
        let artifact = compile_source(
            "float.el",
            ";;; -*- lexical-binding: t; -*-\n(defun pi-ish () 3.5)",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_float_const")
        );
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert!(matches!(
            &safepoint.kind,
            ClifRuntimeCallKind::FloatConst { bits } if *bits == 3.5f64.to_bits()
        ));
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn lowers_quote_to_runtime_materialization() {
        let artifact = compile_source(
            "quote.el",
            ";;; -*- lexical-binding: t; -*-\n(defun quoted () '(a b))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_quote")
        );
        assert_eq!(clif.runtime.quoted_forms().len(), 1);
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert!(matches!(
            &safepoint.kind,
            ClifRuntimeCallKind::Quote { index } if *index == 0
        ));
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn lowers_function_quote_to_runtime_materialization() {
        let artifact = compile_source(
            "function-quote.el",
            ";;; -*- lexical-binding: t; -*-\n(defun quoted-fn () #'foo)",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_function_quote")
        );
        assert_eq!(clif.runtime.quoted_forms().len(), 1);
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert!(matches!(
            &safepoint.kind,
            ClifRuntimeCallKind::FunctionQuote { index } if *index == 0
        ));
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn lowers_lambda_to_runtime_materialization() {
        let artifact = compile_source(
            "lambda.el",
            ";;; -*- lexical-binding: t; -*-\n(defun make-identity () (lambda (x) x))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_lambda_0")
        );
        assert_eq!(clif.runtime.lambda_templates().len(), 1);
        assert_eq!(
            clif.runtime.lambda_templates()[0].params.required,
            vec!["x"]
        );
        assert!(clif.runtime.lambda_templates()[0].captures.is_empty());
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert!(matches!(
            &safepoint.kind,
            ClifRuntimeCallKind::Lambda {
                index,
                capture_count
            } if *index == 0 && *capture_count == 0
        ));
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn lowers_lambda_captures_to_runtime_materialization_args() {
        let artifact = compile_source(
            "capture.el",
            ";;; -*- lexical-binding: t; -*-\n(defun make-adder (x) (lambda (y) (+ x y)))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_lambda_1")
        );
        assert_eq!(clif.runtime.lambda_templates().len(), 1);
        assert_eq!(
            clif.runtime.lambda_templates()[0].params.required,
            vec!["y"]
        );
        assert_eq!(
            capture_names(&clif.runtime.lambda_templates()[0].captures),
            vec!["x"]
        );
        assert_eq!(
            clif.runtime.lambda_templates()[0].captures[0].mode,
            SsaCaptureMode::Value
        );
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert_eq!(safepoint.live_roots.len(), 1);
        assert!(matches!(
            &safepoint.kind,
            ClifRuntimeCallKind::Lambda {
                index,
                capture_count
            } if *index == 0 && *capture_count == 1
        ));
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn lowers_function_quoted_lambda_to_runtime_materialization() {
        let artifact = compile_source(
            "function-lambda.el",
            ";;; -*- lexical-binding: t; -*-\n(defun make-adder (x) #'(lambda (y) (+ x y)))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_lambda_1")
        );
        assert_eq!(clif.runtime.lambda_templates().len(), 1);
        assert_eq!(
            clif.runtime.lambda_templates()[0].params.required,
            vec!["y"]
        );
        assert_eq!(
            capture_names(&clif.runtime.lambda_templates()[0].captures),
            vec!["x"]
        );
        assert_eq!(
            clif.runtime.lambda_templates()[0].captures[0].mode,
            SsaCaptureMode::Value
        );
    }

    #[test]
    fn marks_mutable_lambda_captures_as_cells() {
        let artifact = compile_source(
            "mutable-capture.el",
            ";;; -*- lexical-binding: t; -*-\n(defun make-counter (x) (lambda () (setq x (+ x 1)) x))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_make_lexical_cell")
        );
        assert_eq!(clif.runtime.lambda_templates().len(), 1);
        assert_eq!(
            capture_names(&clif.runtime.lambda_templates()[0].captures),
            vec!["x"]
        );
        assert_eq!(
            clif.runtime.lambda_templates()[0].captures[0].mode,
            SsaCaptureMode::Cell
        );
    }

    #[test]
    fn lowers_captured_mutation_to_lexical_cell_runtime_calls() {
        let artifact = compile_source(
            "captured-mutation.el",
            ";;; -*- lexical-binding: t; -*-\n(defun bump (x) (let ((f (lambda () x))) (setq x (+ x 1)) x))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        let imported_names = clif.runtime.imported_function_names();
        assert!(imported_names.contains(&"__neomacs_rt_make_lexical_cell"));
        assert!(imported_names.contains(&"__neomacs_rt_lexical_cell_get"));
        assert!(imported_names.contains(&"__neomacs_rt_lexical_cell_set"));
        assert!(imported_names.contains(&"__neomacs_rt_lambda_1"));
        assert_eq!(
            clif.runtime.lambda_templates()[0].captures[0].mode,
            SsaCaptureMode::Cell
        );
        assert!(clif.safepoints.entries.len() >= 4);
    }

    #[test]
    fn lowers_lambda_template_body_to_standalone_clif() {
        let artifact = compile_source(
            "lambda-body.el",
            ";;; -*- lexical-binding: t; -*-\n(defun make-adder (x) (lambda (y) (+ x y)))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());

        let outer_clif = ssa_to_clif(&ssa.value);
        assert_eq!(outer_clif.diagnostics, Vec::new());
        let template = outer_clif.runtime.lambda_templates()[0].clone();
        let lambda_ssa = lambda_template_to_ssa(&template);
        assert_eq!(lambda_ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&lambda_ssa.value), Vec::new());
        let entry = lambda_ssa.value.entry.expect("entry block");
        assert_eq!(lambda_ssa.value.blocks[entry].params.len(), 2);

        let lambda_clif = ssa_to_clif(&lambda_ssa.value);
        assert_eq!(lambda_clif.diagnostics, Vec::new());
        // + goes through runtime dispatch for float promotion support
        assert!(
            lambda_clif
                .runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_call_named_2")
        );
    }

    #[test]
    fn lowers_mutable_lambda_template_body_through_capture_cell() {
        let artifact = compile_source(
            "lambda-cell-body.el",
            ";;; -*- lexical-binding: t; -*-\n(defun make-counter (x) (lambda () (setq x (+ x 1)) x))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());

        let outer_clif = ssa_to_clif(&ssa.value);
        assert_eq!(outer_clif.diagnostics, Vec::new());
        let template = outer_clif.runtime.lambda_templates()[0].clone();
        let lambda_ssa = lambda_template_to_ssa(&template);
        assert_eq!(lambda_ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&lambda_ssa.value), Vec::new());

        let lambda_clif = ssa_to_clif(&lambda_ssa.value);
        assert_eq!(lambda_clif.diagnostics, Vec::new());
        let imported_names = lambda_clif.runtime.imported_function_names();
        assert!(imported_names.contains(&"__neomacs_rt_lexical_cell_get"));
        assert!(imported_names.contains(&"__neomacs_rt_lexical_cell_set"));
        assert!(!imported_names.contains(&"__neomacs_rt_make_lexical_cell"));
    }

    #[test]
    fn lowers_if_and_block_params_to_cranelift_ir() {
        let artifact = compile_source(
            "choose.el",
            ";;; -*- lexical-binding: t; -*-\n(defun choose (x y) (if x x y))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("brif"));
        assert!(dump.contains("jump"));
        assert!(dump.contains("return"));
    }

    #[test]
    fn lowers_lexical_set_to_cranelift_variable() {
        let artifact = compile_source(
            "set-local.el",
            ";;; -*- lexical-binding: t; -*-\n(defun set-local () (let ((x 1)) (setq x 2) x))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert_eq!(clif.safepoints.entries.len(), 0);
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("iconst.i64 16")); // 2 << 3 (tagged fixnum)
        assert!(dump.contains("return"));
    }

    #[test]
    fn lowers_lexical_set_across_branch_merge() {
        let artifact = compile_source(
            "set-branch.el",
            ";;; -*- lexical-binding: t; -*-\n(defun set-branch (flag) (let ((x 1)) (if flag (setq x 2) (setq x 3)) x))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert_eq!(clif.safepoints.entries.len(), 0);
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("iconst.i64 16")); // 2 << 3 (tagged fixnum)
        assert!(dump.contains("iconst.i64 24")); // 3 << 3 (tagged fixnum)
        assert!(dump.contains("return"));
    }

    #[test]
    fn dispatches_arithmetic_to_runtime() {
        let artifact = compile_source(
            "call.el",
            ";;; -*- lexical-binding: t; -*-\n(defun add1-native (x) (+ x 1))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        // Arithmetic goes through runtime dispatch for float promotion support
        assert!(
            clif
                .runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_call_named_2")
        );
    }

    #[test]
    fn dispatches_comparison_to_runtime() {
        let artifact = compile_source(
            "compare.el",
            ";;; -*- lexical-binding: t; -*-\n(defun ordered (x y z) (if (< x y z) 1 0))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        // Comparisons go through runtime dispatch for float promotion support
        assert!(
            clif
                .runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_call_named_3")
        );
    }

    #[test]
    fn lowers_cons_to_pair_runtime_call() {
        let artifact = compile_source(
            "cons.el",
            ";;; -*- lexical-binding: t; -*-\n(defun make-pair (x y) (cons x y))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        let imported_names = clif.runtime.imported_function_names();
        assert!(imported_names.contains(&"__neomacs_rt_cons"));
        assert!(!imported_names.contains(&"__neomacs_rt_call_named_2"));
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert!(matches!(&safepoint.kind, ClifRuntimeCallKind::Cons));
        assert_eq!(safepoint.live_roots.len(), 2);
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn lowers_car_cdr_to_pair_runtime_calls() {
        let artifact = compile_source(
            "pair-parts.el",
            ";;; -*- lexical-binding: t; -*-\n(defun pair-parts (pair) (cons (car pair) (cdr pair)))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        let imported_names = clif.runtime.imported_function_names();
        assert!(imported_names.contains(&"__neomacs_rt_car"));
        assert!(imported_names.contains(&"__neomacs_rt_cdr"));
        assert!(imported_names.contains(&"__neomacs_rt_cons"));
        assert!(!imported_names.contains(&"__neomacs_rt_call_named_1"));
        assert!(!imported_names.contains(&"__neomacs_rt_call_named_2"));
        let safepoints = clif
            .safepoints
            .entries
            .iter()
            .map(|(_, safepoint)| safepoint)
            .collect::<Vec<_>>();
        assert_eq!(safepoints.len(), 3);
        assert!(matches!(&safepoints[0].kind, ClifRuntimeCallKind::Car));
        assert!(matches!(&safepoints[1].kind, ClifRuntimeCallKind::Cdr));
        assert!(matches!(&safepoints[2].kind, ClifRuntimeCallKind::Cons));
    }

    #[test]
    fn marks_live_lexical_roots_for_cranelift_stack_maps() {
        let artifact = compile_source(
            "stack-map-live-root.el",
            ";;; -*- lexical-binding: t; -*-\n(defun stack-map-live-root (x) global-value x)",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("stack_map=["));
    }

    #[test]
    fn lowers_symbol_get_to_runtime_abi_call() {
        let artifact = compile_source(
            "symbol-get.el",
            ";;; -*- lexical-binding: t; -*-\n(defun read-global () global-value)",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert_eq!(
            clif.runtime
                .symbol_key("global-value")
                .map(|symbol| clif.runtime.resolve_symbol(symbol)),
            Some("global-value")
        );
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_symbol_get")
        );
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn lowers_symbol_set_to_runtime_abi_call() {
        let artifact = compile_source(
            "symbol-set.el",
            ";;; -*- lexical-binding: t; -*-\n(defun write-global () (setq global-value 7))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert_eq!(
            clif.runtime
                .symbol_key("global-value")
                .map(|symbol| clif.runtime.resolve_symbol(symbol)),
            Some("global-value")
        );
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_symbol_set")
        );
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn lowers_dynamic_binding_to_runtime_scope_calls() {
        let artifact = compile_source(
            "dynamic-let.el",
            ";;; -*- lexical-binding: nil; -*-\n(defun dynamic-let (x) (let ((dyn-value x)) dyn-value))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert_eq!(
            clif.runtime
                .symbol_key("dyn-value")
                .map(|symbol| clif.runtime.resolve_symbol(symbol)),
            Some("dyn-value")
        );
        let imported_names = clif.runtime.imported_function_names();
        assert!(imported_names.contains(&"__neomacs_rt_bind_dynamic"));
        assert!(imported_names.contains(&"__neomacs_rt_symbol_get"));
        assert!(imported_names.contains(&"__neomacs_rt_unbind_dynamic"));
        let safepoints = clif
            .safepoints
            .entries
            .iter()
            .map(|(_, safepoint)| safepoint)
            .collect::<Vec<_>>();
        assert_eq!(safepoints.len(), 3);
        assert!(matches!(
            &safepoints[0].kind,
            ClifRuntimeCallKind::BindDynamic { name } if name == "dyn-value"
        ));
        assert!(matches!(
            &safepoints[1].kind,
            ClifRuntimeCallKind::SymbolGet { name } if name == "dyn-value"
        ));
        assert!(matches!(
            &safepoints[2].kind,
            ClifRuntimeCallKind::UnbindDynamic { count } if *count == 1
        ));
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn dynamic_parallel_let_initializers_run_before_binds() {
        let artifact = compile_source(
            "dynamic-parallel-let.el",
            ";;; -*- lexical-binding: nil; -*-\n(defun dynamic-parallel-let () (let ((a 1) (b a)) b))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        let safepoints = clif
            .safepoints
            .entries
            .iter()
            .map(|(_, safepoint)| safepoint)
            .collect::<Vec<_>>();
        assert_eq!(safepoints.len(), 5);
        assert!(matches!(
            &safepoints[0].kind,
            ClifRuntimeCallKind::SymbolGet { name } if name == "a"
        ));
        assert!(matches!(
            &safepoints[1].kind,
            ClifRuntimeCallKind::BindDynamic { name } if name == "a"
        ));
        assert!(matches!(
            &safepoints[2].kind,
            ClifRuntimeCallKind::BindDynamic { name } if name == "b"
        ));
        assert!(matches!(
            &safepoints[3].kind,
            ClifRuntimeCallKind::SymbolGet { name } if name == "b"
        ));
        assert!(matches!(
            &safepoints[4].kind,
            ClifRuntimeCallKind::UnbindDynamic { count } if *count == 2
        ));
    }

    #[test]
    fn lowers_funcall_to_runtime_abi_call() {
        let artifact = compile_source(
            "funcall.el",
            ";;; -*- lexical-binding: t; -*-\n(defun call-it (f x) (funcall f x))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_funcall_1")
        );
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert!(matches!(
            &safepoint.kind,
            ClifRuntimeCallKind::Funcall { arity } if *arity == 1
        ));
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn lowers_apply_to_runtime_abi_call() {
        let artifact = compile_source(
            "apply.el",
            ";;; -*- lexical-binding: t; -*-\n(defun apply-it (f x xs) (apply f x xs))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_apply_2")
        );
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert!(matches!(
            &safepoint.kind,
            ClifRuntimeCallKind::Apply { arity } if *arity == 2
        ));
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
    }

    #[test]
    fn records_safepoints_for_each_runtime_call() {
        let artifact = compile_source(
            "safepoints.el",
            ";;; -*- lexical-binding: t; -*-\n(defun call-global (f) (funcall f global-value))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        let safepoints = clif
            .safepoints
            .entries
            .iter()
            .map(|(_, safepoint)| safepoint)
            .collect::<Vec<_>>();
        assert_eq!(safepoints.len(), 2);
        assert!(matches!(
            &safepoints[0].kind,
            ClifRuntimeCallKind::SymbolGet { name } if name == "global-value"
        ));
        assert!(matches!(
            &safepoints[1].kind,
            ClifRuntimeCallKind::Funcall { arity } if *arity == 1
        ));
        assert!(
            safepoints
                .iter()
                .all(|safepoint| !safepoint.live_roots.is_empty())
        );
    }

    #[test]
    fn records_liveness_pruned_safepoint_roots() {
        let artifact = compile_source(
            "precise-roots.el",
            ";;; -*- lexical-binding: t; -*-\n(defun precise-roots (x) (let ((dead \"dead\")) (foo x)))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert_eq!(clif.diagnostics, Vec::new());
        let safepoints = clif
            .safepoints
            .entries
            .iter()
            .map(|(_, safepoint)| safepoint)
            .collect::<Vec<_>>();
        assert_eq!(safepoints.len(), 2);
        assert!(matches!(
            &safepoints[0].kind,
            ClifRuntimeCallKind::StringConst { value } if value == "dead"
        ));
        assert!(matches!(
            &safepoints[1].kind,
            ClifRuntimeCallKind::CallNamed { name, arity } if name == "foo" && *arity == 1
        ));
        assert_eq!(safepoints[1].live_roots.len(), 1);
    }
}
