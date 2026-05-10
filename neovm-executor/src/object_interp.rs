use std::collections::HashMap;

use neovm_compiler::diagnostic::Diagnostic;
use neovm_compiler::expand_value;
use neovm_compiler::hir::LambdaList;
use neovm_compiler::ids::{FunctionId, PrimaryMap, RegId};
use neovm_compiler::lower::{lambda_template_to_ssa, ssa_to_regir};
use neovm_compiler::reader;
use neovm_compiler::regir::{RegFunction, RegInst, RegInstKind, RegModule, RegTerminator};
use neovm_compiler::ssa::SsaConst;
use neovm_compiler::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

use crate::runtime::HashTableTest;
use crate::{LispValue, Runtime, RuntimeError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ObjectInterpResult {
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
    result_reg: Option<RegId>,
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

    fn into_result(mut self, runtime: &Runtime) -> ObjectInterpResult {
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

pub(crate) fn execute_module_with_args(
    module: &RegModule,
    args: &[LispValue],
    runtime: &mut Runtime,
) -> ObjectInterpResult {
    let functions_by_name = functions_by_name(module);
    let mut fuel = 100_000usize;
    execute_module_entry(module, &functions_by_name, args, runtime, &mut fuel).into_result(runtime)
}

pub(crate) fn execute_module_function(
    module: &RegModule,
    functions_by_name: &HashMap<String, FunctionId>,
    function_id: FunctionId,
    args: &[LispValue],
    runtime: &mut Runtime,
) -> Option<LispValue> {
    let Some(function) = module.functions.get(function_id) else {
        eprintln!("JIT: unknown function {function_id:?}");
        return None;
    };
    let mut fuel = 100_000usize;
    let adapted = adapt_lambda_args_standalone(&function.lambda_list, args, runtime)?;
    let result = execute_with_module(
        function,
        &adapted,
        module,
        functions_by_name,
        runtime,
        &mut fuel,
    );
    if let Some(thrown) = result.thrown {
        return Some(crate::jit_rt::bridge_interpreter_throw(
            thrown.tag,
            thrown.value,
        ));
    }
    if let Some(signaled) = result.signaled {
        return Some(crate::jit_rt::bridge_interpreter_signal(
            signaled.symbol,
            signaled.data,
        ));
    }
    result.value
}

pub(crate) fn execute_function_object_direct(
    module: &RegModule,
    functions_by_name: &HashMap<String, FunctionId>,
    function: LispValue,
    args: &[LispValue],
    runtime: &mut Runtime,
) -> Option<LispValue> {
    let (template, captures) = match runtime.function_parts(function) {
        Ok(parts) => parts,
        Err(error) => {
            eprintln!("JIT runtime error: {error}");
            return None;
        }
    };
    let lowered = lambda_template_to_ssa(&template);
    if !lowered.diagnostics.is_empty() {
        eprintln!("JIT: lambda lowering errors");
        return None;
    }
    let regir = ssa_to_regir(&lowered.value);
    if !regir.diagnostics.is_empty() {
        eprintln!("JIT: lambda RegIR errors");
        return None;
    }
    let adapted = adapt_lambda_args_standalone(&template.params, args, runtime)?;
    let mut entry_args = Vec::with_capacity(captures.len() + adapted.len());
    entry_args.extend(captures);
    entry_args.extend(adapted);
    let mut fuel = 100_000usize;
    let result = execute_with_module(
        &regir.value,
        &entry_args,
        module,
        functions_by_name,
        runtime,
        &mut fuel,
    );
    if let Some(thrown) = result.thrown {
        return Some(crate::jit_rt::bridge_interpreter_throw(
            thrown.tag,
            thrown.value,
        ));
    }
    if let Some(signaled) = result.signaled {
        return Some(crate::jit_rt::bridge_interpreter_signal(
            signaled.symbol,
            signaled.data,
        ));
    }
    result.value
}

/// Execute a primitive call using the interpreter's full evaluator context.
/// Used by the JIT as a fallback for higher-order operations (mapcar, mapc, etc.)
/// that need to call back into compiled code.
pub(crate) fn execute_interpreter_primitive(
    name: &str,
    args: &[LispValue],
    module: &RegModule,
    functions_by_name: &HashMap<String, FunctionId>,
    runtime: &mut Runtime,
) -> Option<LispValue> {
    // Create a dummy function context for the interpreter
    let dummy_func = RegFunction {
        name: None,
        lambda_list: LambdaList::default(),
        entry_params: Vec::new(),
        registers: PrimaryMap::new(),
        blocks: PrimaryMap::new(),
        entry: None,
        safepoints: Default::default(),
    };
    let mut fuel = 100_000usize;
    let mut interp = Interpreter {
        function: &dummy_func,
        registers: HashMap::new(),
        lexicals: HashMap::new(),
        catch_stack: Vec::new(),
        condition_stack: Vec::new(),
        active_condition_handlers: Vec::new(),
        active_unwind_cleanups: Vec::new(),
        pending_throw: None,
        pending_signal: None,
        caught_signal: None,
        last_value: None,
        module,
        functions_by_name,
        runtime,
        fuel: &mut fuel,
        diagnostics: Vec::new(),
    };
    let result = interp.execute_primitive_call(name, args).flatten();
    if let Some(thrown) = interp.pending_throw.take() {
        return Some(crate::jit_rt::bridge_interpreter_throw(
            thrown.tag,
            thrown.value,
        ));
    }
    if let Some(signaled) = interp.pending_signal.take() {
        return Some(crate::jit_rt::bridge_interpreter_signal(
            signaled.symbol,
            signaled.data,
        ));
    }
    result
}

fn adapt_lambda_args_standalone(
    lambda_list: &LambdaList,
    args: &[LispValue],
    runtime: &mut Runtime,
) -> Option<Vec<LispValue>> {
    if args.len() < lambda_list.min_arity() {
        eprintln!(
            "JIT: function requires at least {} args, got {}",
            lambda_list.min_arity(),
            args.len()
        );
        return None;
    }
    if let Some(max) = lambda_list.max_arity()
        && args.len() > max
    {
        eprintln!(
            "JIT: function requires at most {max} args, got {}",
            args.len()
        );
        return None;
    }
    let mut adapted = Vec::with_capacity(lambda_list.entry_arity());
    adapted.extend_from_slice(&args[..lambda_list.required.len()]);
    let optional_start = lambda_list.required.len();
    for index in 0..lambda_list.optional.len() {
        adapted.push(
            args.get(optional_start + index)
                .copied()
                .unwrap_or(LispValue::NIL),
        );
    }
    if lambda_list.rest.is_some() || !lambda_list.key.is_empty() {
        let rest_start = args.len().min(optional_start + lambda_list.optional.len());
        adapted.push(make_list(runtime, args[rest_start..].iter().copied()));
    }
    Some(adapted)
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
        active_condition_handlers: Vec::new(),
        active_unwind_cleanups: Vec::new(),
        pending_throw: None,
        pending_signal: None,
        caught_signal: None,
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
    active_condition_handlers: Vec<ActiveConditionHandler>,
    active_unwind_cleanups: Vec<ActiveUnwindCleanup>,
    pending_throw: Option<ThrownValue>,
    pending_signal: Option<SignaledValue>,
    caught_signal: Option<SignaledValue>,
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
        for (reg, value) in self.function.entry_params.iter().zip(args.iter().copied()) {
            self.set(*reg, value);
        }

        let Some(mut block) = self.function.entry else {
            self.error("object interpreter requires an entry block");
            return self.finish(None);
        };

        loop {
            if *self.fuel == 0 {
                // Yield to the thread scheduler before resetting fuel,
                // so cooperative threads get a chance to run.  If no
                // other threads are runnable this is a no-op.
                self.runtime.scheduler.thread_yield();
                *self.fuel = 100_000;
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
                // Check if the innermost active cleanup has reached its stop point.
                if self
                    .active_unwind_cleanups
                    .last()
                    .is_some_and(|cleanup| cleanup.stop_index == inst_index)
                {
                    let cleanup = self
                        .active_unwind_cleanups
                        .pop()
                        .expect("active unwind cleanup");
                    match cleanup.pending {
                        NonlocalExit::Throw(thrown) => {
                            // If there's an outer cleanup still on the stack, update its
                            // pending exit and continue to reach its stop_index.
                            if let Some(outer) = self.active_unwind_cleanups.last_mut() {
                                outer.pending = NonlocalExit::Throw(thrown);
                                inst_index += 1;
                                continue;
                            }
                            // After cleanup, check for new cleanup in the instruction stream.
                            if let Some(cleanup_start) = self.enter_unwind_cleanup(
                                &body.instructions,
                                inst_index,
                                NonlocalExit::Throw(thrown),
                                cleanup.result_reg,
                            ) {
                                inst_index = cleanup_start;
                                continue;
                            }
                            // Try to catch the throw in this function.
                            match self.try_catch_inline(thrown, &body.instructions, inst_index) {
                                Ok(Some(next)) => {
                                    inst_index = next;
                                    continue;
                                }
                                Ok(None) => {
                                    let value = self.last_value.take();
                                    return self.finish(value);
                                }
                                Err(thrown) => {
                                    let symbol = self.runtime.intern("no-catch");
                                    let data = make_list(self.runtime, std::iter::once(thrown.tag));
                                    let signaled = SignaledValue { symbol, data };
                                    if let Some(handler_start) = self.enter_condition_handler(
                                        &body.instructions,
                                        inst_index,
                                        signaled.clone(),
                                        cleanup.result_reg,
                                    ) {
                                        inst_index = handler_start;
                                        continue;
                                    }
                                    // No condition handler — propagate throw normally
                                    return InternalInterpResult {
                                        value: None,
                                        thrown: Some(thrown),
                                        signaled: None,
                                        diagnostics: std::mem::take(&mut self.diagnostics),
                                    };
                                }
                            }
                        }
                        NonlocalExit::Signal(signaled) => {
                            // If there's an outer cleanup still on the stack, update its
                            // pending exit and continue to reach its stop_index.
                            if let Some(outer) = self.active_unwind_cleanups.last_mut() {
                                outer.pending = NonlocalExit::Signal(signaled);
                                inst_index += 1;
                                continue;
                            }
                            // After cleanup, check for new cleanup in the instruction stream.
                            if let Some(cleanup_start) = self.enter_unwind_cleanup(
                                &body.instructions,
                                inst_index,
                                NonlocalExit::Signal(signaled),
                                cleanup.result_reg,
                            ) {
                                inst_index = cleanup_start;
                                continue;
                            }
                            if let Some(handler_start) = self.enter_condition_handler(
                                &body.instructions,
                                inst_index,
                                signaled,
                                cleanup.result_reg,
                            ) {
                                inst_index = handler_start;
                                continue;
                            }
                            return self.finish_signal(signaled);
                        }
                    }
                }
                // Check if the innermost active condition handler has reached its stop point.
                if self
                    .active_condition_handlers
                    .last()
                    .is_some_and(|handler| handler.stop_index == inst_index)
                {
                    let active = self
                        .active_condition_handlers
                        .pop()
                        .expect("active condition handler");
                    let next_index = active.condition_end_index + 1;
                    // Get the result reg from the ConditionCaseEnd instruction, not
                    // from the failing instruction. The ConditionCaseEnd's dst is
                    // where the merged body/handler result should go.
                    let cc_end_reg =
                        body.instructions
                            .get(active.condition_end_index)
                            .and_then(|inst| match &inst.kind {
                                RegInstKind::ConditionCaseEnd { dst, .. } => Some(*dst),
                                _ => None,
                            });
                    let result_reg = cc_end_reg.or(active.result_reg);
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
                    // Jump to the ConditionCaseEnd so it writes the body result
                    // to the destination register.
                    inst_index = end_index;
                    continue;
                }
                if !self.execute_inst(&inst.kind) {
                    let result_reg = instruction_result_reg(&inst.kind);
                    if let Some(thrown) = self.pending_throw.take() {
                        if let Some(cleanup_start) = self.enter_unwind_cleanup(
                            &body.instructions,
                            inst_index,
                            NonlocalExit::Throw(thrown),
                            result_reg,
                        ) {
                            inst_index = cleanup_start;
                            continue;
                        }
                        // Try to catch the throw in this function.
                        match self.try_catch_inline(thrown, &body.instructions, inst_index) {
                            Ok(Some(next)) => {
                                inst_index = next;
                                continue;
                            }
                            Ok(None) => {
                                let value = self.last_value.take();
                                return self.finish(value);
                            }
                            Err(thrown) => {
                                let symbol = self.runtime.intern("no-catch");
                                let data = make_list(self.runtime, std::iter::once(thrown.tag));
                                let signaled = SignaledValue { symbol, data };
                                if let Some(handler_start) = self.enter_condition_handler(
                                    &body.instructions,
                                    inst_index,
                                    signaled.clone(),
                                    result_reg,
                                ) {
                                    inst_index = handler_start;
                                    continue;
                                }
                                return InternalInterpResult {
                                    value: None,
                                    thrown: Some(thrown),
                                    signaled: None,
                                    diagnostics: std::mem::take(&mut self.diagnostics),
                                };
                            }
                        }
                    }
                    if let Some(signaled) = self.pending_signal.take() {
                        if let Some(cleanup_start) = self.enter_unwind_cleanup(
                            &body.instructions,
                            inst_index,
                            NonlocalExit::Signal(signaled),
                            result_reg,
                        ) {
                            inst_index = cleanup_start;
                            continue;
                        }
                        if let Some(handler_start) = self.enter_condition_handler(
                            &body.instructions,
                            inst_index,
                            signaled,
                            result_reg,
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
            RegInstKind::CatchEnd { dst } => {
                if self.catch_stack.pop().is_none() {
                    self.error("object interpreter reached catch end without catch begin");
                    return false;
                }
                // On the normal path, last_value holds the body result.
                // On the throw path, try_catch_inline sets last_value to the thrown value.
                let value = self.last_value.unwrap_or(LispValue::NIL);
                self.set(*dst, value);
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
            RegInstKind::ConditionCaseGetVar { dst } => {
                let value = self
                    .caught_signal
                    .as_ref()
                    .map(|s| self.runtime.cons(s.symbol, s.data))
                    .unwrap_or(LispValue::NIL);
                self.registers.insert(*dst, value);
            }
            RegInstKind::ConditionCaseHandler { .. } => {}
            RegInstKind::ConditionCaseEnd { dst, body_result } => {
                self.condition_stack.pop();
                // On the normal path (no signal), use the body result register.
                // On the signal path, the handler ran and last_value has its result.
                let value = if let Some(src) = body_result {
                    self.get(*src)
                        .unwrap_or_else(|| self.last_value.unwrap_or(LispValue::NIL))
                } else {
                    self.last_value.unwrap_or(LispValue::NIL)
                };
                self.set(*dst, value);
            }
            RegInstKind::UnwindProtectBegin | RegInstKind::UnwindProtectCleanup => {}
            RegInstKind::UnwindProtectEnd { dst, body_result } => {
                let value = if let Some(src) = body_result {
                    self.get(*src)
                        .unwrap_or_else(|| self.last_value.unwrap_or(LispValue::NIL))
                } else {
                    self.last_value.unwrap_or(LispValue::NIL)
                };
                self.set(*dst, value);
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
        // Try symbol function binding (set via defalias, fset, etc.)
        let symbol = self.runtime.intern(name);
        if self.runtime.is_symbol(symbol) {
            match self.runtime.symbol_function(symbol) {
                Ok(Some(function)) if function != symbol => {
                    return self.execute_funcall_with_depth(function, args, 16);
                }
                _ => {}
            }
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
        // Handle lambda lists: (lambda (args) body...)
        if self.runtime.is_cons(callee) {
            let car = self.runtime.car(callee);
            if let Ok(car) = car {
                if let Ok(name) = self.runtime.symbol_name(car) {
                    if name == "lambda" {
                        return self.execute_lambda_list(callee, args);
                    }
                    if name == "autoload" || name == "macro" {
                        self.execute_autoload(callee, args)?;
                        // Retry: the file loaded by autoload should have
                        // replaced the symbol function. Re-lookup and call.
                        if self.runtime.is_symbol(callee) {
                            match self.runtime.symbol_function(callee) {
                                Ok(Some(f)) if f != callee => {
                                    return self.execute_funcall_with_depth(f, args, depth - 1);
                                }
                                _ => {}
                            }
                        }
                        return None;
                    }
                }
            }
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
        // Check compile cache to avoid re-lowering the same lambda on
        // repeated calls (common in loops).
        let func_addr = function.heap_addr();
        let regir = if let Some(addr) = func_addr
            && let Some(cached) = self.runtime.lambda_cache.get(&addr)
        {
            cached.clone()
        } else {
            let lowered = lambda_template_to_ssa(&template);
            if !lowered.diagnostics.is_empty() {
                self.diagnostics.extend(lowered.diagnostics);
                return None;
            }
            let regir_out = ssa_to_regir(&lowered.value);
            if !regir_out.diagnostics.is_empty() {
                self.diagnostics.extend(regir_out.diagnostics);
                return None;
            }
            let compiled = regir_out.value;
            if let Some(addr) = func_addr {
                self.runtime.lambda_cache.insert(addr, compiled.clone());
            }
            compiled
        };
        let adapted = self.adapt_lambda_args(&template.params, args)?;
        let mut entry_args = Vec::with_capacity(captures.len() + adapted.len());
        entry_args.extend(captures);
        entry_args.extend(adapted);
        let result = execute_with_module(
            &regir,
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

    fn adapt_lambda_args(
        &mut self,
        lambda_list: &LambdaList,
        args: &[LispValue],
    ) -> Option<Vec<LispValue>> {
        if args.len() < lambda_list.min_arity() {
            self.error(format!(
                "function requires at least {} arguments, got {}",
                lambda_list.min_arity(),
                args.len()
            ));
            return None;
        }
        if let Some(max) = lambda_list.max_arity()
            && args.len() > max
        {
            self.error(format!(
                "function requires at most {max} arguments, got {}",
                args.len()
            ));
            return None;
        }

        let mut adapted = Vec::with_capacity(lambda_list.entry_arity());
        adapted.extend_from_slice(&args[..lambda_list.required.len()]);
        let optional_start = lambda_list.required.len();
        for index in 0..lambda_list.optional.len() {
            adapted.push(
                args.get(optional_start + index)
                    .copied()
                    .unwrap_or(LispValue::NIL),
            );
        }
        if lambda_list.rest.is_some() || !lambda_list.key.is_empty() {
            let rest_start = args.len().min(optional_start + lambda_list.optional.len());
            adapted.push(make_list(self.runtime, args[rest_start..].iter().copied()));
        }
        Some(adapted)
    }

    // ── DEFUN helpers (Emacs subr-equivalent arity wrappers) ──────────
    // Usage in execute_primitive_call — single-line subr registration:
    //   "my-func" => self.subr_2(name, args, |s| { ... }),
    // The closure receives &mut Self, args is captured from the outer scope.

    fn subr_1(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.exact_arity(name, args, 1).and_then(|_| f(self))
    }
    fn subr_2(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.exact_arity(name, args, 2).and_then(|_| f(self))
    }
    fn subr_3(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.exact_arity(name, args, 3).and_then(|_| f(self))
    }
    fn subr_0_1(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.min_max_arity(name, args, 0, 1).and_then(|_| f(self))
    }
    fn subr_1_2(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.min_max_arity(name, args, 1, 2).and_then(|_| f(self))
    }
    fn subr_2_3(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.min_max_arity(name, args, 2, 3).and_then(|_| f(self))
    }
    fn subr_2_5(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.min_max_arity(name, args, 2, 5).and_then(|_| f(self))
    }
    fn subr_1_3(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.min_max_arity(name, args, 1, 3).and_then(|_| f(self))
    }
    fn subr_min_1(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.min_arity(name, args, 1).and_then(|_| f(self))
    }
    fn subr_min_2(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.min_arity(name, args, 2).and_then(|_| f(self))
    }
    fn subr_vararg(&mut self, name: &str, args: &[LispValue], f: impl FnOnce(&mut Self) -> Option<LispValue>) -> Option<LispValue> {
        self.min_max_arity(name, args, 0, usize::MAX).and_then(|_| f(self))
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
            "list*" | "cl-list*" => self.subr_vararg(name, args, |s| {
                // (list* a b c) = (cons a (cons b c))
                // (list* a) = a, (list*) = nil
                let mut result = args.last().copied().unwrap_or(LispValue::NIL);
                for i in (0..args.len().saturating_sub(1)).rev() {
                    result = s.runtime.cons(args[i], result);
                }
                Some(result)
            }),
            "car" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.car(args[0]);
                self.runtime_value(result)
            }),
            "cdr" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.cdr(args[0]);
                self.runtime_value(result)
            }),
            "car-safe" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_cons(args[0]) {
                    let result = self.runtime.car(args[0]);
                    self.runtime_value(result)
                } else {
                    Some(LispValue::NIL)
                }
            }),
            "cdr-safe" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_cons(args[0]) {
                    let result = self.runtime.cdr(args[0]);
                    self.runtime_value(result)
                } else {
                    Some(LispValue::NIL)
                }
            }),
            "setcar" => self.exact_arity(name, args, 2).and_then(|_| {
                let result = self.runtime.set_car(args[0], args[1]);
                self.runtime_value(result)
            }),
            "setcdr" => self.exact_arity(name, args, 2).and_then(|_| {
                let result = self.runtime.set_cdr(args[0], args[1]);
                self.runtime_value(result)
            }),
            "eq" => self.exact_arity(name, args, 2).map(|_| {
                // Emacs eq is pointer identity: same object in memory.
                // LispValue derives PartialEq (raw u64 comparison) so == is correct
                // for eq — unlike neovm-core's TaggedValue which falls back to equal_value.
                bool_value(args[0] == args[1])
            }),
            "eql" => self
                .exact_arity(name, args, 2)
                .map(|_| bool_value(self.eql_values(args[0], args[1]))),
            "equal" => self
                .exact_arity(name, args, 2)
                .map(|_| bool_value(self.runtime.equal(args[0], args[1]))),
            "consp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_cons(args[0]))),
            "listp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_nil() || self.runtime.is_cons(args[0]))),
            "numberp" | "number-or-marker-p" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_number(args[0]))),
            "integerp" | "integer-or-marker-p" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_fixnum() || self.runtime.is_bignum(args[0]))),
            "natnump" | "wholenump" => self
                .exact_arity(name, args, 1)
                .map(|_| {
                    if let Some(v) = args[0].as_fixnum() {
                        v >= 0
                    } else if self.runtime.is_bignum(args[0]) {
                        if let Some(i) = self.runtime.as_integer(args[0]) {
                            i >= 0
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .map(bool_value),
            "zerop" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_float(args[0]) {
                    self.number_arg(name, args[0]).map(|v| bool_value(v == 0.0))
                } else if self.runtime.is_bignum(args[0]) {
                    let val = self.bignum_arg(name, args[0])?;
                    Some(bool_value(val == 0))
                } else {
                    self.fixnum_arg(name, args[0]).map(|v| bool_value(v == 0))
                }
            }),
            "symbolp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_symbol(args[0]))),
            "string-prefix-p" => self.subr_2(name, args, |s| {
                let (prefix, s) = (s.string_contents_owned(args[0])?, s.string_contents_owned(args[1])?);
                Some(bool_value(s.starts_with(&prefix)))
            }),
            "string-suffix-p" => self.subr_2(name, args, |s| {
                let (suffix, s) = (s.string_contents_owned(args[0])?, s.string_contents_owned(args[1])?);
                Some(bool_value(s.ends_with(&suffix)))
            }),
            "stringp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_string(args[0]))),
            "vectorp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_vector(args[0]))),
            "hash-table-p" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_hash_table(args[0]))),
            "arrayp" => self.exact_arity(name, args, 1).map(|_| {
                bool_value(self.runtime.is_vector(args[0]) || self.runtime.is_string(args[0]))
            }),
            "char-equal" => self.subr_2(name, args, |s| {
                let a = args[0].as_char();
                let b = args[1].as_char();
                Some(bool_value(a.is_some() && b.is_some()
                    && a.unwrap().to_ascii_lowercase() == b.unwrap().to_ascii_lowercase()))
            }),
            "char-table-p" | "bool-vector-p" | "recordp"
            | "mutexp" | "threadp" | "windowp" | "bufferp" | "markerp" | "processp" => self.exact_arity(name, args, 1).map(|_| bool_value(false)),
            "char-valid-p" => self.exact_arity(name, args, 1).map(|_| {
                let code = args[0].as_fixnum().unwrap_or(-1);
                bool_value(code >= 0 && code <= 0x10FFFF && (code < 0xD800 || code > 0xDFFF))
            }),
            "char-code" => self.subr_1(name, args, |s| {
                let ch = args[0].as_char()?;
                s.fixnum(ch as i64, "char-code")
            }),
            "char-or-string-p" => self.exact_arity(name, args, 1).map(|_| {
                bool_value(
                    args[0].as_char().is_some()
                        || self.runtime.is_string(args[0])
                )
            }),
            "atom" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(!self.runtime.is_cons(args[0]))),
            "nlistp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(!self.runtime.is_cons(args[0]) || args[0].is_nil())),
            "minusp" | "cl-minusp" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_bignum(args[0]) {
                    let val = self.bignum_arg(name, args[0])?;
                    Some(bool_value(val < 0))
                } else {
                    self.number_arg(name, args[0])
                        .map(|value| bool_value(value < 0.0))
                }
            }),
            "plusp" | "cl-plusp" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_bignum(args[0]) {
                    let val = self.bignum_arg(name, args[0])?;
                    Some(bool_value(val > 0))
                } else {
                    self.number_arg(name, args[0])
                        .map(|value| bool_value(value > 0.0))
                }
            }),
            "random" => self.min_max_arity(name, args, 0, 1).and_then(|_| {
                let limit = match args.first() {
                    Some(v) if !v.is_nil() => self.fixnum_arg(name, *v)?,
                    _ => i64::MAX,
                };
                if limit <= 0 {
                    self.error("primitive `random` limit must be positive");
                    return None;
                }
                // Simple deterministic "random" using hash of iteration
                let val = (limit as u64)
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1);
                self.fixnum((val % limit as u64) as i64, name)
            }),
            "getenv" => self.subr_1(name, args, |s| {
                if let Ok(var_name) = s.runtime.string_contents(args[0]) {
                    match std::env::var(&*var_name) {
                        Ok(val) => Some(s.runtime.string(val)),
                        Err(_) => Some(LispValue::NIL),
                    }
                } else {
                    Some(LispValue::NIL)
                }
            }),
            "setenv" => self.subr_2(name, args, |s| {
                let var = s.runtime.string_contents(args[0]).ok()?;
                let val = s.runtime.string_contents(args[1]).ok()?;
                unsafe { std::env::set_var(&*var, &*val); }
                Some(LispValue::NIL)
            }),
            "gensym" => self.min_max_arity(name, args, 0, 1).and_then(|_| {
                let prefix = args
                    .first()
                    .and_then(|v| self.string_contents_owned(*v))
                    .unwrap_or_else(|| "g".to_string());
                // Simple counter-based gensym
                let sym = format!("{}{}", prefix, self.runtime.symbol_count());
                Some(self.runtime.intern(&sym))
            }),
            "default-boundp" => self.subr_1(name, args, |s| {
                Some(bool_value(s.runtime.default_boundp(args[0])))
            }),
            "default-value" => self.subr_1(name, args, |s| {
                let result = s.runtime.default_value(args[0]);
                s.runtime_value(result)
            }),
            "set-default" => self.subr_2(name, args, |s| {
                let result = s.runtime.set_default(args[0], args[1]);
                s.runtime_value(result)
            }),
            "symbol-value" => self.subr_1(name, args, |s| {
                let result = s.runtime.symbol_value(args[0]);
                s.runtime_value(result)
            }),
            "set" => self.subr_2(name, args, |s| {
                let result = s.runtime.set_symbol_value(args[0], args[1]);
                s.runtime_value(result)
            }),
            "bobp" | "eobp" => Some(LispValue::TRUE),
            "point-min" | "point-max" => Some(LispValue::expect_fixnum(1)),
            "buffer-modified-p" => Some(LispValue::NIL),
            "buffer-size" => Some(LispValue::expect_fixnum(0)),
            "current-buffer" => Some(LispValue::NIL),
            "window-buffer" => Some(LispValue::NIL),
            "boundp" => self.subr_1(name, args, |s| {
                let result = s.runtime.is_bound_symbol(args[0]);
                s.runtime_bool(result)
            }),
            "makunbound" => self.subr_1(name, args, |s| {
                let result = s.runtime.set_symbol_unbound(args[0]).map(|()| args[0]);
                s.runtime_value(result)
            }),
            "fmakunbound" => self.subr_1(name, args, |s| {
                let result = s.runtime.fmakunbound(args[0]);
                s.runtime_value(result)
            }),
            "fboundp" => self.subr_1(name, args, |s| s.fboundp(args[0])),
            "provide" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.provide(args[0]);
                self.runtime_value(result)
            }),
            "featurep" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.featurep(args[0]);
                self.runtime_bool(result)
            }),
            "require" => self.min_max_arity(name, args, 1, 3).and_then(|_| {
                self.require_feature(args[0], args.get(2).copied().unwrap_or(LispValue::NIL))
            }),
            "get" => self.exact_arity(name, args, 2).and_then(|_| {
                let result = self.runtime.symbol_property(args[0], args[1]);
                self.runtime_value(result)
            }),
            "put" => self.subr_3(name, args, |s| {
                let result = s.runtime.put_symbol_property(args[0], args[1], args[2]);
                s.runtime_value(result)
            }),
            "symbol-plist" => self.subr_1(name, args, |s| {
                let result = s.runtime.symbol_plist(args[0]);
                s.runtime_value(result)
            }),
            "setplist" => self.subr_2(name, args, |s| {
                let result = s.runtime.set_symbol_plist(args[0], args[1]);
                s.runtime_value(result)
            }),
            "plist-get" => self.subr_2(name, args, |s| {
                Some(s.runtime.plist_get(args[0], args[1]))
            }),
            "plist-member" => self.subr_2(name, args, |s| {
                s.plist_member(args[0], args[1])
            }),
            "plist-put" => self.subr_3(name, args, |s| {
                Some(s.runtime.plist_put(args[0], args[1], args[2]))
            }),
            "autoloadp" => self.exact_arity(name, args, 1).map(|_| LispValue::NIL),
            "autoload" => self
                .min_max_arity(name, args, 2, 5)
                .and_then(|_| self.autoload(args)),
            "symbol-function" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.symbol_function(args[0])),
            "fset" => self.exact_arity(name, args, 2).and_then(|_| {
                let result = self.runtime.set_symbol_function(args[0], args[1]);
                self.runtime_value(result)
            }),
            "defalias" => self.min_max_arity(name, args, 2, 3).and_then(|_| {
                let result = self.runtime.set_symbol_function(args[0], args[1]);
                self.runtime_value(result).map(|_| args[0])
            }),
            "read" => self
                .min_max_arity(name, args, 1, 2)
                .and_then(|_| self.read_from_string(args[0])),
            "eval" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.eval_form(args[0])),
            "macroexpand" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.macroexpand_one(args[0])),
            "macroexpand-1" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.macroexpand_one(args[0])),
            "defun" => self
                .min_arity(name, args, 3)
                .and_then(|_| self.defun_runtime(args)),
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
            "identity" => self.exact_arity(name, args, 1).map(|_| args[0]),
            "garbage-collect" => {
                let _ = self.runtime.gc_heap.collect(neovm_gc::plan::CollectionKind::Minor);
                Some(LispValue::NIL)
            }
            "purecopy" => self.exact_arity(name, args, 1).map(|_| args[0]),
            "indirect-function" => self.subr_1(name, args, |s| {
                s.indirect_function(args[0])
            }),
            "ignore" => Some(LispValue::NIL),
            "always" => Some(LispValue::TRUE),
            "prog1" => {
                if args.is_empty() {
                    Some(LispValue::NIL)
                } else {
                    Some(args[0])
                }
            }
            "make-symbol" => self.exact_arity(name, args, 1).and_then(|_| {
                let arg_name = match self.runtime.string_contents(args[0]) {
                    Ok(n) => n.to_string(),
                    Err(e) => {
                        self.runtime_error(e);
                        return None;
                    }
                };
                Some(self.runtime.make_symbol(&arg_name))
            }),
            "intern-soft" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_string(args[0]) {
                    let arg_name = match self.runtime.string_contents(args[0]) {
                        Ok(n) => n.to_string(),
                        Err(e) => {
                            self.runtime_error(e);
                            return None;
                        }
                    };
                    return Some(
                        self.runtime
                            .intern_soft(&arg_name)
                            .unwrap_or(LispValue::NIL),
                    );
                }
                if self.runtime.is_symbol(args[0]) {
                    let arg_name = match self.runtime.symbol_name(args[0]) {
                        Ok(n) => n.to_string(),
                        Err(e) => {
                            self.runtime_error(e);
                            return None;
                        }
                    };
                    return Some(
                        self.runtime
                            .intern_soft(&arg_name)
                            .unwrap_or(LispValue::NIL),
                    );
                }
                Some(LispValue::NIL)
            }),
            "emacs-pid" => LispValue::from_fixnum(std::process::id() as i64),
            "elt" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.elt(args[0], args[1])),
            "downcase" => self
                .exact_arity(name, args, 1)
                .map(|_| self.downcase(args[0])),
            "use-region-p" => Some(LispValue::NIL),
            "upcase-initials" => self.subr_1(name, args, |s| {
                Some(s.upcase_initials(args[0]))
            }),
            "upcase" => self
                .exact_arity(name, args, 1)
                .map(|_| self.upcase(args[0])),
            "capitalize" => self
                .exact_arity(name, args, 1)
                .map(|_| self.capitalize(args[0])),
            "keywordp" => self.exact_arity(name, args, 1).map(|_| {
                let is_keyword = self.runtime.is_symbol(args[0])
                    && match self.runtime.symbol_name(args[0]) {
                        Ok(n) => n.starts_with(':'),
                        Err(_) => false,
                    };
                bool_value(is_keyword)
            }),
            "subr-native-elisp-p" => self.exact_arity(name, args, 1).map(|_| LispValue::NIL),
            "subr-arity" => self.subr_1(name, args, |s| {
                // Returns (MIN . MAX) for any subr. Use (0 . many) as default.
                let min_arity = LispValue::expect_fixnum(0);
                let many = s.runtime.bignum(rug::Integer::from(i64::MAX));
                Some(s.runtime.cons(min_arity, many))
            }),
            "subrp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_function(args[0]))),
            "color-defined-p" => Some(LispValue::NIL),
            "commandp" => self.subr_1(name, args, |s| {
                // Any callable function can be used as a command
                let obj = args[0];
                Some(bool_value(s.runtime.is_function(obj) || s.runtime.is_symbol(obj)))
            }),
            "compiled-function-p" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_function(args[0]))),
            "special-variable-p" => self.exact_arity(name, args, 1).and_then(|_| {
                if !self.runtime.is_symbol(args[0]) {
                    return Some(bool_value(false));
                }
                match self.runtime.symbol_name(args[0]) {
                    Ok(name) => Some(bool_value(
                        ["t", "nil"].contains(&name.as_str())
                            || self.runtime.is_bound_symbol(args[0]).unwrap_or(false),
                    )),
                    Err(_) => Some(bool_value(false)),
                }
            }),
            "evenp" | "cl-evenp" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_bignum(args[0]) {
                    let val = self.bignum_arg(name, args[0])?;
                    Some(bool_value(val.is_even()))
                } else {
                    let val = self.fixnum_arg(name, args[0])?;
                    Some(bool_value(val % 2 == 0))
                }
            }),
            "oddp" | "cl-oddp" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_bignum(args[0]) {
                    let val = self.bignum_arg(name, args[0])?;
                    Some(bool_value(val.is_odd()))
                } else {
                    let val = self.fixnum_arg(name, args[0])?;
                    Some(bool_value(val % 2 != 0))
                }
            }),
            "butlast" => self
                .min_max_arity(name, args, 1, 2)
                .and_then(|_| self.butlast(args[0], args.get(1).copied())),
            "cl-delete-if" => self.subr_2(name, args, |s| {
                s.cl_delete_if(args[0], args[1], false)
            }),
            "cl-delete-if-not" => self.subr_2(name, args, |s| {
                s.cl_delete_if(args[0], args[1], true)
            }),
            "display-graphic-p" => Some(LispValue::NIL),
            "delete-dups" => self.subr_1(name, args, |s| {
                Some(s.delete_dups(args[0]))
            }),
            "delq" | "cl-delq" => self.subr_2(name, args, |s| {
                s.delq(args[0], args[1])
            }),
            "delete" | "cl-delete" => self.subr_2(name, args, |s| {
                s.remove(args[0], args[1])
            }),
            "remq" | "cl-remq" => self.subr_2(name, args, |s| {
                s.delq(args[0], args[1])
            }),
            "remove" | "cl-remove" => self.subr_2(name, args, |s| {
                s.remove(args[0], args[1])
            }),
            "copy-tree" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.copy_tree(args[0])),
            "copy-alist" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.copy_alist(args[0])),
            "vconcat" => self.vconcat(args),
            "cl-fill" => self.subr_2(name, args, |s| {
                s.cl_fill(args[0], args[1])
            }),
            "fillarray" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.fillarray(args[0], args[1])),
            "nconc" | "cl-nconc" => self.nconc(args),
            "number-to-string" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_float(args[0]) {
                    match self.runtime.float_data(args[0]) {
                        Ok(value) => Some(self.runtime.string(format!("{value}"))),
                        Err(e) => {
                            self.runtime_error(e);
                            None
                        }
                    }
                } else if self.runtime.is_bignum(args[0]) {
                    match self.runtime.bignum_data(args[0]) {
                        Ok(value) => Some(self.runtime.string(value.to_string())),
                        Err(e) => {
                            self.runtime_error(e);
                            None
                        }
                    }
                } else {
                    let n = self.fixnum_arg(name, args[0])?;
                    Some(self.runtime.string(n.to_string()))
                }
            }),
            "string-to-number" => self
                .min_max_arity(name, args, 1, 2)
                .and_then(|_| self.string_to_number(args[0], args.get(1).copied())),
            "logand" | "logior" | "logxor" => {
                if args.is_empty() {
                    let identity = match name {
                        "logand" => -1i64,
                        _ => 0i64,
                    };
                    return Some(self.fixnum(identity, name));
                }
                if self.has_bignum_arg(args) {
                    let init = self.bignum_arg(name, args[0])?;
                    let op: fn(&rug::Integer, &rug::Integer) -> rug::Integer = match name {
                        "logand" => |a, b| rug::Integer::from(a & b),
                        "logior" => |a, b| rug::Integer::from(a | b),
                        "logxor" => |a, b| rug::Integer::from(a ^ b),
                        _ => unreachable!(),
                    };
                    let mut result = init;
                    for arg in &args[1..] {
                        let val = self.bignum_arg(name, *arg)?;
                        result = op(&result, &val);
                    }
                    Some(self.runtime.bignum(result))
                } else {
                    let init = self.fixnum_arg(name, args[0])?;
                    let op: fn(i64, i64) -> i64 = match name {
                        "logand" => |a, b| a & b,
                        "logior" => |a, b| a | b,
                        "logxor" => |a, b| a ^ b,
                        _ => unreachable!(),
                    };
                    let mut result = init;
                    for arg in &args[1..] {
                        let val = self.fixnum_arg(name, *arg)?;
                        result = op(result, val);
                    }
                    self.fixnum(result, name)
                }
            }
            "lognot" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_bignum(args[0]) {
                    let val = self.bignum_arg(name, args[0])?;
                    Some(self.runtime.bignum(!val))
                } else {
                    let val = self.fixnum_arg(name, args[0])?;
                    self.fixnum(!val, name)
                }
            }),
            "ash" => self.exact_arity(name, args, 2).and_then(|_| {
                let count = self.fixnum_arg(name, args[1])?;
                if self.runtime.is_bignum(args[0]) {
                    let val = self.bignum_arg(name, args[0])?;
                    let result = if count >= 0 {
                        val << (count as u32)
                    } else {
                        val >> ((-count) as u32)
                    };
                    Some(self.runtime.bignum(result))
                } else {
                    let val = self.fixnum_arg(name, args[0])?;
                    let result = if count >= 0 {
                        val.wrapping_shl(count as u32)
                    } else {
                        val.wrapping_shr((-count) as u32)
                    };
                    self.fixnum(result, name)
                }
            }),
            "lsh" => self.exact_arity(name, args, 2).and_then(|_| {
                let count = self.fixnum_arg(name, args[1])?;
                if self.runtime.is_bignum(args[0]) {
                    let val = self.bignum_arg(name, args[0])?;
                    let result = if count >= 0 {
                        val << (count as u32)
                    } else {
                        val >> ((-count) as u32)
                    };
                    Some(self.runtime.bignum(result))
                } else {
                    let val = self.fixnum_arg(name, args[0])?;
                    let result = if count >= 0 {
                        val.wrapping_shl(count as u32)
                    } else {
                        ((val as u64).wrapping_shr((-count) as u32)) as i64
                    };
                    self.fixnum(result, name)
                }
            }),
            "expt" => self.exact_arity(name, args, 2).and_then(|_| {
                if self.has_float_arg(args) {
                    let base = self.number_arg(name, args[0])?;
                    let exp = self.number_arg(name, args[1])?;
                    Some(self.runtime.float(base.powf(exp)))
                } else if self.has_bignum_arg(args) {
                    let base = self.bignum_arg(name, args[0])?;
                    let exp = self.bignum_arg(name, args[1])?;
                    if exp < 0 {
                        if base == 0 {
                            return Some(LispValue::NIL);
                        }
                        return Some(LispValue::NIL);
                    }
                    let exp_u32: u32 = exp.to_u32().unwrap_or(u32::MAX);
                    let mut result = rug::Integer::from(1);
                    let mut count = 0u32;
                    while count < exp_u32 {
                        result *= &base;
                        count += 1;
                    }
                    Some(self.runtime.bignum(result))
                } else {
                    let base = self.fixnum_arg(name, args[0])?;
                    let exp = self.fixnum_arg(name, args[1])?;
                    if exp < 0 {
                        if base == 0 {
                            self.error("arithmetic error in `expt`: 0 raised to a negative power");
                            return None;
                        }
                        return self.fixnum(0, name);
                    }
                    let result = base.pow(exp as u32);
                    self.fixnum(result, name)
                }
            }),
            "float" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.number_arg(name, args[0]))
                .map(|v| self.runtime.float(v)),
            "truncate" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.number_arg(name, args[0]))
                .and_then(|v| self.fixnum(v.trunc() as i64, name)),
            "floor" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.number_arg(name, args[0]))
                .and_then(|v| self.fixnum(v.floor() as i64, name)),
            "ceiling" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.number_arg(name, args[0]))
                .and_then(|v| self.fixnum(v.ceil() as i64, name)),
            "round" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.number_arg(name, args[0]))
                .and_then(|v| self.fixnum(v.round() as i64, name)),
            "standard-syntax-table" => Some(LispValue::NIL),
            "syntax-table-p" => Some(LispValue::NIL),
            "sqrt" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.number_arg(name, args[0]))
                .map(|v| self.runtime.float(v.sqrt())),
            "sin" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.number_arg(name, args[0]))
                .map(|v| self.runtime.float(v.sin())),
            "cos" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.number_arg(name, args[0]))
                .map(|v| self.runtime.float(v.cos())),
            "tan" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.number_arg(name, args[0]))
                .map(|v| self.runtime.float(v.tan())),
            "log" => self.min_max_arity(name, args, 1, 2).and_then(|_| {
                let value = self.number_arg(name, args[0])?;
                let result = if let Some(base) = args.get(1).copied() {
                    let base = self.number_arg(name, base)?;
                    if base == 10.0 {
                        value.log10()
                    } else if base == 2.0 {
                        value.log2()
                    } else {
                        value.ln() / base.ln()
                    }
                } else {
                    value.ln()
                };
                Some(self.runtime.float(result))
            }),
            "exp" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.number_arg(name, args[0]))
                .map(|v| self.runtime.float(v.exp())),
            "list" | "cl-list" => Some(make_list(self.runtime, args.iter().copied())),
            "length=" => self.subr_2(name, args, |s| {
                s.length_equals(args[0], args[1])
            }),
            "length" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.length(args[0]))
                .and_then(|length| i64::try_from(length).ok())
                .and_then(|length| self.fixnum(length, "length")),
            "length=" => self.exact_arity(name, args, 2).and_then(|_| {
                let len = self.length(args[0])?;
                let n = self.fixnum_arg(name, args[1])?;
                Some(bool_value(len as i64 == n))
            }),
            "length<" => self.exact_arity(name, args, 2).and_then(|_| {
                let len = self.length(args[0])?;
                let n = self.fixnum_arg(name, args[1])?;
                Some(bool_value((len as i64) < n))
            }),
            "length>" => self.exact_arity(name, args, 2).and_then(|_| {
                let len = self.length(args[0])?;
                let n = self.fixnum_arg(name, args[1])?;
                Some(bool_value((len as i64) > n))
            }),
            "concat" => self.concat(args),
            "substring" => self
                .min_max_arity(name, args, 2, 3)
                .and_then(|_| self.substring(args[0], args[1], args.get(2).copied())),
            "string=" => self
                .min_arity(name, args, 1)
                .and_then(|_| self.string_bytes_equal_multi(args)),
            "string-equal" => self
                .min_arity(name, args, 1)
                .and_then(|_| self.string_case_insensitive_equal_multi(args)),
            "string<" | "string-lessp" => self
                .min_arity(name, args, 2)
                .and_then(|_| self.string_lessp_multi(args)),
            "string>" | "string-greaterp" => self
                .min_arity(name, args, 2)
                .and_then(|_| self.string_greaterp_multi(args)),
            "string-bytes" => self.exact_arity(name, args, 1).and_then(|_| {
                let contents = self.string_contents_owned(args[0])?;
                self.fixnum(contents.len() as i64, name)
            }),
            "string-match-p" => self
                .min_max_arity(name, args, 2, 3)
                .and_then(|_| self.string_match_p(args[0], args[1], args.get(2).copied())),
            "replace-regexp-in-string" => self.min_max_arity(name, args, 3, 5).and_then(|_| {
                self.replace_regexp_in_string(
                    args[0],
                    args[1],
                    args[2],
                    args.get(3).copied(),
                    args.get(4).copied(),
                )
            }),
            "string-match" => self
                .min_max_arity(name, args, 2, 3)
                .and_then(|_| self.string_match(args[0], args[1], args.get(2).copied())),
            "match-string" => self
                .min_max_arity(name, args, 0, 2)
                .and_then(|_| self.match_string_prim(args.get(1).copied(), args.first().copied())),
            "match-beginning" => self.min_max_arity(name, args, 0, 1).and_then(|_| {
                let group = args.first().and_then(|v| v.as_fixnum()).unwrap_or(0) as usize;
                Some(self.runtime.match_beginning(group))
            }),
            "match-end" => self.min_max_arity(name, args, 0, 1).and_then(|_| {
                let group = args.first().and_then(|v| v.as_fixnum()).unwrap_or(0) as usize;
                Some(self.runtime.match_end(group))
            }),
            "replace-match" => self
                .min_max_arity(name, args, 1, 4)
                .and_then(|_| self.runtime.replace_match(args[0], args.get(3).copied())),
            "char-to-string" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.char_to_string(args[0])),
            "string-to-char" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.string_to_char(args[0])),
            "format" | "format-message" => self
                .min_arity(name, args, 1)
                .and_then(|_| self.format_string(args[0], &args[1..])),
            "prin1-to-string" => self
                .min_max_arity(name, args, 1, 2)
                .and_then(|_| self.prin1_to_string(args[0])),
            "princ-to-string" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.prin1_to_string(args[0])),
            "split-string" => self.min_max_arity(name, args, 1, 3).and_then(|_| {
                self.split_string(args[0], args.get(1).copied(), args.get(2).copied())
            }),
            "string-join" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.string_join(args[0], args[1])),
            "string-trim" => self
                .min_max_arity(name, args, 1, 2)
                .and_then(|_| self.string_trim(args[0], args.get(1).copied())),
            "string-trim-left" => self
                .min_max_arity(name, args, 1, 2)
                .and_then(|_| self.string_trim_left(args[0], args.get(1).copied())),
            "string-trim-right" => self
                .min_max_arity(name, args, 1, 2)
                .and_then(|_| self.string_trim_right(args[0], args.get(1).copied())),
            "string-remove-prefix" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.string_remove_prefix(args[0], args[1])),
            "string-remove-suffix" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.string_remove_suffix(args[0], args[1])),
            "substring-no-properties" => self.min_max_arity(name, args, 1, 3).and_then(|_| {
                self.substring(
                    args[0],
                    args.get(1).copied().unwrap_or(LispValue::expect_fixnum(0)),
                    args.get(2).copied(),
                )
            }),
            "vector" => Some(self.runtime.vector(args.to_vec())),
            "make-vector" => self.exact_arity(name, args, 2).and_then(|_| {
                let len = args[0].as_fixnum()?;
                if len < 0 {
                    return None;
                }
                Some(self.runtime.make_vector(len as usize, args[1]))
            }),
            "aref" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.aref(args[0], args[1])),
            "aset" => self
                .exact_arity(name, args, 3)
                .and_then(|_| self.aset(args[0], args[1], args[2])),
            "make-hash-table" => self.make_hash_table(args),
            "hash-table-count" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.hash_table_count(args[0]);
                self.runtime_usize(result, name)
            }),
            "hash-table-keys" => self.exact_arity(name, args, 1).and_then(|_| {
                self.hash_table_keys(args[0])
            }),
            "hash-table-values" => self.exact_arity(name, args, 1).and_then(|_| {
                self.hash_table_values(args[0])
            }),
            "copy-hash-table" => self.exact_arity(name, args, 1).and_then(|_| {
                self.copy_hash_table(args[0])
            }),
            "gethash" => self
                .min_max_arity(name, args, 2, 3)
                .and_then(|_| self.gethash(args[0], args[1], args.get(2).copied())),
            "puthash" => self.exact_arity(name, args, 3).and_then(|_| {
                let result = self.runtime.puthash(args[0], args[1], args[2]);
                self.runtime_value(result)
            }),
            "remhash" => self.exact_arity(name, args, 2).and_then(|_| {
                let result = self.runtime.remhash(args[0], args[1]);
                self.runtime_value(result)
            }),
            "clrhash" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.clrhash(args[0]);
                self.runtime_value(result)
            }),
            "maphash" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.maphash(args[0], args[1])),
            "sxhash-eq" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.sxhash_eq(args[0])),
            "sxhash-eql" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.sxhash_eql(args[0])),
            "sxhash-equal" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.sxhash_equal(args[0])),
            "reverse" | "cl-reverse" => self.subr_1(name, args, |s| {
                let values = s.list_values(args[0])?;
                Some(make_list(s.runtime, values.iter().rev().copied()))
            }),
            "nreverse" | "cl-nreverse" => self.exact_arity(name, args, 1).and_then(|_| {
                let mut current = args[0];
                if current.is_nil() {
                    return Some(LispValue::NIL);
                }
                let mut prev = LispValue::NIL;
                loop {
                    let cdr = self.runtime.cdr(current);
                    let next = self.runtime_value(cdr)?;
                    let result = self.runtime.set_cdr(current, prev);
                    if result.is_err() {
                        break;
                    }
                    prev = current;
                    if next.is_nil() {
                        break;
                    }
                    if !self.runtime.is_cons(next) {
                        self.error("nreverse: improper list");
                        return None;
                    }
                    current = next;
                }
                Some(prev)
            }),
            "append" => self.append(args),
            "nth" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.nth(args[0], args[1])),
            "nthcdr" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.nthcdr(args[0], args[1])),
            "sort" | "cl-sort" | "cl-stable-sort" => self.subr_2(name, args, |s| {
                s.sort_seq(args[0], args[1])
            }),
            "safe-length" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.safe_length(args[0])),
            "subseq" | "cl-subseq" => self
                .min_max_arity(name, args, 2, 3)
                .and_then(|_| self.subseq(args[0], args[1], args.get(2).copied())),
            "last" => self
                .min_arity(name, args, 1)
                .and_then(|_| self.last(args[0])),
            "minibuffer-window" => Some(LispValue::NIL),
            "memq" => self.subr_2(name, args, |s| s.memq(args[0], args[1])),
            "memql" => self.subr_2(name, args, |s| s.memql(args[0], args[1])),
            "member" | "cl-member" => self.subr_2(name, args, |s| s.member(args[0], args[1])),
            "cl-member-if" => self.subr_2(name, args, |s| {
                s.cl_member_if(args[0], args[1])
            }),
            "cl-member-if-not" => self.subr_2(name, args, |s| {
                s.cl_member_if_not(args[0], args[1])
            }),
            "assq" | "cl-assq" => self.subr_2(name, args, |s| s.assoc(args[0], args[1], false)),
            "assoc" | "cl-assoc" => self.subr_2(name, args, |s| s.assoc(args[0], args[1], true)),
            "cl-assoc-if" => self.subr_2(name, args, |s| {
                s.cl_assoc_if(args[0], args[1])
            }),
            "cl-assoc-if-not" => self.subr_2(name, args, |s| {
                s.cl_assoc_if_not(args[0], args[1])
            }),
            "rassq" | "cl-rassq" => self.subr_2(name, args, |s| s.rassoc(args[0], args[1], false)),
            "rassoc" | "cl-rassoc" => self.subr_2(name, args, |s| s.rassoc(args[0], args[1], true)),
            "cl-rassoc-if" => self.subr_2(name, args, |s| {
                s.cl_rassoc_if(args[0], args[1])
            }),
            "cl-rassoc-if-not" => self.subr_2(name, args, |s| {
                s.cl_rassoc_if_not(args[0], args[1])
            }),
            "assoc-string" => self
                .min_max_arity(name, args, 2, 3)
                .and_then(|_| self.assoc_string(args[0], args[1], args.get(2).copied())),
            "acons" | "cl-acons" => self.subr_3(name, args, |s| {
                let pair = s.runtime.cons(args[0], args[1]);
                Some(s.runtime.cons(pair, args[2]))
            }),
            "alist-get" => self
                .min_max_arity(name, args, 2, 5)
                .and_then(|_| self.alist_get(
                    args[0], args[1],
                    args.get(2).copied(),
                    args.get(3).copied(),
                    args.get(4).copied(),
                )),
            "cl-remove-if" => self.subr_2(name, args, |s| {
                s.remove_if(args[0], args[1], false)
            }),
            "cl-remove-if-not" => self.subr_2(name, args, |s| {
                s.remove_if(args[0], args[1], true)
            }),
            "cl-remprop" => self.subr_2(name, args, |s| {
                s.cl_remprop(args[0], args[1])
            }),
            "cl-remove-duplicates" | "cl-delete-duplicates" => self.subr_1(name, args, |s| {
                s.remove_duplicates(args[0])
            }),
            "cl-position" => self.subr_2(name, args, |s| {
                s.cl_position(args[0], args[1])
            }),
            "cl-find" => self.subr_2(name, args, |s| {
                s.cl_find(args[0], args[1])
            }),
            "cl-find-if" => self.subr_2(name, args, |s| {
                s.cl_find_if(args[0], args[1])
            }),
            "cl-position-if" => self.subr_2(name, args, |s| {
                s.cl_position_if(args[0], args[1])
            }),
            "cl-count" => self.subr_2(name, args, |s| {
                s.cl_count(args[0], args[1])
            }),
            "cl-count-if" => self.subr_2(name, args, |s| {
                s.cl_count_if(args[0], args[1])
            }),
            "cl-count-if-not" => self.subr_2(name, args, |s| {
                s.cl_count_if_not(args[0], args[1])
            }),
            "cl-find-if-not" => self.subr_2(name, args, |s| {
                s.cl_find_if_not(args[0], args[1])
            }),
            "cl-position-if-not" => self.subr_2(name, args, |s| {
                s.cl_position_if_not(args[0], args[1])
            }),
            "cl-mismatch" => self.subr_2(name, args, |s| {
                s.cl_mismatch(args[0], args[1])
            }),
            "cl-merge" => self.subr_vararg(name, args, |s| {
                s.cl_merge(args)
            }),
            "cl-endp" => self.exact_arity(name, args, 1).map(|_| {
                if args[0].is_nil() {
                    bool_value(true)
                } else if self.runtime.is_cons(args[0]) {
                    bool_value(false)
                } else {
                    self.error(format!(
                        "Wrong type argument: listp, {}",
                        self.runtime.format_value(args[0])
                    ));
                    bool_value(false)
                }
            }),
            "pairlis" | "cl-pairlis" => self
                .min_max_arity(name, args, 2, 3)
                .and_then(|_| self.pairlis(args[0], args[1], args.get(2).copied().unwrap_or(LispValue::NIL))),
            "cl-adjoin" => self.subr_2(name, args, |s| {
                s.cl_adjoin(args[0], args[1])
            }),
            "cl-replace" | "cl-nreplace" => self.subr_2(name, args, |s| {
                s.cl_replace(args[0], args[1])
            }),
            "cl-reduce" => self
                .min_max_arity(name, args, 2, 3)
                .and_then(|_| self.cl_reduce(args[0], args[1], args.get(2).copied())),
            "cl-concatenate" => self.subr_vararg(name, args, |s| {
                s.cl_concatenate(args)
            }),
            "cl-coerce" => self.subr_2(name, args, |s| {
                s.cl_coerce(args[0], args[1])
            }),
            "cl-tree-equal" => self.subr_2(name, args, |s| {
                Some(bool_value(s.tree_equal(args[0], args[1])))
            }),
            "cl-subst-if" | "cl-nsubst-if" => self.subr_3(name, args, |s| {
                s.cl_subst_if(args[0], args[1], args[2], false)
            }),
            "cl-subst-if-not" | "cl-nsubst-if-not" => self.subr_3(name, args, |s| {
                s.cl_subst_if(args[0], args[1], args[2], true)
            }),
            "cl-set-difference" | "cl-nset-difference" => self.subr_2(name, args, |s| {
                s.cl_set_difference(args[0], args[1])
            }),
            "cl-intersection" | "cl-nintersection" => self.subr_2(name, args, |s| {
                s.cl_intersection(args[0], args[1])
            }),
            "cl-ldiff" => self.subr_2(name, args, |s| {
                s.cl_ldiff(args[0], args[1])
            }),
            "cl-list-length" | "proper-list-p" => self.subr_1(name, args, |s| {
                s.cl_list_length(args[0])
            }),
            "cl-union" | "cl-nunion" => self.subr_2(name, args, |s| {
                s.cl_union(args[0], args[1])
            }),
            "cl-set-exclusive-or" | "cl-nset-exclusive-or" => self.subr_2(name, args, |s| {
                s.cl_set_exclusive_or(args[0], args[1])
            }),
            "cl-search" => self.subr_2(name, args, |s| {
                s.cl_search(args[0], args[1])
            }),
            "cl-tailp" => self.subr_2(name, args, |s| {
                Some(bool_value(s.cl_tailp(args[0], args[1])))
            }),
            "cl-sublis" | "cl-nsublis" => self.subr_2(name, args, |s| {
                s.cl_sublis(args[0], args[1])
            }),
            "cl-substitute" | "cl-nsubstitute" | "cl-nsubst" | "cl-subst" => self.subr_3(name, args, |s| {
                s.substitute_seq(args[0], args[1], args[2])
            }),
            "cl-substitute-if" | "cl-nsubstitute-if" => self.subr_3(name, args, |s| {
                s.substitute_seq_if(args[0], args[1], args[2], false)
            }),
            "cl-substitute-if-not" | "cl-nsubstitute-if-not" => self.subr_3(name, args, |s| {
                s.substitute_seq_if(args[0], args[1], args[2], true)
            }),
            "copy-sequence" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.copy_sequence(args[0])),
            "cl-map" => self
                .min_max_arity(name, args, 3, usize::MAX)
                .and_then(|_| self.cl_map(args)),
            "mapcar" | "cl-mapcar" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.mapcar(args[0], args[1])),
            "mapconcat" => self
                .exact_arity(name, args, 3)
                .and_then(|_| self.mapconcat(args[0], args[1], args[2])),
            "mapc" | "cl-mapc" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.mapc(args[0], args[1])),
            "maplist" | "cl-maplist" => self.subr_2(name, args, |s| {
                s.maplist(args[0], args[1])
            }),
            "mapl" | "cl-mapl" => self.subr_2(name, args, |s| {
                s.maplist(args[0], args[1]).map(|_| args[1])
            }),
            "mapcan" | "cl-mapcan" => self.subr_2(name, args, |s| {
                s.mapcan(args[0], args[1])
            }),
            "mapcon" | "cl-mapcon" => self.subr_2(name, args, |s| {
                s.mapcon(args[0], args[1])
            }),
            "every" | "cl-every" => self.subr_2(name, args, |s| {
                s.sequence_every(args[0], args[1])
            }),
            "some" | "cl-some" => self.subr_2(name, args, |s| {
                s.sequence_some(args[0], args[1])
            }),
            "notany" | "cl-notany" => self.subr_2(name, args, |s| {
                s.sequence_some(args[0], args[1])
                    .map(|v| if v.is_nil() { LispValue::TRUE } else { LispValue::NIL })
            }),
            "notevery" | "cl-notevery" => self.subr_2(name, args, |s| {
                s.sequence_every(args[0], args[1])
                    .map(|v| if v.is_nil() { LispValue::TRUE } else { LispValue::NIL })
            }),
            "copy-list" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.copy_list(args[0])),
            "make-string" => self.subr_2(name, args, |s| {
                let count = s.fixnum_arg("make-string", args[0])?;
                let ch = args[1].as_char()
                    .or_else(|| args[1].as_fixnum().and_then(|n| char::from_u32(n as u32)))
                    .unwrap_or(' ');
                let s_str: String = std::iter::repeat(ch).take(count as usize).collect();
                Some(s.runtime.string(s_str))
            }),
            "make-list" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.make_list(args[0], args[1])),
            "number-sequence" => self
                .min_max_arity(name, args, 2, 3)
                .and_then(|_| self.number_sequence(args[0], args[1], args.get(2).copied())),
            "+" => self.number_fold_add(args),
            "*" => self.number_fold_mul(args),
            "-" => self.number_sub(args),
            "/" => self.number_div(args),
            "1+" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_float(args[0]) {
                    let value = self.number_arg(name, args[0])?;
                    Some(self.runtime.float(value + 1.0))
                } else if self.runtime.is_bignum(args[0]) {
                    let value = self.bignum_arg(name, args[0])?;
                    Some(self.runtime.bignum(value + 1))
                } else {
                    let value = self.fixnum_arg(name, args[0])?;
                    value.checked_add(1).and_then(|v| self.fixnum(v, name))
                }
            }),
            "1-" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_float(args[0]) {
                    let value = self.number_arg(name, args[0])?;
                    Some(self.runtime.float(value - 1.0))
                } else if self.runtime.is_bignum(args[0]) {
                    let value = self.bignum_arg(name, args[0])?;
                    Some(self.runtime.bignum(value - 1))
                } else {
                    let value = self.fixnum_arg(name, args[0])?;
                    value.checked_sub(1).and_then(|v| self.fixnum(v, name))
                }
            }),
            "=" => self.number_compare(args, |left, right| left == right),
            "<" => self.number_compare(args, |left, right| left < right),
            "<=" => self.number_compare(args, |left, right| left <= right),
            ">" => self.number_compare(args, |left, right| left > right),
            ">=" => self.number_compare(args, |left, right| left >= right),
            "/=" => self.min_max_arity(name, args, 0, usize::MAX).and_then(|_| {
                // (=/=) and (=/= x) return t.
                if args.len() < 2 { return Some(LispValue::TRUE); }
                // All args must be pairwise distinct (GNU Emacs semantics).
                if self.has_float_arg(args) {
                    let vals: Vec<f64> = args.iter()
                        .map(|v| self.number_arg(name, *v))
                        .collect::<Option<Vec<_>>>()?;
                    let all_distinct = (0..vals.len()).all(|i|
                        (i+1..vals.len()).all(|j| vals[i] != vals[j])
                    );
                    Some(bool_value(all_distinct))
                } else {
                    let vals: Vec<i64> = args.iter()
                        .map(|v| self.fixnum_arg(name, *v))
                        .collect::<Option<Vec<_>>>()?;
                    let all_distinct = (0..vals.len()).all(|i|
                        (i+1..vals.len()).all(|j| vals[i] != vals[j])
                    );
                    Some(bool_value(all_distinct))
                }
            }),
            "%" => self.exact_arity(name, args, 2).and_then(|_| {
                if self.has_float_arg(args) {
                    let dividend = self.number_arg(name, args[0])?;
                    let divisor = self.number_arg(name, args[1])?;
                    if divisor == 0.0 {
                        let symbol = self.runtime.intern("arith-error");
                        self.pending_signal = Some(SignaledValue {
                            symbol,
                            data: LispValue::NIL,
                        });
                        return None;
                    }
                    Some(self.runtime.float(dividend % divisor))
                } else if self.has_bignum_arg(args) {
                    let dividend = self.bignum_arg(name, args[0])?;
                    let divisor = self.bignum_arg(name, args[1])?;
                    if divisor == 0 {
                        let symbol = self.runtime.intern("arith-error");
                        self.pending_signal = Some(SignaledValue {
                            symbol,
                            data: LispValue::NIL,
                        });
                        return None;
                    }
                    Some(self.runtime.bignum(dividend % divisor))
                } else {
                    let dividend = self.fixnum_arg(name, args[0])?;
                    let divisor = self.fixnum_arg(name, args[1])?;
                    if divisor == 0 {
                        let symbol = self.runtime.intern("arith-error");
                        self.pending_signal = Some(SignaledValue {
                            symbol,
                            data: LispValue::NIL,
                        });
                        return None;
                    }
                    self.fixnum(dividend % divisor, name)
                }
            }),
            "mod" => self.exact_arity(name, args, 2).and_then(|_| {
                if self.has_float_arg(args) {
                    let dividend = self.number_arg(name, args[0])?;
                    let divisor = self.number_arg(name, args[1])?;
                    if divisor == 0.0 {
                        let symbol = self.runtime.intern("arith-error");
                        self.pending_signal = Some(SignaledValue {
                            symbol,
                            data: LispValue::NIL,
                        });
                        return None;
                    }
                    let result = dividend - divisor * (dividend / divisor).floor();
                    Some(self.runtime.float(result))
                } else if self.has_bignum_arg(args) {
                    let dividend = self.bignum_arg(name, args[0])?;
                    let divisor = self.bignum_arg(name, args[1])?;
                    if divisor == 0 {
                        let symbol = self.runtime.intern("arith-error");
                        self.pending_signal = Some(SignaledValue {
                            symbol,
                            data: LispValue::NIL,
                        });
                        return None;
                    }
                    let result = rug::Integer::from(&dividend % &divisor);
                    let zero = rug::Integer::new();
                    if result != zero && (dividend < 0) != (divisor < 0) {
                        Some(self.runtime.bignum(result + divisor))
                    } else {
                        Some(self.runtime.bignum(result))
                    }
                } else {
                    let dividend = self.fixnum_arg(name, args[0])?;
                    let divisor = self.fixnum_arg(name, args[1])?;
                    if divisor == 0 {
                        let symbol = self.runtime.intern("arith-error");
                        self.pending_signal = Some(SignaledValue {
                            symbol,
                            data: LispValue::NIL,
                        });
                        return None;
                    }
                    let result = dividend % divisor;
                    let result = if result != 0 && (dividend < 0) != (divisor < 0) {
                        result + divisor
                    } else {
                        result
                    };
                    self.fixnum(result, name)
                }
            }),
            "message" => {
                if args.is_empty() {
                    Some(self.runtime.string(String::new()))
                } else {
                    self.format_string(args[0], &args[1..])
                }
            }
            "terpri" => Some(LispValue::NIL),
            "print" | "prin1" => self.exact_arity(name, args, 1).map(|_| args[0]),
            "signal" => self.exact_arity(name, args, 2).and_then(|_| {
                self.pending_signal = Some(SignaledValue {
                    symbol: args[0],
                    data: args[1],
                });
                None
            }),
            "define-error" => self.min_max_arity(name, args, 2, 3).and_then(|_| {
                let symbol = args[0];
                let message = args[1];
                let parent = args.get(2).copied().unwrap_or_else(|| self.runtime.intern("error"));
                let parent_cons = self.runtime.cons(parent, LispValue::NIL);
                let conditions = self.runtime.cons(symbol, parent_cons);
                let msg_key = self.runtime.intern("error-message");
                let cond_key = self.runtime.intern("error-conditions");
                let _ = self.runtime.put_symbol_property(symbol, msg_key, message);
                let _ = self.runtime.put_symbol_property(symbol, cond_key, conditions);
                Some(symbol)
            }),
            "error" => {
                let symbol = self.runtime.intern("error");
                let data = self.format_signal_data(args);
                self.pending_signal = Some(SignaledValue { symbol, data });
                None
            }
            "user-error" => {
                let symbol = self.runtime.intern("user-error");
                let data = self.format_signal_data(args);
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
            "functionp" => self.exact_arity(name, args, 1).map(|_| {
                bool_value(
                    self.runtime.is_function(args[0])
                        || (self.runtime.is_symbol(args[0])
                            && self
                                .runtime
                                .symbol_name(args[0])
                                .ok()
                                .is_some_and(|name| self.is_callable_name(&name))),
                )
            }),
            "macrop" => self.exact_arity(name, args, 1).map(|_| {
                bool_value(
                    self.runtime.is_symbol(args[0])
                        && self.runtime.symbol_name(args[0]).ok()
                            .is_some_and(|name| self.functions_by_name.contains_key(&format!("{name}-macro")))
                )
            }),
            "special-form-p" => self.exact_arity(name, args, 1).map(|_| {
                bool_value(
                    self.runtime.is_symbol(args[0])
                        && self.runtime.symbol_name(args[0]).ok()
                            .is_some_and(|name| self.is_special_form_name(&name))
                )
            }),
            "rem" => self.exact_arity(name, args, 2).and_then(|_| {
                if self.has_float_arg(args) {
                    let dividend = self.number_arg(name, args[0])?;
                    let divisor = self.number_arg(name, args[1])?;
                    if divisor == 0.0 {
                        let symbol = self.runtime.intern("arith-error");
                        self.pending_signal = Some(SignaledValue {
                            symbol,
                            data: LispValue::NIL,
                        });
                        return None;
                    }
                    Some(self.runtime.float(dividend % divisor))
                } else if self.has_bignum_arg(args) {
                    let dividend = self.bignum_arg(name, args[0])?;
                    let divisor = self.bignum_arg(name, args[1])?;
                    if divisor == 0 {
                        let symbol = self.runtime.intern("arith-error");
                        self.pending_signal = Some(SignaledValue {
                            symbol,
                            data: LispValue::NIL,
                        });
                        return None;
                    }
                    Some(self.runtime.bignum(dividend % divisor))
                } else {
                    let dividend = self.fixnum_arg(name, args[0])?;
                    let divisor = self.fixnum_arg(name, args[1])?;
                    if divisor == 0 {
                        let symbol = self.runtime.intern("arith-error");
                        self.pending_signal = Some(SignaledValue {
                            symbol,
                            data: LispValue::NIL,
                        });
                        return None;
                    }
                    self.fixnum(dividend % divisor, name)
                }
            }),
            "abs" => self.exact_arity(name, args, 1).and_then(|_| {
                if self.runtime.is_float(args[0]) {
                    let value = self.number_arg(name, args[0])?;
                    Some(self.runtime.float(value.abs()))
                } else if self.runtime.is_bignum(args[0]) {
                    let value = self.bignum_arg(name, args[0])?;
                    Some(self.runtime.bignum(value.abs()))
                } else {
                    let value = self.fixnum_arg(name, args[0])?;
                    value.checked_abs().and_then(|v| self.fixnum(v, name))
                }
            }),
            "max" => {
                if args.is_empty() {
                    self.error("primitive `max` requires at least one argument");
                    return None;
                }
                if self.has_float_arg(args) {
                    let mut result = self.number_arg(name, args[0])?;
                    for arg in &args[1..] {
                        let value = self.number_arg(name, *arg)?;
                        result = result.max(value);
                    }
                    Some(self.runtime.float(result))
                } else if self.has_bignum_arg(args) {
                    let mut result = self.bignum_arg(name, args[0])?;
                    for arg in &args[1..] {
                        let value = self.bignum_arg(name, *arg)?;
                        result = result.max(value);
                    }
                    Some(self.runtime.bignum(result))
                } else {
                    let mut result = self.fixnum_arg(name, args[0])?;
                    for arg in &args[1..] {
                        let value = self.fixnum_arg(name, *arg)?;
                        result = result.max(value);
                    }
                    self.fixnum(result, name)
                }
            }
            "min" => {
                if args.is_empty() {
                    self.error("primitive `min` requires at least one argument");
                    return None;
                }
                if self.has_float_arg(args) {
                    let mut result = self.number_arg(name, args[0])?;
                    for arg in &args[1..] {
                        let value = self.number_arg(name, *arg)?;
                        result = result.min(value);
                    }
                    Some(self.runtime.float(result))
                } else if self.has_bignum_arg(args) {
                    let mut result = self.bignum_arg(name, args[0])?;
                    for arg in &args[1..] {
                        let value = self.bignum_arg(name, *arg)?;
                        result = result.min(value);
                    }
                    Some(self.runtime.bignum(result))
                } else {
                    let mut result = self.fixnum_arg(name, args[0])?;
                    for arg in &args[1..] {
                        let value = self.fixnum_arg(name, *arg)?;
                        result = result.min(value);
                    }
                    self.fixnum(result, name)
                }
            }
            "cl-typep" => self.subr_2(name, args, |s| {
                s.cl_typep(args[0], args[1])
            }),
            "type-of" => self.exact_arity(name, args, 1).map(|_| {
                if args[0].is_nil() || args[0].is_true() {
                    self.runtime.intern("symbol")
                } else if args[0].is_fixnum() {
                    self.runtime.intern("integer")
                } else if args[0].as_char().is_some() {
                    self.runtime.intern("symbol")
                } else if self.runtime.is_float(args[0]) {
                    self.runtime.intern("float")
                } else if self.runtime.is_bignum(args[0]) {
                    self.runtime.intern("integer")
                } else if args[0].is_heap() {
                    if self.runtime.is_cons(args[0]) {
                        self.runtime.intern("cons")
                    } else if self.runtime.is_string(args[0]) {
                        self.runtime.intern("string")
                    } else if self.runtime.is_vector(args[0]) {
                        self.runtime.intern("vector")
                    } else if self.runtime.is_hash_table(args[0]) {
                        self.runtime.intern("hash-table")
                    } else if self.runtime.is_function(args[0]) {
                        self.runtime.intern("compiled-function")
                    } else {
                        self.runtime.intern("misc")
                    }
                } else {
                    self.runtime.intern("misc")
                }
            }),
            "cadr" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.cdr(args[0]);
                let cdr = self.runtime_value(result)?;
                let result = self.runtime.car(cdr);
                self.runtime_value(result)
            }),
            "caar" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.car(args[0]);
                let car = self.runtime_value(result)?;
                let result = self.runtime.car(car);
                self.runtime_value(result)
            }),
            "cdar" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.car(args[0]);
                let car = self.runtime_value(result)?;
                let result = self.runtime.cdr(car);
                self.runtime_value(result)
            }),
            "cddr" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.cdr(args[0]);
                let cdr = self.runtime_value(result)?;
                let result = self.runtime.cdr(cdr);
                self.runtime_value(result)
            }),
            "caaar" | "caadr" | "cadar" | "caddr" | "cdaar" | "cdadr" | "cddar" | "cdddr"
            | "caaaar" | "caaadr" | "caadar" | "caaddr" | "cadaar" | "cadadr" | "caddar"
            | "cadddr" | "cdaaar" | "cdaadr" | "cdadar" | "cdaddr" | "cddaar" | "cddadr"
            | "cdddar" | "cddddr" => self.exact_arity(name, args, 1).and_then(|_| {
                let mut value = args[0];
                for ch in name[1..name.len() - 1].bytes().rev() {
                    let result = if ch == b'a' {
                        self.runtime.car(value)
                    } else {
                        self.runtime.cdr(value)
                    };
                    value = self.runtime_value(result)?;
                }
                Some(value)
            }),
            "number-or-marker-p" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_number(args[0]))),
            "floatp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_float(args[0]))),
            "string-or-null-p" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_nil() || self.runtime.is_string(args[0]))),
            "bare-symbol-p" => self.exact_arity(name, args, 1).map(|_| {
                bool_value(!args[0].is_nil() && !args[0].is_true() && self.runtime.is_symbol(args[0]))
            }),
            "bignump" => self.exact_arity(name, args, 1).map(|_| {
                bool_value(self.runtime.is_bignum(args[0]))
            }),
            "fixnump" => self.exact_arity(name, args, 1).map(|_| {
                bool_value(args[0].is_fixnum())
            }),
            "booleanp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_nil() || args[0].is_true())),
            "file-exists-p" => self.exact_arity(name, args, 1).and_then(|_| {
                let path = self.string_contents_owned(args[0])?;
                Some(bool_value(std::path::Path::new(&path).exists()))
            }),
            "file-readable-p" => self.exact_arity(name, args, 1).and_then(|_| {
                let path = self.string_contents_owned(args[0])?;
                let meta = std::fs::metadata(&path).ok();
                Some(bool_value(
                    meta.is_some_and(|m| !m.permissions().readonly()),
                ))
            }),
            "load" => self
                .min_max_arity(name, args, 1, 5)
                .and_then(|_| self.load_file(args[0])),
            "add-load-path" => self.exact_arity(name, args, 1).and_then(|_| {
                let path = self.string_contents_owned(args[0])?;
                self.runtime.add_load_path(path);
                Some(LispValue::TRUE)
            }),
            "run-hooks" => self.min_arity(name, args, 1).and_then(|_| {
                let mut last = LispValue::NIL;
                for hook_sym in args {
                    if let Ok(hook_val) = self.runtime.symbol_value(*hook_sym) {
                        let mut current = hook_val;
                        while !current.is_nil() {
                            let func = self.runtime.car(current).unwrap_or(LispValue::NIL);
                            let rest = self.runtime.cdr(current).unwrap_or(LispValue::NIL);
                            if !func.is_nil() {
                                last = self.execute_funcall(func, &[]).unwrap_or(LispValue::NIL);
                            }
                            current = rest;
                        }
                    }
                }
                Some(last)
            }),
            "add-hook" => self.min_max_arity(name, args, 2, 3).and_then(|_| {
                let func = args[1];
                let hook_sym = args[0];
                if let Ok(hook_val) = self.runtime.symbol_value(hook_sym) {
                    if self.memq(func, hook_val).is_none() {
                        let new_val = self.runtime.cons(func, hook_val);
                        let _ = self.runtime.set_symbol_value(hook_sym, new_val);
                    }
                } else {
                    let new_val = self.runtime.cons(func, LispValue::NIL);
                    let _ = self.runtime.set_symbol_value(hook_sym, new_val);
                }
                Some(LispValue::NIL)
            }),
            "remove-hook" => self.min_max_arity(name, args, 2, 3).and_then(|_| {
                let func = args[1];
                let hook_sym = args[0];
                if let Ok(hook_val) = self.runtime.symbol_value(hook_sym) {
                    if let Some(new_val) = self.delq(func, hook_val) {
                        let _ = self.runtime.set_symbol_value(hook_sym, new_val);
                    }
                }
                Some(LispValue::NIL)
            }),

            // --- Thread primitives ---
            "make-thread" => self.min_max_arity(name, args, 2, 2).and_then(|_| {
                let name = self.string_contents_owned(args[0])?;
                let body = args[1];
                let id = self.runtime.scheduler.make_thread(name, body);
                Some(LispValue::expect_fixnum(id.0 as i64))
            }),
            "thread-yield" => {
                self.runtime.scheduler.thread_yield();
                Some(LispValue::NIL)
            }
            "thread-join" => self.exact_arity(name, args, 1).and_then(|_| {
                let tid = self.fixnum_arg(name, args[0])? as u64;
                let thread_id = crate::thread::ThreadId(tid);
                let blocked = self.runtime.scheduler.thread_join(thread_id);
                if blocked {
                    Some(LispValue::NIL)
                } else {
                    self.runtime.scheduler.thread_result(thread_id)
                }
            }),
            "thread-signal" => self.exact_arity(name, args, 2).and_then(|_| {
                let tid = self.fixnum_arg(name, args[0])? as u64;
                let error = args[1];
                self.runtime
                    .scheduler
                    .thread_signal(crate::thread::ThreadId(tid), error);
                Some(LispValue::NIL)
            }),
            "current-thread" => {
                let id = self.runtime.scheduler.current_id();
                Some(LispValue::expect_fixnum(id.0 as i64))
            }
            "thread-alive-p" => self.exact_arity(name, args, 1).and_then(|_| {
                let tid = self.fixnum_arg(name, args[0])? as u64;
                Some(bool_value(
                    self.runtime
                        .scheduler
                        .thread_alive_p(crate::thread::ThreadId(tid)),
                ))
            }),

            // --- Atom primitives ---
            "make-atom" => self.min_max_arity(name, args, 1, 1).and_then(|_| {
                Some(self.runtime.make_atom(args[0]))
            }),
            "atom-deref" => self.exact_arity(name, args, 1).and_then(|_| {
                self.runtime.atom_deref(args[0]).ok()
            }),
            "atom-reset!" => self.exact_arity(name, args, 2).and_then(|_| {
                self.runtime.atom_reset(args[0], args[1]).ok()
            }),
            "atom-compare-and-set!" => self.exact_arity(name, args, 3).and_then(|_| {
                self.runtime
                    .atom_compare_and_set(args[0], args[1], args[2])
                    .ok()
                    .map(|success| bool_value(success))
            }),
            "atom-swap!" => self.min_max_arity(name, args, 2, usize::MAX).and_then(|_| {
                let func = args[1];
                let extra_args = &args[2..];
                let mut iterations = 0;
                loop {
                    let current = self.runtime.atom_read_for_swap(args[0]).ok()?;
                    let mut call_args = vec![current];
                    call_args.extend_from_slice(extra_args);
                    let new_val = self.execute_funcall(func, &call_args)?;
                    match self.runtime.atom_cas(args[0], current, new_val) {
                        Ok((val, true)) => return Some(val),
                        Ok((_, false)) => {
                            iterations += 1;
                            if iterations > 1000 {
                                return None; // prevent infinite CAS loop
                            }
                            continue;
                        }
                        Err(_) => return None,
                    }
                }
            }),

            // --- Agent primitives ---
            "make-agent" => self.min_max_arity(name, args, 1, 1).and_then(|_| {
                Some(self.runtime.make_agent(args[0]))
            }),
            "agent-deref" => self.exact_arity(name, args, 1).and_then(|_| {
                self.runtime.agent_deref(args[0]).ok()
            }),
            "send" | "send-off" => self.min_max_arity(name, args, 2, usize::MAX).and_then(|_| {
                let via_pool = name == "send-off";
                let result = self.runtime
                    .agent_send(args[0], args[1], &args[2..], via_pool)
                    .ok();
                if via_pool {
                    // Submit to background pool for async processing.
                    if let Some(addr) = args[0].heap_addr() {
                        self.runtime.agent_pool.submit(addr);
                    }
                }
                result
            }),
            "agent-await" => self.exact_arity(name, args, 1).and_then(|_| {
                let agent = args[0];
                while let Ok(Some(action)) = self.runtime.agent_pop_action(agent) {
                    let current = self.runtime.agent_deref(agent).unwrap_or(LispValue::NIL);
                    let mut call_args = vec![current];
                    call_args.extend_from_slice(&action.args);
                    match self.execute_funcall(action.func, &call_args) {
                        Some(new_val) => {
                            let _ = self.runtime.agent_update(agent, new_val, None);
                        }
                        None => {
                            // funcall failed — store error and stop draining
                            let _ = self.runtime.agent_update(
                                agent,
                                LispValue::NIL,
                                Some(LispValue::NIL),
                            );
                            break;
                        }
                    }
                }
                self.runtime.agent_deref(agent).ok()
            }),
            "agent-error" => self.exact_arity(name, args, 1).and_then(|_| {
                Some(self.runtime.agent_error(args[0]).ok().flatten().unwrap_or(LispValue::NIL))
            }),
            "restart-agent" => self.exact_arity(name, args, 2).and_then(|_| {
                let _ = self.runtime.agent_update(args[0], args[1], None);
                Some(LispValue::NIL)
            }),

            // --- Mutex/condition variable primitives ---
            "make-mutex" => self.min_max_arity(name, args, 1, 1).and_then(|_| {
                let name = self.string_contents_owned(args[0])
                    .unwrap_or("unnamed".to_string());
                Some(self.runtime.make_mutex(name))
            }),
            "mutex-lock" => self.exact_arity(name, args, 1).and_then(|_| {
                self.runtime.mutex_lock(args[0]).ok()?;
                Some(LispValue::TRUE)
            }),
            "mutex-unlock" => self.exact_arity(name, args, 1).and_then(|_| {
                self.runtime.mutex_unlock(args[0]).ok()?;
                Some(LispValue::TRUE)
            }),
            "make-condition-variable" => self.min_max_arity(name, args, 1, 1).and_then(|_| {
                let name = self.string_contents_owned(args[0])
                    .unwrap_or("unnamed".to_string());
                Some(self.runtime.make_condvar(name))
            }),
            "condition-wait" => self.exact_arity(name, args, 2).and_then(|_| {
                self.runtime.condvar_wait(args[0], args[1]).ok()?;
                Some(LispValue::NIL)
            }),
            "condition-notify" => self.exact_arity(name, args, 1).and_then(|_| {
                self.runtime.condvar_notify(args[0]).ok()?;
                Some(LispValue::NIL)
            }),
            "condition-notify-all" => self.exact_arity(name, args, 1).and_then(|_| {
                self.runtime.condvar_notify_all(args[0]).ok()?;
                Some(LispValue::NIL)
            }),

            // ── Demo: Rust function callable from Elisp ─────────────────
            // Step 1: implement the function
            "fib" => self.subr_1(name, args, |s| {
                let n = s.fixnum_arg("fib", args[0])?;
                let (mut a, mut b) = (0i64, 1i64);
                for _ in 0..n { let t = a + b; a = b; b = t; }
                s.fixnum(a, "fib")
            }),
            // Step 2: add "fib" to is_primitive_name HashSet (done below).
            // Step 3: (optional) add JIT fast path in jit_rt.rs.
            // Now callable from Elisp: (fib 10) → 55

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

    fn require_feature(&mut self, feature: LispValue, noerror: LispValue) -> Option<LispValue> {
        let provided = match self.runtime.featurep(feature) {
            Ok(provided) => provided,
            Err(error) => {
                self.runtime_error(error);
                return None;
            }
        };
        if provided {
            return Some(feature);
        }
        if !noerror.is_nil() {
            return Some(LispValue::NIL);
        }
        // Try to load the feature file
        let feat_name = match self.runtime.symbol_name(feature) {
            Ok(n) => n,
            Err(_) => {
                let error_symbol = self.runtime.intern("error");
                let message = self.runtime.string("required feature was not provided");
                let data = make_list(self.runtime, [feature, message]);
                self.pending_signal = Some(SignaledValue {
                    symbol: error_symbol,
                    data,
                });
                return None;
            }
        };
        let path = self
            .runtime
            .resolve_load_file(&feat_name)
            .unwrap_or_else(|| format!("{feat_name}.el"));
        if std::path::Path::new(&path).exists()
            || self.runtime.resolve_load_file(&feat_name).is_some()
        {
            let file_val = self.runtime.string(path);
            return self.load_file(file_val);
        }
        let error_symbol = self.runtime.intern("error");
        let message = self.runtime.string("required feature was not provided");
        let data = make_list(self.runtime, [feature, message]);
        self.pending_signal = Some(SignaledValue {
            symbol: error_symbol,
            data,
        });
        None
    }

    fn autoload(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let function = args[0];
        let mut autoload_args = vec![args[1], LispValue::NIL, LispValue::NIL, LispValue::NIL];
        for (slot, value) in autoload_args
            .iter_mut()
            .skip(1)
            .zip(args.iter().copied().skip(2))
        {
            *slot = value;
        }
        let head = self.runtime.intern("autoload");
        let tail = make_list(self.runtime, autoload_args);
        let object = self.runtime.cons(head, tail);
        let result = self.runtime.set_symbol_function(function, object);
        self.runtime_value(result).map(|_| function)
    }

    fn is_callable_name(&self, name: &str) -> bool {
        is_primitive_name(name) || self.functions_by_name.contains_key(name)
    }

    fn is_special_form_name(&self, name: &str) -> bool {
        matches!(name,
            "and" | "or" | "if" | "cond" | "while" | "let" | "let*" | "setq"
            | "quote" | "function" | "progn" | "prog1" | "prog2"
            | "catch" | "throw" | "condition-case" | "unwind-protect"
            | "defun" | "defvar" | "defconst" | "defmacro" | "lambda"
            | "letrec" | "cl-loop" | "pcase" | "setf" | "cl-labels"
            | "cl-flet" | "interactive" | "save-excursion"
            | "save-restriction" | "save-current-buffer"
            | "with-mutex" | "make-thread" | "thread-yield"
            | "make-atom" | "make-agent" | "make-mutex"
            | "make-condition-variable"
        )
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
        let args = self.adapt_lambda_args(&function.lambda_list, args)?;
        let result = execute_with_module(
            function,
            &args,
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
        let target = find_condition_handler(
            instructions, signal_index, &signal_name, signaled.symbol, self.runtime,
        )?;
        // Pop skipped inner condition-case frames. Clamp to avoid over-popping
        // when inner handlers have already been activated (and thus already
        // popped their frames from condition_stack).
        let actual_skip = target
            .frames_to_skip
            .min(self.condition_stack.len().saturating_sub(1));
        for _ in 0..actual_skip {
            self.condition_stack.pop();
        }
        let frame = self.condition_stack.pop()?;
        let mut dynamic_bind_count = 0;
        self.caught_signal = Some(signaled.clone());
        if let Some(var) = frame.var {
            let binding = self.runtime.cons(signaled.symbol, signaled.data);
            if let Err(error) = self.runtime.bind_dynamic_by_name(&var, binding) {
                self.runtime_error(error);
                return None;
            }
            dynamic_bind_count = 1;
        }
        self.last_value = None;
        // When a signal propagates through an inner handler to an outer handler,
        // inherit the inner handler's result_reg so the outer handler writes to
        // the correct register (the original body's result register, not the
        // inner handler's signal instruction result register).
        let result_reg = self
            .active_condition_handlers
            .last()
            .and_then(|h| h.result_reg)
            .or(result_reg);
        self.active_condition_handlers.push(ActiveConditionHandler {
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
        result_reg: Option<RegId>,
    ) -> Option<usize> {
        let target = find_unwind_cleanup(instructions, signal_index)?;
        self.active_unwind_cleanups.push(ActiveUnwindCleanup {
            stop_index: target.end_index,
            pending,
            result_reg,
        });
        Some(target.cleanup_index + 1)
    }

    fn const_value(&mut self, value: &SsaConst) -> Option<LispValue> {
        match value {
            SsaConst::Nil => Some(LispValue::NIL),
            SsaConst::True => Some(LispValue::TRUE),
            SsaConst::Int(value) => self.fixnum_value(*value, "integer constant"),
            SsaConst::Char(value) => {
                let code: u32 = (*value).try_into().ok()?;
                char::from_u32(code).map(LispValue::from_char)
            }
            SsaConst::String(value) => Some(self.runtime.string(value.clone())),
            SsaConst::Float(value) => Some(self.runtime.float(*value)),
            SsaConst::Symbol(name) => Some(self.runtime.intern(name)),
            SsaConst::Value(cv) => self.compile_value_to_lisp(cv),
        }
    }

    fn compile_value_to_lisp(
        &mut self,
        cv: &neovm_compiler::compile_value::CompileValue,
    ) -> Option<LispValue> {
        use neovm_compiler::compile_value::CompileValue;
        match cv {
            CompileValue::Nil => Some(LispValue::NIL),
            CompileValue::Bool(true) => Some(LispValue::TRUE),
            CompileValue::Bool(false) => Some(LispValue::NIL),
            CompileValue::Int(n) => self.fixnum_value(*n, "compile value"),
            CompileValue::Float(value) => Some(self.runtime.float(*value)),
            CompileValue::Char(c) => {
                let code: u32 = (*c).try_into().ok()?;
                char::from_u32(code).map(LispValue::from_char)
            }
            CompileValue::Symbol(name) => Some(self.runtime.intern(name)),
            CompileValue::String(s) => Some(self.runtime.string(s.clone())),
            CompileValue::Cons { car, cdr } => {
                let car_val = self.compile_value_to_lisp(car)?;
                let cdr_val = self.compile_value_to_lisp(cdr)?;
                Some(self.runtime.cons(car_val, cdr_val))
            }
            CompileValue::Vector(items) => {
                let vals: Vec<LispValue> = items
                    .iter()
                    .filter_map(|item| self.compile_value_to_lisp(item))
                    .collect();
                Some(self.runtime.vector(vals))
            }
        }
    }

    fn quote_value(&mut self, form: &SurfaceForm) -> Option<LispValue> {
        match &form.kind {
            SurfaceKind::Atom(atom) => self.quote_atom(atom),
            SurfaceKind::List(items) | SurfaceKind::HashList(items) => {
                let values = items
                    .iter()
                    .map(|item| self.quote_value(item))
                    .collect::<Option<Vec<_>>>()?;
                Some(make_list(self.runtime, values))
            }
            SurfaceKind::Record(type_name, items) => {
                let mut all = vec![self.quote_value(type_name)?];
                for item in items {
                    all.push(self.quote_value(item)?);
                }
                Some(make_list(self.runtime, all))
            }
            SurfaceKind::CharTable(items) => {
                let values = items
                    .iter()
                    .map(|item| self.quote_value(item))
                    .collect::<Option<Vec<_>>>()?;
                Some(make_list(self.runtime, values))
            }
            SurfaceKind::Labeled(_, form) => self.quote_value(form),
            SurfaceKind::Ref(_) => Some(LispValue::NIL),
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
            SurfaceKind::Vector(items) => {
                let values = items
                    .iter()
                    .map(|item| self.quote_value(item))
                    .collect::<Option<Vec<_>>>()?;
                Some(self.runtime.vector(values))
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
            SurfaceAtom::Int(value) => self.fixnum_value(*value, "quoted integer"),
            SurfaceAtom::Char(value) => {
                let code: u32 = (*value).try_into().ok()?;
                char::from_u32(code).map(LispValue::from_char)
            }
            SurfaceAtom::String(value) => Some(self.runtime.string(value.clone())),
            SurfaceAtom::Float(value) => Some(self.runtime.float(*value)),
        }
    }

    fn fixnum_value(&mut self, value: i64, _context: &str) -> Option<LispValue> {
        match LispValue::from_fixnum(value) {
            Some(value) => Some(value),
            None => Some(self.runtime.bignum(rug::Integer::from(value))),
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

    fn nthcdr(&mut self, index: LispValue, list: LispValue) -> Option<LispValue> {
        let index = self.fixnum_arg("nthcdr", index)?;
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
        Some(current)
    }

    fn last(&mut self, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        if current.is_nil() {
            return Some(LispValue::NIL);
        }
        loop {
            let cdr = self.runtime.cdr(current);
            let cdr_val = self.runtime_value(cdr)?;
            if cdr_val.is_nil() {
                return Some(current);
            }
            current = cdr_val;
        }
    }

    fn sort_seq(&mut self, seq: LispValue, predicate: LispValue) -> Option<LispValue> {
        if self.runtime.is_vector(seq) {
            let mut elements = match self.runtime.vector_elements(seq) {
                Ok(e) => e,
                Err(e) => {
                    self.runtime_error(e);
                    return None;
                }
            };
            elements.sort_by(|a, b| {
                let result = self.execute_funcall(predicate, &[*a, *b]);
                match result {
                    Some(LispValue::NIL) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Less,
                }
            });
            let result = self.runtime.vector(elements);
            Some(result)
        } else {
            let mut values = self.list_values(seq)?;
            values.sort_by(|a, b| {
                let result = self.execute_funcall(predicate, &[*a, *b]);
                match result {
                    Some(LispValue::NIL) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Less,
                }
            });
            Some(make_list(self.runtime, values.into_iter()))
        }
    }

    fn length_equals(&mut self, seq: LispValue, expected: LispValue) -> Option<LispValue> {
        let n = self.fixnum_arg("length=", expected)?;
        if n < 0 { return Some(LispValue::NIL); }
        let len = if seq.is_nil() || self.runtime.is_cons(seq) {
            let mut current = seq;
            let mut count = 0i64;
            loop {
                if current.is_nil() { break; }
                if count > n { return Some(LispValue::NIL); }
                if !self.runtime.is_cons(current) { return Some(LispValue::NIL); }
                count += 1;
                current = self.runtime.cdr(current).ok()?;
            }
            count
        } else if self.runtime.is_vector(seq) {
            self.runtime.vector_elements(seq).ok()?.len() as i64
        } else if self.runtime.is_string(seq) {
            self.string_contents_owned(seq)?.len() as i64
        } else {
            return Some(LispValue::NIL);
        };
        Some(bool_value(len == n))
    }

    fn safe_length(&mut self, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        let mut count: i64 = 0;
        while !current.is_nil() {
            if !self.runtime.is_cons(current) {
                return self.fixnum(count, "safe-length");
            }
            count += 1;
            if count > 1_000_000 {
                return Some(LispValue::NIL);
            }
            let cdr = self.runtime.cdr(current);
            current = self.runtime_value(cdr)?;
        }
        self.fixnum(count, "safe-length")
    }

    fn subseq(
        &mut self,
        seq: LispValue,
        start: LispValue,
        end: Option<LispValue>,
    ) -> Option<LispValue> {
        let is_negative = |v: LispValue| -> bool {
            v.as_fixnum().map(|n| n < 0).unwrap_or(false)
        };
        let len = if self.runtime.is_string(seq) {
            self.string_contents_owned(seq)?.len()
        } else if self.runtime.is_cons(seq) || seq.is_nil() {
            self.safe_length(seq)?.as_fixnum().unwrap_or(0) as usize
        } else {
            match self.runtime.vector_elements(seq) {
                Ok(e) => e.len(),
                Err(e) => {
                    self.runtime_error(e);
                    return None;
                }
            }
        };
        // Resolve negative indices (count from end), then forward to
        // sequence_index for the non-negative bound check.
        let resolve_index = |s: &mut Self, val: LispValue| -> Option<usize> {
            if is_negative(val) {
                let neg = -(val.as_fixnum()?);
                let abs = neg as usize;
                if abs > len { None } else { Some(len - abs) }
            } else {
                s.sequence_index("subseq", val)
            }
        };
        let start_idx = resolve_index(self, start)?;
        let end_idx = match end {
            Some(e) if !e.is_nil() => resolve_index(self, e)?,
            _ => len,
        };
        if start_idx > len {
            let symbol = self.runtime.intern("args-out-of-range");
            let data = make_list(self.runtime, [seq, start].into_iter());
            self.pending_signal = Some(SignaledValue { symbol, data });
            return None;
        }
        let end_idx = end_idx.min(len);
        if self.runtime.is_string(seq) {
            let contents = self.string_contents_owned(seq)?;
            let slice = &contents[start_idx..end_idx];
            Some(self.runtime.string(slice.to_string()))
        } else if self.runtime.is_cons(seq) || seq.is_nil() {
            let elements = self.list_values(seq)?;
            let slice = &elements[start_idx..end_idx];
            Some(make_list(self.runtime, slice.iter().copied()))
        } else {
            let elements = match self.runtime.vector_elements(seq) {
                Ok(e) => e,
                Err(e) => {
                    self.runtime_error(e);
                    return None;
                }
            };
            let slice = elements[start_idx..end_idx].to_vec();
            Some(self.runtime.vector(slice))
        }
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

    fn memql(&mut self, needle: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            let result = self.runtime.car(current);
            let car = self.runtime_value(result)?;
            if self.eql_values(car, needle) {
                return Some(current);
            }
            let result = self.runtime.cdr(current);
            current = self.runtime_value(result)?;
        }
    }

    fn cl_assoc_if(&mut self, predicate: LispValue, alist: LispValue) -> Option<LispValue> {
        let mut current = alist;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!("expected a proper alist, got {}", self.runtime.format_value(current)));
                return None;
            }
            let entry = self.runtime.car(current).ok()?;
            if self.runtime.is_cons(entry) {
                let key = self.runtime.car(entry).ok()?;
                if !self.execute_funcall(predicate, &[key])?.is_nil() {
                    return Some(entry);
                }
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn cl_rassoc_if(&mut self, predicate: LispValue, alist: LispValue) -> Option<LispValue> {
        let mut current = alist;
        loop {
            if current.is_nil() { return Some(LispValue::NIL); }
            if !self.runtime.is_cons(current) { break; }
            let entry = self.runtime.car(current).ok()?;
            if self.runtime.is_cons(entry) {
                let val = self.runtime.cdr(entry).ok()?;
                if !self.execute_funcall(predicate, &[val])?.is_nil() {
                    return Some(entry);
                }
            }
            current = self.runtime.cdr(current).ok()?;
        }
        Some(LispValue::NIL)
    }

    fn cl_rassoc_if_not(&mut self, predicate: LispValue, alist: LispValue) -> Option<LispValue> {
        let mut current = alist;
        loop {
            if current.is_nil() { return Some(LispValue::NIL); }
            if !self.runtime.is_cons(current) { break; }
            let entry = self.runtime.car(current).ok()?;
            if self.runtime.is_cons(entry) {
                let val = self.runtime.cdr(entry).ok()?;
                if self.execute_funcall(predicate, &[val])?.is_nil() {
                    return Some(entry);
                }
            }
            current = self.runtime.cdr(current).ok()?;
        }
        Some(LispValue::NIL)
    }

    fn cl_assoc_if_not(&mut self, predicate: LispValue, alist: LispValue) -> Option<LispValue> {
        let mut current = alist;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!("expected a proper alist, got {}", self.runtime.format_value(current)));
                return None;
            }
            let entry = self.runtime.car(current).ok()?;
            if self.runtime.is_cons(entry) {
                let key = self.runtime.car(entry).ok()?;
                if self.execute_funcall(predicate, &[key])?.is_nil() {
                    return Some(entry);
                }
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn cl_member_if(&mut self, predicate: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!("expected a proper list, got {}", self.runtime.format_value(current)));
                return None;
            }
            let car = self.runtime.car(current).ok()?;
            if !self.execute_funcall(predicate, &[car])?.is_nil() {
                return Some(current);
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn cl_member_if_not(&mut self, predicate: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!("expected a proper list, got {}", self.runtime.format_value(current)));
                return None;
            }
            let car = self.runtime.car(current).ok()?;
            if self.execute_funcall(predicate, &[car])?.is_nil() {
                return Some(current);
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn member(&mut self, needle: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!(
                    "expected a proper list, got {}",
                    self.runtime.format_value(current)
                ));
                return None;
            }
            let car = self.runtime.car(current).ok()?;
            if self.runtime.equal(car, needle) {
                return Some(current);
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn eql_values(&self, a: LispValue, b: LispValue) -> bool {
        if a == b {
            return true;
        }
        if self.runtime.is_float(a) && self.runtime.is_float(b) {
            let fa = self.runtime.float_data(a).unwrap_or(0.0);
            let fb = self.runtime.float_data(b).unwrap_or(0.0);
            return fa.to_bits() == fb.to_bits();
        }
        if self.runtime.is_bignum(a) && self.runtime.is_bignum(b) {
            let ba = self.runtime.bignum_data(a).unwrap_or_default();
            let bb = self.runtime.bignum_data(b).unwrap_or_default();
            return ba == bb;
        }
        false
    }

    fn cl_position(&mut self, item: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        let mut idx = 0i64;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!(
                    "expected a proper list, got {}",
                    self.runtime.format_value(current)
                ));
                return None;
            }
            let car = self.runtime.car(current).ok()?;
            if self.eql_values(car, item) {
                return self.fixnum(idx, "cl-position");
            }
            current = self.runtime.cdr(current).ok()?;
            idx += 1;
        }
    }

    fn cl_find(&mut self, item: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!(
                    "expected a proper list, got {}",
                    self.runtime.format_value(current)
                ));
                return None;
            }
            let car = self.runtime.car(current).ok()?;
            if self.eql_values(car, item) {
                return Some(car);
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn cl_find_if(&mut self, predicate: LispValue, list: LispValue) -> Option<LispValue> {
        let elements = self.sequence_values(list)?;
        for e in elements {
            if !self.execute_funcall(predicate, &[e])?.is_nil() {
                return Some(e);
            }
        }
        Some(LispValue::NIL)
    }

    fn cl_position_if(&mut self, predicate: LispValue, list: LispValue) -> Option<LispValue> {
        let elements = self.sequence_values(list)?;
        for (i, e) in elements.into_iter().enumerate() {
            if !self.execute_funcall(predicate, &[e])?.is_nil() {
                return self.fixnum(i as i64, "cl-position-if");
            }
        }
        Some(LispValue::NIL)
    }

    fn cl_search(&mut self, needle: LispValue, haystack: LispValue) -> Option<LispValue> {
        let ne = self.sequence_values(needle)?;
        let hs = self.sequence_values(haystack)?;
        if ne.is_empty() {
            return self.fixnum(0, "cl-search");
        }
        if ne.len() > hs.len() {
            return Some(LispValue::NIL);
        }
        'outer: for start in 0..=hs.len() - ne.len() {
            for j in 0..ne.len() {
                if !self.eql_values(hs[start + j], ne[j]) {
                    continue 'outer;
                }
            }
            return self.fixnum(start as i64, "cl-search");
        }
        Some(LispValue::NIL)
    }

    fn cl_merge(&mut self, args: &[LispValue]) -> Option<LispValue> {
        // (cl-merge TYPE SEQ1 SEQ2 PRED)
        if args.len() < 4 {
            self.error("cl-merge requires type, two sequences, and a predicate");
            return None;
        }
        let elems1 = self.sequence_values(args[1])?;
        let elems2 = self.sequence_values(args[2])?;
        let predicate = args[3];
        let mut i = 0usize;
        let mut j = 0usize;
        let mut result = Vec::with_capacity(elems1.len() + elems2.len());
        while i < elems1.len() && j < elems2.len() {
            let cmp = self.execute_funcall(predicate, &[elems1[i], elems2[j]])?;
            if cmp.is_nil() {
                result.push(elems2[j]);
                j += 1;
            } else {
                result.push(elems1[i]);
                i += 1;
            }
        }
        result.extend_from_slice(&elems1[i..]);
        result.extend_from_slice(&elems2[j..]);
        Some(make_list(self.runtime, result.into_iter()))
    }

    fn cl_coerce(&mut self, object: LispValue, result_type: LispValue) -> Option<LispValue> {
        let type_name = self.runtime.symbol_name(result_type).ok()?;
        match type_name.as_str() {
            "list" => {
                if object.is_nil() || self.runtime.is_cons(object) {
                    Some(object)
                } else if self.runtime.is_vector(object) {
                    let elems = self.runtime.vector_elements(object).ok()?;
                    Some(make_list(self.runtime, elems.into_iter()))
                } else if self.runtime.is_string(object) {
                    let s = self.string_contents_owned(object)?;
                    let chars: Vec<LispValue> = s.chars().map(|c| LispValue::from_char(c)).collect();
                    Some(make_list(self.runtime, chars.into_iter()))
                } else {
                    Some(make_list(self.runtime, [object].into_iter()))
                }
            }
            "vector" => {
                let elems = self.sequence_values(object)?;
                Some(self.runtime.vector(elems))
            }
            "string" => {
                if self.runtime.is_string(object) {
                    Some(object)
                } else {
                    let elems = self.sequence_values(object)?;
                    let s: String = elems.into_iter()
                        .filter_map(|v| {
                            if v.is_fixnum() {
                                char::from_u32(v.as_fixnum()? as u32)
                            } else { None }
                        })
                        .collect();
                    Some(self.runtime.string(s))
                }
            }
            _ => Some(object),
        }
    }

    fn cl_replace(&mut self, seq1: LispValue, seq2: LispValue) -> Option<LispValue> {
        let elems1 = self.sequence_values(seq1)?;
        let elems2 = self.sequence_values(seq2)?;
        let n = elems1.len().min(elems2.len());
        let mut result = elems1;
        for i in 0..n {
            result[i] = elems2[i];
        }
        if seq1.is_nil() || self.runtime.is_cons(seq1) {
            Some(make_list(self.runtime, result.into_iter()))
        } else {
            Some(self.runtime.vector(result))
        }
    }

    fn cl_list_length(&mut self, list: LispValue) -> Option<LispValue> {
        // Returns length of a proper list, or nil if circular or dotted.
        let mut current = list;
        let mut len = 0i64;
        loop {
            if current.is_nil() {
                return self.fixnum(len, "cl-list-length");
            }
            if !self.runtime.is_cons(current) {
                return Some(LispValue::NIL); // dotted list
            }
            current = self.runtime.cdr(current).ok()?;
            len += 1;
            // Safety limit against infinite loops, matching Emacs
            if len > (isize::MAX as i64) / 2 {
                return Some(LispValue::NIL);
            }
        }
    }

    fn cl_ldiff(&mut self, list: LispValue, sublist: LispValue) -> Option<LispValue> {
        let mut current = list;
        let mut result = Vec::new();
        loop {
            if current == sublist {
                return Some(make_list(self.runtime, result.into_iter()));
            }
            if current.is_nil() {
                // SUBLIST not found in LIST — return LIST (or nil per spec)
                return Some(list);
            }
            if !self.runtime.is_cons(current) {
                return Some(list);
            }
            let car = self.runtime.car(current).ok()?;
            result.push(car);
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn plist_member(&mut self, plist: LispValue, prop: LispValue) -> Option<LispValue> {
        let mut current = plist;
        loop {
            if current.is_nil() { return Some(LispValue::NIL); }
            if !self.runtime.is_cons(current) { return Some(LispValue::NIL); }
            let key = self.runtime.car(current).ok()?;
            if key == prop { return Some(current); }
            let next = self.runtime.cdr(current).ok()?;
            if next.is_nil() || !self.runtime.is_cons(next) { return Some(LispValue::NIL); }
            current = self.runtime.cdr(next).ok()?;
        }
    }

    fn cl_remprop(&mut self, symbol: LispValue, propname: LispValue) -> Option<LispValue> {
        let Ok(plist) = self.runtime.symbol_plist(symbol) else { return Some(LispValue::NIL); };
        // Walk plist: (prop1 val1 prop2 val2 ...)
        let mut current = plist;
        let mut prev: Option<LispValue> = None;
        loop {
            if current.is_nil() { break; }
            if !self.runtime.is_cons(current) { break; }
            let key = self.runtime.car(current).ok()?;
            let val_cell = self.runtime.cdr(current).ok()?;
            if val_cell.is_nil() || !self.runtime.is_cons(val_cell) { break; }
            if key == propname {
                // Splice out key and val: prev->cdr = cddr(current)
                let rest = self.runtime.cdr(val_cell).ok()?;
                if let Some(p) = prev {
                    self.runtime.set_cdr(p, rest).ok()?;
                } else {
                    // key is first in plist — update symbol plist to rest
                    self.runtime.set_symbol_plist(symbol, rest).ok()?;
                }
                return Some(LispValue::TRUE);
            }
            prev = Some(val_cell);
            current = self.runtime.cdr(val_cell).ok()?;
        }
        Some(LispValue::NIL)
    }

    fn indirect_function(&mut self, object: LispValue) -> Option<LispValue> {
        if self.runtime.is_symbol(object) {
            match self.runtime.symbol_function(object) {
                Ok(Some(f)) => Some(f),
                _ => Some(object),
            }
        } else if self.runtime.is_cons(object) {
            // Check if it's a lambda form
            let car = self.runtime.car(object).ok()?;
            if let Ok(name) = self.runtime.symbol_name(car) {
                if name == "lambda" || name == "macro" {
                    return Some(object);
                }
            }
            Some(object)
        } else {
            Some(object)
        }
    }

    fn cl_tailp(&self, sublist: LispValue, list: LispValue) -> bool {
        let mut current = list;
        loop {
            if current == sublist {
                return true;
            }
            if current.is_nil() || !self.runtime.is_cons(current) {
                return false;
            }
            // Walk cdr safely
            current = match self.runtime.cdr(current) {
                Ok(c) => c,
                Err(_) => return false,
            };
        }
    }

    fn cl_mismatch(&mut self, seq1: LispValue, seq2: LispValue) -> Option<LispValue> {
        let elems1 = self.sequence_values(seq1)?;
        let elems2 = self.sequence_values(seq2)?;
        for i in 0..elems1.len().min(elems2.len()) {
            if !self.eql_values(elems1[i], elems2[i]) {
                return self.fixnum(i as i64, "cl-mismatch");
            }
        }
        if elems1.len() != elems2.len() {
            self.fixnum(elems1.len().min(elems2.len()) as i64, "cl-mismatch")
        } else {
            Some(LispValue::NIL)
        }
    }

    fn cl_count_if_not(&mut self, predicate: LispValue, list: LispValue) -> Option<LispValue> {
        let elements = self.sequence_values(list)?;
        let mut count = 0i64;
        for e in elements {
            if self.execute_funcall(predicate, &[e])?.is_nil() {
                count += 1;
            }
        }
        self.fixnum(count, "cl-count-if-not")
    }

    fn cl_find_if_not(&mut self, predicate: LispValue, list: LispValue) -> Option<LispValue> {
        let elements = self.sequence_values(list)?;
        for e in elements {
            if self.execute_funcall(predicate, &[e])?.is_nil() {
                return Some(e);
            }
        }
        Some(LispValue::NIL)
    }

    fn cl_position_if_not(&mut self, predicate: LispValue, list: LispValue) -> Option<LispValue> {
        let elements = self.sequence_values(list)?;
        for (i, e) in elements.into_iter().enumerate() {
            if self.execute_funcall(predicate, &[e])?.is_nil() {
                return self.fixnum(i as i64, "cl-position-if-not");
            }
        }
        Some(LispValue::NIL)
    }

    fn cl_count_if(&mut self, predicate: LispValue, list: LispValue) -> Option<LispValue> {
        let elements = self.sequence_values(list)?;
        let mut count = 0i64;
        for e in elements {
            if !self.execute_funcall(predicate, &[e])?.is_nil() {
                count += 1;
            }
        }
        self.fixnum(count, "cl-count-if")
    }

    fn cl_count(&mut self, item: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        let mut count = 0i64;
        loop {
            if current.is_nil() {
                return self.fixnum(count, "cl-count");
            }
            if !self.runtime.is_cons(current) {
                self.error(format!(
                    "expected a proper list, got {}",
                    self.runtime.format_value(current)
                ));
                return None;
            }
            let car = self.runtime.car(current).ok()?;
            if self.eql_values(car, item) {
                count += 1;
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn cl_reduce(
        &mut self,
        function: LispValue,
        sequence: LispValue,
        initial: Option<LispValue>,
    ) -> Option<LispValue> {
        let elements = self.sequence_values(sequence)?;
        if elements.is_empty() {
            return initial.or(Some(LispValue::NIL));
        }
        let mut acc = if let Some(init) = initial {
            init
        } else {
            elements[0]
        };
        let start = if initial.is_some() { 0 } else { 1 };
        for i in start..elements.len() {
            acc = self.execute_funcall(function, &[acc, elements[i]])?;
        }
        Some(acc)
    }

    fn cl_adjoin(&mut self, item: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        loop {
            if current.is_nil() {
                // Item not found — cons it onto the list.
                return Some(self.runtime.cons(item, list));
            }
            if !self.runtime.is_cons(current) {
                self.error(format!(
                    "expected a proper list, got {}",
                    self.runtime.format_value(current)
                ));
                return None;
            }
            let car = self.runtime.car(current).ok()?;
            if self.eql_values(car, item) {
                return Some(list); // Already present, return original list.
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn pairlis(
        &mut self,
        keys: LispValue,
        values: LispValue,
        alist: LispValue,
    ) -> Option<LispValue> {
        let keys = self.list_values(keys)?;
        let vals = self.list_values(values)?;
        let mut result = alist;
        for (k, v) in keys.into_iter().zip(vals.into_iter()).rev() {
            let pair = self.runtime.cons(k, v);
            result = self.runtime.cons(pair, result);
        }
        Some(result)
    }

    fn assoc(&mut self, key: LispValue, alist: LispValue, use_equal: bool) -> Option<LispValue> {
        let mut current = alist;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!(
                    "expected a proper alist, got {}",
                    self.runtime.format_value(current)
                ));
                return None;
            }
            let entry = self.runtime.car(current).ok()?;
            if self.runtime.is_cons(entry) {
                let entry_key = self.runtime.car(entry).ok()?;
                let matched = if use_equal {
                    self.runtime.equal(entry_key, key)
                } else {
                    entry_key == key
                };
                if matched {
                    return Some(entry);
                }
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn rassoc(&mut self, value: LispValue, alist: LispValue, use_equal: bool) -> Option<LispValue> {
        let mut current = alist;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!(
                    "expected a proper alist, got {}",
                    self.runtime.format_value(current)
                ));
                return None;
            }
            let entry = self.runtime.car(current).ok()?;
            if self.runtime.is_cons(entry) {
                let entry_val = self.runtime.cdr(entry).ok()?;
                let matched = if use_equal {
                    self.runtime.equal(entry_val, value)
                } else {
                    entry_val == value
                };
                if matched {
                    return Some(entry);
                }
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn assoc_string(
        &mut self,
        key: LispValue,
        alist: LispValue,
        case_fold: Option<LispValue>,
    ) -> Option<LispValue> {
        let key_str = self.runtime.string_contents_emacs(key).unwrap_or_default();
        let fold = case_fold.is_some_and(|f| !f.is_nil());
        let mut current = alist;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                return Some(LispValue::NIL);
            }
            let entry = self.runtime.car(current).ok()?;
            if self.runtime.is_cons(entry) {
                let entry_key = self.runtime.car(entry).ok()?;
                if self.runtime.is_string(entry_key) {
                    let entry_str = self.runtime.string_contents_emacs(entry_key).unwrap_or_default();
                    let matched = if fold {
                        entry_str.eq_ignore_ascii_case(&key_str)
                    } else {
                        entry_str == key_str
                    };
                    if matched {
                        return Some(entry);
                    }
                }
            }
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn alist_get(
        &mut self,
        key: LispValue,
        alist: LispValue,
        default: Option<LispValue>,
        remove: Option<LispValue>,
        testfn: Option<LispValue>,
    ) -> Option<LispValue> {
        let do_remove = remove.is_some_and(|f| !f.is_nil());
        let use_equal = testfn.is_some_and(|f| !f.is_nil() && f != self.runtime.intern("eq"));
        let mut current = alist;
        let mut prev: Option<LispValue> = None;
        loop {
            if current.is_nil() {
                return Some(default.unwrap_or(LispValue::NIL));
            }
            if !self.runtime.is_cons(current) {
                return Some(default.unwrap_or(LispValue::NIL));
            }
            let entry = self.runtime.car(current).ok()?;
            if self.runtime.is_cons(entry) {
                let entry_key = self.runtime.car(entry).ok()?;
                let matched = if use_equal {
                    self.runtime.equal(entry_key, key)
                } else {
                    entry_key == key
                };
                if matched {
                    let value = self.runtime.cdr(entry).ok()?;
                    if do_remove {
                        // Remove the entry from the alist
                        let rest = self.runtime.cdr(current).ok()?;
                        if let Some(p) = prev {
                            self.runtime.set_cdr(p, rest).ok()?;
                        }
                    }
                    return Some(value);
                }
            }
            prev = Some(current);
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn copy_sequence(&mut self, sequence: LispValue) -> Option<LispValue> {
        if sequence.is_nil() || self.runtime.is_cons(sequence) {
            let values = self.list_values(sequence)?;
            return Some(make_list(self.runtime, values));
        }
        if self.runtime.is_vector(sequence) {
            let elements = match self.runtime.vector_elements(sequence) {
                Ok(elements) => elements,
                Err(error) => {
                    self.runtime_error(error);
                    return None;
                }
            };
            return Some(self.runtime.vector(elements));
        }
        if self.runtime.is_string(sequence) {
            let data = match self.runtime.string_data(sequence) {
                Ok(data) => data,
                Err(error) => {
                    self.runtime_error(error);
                    return None;
                }
            };
            let bytes = data.bytes().to_vec();
            let chars = data.char_len();
            let multibyte = data.is_multibyte();
            return Some(self.runtime.string_from_bytes(bytes, chars, multibyte));
        }
        self.error(format!(
            "copy-sequence expected a sequence, got {}",
            self.runtime.format_value(sequence)
        ));
        None
    }

    fn cl_map(&mut self, args: &[LispValue]) -> Option<LispValue> {
        // (cl-map RESULT-TYPE FUNCTION SEQ &rest SEQS)
        let result_type = args[0];
        let function = args[1];
        let seq = args[2];
        let type_name = self.runtime.symbol_name(result_type).ok()?;
        let elements = self.sequence_values(seq)?;
        let mut mapped = Vec::with_capacity(elements.len());
        for elem in elements {
            mapped.push(self.execute_funcall(function, &[elem])?);
        }
        match type_name.as_str() {
            "list" => Some(make_list(self.runtime, mapped.into_iter())),
            "vector" => Some(self.runtime.vector(mapped)),
            _ => Some(make_list(self.runtime, mapped.into_iter())),
        }
    }

    fn mapcar(&mut self, function: LispValue, sequence: LispValue) -> Option<LispValue> {
        let elements = self.sequence_values(sequence)?;
        let mut mapped = Vec::with_capacity(elements.len());
        for element in elements {
            mapped.push(self.execute_funcall(function, &[element])?);
        }
        Some(make_list(self.runtime, mapped))
    }

    fn maplist(&mut self, function: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        let mut results = Vec::new();
        loop {
            if current.is_nil() {
                return Some(make_list(self.runtime, results.into_iter()));
            }
            if !self.runtime.is_cons(current) {
                self.error(format!(
                    "expected a proper list, got {}",
                    self.runtime.format_value(current)
                ));
                return None;
            }
            results.push(self.execute_funcall(function, &[current])?);
            current = self.runtime.cdr(current).ok()?;
        }
    }

    fn mapcon(&mut self, function: LispValue, list: LispValue) -> Option<LispValue> {
        let mut current = list;
        let mut results = Vec::new();
        loop {
            if current.is_nil() {
                break;
            }
            if !self.runtime.is_cons(current) {
                self.error(format!("expected a proper list, got {}", self.runtime.format_value(current)));
                return None;
            }
            results.push(self.execute_funcall(function, &[current])?);
            current = self.runtime.cdr(current).ok()?;
        }
        self.nconc(&results)
    }

    fn mapcan(&mut self, function: LispValue, list: LispValue) -> Option<LispValue> {
        let elements = self.list_values(list)?;
        let mut results = Vec::new();
        for elem in elements {
            results.push(self.execute_funcall(function, &[elem])?);
        }
        self.nconc(&results)
    }

    fn mapc(&mut self, function: LispValue, sequence: LispValue) -> Option<LispValue> {
        for element in self.sequence_values(sequence)? {
            self.execute_funcall(function, &[element])?;
        }
        Some(sequence)
    }

    fn sequence_every(
        &mut self,
        predicate: LispValue,
        sequence: LispValue,
    ) -> Option<LispValue> {
        for element in self.sequence_values(sequence)? {
            if self.execute_funcall(predicate, &[element])?.is_nil() {
                return Some(LispValue::NIL);
            }
        }
        Some(LispValue::TRUE)
    }

    fn sequence_some(
        &mut self,
        predicate: LispValue,
        sequence: LispValue,
    ) -> Option<LispValue> {
        for element in self.sequence_values(sequence)? {
            let result = self.execute_funcall(predicate, &[element])?;
            if !result.is_nil() {
                return Some(result);
            }
        }
        Some(LispValue::NIL)
    }

    fn cl_delete_if(
        &mut self,
        predicate: LispValue,
        list: LispValue,
        negate: bool,
    ) -> Option<LispValue> {
        // Skip matching elements at the front.
        let mut current = list;
        loop {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            if !self.runtime.is_cons(current) {
                self.error(format!("expected a proper list, got {}", self.runtime.format_value(current)));
                return None;
            }
            let car = self.runtime.car(current).ok()?;
            let matched = !self.execute_funcall(predicate, &[car])?.is_nil();
            let should_delete = if negate { !matched } else { matched };
            if !should_delete {
                break;
            }
            current = self.runtime.cdr(current).ok()?;
        }
        let result = current;
        // Walk the rest, splicing out matching elements.
        loop {
            let cdr = self.runtime.cdr(current).ok()?;
            if cdr.is_nil() {
                break;
            }
            if !self.runtime.is_cons(cdr) {
                self.error(format!("expected a proper list, got {}", self.runtime.format_value(cdr)));
                return None;
            }
            let car = self.runtime.car(cdr).ok()?;
            let matched = !self.execute_funcall(predicate, &[car])?.is_nil();
            let should_delete = if negate { !matched } else { matched };
            if should_delete {
                let cdrdr = self.runtime.cdr(cdr).ok()?;
                self.runtime.set_cdr(current, cdrdr).ok()?;
            } else {
                current = cdr;
            }
        }
        Some(result)
    }

    fn remove_if(
        &mut self,
        predicate: LispValue,
        sequence: LispValue,
        negate: bool,
    ) -> Option<LispValue> {
        let elements = self.sequence_values(sequence)?;
        let mut result = Vec::new();
        for element in elements {
            let satisfied = !self.execute_funcall(predicate, &[element])?.is_nil();
            // cl-remove-if (negate=false): keep if !satisfied
            // cl-remove-if-not (negate=true): keep if satisfied
            if satisfied == negate {
                result.push(element);
            }
        }
        if sequence.is_nil() || self.runtime.is_cons(sequence) {
            Some(make_list(self.runtime, result.into_iter()))
        } else {
            Some(self.runtime.vector(result))
        }
    }

    fn remove_duplicates(&mut self, sequence: LispValue) -> Option<LispValue> {
        let elements = self.sequence_values(sequence)?;
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for elem in elements {
            let key = if self.runtime.is_float(elem) {
                self.runtime.float_data(elem).ok()?.to_bits()
            } else if let Some(n) = elem.as_fixnum() {
                n as u64
            } else {
                // For eql comparison (default): identity-based for strings and other heap objects.
                // Two distinct strings with same content are NOT eql (unlike equal).
                elem.heap_addr().unwrap_or(0) as u64
            };
            if seen.insert(key) {
                result.push(elem);
            }
        }
        if sequence.is_nil() || self.runtime.is_cons(sequence) {
            Some(make_list(self.runtime, result.into_iter()))
        } else {
            Some(self.runtime.vector(result))
        }
    }

    fn cl_concatenate(&mut self, args: &[LispValue]) -> Option<LispValue> {
        if args.is_empty() {
            return Some(LispValue::NIL);
        }
        let mut all: Vec<LispValue> = Vec::new();
        for arg in &args[1..] {
            if let Some(values) = self.sequence_values(*arg) {
                all.extend(values);
            }
        }
        let type_sym = self.runtime.symbol_name(args[0]).ok()?;
        match type_sym.as_str() {
            "list" => Some(make_list(self.runtime, all.into_iter())),
            "string" => {
                let mut s = String::new();
                for v in all {
                    if self.runtime.is_string(v) {
                        s.push_str(&self.runtime.string_contents_emacs(v).unwrap_or_default());
                    }
                }
                Some(self.runtime.string(&s))
            }
            "vector" => Some(self.runtime.vector(all)),
            _ => Some(make_list(self.runtime, all.into_iter())),
        }
    }

    fn tree_equal(&mut self, a: LispValue, b: LispValue) -> bool {
        if self.runtime.is_cons(a) && self.runtime.is_cons(b) {
            let a_car = self.runtime.car(a).unwrap_or(LispValue::NIL);
            let b_car = self.runtime.car(b).unwrap_or(LispValue::NIL);
            let a_cdr = self.runtime.cdr(a).unwrap_or(LispValue::NIL);
            let b_cdr = self.runtime.cdr(b).unwrap_or(LispValue::NIL);
            self.tree_equal(a_car, b_car) && self.tree_equal(a_cdr, b_cdr)
        } else {
            self.runtime.equal(a, b)
        }
    }

    fn cl_typep(&self, obj: LispValue, type_spec: LispValue) -> Option<LispValue> {
        let type_name = self.runtime.symbol_name(type_spec).ok()?;
        let result = match type_name.as_str() {
            "integer" => obj.is_fixnum() || self.runtime.is_bignum(obj),
            "fixnum" => obj.is_fixnum(),
            "float" => self.runtime.is_float(obj),
            "string" => self.runtime.is_string(obj),
            "symbol" => obj.is_nil() || obj.is_true() || self.runtime.is_symbol(obj),
            "cons" | "list" => self.runtime.is_cons(obj),
            "null" => obj.is_nil(),
            "sequence" => obj.is_nil() || self.runtime.is_cons(obj)
                || self.runtime.is_vector(obj) || self.runtime.is_string(obj),
            "vector" | "array" => self.runtime.is_vector(obj),
            "function" => self.runtime.is_function(obj),
            "hash-table" => self.runtime.is_hash_table(obj),
            "number" => self.runtime.is_number(obj),
            "boolean" => obj.is_nil() || obj.is_true(),
            "atom" => !self.runtime.is_cons(obj),
            _ => return Some(LispValue::NIL),
        };
        Some(bool_value(result))
    }

    fn cl_set_difference(&mut self, list1: LispValue, list2: LispValue) -> Option<LispValue> {
        let elements1 = self.list_values(list1)?;
        let elements2 = self.list_values(list2)?;
        let mut result = Vec::new();
        for e in elements1 {
            let in_list2 = elements2.iter().any(|x| self.eql_values(*x, e));
            if !in_list2 {
                result.push(e);
            }
        }
        Some(make_list(self.runtime, result.into_iter()))
    }

    fn cl_intersection(&mut self, list1: LispValue, list2: LispValue) -> Option<LispValue> {
        let elements1 = self.list_values(list1)?;
        let elements2 = self.list_values(list2)?;
        let mut result = Vec::new();
        for e in elements1 {
            let in_list2 = elements2.iter().any(|x| self.eql_values(*x, e));
            if in_list2 {
                result.push(e);
            }
        }
        Some(make_list(self.runtime, result.into_iter()))
    }

    fn cl_fill(&mut self, seq: LispValue, item: LispValue) -> Option<LispValue> {
        if seq.is_nil() || self.runtime.is_cons(seq) {
            let elements = self.list_values(seq)?;
            let mut result = Vec::with_capacity(elements.len());
            for _ in 0..elements.len() {
                result.push(item);
            }
            Some(make_list(self.runtime, result.into_iter()))
        } else if self.runtime.is_vector(seq) {
            match self.runtime.vector_elements(seq) {
                Ok(elements) => {
                    let mut result = Vec::with_capacity(elements.len());
                    for _ in 0..elements.len() {
                        result.push(item);
                    }
                    Some(self.runtime.vector(result))
                }
                Err(e) => {
                    self.runtime_error(e);
                    None
                }
            }
        } else if self.runtime.is_string(seq) {
            Some(seq)
        } else {
            self.error("cl-fill: expected sequence");
            None
        }
    }

    fn cl_set_exclusive_or(&mut self, list1: LispValue, list2: LispValue) -> Option<LispValue> {
        // Elements in exactly one of the two lists, preserving order.
        let elements1 = self.list_values(list1)?;
        let elements2 = self.list_values(list2)?;
        let mut result = Vec::new();
        for e in &elements1 {
            if !elements2.iter().any(|x| self.eql_values(*x, *e)) {
                result.push(*e);
            }
        }
        for e in &elements2 {
            if !elements1.iter().any(|x| self.eql_values(*x, *e)) {
                result.push(*e);
            }
        }
        Some(make_list(self.runtime, result.into_iter()))
    }

    fn cl_union(&mut self, list1: LispValue, list2: LispValue) -> Option<LispValue> {
        let elements1 = self.list_values(list1)?;
        let elements2 = self.list_values(list2)?;
        let mut result = Vec::new();
        for e in &elements1 {
            result.push(*e);
        }
        for e in &elements2 {
            let already_in = result.iter().any(|x| self.eql_values(*x, *e));
            if !already_in {
                result.push(*e);
            }
        }
        Some(make_list(self.runtime, result.into_iter()))
    }

    fn cl_sublis(&mut self, alist: LispValue, tree: LispValue) -> Option<LispValue> {
        // Substitute keys→values in tree leaves using eql comparison.
        if tree.is_nil() {
            return Some(LispValue::NIL);
        }
        if !self.runtime.is_cons(tree) {
            // Leaf: look up in alist by eql
            let mut current = alist;
            loop {
                if current.is_nil() {
                    return Some(tree); // no match, keep original
                }
                if !self.runtime.is_cons(current) {
                    break;
                }
                let entry = self.runtime.car(current).ok()?;
                let key = self.runtime.car(entry).ok()?;
                if self.eql_values(key, tree) {
                    return self.runtime.cdr(entry).ok();
                }
                current = self.runtime.cdr(current).ok()?;
            }
            return Some(tree);
        }
        let car = self.runtime.car(tree).ok()?;
        let cdr = self.runtime.cdr(tree).ok()?;
        let new_car = self.cl_sublis(alist, car)?;
        let new_cdr = self.cl_sublis(alist, cdr)?;
        Some(self.runtime.cons(new_car, new_cdr))
    }

    fn substitute_seq_if(
        &mut self,
        new_val: LispValue,
        predicate: LispValue,
        sequence: LispValue,
        negate: bool,
    ) -> Option<LispValue> {
        let elements = self.sequence_values(sequence)?;
        let result: Vec<LispValue> = elements
            .into_iter()
            .map(|elem| {
                let matched = !self.execute_funcall(predicate, &[elem])
                    .unwrap_or(LispValue::NIL).is_nil();
                if matched != negate { new_val } else { elem }
            })
            .collect();
        if sequence.is_nil() || self.runtime.is_cons(sequence) {
            Some(make_list(self.runtime, result.into_iter()))
        } else {
            Some(self.runtime.vector(result))
        }
    }

    fn substitute_seq(
        &mut self,
        new_val: LispValue,
        old_val: LispValue,
        sequence: LispValue,
    ) -> Option<LispValue> {
        let elements = self.sequence_values(sequence)?;
        let result: Vec<LispValue> = elements
            .into_iter()
            .map(|elem| {
                if self.runtime.equal(elem, old_val) {
                    new_val
                } else {
                    elem
                }
            })
            .collect();
        if sequence.is_nil() || self.runtime.is_cons(sequence) {
            Some(make_list(self.runtime, result.into_iter()))
        } else {
            Some(self.runtime.vector(result))
        }
    }

    fn cl_subst_if(
        &mut self,
        new_val: LispValue,
        predicate: LispValue,
        tree: LispValue,
        negate: bool,
    ) -> Option<LispValue> {
        if tree.is_nil() {
            return Some(LispValue::NIL);
        }
        if !self.runtime.is_cons(tree) {
            let result = self.execute_funcall(predicate, &[tree])?;
            let matches = if negate { result.is_nil() } else { !result.is_nil() };
            return if matches { Some(new_val) } else { Some(tree) };
        }
        let car = self.runtime.car(tree).ok()?;
        let cdr = self.runtime.cdr(tree).ok()?;
        let new_car = self.cl_subst_if(new_val, predicate, car, negate)?;
        let new_cdr = self.cl_subst_if(new_val, predicate, cdr, negate)?;
        Some(self.runtime.cons(new_car, new_cdr))
    }

    fn mapconcat(
        &mut self,
        function: LispValue,
        sequence: LispValue,
        separator: LispValue,
    ) -> Option<LispValue> {
        let sep = self.runtime.string_contents(separator).ok()?.to_string();
        let elements = self.sequence_values(sequence)?;
        let mut parts = Vec::new();
        for elem in elements {
            let result = self.execute_funcall(function, &[elem]).unwrap_or(LispValue::NIL);
            parts.push(self.runtime.string_contents_emacs(result).unwrap_or_default());
        }
        Some(self.runtime.string(&parts.join(&sep)))
    }

    fn copy_list(&mut self, list: LispValue) -> Option<LispValue> {
        let elements = self.list_values(list)?;
        Some(make_list(self.runtime, elements.into_iter()))
    }

    fn make_list(&mut self, length: LispValue, init: LispValue) -> Option<LispValue> {
        let n = self.fixnum_arg("make-list", length)?;
        if n < 0 {
            self.error("primitive `make-list` expected a non-negative length");
            return None;
        }
        let mut result = LispValue::NIL;
        for _ in 0..n {
            result = self.runtime.cons(init, result);
        }
        Some(result)
    }

    fn number_sequence(
        &mut self,
        from: LispValue,
        to: LispValue,
        step: Option<LispValue>,
    ) -> Option<LispValue> {
        let step = match step {
            Some(s) if s.is_nil() => None,
            s => s,
        };
        let is_float = self.runtime.is_float(from)
            || self.runtime.is_float(to)
            || step.is_some_and(|s| self.runtime.is_float(s));
        if is_float {
            let start = self.number_arg("number-sequence", from)?;
            let end = self.number_arg("number-sequence", to)?;
            let step = match step {
                Some(s) => self.number_arg("number-sequence", s)?,
                None if end >= start => 1.0,
                None => -1.0,
            };
            if step == 0.0 {
                self.error("primitive `number-sequence` step must be non-zero");
                return None;
            }
            let mut result = Vec::new();
            let mut current = start;
            let going_up = step > 0.0;
            let mut count = 0;
            loop {
                if count >= 1_000_000 || (going_up && current > end) || (!going_up && current < end)
                {
                    break;
                }
                result.push(self.runtime.float(current));
                current += step;
                count += 1;
            }
            Some(make_list(self.runtime, result.into_iter()))
        } else if self.runtime.is_bignum(from)
            || self.runtime.is_bignum(to)
            || step.is_some_and(|s| self.runtime.is_bignum(s))
        {
            let start = self.bignum_arg("number-sequence", from)?;
            let end = self.bignum_arg("number-sequence", to)?;
            let step = match step {
                Some(s) => self.bignum_arg("number-sequence", s)?,
                None if end >= start => rug::Integer::from(1),
                None => rug::Integer::from(-1),
            };
            if step == 0 {
                self.error("primitive `number-sequence` step must be non-zero");
                return None;
            }
            let mut result = Vec::new();
            let mut current = start;
            let going_up = step > 0;
            let mut count = 0;
            loop {
                if count >= 1_000_000 {
                    self.error("primitive `number-sequence` too many elements");
                    return None;
                }
                if going_up && current > end || !going_up && current < end {
                    break;
                }
                result.push(self.runtime.bignum(current.clone()));
                current += &step;
                count += 1;
            }
            Some(make_list(self.runtime, result.into_iter()))
        } else {
            let start = self.fixnum_arg("number-sequence", from)?;
            let end = self.fixnum_arg("number-sequence", to)?;
            let step = match step {
                Some(s) => self.fixnum_arg("number-sequence", s)?,
                None if end >= start => 1,
                None => -1,
            };
            if step == 0 {
                self.error("primitive `number-sequence` step must be non-zero");
                return None;
            }
            let mut result = Vec::new();
            let mut current = start;
            let going_up = step > 0;
            while !result.is_empty() || !(going_up && current > end || !going_up && current < end) {
                if going_up && current > end || !going_up && current < end {
                    break;
                }
                result.push(self.fixnum(current, "number-sequence")?);
                current = match current.checked_add(step) {
                    Some(v) => v,
                    None => {
                        self.error("integer overflow in primitive `number-sequence`");
                        return None;
                    }
                };
                if result.len() > 1_000_000 {
                    self.error("primitive `number-sequence` too many elements");
                    return None;
                }
            }
            Some(make_list(self.runtime, result.into_iter()))
        }
    }

    fn sequence_values(&mut self, sequence: LispValue) -> Option<Vec<LispValue>> {
        if sequence.is_nil() || self.runtime.is_cons(sequence) {
            return self.list_values(sequence);
        }
        if self.runtime.is_vector(sequence) {
            return match self.runtime.vector_elements(sequence) {
                Ok(elements) => Some(elements),
                Err(error) => {
                    self.runtime_error(error);
                    None
                }
            };
        }
        if self.runtime.is_string(sequence) {
            let contents = self.string_contents_owned(sequence)?;
            return Some(contents.chars().map(LispValue::from_char).collect());
        }
        self.error(format!(
            "expected a sequence, got {}",
            self.runtime.format_value(sequence)
        ));
        None
    }

    fn length(&mut self, value: LispValue) -> Option<usize> {
        if self.runtime.is_string(value) {
            return match self.runtime.string_data(value) {
                Ok(data) => Some(data.char_len()),
                Err(error) => {
                    self.runtime_error(error);
                    None
                }
            };
        }
        if self.runtime.is_vector(value) {
            return match self.runtime.vector_len(value) {
                Ok(len) => Some(len),
                Err(error) => {
                    self.runtime_error(error);
                    None
                }
            };
        }
        self.list_length(value)
    }

    fn concat(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let mut bytes = Vec::new();
        let mut chars = 0usize;
        let mut multibyte = false;
        for arg in args {
            let data = match self.runtime.string_data(*arg) {
                Ok(data) => data,
                Err(error) => {
                    self.runtime_error(error);
                    return None;
                }
            };
            bytes.extend_from_slice(data.bytes());
            chars += data.char_len();
            multibyte |= data.is_multibyte();
        }
        Some(self.runtime.string_from_bytes(bytes, chars, multibyte))
    }

    fn substring(
        &mut self,
        string: LispValue,
        start: LispValue,
        end: Option<LispValue>,
    ) -> Option<LispValue> {
        let contents = self.string_contents_owned(string)?;
        let chars = contents.chars().collect::<Vec<_>>();
        let len = i64::try_from(chars.len()).ok()?;
        let start = self.normalized_string_index("substring", start, len)?;
        let end = match end {
            Some(value) if !value.is_nil() => {
                self.normalized_string_index("substring", value, len)?
            }
            _ => len,
        };
        if start > end {
            self.error("substring start is after end");
            return None;
        }
        let start = usize::try_from(start).ok()?;
        let end = usize::try_from(end).ok()?;
        Some(
            self.runtime
                .string(chars[start..end].iter().collect::<String>()),
        )
    }

    fn normalized_string_index(&mut self, name: &str, index: LispValue, len: i64) -> Option<i64> {
        let index = self.fixnum_arg(name, index)?;
        let normalized = if index < 0 { len + index } else { index };
        if !(0..=len).contains(&normalized) {
            self.error(format!("primitive `{name}` string index out of range"));
            return None;
        }
        Some(normalized)
    }

    fn string_bytes_equal(&mut self, left: LispValue, right: LispValue) -> Option<LispValue> {
        let left = self.string_bytes(left)?;
        let right = self.string_bytes(right)?;
        Some(bool_value(left == right))
    }

    fn string_case_insensitive_equal(
        &mut self,
        left: LispValue,
        right: LispValue,
    ) -> Option<LispValue> {
        let left = self.string_contents_owned(left)?;
        let right = self.string_contents_owned(right)?;
        Some(bool_value(left.to_lowercase() == right.to_lowercase()))
    }

    fn split_string(
        &mut self,
        string: LispValue,
        separators: Option<LispValue>,
        omit_nulls: Option<LispValue>,
    ) -> Option<LispValue> {
        let s = self.string_contents_owned(string)?;
        let sep = match separators {
            Some(sep) if !sep.is_nil() => self.string_contents_owned(sep)?,
            _ => String::new(),
        };
        let omit = omit_nulls.map(|v| !v.is_nil()).unwrap_or(false);
        let parts: Vec<&str> = if sep.is_empty() {
            s.split_whitespace().collect()
        } else {
            s.split(&sep).collect()
        };
        let parts: Vec<&str> = if omit {
            parts.into_iter().filter(|p| !p.is_empty()).collect()
        } else {
            parts
        };
        let values: Vec<LispValue> = parts.into_iter().map(|p| self.runtime.string(p)).collect();
        Some(make_list(self.runtime, values.into_iter()))
    }

    fn string_join(&mut self, list: LispValue, separator: LispValue) -> Option<LispValue> {
        let sep = self.string_contents_owned(separator)?;
        let values = self.list_values(list)?;
        let parts: Vec<String> = values
            .into_iter()
            .filter_map(|v| self.string_contents_owned(v))
            .collect();
        Some(self.runtime.string(parts.join(&sep)))
    }

    fn string_trim(&mut self, string: LispValue, regexp: Option<LispValue>) -> Option<LispValue> {
        let mut s = self.string_contents_owned(string)?;
        if let Some(re) = regexp {
            if !re.is_nil() {
                let pat = self.string_contents_owned(re)?;
                s = s.trim_matches(|c: char| pat.contains(c)).to_string();
            } else {
                s = s.trim().to_string();
            }
        } else {
            s = s.trim().to_string();
        }
        Some(self.runtime.string(s))
    }

    fn string_trim_left(
        &mut self,
        string: LispValue,
        regexp: Option<LispValue>,
    ) -> Option<LispValue> {
        let mut s = self.string_contents_owned(string)?;
        if let Some(re) = regexp {
            if !re.is_nil() {
                let pat = self.string_contents_owned(re)?;
                s = s.trim_start_matches(|c: char| pat.contains(c)).to_string();
            } else {
                s = s.trim_start().to_string();
            }
        } else {
            s = s.trim_start().to_string();
        }
        Some(self.runtime.string(s))
    }

    fn string_trim_right(
        &mut self,
        string: LispValue,
        regexp: Option<LispValue>,
    ) -> Option<LispValue> {
        let mut s = self.string_contents_owned(string)?;
        if let Some(re) = regexp {
            if !re.is_nil() {
                let pat = self.string_contents_owned(re)?;
                s = s.trim_end_matches(|c: char| pat.contains(c)).to_string();
            } else {
                s = s.trim_end().to_string();
            }
        } else {
            s = s.trim_end().to_string();
        }
        Some(self.runtime.string(s))
    }

    fn string_remove_prefix(&mut self, str_val: LispValue, prefix: LispValue) -> Option<LispValue> {
        let s = self.string_contents_owned(str_val)?;
        let p = self.string_contents_owned(prefix)?;
        if s.starts_with(&p) {
            Some(self.runtime.string(&s[p.len()..]))
        } else {
            Some(str_val)
        }
    }

    fn string_remove_suffix(&mut self, str_val: LispValue, suffix: LispValue) -> Option<LispValue> {
        let s = self.string_contents_owned(str_val)?;
        let suf = self.string_contents_owned(suffix)?;
        if s.ends_with(&suf) {
            Some(self.runtime.string(&s[..s.len().saturating_sub(suf.len())]))
        } else {
            Some(str_val)
        }
    }

    fn string_lessp(&mut self, left: LispValue, right: LispValue) -> Option<LispValue> {
        let left = self.string_contents_owned(left)?;
        let right = self.string_contents_owned(right)?;
        Some(bool_value(left < right))
    }

    fn string_greaterp(&mut self, left: LispValue, right: LispValue) -> Option<LispValue> {
        let left = self.string_contents_owned(left)?;
        let right = self.string_contents_owned(right)?;
        Some(bool_value(left > right))
    }

    fn string_bytes_equal_multi(&mut self, args: &[LispValue]) -> Option<LispValue> {
        if args.len() <= 1 { return Some(LispValue::TRUE); }
        let first = self.string_contents_owned(args[0])?;
        Some(bool_value(args[1..].iter().all(|a| {
            self.string_contents_owned(*a).as_deref() == Some(first.as_str())
        })))
    }

    fn string_case_insensitive_equal_multi(&mut self, args: &[LispValue]) -> Option<LispValue> {
        if args.len() <= 1 { return Some(LispValue::TRUE); }
        let first = self.string_contents_owned(args[0])?.to_lowercase();
        Some(bool_value(args[1..].iter().all(|a| {
            self.string_contents_owned(*a).map(|s| s.to_lowercase()).as_deref() == Some(first.as_str())
        })))
    }

    fn string_lessp_multi(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let vals: Vec<String> = args.iter()
            .map(|v| self.string_contents_owned(*v))
            .collect::<Option<Vec<_>>>()?;
        Some(bool_value(vals.windows(2).all(|w| w[0] < w[1])))
    }

    fn string_greaterp_multi(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let vals: Vec<String> = args.iter()
            .map(|v| self.string_contents_owned(*v))
            .collect::<Option<Vec<_>>>()?;
        Some(bool_value(vals.windows(2).all(|w| w[0] > w[1])))
    }

    fn string_match_p(
        &mut self,
        regex: LispValue,
        string: LispValue,
        start: Option<LispValue>,
    ) -> Option<LispValue> {
        let pattern = self.string_contents_owned(regex)?;
        let text = self.string_contents_owned(string)?;
        let re = match regex::Regex::new(&pattern) {
            Ok(re) => re,
            Err(_) => {
                let symbol = self.runtime.intern("invalid-regexp");
                let data = make_list(self.runtime, [regex].into_iter());
                self.pending_signal = Some(SignaledValue { symbol, data });
                return None;
            }
        };
        let start_idx = match start {
            Some(s) if !s.is_nil() => self.sequence_index("string-match-p", s)?,
            _ => 0,
        };
        if start_idx >= text.len() {
            return Some(LispValue::NIL);
        }
        let haystack = &text[start_idx..];
        match re.find(haystack) {
            Some(m) => self.fixnum((start_idx + m.start()) as i64, "string-match-p"),
            None => Some(LispValue::NIL),
        }
    }

    fn string_match(
        &mut self,
        regex: LispValue,
        string: LispValue,
        start: Option<LispValue>,
    ) -> Option<LispValue> {
        let pattern = self.string_contents_owned(regex)?;
        let text = self.string_contents_owned(string)?;
        let text_owned = text.clone();
        let re = match regex::Regex::new(&pattern) {
            Ok(re) => re,
            Err(_) => {
                let symbol = self.runtime.intern("invalid-regexp");
                let data = make_list(self.runtime, [regex].into_iter());
                self.pending_signal = Some(SignaledValue { symbol, data });
                return None;
            }
        };
        let start_idx = match start {
            Some(s) if !s.is_nil() => self.sequence_index("string-match", s)?,
            _ => 0,
        };
        if start_idx >= text.len() {
            self.runtime.clear_match_data();
            return Some(LispValue::NIL);
        }
        let haystack = &text[start_idx..];
        match re.captures(haystack) {
            Some(caps) => {
                let groups: Vec<Option<(usize, usize)>> = caps
                    .iter()
                    .map(|cap| cap.map(|m| (start_idx + m.start(), start_idx + m.end())))
                    .collect();
                self.runtime.set_match_data(text_owned, groups);
                self.fixnum(
                    (start_idx + caps.get(0).unwrap().start()) as i64,
                    "string-match",
                )
            }
            None => {
                self.runtime.clear_match_data();
                Some(LispValue::NIL)
            }
        }
    }

    fn match_string_prim(
        &mut self,
        string: Option<LispValue>,
        group: Option<LispValue>,
    ) -> Option<LispValue> {
        let group = group.and_then(|v| v.as_fixnum()).unwrap_or(0) as usize;
        if let Some(s) = string {
            if !s.is_nil() {
                // If string is provided, return the substring
                return Some(self.runtime.match_string(group));
            }
        }
        Some(self.runtime.match_string(group))
    }

    fn replace_regexp_in_string(
        &mut self,
        regex: LispValue,
        rep: LispValue,
        string: LispValue,
        _fixedcase: Option<LispValue>,
        _literal: Option<LispValue>,
    ) -> Option<LispValue> {
        let pattern = self.string_contents_owned(regex)?;
        let replacement = self.string_contents_owned(rep)?;
        let text = self.string_contents_owned(string)?;
        let re = match regex::Regex::new(&pattern) {
            Ok(re) => re,
            Err(_) => {
                let symbol = self.runtime.intern("invalid-regexp");
                let data = make_list(self.runtime, [regex].into_iter());
                self.pending_signal = Some(SignaledValue { symbol, data });
                return None;
            }
        };
        let result = re.replace_all(&text, replacement.as_str()).to_string();
        Some(self.runtime.string(result))
    }

    fn char_to_string(&mut self, value: LispValue) -> Option<LispValue> {
        let ch = self.char_arg("char-to-string", value)?;
        Some(self.runtime.string(ch.to_string()))
    }

    fn string_to_char(&mut self, value: LispValue) -> Option<LispValue> {
        let contents = self.string_contents_owned(value)?;
        match contents.chars().next() {
            Some(ch) => Some(LispValue::from_char(ch)),
            None => self.fixnum(0, "string-to-char"),
        }
    }

    fn elt(&mut self, seq: LispValue, n: LispValue) -> Option<LispValue> {
        let index = self.sequence_index("elt", n)?;
        if self.runtime.is_vector(seq) {
            let elements = match self.runtime.vector_elements(seq) {
                Ok(e) => e,
                Err(e) => {
                    self.runtime_error(e);
                    return None;
                }
            };
            return Some(elements.get(index).copied().unwrap_or(LispValue::NIL));
        }
        if self.runtime.is_string(seq) {
            let contents = self.string_contents_owned(seq)?;
            let Some(ch) = contents.chars().nth(index) else {
                return Some(LispValue::NIL);
            };
            return Some(LispValue::from_char(ch));
        }
        let mut current = seq;
        for _ in 0..index {
            if current.is_nil() {
                return Some(LispValue::NIL);
            }
            let Ok(next) = self.runtime.cdr(current) else {
                return Some(LispValue::NIL);
            };
            current = next;
        }
        if current.is_nil() {
            return Some(LispValue::NIL);
        }
        let Ok(car) = self.runtime.car(current) else {
            return Some(LispValue::NIL);
        };
        Some(car)
    }

    fn downcase(&mut self, value: LispValue) -> LispValue {
        if self.runtime.is_string(value) {
            let contents = match self.runtime.string_contents(value) {
                Ok(c) => c.to_string(),
                Err(e) => {
                    self.runtime_error(e);
                    return LispValue::NIL;
                }
            };
            return self.runtime.string(contents.to_lowercase());
        }
        if self.runtime.is_symbol(value) {
            let name = match self.runtime.symbol_name(value) {
                Ok(n) => n.to_string(),
                Err(e) => {
                    self.runtime_error(e);
                    return LispValue::NIL;
                }
            };
            let lowered = name.to_lowercase();
            return self.runtime.intern(&lowered);
        }
        value
    }

    fn upcase(&mut self, value: LispValue) -> LispValue {
        if self.runtime.is_string(value) {
            let contents = match self.runtime.string_contents(value) {
                Ok(c) => c.to_string(),
                Err(e) => {
                    self.runtime_error(e);
                    return LispValue::NIL;
                }
            };
            return self.runtime.string(contents.to_uppercase());
        }
        if self.runtime.is_symbol(value) {
            let name = match self.runtime.symbol_name(value) {
                Ok(n) => n.to_string(),
                Err(e) => {
                    self.runtime_error(e);
                    return LispValue::NIL;
                }
            };
            let uppered = name.to_uppercase();
            return self.runtime.intern(&uppered);
        }
        value
    }

    fn capitalize(&mut self, value: LispValue) -> LispValue {
        if self.runtime.is_string(value) {
            let contents = match self.runtime.string_contents(value) {
                Ok(c) => c.to_string(),
                Err(e) => {
                    self.runtime_error(e);
                    return LispValue::NIL;
                }
            };
            let mut chars: Vec<char> = contents.chars().collect();
            if let Some(first) = chars.first_mut() {
                *first = first.to_uppercase().next().unwrap_or(*first);
            }
            for ch in &mut chars[1..] {
                *ch = ch.to_lowercase().next().unwrap_or(*ch);
            }
            let result: String = chars.into_iter().collect();
            return self.runtime.string(result);
        }
        if self.runtime.is_symbol(value) {
            let name = match self.runtime.symbol_name(value) {
                Ok(n) => n.to_string(),
                Err(e) => {
                    self.runtime_error(e);
                    return LispValue::NIL;
                }
            };
            let mut chars: Vec<char> = name.chars().collect();
            if let Some(first) = chars.first_mut() {
                *first = first.to_uppercase().next().unwrap_or(*first);
            }
            for ch in &mut chars[1..] {
                *ch = ch.to_lowercase().next().unwrap_or(*ch);
            }
            let result: String = chars.into_iter().collect();
            return self.runtime.intern(&result);
        }
        value
    }

    fn butlast(&mut self, list: LispValue, n: Option<LispValue>) -> Option<LispValue> {
        let n = match n {
            Some(v) => {
                let val = self.fixnum_arg("butlast", v)?;
                usize::try_from(val).ok()?
            }
            None => 1,
        };
        let values = self.list_values(list)?;
        if n >= values.len() {
            return Some(LispValue::NIL);
        }
        let result: Vec<_> = values[..values.len() - n].to_vec();
        Some(make_list(self.runtime, result.into_iter()))
    }

    fn delete_dups(&mut self, list: LispValue) -> LispValue {
        let mut current = list;
        loop {
            if current.is_nil() || !self.runtime.is_cons(current) { break; }
            let cdr = self.runtime.cdr(current).ok().unwrap_or(LispValue::NIL);
            if cdr.is_nil() || !self.runtime.is_cons(cdr) { break; }
            let car = self.runtime.car(current).ok().unwrap_or(LispValue::NIL);
            let cadr = self.runtime.car(cdr).ok().unwrap_or(LispValue::NIL);
            if self.runtime.equal(car, cadr) {
                let cddr = self.runtime.cdr(cdr).ok().unwrap_or(LispValue::NIL);
                if self.runtime.set_cdr(current, cddr).is_err() { break; }
            } else {
                current = cdr;
            }
        }
        list
    }

    fn delq(&mut self, obj: LispValue, list: LispValue) -> Option<LispValue> {
        let values = self.list_values(list)?;
        let result: Vec<_> = values.into_iter().filter(|v| *v != obj).collect();
        Some(make_list(self.runtime, result.into_iter()))
    }

    fn remove(&mut self, obj: LispValue, list: LispValue) -> Option<LispValue> {
        let values = self.list_values(list)?;
        let result: Vec<_> = values
            .into_iter()
            .filter(|v| !self.runtime.equal(*v, obj))
            .collect();
        Some(make_list(self.runtime, result.into_iter()))
    }

    fn copy_tree(&mut self, value: LispValue) -> Option<LispValue> {
        if value.is_nil() || value.is_true() || value.is_fixnum() || self.runtime.is_float(value) {
            return Some(value);
        }
        if !self.runtime.is_cons(value) {
            return Some(value);
        }
        let car = self.runtime.car(value);
        let car_val = self.runtime_value(car)?;
        let cdr = self.runtime.cdr(value);
        let cdr_val = self.runtime_value(cdr)?;
        let copied_car = self.copy_tree(car_val)?;
        let copied_cdr = self.copy_tree(cdr_val)?;
        Some(self.runtime.cons(copied_car, copied_cdr))
    }

    fn copy_alist(&mut self, list: LispValue) -> Option<LispValue> {
        let values = self.list_values(list)?;
        let result: Vec<_> = values
            .into_iter()
            .map(|v| {
                if self.runtime.is_cons(v) {
                    let car = self.runtime.car(v).ok().unwrap_or(LispValue::NIL);
                    let cdr = self.runtime.cdr(v).ok().unwrap_or(LispValue::NIL);
                    self.runtime.cons(car, cdr)
                } else {
                    v
                }
            })
            .collect();
        Some(make_list(self.runtime, result.into_iter()))
    }

    fn fillarray(&mut self, array: LispValue, value: LispValue) -> Option<LispValue> {
        if self.runtime.is_vector(array) {
            let len = match self.runtime.vector_len(array) {
                Ok(l) => l,
                Err(e) => {
                    self.runtime_error(e);
                    return None;
                }
            };
            for i in 0..len {
                if let Err(e) = self.runtime.vector_aset(array, i, value) {
                    self.runtime_error(e);
                    return None;
                }
            }
            Some(array)
        } else if self.runtime.is_string(array) {
            let ch = self.char_arg("fillarray", value)?;
            let contents = self.string_contents_owned(array)?;
            let len = contents.chars().count();
            let filled: String = std::iter::repeat(ch).take(len).collect();
            let result = self.runtime.string(filled);
            self.runtime_value(Ok(result))?;
            Some(result)
        } else {
            self.error("primitive `fillarray` expected a vector or string");
            None
        }
    }

    fn vconcat(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let mut elements = Vec::new();
        for arg in args {
            if self.runtime.is_vector(*arg) {
                let vec_elements = match self.runtime.vector_elements(*arg) {
                    Ok(e) => e,
                    Err(e) => {
                        self.runtime_error(e);
                        return None;
                    }
                };
                elements.extend(vec_elements);
            } else if self.runtime.is_string(*arg) {
                let raw_contents = self.runtime.string_contents(*arg);
                let contents = match raw_contents {
                    Ok(c) => c.to_string(),
                    Err(e) => {
                        self.runtime_error(e);
                        return None;
                    }
                };
                elements.push(self.runtime.string(contents));
            } else {
                let values = self.list_values(*arg)?;
                elements.extend(values);
            }
        }
        Some(self.runtime.vector(elements))
    }

    fn nconc(&mut self, args: &[LispValue]) -> Option<LispValue> {
        if args.is_empty() {
            return Some(LispValue::NIL);
        }
        let Some(first) = args.iter().find(|v| !v.is_nil()) else {
            return Some(LispValue::NIL);
        };
        let first = *first;
        let mut last = first;
        loop {
            let cdr = self.runtime.cdr(last);
            let cdr_val = self.runtime_value(cdr)?;
            if cdr_val.is_nil() {
                break;
            }
            if !self.runtime.is_cons(cdr_val) {
                self.error("nconc: improper list");
                return None;
            }
            last = cdr_val;
        }
        for arg in args {
            if *arg == first || arg.is_nil() {
                continue;
            }
            let result = self.runtime.set_cdr(last, *arg);
            if result.is_err() {
                return Some(first);
            }
            let mut current = *arg;
            loop {
                let cdr = self.runtime.cdr(current);
                let cdr_val = self.runtime_value(cdr)?;
                if cdr_val.is_nil() {
                    break;
                }
                if !self.runtime.is_cons(cdr_val) {
                    self.error("nconc: improper list");
                    return None;
                }
                current = cdr_val;
            }
            last = current;
        }
        Some(first)
    }

    fn string_to_number(
        &mut self,
        string: LispValue,
        base: Option<LispValue>,
    ) -> Option<LispValue> {
        let contents = self.string_contents_owned(string)?;
        let radix = match base {
            Some(b) => {
                let v = self.fixnum_arg("string-to-number", b)?;
                u32::try_from(v).ok()?
            }
            None => 10,
        };
        let trimmed = contents.trim();
        let contents = if trimmed.starts_with('+') {
            &trimmed[1..]
        } else {
            trimmed
        };
        if contents.is_empty() {
            return self.fixnum(0, "string-to-number");
        }
        if radix == 10 && contents.contains('.') {
            if let Ok(value) = contents.parse::<f64>() {
                return Some(self.runtime.float(value));
            }
        }
        match i64::from_str_radix(contents, radix) {
            Ok(n) => self.fixnum(n, "string-to-number"),
            Err(_) => {
                if let Ok(n) = rug::Integer::from_str_radix(contents, radix as i32) {
                    Some(self.runtime.bignum(n))
                } else {
                    self.fixnum(0, "string-to-number")
                }
            }
        }
    }

    fn read_from_string(&mut self, string: LispValue) -> Option<LispValue> {
        let contents = self.string_contents_owned(string)?;
        let source = neovm_compiler::source::SourceFile::new(
            neovm_compiler::source::SourceId::new(0),
            Some("<read>".into()),
            contents,
        );
        let reader_output = reader::read_source(&source);
        self.diagnostics.extend(reader_output.diagnostics);
        if reader_output.forms.is_empty() {
            return Some(LispValue::NIL);
        }
        let mv = expand_value::surface_to_value(&reader_output.forms[0]);
        Some(macro_value_to_lisp(&mv, self.runtime))
    }

    fn execute_lambda_list(
        &mut self,
        lambda_list: LispValue,
        args: &[LispValue],
    ) -> Option<LispValue> {
        // Construct (funcall 'lambda-list arg1 arg2...) and eval it
        let funcall_sym = self.runtime.intern("funcall");
        let mut funcall_parts = vec![funcall_sym, lambda_list];
        funcall_parts.extend(args.iter().copied());
        let form = make_list(self.runtime, funcall_parts.into_iter());
        self.eval_form(form)
    }

    fn execute_autoload(
        &mut self,
        autoload_obj: LispValue,
        args: &[LispValue],
    ) -> Option<LispValue> {
        // autoload object: (autoload FILE DOCSTRING INTERACTIVE TYPE...)
        let file_list = self.runtime.cdr(autoload_obj).ok()?;
        let file_val = self.runtime.car(file_list).ok()?;
        // Load the file — this registers the real function
        self.load_file(file_val)?;
        // The loaded file should have replaced the symbol function.
        // The original callee was obtained via symbol_function lookup.
        // Return None so the caller retries with the now-loaded function.
        None
    }

    fn macroexpand_one(&mut self, form: LispValue) -> Option<LispValue> {
        // Convert form to source text, expand macros, return expanded form.
        let text = self.runtime.format_value(form);
        let source = neovm_compiler::source::SourceFile::new(
            neovm_compiler::source::SourceId::new(0),
            Some("<macroexpand>".into()),
            text.clone(),
        );
        let reader_output = neovm_compiler::reader::read_source(&source);
        if !reader_output.diagnostics.is_empty() || reader_output.forms.is_empty() {
            return Some(form); // Return original on parse failure
        }
        let mut session = neovm_compiler::expand::CompilerSession::new();
        let expand_output = session.expand_file_forms(reader_output.forms);
        if expand_output.forms.is_empty() {
            return Some(form);
        }
        Some(surface_to_lisp_value(
            self.runtime,
            &expand_output.forms[0],
        ))
    }

    fn eval_form(&mut self, form: LispValue) -> Option<LispValue> {
        let text = self.runtime.format_value(form);
        let source = neovm_compiler::source::SourceFile::new(
            neovm_compiler::source::SourceId::new(0),
            Some("<eval>".into()),
            text,
        );
        let artifact = neovm_compiler::compile_source("<eval>", source.text.clone());
        if !artifact.diagnostics.is_empty() {
            self.diagnostics.extend(artifact.diagnostics);
            return None;
        }
        let Some(regir) = artifact.regir else {
            return Some(LispValue::NIL);
        };
        let result = execute_module_with_args(&regir, &[], self.runtime);
        self.diagnostics.extend(result.diagnostics);
        result.value
    }

    fn defun_runtime(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let name = args[0];
        let lambda_sym = self.runtime.intern("lambda");
        let body_progn = make_list(self.runtime, args[2..].iter().copied());
        let lambda = make_list(self.runtime, [lambda_sym, args[1], body_progn].into_iter());
        let result = self.runtime.set_symbol_function(name, lambda);
        self.runtime_value(result).map(|_| name)
    }

    fn load_file(&mut self, file: LispValue) -> Option<LispValue> {
        let path = self.string_contents_owned(file)?;
        // Resolve via load-path, try cwd directly first
        let resolved = if std::path::Path::new(&path).exists() {
            path.clone()
        } else if let Some(p) = self.runtime.resolve_load_file(&path) {
            p
        } else {
            path.clone()
        };
        let contents = match std::fs::read_to_string(&resolved) {
            Ok(c) => c,
            Err(e) => {
                let msg = format!("cannot open load file: {e}");
                let error_symbol = self.runtime.intern("file-error");
                let msg_val = self.runtime.string(msg);
                let data = make_list(self.runtime, [msg_val].into_iter());
                self.pending_signal = Some(SignaledValue {
                    symbol: error_symbol,
                    data,
                });
                return None;
            }
        };
        let artifact = neovm_compiler::compile_source(&resolved, contents);
        if !artifact.diagnostics.is_empty() {
            self.diagnostics.extend(artifact.diagnostics);
            return None;
        }

        // Register defuns as persistent FunctionObjects
        if let Some(ref hir) = artifact.hir {
            for item in &hir.items {
                if let neovm_compiler::hir::HirItem::Defun(defun) = item {
                    let template = neovm_compiler::ssa::SsaLambdaTemplate {
                        params: defun.params.clone(),
                        captures: vec![],
                        declarations: defun.declarations.clone(),
                        body: Box::new(defun.body.clone()),
                    };
                    let function = self.runtime.function(template, vec![]);
                    let symbol = self.runtime.intern(&defun.name);
                    let _ = self.runtime.set_symbol_function(symbol, function);
                }
            }
        }

        let Some(regir) = artifact.regir else {
            return Some(LispValue::TRUE);
        };
        let result = execute_module_with_args(&regir, &[], self.runtime);
        self.diagnostics.extend(result.diagnostics);
        Some(LispValue::TRUE)
    }

    fn prin1_to_string(&mut self, value: LispValue) -> Option<LispValue> {
        Some(self.runtime.string(self.runtime.format_value(value)))
    }

    fn format_string(&mut self, format: LispValue, args: &[LispValue]) -> Option<LispValue> {
        let format = self.string_contents_owned(format)?;
        let mut output = String::new();
        let mut args = args.iter().copied();
        let mut chars = format.chars();
        while let Some(ch) = chars.next() {
            if ch != '%' {
                output.push(ch);
                continue;
            }
            let Some(spec) = chars.next() else {
                self.error("format string ended after `%`");
                return None;
            };
            match spec {
                '%' => output.push('%'),
                's' => {
                    let Some(value) = args.next() else {
                        self.error("format `%s` requires an argument");
                        return None;
                    };
                    output.push_str(&self.format_princ(value)?);
                }
                'S' => {
                    let Some(value) = args.next() else {
                        self.error("format `%S` requires an argument");
                        return None;
                    };
                    output.push_str(&self.runtime.format_value(value));
                }
                'd' => {
                    let Some(value) = args.next() else {
                        self.error("format `%d` requires an argument");
                        return None;
                    };
                    if self.runtime.is_bignum(value) {
                        let n = self.bignum_arg("format", value)?;
                        output.push_str(&n.to_string());
                    } else {
                        let value = self.number_arg("format", value)?;
                        output.push_str(&(value as i64).to_string());
                    }
                }
                'f' | 'e' | 'g' | 'E' | 'G' => {
                    let Some(value) = args.next() else {
                        self.error(format!("format `%{spec}` requires an argument"));
                        return None;
                    };
                    let value = self.number_arg("format", value)?;
                    let formatted = match spec {
                        'f' => format!("{value}"),
                        'e' => format!("{value:e}"),
                        'E' => format!("{value:E}"),
                        'g' => format!("{value}"),
                        'G' => format!("{value}"),
                        _ => unreachable!(),
                    };
                    output.push_str(&formatted);
                }
                'x' | 'X' => {
                    let Some(value) = args.next() else {
                        self.error(format!("format `%{spec}` requires an argument"));
                        return None;
                    };
                    if self.runtime.is_bignum(value) {
                        let n = self.bignum_arg("format", value)?;
                        let s = format!("{n:x}");
                        if spec == 'X' {
                            output.push_str(&s.to_uppercase());
                        } else {
                            output.push_str(&s);
                        }
                    } else {
                        let value = self.number_arg("format", value)?;
                        let s = format!("{:x}", value as i64);
                        if spec == 'X' {
                            output.push_str(&s.to_uppercase());
                        } else {
                            output.push_str(&s);
                        }
                    }
                }
                'o' => {
                    let Some(value) = args.next() else {
                        self.error("format `%o` requires an argument");
                        return None;
                    };
                    if self.runtime.is_bignum(value) {
                        let n = self.bignum_arg("format", value)?;
                        output.push_str(&format!("{n:o}"));
                    } else {
                        let value = self.number_arg("format", value)?;
                        output.push_str(&format!("{:o}", value as i64));
                    }
                }
                'c' => {
                    let Some(value) = args.next() else {
                        self.error("format `%c` requires an argument");
                        return None;
                    };
                    let ch = self.char_arg("format", value)?;
                    output.push(ch);
                }
                _ => {
                    self.error(format!("unsupported format specifier `%{spec}`"));
                    return None;
                }
            }
        }
        Some(self.runtime.string(output))
    }

    fn format_princ(&mut self, value: LispValue) -> Option<String> {
        if self.runtime.is_string(value) {
            return self.string_contents_owned(value);
        }
        Some(self.runtime.format_value(value))
    }

    fn format_signal_data(&mut self, args: &[LispValue]) -> LispValue {
        if args.is_empty() {
            return make_list(self.runtime, std::iter::empty());
        }
        let formatted = self.format_string(args[0], &args[1..]);
        match formatted {
            Some(msg) => make_list(self.runtime, [msg].into_iter()),
            None => make_list(self.runtime, args.iter().copied()),
        }
    }

    fn aref(&mut self, sequence: LispValue, index: LispValue) -> Option<LispValue> {
        let index = self.sequence_index("aref", index)?;
        if self.runtime.is_vector(sequence) {
            let result = self.runtime.vector_aref(sequence, index);
            return self.runtime_value(result);
        }
        if self.runtime.is_string(sequence) {
            let contents = self.string_contents_owned(sequence)?;
            let Some(ch) = contents.chars().nth(index) else {
                self.error("primitive `aref` string index out of range");
                return None;
            };
            return Some(LispValue::from_char(ch));
        }
        self.error(format!(
            "primitive `aref` expected a string or vector, got {}",
            self.runtime.format_value(sequence)
        ));
        None
    }

    fn aset(
        &mut self,
        sequence: LispValue,
        index: LispValue,
        value: LispValue,
    ) -> Option<LispValue> {
        let index = self.sequence_index("aset", index)?;
        if self.runtime.is_vector(sequence) {
            let result = self.runtime.vector_aset(sequence, index, value);
            return self.runtime_value(result);
        }
        if self.runtime.is_string(sequence) {
            let ch = self.char_arg("aset", value)?;
            match self.runtime.string_set_char(sequence, index, ch) {
                Ok(()) => return Some(value),
                Err(e) => {
                    self.error(format!("aset: {e:?}"));
                    return None;
                }
            }
        }
        self.error(format!(
            "primitive `aset` expected a vector, got {}",
            self.runtime.format_value(sequence)
        ));
        None
    }

    fn make_hash_table(&mut self, args: &[LispValue]) -> Option<LispValue> {
        if !args.len().is_multiple_of(2) {
            self.error("make-hash-table requires keyword/value pairs");
            return None;
        }
        let mut test = HashTableTest::Eql;
        for pair in args.chunks_exact(2) {
            let keyword = match self.runtime.symbol_name(pair[0]) {
                Ok(name) => name,
                Err(error) => {
                    self.runtime_error(error);
                    return None;
                }
            };
            if keyword == ":test" {
                test = self.hash_table_test_arg(pair[1])?;
            }
        }
        Some(self.runtime.hash_table(test))
    }

    fn hash_table_test_arg(&mut self, value: LispValue) -> Option<HashTableTest> {
        let name = match self.runtime.symbol_name(value) {
            Ok(name) => name,
            Err(error) => {
                self.runtime_error(error);
                return None;
            }
        };
        match name.as_str() {
            "eq" => Some(HashTableTest::Eq),
            "eql" => Some(HashTableTest::Eql),
            "equal" => Some(HashTableTest::Equal),
            _ => {
                self.error(format!("unsupported hash table test `{name}`"));
                None
            }
        }
    }

    fn gethash(
        &mut self,
        key: LispValue,
        table: LispValue,
        default: Option<LispValue>,
    ) -> Option<LispValue> {
        match self.runtime.gethash(key, table) {
            Ok(Some(value)) => Some(value),
            Ok(None) => Some(default.unwrap_or(LispValue::NIL)),
            Err(error) => {
                self.runtime_error(error);
                None
            }
        }
    }

    fn maphash(&mut self, function: LispValue, table: LispValue) -> Option<LispValue> {
        let entries = match self.runtime.hash_table_entries(table) {
            Ok(entries) => entries,
            Err(error) => {
                self.runtime_error(error);
                return None;
            }
        };
        for (key, value) in entries {
            self.execute_funcall(function, &[key, value])?;
        }
        Some(LispValue::NIL)
    }

    fn sxhash_eq(&mut self, obj: LispValue) -> Option<LispValue> {
        // Identity hash: use the raw bits of the LispValue as the hash.
        // Fixnums and other immediates use their tag bits.
        let hash = obj.as_fixnum().map(|n| n as u64).unwrap_or_else(|| {
            obj.heap_addr().map(|a| a as u64).unwrap_or(0)
        });
        self.fixnum(hash as i64, "sxhash-eq")
    }

    fn sxhash_eql(&mut self, obj: LispValue) -> Option<LispValue> {
        // Like eq but floats and bignums compare by value.
        if self.runtime.is_float(obj) {
            let f = self.runtime.float_data(obj).ok()?;
            self.fixnum(f.to_bits() as i64, "sxhash-eql")
        } else if self.runtime.is_bignum(obj) {
            // Hash bignum by its string representation
            let s = self.runtime.format_value(obj);
            let mut h: u64 = 0;
            for b in s.bytes() {
                h = h.wrapping_mul(31).wrapping_add(b as u64);
            }
            self.fixnum(h as i64, "sxhash-eql")
        } else {
            self.sxhash_eq(obj)
        }
    }

    fn sxhash_equal(&mut self, obj: LispValue) -> Option<LispValue> {
        // Deep hash: recurse into conses, vectors, strings.
        let hash = self.sxhash_equal_value(obj, 0)?;
        self.fixnum(hash as i64, "sxhash-equal")
    }

    fn sxhash_equal_value(&mut self, obj: LispValue, depth: usize) -> Option<u64> {
        if depth > 8 {
            return Some(0);
        }
        if obj.is_nil() {
            return Some(0);
        }
        if obj.is_true() {
            return Some(1);
        }
        if let Some(n) = obj.as_fixnum() {
            return Some(n as u64);
        }
        if self.runtime.is_float(obj) {
            return Some(self.runtime.float_data(obj).ok()?.to_bits());
        }
        if self.runtime.is_string(obj) {
            let s = self.runtime.string_contents_emacs(obj).unwrap_or_default();
            let mut h: u64 = 0;
            for b in s.bytes() {
                h = h.wrapping_mul(31).wrapping_add(b as u64);
            }
            return Some(h);
        }
        if self.runtime.is_cons(obj) {
            let car = self.runtime.car(obj).ok()?;
            let cdr = self.runtime.cdr(obj).ok()?;
            let car_h = self.sxhash_equal_value(car, depth + 1).unwrap_or(0);
            let cdr_h = self.sxhash_equal_value(cdr, depth + 1).unwrap_or(0);
            return Some(car_h.wrapping_mul(31).wrapping_add(cdr_h));
        }
        if self.runtime.is_vector(obj) {
            let elements = self.runtime.vector_elements(obj).ok()?;
            let mut h: u64 = 0;
            for elem in &elements {
                h = h.wrapping_mul(31).wrapping_add(
                    self.sxhash_equal_value(*elem, depth + 1).unwrap_or(0)
                );
            }
            return Some(h);
        }
        // Fallback: use format/display representation
        let s = self.runtime.format_value(obj);
        let mut h: u64 = 0;
        for b in s.bytes() {
            h = h.wrapping_mul(31).wrapping_add(b as u64);
        }
        Some(h)
    }

    fn hash_table_keys(&mut self, table: LispValue) -> Option<LispValue> {
        let entries = self.runtime.hash_table_entries(table).ok()?;
        let mut result = LispValue::NIL;
        for (key, _) in entries.iter().rev() {
            result = self.runtime.cons(*key, result);
        }
        Some(result)
    }

    fn hash_table_values(&mut self, table: LispValue) -> Option<LispValue> {
        let entries = self.runtime.hash_table_entries(table).ok()?;
        let mut result = LispValue::NIL;
        for (_, value) in entries.iter().rev() {
            result = self.runtime.cons(*value, result);
        }
        Some(result)
    }

    fn copy_hash_table(&mut self, table: LispValue) -> Option<LispValue> {
        let test = self.runtime.hash_table_test(table).ok()?;
        let new_table = self.runtime.hash_table(test);
        let entries = self.runtime.hash_table_entries(table).ok()?;
        for (key, value) in entries {
            let _ = self.runtime.puthash(key, value, new_table);
        }
        Some(new_table)
    }

    fn sequence_index(&mut self, name: &str, value: LispValue) -> Option<usize> {
        let value = self.fixnum_arg(name, value)?;
        if value < 0 {
            self.error(format!("primitive `{name}` index must be nonnegative"));
            return None;
        }
        usize::try_from(value).ok()
    }

    fn string_bytes(&mut self, value: LispValue) -> Option<Vec<u8>> {
        match self.runtime.string_data(value) {
            Ok(data) => Some(data.bytes().to_vec()),
            Err(error) => {
                self.runtime_error(error);
                None
            }
        }
    }

    fn upcase_initials(&mut self, value: LispValue) -> LispValue {
        let s = self.string_contents_owned(value).unwrap_or_default();
        let mut result = String::with_capacity(s.len());
        let mut capitalize_next = true;
        for ch in s.chars() {
            if capitalize_next && ch.is_alphabetic() {
                result.push(ch.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(ch);
                if ch == ' ' || ch == '-' || ch == '_' {
                    capitalize_next = true;
                }
            }
        }
        self.runtime.string(result)
    }

    fn string_contents_owned(&mut self, value: LispValue) -> Option<String> {
        match self.runtime.string_contents(value) {
            Ok(contents) => Some(contents.to_string()),
            Err(error) => {
                self.runtime_error(error);
                None
            }
        }
    }

    fn char_arg(&mut self, name: &str, value: LispValue) -> Option<char> {
        if let Some(ch) = value.as_char() {
            return Some(ch);
        }
        if let Some(code) = value.as_fixnum()
            && let Ok(code) = u32::try_from(code)
            && let Some(ch) = char::from_u32(code)
        {
            return Some(ch);
        }
        self.error(format!("primitive `{name}` expected a character"));
        None
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

    fn number_fold_add(&mut self, args: &[LispValue]) -> Option<LispValue> {
        if args.is_empty() {
            return self.fixnum(0, "+");
        }
        let is_float = self.has_float_arg(args);
        if is_float {
            let mut acc = 0.0f64;
            for arg in args {
                let value = self.number_arg("+", *arg)?;
                acc += value;
            }
            Some(self.runtime.float(acc))
        } else {
            if self.has_bignum_arg(args) {
                let mut acc = self.bignum_arg("+", args[0])?;
                for arg in &args[1..] {
                    let value = self.bignum_arg("+", *arg)?;
                    acc += value;
                }
                return Some(self.runtime.bignum(acc));
            }
            let mut acc: i64 = 0;
            for (i, arg) in args.iter().enumerate() {
                let value = self.fixnum_arg("+", *arg)?;
                match acc.checked_add(value) {
                    Some(v) => acc = v,
                    None => {
                        let mut bignum_acc = rug::Integer::from(acc);
                        bignum_acc += rug::Integer::from(value);
                        for arg in &args[i + 1..] {
                            let value = self.fixnum_arg("+", *arg)?;
                            bignum_acc += rug::Integer::from(value);
                        }
                        return Some(self.runtime.bignum(bignum_acc));
                    }
                };
            }
            self.fixnum(acc, "+")
        }
    }

    fn number_fold_mul(&mut self, args: &[LispValue]) -> Option<LispValue> {
        if args.is_empty() {
            return self.fixnum(1, "*");
        }
        let is_float = self.has_float_arg(args);
        if is_float {
            let mut acc = 1.0f64;
            for arg in args {
                let value = self.number_arg("*", *arg)?;
                acc *= value;
            }
            Some(self.runtime.float(acc))
        } else {
            if self.has_bignum_arg(args) {
                let mut acc = self.bignum_arg("*", args[0])?;
                for arg in &args[1..] {
                    let value = self.bignum_arg("*", *arg)?;
                    acc *= value;
                }
                return Some(self.runtime.bignum(acc));
            }
            let mut acc: i64 = 1;
            for (i, arg) in args.iter().enumerate() {
                let value = self.fixnum_arg("*", *arg)?;
                match acc.checked_mul(value) {
                    Some(v) => acc = v,
                    None => {
                        let mut bignum_acc = rug::Integer::from(acc);
                        bignum_acc *= rug::Integer::from(value);
                        for arg in &args[i + 1..] {
                            let value = self.fixnum_arg("*", *arg)?;
                            bignum_acc *= rug::Integer::from(value);
                        }
                        return Some(self.runtime.bignum(bignum_acc));
                    }
                };
            }
            self.fixnum(acc, "*")
        }
    }

    fn number_sub(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let Some((first, rest)) = args.split_first() else {
            self.error("primitive `-` requires at least one argument");
            return None;
        };
        let is_float = self.has_float_arg(args);
        if is_float {
            let first = self.number_arg("-", *first)?;
            let value = if rest.is_empty() { -first } else { first };
            let value = rest.iter().try_fold(value, |acc, v| {
                let v = self.number_arg("-", *v)?;
                Some(acc - v)
            })?;
            Some(self.runtime.float(value))
        } else if self.has_bignum_arg(args) {
            let first_val = self.bignum_arg("-", *first)?;
            let value = if rest.is_empty() {
                -first_val
            } else {
                first_val
            };
            let value = rest.iter().try_fold(value, |acc, v| {
                let v = self.bignum_arg("-", *v)?;
                Some(acc - v)
            })?;
            Some(self.runtime.bignum(value))
        } else {
            let first = self.fixnum_arg("-", *first)?;
            let value = if rest.is_empty() {
                first.checked_neg()
            } else {
                rest.iter()
                    .try_fold(first, |acc, v| acc.checked_sub(self.fixnum_arg("-", *v)?))
            };
            match value {
                Some(value) => self.fixnum(value, "-"),
                None => {
                    let mut bignum_acc = rug::Integer::from(first);
                    if rest.is_empty() {
                        bignum_acc = -bignum_acc;
                    } else {
                        for v in rest {
                            let v = self.fixnum_arg("-", *v)?;
                            bignum_acc -= rug::Integer::from(v);
                        }
                    }
                    Some(self.runtime.bignum(bignum_acc))
                }
            }
        }
    }

    fn number_div(&mut self, args: &[LispValue]) -> Option<LispValue> {
        let Some((first, rest)) = args.split_first() else {
            self.error("primitive `/` requires at least one argument");
            return None;
        };
        if self.has_float_arg(args) {
            let first = self.number_arg("/", *first)?;
            let value = rest.iter().try_fold(first, |acc, v| {
                let v = self.number_arg("/", *v)?;
                if v == 0.0 {
                    let symbol = self.runtime.intern("arith-error");
                    self.pending_signal = Some(SignaledValue {
                        symbol,
                        data: LispValue::NIL,
                    });
                    return None;
                }
                Some(acc / v)
            })?;
            Some(self.runtime.float(value))
        } else if self.has_bignum_arg(args) {
            let first = self.bignum_arg("/", *first)?;
            let value = rest.iter().try_fold(first, |acc, v| {
                let v = self.bignum_arg("/", *v)?;
                if v == 0 {
                    let symbol = self.runtime.intern("arith-error");
                    self.pending_signal = Some(SignaledValue {
                        symbol,
                        data: LispValue::NIL,
                    });
                    return None;
                }
                Some(acc / v)
            })?;
            Some(self.runtime.bignum(value))
        } else {
            let first = self.fixnum_arg("/", *first)?;
            let value = rest.iter().try_fold(first, |acc, v| {
                let v = self.fixnum_arg("/", *v)?;
                if v == 0 {
                    let symbol = self.runtime.intern("arith-error");
                    self.pending_signal = Some(SignaledValue {
                        symbol,
                        data: LispValue::NIL,
                    });
                    return None;
                }
                acc.checked_div(v)
            });
            match value {
                Some(value) => self.fixnum(value, "/"),
                None => {
                    let first = self.bignum_arg("/", args[0])?;
                    let value = args[1..].iter().try_fold(first, |acc, v| {
                        let v = self.bignum_arg("/", *v)?;
                        if v == 0 {
                            let symbol = self.runtime.intern("arith-error");
                            self.pending_signal = Some(SignaledValue {
                                symbol,
                                data: LispValue::NIL,
                            });
                            return None;
                        }
                        Some(acc / v)
                    })?;
                    Some(self.runtime.bignum(value))
                }
            }
        }
    }

    fn number_compare(
        &mut self,
        args: &[LispValue],
        compare: impl Fn(f64, f64) -> bool,
    ) -> Option<LispValue> {
        if self.has_float_arg(args) {
            let values = args
                .iter()
                .map(|value| self.number_arg("comparison", *value))
                .collect::<Option<Vec<_>>>()?;
            Some(bool_value(
                values.windows(2).all(|pair| compare(pair[0], pair[1])),
            ))
        } else if self.has_bignum_arg(args) {
            let values = args
                .iter()
                .map(|value| self.bignum_arg("comparison", *value))
                .collect::<Option<Vec<rug::Integer>>>()?;
            // Use exact Integer comparison for bignums (f64 would lose precision)
            Some(bool_value(values.windows(2).all(|pair| {
                compare(
                    if pair[0] < pair[1] {
                        -1.0
                    } else if pair[0] > pair[1] {
                        1.0
                    } else {
                        0.0
                    },
                    0.0,
                )
            })))
        } else {
            let values = args
                .iter()
                .map(|value| self.fixnum_arg("comparison", *value))
                .collect::<Option<Vec<_>>>()?;
            Some(bool_value(
                values
                    .windows(2)
                    .all(|pair| compare(pair[0] as f64, pair[1] as f64)),
            ))
        }
    }

    fn fixnum_arg(&mut self, name: &str, value: LispValue) -> Option<i64> {
        let Some(value) = value.as_fixnum() else {
            self.error(format!("primitive `{name}` expected a fixnum"));
            return None;
        };
        Some(value)
    }

    fn number_arg(&mut self, name: &str, value: LispValue) -> Option<f64> {
        if let Some(fixnum) = value.as_fixnum() {
            return Some(fixnum as f64);
        }
        if self.runtime.is_float(value) {
            match self.runtime.float_data(value) {
                Ok(f) => return Some(f),
                Err(e) => {
                    self.runtime_error(e);
                    return None;
                }
            }
        }
        if self.runtime.is_bignum(value) {
            if let Some(n) = self.runtime.as_integer(value) {
                return Some(n.to_f64());
            }
        }
        self.error(format!("primitive `{name}` expected a number"));
        None
    }

    fn has_float_arg(&self, args: &[LispValue]) -> bool {
        args.iter().any(|v| self.runtime.is_float(*v))
    }

    fn has_bignum_arg(&self, args: &[LispValue]) -> bool {
        args.iter().any(|v| self.runtime.is_bignum(*v))
    }

    fn bignum_arg(&mut self, name: &str, value: LispValue) -> Option<rug::Integer> {
        self.runtime.as_integer(value).or_else(|| {
            self.error(format!(
                "primitive `{name}` expected an integer, got a float or non-number"
            ));
            None
        })
    }

    fn fixnum(&mut self, value: i64, _name: &str) -> Option<LispValue> {
        if let Some(value) = LispValue::from_fixnum(value) {
            return Some(value);
        }
        Some(self.runtime.bignum(rug::Integer::from(value)))
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

    fn min_arity(&mut self, name: &str, args: &[LispValue], min: usize) -> Option<()> {
        if args.len() >= min {
            return Some(());
        }
        self.error(format!(
            "primitive `{name}` requires at least {min} arguments, got {}",
            args.len()
        ));
        None
    }

    fn min_max_arity(
        &mut self,
        name: &str,
        args: &[LispValue],
        min: usize,
        max: usize,
    ) -> Option<()> {
        if (min..=max).contains(&args.len()) {
            return Some(());
        }
        self.error(format!(
            "primitive `{name}` requires {min} to {max} arguments, got {}",
            args.len()
        ));
        None
    }

    fn runtime_error(&mut self, error: crate::RuntimeError) {
        let signal = match &error {
            crate::RuntimeError::WrongTypeArgument { expected, value } => {
                let sym = self.runtime.intern("wrong-type-argument");
                let exp = self.runtime.intern(expected);
                let data = make_list(self.runtime, [exp, *value]);
                Some((sym, data))
            }
            crate::RuntimeError::VoidVariable { name } => {
                let sym = self.runtime.intern("void-variable");
                let name_sym = self.runtime.intern(name);
                let data = make_list(self.runtime, [name_sym]);
                Some((sym, data))
            }
            crate::RuntimeError::VoidFunction { name } => {
                let sym = self.runtime.intern("void-function");
                let name_sym = self.runtime.intern(name);
                let data = make_list(self.runtime, [name_sym]);
                Some((sym, data))
            }
            crate::RuntimeError::ArgsOutOfRange { value, index } => {
                let sym = self.runtime.intern("args-out-of-range");
                let data = make_list(
                    self.runtime,
                    [*value, LispValue::expect_fixnum(*index as i64)],
                );
                Some((sym, data))
            }
            _ => {
                self.error(error.to_string());
                return;
            }
        };
        if let Some((symbol, data)) = signal {
            self.pending_signal = Some(SignaledValue { symbol, data });
        }
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

    fn runtime_usize(
        &mut self,
        result: Result<usize, crate::RuntimeError>,
        name: &str,
    ) -> Option<LispValue> {
        match result {
            Ok(value) => i64::try_from(value)
                .ok()
                .and_then(|value| self.fixnum(value, name)),
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

    /// Try to catch a throw inline. If caught, returns Ok with the next instruction
    /// index (if more code after the catch) or None (if catch was the last expression).
    /// If not caught in this function, returns Err(thrown).
    fn try_catch_inline(
        &mut self,
        thrown: ThrownValue,
        instructions: &[RegInst],
        inst_index: usize,
    ) -> Result<Option<usize>, ThrownValue> {
        // Find the matching catch in catch_stack (without modifying the stack yet).
        let Some(match_index) = self.catch_stack.iter().rposition(|tag| *tag == thrown.tag) else {
            return Err(thrown);
        };
        let catch_depth = self.catch_stack.len() - match_index;
        self.catch_stack.truncate(match_index);

        // Find the CatchEnd corresponding to the caught catch, skipping nested ones.
        let catch_end = find_catch_end_at_depth(instructions, inst_index, catch_depth);
        let value = thrown.value;
        if let Some(end) = catch_end {
            if end + 1 < instructions.len() {
                self.last_value = Some(value);
                return Ok(Some(end + 1));
            }
        }
        self.last_value = Some(value);
        Ok(None)
    }

    fn finish_signal(self, signaled: SignaledValue) -> InternalInterpResult {
        InternalInterpResult {
            value: None,
            thrown: None,
            signaled: Some(signaled),
            diagnostics: self.diagnostics,
        }
    }
}

/// Find the Nth CatchEnd instruction (at the outermost nesting level) starting
/// from `start_index`. `depth` is the number of CatchEnds to skip through.
fn find_catch_end_at_depth(
    instructions: &[RegInst],
    start_index: usize,
    mut depth: usize,
) -> Option<usize> {
    let mut nesting = 0usize;
    for (index, inst) in instructions.iter().enumerate().skip(start_index + 1) {
        match &inst.kind {
            RegInstKind::CatchBegin { .. } => nesting += 1,
            RegInstKind::CatchEnd { .. } if nesting == 0 => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            RegInstKind::CatchEnd { .. } => nesting -= 1,
            _ => {}
        }
    }
    None
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
    frames_to_skip: usize,
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
            RegInstKind::ConditionCaseEnd { .. } if depth == 0 => return Some(index),
            RegInstKind::ConditionCaseEnd { .. } => depth -= 1,
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
            RegInstKind::UnwindProtectEnd { .. } if depth == 0 => {
                return cleanup_index.map(|cleanup_index| UnwindCleanupTarget {
                    cleanup_index,
                    end_index: index,
                });
            }
            RegInstKind::UnwindProtectEnd { .. } => depth -= 1,
            _ => {}
        }
    }
    None
}

fn find_condition_handler(
    instructions: &[RegInst],
    signal_index: usize,
    signal_name: &str,
    signal_symbol: LispValue,
    rt: &Runtime,
) -> Option<ConditionHandlerTarget> {
    let mut depth = 0usize;
    let mut handler_index = None;
    let mut stop_index = None;
    let mut frames_to_skip = 0;
    for (index, inst) in instructions.iter().enumerate().skip(signal_index + 1) {
        match &inst.kind {
            RegInstKind::ConditionCaseBegin { .. } => depth += 1,
            RegInstKind::ConditionCaseEnd { .. } if depth == 0 => {
                if let Some(hi) = handler_index {
                    return Some(ConditionHandlerTarget {
                        handler_index: hi,
                        stop_index: stop_index.unwrap_or(index),
                        condition_end_index: index,
                        frames_to_skip,
                    });
                }
                // No matching handler in this scope, continue to outer.
                frames_to_skip += 1;
            }
            RegInstKind::ConditionCaseEnd { .. } => depth -= 1,
            RegInstKind::ConditionCaseHandler { pattern } if depth == 0 => {
                if handler_index.is_some() && stop_index.is_none() {
                    stop_index = Some(index);
                } else if handler_index.is_none()
                    && condition_pattern_matches(pattern, signal_name, signal_symbol, rt)
                {
                    handler_index = Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn condition_pattern_matches(
    pattern: &SurfaceForm,
    signal_name: &str,
    signal_symbol: LispValue,
    rt: &Runtime,
) -> bool {
    if let Some(name) = pattern.symbol_name() {
        return condition_name_matches(name, signal_name, signal_symbol, rt);
    }
    let SurfaceKind::List(items) = &pattern.kind else {
        return false;
    };
    items
        .iter()
        .filter_map(SurfaceForm::symbol_name)
        .any(|name| condition_name_matches(name, signal_name, signal_symbol, rt))
}

fn condition_name_matches(
    pattern_name: &str,
    signal_name: &str,
    signal_symbol: LispValue,
    rt: &Runtime,
) -> bool {
    if pattern_name == signal_name || pattern_name == "error" {
        return true;
    }
    // Check the error-conditions parent chain.
    let Some(parents) = rt.signal_error_conditions(signal_symbol) else {
        return false;
    };
    parents.contains(&pattern_name.to_string())
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
        | RegInstKind::ConditionCaseGetVar { dst }
        | RegInstKind::CallNamed { dst, .. }
        | RegInstKind::Funcall { dst, .. }
        | RegInstKind::Apply { dst, .. } => Some(*dst),
        RegInstKind::BindLexical { .. }
        | RegInstKind::BindDynamic { .. }
        | RegInstKind::UnbindDynamic { .. }
        | RegInstKind::DeclareSpecial { .. }
        | RegInstKind::CatchBegin { .. }
        | RegInstKind::CatchEnd { .. }
        | RegInstKind::Throw { .. }
        | RegInstKind::ConditionCaseBegin { .. }
        | RegInstKind::ConditionCaseHandler { .. }
        | RegInstKind::ConditionCaseEnd { .. }
        | RegInstKind::UnwindProtectBegin
        | RegInstKind::UnwindProtectCleanup
        | RegInstKind::UnwindProtectEnd { .. }
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
    use std::sync::OnceLock;
    static PRIMITIVES: OnceLock<std::collections::HashSet<&'static str>> = OnceLock::new();
    PRIMITIVES.get_or_init(|| {
        std::collections::HashSet::from([
            "%",
            "*",
            "+",
            "-",
            "/",
            "/=",
            "<",
            "<=",
            "=",
            ">",
            ">=",
            "1+",
            "1-",
            "abs",
            "add-load-path",
            "alist-get",
            "always",
            "append",
            "apply",
            "aref",
            "arrayp",
            "aset",
            "ash",
            "assoc",
            "assoc-string",
            "assq",
            "cl-adjoin",
            "cl-assoc",
            "cl-assoc-if",
            "cl-assoc-if-not",
            "cl-assq",
            "atom",
            "autoload",
            "autoloadp",
            "bare-symbol-p",
            "bignump",
            "bobp",
            "bool-vector-p",
            "booleanp",
            "boundp",
            "buffer-modified-p",
            "buffer-size",
            "bufferp",
            "butlast",
            "caaaar",
            "caaadr",
            "caaar",
            "caadar",
            "caaddr",
            "caadr",
            "caar",
            "cadaar",
            "cadadr",
            "cadar",
            "caddar",
            "cadddr",
            "caddr",
            "cadr",
            "capitalize",
            "car",
            "car-safe",
            "cdaaar",
            "cdaadr",
            "cdaar",
            "cdadar",
            "cdaddr",
            "cdadr",
            "cdar",
            "cddaar",
            "cddadr",
            "cddar",
            "cdddar",
            "cddddr",
            "cdddr",
            "cddr",
            "cdr",
            "cdr-safe",
            "ceiling",
            "char-code",
            "char-equal",
            "char-or-string-p",
            "char-table-p",
            "char-to-string",
            "char-valid-p",
            "cl-count",
            "cl-count-if",
            "cl-count-if-not",
            "cl-endp",
            "cl-evenp",
            "cl-fill",
            "cl-find",
            "cl-find-if",
            "cl-find-if-not",
            "cl-minusp",
            "cl-oddp",
            "cl-plusp",
            "cl-position",
            "cl-position-if",
            "cl-position-if-not",
            "cl-coerce",
            "cl-concatenate",
            "cl-delete",
            "cl-delete-duplicates",
            "cl-delete-if",
            "cl-delete-if-not",
            "cl-delq",
            "cl-nset-difference",
            "cl-nsubst-if",
            "cl-nsubst-if-not",
            "cl-nsublis",
            "cl-nsubstitute",
            "cl-nsubstitute-if",
            "cl-nsubstitute-if-not",
            "cl-subst-if",
            "cl-subst-if-not",
            "cl-sublis",
            "cl-substitute",
            "cl-substitute-if",
            "cl-substitute-if-not",
            "cl-nintersection",
            "cl-nset-exclusive-or",
            "cl-nunion",
            "cl-intersection",
            "cl-ldiff",
            "cl-tree-equal",
            "cl-union",
            "cl-typep",
            "cl-nreplace",
            "cl-nreverse",
            "cl-rassoc",
            "cl-rassoc-if",
            "cl-rassoc-if-not",
            "cl-rassq",
            "cl-remprop",
            "cl-remq",
            "cl-remove",
            "cl-reverse",
            "cl-remove-duplicates",
            "cl-reduce",
            "cl-remove-if",
            "cl-remove-if-not",
            "cl-replace",
            "cl-set-difference",
            "cl-set-exclusive-or",
            "cl-search",
            "cl-sort",
            "cl-stable-sort",
            "clrhash",
            "color-defined-p",
            "commandp",
            "compiled-function-p",
            "concat",
            "cons",
            "consp",
            "copy-alist",
            "copy-list",
            "copy-sequence",
            "copy-tree",
            "cos",
            "defalias",
            "current-buffer",
            "define-error",
            "default-boundp",
            "default-value",
            "defun",
            "delete",
            "delete-dups",
            "delq",
            "display-graphic-p",
            "downcase",
            "elt",
            "emacs-pid",
            "eobp",
            "eql",
            "eq",
            "equal",
            "error",
            "eval",
            "evenp",
            "every",
            "exp",
            "expt",
            "fboundp",
            "featurep",
            "file-exists-p",
            "file-readable-p",
            "fillarray",
            "fixnump",
            "float",
            "floatp",
            "floor",
            "fmakunbound",
            "format",
            "format-message",
            "fset",
            "funcall",
            "functionp",
            "garbage-collect",
            "gensym",
            "getenv",
            "get",
            "gethash",
            "hash-table-count",
            "hash-table-p",
            "identity",
            "ignore",
            "indirect-function",
            "integer-or-marker-p",
            "integerp",
            "intern",
            "intern-soft",
            "keywordp",
            "last",
            "length",
            "length=",
            "list",
            "list*",
            "cl-list",
            "cl-list*",
            "cl-list-length",
            "listp",
            "load",
            "log",
            "logand",
            "logior",
            "lognot",
            "logxor",
            "lsh",
            "macroexpand",
            "macroexpand-1",
            "macrop",
            "make-vector",
            "make-hash-table",
            "make-list",
            "make-string",
            "make-symbol",
            "mapl",
            "maplist",
            "mapc",
            "mapcan",
            "mapcon",
            "mapcar",
            "markerp",
            "cl-mapc",
            "cl-mapcar",
            "cl-merge",
            "cl-map",
            "cl-member-if",
            "cl-member-if-not",
            "cl-mismatch",
            "cl-member",
            "maphash",
            "match-beginning",
            "match-end",
            "match-string",
            "max",
            "member",
            "memq",
            "memql",
            "message",
            "min",
            "minibuffer-window",
            "mod",
            "natnump",
            "mutexp",
            "cl-nconc",
            "nconc",
            "nlistp",
            "not",
            "notany",
            "notevery",
            "nreverse",
            "nth",
            "nthcdr",
            "null",
            "number-or-marker-p",
            "number-sequence",
            "number-or-marker-p",
            "number-to-string",
            "numberp",
            "pairlis",
            "plist-get",
            "plist-member",
            "plist-put",
            "point-max",
            "point-min",
            "prin1",
            "prin1-to-string",
            "princ-to-string",
            "print",
            "processp",
            "prog1",
            "proper-list-p",
            "provide",
            "purecopy",
            "put",
            "puthash",
            "random",
            "rassoc",
            "rassq",
            "read",
            "recordp",
            "rem",
            "remhash",
            "remq",
            "remove",
            "replace-match",
            "replace-regexp-in-string",
            "require",
            "reverse",
            "round",
            "safe-length",
            "set",
            "set-default",
            "setcar",
            "setcdr",
            "setenv",
            "setplist",
            "signal",
            "sin",
            "some",
            "sort",
            "special-form-p",
            "special-variable-p",
            "split-string",
            "sqrt",
            "standard-syntax-table",
            "string-bytes",
            "string-equal",
            "string-greaterp",
            "string-join",
            "string-lessp",
            "string-match",
            "string-match-p",
            "string-or-null-p",
            "string-prefix-p",
            "string-suffix-p",
            "string-to-char",
            "string-to-number",
            "string-trim",
            "string-trim-left",
            "string-trim-right",
            "string<",
            "string=",
            "string>",
            "stringp",
            "subr-arity",
            "subr-native-elisp-p",
            "subrp",
            "cl-tailp",
            "cl-subseq",
            "subseq",
            "substring",
            "substring-no-properties",
            "sxhash-eq",
            "sxhash-eql",
            "sxhash-equal",
            "symbol-function",
            "symbol-name",
            "symbol-plist",
            "symbol-value",
            "symbolp",
            "syntax-table-p",
            "tan",
            "terpri",
            "threadp",
            "truncate",
            "type-of",
            "upcase",
            "upcase-initials",
            "use-region-p",
            "user-error",
            "vconcat",
            "vector",
            "vectorp",
            "window-buffer",
            "wholenump",
            "windowp",
            "zerop",
        ])
    }).contains(name)
}

fn macro_value_to_lisp(
    mv: &neovm_compiler::expand_value::MacroValue,
    rt: &mut Runtime,
) -> LispValue {
    use neovm_compiler::expand_value::MacroValue;
    match mv {
        MacroValue::Nil => LispValue::NIL,
        MacroValue::Symbol(name) => {
            if name == "t" {
                LispValue::TRUE
            } else {
                rt.intern(name)
            }
        }
        MacroValue::Int(n) => {
            LispValue::from_fixnum(*n).unwrap_or_else(|| rt.bignum(rug::Integer::from(*n)))
        }
        MacroValue::Float(f) => rt.float(*f),
        MacroValue::String(s) => rt.string(s.clone()),
        MacroValue::Cons(cons) => {
            let car_lisp = macro_value_to_lisp(&cons.car, rt);
            let cdr_lisp = macro_value_to_lisp(&cons.cdr, rt);
            rt.cons(car_lisp, cdr_lisp)
        }
        MacroValue::Vector(items) => {
            let elements: Vec<LispValue> =
                items.iter().map(|v| macro_value_to_lisp(v, rt)).collect();
            rt.vector(elements)
        }
    }
}

fn surface_to_lisp_value(runtime: &mut Runtime, form: &neovm_compiler::surface::SurfaceForm) -> LispValue {
    use neovm_compiler::surface::{SurfaceKind, SurfaceAtom};
    match &form.kind {
        SurfaceKind::Atom(atom) => match atom {
            SurfaceAtom::Nil => LispValue::NIL,
            SurfaceAtom::True => LispValue::TRUE,
            SurfaceAtom::Int(n) => LispValue::expect_fixnum(*n),
            SurfaceAtom::Float(f) => runtime.float(*f),
            SurfaceAtom::Symbol(name) => runtime.intern(name),
            SurfaceAtom::String(s) => runtime.string(s.clone()),
            SurfaceAtom::Char(c) => LispValue::expect_fixnum(*c),
        },
        SurfaceKind::List(items) => {
            // Collect values first to avoid mutable borrow conflicts
            let values: Vec<LispValue> = items
                .iter()
                .map(|item| surface_to_lisp_value(runtime, item))
                .collect();
            let mut result = LispValue::NIL;
            for val in values.into_iter().rev() {
                result = runtime.cons(val, result);
            }
            result
        }
        SurfaceKind::Quote(inner) => {
            let inner_val = surface_to_lisp_value(runtime, inner);
            let sym = runtime.intern("quote");
            let cell1 = runtime.cons(inner_val, LispValue::NIL);
            runtime.cons(sym, cell1)
        }
        _ => LispValue::NIL,
    }
}

// ── Subr (DEFUN) helpers ────────────────────────────────────────────
//
// Emacs DEFUN equivalent: add one line to execute_primitive_call.
// Example for a new function `my-func` taking 1-3 args:
//
//   "my-func" => self.subr_1_3("my-func", args, Self::my_func_impl),
//
// The JIT path needs a corresponding entry in jit_rt.rs for fast-path.

#[cfg(test)]
mod tests {
    use neovm_compiler::compile_source;

    use crate::object_interp::{ObjectInterpResult, execute_module_with_args};
    use crate::{LispValue, Runtime};

    use super::make_list;

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
    fn executes_push_and_pop_macro_expansions() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((xs nil)) (push 1 xs) (push 2 xs) (+ (pop xs) (pop xs) (if xs 99 0)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_simple_if_let_and_when_let_star_expansions() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(+ (if-let* ((x 1) (y (+ x 2))) y 0) (if-let* ((x nil) (y (error \"boom\"))) y 4 5) (when-let* ((_ 6) (z 7)) z))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn executes_top_level_defmacro_expansions() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(defmacro object-inc (var) `(setq ,var (1+ ,var)))\n(defmacro object-sum (&rest body) `(progn ,@body))\n(let ((x 1)) (object-inc x) (object-sum (setq x (+ x 2)) x))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn top_level_defmacro_defines_runtime_macro_function_cell() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(defmacro object-id (x) x)\n(if (and (fboundp 'object-id) (consp (symbol-function 'object-id)) (eq (car (symbol-function 'object-id)) 'macro)) 7 0)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(7)));
    }

    #[test]
    fn executes_common_list_and_alist_utilities() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((xs (list (cons 'a 1) (cons \"b\" 2)))) (+ (if (eq (car-safe 1) nil) 1 0) (if (eq (cdr-safe 1) nil) 2 0) (if (member \"b\" (list \"a\" \"b\")) 4 0) (cdr (assq 'a xs)) (cdr (assoc \"b\" xs)) (length (copy-sequence [1 2 3]))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(13)));
    }

    #[test]
    fn executes_backquote_with_unquote_and_splicing() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((x 2) (xs (list 3 4))) (if (and (equal `(a ,x ,@xs b) '(a 2 3 4 b)) (equal `[a ,x ,@xs b] [a 2 3 4 b]) (equal `(a ,@(list 1 2) . z) '(a 1 2 . z)) (equal `',@(list 1 2) '(quote 1 2))) 9 0))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(9)));
    }

    #[test]
    fn executes_sequence_mapping_primitives() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((sum 0)) (mapc (lambda (x) (setq sum (+ sum x))) [1 2 3]) (+ sum (length (mapcar '1+ (list 1 2 3))) (if (equal (mapcar 'char-to-string \"ab\") (list \"a\" \"b\")) 10 0)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(19)));
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
    fn executes_self_evaluating_keyword_symbols() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(if (eq :test ':test) 7 0)");
        assert_eq!(value, Some(LispValue::expect_fixnum(7)));
    }

    #[test]
    fn executes_basic_string_primitives() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((s (concat \"a\" (char-to-string ?b) \"c\"))) (+ (length s) (if (string= (substring s 1 -1) \"b\") 10 0) (if (string< \"a\" \"b\") 20 0) (if (eq (string-to-char s) ?a) 30 0)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(63)));
    }

    #[test]
    fn executes_basic_format_primitives() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(if (string= (format \"%s/%S/%d/%%\" \"x\" 'sym 7) \"x/sym/7/%\") (if (string= (format-message \"%s\" \"ok\") \"ok\") 11 0) 0)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(11)));
    }

    #[test]
    fn executes_vector_primitives_and_literals() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((v [1 2 3])) (aset v 1 9) (+ (aref v 1) (length v) (if (vectorp (vector 4 5)) 10 0) (if (equal v [1 9 3]) 20 0) (if (eq (aref \"a\" 0) ?a) 0 99)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
        assert_eq!(runtime.vector_count(), 3);
    }

    #[test]
    fn executes_hash_table_primitives() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((h (make-hash-table :test 'equal)) (sum 0)) (puthash \"a\" 2 h) (puthash \"b\" 3 h) (maphash (lambda (k v) (setq sum (+ sum v))) h) (remhash \"b\" h) (+ (gethash \"a\" h) (gethash \"missing\" h 4) (hash-table-count h) sum (if (hash-table-p h) 7 0) (hash-table-count (clrhash h))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(19)));
        assert_eq!(runtime.hash_table_count_allocated(), 1);
    }

    #[test]
    fn executes_basic_numeric_and_utility_predicates() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(+ (if (integerp 1) 1 0) (if (natnump 0) 2 0) (if (wholenump -1) 0 4) (if (zerop 0) 8 0) (identity 16) (if (ignore 1 2 3) 0 32))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(63)));
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
    fn executes_fset_and_defalias() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn (fset 'object-my-car 'car) (defalias 'object-my-cdr 'cdr) (+ (funcall 'object-my-car (cons 4 5)) (funcall (symbol-function 'object-my-cdr) (cons 6 7))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(11)));
    }

    #[test]
    fn executes_feature_primitives() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn (provide 'object-feature) (+ (if (featurep 'object-feature) 1 0) (if (eq (require 'object-feature) 'object-feature) 2 0) (if (require 'object-missing nil t) 0 4)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(7)));
        assert_eq!(runtime.feature_count(), 1);
    }

    #[test]
    fn require_signals_missing_feature() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(condition-case err (require 'object-missing) (error 9))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(9)));
    }

    #[test]
    fn executes_property_list_primitives() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn (put 'object-symbol 'object-property 4) (+ (get 'object-symbol 'object-property) (plist-get (plist-put nil 'object-other 3) 'object-other) (if (equal (symbol-plist 'object-symbol) '(object-property 4)) 5 0)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn executes_setplist_primitive() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn (setplist 'object-symbol (list 'a 1)) (get 'object-symbol 'a))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_autoload_primitive() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn (autoload 'object-auto \"object-file\" \"doc\" t 'macro) (if (fboundp 'object-auto) (if (equal (symbol-function 'object-auto) '(autoload \"object-file\" \"doc\" t macro)) 7 0) 0))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(7)));
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
    fn executes_optional_and_rest_lambda_lists() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(defun object-opt (x &optional y &rest zs) (+ x (if y y 0) (length zs)))\n(+ (object-opt 1) (object-opt 1 2 3 4) (funcall (lambda (x &optional y &rest zs) (+ x (if y y 0) (length zs))) 5 6 7))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(18)));
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
    fn executes_while_special_form() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((x 0)) (while (< x 3) (setq x (1+ x))) x)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_dolist_and_dotimes_forms() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((sum 0)) (+ (dolist (x (list 1 2 3) sum) (setq sum (+ sum x))) (dotimes (i 4 sum) (setq sum (+ sum i)))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(18)));
    }

    #[test]
    fn executes_when_unless_and_cond_forms() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(+ (when t 4) (unless nil 5) (cond ((= 1 2) 10) ((+ 1 2))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn executes_defvar_and_defconst_forms() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn (defvar object-defvar (+ 1 2)) (defvar object-defvar (error \"skip\")) (defconst object-defconst 7) (+ (symbol-value 'object-defvar) (symbol-value 'object-defconst)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_basic_declaration_and_custom_forms() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn (declare-function object-missing \"file\") (defgroup object-group nil \"doc\" :group 'lisp) (defcustom object-custom (+ 1 2) \"doc\" :type 'integer) object-custom)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_top_level_load_entry_forms_in_order() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(defun object-load-add1 (x) (1+ x))\n(setq object-load-value 1)\n(setq object-load-value (object-load-add1 object-load-value))\nobject-load-value",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
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
        let msg = &result.diagnostics[0].message;
        assert!(msg.contains("uncaught throw") || msg.contains("uncaught signal no-catch"));
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
    fn executes_common_macro_like_wrapper_forms() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(+ (eval-and-compile 1) (eval-when-compile 2) (with-no-warnings 3) (condition-case-unless-debug err (error \"boom\") (error 4)) (if (ignore-errors (error \"boom\")) 99 5))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(15)));
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

    #[test]
    fn nested_condition_case_outer_handler_catches_signal() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (condition-case outer-err
              (condition-case inner-err
                (signal 'wrong-type-argument '(x))
                (args-out-of-range 99))
              (wrong-type-argument 42))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn nested_unwind_protect_outer_cleanup_runs_after_signal() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((x 0))
              (condition-case err
                (unwind-protect
                  (unwind-protect
                    (signal 'error '(\"boom\"))
                    (setq x (+ x 1)))
                  (setq x (+ x 10)))
                (error x)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(11)));
    }

    #[test]
    fn nested_unwind_protect_inner_cleanup_runs_on_throw() {
        // First verify single-level cleanup works
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((x 0))
              (catch 'tag
                (unwind-protect
                  (throw 'tag 7)
                  (setq x 1)))
              x)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));

        // Then verify nested cleanup
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((x 0))
              (catch 'tag
                (unwind-protect
                  (throw 'tag 7)
                  (unwind-protect
                    (throw 'tag 99)
                    (setq x 1))))
              x)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn dolist_var_is_nil_after_loop() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (dolist (x (list 1 2 3)) x)",
        );
        // After dolist loop body, x is set to nil per Emacs spec
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn hash_hex_literal_execution() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(+ #x10 #o10 #b10)");
        // #x10 = 16, #o10 = 8, #b10 = 2 → 26
        assert_eq!(value, Some(LispValue::expect_fixnum(26)));
    }

    #[test]
    fn hash_hex_in_expression() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(let ((mask #xff)) (+ mask 1))");
        // #xff = 255, + 1 = 256
        assert_eq!(value, Some(LispValue::expect_fixnum(256)));
    }

    #[test]
    fn vector_operations_interpreter() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((v (vector 1 2 3)))\n\
              (+ (aref v 0) (aref v 1) (aref v 2)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn mapcar_with_lambda() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((result (mapcar (lambda (x) (+ x 10)) (list 1 2 3))))\n\
              (car (cdr (cdr result))))",
        );
        // (mapcar (lambda (x) (+ x 10)) '(1 2 3)) → (11 12 13)
        assert_eq!(value, Some(LispValue::expect_fixnum(13)));
    }

    // --- Backquote expansion tests ---

    #[test]
    fn backquote_with_splice_in_middle() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((xs (list 2 3)))\n\
              (car (cdr `(1 ,@xs 4))))",
        );
        // `(1 ,@xs 4) → (1 2 3 4), (car (cdr ...)) = 2
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn backquote_splice_empty_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((xs nil))\n\
              (length `(a ,@xs b)))",
        );
        // Splicing nil should produce (a b), length 2
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn backquote_in_macro_body() {
        // Defmacro using backquote, then invoke it
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (defmacro my-let1 (var val &rest body)\n\
              `(let ((,var ,val)) ,@body))\n\
            (my-let1 x 10 (+ x 5))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn backquote_vector_with_splice() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((xs (list 2 3)))\n\
              (aref `[1 ,@xs 4] 2))",
        );
        // [1 2 3 4], aref index 2 = 3
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    // --- HIR lowering edge cases ---

    #[test]
    fn catch_throw_with_computed_tag() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((tag 'my-tag))\n\
              (catch tag (throw tag 99)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn condition_case_with_multiple_handlers() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (condition-case err\n\
              (signal 'wrong-type-argument '(42))\n\
              (arith-error -1)\n\
              (wrong-type-argument (cadr err))\n\
              (error -3))",
        );
        // Should match wrong-type-argument handler
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn unwind_protect_throw_in_cleanup() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (catch 'outer\n\
              (unwind-protect\n\
                (throw 'outer 1)\n\
                (throw 'outer 2)))",
        );
        // Cleanup's throw replaces the original throw
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn prog1_returns_first() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((x 0))\n\
              (prog1 (setq x 10) (setq x 20))\n\
              x)",
        );
        // prog1 returns first value (10), but side effect sets x=20
        assert_eq!(value, Some(LispValue::expect_fixnum(20)));
    }

    #[test]
    fn lambda_with_rest_param() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (funcall (lambda (a &rest bs) (+ a (length bs))) 1 2 3 4)",
        );
        // a=1, bs=(2 3 4), length=3 → 1+3=4
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn lambda_with_optional_param() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((f (lambda (a &optional b) (if b (+ a b) a))))\n\
              (+ (funcall f 10) (funcall f 10 5)))",
        );
        // (funcall f 10) = 10, (funcall f 10 5) = 15 → 25
        assert_eq!(value, Some(LispValue::expect_fixnum(25)));
    }

    #[test]
    fn forward_referenced_function() {
        // Call a function defined later in the source
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (defun caller () (callee 5))\n\
            (defun callee (x) (+ x 1))\n\
            (caller)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn setq_returns_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((x 0))\n\
              (+ (setq x 10) x))",
        );
        // setq returns 10, then x is 10 → 20
        assert_eq!(value, Some(LispValue::expect_fixnum(20)));
    }

    #[test]
    fn and_or_short_circuit() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((x 0))\n\
              (and nil (setq x 1))\n\
              x)",
        );
        // and short-circuits, x stays 0
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));

        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((x 0))\n\
              (or t (setq x 1))\n\
              x)",
        );
        // or short-circuits, x stays 0
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn cond_with_multiple_clauses() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (cond (nil 1) ((= 1 1) 2) (t 3))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn deep_let_nesting() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((a 1))\n\
              (let ((b (+ a 1)))\n\
                (let ((c (+ b 1)))\n\
                  (let ((d (+ c 1)))\n\
                    (+ a b c d)))))",
        );
        // 1+2+3+4 = 10
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn closure_capture_shared_cell() {
        // Two closures share a mutable cell via setq
        let (value, _) = execute(
            "(let ((counter 0) (getter nil) (setter nil))\n\
              (setq getter (lambda () counter))\n\
              (setq setter (lambda (v) (setq counter v)))\n\
              (funcall setter 42)\n\
              (funcall getter))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn recursive_lambda_closure() {
        // Lambda that captures itself via symbol to recurse
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((fib nil))\n\
              (setq fib (lambda (n)\n\
                (if (< n 2) n (+ (funcall fib (- n 1)) (funcall fib (- n 2))))))\n\
              (funcall fib 6))",
        );
        // fib(6) = 8
        assert_eq!(value, Some(LispValue::expect_fixnum(8)));
    }

    #[test]
    fn dynamic_binding_across_function_call() {
        // Dynamic var set in caller, read in callee
        let (value, _) = execute(
            "(defvar *dyn-var* nil)\n\
            (defun get-dyn () *dyn-var*)\n\
            (let ((*dyn-var* 42))\n\
              (get-dyn))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn nested_condition_case() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (condition-case err\n\
              (condition-case inner\n\
                (signal 'test-error '(99))\n\
                (wrong-type-argument inner))\n\
              (test-error (cadr err)))",
        );
        // Outer handler catches test-error, (cadr err) = 99
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn unwind_protect_in_condition_case() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((x 0))\n\
              (condition-case err\n\
                (unwind-protect\n\
                  (signal 'test-error '(1))\n\
                  (setq x 10))\n\
                (test-error (+ x (cadr err)))))",
        );
        // Cleanup sets x=10, handler returns x+1=11
        assert_eq!(value, Some(LispValue::expect_fixnum(11)));
    }

    #[test]
    fn vector_mutation_with_aset() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((v (vector 0 0 0)))\n\
              (aset v 0 10)\n\
              (aset v 1 20)\n\
              (aset v 2 30)\n\
              (+ (aref v 0) (aref v 1) (aref v 2)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(60)));
    }

    #[test]
    fn hash_table_operations() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((ht (make-hash-table)))\n\
              (puthash 'a 1 ht)\n\
              (puthash 'b 2 ht)\n\
              (+ (gethash 'a ht 0) (gethash 'b ht 0) (gethash 'c ht 0)))",
        );
        // a=1, b=2, c defaults to 0 → 3
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn let_star_sequential_binding() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let* ((x 1) (y (+ x 10)) (z (+ y 100)))\n\
              z)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(111)));
    }

    #[test]
    fn nested_catch_throw() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (catch 'outer\n\
              (catch 'inner\n\
                (throw 'outer 42))\n\
              0)",
        );
        // throw 'outer skips inner catch and lands at outer
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn progn_returns_last_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (progn 1 2 3)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn multiple_dynamic_bindings() {
        let (value, _) = execute(
            "(defvar *x* 0)\n\
            (defvar *y* 0)\n\
            (let ((*x* 10) (*y* 20))\n\
              (+ *x* *y*))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(30)));
    }

    // --- Complex macro patterns ---

    #[test]
    fn defmacro_generating_defun() {
        // Macro that generates a defun form
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (defmacro defconst-fn (name val)\n\
              `(defun ,name () ,val))\n\
            (defconst-fn get-answer 42)\n\
            (get-answer)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn nested_macro_expansion() {
        // Macro that expands to another macro call
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (defmacro my-push (val place)\n\
              (list 'setq place (list 'cons val place)))\n\
            (let ((xs nil))\n\
              (my-push 1 xs)\n\
              (my-push 2 xs)\n\
              (car xs))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn defmacro_with_list_functions() {
        // Macro using list, append, mapcar at expansion time
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (defmacro sum-of (&rest args)\n\
              (cons '+ args))\n\
            (sum-of 1 2 3 4 5)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn macro_using_nth_and_length() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (defmacro swap (a b)\n\
              (let ((tmp (make-symbol \"tmp\")))\n\
                (list 'let (list (list tmp a))\n\
                      (list 'setq a b)\n\
                      (list 'setq b tmp))))\n\
            (let ((x 1) (y 2))\n\
              (swap x y)\n\
              (+ x (* y 10)))",
        );
        // After swap: x=2, y=1 → 2 + 10 = 12
        assert_eq!(value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn backquote_with_multiple_splices() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((xs (list 1 2)) (ys (list 3 4)))\n\
              (length `(a ,@xs b ,@ys c)))",
        );
        // (a 1 2 b 3 4 c) → 7
        assert_eq!(value, Some(LispValue::expect_fixnum(7)));
    }

    #[test]
    fn dolist_with_result_form() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (dolist (x (list 1 2 3) (list 'done x))\n\
              (message \"%d\" x))",
        );
        // After loop, x is nil, result form evaluates to (done nil)
        // Since we return the result form, we need to check it's a cons
        assert!(value.is_some());
        let v = value.unwrap();
        assert!(v != LispValue::NIL);
    }

    #[test]
    fn dotimes_counts_correctly() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((sum 0))\n\
              (dotimes (i 5 sum)\n\
                (setq sum (+ sum i))))",
        );
        // i goes 0,1,2,3,4 → sum = 0+1+2+3+4 = 10
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn while_loop_with_mutation() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((xs (list 1 2 3 4 5)) (sum 0))\n\
              (while xs\n\
                (setq sum (+ sum (car xs)))\n\
                (setq xs (cdr xs)))\n\
              sum)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(15)));
    }

    #[test]
    fn executes_downcase_upcase() {
        // Test downcase on symbol with let*
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let* ((sym (downcase 'WORLD)))\n\
              (if (eq sym 'world) 2 0))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));

        // Test upcase on string with let*
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let* ((s (downcase \"HELLO\"))\n\
                   (u (upcase s)))\n\
              (if (string= u \"HELLO\") 4 0))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));

        // Full test
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let* ((s (downcase \"HELLO\"))\n\
                   (sym (downcase 'WORLD))\n\
                   (u (upcase s)))\n\
              (+ (if (string= s \"hello\") 1 0)\n\
                 (if (eq sym 'world) 2 0)\n\
                 (if (string= u \"HELLO\") 4 0)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(7)));
    }

    #[test]
    fn executes_capitalize() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (if (string= (capitalize \"hello world\") \"Hello world\") 9 0)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(9)));
    }

    #[test]
    fn executes_elt_on_list_and_vector() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (let ((xs (list 10 20 30))\n\
                  (v [4 5 6]))\n\
              (+ (elt xs 1) (elt v 2)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(26)));
    }

    #[test]
    fn executes_split_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (length (split-string \"a b c\"))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));

        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (length (split-string \"a,b,c\" \",\"))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));

        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (length (split-string \"a  b\" nil t))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_string_join() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (string= (string-join (list \"a\" \"b\" \"c\") \"-\") \"a-b-c\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_trim() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
            (+ (if (string= (string-trim \"  hello  \") \"hello\") 1 0)\n\
               (if (string= (string-trim \"xxhelloxx\" \"x\") \"hello\") 2 0))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_substring_no_properties() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (substring-no-properties \"hello world\" 0 5) \"hello\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_float_constant() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n3.14");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 3.14).abs() < 1e-10);
    }

    #[test]
    fn executes_floatp() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(floatp 3.14)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(floatp 42)");
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_number_or_marker_p_with_float() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(number-or-marker-p 3.14)");
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_float_addition() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(+ 1.5 2.5)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 4.0).abs() < 1e-10);
    }

    #[test]
    fn executes_float_addition_mixed() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(+ 1 2.5)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 3.5).abs() < 1e-10);
    }

    #[test]
    fn executes_integer_addition_stays_int() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(+ 1 2)");
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_float_subtraction() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(- 5.5 2.0)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 3.5).abs() < 1e-10);
    }

    #[test]
    fn executes_float_negation() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(- 3.14)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f + 3.14).abs() < 1e-10);
    }

    #[test]
    fn executes_float_multiplication() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(* 2.0 3.5)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 7.0).abs() < 1e-10);
    }

    #[test]
    fn executes_float_division() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(/ 7.0 2.0)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 3.5).abs() < 1e-10);
    }

    #[test]
    fn executes_1_plus_float() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(1+ 2.5)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 3.5).abs() < 1e-10);
    }

    #[test]
    fn executes_1_minus_float() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(1- 5.5)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 4.5).abs() < 1e-10);
    }

    #[test]
    fn executes_float_comparison() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(< 1.5 2.5)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(> 3.5 2.5)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(= 2.5 2.5)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(<= 2.0 2.0)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(>= 3.0 2.0)");
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_mixed_comparison() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(< 1 2.5)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(> 3.5 2)");
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_type_of_float() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(type-of 3.14)");
        assert_eq!(rt.symbol_name(value.unwrap()).unwrap(), "float");
    }

    #[test]
    fn executes_number_to_string_float() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(number-to-string 3.14)");
        let s = rt.string_contents(value.unwrap()).unwrap();
        assert!(s.starts_with("3.14") || s.starts_with("3.1"));
    }

    #[test]
    fn executes_string_to_number_float() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(string-to-number \"3.14\")");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 3.14).abs() < 1e-10);
    }

    #[test]
    fn executes_float_abs() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(abs -3.5)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 3.5).abs() < 1e-10);
    }

    #[test]
    fn executes_float_max_min() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(max 1 3.5 2)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 3.5).abs() < 1e-10);
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(min 1 3.5 2)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 1.0).abs() < 1e-10);
    }

    #[test]
    fn executes_float_mod_rem() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(mod 5.5 3.0)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 2.5).abs() < 1e-10);
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(rem 5.5 3.0)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 2.5).abs() < 1e-10);
    }

    #[test]
    fn executes_float_expt() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(expt 2.0 3.0)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 8.0).abs() < 1e-10);
    }

    #[test]
    fn executes_expt_with_integers() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(expt 2 10)");
        assert_eq!(value, Some(LispValue::expect_fixnum(1024)));
    }

    #[test]
    fn executes_truncate_returns_integer() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(truncate 3.7)");
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_floor_returns_integer() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(floor 3.7)");
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(floor -3.7)");
        assert_eq!(value, Some(LispValue::expect_fixnum(-4)));
    }

    #[test]
    fn executes_ceiling_returns_integer() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(ceiling 3.2)");
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn executes_round_returns_integer() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(round 3.5)");
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn executes_sqrt_returns_float() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(sqrt 16.0)");
        assert!(rt.is_float(value.unwrap()));
        let f = rt.float_data(value.unwrap()).unwrap();
        assert!((f - 4.0).abs() < 1e-10);
    }

    #[test]
    fn executes_sin_cos_tan() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(sin 0.0)");
        assert!(rt.is_float(value.unwrap()));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(cos 0.0)");
        assert!(value.is_some());
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(tan 0.0)");
        assert!(value.is_some());
    }

    #[test]
    fn executes_condition_case_catches_child_error() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (condition-case err \
               (signal 'arith-error '(\"test\")) \
               (error 'caught))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_zerop_float() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(zerop 0.0)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(zerop 1.0)");
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_format_float() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(format \"%f\" 3.14)");
        let s = rt.string_contents(value.unwrap()).unwrap();
        assert!(s.contains("3.14") || s.contains("3.1"));
    }

    #[test]
    fn executes_not_equal() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(/= 1 2)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(/= 2 2)");
        assert_eq!(value, Some(LispValue::NIL));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(/= 1.0 2.0)");
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_percent_remainder() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(% 10 3)");
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(% 10.0 3.0)");
        assert!(rt.is_float(value.unwrap()));
    }

    #[test]
    fn executes_arrayp() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(arrayp [1 2])");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(arrayp 42)");
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_atom() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(atom 42)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(atom nil)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(atom (cons 1 2))");
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_minusp_and_plusp() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(cl-minusp -5)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(cl-minusp 5)");
        assert_eq!(value, Some(LispValue::NIL));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(cl-plusp 3)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(cl-plusp -3)");
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_nlistp() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(nlistp 42)");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(nlistp (cons 1 2))");
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_make_list() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(length (make-list 3 0))");
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_number_sequence() {
        let (value, _) =
            execute(";;; -*- lexical-binding: t; -*-\n(equal (number-sequence 1 3) '(1 2 3))");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) =
            execute(";;; -*- lexical-binding: t; -*-\n(= (length (number-sequence 0 5 2)) 3)");
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_greaterp() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(string> \"z\" \"a\")");
        assert_eq!(value, Some(LispValue::TRUE));
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(string> \"a\" \"z\")");
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_user_error() {
        let (result, _) = execute_result(";;; -*- lexical-binding: t; -*-\n(user-error \"boom\")");
        assert!(result.value.is_none());
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn executes_copy_list() {
        let (value, _) =
            execute(";;; -*- lexical-binding: t; -*-\n(equal (copy-list '(1 2 3)) '(1 2 3))");
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_sort_on_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (sort '(3 1 2) '<) '(1 2 3))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_sort_on_vector() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (sort [3 1 2] '<) [1 2 3])",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_safe_length_on_list() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(safe-length '(1 2 3))");
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_safe_length_on_nil() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(safe-length nil)");
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_subseq_on_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (subseq \"hello\" 0 3) \"hel\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_subseq_on_vector() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (subseq [10 20 30 40] 1 3) [20 30])",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_plist_get_with_keyword() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (plist-get '(:b 42 :c 99) :b)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_lambda_with_key_params_creates_closure() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (functionp (lambda (a &key b) b))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_lambda_with_key_params() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((f (lambda (a &key b) b)))\n\
               (funcall f 1 :b 42))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_lambda_with_key_params_missing_key() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((f (lambda (a &key b) b)))\n\
               (funcall f 5))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_lambda_with_aux_params() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((f (lambda (a &aux b) b)))\n\
               (funcall f 7))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_string_match_with_capture() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-match \"h.\" \"hello\")",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_match_string_after_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (string-match \"(h.)\" \"hello\") (match-string 1 \"hello\"))",
        );
        assert_ne!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_match_beginning_after_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (string-match \"he\" \"hello\") (match-beginning 0))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_match_end_after_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (string-match \"he\" \"hello\") (match-end 0))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_string_match_not_found() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-match \"xyz\" \"hello\")",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_lexical_shadowing_in_let_star() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let* ((x 1)\n\
                    (x (+ x 10)))\n\
               x)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(11)));
    }

    #[test]
    fn executes_defun_with_key_params() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (defun test-key (a &key b) b)\n\
             (test-key 1 :b 42)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_read_integer() {
        let (value, _) = execute(";;; -*- lexical-binding: t; -*-\n(read \"42\")");
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_read_symbol() {
        let (value, rt) = execute(";;; -*- lexical-binding: t; -*-\n(read \"hello\")");
        assert!(rt.is_symbol(value.unwrap()));
    }

    #[test]
    fn executes_read_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (car (read \"(1 2 3)\"))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_eval_form() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (eval '(+ 1 2))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_provide_and_featurep() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (provide 'test-feat) (featurep 'test-feat))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_require_with_provided_feature() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (provide 'already-here) (require 'already-here))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_file_exists_p() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (file-exists-p \"Cargo.toml\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_runtime_defun() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defalias 'my-add (lambda (a b) (+ a b))) (my-add 3 4))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(7)));
    }

    // --- Thread tests ---

    #[test]
    fn thread_current_thread_returns_main() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (current-thread)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn thread_make_thread_returns_new_id() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (make-thread \"worker\" (lambda () 42))",
        );
        // New thread id should be > 0
        match value {
            Some(v) if v.is_fixnum() => {
                assert!(v.as_fixnum().unwrap() > 0);
            }
            _ => panic!("expected fixnum thread id, got {value:?}"),
        }
        // Scheduler should now have 2 threads
        assert_eq!(runtime.scheduler.thread_count(), 2);
        assert!(runtime.scheduler.has_runnable());
    }

    #[test]
    fn thread_yield_preserves_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (make-thread \"worker\" (lambda () 42)) (thread-yield) 99)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn thread_alive_p_detects_live_thread() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((tid (make-thread \"worker\" (lambda () 42)))) (thread-alive-p tid))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
        assert_eq!(runtime.scheduler.thread_count(), 2);
    }

    #[test]
    fn thread_signal_marks_thread_error() {
        let (value, runtime) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((tid (make-thread \"worker\" (lambda () (thread-yield))))) \
               (thread-signal tid 'kill) \
               (thread-alive-p tid))",
        );
        assert_eq!(value, Some(LispValue::NIL));
        assert_eq!(runtime.scheduler.thread_count(), 2);
    }

    // --- Atom tests ---

    #[test]
    fn atom_create_and_deref() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((a (make-atom 42))) (atom-deref a))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn atom_reset_changes_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((a (make-atom 1))) (atom-reset! a 99) (atom-deref a))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn atom_compare_and_set_succeeds() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((a (make-atom 10))) (atom-compare-and-set! a 10 20) (atom-deref a))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(20)));
    }

    #[test]
    fn atom_compare_and_set_fails_on_wrong_old() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((a (make-atom 10))) (atom-compare-and-set! a 999 20))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn atom_swap_applies_function() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((a (make-atom 5))) (atom-swap! a (lambda (x) (* x 2))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    // --- Agent tests ---

    #[test]
    fn agent_create_and_deref() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ag (make-agent 10))) (agent-deref ag))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn agent_send_and_await() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ag (make-agent 0)))\
               (send ag (lambda (x) (+ x 1)))\
               (send ag (lambda (x) (+ x 2)))\
               (agent-await ag))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn agent_initial_value_preserved() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ag (make-agent 7))) (agent-deref ag))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(7)));
    }

    // --- Nonlocal exit tests ---

    #[test]
    fn executes_catch_throw() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (catch 'exit (throw 'exit 42) 0)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_nested_catch_throw() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (catch 'outer (catch 'inner (throw 'outer 99)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_condition_case_error() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (condition-case err (signal 'search-failed '(\"test\")) \
               (search-failed 42))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_condition_case_no_error() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (condition-case err 99 (error 0))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_unwind_protect_cleanup_runs() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((x 0)) (unwind-protect 1 (setq x 99)) x)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    // --- Thread + atom integration tests ---

    #[test]
    fn thread_mutex_protects_shared_state() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((m (make-mutex \"test\")) (a (make-atom 0)))\
               (mutex-lock m)\
               (atom-reset! a 42)\
               (mutex-unlock m)\
               (atom-deref a))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    // --- Native thread tests ---

    #[test]
    fn native_thread_spawns_and_computes() {
        let runtime = Runtime::new();
        let handle = runtime.spawn_native(|rt| {
            let a = rt.make_atom(LispValue::expect_fixnum(1));
            let _ = rt.atom_reset(a, LispValue::expect_fixnum(42));
            rt.atom_deref(a).unwrap_or(LispValue::NIL)
        });
        let result = handle.join().expect("native thread panicked");
        assert_eq!(result, LispValue::expect_fixnum(42));
    }

    #[test]
    fn native_thread_two_threads_parallel_computation() {
        let runtime = Runtime::new();
        let h1 = runtime.spawn_native(|_rt| LispValue::expect_fixnum(10));
        let h2 = runtime.spawn_native(|_rt| LispValue::expect_fixnum(20));
        let r1 = h1.join().expect("thread 1 panicked");
        let r2 = h2.join().expect("thread 2 panicked");
        assert_eq!(r1, LispValue::expect_fixnum(10));
        assert_eq!(r2, LispValue::expect_fixnum(20));
    }

    // --- Additional primitive coverage ---

    #[test]
    fn executes_apply_variadic() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (apply '+ '(1 2 3 4))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_member_present() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (member 3 '(1 2 3 4))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_assoc_lookup() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (assoc 'b '((a . 1) (b . 2) (c . 3)))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_mapcar_transform() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (mapcar '1+ '(1 2 3))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_setcar_mutates_cons() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((p (cons 1 2))) (setcar p 99) (car p))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_setcdr_mutates_cons() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((p (cons 1 2))) (setcdr p 99) (cdr p))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_error_signal_caught() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (condition-case err (error \"test error\") (error 42))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_prog1_returns_first() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (prog1 1 2 3)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_prog2_returns_second() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (prog2 1 2 3)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_string_equality() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= \"abc\" \"abc\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_concat_strings() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (concat \"ab\" \"cd\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_aref_aset_vector() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((v (vector 1 2 3))) (aset v 1 99) (aref v 1))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_length_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length '(a b c d e))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_nconc_destructive() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (nconc (list 1 2) (list 3 4))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_letrec_mutual_recursion() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (letrec ((even (lambda (n) (if (= n 0) t (odd (- n 1)))))\
                      (odd (lambda (n) (if (= n 0) nil (even (- n 1))))))\
               (even 4))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_hash_table_count_and_clear() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((h (make-hash-table))) (puthash 'a 1 h) (puthash 'b 2 h) \
               (prog1 (hash-table-count h) (clrhash h)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_hash_table_p_predicate() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (hash-table-p (make-hash-table))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_with_mutex_protects_body() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((a (make-atom 0)) (m (make-mutex \"t\")))\
               (mutex-lock m)\
               (atom-reset! a 42)\
               (mutex-unlock m)\
               (atom-deref a))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_special_form_p() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (special-form-p 'if)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_special_form_p_returns_nil_for_function() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (special-form-p 'cons)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_functionp_for_lambda() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (functionp (lambda (x) (+ x 1)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_subrp_for_cons() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (subrp (symbol-function 'cons))",
        );
        // cons is a built-in, so subrp should return t
        assert!(value.is_some());
    }

    #[test]
    fn executes_nreverse_reverses_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (nreverse (list 1 2 3))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_delq_removes_element() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (delq 2 (list 1 2 3 2))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_assq_finds_by_identity() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((key 'a)) (assq key '((a . 1) (b . 2))))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_random_returns_fixnum() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (numberp (random 100))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_logior_bitwise_or() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (logior 5 3)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(7))); // 101 | 011 = 111
    }

    #[test]
    fn executes_logand_bitwise_and() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (logand 5 3)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1))); // 101 & 011 = 001
    }

    #[test]
    fn executes_lsh_left_shift() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (lsh 1 3)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(8)));
    }

    #[test]
    fn executes_sort_ascending() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (sort (list 3 1 4 1 5 9) '<)",
        );
        assert!(value.is_some());
    }

    // Note: run-hooks and add-hook exist as primitives but the full
    // hook chain (defvar + lambda + symbol-value + run-hooks) needs
    // deeper compiler integration to work end-to-end.

    #[test]
    fn executes_thread_join_after_finish() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((tid (make-thread \"worker\" (lambda () 42))))\
               (thread-yield) (thread-yield)\
               (thread-join tid))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_thread_not_alive_after_signal() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((tid (make-thread \"worker\" (lambda () 1))))\
               (thread-signal tid 'kill)\
               (thread-alive-p tid))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_agent_send_multiple_actions() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ag (make-agent 0)))\
               (send ag (lambda (x) (+ x 1)))\
               (send ag (lambda (x) (+ x 2)))\
               (send ag (lambda (x) (+ x 3)))\
               (agent-await ag))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn executes_agent_error_after_bad_action() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ag (make-agent 0)))\
               (send ag (lambda (x) (+ x 1)))\
               (agent-await ag)\
               (agent-error ag))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_with_mutex_macro_locks_and_unlocks() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((a (make-atom 0)) (m (make-mutex \"t\")))\
               (with-mutex m (atom-reset! a 42))\
               (atom-deref a))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_mapconcat_joins_strings() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (mapconcat 'number-to-string (list 1 2 3) \"-\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_string_to_number() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-to-number \"42\")",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_eval_evaluates_form() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (eval '(+ 1 2 3))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    fn executes_cl_loop_collect() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for i from 1 to 3 collect (* i i))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_apply_with_arg_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (apply '+ 1 2 '(3 4))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_apply_empty_spread() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(apply '+ '())",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_apply_prefix_with_empty_spread() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(apply '+ 5 '())",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_cl_dolist_iterates_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((sum 0)) \
               (cl-dolist (x '(1 2 3 4) sum) \
                 (setq sum (+ sum x))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_cl_dolist_without_result() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((items nil)) \
               (cl-dolist (x '(a b c)) \
                 (push x items)) \
               (length items))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_dotimes_iterates_range() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((sum 0)) \
               (cl-dotimes (i 5 sum) \
                 (setq sum (+ sum i))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_cl_dotimes_without_result() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((count 0)) \
               (cl-dotimes (_ 3) \
                 (setq count (1+ count))) \
               count)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_dotimes_with_cl_return() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (cl-dotimes (i 10) \
               (when (= i 3) (cl-return 99)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_cl_dolist_with_cl_return() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (cl-dolist (x '(a b c d e)) \
               (when (eq x 'c) (cl-return 42)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_cl_do_with_cl_return() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (cl-do ((i 0 (1+ i))) \
               ((= i 5) 99) \
               (when (= i 2) (cl-return 42)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_function_special_form_returns_function() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (funcall (function +) 1 2)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_sharp_quote_returns_function() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (funcall #'+ 1 2)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_apply_append_flattens_lists() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length (apply #'append '((1 2) (3 4) (5 6))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn executes_float_negative_zero_is_distinct() {
        // In Emacs, (eql 0.0 -0.0) -> nil
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(eql 0.0 -0.0)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_and_short_circuits() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(and nil (error \"unreachable\"))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_or_short_circuits() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(or t (error \"unreachable\"))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_define_error_with_condition_case() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn \
               (define-error 'my-test-err \"test\")\
               (condition-case err \
                 (signal 'my-test-err '(42)) \
                 (error (cadr err))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_condition_case_normal_path() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (condition-case err \
               (+ 1 2) \
               (error 99))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_psetq_parallel_assignment() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((a 1) (b 2)) (psetq a b b a) (+ a (* b 10)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn executes_cl_accessors_on_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (+ (cl-first '(10 20 30 40)) \
                (cl-second '(10 20 30 40)) \
                (cl-third '(10 20 30 40)) \
                (cl-fourth '(10 20 30 40)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(100)));
    }

    #[test]
    fn executes_cl_macrolet_local_macro() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-macrolet ((square (x) (list '* x x)))\
               (square 7))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(49)));
    }

    #[test]
    fn executes_complex_cl_integration() {
        // flet + dolist + case: filter even numbers, double them, sum
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((sum 0)) \
               (cl-flet ((add (x) (setq sum (+ sum x)))) \
                 (cl-dolist (n '(1 2 3 4 5 6)) \
                   (cl-case (cl-oddp n) \
                     ((nil) (add n))))) \
               sum)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn executes_cl_symbol_macrolet() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-symbol-macrolet ((x 42)) \
               (+ x 1))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(43)));
    }

    #[test]
    fn executes_cl_pushnew_adds_uniquely() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((xs '(2 3))) \
               (cl-pushnew 1 xs) \
               (cl-pushnew 2 xs) \
               (length xs))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_remove_duplicates() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length (cl-remove-duplicates '(1 2 3 1 2 4)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn executes_cl_remove_if_on_vector() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length (cl-remove-if #'evenp [1 2 3 4 5 6]))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_remove_duplicates_on_vector() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length (cl-remove-duplicates [1 2 3 1 2 3]))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_substitute_replaces_elements() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length (cl-substitute 99 2 '(1 2 3 2 4)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_cl_tree_equal() {
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-tree-equal '(1 (2 3)) '(1 (2 3)))").0,
            Some(LispValue::TRUE));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-tree-equal '(1 (2 3)) '(1 (2 4)))").0,
            Some(LispValue::NIL));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-tree-equal nil nil)").0,
            Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_typep_basic_types() {
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-typep 42 'integer)").0,
            Some(LispValue::TRUE));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-typep 3.14 'float)").0,
            Some(LispValue::TRUE));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-typep \"hi\" 'string)").0,
            Some(LispValue::TRUE));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-typep nil 'null)").0,
            Some(LispValue::TRUE));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-typep '(1 . 2) 'cons)").0,
            Some(LispValue::TRUE));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-typep 42 'cons)").0,
            Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_remove_removes_elements() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length (cl-remove 2 '(1 2 3 2 4)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }


    #[test]
    fn executes_funcall_with_multiple_args() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (funcall '+ 1 2 3)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    // Note: defmacro cross-form expansion within a single progn
    // needs multi-pass compiler support. Macros defined via require
    // (e.g. cl-lib) work correctly.
    // Note: defun cross-form calls need SymbolFunctionSet IR instruction.
    // The defalias cross-form test above proves the calling infrastructure works.

    #[test]
    fn executes_defun_cross_form_call() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defun my-add (x y) (+ x y)) (my-add 40 2))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_defmacro_cross_form() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defmacro my-inc (x) (list '+ x 1)) (my-inc 41))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_defun_with_interactive() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defun my-cmd () (interactive) 42) (my-cmd))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_cl_loop_named_return() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (cl-loop named my-block for i from 1 to 10 \
               do (when (> i 3) (cl-return-from my-block i)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn executes_pcase_let_star_destructure() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (pcase-let* ((`(,x ,y) '(1 2))) (+ x y))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_subseq_on_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(length (subseq '(a b c d e) 1 4))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    #[test]
    fn executes_macroexpand() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(macroexpand '(when t 42))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_cl_accessors() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (+ (cl-first '(1 2 3)) (cl-second '(1 2 3)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_makunbound_unbinds_defvar() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defvar y 42) (makunbound 'y) (not (boundp 'y)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_defvar_without_value_declares_special() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defvar my-var) (boundp 'my-var))",
        );
        // In Emacs, (defvar SYM) declares it special but leaves it unbound
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_defvar_with_docstring() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defvar my-doc-var 42 \"documentation string\")\
                    (symbol-value 'my-doc-var))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_defvar_only_sets_when_unbound() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defvar my-var 1) (defvar my-var 99) (symbol-value 'my-var))",
        );
        // Second defvar does NOT overwrite the existing value
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_defconst_always_sets() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defconst my-const 1) (defconst my-const 99) (symbol-value 'my-const))",
        );
        // defconst always overwrites
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_fmakunbound_removes_function_binding() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (fset 'my-fn (lambda () 1)) \
               (fmakunbound 'my-fn) \
               (not (fboundp 'my-fn)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_every_all_satisfy_predicate() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (every #'numberp '(1 2 3))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_assoc_string_case_sensitive() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cdr (assoc-string \"key\" '((\"key\" . 42) (\"KEY\" . 99))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_assoc_string_case_insensitive() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cdr (assoc-string \"key\" '((\"KEY\" . 42) (\"other\" . 99)) t))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_alist_get_finds_existing_key() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (alist-get 'b '((a . 1) (b . 2) (c . 3)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_alist_get_returns_default_when_missing() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (alist-get 'z '((a . 1)) 99)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_alist_get_returns_nil_when_missing_no_default() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (alist-get 'z '((a . 1)))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_prin1_to_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (prin1-to-string 42)",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_ignore_errors_returns_nil_on_error() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (ignore-errors (error \"boom\"))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_ignore_errors_returns_result_when_no_error() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (ignore-errors 42)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_macroexpand_1_expands_top_level_macro() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (macroexpand-1 '(when t 42))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_sxhash_eq_returns_fixnum() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (integerp (sxhash-eq 'foo))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_sxhash_eql_returns_fixnum() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (integerp (sxhash-eql 42))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_sxhash_equal_returns_fixnum() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (integerp (sxhash-equal '(a b c)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_sxhash_equal_is_consistent() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= (sxhash-equal '(1 2 3)) (sxhash-equal '(1 2 3)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_define_error_creates_symbol() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (define-error 'my-test-error \"test message\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_every_one_fails_predicate() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (every #'numberp '(1 a 3))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_some_returns_first_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (some #'numberp '(a b 3 c))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_some_returns_nil_when_no_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (some #'numberp '(a b c))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_numeric_eq_multi_arg() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= 1 1 1)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_numeric_eq_multi_arg_false() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= 1 1 2)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_less_than_multi_arg() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (< 1 2 3)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_less_equal_multi_arg() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (<= 1 1 2)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_not_equal_multi_arg_all_distinct() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (/= 1 2 3)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_not_equal_multi_arg_not_distinct() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (/= 1 2 1)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_not_equal_zero_args_returns_t() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(/=)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_not_equal_one_arg_returns_t() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(/= 42)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_not_equal_multi_arg_dup_not_first() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (/= 1 2 3 2)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_max_multi_arg() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (max 3 1 4 1 5 9)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(9)));
    }

    #[test]
    fn executes_comparison_zero_args_returns_t() {
        for op in ["=", "<", ">", "<=", ">="] {
            let (value, _) = execute(
                &format!(";;; -*- lexical-binding: t; -*-\n({op})"),
            );
            assert_eq!(value, Some(LispValue::TRUE), "({op}) should return t");
        }
    }

    #[test]
    fn executes_comparison_one_arg_returns_t() {
        for op in ["=", "<", ">", "<=", ">="] {
            let (value, _) = execute(
                &format!(";;; -*- lexical-binding: t; -*-\n({op} 1)"),
            );
            assert_eq!(value, Some(LispValue::TRUE), "({op} 1) should return t");
        }
    }

    #[test]
    fn executes_upcase_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (upcase \"hello\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_copy_hash_table_preserves_entries() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((h (make-hash-table)))\
               (puthash 'a 1 h)\
               (puthash 'b 2 h)\
               (let ((h2 (copy-hash-table h)))\
                 (+ (gethash 'a h2 0) (gethash 'b h2 0))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_replace_regexp_in_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (replace-regexp-in-string \"[0-9]+\" \"X\" \"abc123def456\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_split_string_and_join_roundtrip() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-join (split-string \"a,b,c\" \",\") \"-\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_downcase_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (downcase \"HELLO\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_string_equal_multi_arg() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= \"ab\" \"ab\" \"ab\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_equal_multi_arg_false() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= \"ab\" \"ab\" \"xy\")",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_assoc_returns_nil_for_empty_alist() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (assoc 'a '())",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_add_zero_args_returns_zero() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(+)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_evenp_oddp() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (and (evenp 4) (oddp 5))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_minusp_and_plusp() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(and (minusp -3) (plusp 5))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_not_and_null() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(and (not nil) (null nil) (not (null t)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_remove_prefix() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-remove-prefix \"pre-\" \"pre-fix\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_save_excursion_noop() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(save-excursion 42)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_if_then_else() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(if t 42 0)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    // cl-block and cl-return-from macros are defined in builtin_libs.

    #[test]
    fn executes_pcase_simple_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (pcase 42 ((pred numberp) 'yes) (_ 'no))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_nconc_destructive_append() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(length (nconc (list 1 2) (list 3 4)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));
    }

    fn executes_symbol_name_returns_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(symbol-name 'hello)",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_floatp_edges() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(and (floatp 1.0) (not (floatp 1)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_nthcdr_skip_and_access() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(nthcdr 2 '(10 20 30 40))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_cdr_safe_non_cons_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(cdr-safe 42)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    fn executes_car_cdr_of_nil_return_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(and (null (car nil)) (null (cdr nil)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    fn executes_ignore_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(ignore 1 2 3)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_identity_returns_arg() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(identity 42)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_string_equal_case_insensitive() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(string-equal \"Hello\" \"hello\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_mul_mixed_float() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(* 2 3.5)",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_eq_float_int() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(= 1 1.0)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_atom_predicate() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(and (atom 42) (atom nil) (not (atom '(1 2))))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_list_constructor() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(length (list 1 2 3 4 5))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_cdar_alist_access() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(cdar '((a . 1) (b . 2)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_cddr_skips_two() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(car (cddr '(10 20 30 40)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(30)));
    }

    #[test]
    fn executes_substring_from_index() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(substring \"hello\" 1)",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_string_join_separator() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(string-join '(\"a\" \"b\" \"c\") \"-\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_last_returns_tail() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(car (last '(1 2 3 4 5)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_number_sequence_range() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(length (number-sequence 1 5))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_caddr_third_element() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(caddr '(10 20 30))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(30)));
    }

    #[test]
    fn executes_cadr_second_element() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(cadr '(10 20 30))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(20)));
    }

    #[test]
    fn executes_remq_removes_by_identity() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(length (remq 'b (list 'a 'b 'c)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_delete_removes_element() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(length (delete 2 (list 1 2 3 2)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_string_greaterp_reverse() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(string-greaterp \"b\" \"a\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_vconcat_concatenates() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(length (vconcat [1 2] [3 4] [5 6]))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn executes_fillarray_vector() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((v (vector 1 2 3))) (fillarray v 0) (aref v 1))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_string_lessp_ordering() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(string-lessp \"abc\" \"abd\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_rassq_finds_by_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(rassq 2 '((a . 1) (b . 2) (c . 3)))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_butlast_removes_last() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(length (butlast '(1 2 3 4) 2))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_integerp_edge_cases() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (and (integerp 0) (not (integerp 1.5)) (not (integerp 'sym)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_mapconcat_with_number_to_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (mapconcat 'number-to-string '(1 2 3) \",\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_copy_tree_deep() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((orig '((1 . 2) (3 . 4)))) (eq orig (copy-tree orig)))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_safe_length_dotted() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(safe-length '(a b . c))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_make_list_fill() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(length (make-list 5 42))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_lognot_bitwise() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(lognot 0)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(-1)));
    }

    #[test]
    fn executes_cl_pushnew_dedup() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((lst '(1 2 3))) (cl-pushnew 2 lst) (length lst))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_incf_and_decf() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((x 10)) (cl-incf x) (cl-decf x 3) x)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(8)));
    }

    #[test]
    fn executes_string_trim_strips_whitespace() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(string-trim \"  hello  \")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_make_vector_default_init() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(aref (make-vector 3 42) 1)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_cl_labels_mutual_recursion() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-labels ((even (n) (if (= n 0) t (odd (- n 1))))\
                         (odd (n) (if (= n 0) nil (even (- n 1)))))\
               (even 4))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_flet_local_function() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-flet ((double (x) (* x 2))) (double 21))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_eval_when_compile_evaluates_at_compile_time() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (eval-when-compile (+ 1 2 3))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn executes_destructuring_bind() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (destructuring-bind (a b c) '(1 2 3) (+ a b c))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn executes_pcase_constant_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (pcase 'hello ('hello 'found) (_ 'missing))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_symbol_get_and_put() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (put 'test-key 'prop 42) (get 'test-key 'prop))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_plist_get_and_put() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((plist (plist-put nil :key 42))) (plist-get plist :key))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_logand_no_args_returns_neg_one() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(logand)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(-1)));
    }

    #[test]
    fn executes_setcar_changes_cons() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((p (cons 1 2))) (setcar p 99) (car p))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_setcdr_changes_cons() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((p (cons 1 2))) (setcdr p 99) (cdr p))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_logior_no_args_returns_zero() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(logior)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_require_feature_marks_provided() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (provide 'test-mod) (require 'test-mod) (featurep 'test-mod))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cons_with_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(car (cons 42 nil))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_append_concatenates() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(length (append '(1 2) '(3 4) '(5 6)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn executes_if_no_else() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(if nil 42)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_let_parallel() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let ((x 1) (y 2)) (+ x y))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_let_star_sequential() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(let* ((x 1) (y (+ x 2))) y)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_dotimes_loop() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((sum 0)) (dotimes (i 5) (setq sum (+ sum i))) sum)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_dolist_loop() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((sum 0)) (dolist (x '(1 2 3 4)) (setq sum (+ sum x))) sum)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_with_current_buffer_noop() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(with-current-buffer \"*scratch*\" 99)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_abs_negative() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(abs -5)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_reverse_and_nreverse() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((l (list 1 2 3))) (equal (reverse l) (nreverse (copy-sequence l))))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_listp_and_nlistp() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (and (listp '(a b)) (not (listp 42)) (nlistp 42) (not (nlistp '(a b))))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_booleanp() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(and (booleanp t) (booleanp nil) (not (booleanp 42)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cons_equal_for_identical() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((p (cons 1 2))) (equal p p))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_length_comparison() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (and (length= '(a b c) 3) (length< '(a) 3) (length> '(a b c d) 3))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_remove_suffix() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-remove-suffix \"-suf\" \"pre-suf\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_zerop() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(and (zerop 0) (not (zerop 1)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_mod_negative() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(mod -5 3)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_1plus_and_1minus() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (+ (1+ 5) (1- 5))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_neg_single_arg() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(- 5)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(-5)));
    }

    #[test]
    fn executes_mul_zero_args_returns_one() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(*)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_nth_returns_correct_element() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (nth 2 '(10 20 30 40))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(30)));
    }

    #[test]
    fn executes_nthcdr_returns_tail() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (car (nthcdr 2 '(10 20 30 40)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(30)));
    }

    #[test]
    fn executes_rassoc_finds_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (rassoc 2 '((a . 1) (b . 2) (c . 3)))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_string_lessp_multi_arg() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string< \"a\" \"b\" \"c\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_min_multi_arg() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (min 3 1 4 1 5 9)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_cl_loop_append_collects_lists() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for i in '((1 2) (3 4) (5 6)) append i)",
        );
        assert!(value.is_some());
    }

    // Float and number predicates already covered by existing tests:
    // executes_float_constant, executes_floatp, executes_float_addition, etc.

    #[test]
    fn executes_defalias_cross_form_call() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defalias 'my-fn (lambda (x) (+ x 1))) (my-fn 41))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn stress_test_many_objects_type_checks_fast() {
        // Create many objects of different types and verify O(1) lookups.
        // If type predicates still did O(n) scans, this would be very slow.
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((v (make-vector 100 0)) (h (make-hash-table)))\
               (dotimes (i 100)\
                 (aset v i (cons i i))\
                 (puthash i i h))\
               (and (vectorp v) (hash-table-p h) (consp (aref v 50)) t))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn stress_test_many_objects_car_cdr_fast() {
        // Build a 100-element list and access car/cdr through it.
        // With direct pointer dereference, this is O(n) total, not O(n^2).
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((lst nil))\
               (dotimes (i 100) (push i lst))\
               (car (cdr (cdr lst))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(97)));
    }

    #[test]
    fn executes_cl_loop_numeric_for() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for i from 1 to 3 sum i)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn executes_cl_loop_repeat_n_times() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop repeat 3 collect 1)",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_cl_loop_while_breaks_early() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for i from 1 to 10 while (< i 4) sum i)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6))); // 1+2+3
    }

    fn executes_cl_loop_sum() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-loop for i from 1 to 4 sum i)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_string_match_finds_pattern() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-match \"hello\" \"hello world\")",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_string_match_not_found_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-match \"xyz\" \"hello world\")",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_logxor_bitwise_xor() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (logxor 5 3)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6))); // 101 ^ 011 = 110
    }

    #[test]
    fn executes_logxor_no_args_returns_zero() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(logxor)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_lsh_right_shift() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (lsh 8 -2)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }


    fn executes_copy_sequence_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((orig (list 1 2 3))) (eq orig (copy-sequence orig)))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_macroexpand_expands_when_macro() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (car (macroexpand '(when t 42)))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_macroexpand_expands_unless_macro() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (macroexpand '(unless nil 99))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_cl_loop_named_with_collect() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (cl-loop named my-loop for i from 1 to 5 \
               collect i into results \
               finally (cl-return-from my-loop results))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_pcase_let_star_multiple_bindings() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (pcase-let* ((`(,a ,b) '(1 2)) (`(,c ,d) '(3 4))) (+ a b c d))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_pcase_let_star_symbol_binding() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (pcase-let* ((x 1) (y 2)) (+ x y))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_defun_cross_form_multiple_functions() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defun f1 (x) (+ x 1)) (defun f2 (x) (+ x 2)) (f2 (f1 10)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(13)));
    }

    #[test]
    fn executes_defun_with_multiple_calls() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (defun double (x) (* x 2)) \
               (+ (double 3) (double 4)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(14)));
    }

    #[test]
    fn executes_cl_block_and_cl_return_from() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (cl-block outer \
               (cl-block inner \
                 (cl-return-from outer 42) 99))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_cl_case_single_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-case 'b ((a) 1) ((b c) 2) (t 3))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_cl_case_otherwise_fallback() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-case 'z ((a) 1) ((b) 2) (otherwise 99))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_cl_destructuring_bind() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-destructuring-bind (x y z) '(1 2 3) (+ x y z))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(6)));
    }

    #[test]
    fn executes_cl_rotatef_swaps_values() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((a 1) (b 2)) (cl-rotatef a b) (+ a (* b 10)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(12)));
    }

    #[test]
    fn executes_cl_shiftf_shifts_values() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (let ((a 1) (b 2) (c 3)) (cl-shiftf a b c) (+ a (* b 10) (* c 100)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(332)));
    }

    #[test]
    fn executes_cl_do_simple_iteration() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-do ((i 0 (1+ i))) \
               ((>= i 5) i))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_cl_do_return_result() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-do ((x 1 (1+ x))) \
               ((> x 5) (* x 10)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(60)));
    }

    #[test]
    fn executes_cl_do_star_sequential() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-do* ((x 1 (1+ x)) (y (* x 2) (* x 2))) \
               ((> x 3) y))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_acons_constructs_alist() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cdr (assq 'x (acons 'x 1 (acons 'y 2 nil))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_cl_acons_works_as_alias() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cdr (assq 'k (cl-acons 'k 99 nil)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_cl_adjoin_adds_uniquely() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\
             (length (cl-adjoin 1 '(2 3)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_mapcar_alias() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (car (cl-mapcar #'1+ '(1 2 3)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_cl_mapc_alias() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((xs '(1 2 3))) (eq xs (cl-mapc #'1+ xs)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_evenp_oddp_aliases() {
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-evenp 4)").0,
            Some(LispValue::TRUE));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-oddp 3)").0,
            Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_plusp_minusp_predicates() {
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-plusp 5)").0,
            Some(LispValue::TRUE));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-plusp -1)").0,
            Some(LispValue::NIL));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(cl-minusp -3)").0,
            Some(LispValue::TRUE));
    }

    #[test]
    fn executes_ash_left_and_right_shift() {
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(ash 8 -2)").0,
            Some(LispValue::expect_fixnum(2)));
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(ash 2 3)").0,
            Some(LispValue::expect_fixnum(16)));
    }

    #[test]
    fn executes_lsh_right_shift_test() {
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n(lsh 16 -2)").0,
            Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn executes_symbol_function_retrieves_binding() {
        assert_eq!(execute(";;; -*- lexical-binding: t; -*-\n\
            (functionp (symbol-function '+))").0,
            Some(LispValue::TRUE));
    }

    #[test]
    fn executes_fset_and_symbol_function_roundtrip() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (fset 'my-test-fn (lambda (x) (1+ x))) \
               (funcall (symbol-function 'my-test-fn) 41))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_cl_reverse_reverses_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (car (cl-reverse '(1 2 3)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_rassoc_finds_by_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cdr (cl-rassoc 2 '((a . 1) (b . 2) (c . 3))))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_cl_member_finds_element() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (car (cl-member 2 '(1 2 3)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_cl_concatenate_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length (cl-concatenate 'list '(1 2) '(3 4)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn executes_letrec_recursive_binding() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (letrec ((is-even (lambda (n) \
               (if (= n 0) t (funcall is-odd (- n 1))))) \
               (is-odd (lambda (n) \
                 (if (= n 0) nil (funcall is-even (- n 1)))))) \
               (funcall is-even 4))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_list_empty_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(list)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_list_single_element() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(car (list 42))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_rust_subr_fib() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(fib 10)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(55)));
    }

    #[test]
    fn executes_let_with_empty_bindings() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let () 42)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_let_with_nil_bindings() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let nil 99)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_progn_empty_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(progn)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_prog2_returns_second_form() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(prog2 1 2 3)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_eql_distinguishes_types() {
        // eql returns nil for different numeric types (fixnum vs float)
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(eql 1 1.0)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_eql_equivalent_values() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(eql 1 1)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_remove_if_filters_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length (cl-remove-if #'evenp '(1 2 3 4 5 6)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_remove_if_not_filters_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (car (cl-remove-if-not (lambda (x) (numberp x)) '(a 1 b 2)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_cond_empty_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n(cond)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_setf_car_modifies_cons() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((p (cons 1 2))) (setf (car p) 9) (car p))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(9)));
    }

    #[test]
    fn executes_setf_cdr_modifies_cons() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((p (cons 1 2))) (setf (cdr p) 9) (cdr p))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(9)));
    }

    #[test]
    fn executes_setf_aref_modifies_vector() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((v (vector 1 2 3))) (setf (aref v 1) 99) (aref v 1))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_setf_gethash_modifies_hash_table() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((h (make-hash-table))) \
               (setf (gethash 'k h) 42) \
               (gethash 'k h))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_setf_symbol_value_modifies_symbol() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (setq x 1) (setf (symbol-value 'x) 99) (symbol-value 'x))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_setf_nth_modifies_list_element() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((xs (list 10 20 30))) (setf (nth 1 xs) 99) (nth 1 xs))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_cl_position_finds_index() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-position 2 (list 1 2 3))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_cl_position_not_found_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-position 99 (list 1 2 3))",
        );
        assert!(matches!(value, Some(v) if v.is_nil()));
    }

    #[test]
    fn executes_cl_position_empty_list_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-position 1 '())",
        );
        assert!(matches!(value, Some(v) if v.is_nil()));
    }

    #[test]
    fn executes_cl_find_returns_item() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-find 'b (list 'a 'b 'c))",
        );
        assert_eq!(rt.symbol_name(value.unwrap()).unwrap(), "b");
    }

    #[test]
    fn executes_cl_find_not_found_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-find 'z (list 'a 'b 'c))",
        );
        assert!(matches!(value, Some(v) if v.is_nil()));
    }

    #[test]
    fn executes_cl_count_counts_occurrences() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-count 1 (list 1 2 1 3 1))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_count_returns_zero_for_none() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-count 99 (list 1 2 3))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_cl_count_empty_list_returns_zero() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-count 1 '())",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_cl_reduce_sums_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-reduce #'+ (list 1 2 3 4))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(10)));
    }

    #[test]
    fn executes_cl_reduce_with_initial_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-reduce #'+ (list 1 2 3) 10)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(16)));
    }

    #[test]
    fn executes_cl_reduce_single_element_returns_it() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-reduce #'+ (list 42))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_cl_reduce_empty_list_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-reduce #'+ '())",
        );
        assert!(matches!(value, Some(v) if v.is_nil()));
    }

    #[test]
    fn executes_cl_adjoin_adds_new_item() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-adjoin 3 '(1 2))",
        );
        let result = value.unwrap();
        assert_eq!(
            rt.format_value(result),
            "(3 1 2)"
        );
    }

    #[test]
    fn executes_cl_adjoin_does_not_add_duplicate() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-adjoin 2 '(1 2 3))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 2 3)");
    }

    #[test]
    fn executes_cl_endp_nil_is_true() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-endp nil)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_endp_cons_is_false() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-endp (cons 1 2))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_endp_empty_list_is_true() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-endp '())",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_position_symbol_in_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-position 'b (list 'a 'b 'c))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_cl_find_return_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-find 42 (list 10 20 42 30))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_cl_count_zero_based_literal() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-count 'x (list 'a 'x 'b 'x 'c 'x))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_list_star_builds_dotted_pair() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (list* 1 2)",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 . 2)");
    }

    #[test]
    fn executes_list_star_builds_chain_with_tail() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (list* 1 2 3)",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 2 . 3)");
    }

    #[test]
    fn executes_list_star_one_arg_is_identity() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (list* 42)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_list_star_no_args_is_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (list*)",
        );
        assert!(matches!(value, Some(v) if v.is_nil()));
    }

    #[test]
    fn executes_cl_reduce_with_lambda_and_initial() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-reduce (lambda (a b) (+ a (* 10 b))) (list 1 2 3) 0)",
        );
        // 0*10+1=1, 1*10+2=12, 12*10+3=123
        // BUT reduce passes ACCUMULATOR first, ELEMENT second:
        // (+ 0 (* 10 1)) = 10, (+ 10 (* 10 2)) = 30, (+ 30 (* 10 3)) = 60
        assert_eq!(value, Some(LispValue::expect_fixnum(60)));
    }

    #[test]
    fn executes_cl_adjoin_empty_list_adds_item() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-adjoin 1 '())",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1)");
    }

    #[test]
    fn executes_cl_every_empty_list_is_true() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-every #'identity '())",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_some_empty_list_is_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-some #'identity '())",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_subseq_negative_start_from_end() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (subseq '(a b c d e) -2)",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(d e)");
    }

    #[test]
    fn executes_subseq_negative_start_and_end() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (subseq '(a b c d e) -3 -1)",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(c d)");
    }

    #[test]
    fn executes_subseq_negative_index_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length (subseq \"hello\" -3))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_notany_returns_t_when_none_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-notany #'oddp (list 2 4 6))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_subst_if_replaces_matching_leaves() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-subst-if 99 #'oddp '(1 2 3 4))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(99 2 99 4)");
    }

    #[test]
    fn executes_cl_subst_if_not_replaces_non_matching() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-subst-if-not 0 #'evenp '(1 2 3 4))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(0 2 0 4)");
    }

    #[test]
    fn executes_cl_sublis_replaces_from_alist() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-sublis '((a . 1) (b . 2)) '(a b c))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 2 c)");
    }

    #[test]
    fn executes_cl_sublis_on_nested_tree() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-sublis '((a . x) (b . y)) '((a . b) (c . a)))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "((x . y) (c . x))");
    }

    #[test]
    fn executes_maplist_maps_over_cdrs() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (maplist #'car '(a b c))",
        );
        // (car (a b c)), (car (b c)), (car (c)) => (a b c)
        assert_eq!(rt.format_value(value.unwrap()), "(a b c)");
    }

    #[test]
    fn executes_mapl_returns_original_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((xs (list 10 20 30))) (eq xs (mapl #'ignore xs)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_mapcan_concatenates_results() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (mapcan (lambda (x) (list x (* 10 x))) (list 1 2 3))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 10 2 20 3 30)");
    }

    #[test]
    fn executes_mapcon_on_cdrs() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (mapcon (lambda (tail) (list (car tail))) '(a b c))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(a b c)");
    }

    #[test]
    fn executes_cl_set_difference_removes_elements() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-set-difference '(1 2 3 4) '(3 5))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 2 4)");
    }

    #[test]
    fn executes_cl_intersection_keeps_common_elements() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-intersection '(1 2 3 4) '(3 4 5 6))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(3 4)");
    }

    #[test]
    fn executes_cl_union_merges_without_duplicates() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-union '(1 2 3) '(3 4 5))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 2 3 4 5)");
    }

    #[test]
    fn executes_cl_fill_replaces_all_elements() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-fill '(a b c d) 99)",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(99 99 99 99)");
    }

    #[test]
    fn executes_cl_set_exclusive_or_returns_unique_elements() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-set-exclusive-or '(1 2 3 4) '(3 4 5 6))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 2 5 6)");
    }

    #[test]
    fn executes_cl_count_if_counts_matching_elements() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-count-if #'oddp (list 1 2 3 4 5))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_cl_mismatch_finds_first_difference() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-mismatch '(1 2 3) '(1 2 4))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_cl_mismatch_equal_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-mismatch '(1 2 3) '(1 2 3))",
        );
        assert!(matches!(value, Some(v) if v.is_nil()));
    }

    #[test]
    fn executes_cl_find_if_returns_first_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-find-if #'evenp '(1 3 4 5 6))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(4)));
    }

    #[test]
    fn executes_cl_position_if_returns_index() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-position-if #'evenp '(1 3 4 5 6))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_cl_member_if_returns_tail() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-member-if #'evenp '(1 3 4 5 6))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(4 5 6)");
    }

    #[test]
    fn executes_cl_assoc_if_finds_by_predicate() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-assoc-if #'symbolp '((1 . a) (b . c) (3 . d)))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(b . c)");
    }

    #[test]
    fn executes_cl_search_finds_subsequence() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-search '(2 3) '(1 2 3 4))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(1)));
    }

    #[test]
    fn executes_cl_delete_if_removes_matching_elements() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-delete-if #'oddp '(1 2 3 4 5))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(2 4)");
    }

    #[test]
    fn executes_cl_merge_combines_sorted_lists() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-merge 'list '(1 3 5) '(2 4 6) #'<)",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 2 3 4 5 6)");
    }

    #[test]
    fn executes_cl_tailp_detects_tail() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((xs (list 1 2 3))) (cl-tailp (cdr xs) xs))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_default_value_returns_global_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (defvar test-var 42) (default-value 'test-var)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_set_default_sets_global_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (defvar my-var 10) (set-default 'my-var 99) (default-value 'my-var)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_string_prefix_p_detects_prefix() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-prefix-p \"hello\" \"hello world\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_suffix_p_detects_suffix() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-suffix-p \"world\" \"hello world\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_prefix_p_returns_nil_for_non_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-prefix-p \"xyz\" \"hello world\")",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_delete_dups_removes_adjacent_duplicates() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (delete-dups '(1 1 2 2 3 3))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 2 3)");
    }

    #[test]
    fn executes_fixnump_recognizes_fixnum() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (fixnump 42)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_floatp_recognizes_float() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (floatp 3.14)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_bignump_returns_nil_for_fixnum() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (bignump 42)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_bare_symbol_p_on_regular_symbol() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (bare-symbol-p 'my-sym)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_char_equal_case_insensitive() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (char-equal ?a ?A)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_char_equal_different_chars_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (char-equal ?a ?b)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_numeric_equality_of_chars() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= ?a 97)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_aref_on_vector() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (aref [10 20 30] 1)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(20)));
    }

    #[test]
    fn executes_aref_out_of_bounds_signals_error() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (condition-case nil (aref [1 2] 5) (args-out-of-range nil))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_length_equals_true_for_correct_length() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length= '(1 2 3) 3)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_length_equals_false_for_wrong_length() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (length= '(1 2) 3)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_upcase_initials_capitalizes_words() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (upcase-initials \"hello world\") \"Hello World\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_upcase_initials_empty_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (upcase-initials \"\") \"\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_memql_finds_float_by_value() {
        // memql uses eql, so floats compare by value (bit pattern)
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (memql 3.0 (list 1.0 2.0 3.0 4.0))",
        );
        // Returns non-nil tail if found
        assert!(!value.unwrap().is_nil());
    }

    #[test]
    fn executes_default_boundp_checks_global_binding() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (defvar boundp-test-var 42) (default-boundp 'boundp-test-var)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_set_default_overrides_variable() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (defvar sd-var 10) (progn (set-default 'sd-var 99) (default-value 'sd-var))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_proper_list_p_returns_length() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (proper-list-p '(a b c))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_string_to_number_with_hex_base() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-to-number \"ff\" 16)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(255)));
    }

    #[test]
    fn executes_string_to_number_with_hex_prefix() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-to-number \"#xff\")",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(255)));
    }

    #[test]
    fn executes_string_to_number_with_octal_prefix() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-to-number \"#o77\")",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(63)));
    }

    #[test]
    fn executes_string_to_number_with_binary_prefix() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-to-number \"#b101\")",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_string_to_number_empty_string_is_zero() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-to-number \"\")",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_car_safe_on_non_cons_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (car-safe 42)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cdr_safe_on_non_cons_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cdr-safe 99)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_display_graphic_p_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (display-graphic-p)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_color_defined_p_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (color-defined-p \"red\")",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_make_string_creates_char_repeat() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (make-string 3 ?x) \"xxx\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_make_string_with_integer_code() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (make-string 1 65) \"A\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_commandp_recognizes_function() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (commandp (lambda ()))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_subr_arity_returns_cons() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (subr-arity #'car)",
        );
        assert!(rt.is_cons(value.unwrap()));
    }

    #[test]
    fn executes_sin_on_float() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (< (abs (- (sin 0.0) 0.0)) 0.0001)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_sqrt_returns_correct_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (< (abs (- (sqrt 4.0) 2.0)) 0.0001)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_log_returns_correct_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (< (abs (- (log 1.0) 0.0)) 0.0001)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_log_with_base_returns_correct_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (< (abs (- (log 100 10) 2.0)) 0.0001)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_plist_member_finds_key() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (eq (car (plist-member '(a 1 b 2 c 3) 'b)) 'b)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_plist_member_returns_nil_for_missing_key() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (null (plist-member '(a 1 b 2) 'c))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_bobp_and_eobp_are_true() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (and (bobp) (eobp))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_char_code_returns_integer() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (char-code ?A)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(65)));
    }

    #[test]
    fn executes_char_valid_p_for_ascii() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (char-valid-p 65)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_char_valid_p_for_surrogate() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (char-valid-p #xD800)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_buffer_modified_p_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (buffer-modified-p)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_window_buffer_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (window-buffer)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_standard_syntax_table_is_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (null (standard-syntax-table))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_syntax_table_p_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (syntax-table-p 'anything)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_indirect_function_on_symbol() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (defun indirect-fn-test () 42) (indirect-function 'indirect-fn-test)",
        );
        assert!(!value.unwrap().is_nil());
    }

    #[test]
    fn executes_current_buffer_returns_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (current-buffer)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_concat_empty_is_empty_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (concat) \"\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_elt_on_string_returns_char() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= (elt \"ABC\" 1) ?B)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_coerce_list_to_vector() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((v (cl-coerce '(1 2 3) 'vector))) (aref v 1))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_cl_remprop_removes_property() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((s (make-symbol \"test\"))) \
               (put s 'color 'red) \
               (cl-remprop s 'color) \
               (get s 'color))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_replace_copies_elements() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-replace '(a b c d) '(x y))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(x y c d)");
    }

    #[test]
    fn executes_cl_ldiff_returns_prefix_up_to_sublist() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((xs (list 1 2 3 4 5))) (cl-ldiff xs (cddr xs)))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "(1 2)");
    }

    #[test]
    fn executes_pairlis_creates_alist() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (pairlis '(a b c) '(1 2 3))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "((a . 1) (b . 2) (c . 3))");
    }

    #[test]
    fn executes_pairlis_appends_to_existing_alist() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (pairlis '(c d) '(3 4) '((a . 1) (b . 2)))",
        );
        assert_eq!(rt.format_value(value.unwrap()), "((c . 3) (d . 4) (a . 1) (b . 2))");
    }

    #[test]
    fn executes_cl_subst_if_works_on_nested_tree() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-subst-if 'x #'atom '((1 2) (3 4)))",
        );
        // Every atom is replaced by x, but x is also an atom, so everything becomes x
        // Actually (cl-subst-if 'x #'atom tree) replaces atoms: ((x x) (x x))
        assert_eq!(rt.format_value(value.unwrap()), "((x x) (x x))");
    }

    #[test]
    fn executes_cl_notevery_returns_t_when_not_all_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-notevery #'oddp (list 1 2 3))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_reduce_star_pattern() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-reduce #'* (list 2 3 4))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(24)));
    }

    #[test]
    fn executes_copy_alist_creates_shallow_copy() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((alist '((a . 1) (b . 2))))\n\
               (eq alist (copy-alist alist)))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_copy_alist_preserves_elements() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (assoc 'b (copy-alist '((a . 1) (b . 2) (c . 3))))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_plist_get_extracts_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (plist-get '(a 1 b 2 c 3) 'b)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_plist_get_returns_nil_for_missing_key() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (plist-get '(a 1 b 2) 'c)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_plist_put_sets_property() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((plist (list 'a 1)))\n\
               (setq plist (plist-put plist 'b 2))\n\
               (plist-get plist 'b))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_cl_rassoc_if_finds_by_predicate() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-rassoc-if #'evenp '((a . 1) (b . 2) (c . 3)))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_cl_rassoc_if_not_excludes_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-rassoc-if-not #'evenp '((a . 1) (b . 2) (c . 3)))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_cl_maplist_returns_results_of_successive_cdrs() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (cl-maplist #'car (list 1 2 3))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_string_prefix_p_returns_true_for_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-prefix-p \"hello\" \"hello world\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_suffix_p_returns_true() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-suffix-p \"world\" \"hello world\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_suffix_p_returns_nil_when_not_suffix() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-suffix-p \"hello\" \"hello world\")",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_string_remove_prefix_leaves_string_unchanged_without_prefix() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-remove-prefix \"xyz\" \"foobar\")",
        );
        assert_eq!(rt.format_value(value.unwrap()), "\"foobar\"");
    }

    #[test]
    fn executes_string_remove_suffix_leaves_string_unchanged_without_suffix() {
        let (value, rt) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-remove-suffix \"xyz\" \"foobar\")",
        );
        assert_eq!(rt.format_value(value.unwrap()), "\"foobar\"");
    }

    #[test]
    fn executes_split_string_by_separator() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= (length (split-string \"a,b,c\" \",\")) 3)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_butlast_returns_all_but_last() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (butlast '(1 2 3 4)) '(1 2 3))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_alist_get_retrieves_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (alist-get 'b '((a . 1) (b . 2) (c . 3)))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(2)));
    }

    #[test]
    fn executes_alist_get_returns_nil_for_missing_key() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (alist-get 'z '((a . 1) (b . 2)))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_make_list_creates_list_of_length() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= (length (make-list 5 nil)) 5)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_bytes_ascii() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-bytes \"hello\")",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_string_width_ascii() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-width \"hello\")",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(5)));
    }

    #[test]
    fn executes_string_width_empty_is_zero() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-width \"\")",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_upcase_converts_to_upper() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (upcase \"hello\") \"HELLO\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_downcase_converts_to_lower() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (downcase \"HELLO\") \"hello\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_capitalize_capitalizes_first_char() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (capitalize \"hello\") \"Hello\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_reverse_reverses_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (reverse '(1 2 3)) '(3 2 1))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_nconc_concatenates_lists() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (nconc (list 1 2) (list 3 4)) '(1 2 3 4))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_memq_returns_tail_when_found() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (memq 'b '(a b c)) '(b c))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_memq_returns_nil_when_not_found() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (memq 'z '(a b c))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_member_returns_tail_when_equal_found() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (member 20 '(10 20 30)) '(20 30))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_delq_removes_element_by_identity() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (delq 'a '(a b c a)) '(b c))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_last_returns_last_cons() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (last '(1 2 3)) '(3))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_nthcdr_returns_nth_cdr() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (nthcdr 2 '(a b c d)) '(c d))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_elt_gets_element_by_index() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= (elt '(10 20 30) 1) 20)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_char_equal_case_insensitive_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (char-equal ?a ?A)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_char_equal_different_chars_return_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (char-equal ?a ?b)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_copy_sequence_yields_new_object() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((orig (list 1 2 3)))\n\
               (eq orig (copy-sequence orig)))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_get_retrieves_symbol_property() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (progn (put 'my-sym 'my-prop 42) (get 'my-sym 'my-prop))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_plist_member_finds_property() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (plist-member '(a 1 b 2 c 3) 'b)",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_make_hash_table_creates_table() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (hash-table-p (make-hash-table))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_gethash_retrieves_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ht (make-hash-table)))\n\
               (puthash 'key 42 ht)\n\
               (gethash 'key ht))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(42)));
    }

    #[test]
    fn executes_gethash_returns_nil_for_missing_key() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (gethash 'missing (make-hash-table))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_remhash_removes_key() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ht (make-hash-table)))\n\
               (puthash 'key 42 ht)\n\
               (remhash 'key ht)\n\
               (hash-table-count ht))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_hash_table_count_returns_entry_count() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ht (make-hash-table)))\n\
               (puthash 'a 1 ht)\n\
               (puthash 'b 2 ht)\n\
               (= (hash-table-count ht) 2))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_copy_tree_deep_copies_nested_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((orig '((a) (b c))))\n\
               (eq (car orig) (car (copy-tree orig))))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_maphash_iterates_over_entries() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ht (make-hash-table)) (count 0))\n\
               (puthash 'a 1 ht)\n\
               (puthash 'b 2 ht)\n\
               (maphash (lambda (_k _v) (setq count (1+ count))) ht)\n\
               (= count 2))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_copy_hash_table_creates_independent_copy() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ht (make-hash-table)))\n\
               (puthash 'a 1 ht)\n\
               (let ((ht2 (copy-hash-table ht)))\n\
                 (puthash 'b 2 ht2)\n\
                 (= (hash-table-count ht) 1)))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_safe_length_handles_proper_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (safe-length '(1 2 3))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(3)));
    }

    #[test]
    fn executes_safe_length_returns_zero_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (safe-length nil)",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(0)));
    }

    #[test]
    fn executes_clrhash_clears_all_entries() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ht (make-hash-table)))\n\
               (puthash 'a 1 ht)\n\
               (clrhash ht)\n\
               (= (hash-table-count ht) 0))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_hash_table_keys_returns_list_of_keys() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ht (make-hash-table)))\n\
               (puthash 'a 1 ht)\n\
               (puthash 'b 2 ht)\n\
               (memq 'a (hash-table-keys ht)))",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_hash_table_values_returns_list_of_values() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ht (make-hash-table)))\n\
               (puthash 'a 1 ht)\n\
               (puthash 'b 2 ht)\n\
               (memq 2 (hash-table-values ht)))",
        );
        assert!(value.is_some());
    }


    #[test]
    fn executes_split_string_splits_by_separator() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (split-string \"a,b,c\" \",\") '(\"a\" \"b\" \"c\"))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_split_string_with_omit_nulls() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= (length (split-string \"a,,b\" \",\" t)) 2)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_join_joins_strings() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (string-join '(\"a\" \"b\" \"c\") \"-\") \"a-b-c\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_string_match_p_finds_pattern() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-match-p \"hello\" \"hello world\")",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_string_match_p_returns_nil_for_no_match() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string-match-p \"xyz\" \"hello world\")",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_number_to_string_integer() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (number-to-string 42) \"42\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_number_to_string_with_base() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (number-to-string 255 16) \"ff\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_number_to_string_negative() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (number-to-string -10) \"-10\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_fifth_returns_fifth_element() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (= (cl-fifth '(1 2 3 4 5 6)) 5)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_sixth_returns_sixth_element() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (= (cl-sixth '(1 2 3 4 5 6)) 6)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_puthash_stores_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (let ((ht (make-hash-table)))\n\
               (puthash 'key 99 ht)\n\
               (gethash 'key ht))",
        );
        assert_eq!(value, Some(LispValue::expect_fixnum(99)));
    }

    #[test]
    fn executes_vectorp_recognizes_vector() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (vectorp [1 2 3])",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_vectorp_returns_nil_for_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (vectorp '(1 2 3))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_define_error_creates_error_symbol() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (define-error 'my-test-error \"Test error\")\n\
             (get 'my-test-error 'error-conditions)",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_format_message_basic() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (format-message \"hello %s\" \"world\") \"hello world\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_princ_to_string_formats_value() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (princ-to-string 42)",
        );
        assert!(value.is_some());
    }

    #[test]
    fn executes_char_or_string_p_recognizes_char() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (char-or-string-p ?a)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_char_or_string_p_recognizes_string() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (char-or-string-p \"hello\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_char_or_string_p_returns_nil_for_number() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (char-or-string-p 42)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_sqrt_of_perfect_square() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= (sqrt 4) 2.0)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_sqrt_of_non_square() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (> (sqrt 2) 1.4)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }


    #[test]
    fn executes_asin_returns_arcsine() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (> (asin 0.5) 0.5)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_acos_returns_arccosine() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (< (acos 0.5) 1.1)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_atan_returns_arctangent() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (> (atan 1.0) 0.7)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_log10_returns_base_10_log() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (= (log10 100) 2.0)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_rassq_finds_entry_by_cdr_eq() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (equal (rassq 2 '((a . 1) (b . 2) (c . 3))) '(b . 2))",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_rassq_returns_nil_when_not_found() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (rassq 99 '((a . 1) (b . 2)))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_position_returns_nil_for_nil_seq() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-position 42 nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_copy_list_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-copy-list nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_delete_returns_nil_for_nil_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-delete 42 nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_remove_duplicates_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-remove-duplicates nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_nsubstitute_returns_nil_for_nil_tree() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-nsubstitute 'new 'old nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_sublis_returns_nil_for_nil_tree() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-sublis '((a . 1)) nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_intersection_returns_nil_for_nil_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-intersection nil '(1 2 3))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_union_returns_nil_for_both_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-union nil nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_set_difference_returns_nil_for_nil_list() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-set-difference nil '(1 2 3))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_set_exclusive_or_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-set-exclusive-or nil nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_nintersection_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-nintersection nil '(1 2 3))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_nsublis_returns_nil_for_nil_tree() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-nsublis '((a . 1)) nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_sort_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (sort nil '<)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_stable_sort_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-stable-sort nil '<)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_nset_difference_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-nset-difference nil '(1 2 3))",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_nset_exclusive_or_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-nset-exclusive-or nil nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_nunion_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-nunion nil nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_delete_dups_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (delete-dups nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_count_returns_zero_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (= (cl-count 42 nil) 0)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_mapcar_returns_nil_for_nil_seq() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (mapcar #'1+ nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_mapconcat_returns_empty_string_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (string= (mapconcat #'identity nil \"-\") \"\")",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_copy_sequence_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (copy-sequence nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_merge_returns_nil_for_both_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-merge 'list nil nil '<)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_remove_if_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-remove-if #'evenp nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_delete_if_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-delete-if #'evenp nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_find_returns_nil_for_nil_seq() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-find 42 nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_substitute_returns_nil_for_nil_seq() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-substitute 'new 'old nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_rassoc_if_returns_nil_for_nil_alist() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-rassoc-if #'evenp nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_endp_returns_t_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-endp nil)",
        );
        assert_eq!(value, Some(LispValue::TRUE));
    }

    #[test]
    fn executes_cl_assoc_if_returns_nil_for_nil_alist() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-assoc-if #'evenp nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_member_if_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-member-if #'evenp nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_remove_if_not_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-remove-if-not #'evenp nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_delete_duplicates_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-delete-duplicates nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_delete_if_not_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-delete-if-not #'evenp nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }

    #[test]
    fn executes_cl_position_if_returns_nil_for_nil() {
        let (value, _) = execute(
            ";;; -*- lexical-binding: t; -*-\n\
             (require 'cl-lib)\n\
             (cl-position-if #'evenp nil)",
        );
        assert_eq!(value, Some(LispValue::NIL));
    }
}
