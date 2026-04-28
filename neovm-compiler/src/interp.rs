use std::collections::HashMap;
use std::cell::RefCell;

use cranelift_entity::EntityRef;

use crate::diagnostic::Diagnostic;
use crate::expand_value::MacroValue;
use crate::hir::{HirConst, HirExpr, HirExprKind, LambdaList};
use crate::ids::FunctionId;
use crate::regir::{RegFunction, RegInstKind, RegModule, RegTerminator};
use crate::ssa::{SsaConst, SsaLambdaTemplate};
use crate::surface::{SurfaceAtom, SurfaceKind};

thread_local! {
    static DYNAMIC_VARS: RefCell<HashMap<String, RuntimeValue>> = RefCell::new(HashMap::new());
}

fn dynamic_get(name: &str) -> RuntimeValue {
    DYNAMIC_VARS.with(|vars| {
        vars.borrow().get(name).cloned().unwrap_or_else(RuntimeValue::nil)
    })
}

fn dynamic_set(name: &str, value: RuntimeValue) -> RuntimeValue {
    DYNAMIC_VARS.with(|vars| {
        vars.borrow_mut().insert(name.to_string(), value.clone());
    });
    value
}

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
    Closure {
        template: SsaLambdaTemplate,
        captured: Vec<RuntimeValue>,
    },
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
            RuntimeValue::Closure { .. } => &MacroValue::Nil,
        }
    }

    fn is_closure(&self) -> bool {
        matches!(self, RuntimeValue::Closure { .. })
    }

    fn display_string(&self) -> String {
        match self {
            RuntimeValue::Val(v) => format_value(v),
            RuntimeValue::Closure { template, .. } => {
                let n_params = template.params.required.len();
                format!("<closure with {} params>", n_params)
            }
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
                self.set(*dst, dynamic_get(name));
            }
            RegInstKind::SymbolSet { dst, name, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                let result = dynamic_set(name, value);
                self.set(*dst, result);
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
                    RuntimeValue::Closure { template, captured } => {
                        match self.execute_closure(template, captured, &args) {
                            Some(v) => v,
                            None => return false,
                        }
                    }
                    _ => {
                        self.set(*dst, RuntimeValue::nil());
                        return true;
                    }
                };
                self.set(*dst, result);
            }
            RegInstKind::Lambda { dst, template, captures } => {
                let captured: Vec<RuntimeValue> = captures
                    .iter()
                    .filter_map(|reg| self.get(*reg))
                    .collect();
                self.set(*dst, RuntimeValue::Closure {
                    template: template.clone(),
                    captured,
                });
            }
            RegInstKind::MakeLexicalCell { dst, initial } => {
                let Some(value) = self.get(*initial) else {
                    return false;
                };
                let cell_val = match &value {
                    RuntimeValue::Val(v) => v.clone(),
                    RuntimeValue::Closure { .. } => MacroValue::Nil,
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
                dynamic_set(name, value);
            }
            RegInstKind::UnbindDynamic { count } => {
                // Dynamic scope unbinding not tracked — values persist
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
                let Some(callee_val) = self.get(*callee) else {
                    return false;
                };
                let Some(raw_args) = self.get_many(args) else {
                    return false;
                };
                // (apply func a1 a2 ... alist) — spread the last list arg
                let spread_args = spread_apply_args(&raw_args);
                let result = match &callee_val {
                    RuntimeValue::Val(MacroValue::Symbol(name)) => {
                        match self.execute_primitive_call(name, &spread_args) {
                            PrimResult::Value(v) => v,
                            PrimResult::Unknown => {
                                match self.execute_module_call(name, &spread_args) {
                                    Some(v) => v,
                                    None => RuntimeValue::nil(),
                                }
                            }
                            PrimResult::Error => return false,
                        }
                    }
                    RuntimeValue::Closure { template, captured } => {
                        match self.execute_closure(template, captured, &spread_args) {
                            Some(v) => v,
                            None => return false,
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
        match eval_primitive(name, args) {
            PrimResult::Unknown => PrimResult::Unknown,
            PrimResult::Error => PrimResult::Error,
            PrimResult::Value(v) => PrimResult::Value(v),
        }
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

    fn execute_closure(
        &mut self,
        template: &SsaLambdaTemplate,
        captured: &[RuntimeValue],
        args: &[RuntimeValue],
    ) -> Option<RuntimeValue> {
        let mut env: HashMap<String, RuntimeValue> = HashMap::new();

        // Bind captured values
        for (i, capture) in template.captures.iter().enumerate() {
            if let Some(val) = captured.get(i) {
                env.insert(capture.name.clone(), val.clone());
            }
        }

        // Bind parameters
        bind_lambda_params(&template.params, args, &mut env);

        // Interpret the HIR body
        eval_hir_expr(&template.body, &env, self.module, self.functions_by_name, self.fuel, &mut self.diagnostics)
    }
}

/// Spread apply args: last argument is a list that gets flattened.
/// (apply f 1 2 '(3 4)) → [1, 2, 3, 4]
fn spread_apply_args(args: &[RuntimeValue]) -> Vec<RuntimeValue> {
    if args.is_empty() {
        return Vec::new();
    }
    let mut result: Vec<RuntimeValue> = args[..args.len() - 1].to_vec();
    if let Some(last) = args.last() {
        match last.as_macro_value() {
            MacroValue::Nil => {}
            MacroValue::Cons(_) => {
                if let Some(vec) = last.as_macro_value().to_vec() {
                    for v in vec {
                        result.push(RuntimeValue::Val(v));
                    }
                }
            }
            _ => result.push(last.clone()),
        }
    }
    result
}

fn bind_lambda_params(params: &LambdaList, args: &[RuntimeValue], env: &mut HashMap<String, RuntimeValue>) {
    let mut arg_idx = 0;
    for name in &params.required {
        if let Some(val) = args.get(arg_idx) {
            env.insert(name.clone(), val.clone());
        } else {
            env.insert(name.clone(), RuntimeValue::nil());
        }
        arg_idx += 1;
    }
    for name in &params.optional {
        if let Some(val) = args.get(arg_idx) {
            env.insert(name.clone(), val.clone());
        } else {
            env.insert(name.clone(), RuntimeValue::nil());
        }
        arg_idx += 1;
    }
    if let Some(ref rest_name) = params.rest {
        let rest: Vec<MacroValue> = args[arg_idx..].iter().map(|a| a.as_macro_value().clone()).collect();
        env.insert(rest_name.clone(), RuntimeValue::Val(MacroValue::list(rest)));
    }
}

/// Interpret an HIR expression tree directly.
fn eval_hir_expr(
    expr: &HirExpr,
    env: &HashMap<String, RuntimeValue>,
    module: Option<&RegModule>,
    functions_by_name: Option<&HashMap<String, FunctionId>>,
    fuel: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<RuntimeValue> {
    match &expr.kind {
        HirExprKind::Const(c) => Some(match c {
            HirConst::Nil => RuntimeValue::nil(),
            HirConst::True => RuntimeValue::Val(MacroValue::Symbol("t".into())),
            HirConst::Int(n) => RuntimeValue::Val(MacroValue::Int(*n)),
            HirConst::Float(f) => RuntimeValue::Val(MacroValue::Int(*f as i64)),
            HirConst::String(s) => RuntimeValue::Val(MacroValue::String(s.clone())),
            HirConst::Char(c) => RuntimeValue::Val(MacroValue::Int(*c)),
        }),
        HirExprKind::Quote(form) => Some(surface_to_runtime_value(form)),
        HirExprKind::FunctionQuote(form) => Some(surface_to_runtime_value(form)),
        HirExprKind::LexicalGet(name) => Some(env.get(name).cloned().unwrap_or_else(RuntimeValue::nil)),
        HirExprKind::LexicalSet { name, value } => {
            let val = eval_hir_expr(value, env, module, functions_by_name, fuel, diagnostics)?;
            // Return the value (lexicals are immutable in HIR, but set returns the value)
            Some(val)
        }
        HirExprKind::SymbolGet(name) => Some(dynamic_get(name)),
        HirExprKind::SymbolSet { name, value } => {
            let val = eval_hir_expr(value, env, module, functions_by_name, fuel, diagnostics)?;
            Some(dynamic_set(name, val))
        }
        HirExprKind::If { test, then_expr, else_expr } => {
            let test_val = eval_hir_expr(test, env, module, functions_by_name, fuel, diagnostics)?;
            if test_val.is_nil() || matches!(&test_val, RuntimeValue::Val(MacroValue::Int(0))) {
                eval_hir_expr(else_expr, env, module, functions_by_name, fuel, diagnostics)
            } else {
                eval_hir_expr(then_expr, env, module, functions_by_name, fuel, diagnostics)
            }
        }
        HirExprKind::While { test, body } => {
            let mut last = RuntimeValue::nil();
            loop {
                if *fuel == 0 {
                    diagnostics.push(Diagnostic::error("closure interpreter exhausted fuel in while loop"));
                    return None;
                }
                *fuel -= 1;
                let test_val = eval_hir_expr(test, env, module, functions_by_name, fuel, diagnostics)?;
                if test_val.is_nil() || matches!(&test_val, RuntimeValue::Val(MacroValue::Int(0))) {
                    break;
                }
                last = eval_hir_expr(body, env, module, functions_by_name, fuel, diagnostics)?;
            }
            Some(last)
        }
        HirExprKind::Progn(exprs) => {
            let mut last = RuntimeValue::nil();
            for e in exprs {
                last = eval_hir_expr(e, env, module, functions_by_name, fuel, diagnostics)?;
            }
            Some(last)
        }
        HirExprKind::Let { mode, sequential, bindings, body, .. } => {
            let mut inner_env = env.clone();
            for binding in bindings {
                let val = eval_hir_expr(&binding.init, &inner_env, module, functions_by_name, fuel, diagnostics)?;
                inner_env.insert(binding.name.clone(), val);
            }
            eval_hir_expr(body, &inner_env, module, functions_by_name, fuel, diagnostics)
        }
        HirExprKind::Lambda { params, body, .. } => {
            // Create a closure template with no captures (captures from current env
            // would need free variable analysis — for now, captures are empty)
            let template = SsaLambdaTemplate {
                params: params.clone(),
                captures: Vec::new(),
                declarations: Vec::new(),
                body: body.clone(),
            };
            Some(RuntimeValue::Closure {
                template,
                captured: Vec::new(),
            })
        }
        HirExprKind::Declare(_) => Some(RuntimeValue::nil()),
        HirExprKind::Catch { .. } => {
            // Catch/throw not supported in closure interpreter
            Some(RuntimeValue::nil())
        }
        HirExprKind::Throw { .. } => {
            diagnostics.push(Diagnostic::error("throw not supported in closure interpreter"));
            None
        }
        HirExprKind::ConditionCase { body, .. } => {
            // Just evaluate the body, ignore handlers
            eval_hir_expr(body, env, module, functions_by_name, fuel, diagnostics)
        }
        HirExprKind::UnwindProtect { body, .. } => {
            // Just evaluate the body, ignore cleanup
            eval_hir_expr(body, env, module, functions_by_name, fuel, diagnostics)
        }
        HirExprKind::Funcall { callee, args } => {
            let callee_val = eval_hir_expr(callee, env, module, functions_by_name, fuel, diagnostics)?;
            let arg_vals: Vec<RuntimeValue> = args.iter().filter_map(|a| {
                eval_hir_expr(a, env, module, functions_by_name, fuel, diagnostics)
            }).collect();
            eval_call(callee_val, &arg_vals, module, functions_by_name, fuel, diagnostics, env)
        }
        HirExprKind::Apply { callee, args } => {
            let callee_val = eval_hir_expr(callee, env, module, functions_by_name, fuel, diagnostics)?;
            let raw_arg_vals: Vec<RuntimeValue> = args.iter().filter_map(|a| {
                eval_hir_expr(a, env, module, functions_by_name, fuel, diagnostics)
            }).collect();
            let spread_args = spread_apply_args(&raw_arg_vals);
            eval_call(callee_val, &spread_args, module, functions_by_name, fuel, diagnostics, env)
        }
        HirExprKind::CallNamed { name, args } => {
            let arg_vals: Vec<RuntimeValue> = args.iter().filter_map(|a| {
                eval_hir_expr(a, env, module, functions_by_name, fuel, diagnostics)
            }).collect();
            let callee = RuntimeValue::Val(MacroValue::Symbol(name.clone()));
            eval_call(callee, &arg_vals, module, functions_by_name, fuel, diagnostics, env)
        }
        HirExprKind::CallValue { callee, args } => {
            let callee_val = eval_hir_expr(callee, env, module, functions_by_name, fuel, diagnostics)?;
            let arg_vals: Vec<RuntimeValue> = args.iter().filter_map(|a| {
                eval_hir_expr(a, env, module, functions_by_name, fuel, diagnostics)
            }).collect();
            eval_call(callee_val, &arg_vals, module, functions_by_name, fuel, diagnostics, env)
        }
    }
}

/// Execute a function call from the HIR interpreter.
fn eval_call(
    callee: RuntimeValue,
    args: &[RuntimeValue],
    module: Option<&RegModule>,
    functions_by_name: Option<&HashMap<String, FunctionId>>,
    fuel: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
    _env: &HashMap<String, RuntimeValue>,
) -> Option<RuntimeValue> {
    match &callee {
        RuntimeValue::Val(MacroValue::Symbol(name)) => {
            // Try the standalone primitive evaluator first
            match eval_primitive(name, args) {
                PrimResult::Value(v) => Some(v),
                PrimResult::Error => {
                    diagnostics.push(Diagnostic::error(format!("runtime error in closure call to `{name}`")));
                    None
                }
                PrimResult::Unknown => {
                    // Try module call
                    if let (Some(mod_), Some(fns)) = (module, functions_by_name) {
                        if let Some(fid) = fns.get(name).copied() {
                            if let Some(func) = mod_.functions.get(fid) {
                                let result = execute_with_module(
                                    func, args, Some(mod_), Some(fns), fuel
                                );
                                diagnostics.extend(result.diagnostics);
                                return result.value.or_else(|| Some(RuntimeValue::nil()));
                            }
                        }
                    }
                    Some(RuntimeValue::nil())
                }
            }
        }
        RuntimeValue::Closure { template, captured } => {
            let mut closure_env: HashMap<String, RuntimeValue> = HashMap::new();
            for (i, capture) in template.captures.iter().enumerate() {
                if let Some(val) = captured.get(i) {
                    closure_env.insert(capture.name.clone(), val.clone());
                }
            }
            bind_lambda_params(&template.params, args, &mut closure_env);
            eval_hir_expr(&template.body, &closure_env, module, functions_by_name, fuel, diagnostics)
        }
        _ => Some(RuntimeValue::nil()),
    }
}

/// Standalone primitive evaluator — no interpreter context needed.
/// Used by both the RegIR interpreter and the HIR closure interpreter.
fn eval_primitive(name: &str, args: &[RuntimeValue]) -> PrimResult {
    // Try i64 fast path first
    let i64_args: Option<Vec<i64>> = args.iter().map(|a| a.as_i64()).collect();
    if let Some(ref iargs) = i64_args {
        if let Some(result) = eval_i64_primitive(name, iargs) {
            return match result {
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
            RuntimeValue::Val(args.first().map(|a| a.as_macro_value().car()).unwrap_or(MacroValue::Nil))
        }
        "cdr" | "cdr-safe" => {
            RuntimeValue::Val(args.first().map(|a| a.as_macro_value().cdr()).unwrap_or(MacroValue::Nil))
        }
        "list" => {
            let vals: Vec<MacroValue> = args.iter().map(|a| a.as_macro_value().clone()).collect();
            RuntimeValue::Val(MacroValue::list(vals))
        }
        "eq" | "eql" => {
            RuntimeValue::Val(MacroValue::from_bool(args.len() >= 2 && args[0].as_macro_value() == args[1].as_macro_value()))
        }
        "equal" => {
            RuntimeValue::Val(MacroValue::from_bool(args.len() >= 2 && args[0].as_macro_value() == args[1].as_macro_value()))
        }
        "null" | "not" => {
            RuntimeValue::Val(MacroValue::from_bool(args.first().map(|a| a.is_nil()).unwrap_or(true)))
        }
        "consp" => {
            RuntimeValue::Val(MacroValue::from_bool(
                args.first().map(|a| matches!(a, RuntimeValue::Val(MacroValue::Cons(_)))).unwrap_or(false)
            ))
        }
        "listp" => {
            RuntimeValue::Val(MacroValue::from_bool(
                args.first().map(|a| a.is_nil() || matches!(a, RuntimeValue::Val(MacroValue::Cons(_)))).unwrap_or(true)
            ))
        }
        "symbolp" => {
            RuntimeValue::Val(MacroValue::from_bool(
                args.first().map(|a| matches!(a, RuntimeValue::Val(MacroValue::Symbol(_)))).unwrap_or(false)
            ))
        }
        "stringp" => {
            RuntimeValue::Val(MacroValue::from_bool(
                args.first().map(|a| matches!(a, RuntimeValue::Val(MacroValue::String(_)))).unwrap_or(false)
            ))
        }
        "numberp" | "integerp" => {
            RuntimeValue::Val(MacroValue::from_bool(
                args.first().map(|a| matches!(a, RuntimeValue::Val(MacroValue::Int(_)))).unwrap_or(false)
            ))
        }
        "atom" => {
            RuntimeValue::Val(MacroValue::from_bool(
                args.first().map(|a| !matches!(a, RuntimeValue::Val(MacroValue::Cons(_)))).unwrap_or(true)
            ))
        }
        "zerop" => {
            RuntimeValue::Val(MacroValue::from_bool(args.first().map(|a| a.as_i64() == Some(0)).unwrap_or(false)))
        }
        "length" => {
            let len = match args.first().map(|a| a.as_macro_value()) {
                Some(MacroValue::Nil) => 0i64,
                Some(MacroValue::String(s)) => s.len() as i64,
                Some(MacroValue::Cons(_)) => args[0].as_macro_value().to_vec().map(|v| v.len() as i64).unwrap_or(0),
                Some(MacroValue::Vector(v)) => v.len() as i64,
                _ => 0,
            };
            RuntimeValue::Val(MacroValue::Int(len))
        }
        "nth" => {
            if args.len() >= 2 {
                RuntimeValue::Val(nth_value(args[1].as_macro_value(), args[0].as_i64().unwrap_or(0)))
            } else {
                RuntimeValue::nil()
            }
        }
        "concat" => {
            let parts: Vec<&str> = args.iter().filter_map(|a| a.as_macro_value().as_string()).collect();
            RuntimeValue::Val(MacroValue::String(parts.join("")))
        }
        "substring" => {
            if args.len() >= 2 {
                if let (Some(s), Some(start)) = (args[0].as_macro_value().as_string(), args[1].as_i64()) {
                    let start = start.max(0) as usize;
                    let end = args.get(2).and_then(|a| a.as_i64()).map(|e| e.max(0) as usize).unwrap_or(s.len());
                    RuntimeValue::Val(MacroValue::String(s[start.min(end)..end.min(s.len())].to_string()))
                } else { RuntimeValue::nil() }
            } else { RuntimeValue::nil() }
        }
        "string=" | "string-equal" => {
            RuntimeValue::Val(MacroValue::from_bool(
                args.len() >= 2 && args[0].as_macro_value().as_string() == args[1].as_macro_value().as_string()
            ))
        }
        "string<" | "string-lessp" => {
            let a = args.first().and_then(|v| v.as_macro_value().as_string()).unwrap_or("");
            let b = args.get(1).and_then(|v| v.as_macro_value().as_string()).unwrap_or("");
            RuntimeValue::Val(MacroValue::from_bool(a < b))
        }
        "format" => {
            if let Some(fmt) = args.first().and_then(|a| a.as_macro_value().as_string()) {
                let mut formatted = fmt.to_string();
                for arg in &args[1..] {
                    if let Some(pos) = formatted.find("%s").or(formatted.find("%d")) {
                        let repl = arg.as_i64().map(|n| n.to_string())
                            .or_else(|| arg.as_macro_value().as_string().map(String::from))
                            .unwrap_or_else(|| arg.display_string());
                        let before = formatted[..pos].to_string();
                        let after = formatted[pos+2..].to_string();
                        formatted = format!("{}{}{}", before, repl, after);
                    }
                }
                RuntimeValue::Val(MacroValue::String(formatted))
            } else { RuntimeValue::nil() }
        }
        "message" => RuntimeValue::nil(),
        "append" => {
            let mut result = Vec::new();
            for (i, arg) in args.iter().enumerate() {
                if i + 1 == args.len() {
                    if let Some(vec) = arg.as_macro_value().to_vec() { result.extend(vec); }
                } else if let Some(vec) = arg.as_macro_value().to_vec() { result.extend(vec); }
            }
            RuntimeValue::Val(MacroValue::list(result))
        }
        "nreverse" | "reverse" => {
            if let Some(first) = args.first() {
                if let Some(mut vec) = first.as_macro_value().to_vec() {
                    vec.reverse();
                    return PrimResult::Value(RuntimeValue::Val(MacroValue::list(vec)));
                }
            }
            RuntimeValue::nil()
        }
        "nthcdr" => {
            if args.len() >= 2 {
                let mut n = args[0].as_i64().unwrap_or(0);
                let mut list = args[1].as_macro_value().clone();
                while n > 0 { list = list.cdr(); n -= 1; }
                RuntimeValue::Val(list)
            } else { RuntimeValue::nil() }
        }
        "last" => {
            if let Some(first) = args.first() {
                let n = args.get(1).and_then(|a| a.as_i64()).unwrap_or(1) as usize;
                RuntimeValue::Val(first.as_macro_value().last(n))
            } else { RuntimeValue::nil() }
        }
        "butlast" => {
            if let Some(first) = args.first() {
                let n = args.get(1).and_then(|a| a.as_i64()).unwrap_or(1) as usize;
                RuntimeValue::Val(first.as_macro_value().butlast(n))
            } else { RuntimeValue::nil() }
        }
        "max" => {
            let vals: Vec<i64> = args.iter().filter_map(|a| a.as_i64()).collect();
            RuntimeValue::Val(MacroValue::Int(vals.into_iter().max().unwrap_or(0)))
        }
        "min" => {
            let vals: Vec<i64> = args.iter().filter_map(|a| a.as_i64()).collect();
            RuntimeValue::Val(MacroValue::Int(vals.into_iter().min().unwrap_or(0)))
        }
        "abs" => {
            RuntimeValue::Val(MacroValue::Int(args.first().and_then(|a| a.as_i64()).unwrap_or(0).abs()))
        }
        "mod" => {
            if args.len() >= 2 {
                let a = args[0].as_i64().unwrap_or(0);
                let b = args[1].as_i64().unwrap_or(1);
                if b != 0 {
                    RuntimeValue::Val(MacroValue::Int(a % b))
                } else { RuntimeValue::nil() }
            } else { RuntimeValue::nil() }
        }
        "number-to-string" | "int-to-string" => {
            RuntimeValue::Val(MacroValue::String(args.first().and_then(|a| a.as_i64()).unwrap_or(0).to_string()))
        }
        "string-to-number" => {
            let s = args.first().and_then(|a| a.as_macro_value().as_string()).unwrap_or("0");
            RuntimeValue::Val(MacroValue::Int(s.parse::<i64>().unwrap_or(0)))
        }
        _ if is_known_nil_returning_builtin(name) => RuntimeValue::nil(),
        _ => return PrimResult::Unknown,
    };
    PrimResult::Value(value)
}

fn eval_i64_primitive(name: &str, args: &[i64]) -> Option<Result<i64, ()>> {
    let value = match name {
        "+" => checked_fold(0, args, i64::checked_add),
        "*" => checked_fold(1, args, i64::checked_mul),
        "-" => match args {
            [] => return None,
            [v] => v.checked_neg(),
            [first, rest @ ..] => checked_fold(*first, rest, i64::checked_sub),
        },
        "/" => {
            if args.is_empty() { return None; }
            let first = *args.first()?;
            if args.len() == 1 { return Some(Ok(first)); }
            let rest = &args[1..];
            rest.iter().try_fold(first, |acc, &v| {
                if v == 0 { None } else { Some(acc / v) }
            })
        }
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
        None => Err(()),
    })
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
        | "propertize" | "purecopy"
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
        | "sort" | "copy-sequence" | "copy-alist"
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
        | "char-to-string" | "string-to-char"
        | "identity" | "ignore" | "always" | "never"
        | "equal-including-properties"
        | "sxhash" | "sxhash-eq" | "sxhash-eql" | "sxhash-equal"
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
