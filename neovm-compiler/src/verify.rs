use std::collections::{HashMap, HashSet};

use cranelift_entity::EntityRef;

use crate::diagnostic::Diagnostic;
use crate::hir::{HirExpr, HirExprKind, HirItem, HirModule};
use crate::ids::{BlockId, RegBlockId, RegId, SafepointId, ValueId};
use crate::liveness::inst_uses;
use crate::regir::{RegFunction, RegInstKind, RegTerminator};
use crate::ssa::{SsaFunction, SsaInstKind, SsaTerminator, SsaValueKind};
use crate::surface::SurfaceForm;

pub fn verify_surface(_forms: &[SurfaceForm]) -> Vec<Diagnostic> {
    Vec::new()
}

pub fn verify_hir(module: &HirModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for item in &module.items {
        match item {
            HirItem::Expr(expr) => verify_hir_expr(expr, &mut diagnostics),
            HirItem::Defun(defun) => verify_hir_expr(&defun.body, &mut diagnostics),
        }
    }
    diagnostics
}

pub fn verify_ssa(function: &SsaFunction) -> Vec<Diagnostic> {
    let mut verifier = SsaVerifier {
        function,
        diagnostics: Vec::new(),
    };
    verifier.verify();
    verifier.diagnostics
}

pub fn verify_regir(function: &RegFunction) -> Vec<Diagnostic> {
    let mut verifier = RegVerifier {
        function,
        diagnostics: Vec::new(),
    };
    verifier.verify();
    verifier.diagnostics
}

struct SsaVerifier<'a> {
    function: &'a SsaFunction,
    diagnostics: Vec<Diagnostic>,
}

struct RegVerifier<'a> {
    function: &'a RegFunction,
    diagnostics: Vec<Diagnostic>,
}

impl RegVerifier<'_> {
    fn verify(&mut self) {
        if let Some(entry) = self.function.entry {
            self.check_block(entry);
        } else {
            self.error("Register IR function has no entry block");
        }
        for (_, block) in self.function.blocks.iter() {
            for inst in &block.instructions {
                self.verify_inst(&inst.kind);
            }
            self.verify_terminator(&block.terminator);
        }
        for (_, safepoint) in self.function.safepoints.entries.iter() {
            for root in &safepoint.live_roots {
                self.check_reg(*root);
            }
        }
    }

    fn verify_inst(&mut self, kind: &RegInstKind) {
        match kind {
            RegInstKind::LoadConst { dst, .. }
            | RegInstKind::Quote { dst, .. }
            | RegInstKind::FunctionQuote { dst, .. }
            | RegInstKind::LexicalGet { dst, .. }
            | RegInstKind::SymbolGet { dst, .. } => self.check_reg(*dst),
            RegInstKind::Move { dst, src } => {
                self.check_reg(*dst);
                self.check_reg(*src);
            }
            RegInstKind::LexicalSet { dst, src, .. } | RegInstKind::SymbolSet { dst, src, .. } => {
                self.check_reg(*dst);
                self.check_reg(*src);
            }
            RegInstKind::BindLexical { src, .. } | RegInstKind::BindDynamic { src, .. } => {
                self.check_reg(*src);
            }
            RegInstKind::UnbindDynamic { .. } => {}
            RegInstKind::CallNamed { dst, args, .. } => {
                self.check_reg(*dst);
                self.check_regs(args);
            }
            RegInstKind::Funcall { dst, callee, args }
            | RegInstKind::Apply { dst, callee, args } => {
                self.check_reg(*dst);
                self.check_reg(*callee);
                self.check_regs(args);
            }
            RegInstKind::CatchBegin { tag } => self.check_reg(*tag),
            RegInstKind::Throw { tag, value } => {
                self.check_reg(*tag);
                self.check_reg(*value);
            }
            RegInstKind::Safepoint { id } => self.check_safepoint(*id),
            RegInstKind::DeclareSpecial { .. }
            | RegInstKind::CatchEnd
            | RegInstKind::ConditionCaseBegin { .. }
            | RegInstKind::ConditionCaseHandler { .. }
            | RegInstKind::ConditionCaseEnd
            | RegInstKind::UnwindProtectBegin
            | RegInstKind::UnwindProtectCleanup
            | RegInstKind::UnwindProtectEnd => {}
        }
    }

    fn verify_terminator(&mut self, terminator: &RegTerminator) {
        match terminator {
            RegTerminator::Return(Some(reg)) => self.check_reg(*reg),
            RegTerminator::Return(None) => {}
            RegTerminator::Jump { target } => self.check_block(*target),
            RegTerminator::BranchIfNil {
                test,
                then_target,
                else_target,
            } => {
                self.check_reg(*test);
                self.check_block(*then_target);
                self.check_block(*else_target);
            }
            RegTerminator::Unreachable => {}
        }
    }

    fn check_regs(&mut self, regs: &[RegId]) {
        for reg in regs {
            self.check_reg(*reg);
        }
    }

    fn check_reg(&mut self, reg: RegId) {
        if reg.index() >= self.function.registers.len() {
            self.error(format!("Register IR references unknown register {reg:?}"));
        }
    }

    fn check_block(&mut self, block: RegBlockId) {
        if block.index() >= self.function.blocks.len() {
            self.error(format!("Register IR references unknown block {block:?}"));
        }
    }

    fn check_safepoint(&mut self, id: SafepointId) {
        if id.index() >= self.function.safepoints.entries.len() {
            self.error(format!("Register IR references unknown safepoint {id:?}"));
        }
    }

    fn error(&mut self, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(message));
    }
}

impl SsaVerifier<'_> {
    fn verify(&mut self) {
        if let Some(entry) = self.function.entry {
            self.check_block(entry);
        } else {
            self.error("SSA function has no entry block");
        }
        let dominators = compute_dominators(self.function);
        for (block_id, block) in self.function.blocks.iter() {
            for (index, param) in block.params.iter().copied().enumerate() {
                self.check_value(param);
                match &self.function.values[param].kind {
                    SsaValueKind::BlockParam {
                        block,
                        index: param_index,
                        ..
                    } if *block == block_id && *param_index == index => {}
                    _ => self.error(format!("SSA block param {param:?} has inconsistent owner")),
                }
            }
            for (inst_index, inst) in block.instructions.iter().enumerate() {
                if let Some(result) = inst.result {
                    self.check_value(result);
                    match &self.function.values[result].kind {
                        SsaValueKind::InstResult { block, inst }
                            if *block == block_id && *inst == inst_index => {}
                        _ => self.error(format!("SSA result {result:?} has inconsistent owner")),
                    }
                }
                self.verify_inst(block_id, inst_index, &inst.kind, &dominators);
            }
            self.verify_terminator(block_id, &block.terminator, &dominators);
        }
    }

    fn verify_inst(
        &mut self,
        block: BlockId,
        inst_index: usize,
        kind: &SsaInstKind,
        dominators: &HashMap<BlockId, HashSet<BlockId>>,
    ) {
        for value in inst_uses(kind) {
            self.check_value(value);
            self.check_value_dominates(value, block, Some(inst_index), dominators);
        }
    }

    fn verify_terminator(
        &mut self,
        block: BlockId,
        terminator: &SsaTerminator,
        dominators: &HashMap<BlockId, HashSet<BlockId>>,
    ) {
        match terminator {
            SsaTerminator::Return(value) => {
                if let Some(value) = value {
                    self.check_value(*value);
                    self.check_value_dominates(*value, block, None, dominators);
                }
            }
            SsaTerminator::Jump { target, args } => {
                self.check_branch_args(block, *target, args, dominators);
            }
            SsaTerminator::BranchIfNil {
                test,
                then_target,
                then_args,
                else_target,
                else_args,
            } => {
                self.check_value(*test);
                self.check_value_dominates(*test, block, None, dominators);
                self.check_branch_args(block, *then_target, then_args, dominators);
                self.check_branch_args(block, *else_target, else_args, dominators);
            }
            SsaTerminator::Unreachable => {}
        }
    }

    fn check_branch_args(
        &mut self,
        branch_block: BlockId,
        target: BlockId,
        args: &[ValueId],
        dominators: &HashMap<BlockId, HashSet<BlockId>>,
    ) {
        self.check_block(target);
        if let Some(block) = self.function.blocks.get(target)
            && block.params.len() != args.len()
        {
            self.error(format!(
                "SSA branch to {target:?} passes {} args for {} params",
                args.len(),
                block.params.len()
            ));
        }
        for value in args {
            self.check_value(*value);
            self.check_value_dominates(*value, branch_block, None, dominators);
        }
    }

    fn check_block(&mut self, block: BlockId) {
        if block.index() >= self.function.blocks.len() {
            self.error(format!("SSA references unknown block {block:?}"));
        }
    }

    fn check_value(&mut self, value: ValueId) {
        if value.index() >= self.function.values.len() {
            self.error(format!("SSA references unknown value {value:?}"));
        }
    }

    fn check_value_dominates(
        &mut self,
        value: ValueId,
        use_block: BlockId,
        use_inst_index: Option<usize>,
        dominators: &HashMap<BlockId, HashSet<BlockId>>,
    ) {
        let Some(def) = self.function.values.get(value) else {
            return;
        };
        let dominates = match &def.kind {
            SsaValueKind::BlockParam { block, .. } => {
                *block == use_block
                    || dominators
                        .get(&use_block)
                        .is_some_and(|blocks| blocks.contains(block))
            }
            SsaValueKind::InstResult { block, inst } => {
                if *block == use_block {
                    use_inst_index.is_none_or(|use_inst| *inst < use_inst)
                } else {
                    dominators
                        .get(&use_block)
                        .is_some_and(|blocks| blocks.contains(block))
                }
            }
        };
        if !dominates {
            self.error(format!(
                "SSA value {value:?} does not dominate its use in {use_block:?}"
            ));
        }
    }

    fn error(&mut self, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(message));
    }
}

fn compute_dominators(function: &SsaFunction) -> HashMap<BlockId, HashSet<BlockId>> {
    let blocks = function
        .blocks
        .iter()
        .map(|(block, _)| block)
        .collect::<Vec<_>>();
    let all_blocks = blocks.iter().copied().collect::<HashSet<_>>();
    let predecessors = predecessor_map(function);
    let mut dominators = HashMap::new();
    for block in &blocks {
        let initial = if Some(*block) == function.entry {
            HashSet::from([*block])
        } else {
            all_blocks.clone()
        };
        dominators.insert(*block, initial);
    }

    let mut changed = true;
    while changed {
        changed = false;
        for block in &blocks {
            if Some(*block) == function.entry {
                continue;
            }
            let preds = predecessors.get(block).map(Vec::as_slice).unwrap_or(&[]);
            let mut next = if let Some((first, rest)) = preds.split_first() {
                let mut intersection = dominators.get(first).cloned().unwrap_or_default();
                for pred in rest {
                    let pred_dominators = dominators.get(pred).cloned().unwrap_or_default();
                    intersection.retain(|candidate| pred_dominators.contains(candidate));
                }
                intersection
            } else {
                HashSet::new()
            };
            next.insert(*block);
            if dominators.get(block) != Some(&next) {
                dominators.insert(*block, next);
                changed = true;
            }
        }
    }

    dominators
}

fn predecessor_map(function: &SsaFunction) -> HashMap<BlockId, Vec<BlockId>> {
    let mut predecessors = HashMap::<BlockId, Vec<BlockId>>::new();
    for (block, _) in function.blocks.iter() {
        predecessors.entry(block).or_default();
    }
    for (block, body) in function.blocks.iter() {
        for successor in terminator_successors(&body.terminator) {
            predecessors.entry(successor).or_default().push(block);
        }
    }
    predecessors
}

fn terminator_successors(terminator: &SsaTerminator) -> Vec<BlockId> {
    match terminator {
        SsaTerminator::Jump { target, .. } => vec![*target],
        SsaTerminator::BranchIfNil {
            then_target,
            else_target,
            ..
        } => vec![*then_target, *else_target],
        SsaTerminator::Return(_) | SsaTerminator::Unreachable => Vec::new(),
    }
}

fn verify_hir_expr(expr: &HirExpr, diagnostics: &mut Vec<Diagnostic>) {
    if expr.span.end < expr.span.start {
        diagnostics.push(Diagnostic::error("HIR expression has invalid span").with_span(expr.span));
    }
    match &expr.kind {
        HirExprKind::LexicalSet { value, .. } | HirExprKind::SymbolSet { value, .. } => {
            verify_hir_expr(value, diagnostics);
        }
        HirExprKind::If {
            test,
            then_expr,
            else_expr,
        } => {
            verify_hir_expr(test, diagnostics);
            verify_hir_expr(then_expr, diagnostics);
            verify_hir_expr(else_expr, diagnostics);
        }
        HirExprKind::Progn(exprs) => {
            for expr in exprs {
                verify_hir_expr(expr, diagnostics);
            }
        }
        HirExprKind::Let { bindings, body, .. } => {
            for binding in bindings {
                verify_hir_expr(&binding.init, diagnostics);
            }
            verify_hir_expr(body, diagnostics);
        }
        HirExprKind::Lambda { body, .. } => verify_hir_expr(body, diagnostics),
        HirExprKind::Declare(_) => {}
        HirExprKind::Catch { tag, body } => {
            verify_hir_expr(tag, diagnostics);
            verify_hir_expr(body, diagnostics);
        }
        HirExprKind::Throw { tag, value } => {
            verify_hir_expr(tag, diagnostics);
            verify_hir_expr(value, diagnostics);
        }
        HirExprKind::ConditionCase { body, handlers, .. } => {
            verify_hir_expr(body, diagnostics);
            for handler in handlers {
                verify_hir_expr(&handler.body, diagnostics);
            }
        }
        HirExprKind::UnwindProtect { body, cleanup } => {
            verify_hir_expr(body, diagnostics);
            verify_hir_expr(cleanup, diagnostics);
        }
        HirExprKind::Funcall { callee, args } | HirExprKind::Apply { callee, args } => {
            verify_hir_expr(callee, diagnostics);
            for arg in args {
                verify_hir_expr(arg, diagnostics);
            }
        }
        HirExprKind::CallNamed { args, .. } => {
            for arg in args {
                verify_hir_expr(arg, diagnostics);
            }
        }
        HirExprKind::CallValue { callee, args } => {
            verify_hir_expr(callee, diagnostics);
            for arg in args {
                verify_hir_expr(arg, diagnostics);
            }
        }
        HirExprKind::Const(_)
        | HirExprKind::Quote(_)
        | HirExprKind::FunctionQuote(_)
        | HirExprKind::LexicalGet(_)
        | HirExprKind::SymbolGet(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::effects::Effects;
    use crate::ids::PrimaryMap;
    use crate::ssa::{
        SsaBlock, SsaConst, SsaFunction, SsaInst, SsaInstKind, SsaTerminator, SsaValue,
        SsaValueKind,
    };
    use crate::verify::verify_ssa;

    #[test]
    fn ssa_verifier_rejects_use_before_definition() {
        let mut blocks = PrimaryMap::new();
        let entry = blocks.push(SsaBlock {
            params: Vec::new(),
            instructions: Vec::new(),
            terminator: SsaTerminator::Unreachable,
        });
        let mut values = PrimaryMap::new();
        let first = values.push(SsaValue {
            kind: SsaValueKind::InstResult {
                block: entry,
                inst: 0,
            },
        });
        let later = values.push(SsaValue {
            kind: SsaValueKind::InstResult {
                block: entry,
                inst: 1,
            },
        });
        blocks[entry].instructions = vec![
            SsaInst {
                result: Some(first),
                kind: SsaInstKind::SymbolSet {
                    name: "x".to_string(),
                    value: later,
                },
                effects: Effects::pure(),
            },
            SsaInst {
                result: Some(later),
                kind: SsaInstKind::Const(SsaConst::Int(1)),
                effects: Effects::pure(),
            },
        ];
        blocks[entry].terminator = SsaTerminator::Return(Some(first));
        let function = SsaFunction {
            name: Some("bad-use-before-def".to_string()),
            values,
            blocks,
            entry: Some(entry),
        };

        let diagnostics = verify_ssa(&function);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("does not dominate"))
        );
    }
}
