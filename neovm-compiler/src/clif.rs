use std::collections::HashMap;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    self, AbiParam, BlockArg, FuncRef, Function, InstBuilder, Signature, UserFuncName, types,
};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings;
use cranelift_codegen::verifier::verify_function;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, ModuleDeclarations};
use lasso::{Key, Rodeo, Spur};

use crate::diagnostic::Diagnostic;
use crate::ids::{BlockId, ValueId};
use crate::ssa::{SsaConst, SsaFunction, SsaInstKind, SsaTerminator};

pub struct ClifLowerOutput {
    pub function: Option<Function>,
    pub runtime: ClifRuntimeAbi,
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

pub struct ClifRuntimeAbi {
    declarations: ModuleDeclarations,
    symbols: Rodeo,
    call_named_by_arity: HashMap<usize, FuncId>,
    symbol_get: Option<FuncId>,
    symbol_set: Option<FuncId>,
}

impl Default for ClifRuntimeAbi {
    fn default() -> Self {
        Self {
            declarations: ModuleDeclarations::default(),
            symbols: Rodeo::default(),
            call_named_by_arity: HashMap::new(),
            symbol_get: None,
            symbol_set: None,
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

struct ClifLowerer<'a> {
    ssa: &'a SsaFunction,
    runtime: ClifRuntimeAbi,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> ClifLowerer<'a> {
    fn new(ssa: &'a SsaFunction) -> Self {
        Self {
            ssa,
            runtime: ClifRuntimeAbi::default(),
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

        let mut state = ClifBlockLowerer {
            builder,
            block_map,
            value_map,
            lexical_values: HashMap::new(),
            runtime_func_refs: HashMap::new(),
            vmctx: None,
            runtime: &mut self.runtime,
            call_conv,
            diagnostics: Vec::new(),
        };
        state.vmctx = entry_vmctx;

        for (block_id, block) in self.ssa.blocks.iter() {
            let clif_block = state.block_map[&block_id];
            state.builder.switch_to_block(clif_block);
            for inst in &block.instructions {
                state.lower_inst(inst);
            }
            state.lower_terminator(&block.terminator);
        }

        state.builder.seal_all_blocks();
        state.builder.finalize();
        self.diagnostics.extend(state.diagnostics);
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
                    SsaInstKind::Const(
                        SsaConst::Nil | SsaConst::True | SsaConst::Int(_) | SsaConst::Char(_),
                    )
                    | SsaInstKind::LexicalGet(_)
                    | SsaInstKind::BindLexical { .. }
                    | SsaInstKind::DeclareSpecial(_)
                    | SsaInstKind::SymbolGet(_)
                    | SsaInstKind::SymbolSet { .. }
                    | SsaInstKind::CallNamed { .. } => {}
                    SsaInstKind::Const(SsaConst::Float(_)) => {
                        self.unsupported("float constants need Lisp value encoding");
                    }
                    SsaInstKind::Const(SsaConst::String(_)) => {
                        self.unsupported("string constants need allocation and GC metadata");
                    }
                    SsaInstKind::Quote(_) => {
                        self.unsupported("quote lowering needs runtime object materialization");
                    }
                    SsaInstKind::FunctionQuote(_) => {
                        self.unsupported("function quote lowering needs function object support");
                    }
                    SsaInstKind::LexicalSet { .. } => {
                        self.unsupported(
                            "lexical set lowering needs mutable lexical environment analysis",
                        );
                    }
                    SsaInstKind::BindDynamic { .. } => {
                        self.unsupported(
                            "dynamic binding needs runtime dynamic environment support",
                        );
                    }
                    SsaInstKind::Funcall { .. } | SsaInstKind::Apply { .. } => {
                        self.unsupported("calls need a Cranelift runtime ABI");
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
                        self.unsupported(
                            "nonlocal control flow needs explicit runtime ABI support",
                        );
                    }
                }
            }

            if matches!(block.terminator, SsaTerminator::Unreachable) {
                self.unsupported("unreachable terminators need trap lowering");
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
            diagnostics: self.diagnostics,
        }
    }
}

struct ClifBlockLowerer<'a> {
    builder: FunctionBuilder<'a>,
    block_map: HashMap<BlockId, ir::Block>,
    value_map: HashMap<ValueId, ir::Value>,
    lexical_values: HashMap<String, ir::Value>,
    runtime_func_refs: HashMap<FuncId, FuncRef>,
    vmctx: Option<ir::Value>,
    runtime: &'a mut ClifRuntimeAbi,
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
                let value = match value {
                    SsaConst::Nil => 0,
                    SsaConst::True => 1,
                    SsaConst::Int(value) => *value,
                    SsaConst::Char(value) => *value,
                    SsaConst::Float(_) | SsaConst::String(_) => unreachable!(),
                };
                let clif_value = self.builder.ins().iconst(types::I64, value);
                self.value_map.insert(result, clif_value);
            }
            SsaInstKind::LexicalGet(name) => {
                let Some(result) = inst.result else {
                    self.error("lexical get instruction has no result");
                    return;
                };
                let Some(value) = self.lexical_values.get(name).copied() else {
                    self.error(format!(
                        "unknown lexical binding `{name}` in Cranelift lowering"
                    ));
                    return;
                };
                self.value_map.insert(result, value);
            }
            SsaInstKind::BindLexical { name, value } => {
                let Some(value) = self.value(*value) else {
                    return;
                };
                self.lexical_values.insert(name.clone(), value);
            }
            SsaInstKind::DeclareSpecial(_) => {}
            SsaInstKind::SymbolGet(name) => {
                let Some(result) = inst.result else {
                    self.error("symbol get instruction has no result");
                    return;
                };
                let Some(value) =
                    self.emit_symbol_runtime_call(RuntimeImportKind::SymbolGet, name, &[])
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
                let Some(value) =
                    self.emit_symbol_runtime_call(RuntimeImportKind::SymbolSet, name, &[value])
                else {
                    return;
                };
                self.value_map.insert(result, value);
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
                let Some(result_value) = self.emit_runtime_call(func_ref, name, &args) else {
                    return;
                };
                self.value_map.insert(result, result_value);
            }
            SsaInstKind::Quote(_)
            | SsaInstKind::FunctionQuote(_)
            | SsaInstKind::LexicalSet { .. }
            | SsaInstKind::BindDynamic { .. }
            | SsaInstKind::Funcall { .. }
            | SsaInstKind::Apply { .. }
            | SsaInstKind::CatchBegin { .. }
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
                unreachable!("unsupported subset checked before lowering")
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

    fn emit_symbol_runtime_call(
        &mut self,
        kind: RuntimeImportKind,
        name: &str,
        args: &[ir::Value],
    ) -> Option<ir::Value> {
        let func_ref = match kind {
            RuntimeImportKind::SymbolGet => self.symbol_get_ref(),
            RuntimeImportKind::SymbolSet => self.symbol_set_ref(),
        }?;
        self.emit_runtime_call(func_ref, name, args)
    }

    fn emit_runtime_call(
        &mut self,
        func_ref: FuncRef,
        symbol_name: &str,
        args: &[ir::Value],
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
        let Some(result) = self.builder.inst_results(call).first().copied() else {
            self.error("runtime call produced no result");
            return None;
        };
        Some(result)
    }
}

enum RuntimeImportKind {
    SymbolGet,
    SymbolSet,
}

#[cfg(test)]
mod tests {
    use crate::clif::{dump_clif, ssa_to_clif};
    use crate::compile_source;
    use crate::lower::hir_to_ssa;
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
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("iconst.i64 42"));
        assert!(dump.contains("return"));
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
        let dump = dump_clif(&clif.function.expect("CLIF function"));
        assert!(dump.contains("call"));
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
    fn reports_indirect_calls_until_runtime_abi_exists() {
        let artifact = compile_source(
            "call.el",
            ";;; -*- lexical-binding: t; -*-\n(defun call-it (f x) (funcall f x))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());

        let clif = ssa_to_clif(&ssa.value);
        assert!(clif.function.is_none());
        assert!(
            clif.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("runtime ABI"))
        );
    }
}
