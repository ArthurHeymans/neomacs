use std::collections::HashMap;

use crate::diagnostic::Diagnostic;
use crate::effects::{Effect, Effects};
use crate::hir::{BindingMode, HirConst, HirDeclaration, HirExpr, HirExprKind, HirItem, HirModule};
use crate::ids::{BlockId, PrimaryMap, RegBlockId, RegId, ValueId};
use crate::regir::{Reg, RegBlock, RegFunction, RegInst, RegInstKind, RegKind, RegTerminator};
use crate::safepoint::SafepointEntry;
use crate::ssa::{
    SsaBlock, SsaConst, SsaFunction, SsaInst, SsaInstKind, SsaTerminator, SsaValue, SsaValueKind,
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
                let value = builder.lower_expr(expr);
                builder.set_terminator(SsaTerminator::Return(value));
            }
            HirItem::Defun(defun) => {
                if builder.function.name.is_none() {
                    builder.function.name = Some(defun.name.clone());
                }
                for declaration in &defun.declarations {
                    builder.lower_declaration(declaration);
                }
                for param in &defun.params {
                    let value =
                        builder.append_block_param(builder.current_block, Some(param.clone()));
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

struct RegLowerer<'a> {
    ssa: &'a SsaFunction,
    function: RegFunction,
    block_map: HashMap<BlockId, RegBlockId>,
    value_map: HashMap<ValueId, RegId>,
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

        Self {
            ssa,
            function,
            block_map,
            value_map,
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
            for inst in &block.instructions {
                self.lower_inst(reg_block, inst);
            }
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
            SsaInstKind::LexicalGet(name) => RegInstKind::LexicalGet {
                dst: self.result_reg(inst),
                name: name.clone(),
            },
            SsaInstKind::LexicalSet { name, value } => RegInstKind::LexicalSet {
                dst: self.result_reg(inst),
                name: name.clone(),
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
            SsaInstKind::Quote(_)
                | SsaInstKind::SymbolGet(_)
                | SsaInstKind::SymbolSet { .. }
                | SsaInstKind::BindDynamic { .. }
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
        let live_roots = self
            .function
            .registers
            .iter()
            .map(|(reg, _)| reg)
            .collect::<Vec<_>>();
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

struct SsaBuilder {
    function: SsaFunction,
    current_block: BlockId,
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
            HirExprKind::LexicalGet(name) => Some(self.emit_value(
                SsaInstKind::LexicalGet(name.clone()),
                Effects::single(Effect::ReadLexical),
            )),
            HirExprKind::SymbolGet(name) => Some(self.emit_value(
                SsaInstKind::SymbolGet(name.clone()),
                Effects::single(Effect::ReadSymbol),
            )),
            HirExprKind::LexicalSet { name, value } => {
                let value = self.lower_expr(value)?;
                Some(self.emit_value(
                    SsaInstKind::LexicalSet {
                        name: name.clone(),
                        value,
                    },
                    Effects::single(Effect::ReadLexical),
                ))
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
                body,
                ..
            } => {
                for declaration in declarations {
                    self.lower_declaration(declaration);
                }
                for binding in bindings {
                    let value = self.lower_expr(&binding.init)?;
                    match binding.mode {
                        BindingMode::Lexical => self.emit_no_result(SsaInstKind::BindLexical {
                            name: binding.name.clone(),
                            value,
                        }),
                        BindingMode::Dynamic => self.emit_no_result(SsaInstKind::BindDynamic {
                            name: binding.name.clone(),
                            value,
                        }),
                    }
                }
                self.lower_expr(body)
            }
            HirExprKind::Lambda { .. } => {
                self.diagnostics.push(
                    Diagnostic::error(
                        "HIR to SSA lowering for lambda values is not implemented yet",
                    )
                    .with_span(expr.span),
                );
                None
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
