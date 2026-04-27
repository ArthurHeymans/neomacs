use std::collections::HashMap;

use indexmap::IndexSet;

use crate::diagnostic::Diagnostic;
use crate::effects::{Effect, Effects};
use crate::hir::{BindingMode, HirConst, HirDeclaration, HirExpr, HirExprKind, HirItem, HirModule};
use crate::ids::{BlockId, PrimaryMap, RegBlockId, RegId, ValueId};
use crate::liveness::SsaSafepointLiveness;
use crate::regir::{Reg, RegBlock, RegFunction, RegInst, RegInstKind, RegKind, RegTerminator};
use crate::safepoint::SafepointEntry;
use crate::ssa::{
    SsaBlock, SsaCaptureMode, SsaConst, SsaFunction, SsaInst, SsaInstKind, SsaLambdaCapture,
    SsaLambdaTemplate, SsaTerminator, SsaValue, SsaValueKind,
};

#[derive(Clone, Debug, PartialEq)]
pub struct LowerOutput<T> {
    pub value: T,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn hir_to_ssa(module: &HirModule) -> LowerOutput<SsaFunction> {
    let mut diagnostics = Vec::new();
    let mut builder = SsaBuilder::new(None);
    for item in &module.items {
        match item {
            HirItem::Expr(expr) => {
                builder.mutable_lexicals = mutable_lexical_names(expr);
                builder.cell_lexicals = cell_lexical_names(expr);
                let value = builder.lower_expr(expr);
                builder.set_terminator(SsaTerminator::Return(value));
            }
            HirItem::Defun(defun) => {
                builder.mutable_lexicals = mutable_lexical_names(&defun.body);
                builder.cell_lexicals = cell_lexical_names(&defun.body);
                if builder.function.name.is_none() {
                    builder.function.name = Some(defun.name.clone());
                }
                for declaration in &defun.declarations {
                    builder.lower_declaration(declaration);
                }
                for param in &defun.params {
                    let value =
                        builder.append_block_param(builder.current_block, Some(param.clone()));
                    let value = builder.maybe_box_lexical(param, value);
                    builder.emit_no_result(SsaInstKind::BindLexical {
                        name: param.clone(),
                        value,
                    });
                }
                let value = builder.lower_expr(&defun.body);
                builder.set_terminator(SsaTerminator::Return(value));
            }
        }
    }
    if module.items.is_empty() {
        let nil = builder.emit_value(SsaInstKind::Const(SsaConst::Nil), Effects::pure());
        builder.set_terminator(SsaTerminator::Return(Some(nil)));
    }
    diagnostics.extend(builder.diagnostics);
    LowerOutput {
        value: builder.function,
        diagnostics,
    }
}

pub fn ssa_to_regir(function: &SsaFunction) -> LowerOutput<RegFunction> {
    let mut lowerer = RegLowerer::new(function);
    lowerer.lower();
    LowerOutput {
        value: lowerer.function,
        diagnostics: lowerer.diagnostics,
    }
}

pub fn lambda_template_to_ssa(template: &SsaLambdaTemplate) -> LowerOutput<SsaFunction> {
    let mut builder = SsaBuilder::new(Some("<lambda>".to_string()));
    builder.mutable_lexicals = mutable_lexical_names(&template.body);
    builder.cell_lexicals = cell_lexical_names(&template.body);
    builder.cell_lexicals.extend(
        template
            .captures
            .iter()
            .filter(|capture| capture.mode == SsaCaptureMode::Cell)
            .map(|capture| capture.name.clone()),
    );

    for declaration in &template.declarations {
        builder.lower_declaration(declaration);
    }
    for capture in &template.captures {
        let value = builder.append_block_param(builder.current_block, Some(capture.name.clone()));
        let value = if capture.mode == SsaCaptureMode::Cell {
            value
        } else {
            builder.maybe_box_lexical(&capture.name, value)
        };
        builder.emit_no_result(SsaInstKind::BindLexical {
            name: capture.name.clone(),
            value,
        });
    }
    for param in &template.params {
        let value = builder.append_block_param(builder.current_block, Some(param.clone()));
        let value = builder.maybe_box_lexical(param, value);
        builder.emit_no_result(SsaInstKind::BindLexical {
            name: param.clone(),
            value,
        });
    }

    let value = builder.lower_expr(&template.body);
    builder.set_terminator(SsaTerminator::Return(value));
    LowerOutput {
        value: builder.function,
        diagnostics: builder.diagnostics,
    }
}

struct RegLowerer<'a> {
    ssa: &'a SsaFunction,
    function: RegFunction,
    block_map: HashMap<BlockId, RegBlockId>,
    value_map: HashMap<ValueId, RegId>,
    safepoint_liveness: SsaSafepointLiveness,
    current_inst: Option<(BlockId, usize)>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> RegLowerer<'a> {
    fn new(ssa: &'a SsaFunction) -> Self {
        let mut function = RegFunction {
            name: ssa.name.clone(),
            ..RegFunction::default()
        };
        let mut block_map = HashMap::new();
        for (block_id, _) in ssa.blocks.iter() {
            let reg_block = function.blocks.push(RegBlock {
                instructions: Vec::new(),
                terminator: RegTerminator::Unreachable,
            });
            block_map.insert(block_id, reg_block);
        }
        function.entry = ssa.entry.and_then(|entry| block_map.get(&entry).copied());

        let mut value_map = HashMap::new();
        for (value_id, value) in ssa.values.iter() {
            let name = match &value.kind {
                SsaValueKind::BlockParam { name, .. } => name.clone(),
                SsaValueKind::InstResult { .. } => None,
            };
            let reg = function.registers.push(Reg {
                kind: RegKind::LispValue,
                name,
            });
            value_map.insert(value_id, reg);
        }
        if let Some(entry) = ssa.entry
            && let Some(entry_block) = ssa.blocks.get(entry)
        {
            function.entry_params = entry_block
                .params
                .iter()
                .filter_map(|value| value_map.get(value).copied())
                .collect();
        }

        Self {
            ssa,
            function,
            block_map,
            value_map,
            safepoint_liveness: SsaSafepointLiveness::compute(ssa),
            current_inst: None,
            diagnostics: Vec::new(),
        }
    }

    fn lower(&mut self) {
        for (block_id, block) in self.ssa.blocks.iter() {
            let Some(reg_block) = self.block_map.get(&block_id).copied() else {
                self.diagnostics.push(Diagnostic::error(format!(
                    "missing register block for {block_id:?}"
                )));
                continue;
            };
            for (inst_index, inst) in block.instructions.iter().enumerate() {
                self.current_inst = Some((block_id, inst_index));
                self.lower_inst(reg_block, inst);
            }
            self.current_inst = None;
            self.lower_terminator(reg_block, &block.terminator);
        }
    }

    fn lower_inst(&mut self, block: RegBlockId, inst: &SsaInst) {
        let reg_inst = match &inst.kind {
            SsaInstKind::Const(value) => RegInstKind::LoadConst {
                dst: self.result_reg(inst),
                value: value.clone(),
            },
            SsaInstKind::Quote(form) => RegInstKind::Quote {
                dst: self.result_reg(inst),
                form: form.clone(),
            },
            SsaInstKind::FunctionQuote(form) => RegInstKind::FunctionQuote {
                dst: self.result_reg(inst),
                form: form.clone(),
            },
            SsaInstKind::Lambda { template, captures } => RegInstKind::Lambda {
                dst: self.result_reg(inst),
                template: template.clone(),
                captures: self.value_regs(captures),
            },
            SsaInstKind::LexicalGet(name) => RegInstKind::LexicalGet {
                dst: self.result_reg(inst),
                name: name.clone(),
            },
            SsaInstKind::LexicalSet { name, value } => RegInstKind::LexicalSet {
                dst: self.result_reg(inst),
                name: name.clone(),
                src: self.value_reg(*value),
            },
            SsaInstKind::MakeLexicalCell { initial } => RegInstKind::MakeLexicalCell {
                dst: self.result_reg(inst),
                initial: self.value_reg(*initial),
            },
            SsaInstKind::LexicalCellGet { cell } => RegInstKind::LexicalCellGet {
                dst: self.result_reg(inst),
                cell: self.value_reg(*cell),
            },
            SsaInstKind::LexicalCellSet { cell, value } => RegInstKind::LexicalCellSet {
                dst: self.result_reg(inst),
                cell: self.value_reg(*cell),
                src: self.value_reg(*value),
            },
            SsaInstKind::SymbolGet(name) => RegInstKind::SymbolGet {
                dst: self.result_reg(inst),
                name: name.clone(),
            },
            SsaInstKind::SymbolSet { name, value } => RegInstKind::SymbolSet {
                dst: self.result_reg(inst),
                name: name.clone(),
                src: self.value_reg(*value),
            },
            SsaInstKind::BindLexical { name, value } => RegInstKind::BindLexical {
                name: name.clone(),
                src: self.value_reg(*value),
            },
            SsaInstKind::BindDynamic { name, value } => RegInstKind::BindDynamic {
                name: name.clone(),
                src: self.value_reg(*value),
            },
            SsaInstKind::UnbindDynamic { count } => RegInstKind::UnbindDynamic { count: *count },
            SsaInstKind::DeclareSpecial(names) => RegInstKind::DeclareSpecial {
                names: names.clone(),
            },
            SsaInstKind::CallNamed { name, args } => RegInstKind::CallNamed {
                dst: self.result_reg(inst),
                name: name.clone(),
                args: self.value_regs(args),
            },
            SsaInstKind::Funcall { callee, args } => RegInstKind::Funcall {
                dst: self.result_reg(inst),
                callee: self.value_reg(*callee),
                args: self.value_regs(args),
            },
            SsaInstKind::Apply { callee, args } => RegInstKind::Apply {
                dst: self.result_reg(inst),
                callee: self.value_reg(*callee),
                args: self.value_regs(args),
            },
            SsaInstKind::CatchBegin { tag } => RegInstKind::CatchBegin {
                tag: self.value_reg(*tag),
            },
            SsaInstKind::CatchEnd => RegInstKind::CatchEnd,
            SsaInstKind::Throw { tag, value } => RegInstKind::Throw {
                tag: self.value_reg(*tag),
                value: self.value_reg(*value),
            },
            SsaInstKind::ConditionCaseBegin { var } => {
                RegInstKind::ConditionCaseBegin { var: var.clone() }
            }
            SsaInstKind::ConditionCaseHandler { pattern } => RegInstKind::ConditionCaseHandler {
                pattern: pattern.clone(),
            },
            SsaInstKind::ConditionCaseEnd => RegInstKind::ConditionCaseEnd,
            SsaInstKind::UnwindProtectBegin => RegInstKind::UnwindProtectBegin,
            SsaInstKind::UnwindProtectCleanup => RegInstKind::UnwindProtectCleanup,
            SsaInstKind::UnwindProtectEnd => RegInstKind::UnwindProtectEnd,
        };
        self.emit(block, reg_inst);
        if self.needs_safepoint(&inst.kind) {
            self.emit_safepoint(block);
        }
    }

    fn lower_terminator(&mut self, block: RegBlockId, terminator: &SsaTerminator) {
        let reg_terminator = match terminator {
            SsaTerminator::Return(value) => {
                RegTerminator::Return(value.map(|value| self.value_reg(value)))
            }
            SsaTerminator::Jump { target, args } => {
                self.emit_branch_moves(block, *target, args);
                RegTerminator::Jump {
                    target: self.block_reg(*target),
                }
            }
            SsaTerminator::BranchIfNil {
                test,
                then_target,
                then_args,
                else_target,
                else_args,
            } => {
                self.emit_branch_moves(block, *then_target, then_args);
                self.emit_branch_moves(block, *else_target, else_args);
                RegTerminator::BranchIfNil {
                    test: self.value_reg(*test),
                    then_target: self.block_reg(*then_target),
                    else_target: self.block_reg(*else_target),
                }
            }
            SsaTerminator::Unreachable => RegTerminator::Unreachable,
        };
        self.function.blocks[block].terminator = reg_terminator;
    }

    fn emit_branch_moves(&mut self, block: RegBlockId, target: BlockId, args: &[ValueId]) {
        let Some(target_block) = self.ssa.blocks.get(target) else {
            self.diagnostics.push(Diagnostic::error(format!(
                "unknown SSA branch target {target:?}"
            )));
            return;
        };
        for (param, arg) in target_block.params.iter().zip(args.iter()) {
            self.emit(
                block,
                RegInstKind::Move {
                    dst: self.value_reg(*param),
                    src: self.value_reg(*arg),
                },
            );
        }
        if target_block.params.len() != args.len() {
            self.diagnostics.push(Diagnostic::error(format!(
                "branch to {target:?} has {} args for {} params",
                args.len(),
                target_block.params.len()
            )));
        }
    }

    fn needs_safepoint(&self, kind: &SsaInstKind) -> bool {
        matches!(
            kind,
            SsaInstKind::Const(SsaConst::Float(_) | SsaConst::String(_))
                | SsaInstKind::Quote(_)
                | SsaInstKind::FunctionQuote(_)
                | SsaInstKind::Lambda { .. }
                | SsaInstKind::MakeLexicalCell { .. }
                | SsaInstKind::LexicalCellGet { .. }
                | SsaInstKind::LexicalCellSet { .. }
                | SsaInstKind::SymbolGet(_)
                | SsaInstKind::SymbolSet { .. }
                | SsaInstKind::BindDynamic { .. }
                | SsaInstKind::UnbindDynamic { .. }
                | SsaInstKind::CallNamed { .. }
                | SsaInstKind::Funcall { .. }
                | SsaInstKind::Apply { .. }
                | SsaInstKind::CatchBegin { .. }
                | SsaInstKind::Throw { .. }
                | SsaInstKind::ConditionCaseBegin { .. }
                | SsaInstKind::ConditionCaseHandler { .. }
                | SsaInstKind::UnwindProtectBegin
                | SsaInstKind::UnwindProtectCleanup
        )
    }

    fn emit(&mut self, block: RegBlockId, kind: RegInstKind) {
        self.function.blocks[block]
            .instructions
            .push(RegInst { kind });
    }

    fn emit_safepoint(&mut self, block: RegBlockId) {
        let Some((ssa_block, inst)) = self.current_inst else {
            self.diagnostics.push(Diagnostic::error(
                "safepoint emitted outside an SSA instruction",
            ));
            return;
        };
        let mut live_roots = Vec::new();
        let roots = self.safepoint_liveness.roots_for(ssa_block, inst).to_vec();
        for value in roots {
            let Some(reg) = self.value_map.get(&value).copied() else {
                self.diagnostics.push(Diagnostic::error(format!(
                    "safepoint references unknown SSA value {value:?}"
                )));
                continue;
            };
            live_roots.push(reg);
        }
        let id = self
            .function
            .safepoints
            .entries
            .push(SafepointEntry { live_roots });
        self.emit(block, RegInstKind::Safepoint { id });
    }

    fn result_reg(&self, inst: &SsaInst) -> RegId {
        let result = inst
            .result
            .expect("SSA instruction expected to have a result");
        self.value_reg(result)
    }

    fn value_reg(&self, value: ValueId) -> RegId {
        self.value_map[&value]
    }

    fn value_regs(&self, values: &[ValueId]) -> Vec<RegId> {
        values.iter().map(|value| self.value_reg(*value)).collect()
    }

    fn block_reg(&self, block: BlockId) -> RegBlockId {
        self.block_map[&block]
    }
}

fn lambda_capture_names(params: &[String], body: &HirExpr) -> Vec<String> {
    let bound = params.iter().cloned().collect::<IndexSet<_>>();
    let mut free = IndexSet::new();
    collect_free_lexicals(body, &bound, &mut free);
    free.into_iter().collect()
}

fn lambda_capture_specs(
    params: &[String],
    body: &HirExpr,
    mutable_lexicals: &IndexSet<String>,
) -> Vec<SsaLambdaCapture> {
    lambda_capture_names(params, body)
        .into_iter()
        .map(|name| {
            let mode = if mutable_lexicals.contains(&name) {
                SsaCaptureMode::Cell
            } else {
                SsaCaptureMode::Value
            };
            SsaLambdaCapture { name, mode }
        })
        .collect()
}

fn mutable_lexical_names(expr: &HirExpr) -> IndexSet<String> {
    let mut names = IndexSet::new();
    collect_mutable_lexicals(expr, &mut names);
    names
}

fn cell_lexical_names(expr: &HirExpr) -> IndexSet<String> {
    let mutable = mutable_lexical_names(expr);
    let mut captured = IndexSet::new();
    collect_lambda_captures(expr, &mut captured);
    mutable
        .into_iter()
        .filter(|name| captured.contains(name))
        .collect()
}

fn collect_lambda_captures(expr: &HirExpr, names: &mut IndexSet<String>) {
    match &expr.kind {
        HirExprKind::Const(_)
        | HirExprKind::Quote(_)
        | HirExprKind::FunctionQuote(_)
        | HirExprKind::LexicalGet(_)
        | HirExprKind::SymbolGet(_)
        | HirExprKind::Declare(_) => {}
        HirExprKind::LexicalSet { value, .. } | HirExprKind::SymbolSet { value, .. } => {
            collect_lambda_captures(value, names);
        }
        HirExprKind::If {
            test,
            then_expr,
            else_expr,
        } => {
            collect_lambda_captures(test, names);
            collect_lambda_captures(then_expr, names);
            collect_lambda_captures(else_expr, names);
        }
        HirExprKind::Progn(exprs) => {
            for expr in exprs {
                collect_lambda_captures(expr, names);
            }
        }
        HirExprKind::Let { bindings, body, .. } => {
            for binding in bindings {
                collect_lambda_captures(&binding.init, names);
            }
            collect_lambda_captures(body, names);
        }
        HirExprKind::Lambda { params, body, .. } => {
            names.extend(lambda_capture_names(params, body));
            collect_lambda_captures(body, names);
        }
        HirExprKind::Catch { tag, body } => {
            collect_lambda_captures(tag, names);
            collect_lambda_captures(body, names);
        }
        HirExprKind::Throw { tag, value } => {
            collect_lambda_captures(tag, names);
            collect_lambda_captures(value, names);
        }
        HirExprKind::ConditionCase { body, handlers, .. } => {
            collect_lambda_captures(body, names);
            for handler in handlers {
                collect_lambda_captures(&handler.body, names);
            }
        }
        HirExprKind::UnwindProtect { body, cleanup } => {
            collect_lambda_captures(body, names);
            collect_lambda_captures(cleanup, names);
        }
        HirExprKind::Funcall { callee, args }
        | HirExprKind::Apply { callee, args }
        | HirExprKind::CallValue { callee, args } => {
            collect_lambda_captures(callee, names);
            for arg in args {
                collect_lambda_captures(arg, names);
            }
        }
        HirExprKind::CallNamed { args, .. } => {
            for arg in args {
                collect_lambda_captures(arg, names);
            }
        }
    }
}

fn collect_mutable_lexicals(expr: &HirExpr, names: &mut IndexSet<String>) {
    match &expr.kind {
        HirExprKind::Const(_)
        | HirExprKind::Quote(_)
        | HirExprKind::FunctionQuote(_)
        | HirExprKind::LexicalGet(_)
        | HirExprKind::SymbolGet(_)
        | HirExprKind::Declare(_) => {}
        HirExprKind::LexicalSet { name, value } => {
            names.insert(name.clone());
            collect_mutable_lexicals(value, names);
        }
        HirExprKind::SymbolSet { value, .. } => {
            collect_mutable_lexicals(value, names);
        }
        HirExprKind::If {
            test,
            then_expr,
            else_expr,
        } => {
            collect_mutable_lexicals(test, names);
            collect_mutable_lexicals(then_expr, names);
            collect_mutable_lexicals(else_expr, names);
        }
        HirExprKind::Progn(exprs) => {
            for expr in exprs {
                collect_mutable_lexicals(expr, names);
            }
        }
        HirExprKind::Let { bindings, body, .. } => {
            for binding in bindings {
                collect_mutable_lexicals(&binding.init, names);
            }
            collect_mutable_lexicals(body, names);
        }
        HirExprKind::Lambda { body, .. } => {
            collect_mutable_lexicals(body, names);
        }
        HirExprKind::Catch { tag, body } => {
            collect_mutable_lexicals(tag, names);
            collect_mutable_lexicals(body, names);
        }
        HirExprKind::Throw { tag, value } => {
            collect_mutable_lexicals(tag, names);
            collect_mutable_lexicals(value, names);
        }
        HirExprKind::ConditionCase { body, handlers, .. } => {
            collect_mutable_lexicals(body, names);
            for handler in handlers {
                collect_mutable_lexicals(&handler.body, names);
            }
        }
        HirExprKind::UnwindProtect { body, cleanup } => {
            collect_mutable_lexicals(body, names);
            collect_mutable_lexicals(cleanup, names);
        }
        HirExprKind::Funcall { callee, args }
        | HirExprKind::Apply { callee, args }
        | HirExprKind::CallValue { callee, args } => {
            collect_mutable_lexicals(callee, names);
            for arg in args {
                collect_mutable_lexicals(arg, names);
            }
        }
        HirExprKind::CallNamed { args, .. } => {
            for arg in args {
                collect_mutable_lexicals(arg, names);
            }
        }
    }
}

fn collect_free_lexicals(expr: &HirExpr, bound: &IndexSet<String>, free: &mut IndexSet<String>) {
    match &expr.kind {
        HirExprKind::Const(_)
        | HirExprKind::Quote(_)
        | HirExprKind::FunctionQuote(_)
        | HirExprKind::SymbolGet(_)
        | HirExprKind::Declare(_) => {}
        HirExprKind::LexicalGet(name) => {
            if !bound.contains(name) {
                free.insert(name.clone());
            }
        }
        HirExprKind::LexicalSet { name, value } => {
            collect_free_lexicals(value, bound, free);
            if !bound.contains(name) {
                free.insert(name.clone());
            }
        }
        HirExprKind::SymbolSet { value, .. } => {
            collect_free_lexicals(value, bound, free);
        }
        HirExprKind::If {
            test,
            then_expr,
            else_expr,
        } => {
            collect_free_lexicals(test, bound, free);
            collect_free_lexicals(then_expr, bound, free);
            collect_free_lexicals(else_expr, bound, free);
        }
        HirExprKind::Progn(exprs) => {
            for expr in exprs {
                collect_free_lexicals(expr, bound, free);
            }
        }
        HirExprKind::Let {
            bindings,
            sequential,
            body,
            ..
        } => {
            if *sequential {
                let mut scoped = bound.clone();
                for binding in bindings {
                    collect_free_lexicals(&binding.init, &scoped, free);
                    if binding.mode == BindingMode::Lexical {
                        scoped.insert(binding.name.clone());
                    }
                }
                collect_free_lexicals(body, &scoped, free);
            } else {
                for binding in bindings {
                    collect_free_lexicals(&binding.init, bound, free);
                }
                let mut scoped = bound.clone();
                for binding in bindings {
                    if binding.mode == BindingMode::Lexical {
                        scoped.insert(binding.name.clone());
                    }
                }
                collect_free_lexicals(body, &scoped, free);
            }
        }
        HirExprKind::Lambda { params, body, .. } => {
            for name in lambda_capture_names(params, body) {
                if !bound.contains(&name) {
                    free.insert(name);
                }
            }
        }
        HirExprKind::Catch { tag, body } => {
            collect_free_lexicals(tag, bound, free);
            collect_free_lexicals(body, bound, free);
        }
        HirExprKind::Throw { tag, value } => {
            collect_free_lexicals(tag, bound, free);
            collect_free_lexicals(value, bound, free);
        }
        HirExprKind::ConditionCase { body, handlers, .. } => {
            collect_free_lexicals(body, bound, free);
            for handler in handlers {
                collect_free_lexicals(&handler.body, bound, free);
            }
        }
        HirExprKind::UnwindProtect { body, cleanup } => {
            collect_free_lexicals(body, bound, free);
            collect_free_lexicals(cleanup, bound, free);
        }
        HirExprKind::Funcall { callee, args }
        | HirExprKind::Apply { callee, args }
        | HirExprKind::CallValue { callee, args } => {
            collect_free_lexicals(callee, bound, free);
            for arg in args {
                collect_free_lexicals(arg, bound, free);
            }
        }
        HirExprKind::CallNamed { args, .. } => {
            for arg in args {
                collect_free_lexicals(arg, bound, free);
            }
        }
    }
}

struct SsaBuilder {
    function: SsaFunction,
    current_block: BlockId,
    mutable_lexicals: IndexSet<String>,
    cell_lexicals: IndexSet<String>,
    diagnostics: Vec<Diagnostic>,
}

impl SsaBuilder {
    fn new(name: Option<String>) -> Self {
        let mut blocks = PrimaryMap::new();
        let entry = blocks.push(SsaBlock {
            params: Vec::new(),
            instructions: Vec::new(),
            terminator: SsaTerminator::Unreachable,
        });
        Self {
            function: SsaFunction {
                name,
                values: PrimaryMap::new(),
                blocks,
                entry: Some(entry),
            },
            current_block: entry,
            mutable_lexicals: IndexSet::new(),
            cell_lexicals: IndexSet::new(),
            diagnostics: Vec::new(),
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> Option<ValueId> {
        match &expr.kind {
            HirExprKind::Const(value) => {
                let value = match value {
                    HirConst::Nil => SsaConst::Nil,
                    HirConst::True => SsaConst::True,
                    HirConst::Int(value) => SsaConst::Int(*value),
                    HirConst::Float(value) => SsaConst::Float(*value),
                    HirConst::String(value) => SsaConst::String(value.clone()),
                    HirConst::Char(value) => SsaConst::Char(*value),
                };
                Some(self.emit_value(SsaInstKind::Const(value), Effects::pure()))
            }
            HirExprKind::Quote(form) => Some(self.emit_value(
                SsaInstKind::Quote((**form).clone()),
                Effects::single(Effect::Allocate),
            )),
            HirExprKind::FunctionQuote(form) => Some(self.emit_value(
                SsaInstKind::FunctionQuote((**form).clone()),
                Effects::pure(),
            )),
            HirExprKind::LexicalGet(name) => Some(self.emit_lexical_get(name)),
            HirExprKind::SymbolGet(name) => Some(self.emit_value(
                SsaInstKind::SymbolGet(name.clone()),
                Effects::single(Effect::ReadSymbol),
            )),
            HirExprKind::LexicalSet { name, value } => {
                let value = self.lower_expr(value)?;
                Some(self.emit_lexical_set(name, value))
            }
            HirExprKind::SymbolSet { name, value } => {
                let value = self.lower_expr(value)?;
                Some(self.emit_value(
                    SsaInstKind::SymbolSet {
                        name: name.clone(),
                        value,
                    },
                    Effects::single(Effect::WriteSymbol),
                ))
            }
            HirExprKind::If {
                test,
                then_expr,
                else_expr,
            } => self.lower_if(test, then_expr, else_expr),
            HirExprKind::Progn(exprs) => self.lower_progn(exprs),
            HirExprKind::Let {
                declarations,
                bindings,
                sequential,
                body,
                ..
            } => {
                for declaration in declarations {
                    self.lower_declaration(declaration);
                }
                let mut dynamic_bind_count = 0;
                if *sequential {
                    for binding in bindings {
                        let value = self.lower_expr(&binding.init)?;
                        self.emit_binding(binding.mode, binding.name.clone(), value);
                        if binding.mode == BindingMode::Dynamic {
                            dynamic_bind_count += 1;
                        }
                    }
                } else {
                    let mut lowered_bindings = Vec::with_capacity(bindings.len());
                    for binding in bindings {
                        let value = self.lower_expr(&binding.init)?;
                        lowered_bindings.push((binding.mode, binding.name.clone(), value));
                    }
                    for (mode, name, value) in lowered_bindings {
                        self.emit_binding(mode, name, value);
                        if mode == BindingMode::Dynamic {
                            dynamic_bind_count += 1;
                        }
                    }
                }
                let body_value = self.lower_expr(body);
                if dynamic_bind_count > 0 && body_value.is_some() {
                    self.emit_no_result(SsaInstKind::UnbindDynamic {
                        count: dynamic_bind_count,
                    });
                }
                body_value
            }
            HirExprKind::Lambda {
                params,
                declarations,
                body,
            } => {
                let capture_specs = lambda_capture_specs(params, body, &self.mutable_lexicals);
                let captures = capture_specs
                    .iter()
                    .map(|capture| self.emit_lexical_capture(&capture.name))
                    .collect::<Vec<_>>();
                Some(self.emit_value(
                    SsaInstKind::Lambda {
                        template: crate::ssa::SsaLambdaTemplate {
                            params: params.clone(),
                            captures: capture_specs,
                            declarations: declarations.clone(),
                            body: body.clone(),
                        },
                        captures,
                    },
                    Effects::new([Effect::Allocate, Effect::MayGc]),
                ))
            }
            HirExprKind::Declare(declarations) => {
                for declaration in declarations {
                    self.lower_declaration(declaration);
                }
                Some(self.emit_value(SsaInstKind::Const(SsaConst::Nil), Effects::pure()))
            }
            HirExprKind::Catch { tag, body } => {
                let tag = self.lower_expr(tag)?;
                self.emit_no_result(SsaInstKind::CatchBegin { tag });
                let result = self.lower_expr(body);
                self.emit_no_result(SsaInstKind::CatchEnd);
                result
            }
            HirExprKind::Throw { tag, value } => {
                let tag = self.lower_expr(tag)?;
                let value = self.lower_expr(value)?;
                self.emit_no_result(SsaInstKind::Throw { tag, value });
                self.set_terminator(SsaTerminator::Unreachable);
                None
            }
            HirExprKind::ConditionCase {
                var,
                body,
                handlers,
            } => {
                self.emit_no_result(SsaInstKind::ConditionCaseBegin { var: var.clone() });
                let body_value = self.lower_expr(body);
                for handler in handlers {
                    self.emit_no_result(SsaInstKind::ConditionCaseHandler {
                        pattern: handler.pattern.clone(),
                    });
                    let _ = self.lower_expr(&handler.body);
                }
                self.emit_no_result(SsaInstKind::ConditionCaseEnd);
                body_value
            }
            HirExprKind::UnwindProtect { body, cleanup } => {
                self.emit_no_result(SsaInstKind::UnwindProtectBegin);
                let value = self.lower_expr(body);
                self.emit_no_result(SsaInstKind::UnwindProtectCleanup);
                let _ = self.lower_expr(cleanup);
                self.emit_no_result(SsaInstKind::UnwindProtectEnd);
                value
            }
            HirExprKind::Funcall { callee, args } => {
                let callee = self.lower_expr(callee)?;
                let args = self.lower_exprs(args)?;
                Some(self.emit_value(
                    SsaInstKind::Funcall { callee, args },
                    Effects::conservative_call(),
                ))
            }
            HirExprKind::Apply { callee, args } => {
                let callee = self.lower_expr(callee)?;
                let args = self.lower_exprs(args)?;
                Some(self.emit_value(
                    SsaInstKind::Apply { callee, args },
                    Effects::conservative_call(),
                ))
            }
            HirExprKind::CallNamed { name, args } => {
                let args = self.lower_exprs(args)?;
                Some(self.emit_value(
                    SsaInstKind::CallNamed {
                        name: name.clone(),
                        args,
                    },
                    Effects::conservative_call(),
                ))
            }
            HirExprKind::CallValue { callee, args } => {
                let callee = self.lower_expr(callee)?;
                let args = self.lower_exprs(args)?;
                Some(self.emit_value(
                    SsaInstKind::Funcall { callee, args },
                    Effects::conservative_call(),
                ))
            }
        }
    }

    fn lower_if(
        &mut self,
        test: &HirExpr,
        then_expr: &HirExpr,
        else_expr: &HirExpr,
    ) -> Option<ValueId> {
        let test = self.lower_expr(test)?;
        let then_block = self.create_block();
        let else_block = self.create_block();
        let merge_block = self.create_block();
        let merge_value = self.append_block_param(merge_block, Some("if.result".to_string()));
        self.set_terminator(SsaTerminator::BranchIfNil {
            test,
            then_target: else_block,
            then_args: Vec::new(),
            else_target: then_block,
            else_args: Vec::new(),
        });

        self.current_block = then_block;
        let then_value = self.lower_expr(then_expr)?;
        self.set_terminator(SsaTerminator::Jump {
            target: merge_block,
            args: vec![then_value],
        });

        self.current_block = else_block;
        let else_value = self.lower_expr(else_expr)?;
        self.set_terminator(SsaTerminator::Jump {
            target: merge_block,
            args: vec![else_value],
        });

        self.current_block = merge_block;
        Some(merge_value)
    }

    fn lower_progn(&mut self, exprs: &[HirExpr]) -> Option<ValueId> {
        let mut last = None;
        for expr in exprs {
            last = self.lower_expr(expr);
        }
        last.or_else(|| Some(self.emit_value(SsaInstKind::Const(SsaConst::Nil), Effects::pure())))
    }

    fn lower_exprs(&mut self, exprs: &[HirExpr]) -> Option<Vec<ValueId>> {
        exprs
            .iter()
            .map(|expr| self.lower_expr(expr))
            .collect::<Option<Vec<_>>>()
    }

    fn lower_declaration(&mut self, declaration: &HirDeclaration) {
        match declaration {
            HirDeclaration::Special(names) => {
                self.emit_no_result(SsaInstKind::DeclareSpecial(names.clone()));
            }
            HirDeclaration::Unknown { .. } => {}
        }
    }

    fn emit_lexical_get(&mut self, name: &str) -> ValueId {
        let binding = self.emit_lexical_capture(name);
        if self.cell_lexicals.contains(name) {
            self.emit_value(
                SsaInstKind::LexicalCellGet { cell: binding },
                Effects::single(Effect::ReadLexical),
            )
        } else {
            binding
        }
    }

    fn emit_lexical_capture(&mut self, name: &str) -> ValueId {
        self.emit_value(
            SsaInstKind::LexicalGet(name.to_string()),
            Effects::single(Effect::ReadLexical),
        )
    }

    fn emit_lexical_set(&mut self, name: &str, value: ValueId) -> ValueId {
        if self.cell_lexicals.contains(name) {
            let cell = self.emit_lexical_capture(name);
            self.emit_value(
                SsaInstKind::LexicalCellSet { cell, value },
                Effects::single(Effect::WriteLexical),
            )
        } else {
            self.emit_value(
                SsaInstKind::LexicalSet {
                    name: name.to_string(),
                    value,
                },
                Effects::single(Effect::WriteLexical),
            )
        }
    }

    fn emit_binding(&mut self, mode: BindingMode, name: String, value: ValueId) {
        match mode {
            BindingMode::Lexical => {
                let value = self.maybe_box_lexical(&name, value);
                self.emit_no_result(SsaInstKind::BindLexical { name, value });
            }
            BindingMode::Dynamic => self.emit_no_result(SsaInstKind::BindDynamic { name, value }),
        }
    }

    fn maybe_box_lexical(&mut self, name: &str, value: ValueId) -> ValueId {
        if self.cell_lexicals.contains(name) {
            self.emit_value(
                SsaInstKind::MakeLexicalCell { initial: value },
                Effects::new([Effect::Allocate, Effect::MayGc]),
            )
        } else {
            value
        }
    }

    fn create_block(&mut self) -> BlockId {
        self.function.blocks.push(SsaBlock {
            params: Vec::new(),
            instructions: Vec::new(),
            terminator: SsaTerminator::Unreachable,
        })
    }

    fn append_block_param(&mut self, block: BlockId, name: Option<String>) -> ValueId {
        let index = self.function.blocks[block].params.len();
        let value = self.function.values.push(SsaValue {
            kind: SsaValueKind::BlockParam { block, index, name },
        });
        self.function.blocks[block].params.push(value);
        value
    }

    fn emit_value(&mut self, kind: SsaInstKind, effects: Effects) -> ValueId {
        let block = self.current_block;
        let inst = self.function.blocks[block].instructions.len();
        let value = self.function.values.push(SsaValue {
            kind: SsaValueKind::InstResult { block, inst },
        });
        self.function.blocks[block].instructions.push(SsaInst {
            result: Some(value),
            kind,
            effects,
        });
        value
    }

    fn emit_no_result(&mut self, kind: SsaInstKind) {
        let effects = match &kind {
            SsaInstKind::BindDynamic { .. } => Effects::single(Effect::BindDynamic),
            SsaInstKind::UnbindDynamic { .. } => Effects::single(Effect::UnbindDynamic),
            SsaInstKind::DeclareSpecial(_) => Effects::pure(),
            SsaInstKind::CatchBegin { .. }
            | SsaInstKind::CatchEnd
            | SsaInstKind::ConditionCaseBegin { .. }
            | SsaInstKind::ConditionCaseHandler { .. }
            | SsaInstKind::ConditionCaseEnd
            | SsaInstKind::UnwindProtectBegin
            | SsaInstKind::UnwindProtectCleanup
            | SsaInstKind::UnwindProtectEnd => Effects::new([Effect::MayThrow, Effect::MaySignal]),
            SsaInstKind::Throw { .. } => Effects::single(Effect::MayThrow),
            _ => Effects::pure(),
        };
        self.function.blocks[self.current_block]
            .instructions
            .push(SsaInst {
                result: None,
                kind,
                effects,
            });
    }

    fn set_terminator(&mut self, terminator: SsaTerminator) {
        self.function.blocks[self.current_block].terminator = terminator;
    }
}
