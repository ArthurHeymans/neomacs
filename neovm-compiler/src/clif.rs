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
use crate::ids::{BlockId, PrimaryMap, SafepointId, ValueId};
use crate::liveness::SsaSafepointLiveness;
use crate::ssa::{SsaConst, SsaFunction, SsaInstKind, SsaLambdaTemplate, SsaTerminator};
use crate::surface::SurfaceForm;

pub struct ClifLowerOutput {
    pub function: Option<Function>,
    pub runtime: ClifRuntimeAbi,
    pub safepoints: ClifSafepointTable,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn ssa_to_clif(function: &SsaFunction) -> ClifLowerOutput {
    ClifLowerer::new(function).lower()
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
    Lambda { index: usize },
}

pub struct ClifRuntimeAbi {
    declarations: ModuleDeclarations,
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
    lambda: Option<FuncId>,
}

impl Default for ClifRuntimeAbi {
    fn default() -> Self {
        Self {
            declarations: ModuleDeclarations::default(),
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
            lambda: None,
        }
    }
}

impl ClifRuntimeAbi {
    pub fn declarations(&self) -> &ModuleDeclarations {
        &self.declarations
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
        self.declarations
            .get_functions()
            .filter_map(|(_, declaration)| declaration.name.as_deref())
            .collect()
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
        let (id, _) = self
            .declarations
            .declare_function(&name, Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
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
        let (id, _) = self
            .declarations
            .declare_function(&name, Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
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
        let (id, _) = self
            .declarations
            .declare_function(&name, Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        self.apply_by_arity.insert(arity, id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn symbol_get(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = symbol_get_signature(call_conv);
        if let Some(id) = self.symbol_get {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let (id, _) = self
            .declarations
            .declare_function("__neomacs_rt_symbol_get", Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        self.symbol_get = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn symbol_set(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = symbol_set_signature(call_conv);
        if let Some(id) = self.symbol_set {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let (id, _) = self
            .declarations
            .declare_function("__neomacs_rt_symbol_set", Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        self.symbol_set = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn bind_dynamic(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = bind_dynamic_signature(call_conv);
        if let Some(id) = self.bind_dynamic {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let (id, _) = self
            .declarations
            .declare_function("__neomacs_rt_bind_dynamic", Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        self.bind_dynamic = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn unbind_dynamic(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = unbind_dynamic_signature(call_conv);
        if let Some(id) = self.unbind_dynamic {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let (id, _) = self
            .declarations
            .declare_function("__neomacs_rt_unbind_dynamic", Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        self.unbind_dynamic = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn string_const(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = indexed_runtime_signature(call_conv);
        if let Some(id) = self.string_const {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let (id, _) = self
            .declarations
            .declare_function("__neomacs_rt_string_const", Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        self.string_const = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn float_const(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = indexed_runtime_signature(call_conv);
        if let Some(id) = self.float_const {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let (id, _) = self
            .declarations
            .declare_function("__neomacs_rt_float_const", Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        self.float_const = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn quote(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = indexed_runtime_signature(call_conv);
        if let Some(id) = self.quote {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let (id, _) = self
            .declarations
            .declare_function("__neomacs_rt_quote", Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        self.quote = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn function_quote(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = indexed_runtime_signature(call_conv);
        if let Some(id) = self.function_quote {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let (id, _) = self
            .declarations
            .declare_function("__neomacs_rt_function_quote", Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        self.function_quote = Some(id);
        Ok(RuntimeFuncImport { id, signature })
    }

    fn lambda(&mut self, call_conv: CallConv) -> Result<RuntimeFuncImport, String> {
        let signature = indexed_runtime_signature(call_conv);
        if let Some(id) = self.lambda {
            return Ok(RuntimeFuncImport { id, signature });
        }

        let (id, _) = self
            .declarations
            .declare_function("__neomacs_rt_lambda", Linkage::Import, &signature)
            .map_err(|error| error.to_string())?;
        self.lambda = Some(id);
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

struct ClifLowerer<'a> {
    ssa: &'a SsaFunction,
    runtime: ClifRuntimeAbi,
    safepoints: ClifSafepointTable,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> ClifLowerer<'a> {
    fn new(ssa: &'a SsaFunction) -> Self {
        Self {
            ssa,
            runtime: ClifRuntimeAbi::default(),
            safepoints: ClifSafepointTable::default(),
            diagnostics: Vec::new(),
        }
    }

    fn lower(mut self) -> ClifLowerOutput {
        let call_conv = CallConv::SystemV;
        self.check_supported_subset();
        if !self.diagnostics.is_empty() {
            return self.finish(None);
        }

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

    fn check_supported_subset(&mut self) {
        for (_, block) in self.ssa.blocks.iter() {
            for inst in &block.instructions {
                match &inst.kind {
                    SsaInstKind::Const(_)
                    | SsaInstKind::Quote(_)
                    | SsaInstKind::FunctionQuote(_)
                    | SsaInstKind::Lambda(_)
                    | SsaInstKind::LexicalGet(_)
                    | SsaInstKind::BindLexical { .. }
                    | SsaInstKind::LexicalSet { .. }
                    | SsaInstKind::DeclareSpecial(_)
                    | SsaInstKind::SymbolGet(_)
                    | SsaInstKind::SymbolSet { .. }
                    | SsaInstKind::BindDynamic { .. }
                    | SsaInstKind::UnbindDynamic { .. }
                    | SsaInstKind::CallNamed { .. }
                    | SsaInstKind::Funcall { .. }
                    | SsaInstKind::Apply { .. } => {}
                    SsaInstKind::CatchBegin { .. }
                    | SsaInstKind::CatchEnd
                    | SsaInstKind::Throw { .. }
                    | SsaInstKind::ConditionCaseBegin { .. }
                    | SsaInstKind::ConditionCaseHandler { .. }
                    | SsaInstKind::ConditionCaseEnd
                    | SsaInstKind::UnwindProtectBegin
                    | SsaInstKind::UnwindProtectCleanup
                    | SsaInstKind::UnwindProtectEnd => {
                        self.unsupported(
                            "nonlocal control flow needs explicit runtime ABI support",
                        );
                    }
                }
            }
        }
    }

    fn unsupported(&mut self, reason: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(format!(
            "unsupported Cranelift lowering: {}",
            reason.into()
        )));
    }

    fn finish(self, function: Option<Function>) -> ClifLowerOutput {
        ClifLowerOutput {
            function,
            runtime: self.runtime,
            safepoints: self.safepoints,
            diagnostics: self.diagnostics,
        }
    }
}

struct ClifBlockLowerer<'a> {
    builder: FunctionBuilder<'a>,
    block_map: HashMap<BlockId, ir::Block>,
    value_map: HashMap<ValueId, ir::Value>,
    lexical_vars: HashMap<String, Variable>,
    runtime_func_refs: HashMap<FuncId, FuncRef>,
    vmctx: Option<ir::Value>,
    runtime: &'a mut ClifRuntimeAbi,
    safepoints: ClifSafepointTable,
    safepoint_liveness: SsaSafepointLiveness,
    current_inst: Option<(BlockId, usize)>,
    call_conv: CallConv,
    diagnostics: Vec<Diagnostic>,
}

impl ClifBlockLowerer<'_> {
    fn lower_inst(&mut self, inst: &crate::ssa::SsaInst) {
        match &inst.kind {
            SsaInstKind::Const(value) => {
                let Some(result) = inst.result else {
                    self.error("constant instruction has no result");
                    return;
                };
                let immediate = match value {
                    SsaConst::Nil => 0,
                    SsaConst::True => 1,
                    SsaConst::Int(value) => *value,
                    SsaConst::Char(value) => *value,
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
            SsaInstKind::Lambda(template) => {
                let Some(result) = inst.result else {
                    self.error("lambda instruction has no result");
                    return;
                };
                let index = self.runtime.intern_lambda_template(template.clone());
                let Some(func_ref) = self.lambda_ref() else {
                    return;
                };
                let Some(value) = self.emit_indexed_runtime_call(
                    func_ref,
                    index as i64,
                    ClifRuntimeCallKind::Lambda { index },
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
                let args = self.value_args(args);
                let Some(func_ref) = self.call_named_ref(args.len()) else {
                    return;
                };
                let Some(result_value) = self.emit_symbol_runtime_call(func_ref, name, &args)
                else {
                    return;
                };
                self.value_map.insert(result, result_value);
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
                self.value_map.insert(result, result_value);
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
                self.value_map.insert(result, result_value);
            }
            SsaInstKind::CatchBegin { .. }
            | SsaInstKind::CatchEnd
            | SsaInstKind::Throw { .. }
            | SsaInstKind::ConditionCaseBegin { .. }
            | SsaInstKind::ConditionCaseHandler { .. }
            | SsaInstKind::ConditionCaseEnd
            | SsaInstKind::UnwindProtectBegin
            | SsaInstKind::UnwindProtectCleanup
            | SsaInstKind::UnwindProtectEnd => {
                unreachable!("unsupported subset checked before lowering")
            }
        }
    }

    fn lower_terminator(&mut self, terminator: &SsaTerminator) {
        match terminator {
            SsaTerminator::Return(value) => {
                let value = value
                    .and_then(|value| self.value(value))
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
                let is_nil = self.builder.ins().icmp_imm(IntCC::Equal, test, 0);
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

    fn lambda_ref(&mut self) -> Option<FuncRef> {
        let import = match self.runtime.lambda(self.call_conv) {
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

    fn runtime_func_ref(&mut self, import: RuntimeFuncImport) -> Option<FuncRef> {
        if let Some(func_ref) = self.runtime_func_refs.get(&import.id).copied() {
            return Some(func_ref);
        }

        let signature = self.builder.import_signature(import.signature);
        let user_name = self
            .builder
            .func
            .declare_imported_user_function(ir::UserExternalName {
                namespace: 0,
                index: import.id.as_u32(),
            });
        let func_ref = self.builder.import_function(ir::ExtFuncData {
            name: ir::ExternalName::user(user_name),
            signature,
            colocated: false,
            patchable: false,
        });
        self.runtime_func_refs.insert(import.id, func_ref);
        Some(func_ref)
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

#[cfg(test)]
mod tests {
    use crate::clif::{ClifRuntimeCallKind, dump_clif, ssa_to_clif};
    use crate::compile_source;
    use crate::ids::PrimaryMap;
    use crate::lower::hir_to_ssa;
    use crate::ssa::{SsaBlock, SsaFunction, SsaTerminator};
    use crate::verify::verify_ssa;

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
        assert!(dump.contains("iconst.i64 42"));
        assert!(dump.contains("return"));
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
                .contains(&"__neomacs_rt_lambda")
        );
        assert_eq!(clif.runtime.lambda_templates().len(), 1);
        assert_eq!(clif.runtime.lambda_templates()[0].params, vec!["x"]);
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert!(matches!(
            &safepoint.kind,
            ClifRuntimeCallKind::Lambda { index } if *index == 0
        ));
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
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
        assert!(dump.contains("iconst.i64 2"));
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
        assert!(dump.contains("iconst.i64 2"));
        assert!(dump.contains("iconst.i64 3"));
        assert!(dump.contains("return"));
    }

    #[test]
    fn lowers_named_call_to_runtime_abi_call() {
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
        assert_eq!(
            clif.runtime
                .symbol_key("+")
                .map(|symbol| clif.runtime.resolve_symbol(symbol)),
            Some("+")
        );
        assert!(
            clif.runtime
                .imported_function_names()
                .contains(&"__neomacs_rt_call_named_2")
        );
        assert_eq!(clif.safepoints.entries.len(), 1);
        let safepoint = clif.safepoints.entries.iter().next().unwrap().1;
        assert!(matches!(
            &safepoint.kind,
            ClifRuntimeCallKind::CallNamed { name, arity } if name == "+" && *arity == 2
        ));
        assert!(!safepoint.live_roots.is_empty());
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
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
            ";;; -*- lexical-binding: t; -*-\n(defun precise-roots (x) \"dead\" (+ x 1))",
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
            ClifRuntimeCallKind::CallNamed { name, arity } if name == "+" && *arity == 2
        ));
        assert_eq!(safepoints[1].live_roots.len(), 2);
    }
}
