use std::collections::HashMap;

use neovm_compiler::diagnostic::Diagnostic;
use neovm_compiler::ids::{FunctionId, RegId};
use neovm_compiler::lower::{lambda_template_to_ssa, ssa_to_regir};
use neovm_compiler::regir::{RegFunction, RegInst, RegInstKind, RegModule, RegTerminator};
use neovm_compiler::ssa::SsaConst;
use neovm_compiler::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

use crate::{LispValue, Runtime, RuntimeError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectInterpResult {
    pub value: Option<LispValue>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ThrownValue {
    tag: LispValue,
    value: LispValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SignaledValue {
    symbol: LispValue,
    data: LispValue,
}

#[derive(Clone, Debug)]
struct ConditionFrame {
    var: Option<String>,
}

#[derive(Clone, Debug)]
struct ActiveConditionHandler {
    stop_index: usize,
    condition_end_index: usize,
    result_reg: Option<RegId>,
    dynamic_bind_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NonlocalExit {
    Throw(ThrownValue),
    Signal(SignaledValue),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveUnwindCleanup {
    stop_index: usize,
    pending: NonlocalExit,
}

#[derive(Clone, Debug)]
struct InternalInterpResult {
    value: Option<LispValue>,
    thrown: Option<ThrownValue>,
    signaled: Option<SignaledValue>,
    diagnostics: Vec<Diagnostic>,
}

impl InternalInterpResult {
    fn error(message: impl Into<String>) -> Self {
        Self {
            value: None,
            thrown: None,
            signaled: None,
            diagnostics: vec![Diagnostic::error(message)],
        }
    }

    fn into_public(mut self, runtime: &Runtime) -> ObjectInterpResult {
        if let Some(thrown) = self.thrown.take() {
            self.diagnostics.push(Diagnostic::error(format!(
                "uncaught throw for tag {}",
                runtime.format_value(thrown.tag)
            )));
        }
        if let Some(signaled) = self.signaled.take() {
            self.diagnostics.push(Diagnostic::error(format!(
                "uncaught signal {} with data {}",
                runtime.format_value(signaled.symbol),
                runtime.format_value(signaled.data)
            )));
        }
        ObjectInterpResult {
            value: self.value,
            diagnostics: self.diagnostics,
        }
    }
}

pub fn execute_module_with_args(
    module: &RegModule,
    args: &[LispValue],
    runtime: &mut Runtime,
) -> ObjectInterpResult {
    let functions_by_name = functions_by_name(module);
    let mut fuel = 100_000usize;
    execute_module_entry(module, &functions_by_name, args, runtime, &mut fuel).into_public(runtime)
}

fn execute_module_entry(
    module: &RegModule,
    functions_by_name: &HashMap<String, FunctionId>,
    args: &[LispValue],
    runtime: &mut Runtime,
    fuel: &mut usize,
) -> InternalInterpResult {
    let Some(entry) = module.entry else {
        return InternalInterpResult::error("object interpreter requires a module entry function");
    };
    let Some(function) = module.functions.get(entry) else {
        return InternalInterpResult::error(format!(
            "object interpreter references unknown module entry function {entry:?}"
        ));
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
) -> InternalInterpResult {
    let interpreter = Interpreter {
        function,
        registers: HashMap::new(),
        lexicals: HashMap::new(),
        catch_stack: Vec::new(),
        condition_stack: Vec::new(),
        active_condition_handler: None,
        active_unwind_cleanup: None,
        pending_throw: None,
        pending_signal: None,
        last_value: None,
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
    catch_stack: Vec<LispValue>,
    condition_stack: Vec<ConditionFrame>,
    active_condition_handler: Option<ActiveConditionHandler>,
    active_unwind_cleanup: Option<ActiveUnwindCleanup>,
    pending_throw: Option<ThrownValue>,
    pending_signal: Option<SignaledValue>,
    last_value: Option<LispValue>,
    module: &'a RegModule,
    functions_by_name: &'a HashMap<String, FunctionId>,
    runtime: &'runtime mut Runtime,
    fuel: &'fuel mut usize,
    diagnostics: Vec<Diagnostic>,
}

impl Interpreter<'_, '_, '_> {
    fn execute(mut self, args: &[LispValue]) -> InternalInterpResult {
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
            let mut inst_index = 0;
            while inst_index < body.instructions.len() {
                let inst = &body.instructions[inst_index];
                if self
                    .active_unwind_cleanup
                    .as_ref()
                    .is_some_and(|cleanup| cleanup.stop_index == inst_index)
                {
                    let cleanup = self
                        .active_unwind_cleanup
                        .take()
                        .expect("active unwind cleanup");
                    return self.finish_nonlocal(cleanup.pending);
                }
                if self
                    .active_condition_handler
                    .as_ref()
                    .is_some_and(|handler| handler.stop_index == inst_index)
                {
                    let active = self
                        .active_condition_handler
                        .take()
                        .expect("active condition handler");
                    let next_index = active.condition_end_index + 1;
                    let result_reg = active.result_reg;
                    let Some(value) = self.complete_condition_handler(active) else {
                        return self.finish(None);
                    };
                    if let Some(result_reg) = result_reg {
                        self.set(result_reg, value);
                        inst_index = next_index;
                        continue;
                    }
                    return self.finish(Some(value));
                }
                if matches!(inst.kind, RegInstKind::ConditionCaseHandler { .. }) {
                    let Some(end_index) = find_condition_case_end(&body.instructions, inst_index)
                    else {
                        self.error("object interpreter reached condition-case handler without end");
                        return self.finish(None);
                    };
                    self.condition_stack.pop();
                    inst_index = end_index + 1;
                    continue;
                }
                if !self.execute_inst(&inst.kind) {
                    if let Some(thrown) = self.pending_throw.take() {
                        let already_in_cleanup = self.active_unwind_cleanup.take().is_some();
                        if !already_in_cleanup {
                            if let Some(cleanup_start) = self.enter_unwind_cleanup(
                                &body.instructions,
                                inst_index,
                                NonlocalExit::Throw(thrown),
                            ) {
                                inst_index = cleanup_start;
                                continue;
                            }
                        }
                        return self.finish_throw(thrown);
                    }
                    if let Some(signaled) = self.pending_signal.take() {
                        let already_in_cleanup = self.active_unwind_cleanup.take().is_some();
                        if !already_in_cleanup {
                            if let Some(cleanup_start) = self.enter_unwind_cleanup(
                                &body.instructions,
                                inst_index,
                                NonlocalExit::Signal(signaled),
                            ) {
                                inst_index = cleanup_start;
                                continue;
                            }
                        }
                        if let Some(handler_start) = self.enter_condition_handler(
                            &body.instructions,
                            inst_index,
                            signaled,
                            instruction_result_reg(&inst.kind),
                        ) {
                            inst_index = handler_start;
                            continue;
                        }
                        return self.finish_signal(signaled);
                    }
                    return self.finish(None);
                }
                inst_index += 1;
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
                let Some(value) = self.const_value(value) else {
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::Quote { dst, form } => {
                let Some(value) = self.quote_value(form) else {
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::FunctionQuote { dst, form } => {
                let Some(value) = self.function_quote_value(form) else {
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
            RegInstKind::SymbolGet { dst, name } => {
                let result = self.runtime.symbol_value_by_name(name);
                let Some(value) = self.runtime_value(result) else {
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::SymbolSet { dst, name, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                let result = self.runtime.set_symbol_value_by_name(name, value);
                let Some(value) = self.runtime_value(result) else {
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::Funcall { dst, callee, args } => {
                let Some(callee) = self.get(*callee) else {
                    return false;
                };
                let Some(args) = self.get_many(args) else {
                    return false;
                };
                let Some(value) = self.execute_funcall(callee, &args) else {
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::Apply { dst, callee, args } => {
                let Some(callee) = self.get(*callee) else {
                    return false;
                };
                let Some(args) = self.get_many(args) else {
                    return false;
                };
                let Some(value) = self.execute_apply(callee, &args) else {
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::Lambda {
                dst,
                template,
                captures,
            } => {
                let Some(captures) = self.get_many(captures) else {
                    return false;
                };
                let function = self.runtime.function(template.clone(), captures);
                self.set(*dst, function);
            }
            RegInstKind::MakeLexicalCell { dst, initial } => {
                let Some(value) = self.get(*initial) else {
                    return false;
                };
                let cell = self.runtime.lexical_cell(value);
                self.set(*dst, cell);
            }
            RegInstKind::LexicalCellGet { dst, cell } => {
                let Some(cell) = self.get(*cell) else {
                    return false;
                };
                let result = self.runtime.lexical_cell_get(cell);
                let Some(value) = self.runtime_value(result) else {
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::LexicalCellSet { dst, cell, src } => {
                let Some(cell) = self.get(*cell) else {
                    return false;
                };
                let Some(value) = self.get(*src) else {
                    return false;
                };
                let result = self.runtime.lexical_cell_set(cell, value);
                let Some(value) = self.runtime_value(result) else {
                    return false;
                };
                self.set(*dst, value);
            }
            RegInstKind::BindDynamic { name, src } => {
                let Some(value) = self.get(*src) else {
                    return false;
                };
                if let Err(error) = self.runtime.bind_dynamic_by_name(name, value) {
                    self.runtime_error(error);
                    return false;
                }
            }
            RegInstKind::UnbindDynamic { count } => {
                if let Err(error) = self.runtime.unbind_dynamic(*count) {
                    self.runtime_error(error);
                    return false;
                }
            }
            RegInstKind::CatchBegin { tag } => {
                let Some(tag) = self.get(*tag) else {
                    return false;
                };
                self.catch_stack.push(tag);
            }
            RegInstKind::CatchEnd => {
                if self.catch_stack.pop().is_none() {
                    self.error("object interpreter reached catch end without catch begin");
                    return false;
                }
            }
            RegInstKind::Throw { tag, value } => {
                let Some(tag) = self.get(*tag) else {
                    return false;
                };
                let Some(value) = self.get(*value) else {
                    return false;
                };
                self.pending_throw = Some(ThrownValue { tag, value });
                return false;
            }
            RegInstKind::ConditionCaseBegin { var } => {
                self.condition_stack
                    .push(ConditionFrame { var: var.clone() });
            }
            RegInstKind::ConditionCaseHandler { .. } => {}
            RegInstKind::ConditionCaseEnd => {
                self.condition_stack.pop();
            }
            RegInstKind::UnwindProtectBegin
            | RegInstKind::UnwindProtectCleanup
            | RegInstKind::UnwindProtectEnd => {}
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

    fn execute_funcall(&mut self, callee: LispValue, args: &[LispValue]) -> Option<LispValue> {
        self.execute_funcall_with_depth(callee, args, 16)
    }

    fn execute_funcall_with_depth(
        &mut self,
        callee: LispValue,
        args: &[LispValue],
        depth: usize,
    ) -> Option<LispValue> {
        if depth == 0 {
            self.error("function indirection exceeded object interpreter recursion limit");
            return None;
        }
        if self.runtime.is_function(callee) {
            return self.execute_function_object(callee, args);
        }
        let name = match self.runtime.symbol_name(callee) {
            Ok(name) => name,
            Err(error) => {
                self.runtime_error(error);
                return None;
            }
        };
        match self.runtime.symbol_function(callee) {
            Ok(Some(function)) if function != callee => {
                return self.execute_funcall_with_depth(function, args, depth - 1);
            }
            Ok(_) => {}
            Err(error) => {
                self.runtime_error(error);
                return None;
            }
        }
        self.execute_named_call(&name, args)
    }

    fn execute_function_object(
        &mut self,
        function: LispValue,
        args: &[LispValue],
    ) -> Option<LispValue> {
        let (template, captures) = match self.runtime.function_parts(function) {
            Ok(function) => function,
            Err(error) => {
                self.runtime_error(error);
                return None;
            }
        };
        let lowered = lambda_template_to_ssa(&template);
        if !lowered.diagnostics.is_empty() {
            self.diagnostics.extend(lowered.diagnostics);
            return None;
        }
        let regir = ssa_to_regir(&lowered.value);
        if !regir.diagnostics.is_empty() {
            self.diagnostics.extend(regir.diagnostics);
            return None;
        }
        let mut entry_args = Vec::with_capacity(captures.len() + args.len());
        entry_args.extend(captures);
        entry_args.extend_from_slice(args);
        let result = execute_with_module(
            &regir.value,
            &entry_args,
            self.module,
            self.functions_by_name,
            self.runtime,
            &mut *self.fuel,
        );
        self.diagnostics.extend(result.diagnostics);
        if let Some(thrown) = result.thrown {
            self.pending_throw = Some(thrown);
            return None;
        }
        if let Some(signaled) = result.signaled {
            self.pending_signal = Some(signaled);
            return None;
        }
        result.value
    }

    fn execute_apply(&mut self, callee: LispValue, args: &[LispValue]) -> Option<LispValue> {
        let Some((last, prefixes)) = args.split_last() else {
            self.error("apply requires at least one argument list");
            return None;
        };
        let tail = self.list_values(*last)?;
        let mut flattened = Vec::with_capacity(prefixes.len() + tail.len());
        flattened.extend(prefixes.iter().copied());
        flattened.extend(tail);
        self.execute_funcall(callee, &flattened)
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
                .map(|_| bool_value(self.runtime.is_symbol(args[0]))),
            "stringp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_string(args[0]))),
            "symbol-value" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.symbol_value(args[0]);
                self.runtime_value(result)
            }),
            "set" => self.exact_arity(name, args, 2).and_then(|_| {
                let result = self.runtime.set_symbol_value(args[0], args[1]);
                self.runtime_value(result)
            }),
            "boundp" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.is_bound_symbol(args[0]);
                self.runtime_bool(result)
            }),
            "fboundp" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.fboundp(args[0])),
            "symbol-function" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.symbol_function(args[0])),
            "intern" => self.exact_arity(name, args, 1).and_then(|_| {
                let name = match self.runtime.string_contents(args[0]) {
                    Ok(name) => name.to_string(),
                    Err(error) => {
                        self.runtime_error(error);
                        return None;
                    }
                };
                Some(self.runtime.intern(&name))
            }),
            "symbol-name" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.symbol_name_value(args[0]);
                self.runtime_value(result)
            }),
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
            "signal" => self.exact_arity(name, args, 2).and_then(|_| {
                self.pending_signal = Some(SignaledValue {
                    symbol: args[0],
                    data: args[1],
                });
                None
            }),
            "error" => {
                let symbol = self.runtime.intern("error");
                let data = make_list(self.runtime, args.iter().copied());
                self.pending_signal = Some(SignaledValue { symbol, data });
                None
            }
            "funcall" => {
                let Some((callee, args)) = args.split_first() else {
                    self.error("funcall requires a function");
                    return Some(None);
                };
                self.execute_funcall(*callee, args)
            }
            "apply" => {
                let Some((callee, args)) = args.split_first() else {
                    self.error("apply requires a function and arguments");
                    return Some(None);
                };
                self.execute_apply(*callee, args)
            }
            _ => return None,
        };
        Some(value)
    }

    fn fboundp(&mut self, symbol: LispValue) -> Option<LispValue> {
        let name = match self.runtime.symbol_name(symbol) {
            Ok(name) => name,
            Err(error) => {
                self.runtime_error(error);
                return None;
            }
        };
        let function = match self.runtime.symbol_function(symbol) {
            Ok(function) => function,
            Err(error) => {
                self.runtime_error(error);
                return None;
            }
        };
        Some(bool_value(
            function.is_some() || self.is_callable_name(&name),
        ))
    }

    fn symbol_function(&mut self, symbol: LispValue) -> Option<LispValue> {
        let name = match self.runtime.symbol_name(symbol) {
            Ok(name) => name,
            Err(error) => {
                self.runtime_error(error);
                return None;
            }
        };
        match self.runtime.symbol_function(symbol) {
            Ok(Some(function)) => Some(function),
            Ok(None) if self.is_callable_name(&name) => Some(symbol),
            Ok(None) => {
                self.runtime_error(RuntimeError::VoidFunction { name });
                None
            }
            Err(error) => {
                self.runtime_error(error);
                None
            }
        }
    }

    fn is_callable_name(&self, name: &str) -> bool {
        is_primitive_name(name) || self.functions_by_name.contains_key(name)
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
        if let Some(thrown) = result.thrown {
            self.pending_throw = Some(thrown);
            return None;
        }
        if let Some(signaled) = result.signaled {
            self.pending_signal = Some(signaled);
            return None;
        }
        result.value
    }

    fn catch_throw(&mut self, thrown: ThrownValue) -> Result<LispValue, ThrownValue> {
        let Some(index) = self.catch_stack.iter().rposition(|tag| *tag == thrown.tag) else {
            return Err(thrown);
        };
        self.catch_stack.truncate(index);
        Ok(thrown.value)
    }

    fn enter_condition_handler(
        &mut self,
        instructions: &[RegInst],
        signal_index: usize,
        signaled: SignaledValue,
        result_reg: Option<RegId>,
    ) -> Option<usize> {
        let signal_name = match self.runtime.symbol_name(signaled.symbol) {
            Ok(name) => name,
            Err(error) => {
                self.runtime_error(error);
                return None;
            }
        };
        let target = find_condition_handler(instructions, signal_index, &signal_name)?;
        let frame = self.condition_stack.pop()?;
        let mut dynamic_bind_count = 0;
        if let Some(var) = frame.var {
            let binding = self.runtime.cons(signaled.symbol, signaled.data);
            if let Err(error) = self.runtime.bind_dynamic_by_name(&var, binding) {
                self.runtime_error(error);
                return None;
            }
            dynamic_bind_count = 1;
        }
        self.last_value = None;
        self.active_condition_handler = Some(ActiveConditionHandler {
            stop_index: target.stop_index,
            condition_end_index: target.condition_end_index,
            result_reg,
            dynamic_bind_count,
        });
        Some(target.handler_index + 1)
    }

    fn complete_condition_handler(&mut self, active: ActiveConditionHandler) -> Option<LispValue> {
        if active.dynamic_bind_count > 0
            && let Err(error) = self.runtime.unbind_dynamic(active.dynamic_bind_count)
        {
            self.runtime_error(error);
            return None;
        }
        Some(self.last_value.unwrap_or(LispValue::NIL))
    }

    fn enter_unwind_cleanup(
        &mut self,
        instructions: &[RegInst],
        signal_index: usize,
        pending: NonlocalExit,
    ) -> Option<usize> {
        let target = find_unwind_cleanup(instructions, signal_index)?;
        self.active_unwind_cleanup = Some(ActiveUnwindCleanup {
            stop_index: target.end_index,
            pending,
        });
        Some(target.cleanup_index + 1)
    }

    fn const_value(&mut self, value: &SsaConst) -> Option<LispValue> {
        match value {
            SsaConst::Nil => Some(LispValue::NIL),
            SsaConst::True => Some(LispValue::TRUE),
            SsaConst::Int(value) => LispValue::from_fixnum(*value),
            SsaConst::Char(value) => {
                let code: u32 = (*value).try_into().ok()?;
                char::from_u32(code).map(LispValue::from_char)
            }
            SsaConst::String(value) => Some(self.runtime.string(value.clone())),
            SsaConst::Float(_) => {
                self.unsupported("float constants require float object support");
                None
            }
        }
    }

    fn quote_value(&mut self, form: &SurfaceForm) -> Option<LispValue> {
        match &form.kind {
            SurfaceKind::Atom(atom) => self.quote_atom(atom),
            SurfaceKind::List(items) => {
                let values = items
                    .iter()
                    .map(|item| self.quote_value(item))
                    .collect::<Option<Vec<_>>>()?;
                Some(make_list(self.runtime, values))
            }
            SurfaceKind::DottedList(items, tail) => {
                let mut result = self.quote_value(tail)?;
                for item in items.iter().rev() {
                    let value = self.quote_value(item)?;
                    result = self.runtime.cons(value, result);
                }
                Some(result)
            }
            SurfaceKind::Quote(inner) => self.quote_prefixed_form("quote", inner),
            SurfaceKind::FunctionQuote(inner) => self.quote_prefixed_form("function", inner),
            SurfaceKind::Backquote(inner) => self.quote_prefixed_form("quasiquote", inner),
            SurfaceKind::Comma(inner) => self.quote_prefixed_form("unquote", inner),
            SurfaceKind::CommaAt(inner) => self.quote_prefixed_form("unquote-splicing", inner),
            SurfaceKind::Vector(_) => {
                self.unsupported("quoted vectors require vector object support");
                None
            }
        }
    }

    fn function_quote_value(&mut self, form: &SurfaceForm) -> Option<LispValue> {
        if let Some(name) = form.symbol_name() {
            return Some(self.runtime.intern(name));
        }
        self.quote_value(form)
    }

    fn quote_atom(&mut self, atom: &SurfaceAtom) -> Option<LispValue> {
        match atom {
            SurfaceAtom::Nil => Some(LispValue::NIL),
            SurfaceAtom::True => Some(LispValue::TRUE),
            SurfaceAtom::Symbol(name) => Some(self.runtime.intern(name)),
            SurfaceAtom::Int(value) => LispValue::from_fixnum(*value),
            SurfaceAtom::Char(value) => {
                let code: u32 = (*value).try_into().ok()?;
                char::from_u32(code).map(LispValue::from_char)
            }
            SurfaceAtom::String(value) => Some(self.runtime.string(value.clone())),
            SurfaceAtom::Float(_) => {
                self.unsupported("quoted floats require float object support");
                None
            }
        }
    }

    fn quote_prefixed_form(&mut self, name: &str, inner: &SurfaceForm) -> Option<LispValue> {
        let head = self.runtime.intern(name);
        let value = self.quote_value(inner)?;
        let tail = self.runtime.cons(value, LispValue::NIL);
        Some(self.runtime.cons(head, tail))
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

    fn runtime_bool(&mut self, result: Result<bool, crate::RuntimeError>) -> Option<LispValue> {
        match result {
            Ok(value) => Some(bool_value(value)),
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
        self.last_value = Some(value);
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

    fn finish(self, value: Option<LispValue>) -> InternalInterpResult {
        InternalInterpResult {
            value,
            thrown: None,
            signaled: None,
            diagnostics: self.diagnostics,
        }
    }

    fn finish_throw(mut self, thrown: ThrownValue) -> InternalInterpResult {
        match self.catch_throw(thrown) {
            Ok(value) => self.finish(Some(value)),
            Err(thrown) => InternalInterpResult {
                value: None,
                thrown: Some(thrown),
                signaled: None,
                diagnostics: self.diagnostics,
            },
        }
    }

    fn finish_signal(self, signaled: SignaledValue) -> InternalInterpResult {
        InternalInterpResult {
            value: None,
            thrown: None,
            signaled: Some(signaled),
            diagnostics: self.diagnostics,
        }
    }

    fn finish_nonlocal(self, pending: NonlocalExit) -> InternalInterpResult {
        match pending {
            NonlocalExit::Throw(thrown) => self.finish_throw(thrown),
            NonlocalExit::Signal(signaled) => self.finish_signal(signaled),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ConditionHandlerTarget {
    handler_index: usize,
    stop_index: usize,
    condition_end_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UnwindCleanupTarget {
    cleanup_index: usize,
    end_index: usize,
}

fn find_condition_case_end(instructions: &[RegInst], handler_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, inst) in instructions.iter().enumerate().skip(handler_index + 1) {
        match &inst.kind {
            RegInstKind::ConditionCaseBegin { .. } => depth += 1,
            RegInstKind::ConditionCaseEnd if depth == 0 => return Some(index),
            RegInstKind::ConditionCaseEnd => depth -= 1,
            _ => {}
        }
    }
    None
}

fn find_unwind_cleanup(
    instructions: &[RegInst],
    signal_index: usize,
) -> Option<UnwindCleanupTarget> {
    let mut depth = 0usize;
    let mut cleanup_index = None;
    for (index, inst) in instructions.iter().enumerate().skip(signal_index + 1) {
        match &inst.kind {
            RegInstKind::UnwindProtectBegin => depth += 1,
            RegInstKind::UnwindProtectCleanup if depth == 0 => cleanup_index = Some(index),
            RegInstKind::UnwindProtectEnd if depth == 0 => {
                return cleanup_index.map(|cleanup_index| UnwindCleanupTarget {
                    cleanup_index,
                    end_index: index,
                });
            }
            RegInstKind::UnwindProtectEnd => depth -= 1,
            _ => {}
        }
    }
    None
}

fn find_condition_handler(
    instructions: &[RegInst],
    signal_index: usize,
    signal_name: &str,
) -> Option<ConditionHandlerTarget> {
    let mut depth = 0usize;
    let mut handler_index = None;
    let mut stop_index = None;
    for (index, inst) in instructions.iter().enumerate().skip(signal_index + 1) {
        match &inst.kind {
            RegInstKind::ConditionCaseBegin { .. } => depth += 1,
            RegInstKind::ConditionCaseEnd if depth == 0 => {
                return handler_index.map(|handler_index| ConditionHandlerTarget {
                    handler_index,
                    stop_index: stop_index.unwrap_or(index),
                    condition_end_index: index,
                });
            }
            RegInstKind::ConditionCaseEnd => depth -= 1,
            RegInstKind::ConditionCaseHandler { pattern } if depth == 0 => {
                if handler_index.is_some() && stop_index.is_none() {
                    stop_index = Some(index);
                } else if handler_index.is_none() && condition_pattern_matches(pattern, signal_name)
                {
                    handler_index = Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn condition_pattern_matches(pattern: &SurfaceForm, signal_name: &str) -> bool {
    if let Some(name) = pattern.symbol_name() {
        return condition_name_matches(name, signal_name);
    }
    let SurfaceKind::List(items) = &pattern.kind else {
        return false;
    };
    items
        .iter()
        .filter_map(SurfaceForm::symbol_name)
        .any(|name| condition_name_matches(name, signal_name))
}

fn condition_name_matches(pattern_name: &str, signal_name: &str) -> bool {
    pattern_name == signal_name || pattern_name == "error"
}

fn instruction_result_reg(kind: &RegInstKind) -> Option<RegId> {
    match kind {
        RegInstKind::LoadConst { dst, .. }
        | RegInstKind::Quote { dst, .. }
        | RegInstKind::FunctionQuote { dst, .. }
        | RegInstKind::Lambda { dst, .. }
        | RegInstKind::Move { dst, .. }
        | RegInstKind::LexicalGet { dst, .. }
        | RegInstKind::LexicalSet { dst, .. }
        | RegInstKind::MakeLexicalCell { dst, .. }
        | RegInstKind::LexicalCellGet { dst, .. }
        | RegInstKind::LexicalCellSet { dst, .. }
        | RegInstKind::SymbolGet { dst, .. }
        | RegInstKind::SymbolSet { dst, .. }
        | RegInstKind::CallNamed { dst, .. }
        | RegInstKind::Funcall { dst, .. }
        | RegInstKind::Apply { dst, .. } => Some(*dst),
        RegInstKind::BindLexical { .. }
        | RegInstKind::BindDynamic { .. }
        | RegInstKind::UnbindDynamic { .. }
        | RegInstKind::DeclareSpecial { .. }
        | RegInstKind::CatchBegin { .. }
        | RegInstKind::CatchEnd
        | RegInstKind::Throw { .. }
        | RegInstKind::ConditionCaseBegin { .. }
        | RegInstKind::ConditionCaseHandler { .. }
        | RegInstKind::ConditionCaseEnd
        | RegInstKind::UnwindProtectBegin
        | RegInstKind::UnwindProtectCleanup
        | RegInstKind::UnwindProtectEnd
        | RegInstKind::Safepoint { .. } => None,
    }
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

fn is_primitive_name(name: &str) -> bool {
    matches!(
        name,
        "cons"
            | "car"
            | "cdr"
            | "setcar"
            | "setcdr"
            | "eq"
            | "eql"
            | "equal"
            | "consp"
            | "listp"
            | "numberp"
            | "symbolp"
            | "stringp"
            | "symbol-value"
            | "set"
            | "boundp"
            | "fboundp"
            | "symbol-function"
            | "intern"
            | "symbol-name"
            | "not"
            | "null"
            | "list"
            | "length"
            | "reverse"
            | "append"
            | "nth"
            | "memq"
            | "+"
            | "*"
            | "-"
            | "/"
            | "1+"
            | "1-"
            | "="
            | "<"
            | "<="
            | ">"
            | ">="
            | "message"
            | "print"
            | "prin1"
            | "signal"
            | "error"
            | "funcall"
            | "apply"
    )
}

#[cfg(test)]
mod tests {
    use neovm_compiler::compile_source;

    use crate::object_interp::{ObjectInterpResult, execute_module_with_args};
    use crate::{LispValue, Runtime};

    fn execute_result(source: &str) -> (ObjectInterpResult, Runtime) {
        let artifact = compile_source("object.el", source);
        assert_eq!(artifact.diagnostics, Vec::new());
        let regir = artifact.regir.expect("RegIR");
        let mut runtime = Runtime::new();
        let result = execute_module_with_args(&regir, &[], &mut runtime);
        (result, runtime)
    }

    fn execute(source: &str) -> (Option<LispValue>, Runtime) {
        let (result, runtime) = execute_result(source);
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

    #[test]
    fn executes_string_and_symbol_primitives() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: t; -*-\n(if (symbolp (intern \"alpha\")) (if (stringp (symbol-name 'alpha)) 9 0) 0)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(9)));
        assert_eq!(runtime.symbol_count(), 1);
    }

    #[test]
    fn executes_symbol_value_slots() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn (set (intern \"object-answer\") 41) (if (boundp 'object-answer) (1+ (symbol-value 'object-answer)) 0))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_global_symbol_get_and_set() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn (setq object-global 5) (1+ object-global))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn executes_funcall_and_apply_on_symbol_functions() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(+ (funcall 'car (cons 7 8)) (apply '+ 1 (list 2 3)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(13)));
    }

    #[test]
    fn executes_symbol_function_and_fboundp() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(if (fboundp 'car) (funcall (symbol-function 'car) (cons 4 5)) 0)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn executes_direct_lambda_function_object() {
        let (value, runtime) =
            execute(";;; -*- lexical-binding: t; -*-\n(funcall (lambda (x) (1+ x)) 4)");
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
        assert_eq!(runtime.function_count(), 1);
    }

    #[test]
    fn executes_lambda_with_value_capture() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((x 10)) (funcall (lambda (y) (+ x y)) 5))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn executes_lambda_with_mutable_cell_capture() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((x 0)) (let ((f (lambda () (setq x (+ x 1)) x))) (+ (funcall f) (funcall f))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
        assert_eq!(runtime.lexical_cell_count(), 1);
    }

    #[test]
    fn executes_dynamic_let_under_dynamic_binding_mode() {
        let (value, runtime) =
            execute(";;; -*- lexical-binding: nil; -*-\n(let ((x 1)) (+ (let ((x 2)) x) x))");
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
        assert_eq!(runtime.dynamic_binding_count(), 0);
    }

    #[test]
    fn setq_updates_active_dynamic_binding() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: nil; -*-\n(progn (setq dyn 7) (+ (let ((dyn 4)) (setq dyn 5) dyn) dyn))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(12)));
        assert_eq!(runtime.dynamic_binding_count(), 0);
    }

    #[test]
    fn executes_declared_special_let_under_lexical_binding() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn (setq special-dyn 10) (+ (let ((special-dyn 1)) (declare (special special-dyn)) (setq special-dyn 2) special-dyn) special-dyn))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(12)));
        assert_eq!(runtime.dynamic_binding_count(), 0);
    }

    #[test]
    fn executes_prog1_special_form() {
        let (value, _) =
            execute(";;; -*- lexical-binding: t; -*-\n(let ((x 1)) (+ (prog1 x (setq x 9)) x))");
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_and_or_special_forms() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((x 0)) (+ (if (and t 1 2) 10 0) (or nil 4 (setq x 99)) x))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(14)));
    }

    #[test]
    fn catches_direct_throw() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(catch 'tag (throw 'tag 42))");
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn propagates_throw_across_function_object() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(catch 'tag (funcall (lambda () (throw 'tag 7))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(7)));
    }

    #[test]
    fn reports_uncaught_throw() {
        let (result, _) = execute_result(";;; -*- lexical-binding: t; -*-\n(throw 'tag 1)");
        assert_eq!(result.value, None);
        assert_eq!(result.diagnostics.len(), 1);
        assert!(
            result.diagnostics[0]
                .message
                .contains("uncaught throw for tag tag")
        );
    }

    #[test]
    fn reports_error_as_uncaught_signal() {
        let (result, _) = execute_result(";;; -*- lexical-binding: t; -*-\n(error \"boom\")");
        assert_eq!(result.value, None);
        assert_eq!(result.diagnostics.len(), 1);
        assert!(
            result.diagnostics[0]
                .message
                .contains("uncaught signal error")
        );
        assert!(result.diagnostics[0].message.contains("\"boom\""));
    }

    #[test]
    fn reports_signal_as_uncaught_signal() {
        let (result, _) = execute_result(
            ";;; -*- lexical-binding: t; -*-\n(signal 'wrong-type-argument (list 'symbolp 1))",
        );
        assert_eq!(result.value, None);
        assert_eq!(result.diagnostics.len(), 1);
        assert!(
            result.diagnostics[0]
                .message
                .contains("uncaught signal wrong-type-argument")
        );
    }

    #[test]
    fn condition_case_skips_handlers_on_normal_completion() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((x 1)) (condition-case err (setq x 2) (error (setq x 99))) x)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn condition_case_handles_error_signal() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(condition-case err (error \"boom\") (error 42))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn condition_case_binds_signal_data() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(condition-case err (signal 'wrong-type-argument (list 'symbolp 1)) (wrong-type-argument (eq (car err) 'wrong-type-argument)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn unwind_protect_runs_cleanup_on_normal_completion() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((x 0)) (unwind-protect 42 (setq x 5)) x)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn unwind_protect_cleanup_can_override_throw() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(catch 'tag (unwind-protect (throw 'tag 7) (throw 'tag 8)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(8)));
    }
}
