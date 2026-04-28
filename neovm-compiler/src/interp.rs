use std::collections::HashMap;

use cranelift_entity::EntityRef;

use crate::diagnostic::Diagnostic;
use crate::expand_value::MacroValue;
use crate::ids::FunctionId;
use crate::regir::{RegFunction, RegInstKind, RegModule, RegTerminator};
use crate::ssa::SsaConst;
use crate::surface::{SurfaceAtom, SurfaceKind};

#[derive(Clone, Debug, PartialEq)]
pub struct InterpResult {
    pub value: Option<RuntimeValue>,
    pub diagnostics: Vec<Diagnostic>,
}

impl InterpResult {
    pub fn as_i64(&self) -> Option<i64> {
        self.value.as_ref().and_then(|v| v.as_i64())
    }
}

pub fn execute(function: &RegFunction) -> InterpResult {
    execute_with_args(function, &[])
}

pub fn execute_module(module: &RegModule) -> InterpResult {
    execute_module_with_args(module, &[])
}

pub fn execute_module_with_args(module: &RegModule, args: &[i64]) -> InterpResult {
    let rv_args: Vec<RuntimeValue> = args.iter().map(|a| RuntimeValue::from_i64(*a)).collect();
    let functions_by_name = functions_by_name(module);
    let mut fuel = 100_000usize;
    execute_module_entry(module, &functions_by_name, &rv_args, &mut fuel)
}

fn execute_module_entry(
    module: &RegModule,
    functions_by_name: &HashMap<String, FunctionId>,
    args: &[RuntimeValue],
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
    let rv_args: Vec<RuntimeValue> = args.iter().map(|a| RuntimeValue::from_i64(*a)).collect();
    let mut fuel = 100_000usize;
    execute_with_module(function, &rv_args, None, None, &mut fuel)
}

fn execute_with_module<'ir>(
    function: &'ir RegFunction,
    args: &[RuntimeValue],
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

/// Rich runtime value type for the RegIR interpreter.
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeValue {
    Val(MacroValue),
}

impl std::fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_string())
    }
}

impl RuntimeValue {
    fn from_i64(n: i64) -> Self {
        RuntimeValue::Val(MacroValue::Int(n))
    }

    fn nil() -> Self {
        RuntimeValue::Val(MacroValue::Nil)
    }

    fn is_nil(&self) -> bool {
        matches!(self, RuntimeValue::Val(MacroValue::Nil))
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            RuntimeValue::Val(MacroValue::Int(n)) => Some(*n),
            _ => None,
        }
    }

    fn as_macro_value(&self) -> &MacroValue {
        match self {
            RuntimeValue::Val(v) => v,
        }
    }

    fn display_string(&self) -> String {
        match self {
            RuntimeValue::Val(v) => format_value(v),
        }
    }
}

fn format_value(v: &MacroValue) -> String {
    match v {
        MacroValue::Nil => "nil".to_string(),
        MacroValue::Int(n) => n.to_string(),
        MacroValue::Symbol(s) => s.clone(),
        MacroValue::String(s) => format!("\"{s}\""),
        MacroValue::Cons(cell) => {
            let mut parts = Vec::new();
            let mut current = v.clone();
            while let MacroValue::Cons(c) = &current {
                parts.push(format_value(&c.car));
                current = c.cdr.clone();
            }
            if current.is_nil() {
                format!("({})", parts.join(" "))
            } else {
                format!("({} . {})", parts.join(" "), format_value(&current))
            }
        }
        MacroValue::Vector(vec) => {
            let parts: Vec<_> = vec.iter().map(format_value).collect();
            format!("[{}]", parts.join(" "))
        }
    }
}

struct Interpreter<'ir, 'fuel> {
    function: &'ir RegFunction,
    registers: Vec<Option<RuntimeValue>>,
    lexicals: HashMap<String, RuntimeValue>,
    module: Option<&'ir RegModule>,
    functions_by_name: Option<&'ir HashMap<String, FunctionId>>,
    fuel: &'fuel mut usize,
    diagnostics: Vec<Diagnostic>,
}

enum PrimResult {
    Value(RuntimeValue),
    Unknown,
    Error,
}

impl Interpreter<'_, '_> {
    fn execute(mut self, args: &[RuntimeValue]) -> InterpResult {
        if args.len() != self.function.entry_params.len() {
            self.error(format!(
                "Register IR interpreter expected {} arguments, got {}",
                self.function.entry_params.len(),
                args.len()
            ));
            return self.finish(None);
        }
        let entry_params = self.function.entry_params.clone();
        for (reg, value) in entry_params.into_iter().zip(args.iter().cloned()) {
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
                    // nil and integer 0 are both falsy in elisp
                    let is_falsy = test.is_nil()
                        || matches!(&test, RuntimeValue::Val(MacroValue::Int(0)));
                    block = if is_falsy {
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
                let rv = match value {
                    SsaConst::Nil => RuntimeValue::Val(MacroValue::Nil),
                    SsaConst::True => RuntimeValue::Val(MacroValue::Symbol("t".into())),
                    SsaConst::Int(n) => RuntimeValue::Val(MacroValue::Int(*n)),
                    SsaConst::Char(c) => RuntimeValue::Val(MacroValue::Int(*c)),
                    SsaConst::Float(f) => RuntimeValue::Val(MacroValue::Int(*f as i64)),
                    SsaConst::String(s) => RuntimeValue::Val(MacroValue::String(s.clone())),
                };
                self.set(*dst, rv);
            }
            RegInstKind::Move { dst, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::LexicalGet { dst, name } => {
                let Some(value) = self.lexicals.get(name).cloned() else {
                    self.error(format!("unknown lexical binding `{name}`"));
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::LexicalSet { dst, name, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                self.lexicals.insert(name.clone(), value.clone());
                self.set(*dst, value);
            }
            RegInstKind::BindLexical { name, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                self.lexicals.insert(name.clone(), value);
            }
            RegInstKind::Quote { dst, form } => {
                let value = surface_to_runtime_value(form);
                self.set(*dst, value);
            }
            RegInstKind::FunctionQuote { dst, form } => {
                let value = surface_to_runtime_value(form);
                self.set(*dst, value);
            }
            RegInstKind::SymbolGet { dst, name } => {
                // Dynamic variable lookup — return nil for unknown symbols
                self.set(*dst, RuntimeValue::Val(MacroValue::Nil));
            }
            RegInstKind::SymbolSet { dst, name, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                // Dynamic variable set — just return the value
                self.set(*dst, value);
            }
            RegInstKind::DeclareSpecial { .. } | RegInstKind::Safepoint { .. } => {}
            RegInstKind::CallNamed { dst, name, args } => {
                let Some(args) = self.get_many(args) else {
                    return false;
                };
                let value = match self.execute_primitive_call(name, &args) {
                    PrimResult::Value(value) => value,
                    PrimResult::Unknown => {
                        let Some(value) = self.execute_module_call(name, &args) else {
                            return false;
                        };
                        value
                    }
                    PrimResult::Error => return false,
                };
                self.set(*dst, value);
            }
            RegInstKind::Funcall { dst, callee, args } => {
                let Some(callee_val) = self.get(*callee) else {
                    return false;
                };
                let Some(args) = self.get_many(args) else {
                    return false;
                };
                let result = match &callee_val {
                    RuntimeValue::Val(MacroValue::Symbol(name)) => {
                        match self.execute_primitive_call(name, &args) {
                            PrimResult::Value(v) => v,
                            PrimResult::Unknown => {
                                match self.execute_module_call(name, &args) {
                                    Some(v) => v,
                                    None => RuntimeValue::nil(),
                                }
                            }
                            PrimResult::Error => return false,
                        }
                    }
                    _ => {
                        self.set(*dst, RuntimeValue::nil());
                        return true;
                    }
                };
                self.set(*dst, result);
            }
            RegInstKind::Lambda { dst, template: _, captures: _ } => {
                // Lambda closures not yet supported — return nil
                self.set(*dst, RuntimeValue::Val(MacroValue::Nil));
            }
            RegInstKind::MakeLexicalCell { dst, initial } => {
                let Some(value) = self.get(*initial) else {
                    return false;
                };
                // Wrap in a cell (cons with marker)
                let cell_val = match &value {
                    RuntimeValue::Val(v) => v.clone(),
                };
                self.set(*dst, RuntimeValue::Val(MacroValue::cons(cell_val, MacroValue::Nil)));
            }
            RegInstKind::LexicalCellGet { dst, cell } => {
                let Some(cell_val) = self.get(*cell) else {
                    return false;
                };
                if let RuntimeValue::Val(MacroValue::Cons(c)) = &cell_val {
                    self.set(*dst, RuntimeValue::Val(c.car.clone()));
                } else {
                    self.set(*dst, RuntimeValue::nil());
                }
            }
            RegInstKind::LexicalCellSet { dst, cell, src } => {
                let Some(_cell_val) = self.get(*cell) else {
                    return false;
                };
                let Some(value) = self.get(*src) else {
                    return false;
                };
                // Cells are immutable in our representation — just set dst to value
                self.set(*dst, value);
            }
            RegInstKind::BindDynamic { name, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                self.lexicals.insert(name.clone(), value);
            }
            RegInstKind::UnbindDynamic { count } => {
                // We don't track dynamic binding scopes — no-op
                let _ = count;
            }
            RegInstKind::CatchBegin { tag } => {
                let Some(_tag) = self.get(*tag) else {
                    return false;
                };
                // Catch/throw not yet implemented — no-op
            }
            RegInstKind::CatchEnd => {}
            RegInstKind::Throw { tag, value } => {
                let Some(_tag) = self.get(*tag) else {
                    return false;
                };
                let Some(_value) = self.get(*value) else {
                    return false;
                };
                self.unsupported("throw requires runtime catch/throw support");
                return false;
            }
            RegInstKind::ConditionCaseBegin { .. } => {}
            RegInstKind::ConditionCaseHandler { .. } => {}
            RegInstKind::ConditionCaseEnd => {}
            RegInstKind::UnwindProtectBegin => {}
            RegInstKind::UnwindProtectCleanup => {}
            RegInstKind::UnwindProtectEnd => {}
            RegInstKind::Apply { dst, callee, args } => {
                // (apply func args) — same as funcall for our purposes
                let Some(callee_val) = self.get(*callee) else {
                    return false;
                };
                let Some(args) = self.get_many(args) else {
                    return false;
                };
                let result = match &callee_val {
                    RuntimeValue::Val(MacroValue::Symbol(name)) => {
                        match self.execute_primitive_call(name, &args) {
                            PrimResult::Value(v) => v,
                            PrimResult::Unknown => {
                                match self.execute_module_call(name, &args) {
                                    Some(v) => v,
                                    None => RuntimeValue::nil(),
                                }
                            }
                            PrimResult::Error => return false,
                        }
                    }
                    _ => RuntimeValue::nil(),
                };
                self.set(*dst, result);
            }
        }
        true
    }

    fn execute_primitive_call(&mut self, name: &str, args: &[RuntimeValue]) -> PrimResult {
        // Try i64 fast path first
        let i64_args: Option<Vec<i64>> = args.iter().map(|a| a.as_i64()).collect();
        if let Some(iargs) = i64_args {
            if let Some(value) = self.execute_i64_primitive(name, &iargs) {
                return match value {
                    Ok(v) => PrimResult::Value(RuntimeValue::from_i64(v)),
                    Err(()) => PrimResult::Error,
                };
            }
        }

        // Rich value primitives
        let value = match name {
            "cons" => {
                if args.len() >= 2 {
                    RuntimeValue::Val(MacroValue::cons(
                        args[0].as_macro_value().clone(),
                        args[1].as_macro_value().clone(),
                    ))
                } else {
                    RuntimeValue::nil()
                }
            }
            "car" | "car-safe" => {
                if let Some(first) = args.first() {
                    RuntimeValue::Val(first.as_macro_value().car())
                } else {
                    RuntimeValue::nil()
                }
            }
            "cdr" | "cdr-safe" => {
                if let Some(first) = args.first() {
                    RuntimeValue::Val(first.as_macro_value().cdr())
                } else {
                    RuntimeValue::nil()
                }
            }
            "list" => {
                let vals: Vec<MacroValue> = args.iter().map(|a| a.as_macro_value().clone()).collect();
                RuntimeValue::Val(MacroValue::list(vals))
            }
            "eq" | "eql" => {
                if args.len() >= 2 {
                    let a = args[0].as_macro_value();
                    let b = args[1].as_macro_value();
                    RuntimeValue::Val(MacroValue::from_bool(a == b))
                } else {
                    RuntimeValue::Val(MacroValue::from_bool(false))
                }
            }
            "equal" => {
                if args.len() >= 2 {
                    let a = args[0].as_macro_value();
                    let b = args[1].as_macro_value();
                    RuntimeValue::Val(MacroValue::from_bool(a == b))
                } else {
                    RuntimeValue::Val(MacroValue::from_bool(false))
                }
            }
            "null" | "not" => {
                if let Some(first) = args.first() {
                    RuntimeValue::Val(MacroValue::from_bool(first.is_nil()))
                } else {
                    RuntimeValue::Val(MacroValue::Nil)
                }
            }
            "consp" => {
                if let Some(first) = args.first() {
                    RuntimeValue::Val(MacroValue::from_bool(matches!(first, RuntimeValue::Val(MacroValue::Cons(_)))))
                } else {
                    RuntimeValue::Val(MacroValue::Nil)
                }
            }
            "listp" => {
                if let Some(first) = args.first() {
                    let is_list = first.is_nil() || matches!(first, RuntimeValue::Val(MacroValue::Cons(_)));
                    RuntimeValue::Val(MacroValue::from_bool(is_list))
                } else {
                    RuntimeValue::Val(MacroValue::Nil)
                }
            }
            "symbolp" => {
                if let Some(first) = args.first() {
                    RuntimeValue::Val(MacroValue::from_bool(matches!(first, RuntimeValue::Val(MacroValue::Symbol(_)))))
                } else {
                    RuntimeValue::Val(MacroValue::Nil)
                }
            }
            "stringp" => {
                if let Some(first) = args.first() {
                    RuntimeValue::Val(MacroValue::from_bool(matches!(first, RuntimeValue::Val(MacroValue::String(_)))))
                } else {
                    RuntimeValue::Val(MacroValue::Nil)
                }
            }
            "numberp" => {
                if let Some(first) = args.first() {
                    RuntimeValue::Val(MacroValue::from_bool(matches!(first, RuntimeValue::Val(MacroValue::Int(_)))))
                } else {
                    RuntimeValue::Val(MacroValue::Nil)
                }
            }
            "length" => {
                if let Some(first) = args.first() {
                    let len = match first.as_macro_value() {
                        MacroValue::Nil => 0i64,
                        MacroValue::String(s) => s.len() as i64,
                        MacroValue::Cons(_) => {
                            first.as_macro_value().to_vec().map(|v| v.len() as i64).unwrap_or(0)
                        }
                        MacroValue::Vector(v) => v.len() as i64,
                        _ => 0,
                    };
                    RuntimeValue::Val(MacroValue::Int(len))
                } else {
                    RuntimeValue::Val(MacroValue::Int(0))
                }
            }
            "nth" => {
                if args.len() >= 2 {
                    let n = args[0].as_i64().unwrap_or(0);
                    let list = args[1].as_macro_value();
                    let result = nth_value(list, n);
                    RuntimeValue::Val(result)
                } else {
                    RuntimeValue::nil()
                }
            }
            "message" => {
                // (message fmt &rest args) — return nil
                RuntimeValue::Val(MacroValue::Nil)
            }
            "error" | "signal" => {
                self.error(format!("runtime error: {}", args.first().map(|a| a.display_string()).unwrap_or_default()));
                return PrimResult::Error;
            }
            // Built-in functions that return nil at runtime
            // (ones not already handled above)
            _ if is_known_nil_returning_builtin(name) => {
                RuntimeValue::Val(MacroValue::Nil)
            }
            _ => return PrimResult::Unknown,
        };
        PrimResult::Value(value)
    }

    fn execute_i64_primitive(&mut self, name: &str, args: &[i64]) -> Option<Result<i64, ()>> {
        let value = match name {
            "+" => checked_fold(0, args, i64::checked_add),
            "*" => checked_fold(1, args, i64::checked_mul),
            "-" => match args {
                [] => {
                    self.error("primitive `-` requires at least one argument");
                    return Some(Err(()));
                }
                [value] => value.checked_neg(),
                [first, rest @ ..] => checked_fold(*first, rest, i64::checked_sub),
            },
            "1+" => args.first()?.checked_add(1),
            "1-" => args.first()?.checked_sub(1),
            "=" => Some(bool_value(args.windows(2).all(|pair| pair[0] == pair[1]))),
            "<" => Some(bool_value(args.windows(2).all(|pair| pair[0] < pair[1]))),
            "<=" => Some(bool_value(args.windows(2).all(|pair| pair[0] <= pair[1]))),
            ">" => Some(bool_value(args.windows(2).all(|pair| pair[0] > pair[1]))),
            ">=" => Some(bool_value(args.windows(2).all(|pair| pair[0] >= pair[1]))),
            _ => return None,
        };
        Some(match value {
            Some(v) => Ok(v),
            None => {
                self.error(format!("integer overflow in primitive `{name}`"));
                Err(())
            }
        })
    }

    fn execute_module_call(&mut self, name: &str, args: &[RuntimeValue]) -> Option<RuntimeValue> {
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
        result.value.or_else(|| Some(RuntimeValue::nil()))
    }

    fn get_many(&mut self, regs: &[crate::ids::RegId]) -> Option<Vec<RuntimeValue>> {
        regs.iter().map(|reg| self.get(*reg)).collect()
    }

    fn get(&mut self, reg: crate::ids::RegId) -> Option<RuntimeValue> {
        let Some(value) = self.registers.get(reg.index()) else {
            self.error(format!("read from unknown register {reg:?}"));
            return None;
        };
        match value {
            Some(v) => Some(v.clone()),
            None => {
                self.error(format!("read from uninitialized register {reg:?}"));
                None
            }
        }
    }

    fn set(&mut self, reg: crate::ids::RegId, value: RuntimeValue) {
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

    fn finish(self, value: Option<RuntimeValue>) -> InterpResult {
        InterpResult {
            value,
            diagnostics: self.diagnostics,
        }
    }
}

fn nth_value(list: &MacroValue, n: i64) -> MacroValue {
    if n < 0 {
        return MacroValue::Nil;
    }
    let mut current = list.clone();
    for _ in 0..n {
        current = current.cdr();
    }
    current.car()
}

fn surface_to_runtime_value(form: &crate::surface::SurfaceForm) -> RuntimeValue {
    match &form.kind {
        SurfaceKind::Atom(atom) => RuntimeValue::Val(match atom {
            SurfaceAtom::Nil => MacroValue::Nil,
            SurfaceAtom::True => MacroValue::Symbol("t".into()),
            SurfaceAtom::Int(n) => MacroValue::Int(*n),
            SurfaceAtom::Float(f) => MacroValue::Int(*f as i64),
            SurfaceAtom::Char(c) => MacroValue::Int(*c),
            SurfaceAtom::String(s) => MacroValue::String(s.clone()),
            SurfaceAtom::Symbol(s) => MacroValue::Symbol(s.clone()),
        }),
        SurfaceKind::List(items) => {
            let vals: Vec<MacroValue> = items.iter().map(|f| {
                match &f.kind {
                    SurfaceKind::Atom(atom) => match atom {
                        SurfaceAtom::Nil => MacroValue::Nil,
                        SurfaceAtom::True => MacroValue::Symbol("t".into()),
                        SurfaceAtom::Int(n) => MacroValue::Int(*n),
                        SurfaceAtom::Float(f) => MacroValue::Int(*f as i64),
                        SurfaceAtom::Char(c) => MacroValue::Int(*c),
                        SurfaceAtom::String(s) => MacroValue::String(s.clone()),
                        SurfaceAtom::Symbol(s) => MacroValue::Symbol(s.clone()),
                    },
                    _ => MacroValue::Nil,
                }
            }).collect();
            RuntimeValue::Val(MacroValue::list(vals))
        }
        _ => RuntimeValue::Val(MacroValue::Nil),
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

fn is_known_nil_returning_builtin(name: &str) -> bool {
    matches!(name,
        | "boundp" | "fboundp" | "featurep" | "facep" | "display-graphic-p"
        | "display-multi-frame-p" | "display-color-p" | "display-mouse-p"
        | "window-system" | "console-type" | "initial-window-system"
        | "daemonp" | "noninteractive" | "interactive-p" | "called-interactively-p"
        | "memq" | "assq" | "assoc" | "rassq" | "member" | "delq" | "remove"
        | "get" | "put" | "plist-get" | "plist-put" | "symbol-plist"
        | "symbol-function" | "symbol-value" | "intern" | "intern-soft"
        | "mapcar" | "mapc" | "mapcan" | "dolist" | "dotimes"
        | "string-match" | "replace-match" | "match-string" | "match-beginning" | "match-end"
        | "concat" | "substring" | "string=" | "string<" | "string>"
        | "format" | "propertize" | "purecopy"
        | "point" | "point-min" | "point-max" | "buffer-size" | "buffer-name"
        | "buffer-file-name" | "current-buffer" | "window-buffer" | "selected-window"
        | "frame-parameter" | "frame-width" | "frame-height"
        | "line-beginning-position" | "line-end-position" | "pos-bol" | "pos-eol"
        | "file-exists-p" | "file-directory-p" | "file-readable-p" | "file-writable-p"
        | "expand-file-name" | "directory-file-name" | "file-name-directory"
        | "file-name-nondirectory" | "file-name-extension"
        | "require" | "provide" | "autoload" | "load" | "load-file"
        | "eval" | "funcall" | "apply" | "funcall-interactively"
        | "macroexpand" | "macroexpand-all"
        | "condition-case" | "condition-case-unless-debug" | "ignore-errors"
        | "catch" | "throw" | "unwind-protect"
        | "nreverse" | "reverse" | "sort" | "copy-sequence" | "copy-alist"
        | "append" | "butlast" | "last" | "nthcdr"
        | "set" | "default-value" | "set-default"
        | "make-sparse-keymap" | "make-keymap" | "define-key"
        | "use-global-map" | "use-local-map" | "current-global-map" | "current-local-map"
        | "lookup-key" | "key-binding" | "global-key-binding" | "local-key-binding"
        | "where-is-internal" | "command-remapping" | "event-basic-type"
        | "make-variable-buffer-local" | "make-local-variable" | "buffer-local-value"
        | "buffer-local-variables" | "local-variable-p" | "local-variable-if-set-p"
        | "kill-local-variable" | "kill-all-local-variables"
        | "default-boundp" | "setq-default"
        | "run-hooks" | "run-hook-with-args" | "add-hook" | "remove-hook"
        | "advice-add" | "advice-remove" | "add-function" | "remove-function"
        | "format-mode-line" | "format-time-string" | "current-time"
        | "read-from-minibuffer" | "read-string" | "read-file-name" | "read-buffer"
        | "completing-read" | "completing-read-default"
        | "insert" | "insert-char" | "insert-before-markers" | "delete-region"
        | "buffer-substring" | "buffer-substring-no-properties"
        | "goto-char" | "forward-char" | "backward-char"
        | "forward-line" | "beginning-of-line" | "end-of-line"
        | "re-search-forward" | "re-search-backward" | "search-forward" | "search-backward"
        | "looking-at" | "looking-at-p" | "string-match-p"
        | "replace-regexp-in-string" | "match-string-no-properties"
        | "upcase" | "downcase" | "capitalize" | "upcase-initials"
        | "char-to-string" | "string-to-char" | "number-to-string" | "string-to-number"
        | "identity" | "ignore" | "always" | "never"
        | "eql" | "equal" | "equal-including-properties"
        | "sxhash" | "sxhash-eq" | "sxhash-eql" | "sxhash-equal"
        | "message"
    )
}

fn bool_value(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn _const_value(value: &SsaConst) -> Option<i64> {
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
        assert_eq!(result.as_i64(), Some(42));
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
        assert_eq!(nil_result.as_i64(), Some(7));
        let true_result = execute_with_args(&regir.value, &[3, 7]);
        assert_eq!(true_result.diagnostics, Vec::new());
        assert_eq!(true_result.as_i64(), Some(3));
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
        assert_eq!(result.as_i64(), Some(9));
    }

    #[test]
    fn executes_integer_primitives() {
        let artifact = crate::execute_source(
            "arith.el",
            ";;; -*- lexical-binding: t; -*-\n(if (<= (1- 3) 2) (+ 10 (* 2 3)) 0)",
            &[],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(16));
    }

    #[test]
    fn executes_integer_primitive_entry_with_arguments() {
        let artifact = crate::execute_source(
            "entry.el",
            ";;; -*- lexical-binding: t; -*-\n(defun dec-if-positive (x) (if (> x 0) (1- x) 0))",
            &[8],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(7));
    }

    #[test]
    fn executes_named_module_function_call() {
        let artifact = crate::execute_source(
            "module-call.el",
            ";;; -*- lexical-binding: t; -*-\n(defun main (x) (add1 x))\n(defun add1 (n) (1+ n))",
            &[4],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(5));
    }

    #[test]
    fn executes_recursive_module_function_call() {
        let artifact = crate::execute_source(
            "fact.el",
            ";;; -*- lexical-binding: t; -*-\n(defun fact (n) (if (<= n 1) 1 (* n (fact (1- n)))))",
            &[5],
        );
        assert_eq!(artifact.result.diagnostics, Vec::new());
        assert_eq!(artifact.result.as_i64(), Some(120));
    }
}
