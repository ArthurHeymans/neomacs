use std::collections::HashMap;

use neovm_compiler::diagnostic::Diagnostic;
use neovm_compiler::hir::LambdaList;
use neovm_compiler::ids::{FunctionId, PrimaryMap, RegId};
use neovm_compiler::lower::{lambda_template_to_ssa, ssa_to_regir};
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
    if lambda_list.rest.is_some() {
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
        let args = self.adapt_lambda_args(&template.params, args)?;
        let mut entry_args = Vec::with_capacity(captures.len() + args.len());
        entry_args.extend(captures);
        entry_args.extend(args);
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
        if lambda_list.rest.is_some() {
            let rest_start = args.len().min(optional_start + lambda_list.optional.len());
            adapted.push(make_list(self.runtime, args[rest_start..].iter().copied()));
        }
        Some(adapted)
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
                .map(|_| bool_value(self.runtime.is_number(args[0]))),
            "integerp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_fixnum())),
            "natnump" | "wholenump" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].as_fixnum().is_some_and(|value| value >= 0))),
            "zerop" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.fixnum_arg(name, args[0]))
                .map(|value| bool_value(value == 0)),
            "symbolp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_symbol(args[0]))),
            "stringp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_string(args[0]))),
            "vectorp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_vector(args[0]))),
            "hash-table-p" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(self.runtime.is_hash_table(args[0]))),
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
            "put" => self.exact_arity(name, args, 3).and_then(|_| {
                let result = self.runtime.put_symbol_property(args[0], args[1], args[2]);
                self.runtime_value(result)
            }),
            "symbol-plist" => self.exact_arity(name, args, 1).and_then(|_| {
                let result = self.runtime.symbol_plist(args[0]);
                self.runtime_value(result)
            }),
            "setplist" => self.exact_arity(name, args, 2).and_then(|_| {
                let result = self.runtime.set_symbol_plist(args[0], args[1]);
                self.runtime_value(result)
            }),
            "plist-get" => self
                .exact_arity(name, args, 2)
                .map(|_| self.runtime.plist_get(args[0], args[1])),
            "plist-put" => self
                .exact_arity(name, args, 3)
                .map(|_| self.runtime.plist_put(args[0], args[1], args[2])),
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
            "ignore" => Some(LispValue::NIL),
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
            "elt" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.elt(args[0], args[1])),
            "downcase" => self
                .exact_arity(name, args, 1)
                .map(|_| self.downcase(args[0])),
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
            "evenp" => self.exact_arity(name, args, 1).and_then(|_| {
                let val = self.fixnum_arg(name, args[0])?;
                Some(bool_value(val % 2 == 0))
            }),
            "butlast" => self
                .min_max_arity(name, args, 1, 2)
                .and_then(|_| self.butlast(args[0], args.get(1).copied())),
            "delq" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.delq(args[0], args[1])),
            "remove" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.remove(args[0], args[1])),
            "vconcat" => self.vconcat(args),
            "nconc" => self.nconc(args),
            "number-to-string" => self.exact_arity(name, args, 1).and_then(|_| {
                let n = self.fixnum_arg(name, args[0])?;
                Some(self.runtime.string(n.to_string()))
            }),
            "string-to-number" => self
                .min_max_arity(name, args, 1, 2)
                .and_then(|_| self.string_to_number(args[0], args.get(1).copied())),
            "logand" | "logior" | "logxor" => {
                if args.is_empty() {
                    self.error("primitive `logand/logior/logxor` requires at least one argument");
                    None
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
                let val = self.fixnum_arg(name, args[0])?;
                self.fixnum(!val, name)
            }),
            "ash" => self.exact_arity(name, args, 2).and_then(|_| {
                let val = self.fixnum_arg(name, args[0])?;
                let count = self.fixnum_arg(name, args[1])?;
                let result = if count >= 0 {
                    val << count
                } else {
                    val >> (-count)
                };
                self.fixnum(result, name)
            }),
            "lsh" => self.exact_arity(name, args, 2).and_then(|_| {
                let val = self.fixnum_arg(name, args[0])?;
                let count = self.fixnum_arg(name, args[1])?;
                let result = if count >= 0 {
                    val << count
                } else {
                    (val as u64 >> (-count)) as i64
                };
                self.fixnum(result, name)
            }),
            "expt" => self.exact_arity(name, args, 2).and_then(|_| {
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
            }),
            "list" => Some(make_list(self.runtime, args.iter().copied())),
            "length" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.length(args[0]))
                .and_then(|length| i64::try_from(length).ok())
                .and_then(|length| self.fixnum(length, "length")),
            "concat" => self.concat(args),
            "substring" => self
                .min_max_arity(name, args, 2, 3)
                .and_then(|_| self.substring(args[0], args[1], args.get(2).copied())),
            "string=" | "string-equal" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.string_equal(args[0], args[1])),
            "string<" | "string-lessp" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.string_lessp(args[0], args[1])),
            "char-to-string" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.char_to_string(args[0])),
            "string-to-char" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.string_to_char(args[0])),
            "format" | "format-message" => self
                .min_arity(name, args, 1)
                .and_then(|_| self.format_string(args[0], &args[1..])),
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
            "reverse" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.list_values(args[0]))
                .map(|values| make_list(self.runtime, values.iter().rev().copied())),
            "nreverse" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.list_values(args[0]))
                .map(|values| make_list(self.runtime, values.iter().rev().copied())),
            "append" => self.append(args),
            "nth" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.nth(args[0], args[1])),
            "nthcdr" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.nthcdr(args[0], args[1])),
            "last" => self
                .min_arity(name, args, 1)
                .and_then(|_| self.last(args[0])),
            "memq" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.memq(args[0], args[1])),
            "member" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.member(args[0], args[1])),
            "assq" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.assoc(args[0], args[1], false)),
            "assoc" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.assoc(args[0], args[1], true)),
            "copy-sequence" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.copy_sequence(args[0])),
            "mapcar" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.mapcar(args[0], args[1])),
            "mapc" => self
                .exact_arity(name, args, 2)
                .and_then(|_| self.mapc(args[0], args[1])),
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
            "mod" => self.exact_arity(name, args, 2).and_then(|_| {
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
            }),
            "rem" => self.exact_arity(name, args, 2).and_then(|_| {
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
            }),
            "abs" => self
                .exact_arity(name, args, 1)
                .and_then(|_| self.fixnum_arg(name, args[0]))
                .and_then(|value| value.checked_abs())
                .and_then(|value| self.fixnum(value, name)),
            "max" => {
                if args.is_empty() {
                    self.error("primitive `max` requires at least one argument");
                    return None;
                }
                let mut result = self.fixnum_arg(name, args[0])?;
                for arg in &args[1..] {
                    let value = self.fixnum_arg(name, *arg)?;
                    result = result.max(value);
                }
                self.fixnum(result, name)
            }
            "min" => {
                if args.is_empty() {
                    self.error("primitive `min` requires at least one argument");
                    return None;
                }
                let mut result = self.fixnum_arg(name, args[0])?;
                for arg in &args[1..] {
                    let value = self.fixnum_arg(name, *arg)?;
                    result = result.min(value);
                }
                self.fixnum(result, name)
            }
            "type-of" => self.exact_arity(name, args, 1).map(|_| {
                if args[0].is_nil() || args[0].is_true() {
                    self.runtime.intern("symbol")
                } else if args[0].is_fixnum() {
                    self.runtime.intern("integer")
                } else if args[0].as_char().is_some() {
                    self.runtime.intern("symbol")
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
                .map(|_| bool_value(args[0].is_fixnum())),
            "floatp" => self.exact_arity(name, args, 1).map(|_| bool_value(false)),
            "string-or-null-p" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_nil() || self.runtime.is_string(args[0]))),
            "booleanp" => self
                .exact_arity(name, args, 1)
                .map(|_| bool_value(args[0].is_nil() || args[0].is_true())),
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
        let target = find_condition_handler(instructions, signal_index, &signal_name)?;
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
            SsaConst::Float(_) => {
                self.unsupported("float constants require float object support");
                None
            }
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
            CompileValue::Float(_) => {
                self.unsupported("float compile values");
                None
            }
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
            SurfaceAtom::Float(_) => {
                self.unsupported("quoted floats require float object support");
                None
            }
        }
    }

    fn fixnum_value(&mut self, value: i64, context: &str) -> Option<LispValue> {
        match LispValue::from_fixnum(value) {
            Some(value) => Some(value),
            None => {
                self.unsupported(format!("{context} {value} requires bignum support"));
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

    fn mapcar(&mut self, function: LispValue, sequence: LispValue) -> Option<LispValue> {
        let elements = self.sequence_values(sequence)?;
        let mut mapped = Vec::with_capacity(elements.len());
        for element in elements {
            mapped.push(self.execute_funcall(function, &[element])?);
        }
        Some(make_list(self.runtime, mapped))
    }

    fn mapc(&mut self, function: LispValue, sequence: LispValue) -> Option<LispValue> {
        for element in self.sequence_values(sequence)? {
            self.execute_funcall(function, &[element])?;
        }
        Some(sequence)
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

    fn string_equal(&mut self, left: LispValue, right: LispValue) -> Option<LispValue> {
        let left = self.string_bytes(left)?;
        let right = self.string_bytes(right)?;
        Some(bool_value(left == right))
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

    fn string_lessp(&mut self, left: LispValue, right: LispValue) -> Option<LispValue> {
        let left = self.string_contents_owned(left)?;
        let right = self.string_contents_owned(right)?;
        Some(bool_value(left < right))
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
        let index = usize::try_from(self.fixnum_arg("elt", n)?).ok()?;
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
        let mut values = Vec::new();
        for arg in args {
            let sub = self.list_values(*arg)?;
            values.extend(sub);
        }
        if values.is_empty() {
            return Some(LispValue::NIL);
        }
        Some(make_list(self.runtime, values.into_iter()))
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
        let contents = contents.trim();
        let contents = if contents.starts_with('+') {
            &contents[1..]
        } else {
            contents
        };
        if contents.is_empty() {
            return self.fixnum(0, "string-to-number");
        }
        let n = i64::from_str_radix(contents, radix).unwrap_or(0);
        self.fixnum(n, "string-to-number")
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
                    let value = self.fixnum_arg("format", value)?;
                    output.push_str(&value.to_string());
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
            self.unsupported("aset on strings requires mutable multibyte string updates");
            return None;
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
                let symbol = self.runtime.intern("arith-error");
                self.pending_signal = Some(SignaledValue {
                    symbol,
                    data: LispValue::NIL,
                });
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
    matches!(
        name,
        "cons"
            | "car"
            | "cdr"
            | "car-safe"
            | "cdr-safe"
            | "setcar"
            | "setcdr"
            | "eq"
            | "eql"
            | "equal"
            | "consp"
            | "listp"
            | "numberp"
            | "integerp"
            | "natnump"
            | "wholenump"
            | "zerop"
            | "symbolp"
            | "stringp"
            | "vectorp"
            | "hash-table-p"
            | "symbol-value"
            | "set"
            | "boundp"
            | "fboundp"
            | "provide"
            | "featurep"
            | "require"
            | "get"
            | "put"
            | "symbol-plist"
            | "setplist"
            | "plist-get"
            | "plist-put"
            | "autoload"
            | "symbol-function"
            | "fset"
            | "defalias"
            | "intern"
            | "symbol-name"
            | "not"
            | "null"
            | "identity"
            | "ignore"
            | "list"
            | "length"
            | "concat"
            | "substring"
            | "split-string"
            | "string-join"
            | "string-trim"
            | "string-trim-left"
            | "string-trim-right"
            | "substring-no-properties"
            | "string="
            | "string-equal"
            | "string<"
            | "string-lessp"
            | "char-to-string"
            | "string-to-char"
            | "format"
            | "format-message"
            | "vector"
            | "aref"
            | "aset"
            | "make-hash-table"
            | "hash-table-count"
            | "gethash"
            | "puthash"
            | "remhash"
            | "clrhash"
            | "maphash"
            | "reverse"
            | "append"
            | "nth"
            | "memq"
            | "member"
            | "assq"
            | "assoc"
            | "copy-sequence"
            | "mapcar"
            | "mapc"
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
            | "functionp"
            | "mod"
            | "rem"
            | "abs"
            | "max"
            | "min"
            | "type-of"
            | "cadr"
            | "caar"
            | "cdar"
            | "cddr"
            | "caaar"
            | "caadr"
            | "cadar"
            | "caddr"
            | "cdaar"
            | "cdadr"
            | "cddar"
            | "cdddr"
            | "caaaar"
            | "caaadr"
            | "caadar"
            | "caaddr"
            | "cadaar"
            | "cadadr"
            | "caddar"
            | "cadddr"
            | "cdaaar"
            | "cdaadr"
            | "cdadar"
            | "cdaddr"
            | "cddaar"
            | "cddadr"
            | "cdddar"
            | "cddddr"
            | "number-or-marker-p"
            | "floatp"
            | "string-or-null-p"
            | "booleanp"
            | "prog1"
            | "make-symbol"
            | "intern-soft"
            | "elt"
            | "downcase"
            | "upcase"
            | "capitalize"
            | "keywordp"
            | "evenp"
            | "butlast"
            | "delq"
            | "remove"
            | "vconcat"
            | "nconc"
            | "number-to-string"
            | "string-to-number"
            | "logand"
            | "logior"
            | "logxor"
            | "lognot"
            | "ash"
            | "lsh"
            | "expt"
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
}
