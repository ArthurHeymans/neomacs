use std::collections::HashMap;

use cranelift_entity::EntityRef;

use crate::diagnostic::Diagnostic;
use crate::ids::FunctionId;
use crate::regir::{RegFunction, RegInstKind, RegModule, RegTerminator};
use crate::ssa::SsaConst;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterpResult {
    pub value: Option<i64>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn execute(function: &RegFunction) -> InterpResult {
    execute_with_args(function, &[])
}

pub fn execute_module(module: &RegModule) -> InterpResult {
    execute_module_with_args(module, &[])
}

pub fn execute_module_with_args(module: &RegModule, args: &[i64]) -> InterpResult {
    let functions_by_name = functions_by_name(module);
    let mut fuel = 10_000usize;
    execute_module_entry(module, &functions_by_name, args, &mut fuel)
}

fn execute_module_entry(
    module: &RegModule,
    functions_by_name: &HashMap<String, FunctionId>,
    args: &[i64],
    fuel: &mut usize,
) -> InterpResult {
    let Some(entry) = module.entry else {
        return InterpResult {
            value: None,
            diagnostics: vec![Diagnostic::error(
                "Register IR interpreter requires a module entry function",
            )],
        };
    };
    let Some(function) = module.functions.get(entry) else {
        return InterpResult {
            value: None,
            diagnostics: vec![Diagnostic::error(format!(
                "Register IR interpreter references unknown module entry function {entry:?}"
            ))],
        };
    };
    execute_with_module(function, args, Some(module), Some(functions_by_name), fuel)
}

pub fn execute_with_args(function: &RegFunction, args: &[i64]) -> InterpResult {
    let mut fuel = 10_000usize;
    execute_with_module(function, args, None, None, &mut fuel)
}

fn execute_with_module<'ir>(
    function: &'ir RegFunction,
    args: &[i64],
    module: Option<&'ir RegModule>,
    functions_by_name: Option<&'ir HashMap<String, FunctionId>>,
    fuel: &mut usize,
) -> InterpResult {
    let interpreter = Interpreter {
        function,
        registers: vec![None; function.registers.len()],
        lexicals: HashMap::new(),
        module,
        functions_by_name,
        fuel,
        diagnostics: Vec::new(),
    };
    interpreter.execute(args)
}

struct Interpreter<'ir, 'fuel> {
    function: &'ir RegFunction,
    registers: Vec<Option<i64>>,
    lexicals: HashMap<String, i64>,
    module: Option<&'ir RegModule>,
    functions_by_name: Option<&'ir HashMap<String, FunctionId>>,
    fuel: &'fuel mut usize,
    diagnostics: Vec<Diagnostic>,
}

enum PrimitiveCallResult {
    Value(i64),
    Unknown,
    Error,
}

impl Interpreter<'_, '_> {
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

        loop {
            if *self.fuel == 0 {
                self.error("Register IR interpreter exhausted execution fuel");
                return self.finish(None);
            }
            *self.fuel -= 1;

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
            RegInstKind::CallNamed { dst, name, args } => {
                let Some(args) = self.get_many(args) else {
                    return false;
                };
                let value = match self.execute_primitive_call(name, &args) {
                    PrimitiveCallResult::Value(value) => value,
                    PrimitiveCallResult::Unknown => {
                        let Some(value) = self.execute_module_call(name, &args) else {
                            return false;
                        };
                        value
                    }
                    PrimitiveCallResult::Error => return false,
                };
                self.set(*dst, value);
            }
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

    fn execute_primitive_call(&mut self, name: &str, args: &[i64]) -> PrimitiveCallResult {
        let value = match name {
            "+" => checked_fold(0, args, i64::checked_add),
            "*" => checked_fold(1, args, i64::checked_mul),
            "-" => match args {
                [] => {
                    self.error("primitive `-` requires at least one argument");
                    return PrimitiveCallResult::Error;
                }
                [value] => value.checked_neg(),
                [first, rest @ ..] => checked_fold(*first, rest, i64::checked_sub),
            },
            "1+" => {
                let Some(value) = exactly_one_arg(name, args, self) else {
                    return PrimitiveCallResult::Error;
                };
                value.checked_add(1)
            }
            "1-" => {
                let Some(value) = exactly_one_arg(name, args, self) else {
                    return PrimitiveCallResult::Error;
                };
                value.checked_sub(1)
            }
            "=" => Some(bool_value(args.windows(2).all(|pair| pair[0] == pair[1]))),
            "<" => Some(bool_value(args.windows(2).all(|pair| pair[0] < pair[1]))),
            "<=" => Some(bool_value(args.windows(2).all(|pair| pair[0] <= pair[1]))),
            ">" => Some(bool_value(args.windows(2).all(|pair| pair[0] > pair[1]))),
            ">=" => Some(bool_value(args.windows(2).all(|pair| pair[0] >= pair[1]))),
            "not" | "null" => {
                let Some(value) = exactly_one_arg(name, args, self) else {
                    return PrimitiveCallResult::Error;
                };
                Some(bool_value(value == 0))
            }
            _ => return PrimitiveCallResult::Unknown,
        };
        match value {
            Some(value) => PrimitiveCallResult::Value(value),
            None => {
                self.error(format!("integer overflow in primitive `{name}`"));
                PrimitiveCallResult::Error
            }
        }
    }

    fn execute_module_call(&mut self, name: &str, args: &[i64]) -> Option<i64> {
        let (Some(module), Some(functions_by_name)) = (self.module, self.functions_by_name) else {
            self.unsupported(format!("named call `{name}` requires runtime support"));
            return None;
        };
        let Some(function_id) = functions_by_name.get(name).copied() else {
            self.unsupported(format!("named call `{name}` requires runtime support"));
            return None;
        };
        let Some(function) = module.functions.get(function_id) else {
            self.error(format!(
                "Register IR interpreter references unknown function {function_id:?}"
            ));
            return None;
        };
        let result = execute_with_module(
            function,
            args,
            Some(module),
            Some(functions_by_name),
            &mut *self.fuel,
        );
        self.diagnostics.extend(result.diagnostics);
        result.value
    }

    fn get_many(&mut self, regs: &[crate::ids::RegId]) -> Option<Vec<i64>> {
        regs.iter().map(|reg| self.get(*reg)).collect()
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

fn functions_by_name(module: &RegModule) -> HashMap<String, FunctionId> {
    module
        .functions
        .iter()
        .filter_map(|(id, function)| function.name.as_ref().map(|name| (name.clone(), id)))
        .collect()
}

fn checked_fold(initial: i64, rest: &[i64], op: impl Fn(i64, i64) -> Option<i64>) -> Option<i64> {
    rest.iter()
        .copied()
        .try_fold(initial, |acc, value| op(acc, value))
}

fn exactly_one_arg(name: &str, args: &[i64], interpreter: &mut Interpreter<'_, '_>) -> Option<i64> {
    match args {
        [value] => Some(*value),
        _ => {
            interpreter.error(format!(
                "primitive `{name}` requires exactly one argument, got {}",
                args.len()
            ));
            None
        }
    }
}

fn bool_value(value: bool) -> i64 {
    if value { 1 } else { 0 }
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
    use crate::interp::{execute, execute_module_with_args, execute_with_args};
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

    #[test]
    fn executes_module_entry_with_arguments() {
        let artifact = compile_source(
            "choose.el",
            ";;; -*- lexical-binding: t; -*-\n(defun choose (x y) (if x x y))",
        );
        let regir = artifact.regir.expect("RegIR module");
        let result = execute_module_with_args(&regir, &[0, 9]);
        assert_eq!(result.diagnostics, Vec::new());
        assert_eq!(result.value, Some(9));
    }

    #[test]
    fn executes_integer_primitives() {
        let artifact = crate::execute_source(
            "arith.el",
            ";;; -*- lexical-binding: t; -*-\n(if (<= (1- 3) 2) (+ 10 (* 2 3)) 0)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(16));
    }

    #[test]
    fn executes_integer_primitive_entry_with_arguments() {
        let artifact = crate::execute_source(
            "entry.el",
            ";;; -*- lexical-binding: t; -*-\n(defun dec-if-positive (x) (if (> x 0) (1- x) 0))",
            &[8],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(7));
    }

    #[test]
    fn executes_named_module_function_call() {
        let artifact = crate::execute_source(
            "module-call.el",
            ";;; -*- lexical-binding: t; -*-\n(defun main (x) (add1 x))\n(defun add1 (n) (1+ n))",
            &[4],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(5));
    }

    #[test]
    fn executes_recursive_module_function_call() {
        let artifact = crate::execute_source(
            "fact.el",
            ";;; -*- lexical-binding: t; -*-\n(defun fact (n) (if (<= n 1) 1 (* n (fact (1- n)))))",
            &[5],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.value, Some(120));
    }
}
