use std::collections::HashMap;

use neovm_compiler::diagnostic::Diagnostic;
use neovm_compiler::ids::{FunctionId, RegId};
use neovm_compiler::regir::{RegFunction, RegInstKind, RegModule, RegTerminator};
use neovm_compiler::ssa::SsaConst;

use crate::{LispValue, Runtime};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectInterpResult {
    pub value: Option<LispValue>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn execute_module_with_args(
    module: &RegModule,
    args: &[LispValue],
    runtime: &mut Runtime,
) -> ObjectInterpResult {
    let functions_by_name = functions_by_name(module);
    let mut fuel = 100_000usize;
    execute_module_entry(module, &functions_by_name, args, runtime, &mut fuel)
}

fn execute_module_entry(
    module: &RegModule,
    functions_by_name: &HashMap<String, FunctionId>,
    args: &[LispValue],
    runtime: &mut Runtime,
    fuel: &mut usize,
) -> ObjectInterpResult {
    let Some(entry) = module.entry else {
        return ObjectInterpResult {
            value: None,
            diagnostics: vec![Diagnostic::error(
                "object interpreter requires a module entry function",
            )],
        };
    };
    let Some(function) = module.functions.get(entry) else {
        return ObjectInterpResult {
            value: None,
            diagnostics: vec![Diagnostic::error(format!(
                "object interpreter references unknown module entry function {entry:?}"
            ))],
        };
    };
    execute_with_module(function, args, module, functions_by_name, runtime, fuel)
}

fn execute_with_module(
    function: &RegFunction,
    args: &[LispValue],
    module: &RegModule,
    functions_by_name: &HashMap<String, FunctionId>,
    runtime: &mut Runtime,
    fuel: &mut usize,
) -> ObjectInterpResult {
    let interpreter = Interpreter {
        function,
        registers: HashMap::new(),
        lexicals: HashMap::new(),
        module,
        functions_by_name,
        runtime,
        fuel,
        diagnostics: Vec::new(),
    };
    interpreter.execute(args)
}

struct Interpreter<'a, 'runtime, 'fuel> {
    function: &'a RegFunction,
    registers: HashMap<RegId, LispValue>,
    lexicals: HashMap<String, LispValue>,
    module: &'a RegModule,
    functions_by_name: &'a HashMap<String, FunctionId>,
    runtime: &'runtime mut Runtime,
    fuel: &'fuel mut usize,
    diagnostics: Vec<Diagnostic>,
}

impl Interpreter<'_, '_, '_> {
    fn execute(mut self, args: &[LispValue]) -> ObjectInterpResult {
        if args.len() != self.function.entry_params.len() {
            self.error(format!(
                "object interpreter expected {} arguments, got {}",
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
            self.error("object interpreter requires an entry block");
            return self.finish(None);
        };

        loop {
            if *self.fuel == 0 {
                self.error("object interpreter exhausted execution fuel");
                return self.finish(None);
            }
            *self.fuel -= 1;

            let Some(body) = self.function.blocks.get(block) else {
                self.error(format!(
                    "object interpreter entered unknown block {block:?}"
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
                    block = if test.is_nil() {
                        *then_target
                    } else {
                        *else_target
                    };
                }
                RegTerminator::Unreachable => {
                    self.error("object interpreter reached unreachable terminator");
                    return self.finish(None);
                }
            }
        }
    }

    fn execute_inst(&mut self, kind: &RegInstKind) -> bool {
        match kind {
            RegInstKind::LoadConst { dst, value } => {
                let Some(value) = const_value(value) else {
                    self.unsupported("heap constants require runtime materialization");
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
                let Some(value) = self.execute_named_call(name, &args) else {
                    return false;
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
                self.unsupported("instruction requires object runtime support");
                return false;
            }
        }
        true
    }

    fn execute_named_call(&mut self, name: &str, args: &[LispValue]) -> Option<LispValue> {
        if let Some(value) = self.execute_primitive_call(name, args) {
            return value;
        }
        if let Some(function_id) = self.functions_by_name.get(name).copied() {
            return self.execute_module_call(function_id, args);
        }
        self.unsupported(format!("named call `{name}` requires runtime support"));
        None
    }

    fn execute_primitive_call(
        &mut self,
        name: &str,
        args: &[LispValue],
    ) -> Option<Option<LispValue>> {
        let value = match name {
            "cons" => self
                .exact_arity(name, args, 2)
                .map(|_| self.runtime.cons(args[0], args[1])),
            "car" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.car(args[0]);
                self.runtime_value(result)
            }),
            "cdr" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.cdr(args[0]);
                self.runtime_value(result)
            }),
            "setcar" => self.exact_arity(name, args, 2).and_then(|_| {
                let result = self.runtime.set_car(args[0], args[1]);
                self.runtime_value(result)
            }),
            "setcdr" => self.exact_arity(name, args, 2).and_then(|_| {
                let result = self.runtime.set_cdr(args[0], args[1]);
                self.runtime_value(result)
            }),
            "eq" | "eql" => self
                .exact_arity(name, args, 2)
                .map(|_| bool_value(args[0] == args[1])),
            "equal" => self
                .exact_arity(name, args, 2)
                .map(|_| bool_value(self.runtime.equal(args[0], args[1]))),
            "consp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_cons(args[0]))),
            "listp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_nil() || self.runtime.is_cons(args[0]))),
            "numberp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_fixnum())),
            "symbolp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_nil() || args[0].is_true())),
            "stringp" => self.exact_arity(name, args, 1).map(|_| LispValue::NIL),
            "not" | "null" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_nil())),
            "list" => Some(make_list(self.runtime, args.iter().copied())),
            "length" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.list_length(args[0]))
                .and_then(|length| i64::try_from(length).ok())
                .and_then(|length| self.fixnum(length, "length")),
            "reverse" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.list_values(args[0]))
                .map(|values| make_list(self.runtime, values.iter().rev().copied())),
            "append" => self.append(args),
            "nth" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.nth(args[0], args[1])),
            "memq" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.memq(args[0], args[1])),
            "+" => self.fixnum_fold(name, args, 0, i64::checked_add),
            "*" => self.fixnum_fold(name, args, 1, i64::checked_mul),
            "-" => self.fixnum_sub(args),
            "/" => self.fixnum_div(args),
            "1+" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.fixnum_arg(name, args[0]))
                .and_then(|value| value.checked_add(1))
                .and_then(|value| self.fixnum(value, name)),
            "1-" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.fixnum_arg(name, args[0]))
                .and_then(|value| value.checked_sub(1))
                .and_then(|value| self.fixnum(value, name)),
            "=" => self.fixnum_compare(args, |left, right| left == right),
            "<" => self.fixnum_compare(args, |left, right| left < right),
            "<=" => self.fixnum_compare(args, |left, right| left <= right),
            ">" => self.fixnum_compare(args, |left, right| left > right),
            ">=" => self.fixnum_compare(args, |left, right| left >= right),
            "message" => Some(args.last().copied().unwrap_or(LispValue::NIL)),
            "print" | "prin1" => self.exact_arity(name, args, 1).map(|_| args[0]),
            _ => return None,
        };
        Some(value)
    }

    fn execute_module_call(
        &mut self,
        function_id: FunctionId,
        args: &[LispValue],
    ) -> Option<LispValue> {
        let Some(function) = self.module.functions.get(function_id) else {
            self.error(format!(
                "object interpreter references unknown function {function_id:?}"
            ));
            return None;
        };
        let result = execute_with_module(
            function,
            args,
            self.module,
            self.functions_by_name,
            self.runtime,
            &mut *self.fuel,
        );
        self.diagnostics.extend(result.diagnostics);
        result.value
    }

    fn append(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let Some((last, prefixes)) = args.split_last() else {
            return Some(LispValue::NIL);
        };
        let mut result = *last;
        for list in prefixes.iter().rev().copied() {
            let values = self.list_values(list)?;
            for value in values.into_iter().rev() {
                result = self.runtime.cons(value, result);
            }
        }
        Some(result)
    }

    fn nth(&mut self, index: LispValue, list: LispValue) -> Option<LispValue> {
        let index = self.fixnum_arg("nth", index)?;
        if index < 0 {
            return Some(LispValue::NIL);
        }
        let mut current = list;
        for _ in 0..index {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            let result = self.runtime.cdr(current);
            current = self.runtime_value(result)?;
        }
        let result = self.runtime.car(current);
        self.runtime_value(result)
    }

    fn memq(&mut self, needle: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            let result = self.runtime.car(current);
            let car = self.runtime_value(result)?;
            if car == needle {
                return Some(current);
            }
            let result = self.runtime.cdr(current);
            current = self.runtime_value(result)?;
        }
    }

    fn list_length(&mut self, list: LispValue) -> Option<usize> {
        let mut current = list;
        let mut len = 0usize;
        loop {
            if current.is_nil() {
                return Some(len);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!(
                    "primitive `length` expected a proper list, got {}",
                    self.runtime.format_value(current)
                ));
                return None;
            }
            len += 1;
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn list_values(&mut self, list: LispValue) -> Option<Vec<LispValue>> {
        let mut current = list;
        let mut values = Vec::new();
        loop {
            if current.is_nil() {
                return Some(values);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!(
                    "expected a proper list, got {}",
                    self.runtime.format_value(current)
                ));
                return None;
            }
            values.push(self.runtime.car(current).ok()?);
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn fixnum_fold(
        &mut self,
        name: &str,
        args: &[LispValue],
        initial: i64,
        op: fn(i64, i64) -> Option<i64>,
    ) -> Option<LispValue> {
        let mut acc = initial;
        for arg in args {
            let value = self.fixnum_arg(name, *arg)?;
            acc = match op(acc, value) {
                Some(value) => value,
                None => {
                    self.error(format!("integer overflow in primitive `{name}`"));
                    return None;
                }
            };
        }
        self.fixnum(acc, name)
    }

    fn fixnum_sub(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let Some((first, rest)) = args.split_first() else {
            self.error("primitive `-` requires at least one argument");
            return None;
        };
        let first = self.fixnum_arg("-", *first)?;
        let value = if rest.is_empty() {
            first.checked_neg()
        } else {
            rest.iter().try_fold(first, |acc, value| {
                acc.checked_sub(self.fixnum_arg("-", *value)?)
            })
        };
        match value {
            Some(value) => self.fixnum(value, "-"),
            None => {
                self.error("integer overflow in primitive `-`");
                None
            }
        }
    }

    fn fixnum_div(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let Some((first, rest)) = args.split_first() else {
            self.error("primitive `/` requires at least one argument");
            return None;
        };
        let first = self.fixnum_arg("/", *first)?;
        let value = rest.iter().try_fold(first, |acc, value| {
            let value = self.fixnum_arg("/", *value)?;
            if value == 0 {
                self.error("division by zero in primitive `/`");
                return None;
            }
            acc.checked_div(value)
        });
        match value {
            Some(value) => self.fixnum(value, "/"),
            None => None,
        }
    }

    fn fixnum_compare(
        &mut self,
        args: &[LispValue],
        compare: impl Fn(i64, i64) -> bool,
    ) -> Option<LispValue> {
        let values = args
            .iter()
            .map(|value| self.fixnum_arg("comparison", *value))
            .collect::<Option<Vec<_>>>()?;
        Some(bool_value(
            values.windows(2).all(|pair| compare(pair[0], pair[1])),
        ))
    }

    fn fixnum_arg(&mut self, name: &str, value: LispValue) -> Option<i64> {
        let Some(value) = value.as_fixnum() else {
            self.error(format!("primitive `{name}` expected a fixnum"));
            return None;
        };
        Some(value)
    }

    fn fixnum(&mut self, value: i64, name: &str) -> Option<LispValue> {
        let Some(value) = LispValue::from_fixnum(value) else {
            self.error(format!("integer overflow in primitive `{name}`"));
            return None;
        };
        Some(value)
    }

    fn exact_arity(&mut self, name: &str, args: &[LispValue], arity: usize) -> Option<()> {
        if args.len() == arity {
            return Some(());
        }
        self.error(format!(
            "primitive `{name}` requires {arity} arguments, got {}",
            args.len()
        ));
        None
    }

    fn runtime_error(&mut self, error: crate::RuntimeError) {
        self.error(error.to_string());
    }

    fn runtime_value(
        &mut self,
        result: Result<LispValue, crate::RuntimeError>,
    ) -> Option<LispValue> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.runtime_error(error);
                None
            }
        }
    }

    fn get_many(&mut self, regs: &[RegId]) -> Option<Vec<LispValue>> {
        regs.iter().map(|reg| self.get(*reg)).collect()
    }

    fn get(&mut self, reg: RegId) -> Option<LispValue> {
        let Some(value) = self.registers.get(&reg).copied() else {
            self.error(format!("read from uninitialized register {reg:?}"));
            return None;
        };
        Some(value)
    }

    fn set(&mut self, reg: RegId, value: LispValue) {
        self.registers.insert(reg, value);
    }

    fn unsupported(&mut self, reason: impl Into<String>) {
        self.error(format!(
            "unsupported object interpreter operation: {}",
            reason.into()
        ));
    }

    fn error(&mut self, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(message));
    }

    fn finish(self, value: Option<LispValue>) -> ObjectInterpResult {
        ObjectInterpResult {
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

fn make_list(runtime: &mut Runtime, values: impl IntoIterator<Item = LispValue>) -> LispValue {
    values
        .into_iter()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .fold(LispValue::NIL, |tail, value| runtime.cons(value, tail))
}

fn bool_value(value: bool) -> LispValue {
    if value {
        LispValue::TRUE
    } else {
        LispValue::NIL
    }
}

fn const_value(value: &SsaConst) -> Option<LispValue> {
    match value {
        SsaConst::Nil => Some(LispValue::NIL),
        SsaConst::True => Some(LispValue::TRUE),
        SsaConst::Int(value) => LispValue::from_fixnum(*value),
        SsaConst::Char(value) => {
            let code: u32 = (*value).try_into().ok()?;
            char::from_u32(code).map(LispValue::from_char)
        }
        SsaConst::Float(_) | SsaConst::String(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use neovm_compiler::compile_source;

    use crate::object_interp::execute_module_with_args;
    use crate::{LispValue, Runtime};

    fn execute(source: &str) -> (Option<LispValue>, Runtime) {
        let artifact = compile_source("object.el", source);
        assert_eq!(artifact.diagnostics, Vec::new());
        let regir = artifact.regir.expect("RegIR");
        let mut runtime = Runtime::new();
        let result = execute_module_with_args(&regir, &[], &mut runtime);
        assert_eq!(result.diagnostics, Vec::new());
        (result.value, runtime)
    }

    #[test]
    fn executes_pairs_and_mutation() {
        let (value, _) =
            execute(";;; -*- lexical-binding: t; -*-\n(let ((p (cons 1 2))) (setcar p 9) (car p))");
        assert_eq!(value, Some(LispValue::expect_fixnum(9)));
    }

    #[test]
    fn executes_list_operations() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(nth 1 (reverse (append (list 1) (list 2 3))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_equality_predicates_and_memq() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((xs (list 1 2 3))) (if (consp xs) (if (memq 2 xs) 7 0) 0))",
        );
        drop(runtime);
        assert_eq!(value, Some(LispValue::expect_fixnum(7)));
    }
}
