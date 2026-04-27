use std::collections::HashMap;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    self, AbiParam, BlockArg, Function, InstBuilder, Signature, UserFuncName, types,
};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings;
use cranelift_codegen::verifier::verify_function;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

use crate::diagnostic::Diagnostic;
use crate::ids::{BlockId, ValueId};
use crate::ssa::{SsaConst, SsaFunction, SsaInstKind, SsaTerminator};

#[derive(Debug)]
pub struct ClifLowerOutput {
    pub function: Option<Function>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn ssa_to_clif(function: &SsaFunction) -> ClifLowerOutput {
    let mut lowerer = ClifLowerer::new(function);
    lowerer.lower()
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

struct ClifLowerer<'a> {
    ssa: &'a SsaFunction,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> ClifLowerer<'a> {
    fn new(ssa: &'a SsaFunction) -> Self {
        Self {
            ssa,
            diagnostics: Vec::new(),
        }
    }

    fn lower(&mut self) -> ClifLowerOutput {
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

        let mut signature = Signature::new(CallConv::SystemV);
        for _ in &entry_block.params {
            signature.params.push(AbiParam::new(types::I64));
        }
        signature.returns.push(AbiParam::new(types::I64));

        let mut function = Function::with_name_signature(UserFuncName::user(0, 0), signature);
        let mut builder_context = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut function, &mut builder_context);
        let mut block_map = HashMap::new();
        let mut value_map = HashMap::new();

        for (block_id, _) in self.ssa.blocks.iter() {
            let clif_block = builder.create_block();
            block_map.insert(block_id, clif_block);
        }

        for (block_id, block) in self.ssa.blocks.iter() {
            let clif_block = block_map[&block_id];
            if block_id == entry {
                builder.append_block_params_for_function_params(clif_block);
                for (ssa_param, clif_param) in block
                    .params
                    .iter()
                    .zip(builder.block_params(clif_block).iter().copied())
                {
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
            diagnostics: Vec::new(),
        };

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
                    | SsaInstKind::DeclareSpecial(_) => {}
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
                    SsaInstKind::SymbolGet(_) | SsaInstKind::SymbolSet { .. } => {
                        self.unsupported("symbol access needs runtime and buffer-local semantics");
                    }
                    SsaInstKind::BindDynamic { .. } => {
                        self.unsupported(
                            "dynamic binding needs runtime dynamic environment support",
                        );
                    }
                    SsaInstKind::CallNamed { .. }
                    | SsaInstKind::Funcall { .. }
                    | SsaInstKind::Apply { .. } => {
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

    fn finish(&mut self, function: Option<Function>) -> ClifLowerOutput {
        ClifLowerOutput {
            function,
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }
}

struct ClifBlockLowerer<'a> {
    builder: FunctionBuilder<'a>,
    block_map: HashMap<BlockId, ir::Block>,
    value_map: HashMap<ValueId, ir::Value>,
    lexical_values: HashMap<String, ir::Value>,
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
            SsaInstKind::Quote(_)
            | SsaInstKind::FunctionQuote(_)
            | SsaInstKind::LexicalSet { .. }
            | SsaInstKind::SymbolGet(_)
            | SsaInstKind::SymbolSet { .. }
            | SsaInstKind::BindDynamic { .. }
            | SsaInstKind::CallNamed { .. }
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
        values
            .iter()
            .filter_map(|value| self.value(*value))
            .map(BlockArg::Value)
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
    fn reports_runtime_operations_until_cranelift_abi_exists() {
        let artifact = compile_source(
            "call.el",
            ";;; -*- lexical-binding: t; -*-\n(defun add1-native (x) (+ x 1))",
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
