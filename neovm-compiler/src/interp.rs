use std::collections::HashMap;

use cranelift_entity::EntityRef;

use crate::diagnostic::Diagnostic;
use crate::regir::{RegFunction, RegInstKind, RegTerminator};
use crate::ssa::SsaConst;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterpResult {
    pub value: Option<i64>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn execute(function: &RegFunction) -> InterpResult {
    execute_with_args(function, &[])
}

pub fn execute_with_args(function: &RegFunction, args: &[i64]) -> InterpResult {
    let interpreter = Interpreter {
        function,
        registers: vec![None; function.registers.len()],
        lexicals: HashMap::new(),
        diagnostics: Vec::new(),
    };
    interpreter.execute(args)
}

struct Interpreter<'a> {
    function: &'a RegFunction,
    registers: Vec<Option<i64>>,
    lexicals: HashMap<String, i64>,
    diagnostics: Vec<Diagnostic>,
}

impl Interpreter<'_> {
    fn execute(mut self, args: &[i64]) -> InterpResult {
        if args.len() != self.function.entry_params.len() {
            self.error(format!(
                "Register IR interpreter expected {} arguments, got {}",
                self.function.entry_params.len(),
                args.len()
            ));
            return self.finish(None);
        }
        let entry_params = self.function.entry_params.clone();
        for (reg, value) in entry_params.into_iter().zip(args.iter().copied()) {
            self.set(reg, value);
        }

        let Some(mut block) = self.function.entry else {
            self.error("Register IR interpreter requires an entry block");
            return self.finish(None);
        };

        let mut fuel = 10_000usize;
        loop {
            if fuel == 0 {
                self.error("Register IR interpreter exhausted execution fuel");
                return self.finish(None);
            }
            fuel -= 1;

            let Some(body) = self.function.blocks.get(block) else {
                self.error(format!(
                    "Register IR interpreter entered unknown block {block:?}"
                ));
                return self.finish(None);
            };
            for inst in &body.instructions {
                if !self.execute_inst(&inst.kind) {
                    return self.finish(None);
                }
            }
            match &body.terminator {
                RegTerminator::Return(value) => {
                    let value = value.and_then(|reg| self.get(reg));
                    return self.finish(value);
                }
                RegTerminator::Jump { target } => block = *target,
                RegTerminator::BranchIfNil {
                    test,
                    then_target,
                    else_target,
                } => {
                    let Some(test) = self.get(*test) else {
                        return self.finish(None);
                    };
                    block = if test == 0 {
                        *then_target
                    } else {
                        *else_target
                    };
                }
                RegTerminator::Unreachable => {
                    self.error("Register IR interpreter reached unreachable terminator");
                    return self.finish(None);
                }
            }
        }
    }

    fn execute_inst(&mut self, kind: &RegInstKind) -> bool {
        match kind {
            RegInstKind::LoadConst { dst, value } => {
                let Some(value) = const_value(value) else {
                    self.unsupported("heap-allocated constants require runtime materialization");
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::Move { dst, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::LexicalGet { dst, name } => {
                let Some(value) = self.lexicals.get(name).copied() else {
                    self.error(format!("unknown lexical binding `{name}`"));
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::LexicalSet { dst, name, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                self.lexicals.insert(name.clone(), value);
                self.set(*dst, value);
            }
            RegInstKind::BindLexical { name, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                self.lexicals.insert(name.clone(), value);
            }
            RegInstKind::DeclareSpecial { .. } | RegInstKind::Safepoint { .. } => {}
            RegInstKind::Quote { .. }
            | RegInstKind::FunctionQuote { .. }
            | RegInstKind::Lambda { .. }
            | RegInstKind::MakeLexicalCell { .. }
            | RegInstKind::LexicalCellGet { .. }
            | RegInstKind::LexicalCellSet { .. }
            | RegInstKind::SymbolGet { .. }
            | RegInstKind::SymbolSet { .. }
            | RegInstKind::BindDynamic { .. }
            | RegInstKind::UnbindDynamic { .. }
            | RegInstKind::CallNamed { .. }
            | RegInstKind::Funcall { .. }
            | RegInstKind::Apply { .. }
            | RegInstKind::CatchBegin { .. }
            | RegInstKind::CatchEnd
            | RegInstKind::Throw { .. }
            | RegInstKind::ConditionCaseBegin { .. }
            | RegInstKind::ConditionCaseHandler { .. }
            | RegInstKind::ConditionCaseEnd
            | RegInstKind::UnwindProtectBegin
            | RegInstKind::UnwindProtectCleanup
            | RegInstKind::UnwindProtectEnd => {
                self.unsupported("instruction requires runtime support");
                return false;
            }
        }
        true
    }

    fn get(&mut self, reg: crate::ids::RegId) -> Option<i64> {
        let Some(value) = self.registers.get(reg.index()).copied().flatten() else {
            self.error(format!("read from uninitialized register {reg:?}"));
            return None;
        };
        Some(value)
    }

    fn set(&mut self, reg: crate::ids::RegId, value: i64) {
        if let Some(slot) = self.registers.get_mut(reg.index()) {
            *slot = Some(value);
        } else {
            self.error(format!("write to unknown register {reg:?}"));
        }
    }

    fn unsupported(&mut self, reason: impl Into<String>) {
        self.error(format!(
            "unsupported Register IR interpreter operation: {}",
            reason.into()
        ));
    }

    fn error(&mut self, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(message));
    }

    fn finish(self, value: Option<i64>) -> InterpResult {
        InterpResult {
            value,
            diagnostics: self.diagnostics,
        }
    }
}

fn const_value(value: &SsaConst) -> Option<i64> {
    match value {
        SsaConst::Nil => Some(0),
        SsaConst::True => Some(1),
        SsaConst::Int(value) => Some(*value),
        SsaConst::Char(value) => Some(*value),
        SsaConst::Float(_) | SsaConst::String(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_source;
    use crate::interp::{execute, execute_with_args};
    use crate::lower::{hir_to_ssa, ssa_to_regir};
    use crate::verify::{verify_regir, verify_ssa};

    #[test]
    fn executes_constant_return() {
        let artifact = compile_source(
            "constant.el",
            ";;; -*- lexical-binding: t; -*-\n(defun forty-two () 42)",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());
        let regir = ssa_to_regir(&ssa.value);
        assert_eq!(regir.diagnostics, Vec::new());
        assert_eq!(verify_regir(&regir.value), Vec::new());

        let result = execute(&regir.value);
        assert_eq!(result.diagnostics, Vec::new());
        assert_eq!(result.value, Some(42));
    }

    #[test]
    fn executes_lexical_branch_with_arguments() {
        let artifact = compile_source(
            "choose.el",
            ";;; -*- lexical-binding: t; -*-\n(defun choose (x y) (if x x y))",
        );
        let hir = artifact.hir.expect("HIR");
        let ssa = hir_to_ssa(&hir);
        assert_eq!(ssa.diagnostics, Vec::new());
        assert_eq!(verify_ssa(&ssa.value), Vec::new());
        let regir = ssa_to_regir(&ssa.value);
        assert_eq!(regir.diagnostics, Vec::new());
        assert_eq!(verify_regir(&regir.value), Vec::new());

        let nil_result = execute_with_args(&regir.value, &[0, 7]);
        assert_eq!(nil_result.diagnostics, Vec::new());
        assert_eq!(nil_result.value, Some(7));
        let true_result = execute_with_args(&regir.value, &[3, 7]);
        assert_eq!(true_result.diagnostics, Vec::new());
        assert_eq!(true_result.value, Some(3));
    }
}
