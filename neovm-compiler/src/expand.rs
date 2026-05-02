use std::collections::HashMap;

use crate::diagnostic::Diagnostic;
use crate::expand_eval::{MacroEnv, MacroEval};
use crate::expand_value::{surface_to_value, value_to_surface, MacroValue};
use crate::source::{SourceId, Span};
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

enum Work {
    Expand(SurfaceForm),
    RejoinDotted(Span, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExpandOutput {
    pub forms: Vec<SurfaceForm>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn expand_forms(forms: Vec<SurfaceForm>) -> ExpandOutput {
    let mut expander = Expander {
        macros: HashMap::new(),
        diagnostics: Vec::new(),
    };
    let mut expanded_forms = Vec::new();
    for form in forms {
        if let Some(defalias_form) = expander.register_top_level_macro(&form) {
            expanded_forms.push(defalias_form);
        } else {
            expanded_forms.push(expander.expand_form(form));
        }
    }
    ExpandOutput {
        forms: expanded_forms,
        diagnostics: expander.diagnostics,
    }
}

struct Expander {
    macros: HashMap<String, MacroDef>,
    diagnostics: Vec<Diagnostic>,
}

impl Expander {
    fn register_top_level_macro(&mut self, form: &SurfaceForm) -> Option<SurfaceForm> {
        let SurfaceKind::List(items) = &form.kind else {
            return None;
        };
        if items.first().and_then(SurfaceForm::symbol_name) != Some("defmacro") {
            return None;
        }
        if items.len() < 4 {
            self.error(
                form.span,
                "defmacro requires a name, parameter list, and body",
            );
            return None;
        }
        let Some(name) = items[1].symbol_name().map(str::to_string) else {
            self.error(items[1].span, "defmacro name must be a symbol");
            return None;
        };
        let Some(params) = self.parse_macro_params(&items[2]) else {
            return None;
        };
        let mut body = &items[3..];
        if matches!(
            body.first().map(|form| &form.kind),
            Some(SurfaceKind::Atom(SurfaceAtom::String(_)))
        ) {
            body = &body[1..];
        }
        while let Some(first) = body.first()
            && list_head_symbol(first) == Some("declare")
        {
            body = &body[1..];
        }
        let body = if body.is_empty() {
            vec![nil_form(form.span)]
        } else {
            body.to_vec()
        };
        let def = MacroDef {
            params,
            body,
            span: form.span,
        };
        self.macros.insert(name.clone(), def.clone());
        Some(macro_defalias_form(&name, &def, form.span))
    }

    fn expand_form(&mut self, form: SurfaceForm) -> SurfaceForm {
        // Stack-based tree traversal to avoid blowing the call stack on
        // deeply nested Elisp forms. Each Work item expands one SurfaceForm.
        // When expansion produces sub-forms that also need expanding, they
        // are pushed as Work items instead of recursing.
        let mut stack = vec![Work::Expand(form)];
        let mut results: Vec<SurfaceForm> = Vec::new();

        while let Some(work) = stack.pop() {
            match work {
                Work::Expand(form) => {
                    match form.kind {
                        SurfaceKind::List(items) => {
                            let expanded = self.expand_single_list(form.span, items);
                            // expand_single_list may have already fully expanded
                            // sub-forms (in the non-macro, non-special case) or
                            // may have returned a form whose sub-forms still need
                            // expansion. Check by structure.
                            results.push(expanded);
                        }
                        SurfaceKind::DottedList(items, tail) => {
                            // Push a reunion task, then push sub-forms in reverse
                            // so they are processed left-to-right.
                            let count = items.len() + 1; // items + tail
                            stack.push(Work::RejoinDotted(form.span, count));
                            stack.push(Work::Expand(*tail));
                            for item in items.into_iter().rev() {
                                stack.push(Work::Expand(item));
                            }
                        }
                        SurfaceKind::Vector(_) => {
                            results.push(form);
                        }
                        SurfaceKind::Quote(_)
                        | SurfaceKind::FunctionQuote(_)
                        | SurfaceKind::Backquote(_)
                        | SurfaceKind::Comma(_)
                        | SurfaceKind::CommaAt(_)
                        | SurfaceKind::Atom(_) => {
                            results.push(form);
                        }
                    }
                }
                Work::RejoinDotted(span, count) => {
                    // Collect `count` results from the top of results, build DottedList.
                    let split = results.len() - count;
                    let tail_idx = split + count - 1;
                    let tail = results.swap_remove(tail_idx);
                    let items: Vec<SurfaceForm> = results.drain(split..).collect();
                    results.push(SurfaceForm::new(
                        SurfaceKind::DottedList(items, Box::new(tail)),
                        span,
                    ));
                }
            }
        }

        results.pop().unwrap_or_else(|| SurfaceForm::new(
            SurfaceKind::Atom(SurfaceAtom::Nil),
            Span::new(SourceId::new(0), 0, 0),
        ))
    }

    /// Expand a single list form (non-recursive: sub-forms are expanded
    /// iteratively via the work stack in expand_form).
    fn expand_single_list(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        let Some(head) = items.first().and_then(SurfaceForm::symbol_name) else {
            // Empty or non-symbol-headed list: expand each sub-form.
            return SurfaceForm::new(
                SurfaceKind::List(
                    items
                        .into_iter()
                        .map(|item| self.expand_form(item))
                        .collect(),
                ),
                span,
            );
        };
        if let Some(def) = self.macros.get(head).cloned() {
            return self.expand_macro_call(span, items, def);
        }
        match head {
            "quote" | "function" => SurfaceForm::new(SurfaceKind::List(items), span),
            "push" => self.expand_push(span, items),
            "pop" => self.expand_pop(span, items),
            "if-let*" => self.expand_if_let(span, items),
            "when-let*" => self.expand_when_let(span, items),
            // declare-function is a compile-time declaration — discard
            "declare-function" => nil_form(span),
            // pcase-let* -> let* (simplified: extracts symbol bindings, ignores destructuring patterns)
            "pcase-let*" => {
                if items.len() >= 3 {
                    // items[1] = ((pattern expr) ...), items[2..] = body
                    let bindings_form = &items[1];
                    let body: Vec<SurfaceForm> = items[2..].iter().map(|f| self.expand_form(f.clone())).collect();
                    // Convert pcase bindings to let* bindings: ((pat expr) ...) -> ((sym expr) ...)
                    let simple_bindings = self.simplify_pcase_bindings(bindings_form);
                    let mut result = vec![symbol_form("let*", span), simple_bindings];
                    result.extend(body);
                    list_form(result, span)
                } else {
                    let expanded: Vec<SurfaceForm> = items.into_iter().map(|f| self.expand_form(f)).collect();
                    SurfaceForm::new(SurfaceKind::List(expanded), span)
                }
            }
            // cl-with-gensyms -> let (simplified: uses symbol names as-is)
            "cl-with-gensyms" => {
                if items.len() >= 3 {
                    let bindings = items[1].clone();
                    let body: Vec<SurfaceForm> = items[2..].iter().map(|f| self.expand_form(f.clone())).collect();
                    list_form(
                        vec![symbol_form("let", span), bindings.clone()].into_iter().chain(body).collect(),
                        span,
                    )
                } else {
                    nil_form(span)
                }
            }
            // cl-check-type -> progn (evaluate the form, ignore type check)
            "cl-check-type" => {
                if items.len() >= 2 {
                    self.expand_form(items[1].clone())
                } else {
                    nil_form(span)
                }
            }
            // cl-assert -> progn (evaluate the assertion form)
            "cl-assert" => {
                if items.len() >= 2 {
                    self.expand_form(items[1].clone())
                } else {
                    nil_form(span)
                }
            }
            "destructuring-bind" => self.expand_destructuring_bind(span, items),
            "flet" => self.expand_flet(span, items),
            "labels" | "cl-labels" => self.expand_labels(span, items),
            _ => SurfaceForm::new(
                SurfaceKind::List(
                    items
                        .into_iter()
                        .map(|item| self.expand_form(item))
                        .collect(),
                ),
                span,
            ),
        }
    }

    fn expand_macro_call(
        &mut self,
        span: Span,
        items: Vec<SurfaceForm>,
        def: MacroDef,
    ) -> SurfaceForm {
        // First expansion attempt.
        let mut form = match self.invoke_macro(&def, &items[1..]) {
            Some(expanded) => expanded,
            None => {
                // Expansion failed. Return the original form without further
                // expansion attempts so we don't loop on failing macros.
                return SurfaceForm::new(SurfaceKind::List(items), span);
            }
        };

        // Iteratively re-expand if the result is another macro call.
        for _ in 0..100 {
            let SurfaceKind::List(ref expansion_items) = form.kind else {
                break;
            };
            let Some(head) = expansion_items.first().and_then(SurfaceForm::symbol_name) else {
                break;
            };
            let Some(next_def) = self.macros.get(head).cloned() else {
                break;
            };
            let expansion_span = form.span;
            let expansion_items = match std::mem::replace(&mut form.kind, SurfaceKind::Atom(SurfaceAtom::Nil)) {
                SurfaceKind::List(items) => items,
                _ => unreachable!(),
            };
            form = match self.invoke_macro(&next_def, &expansion_items[1..]) {
                Some(expanded) => expanded,
                None => {
                    // Expansion failed. Stop re-expanding to avoid infinite loops.
                    form = SurfaceForm::new(SurfaceKind::List(expansion_items), expansion_span);
                    break;
                }
            };
        }

        // Tree-expand sub-forms (but don't recurse back into expand_macro_call
        // for this form — only for nested sub-forms).
        self.expand_form(form)
    }

    fn invoke_macro(&mut self, def: &MacroDef, args: &[SurfaceForm]) -> Option<SurfaceForm> {
        let arg_values: Vec<MacroValue> = args.iter().map(surface_to_value).collect();

        if arg_values.len() < def.params.required.len() {
            // Arity mismatch — likely due to incomplete macro loading, pass through
            return None;
        }
        let max_arity = def
            .params
            .rest
            .is_none()
            .then_some(def.params.required.len() + def.params.optional.len());
        if let Some(max_arity) = max_arity
            && arg_values.len() > max_arity
        {
            self.error(
                def.span,
                format!(
                    "macro requires at most {max_arity} arguments, got {}",
                    arg_values.len()
                ),
            );
            return None;
        }

        let mut env = MacroEnv::default();
        for (name, arg) in def.params.required.iter().zip(arg_values.iter()) {
            env.bind(name.clone(), arg.clone());
        }
        let optional_start = def.params.required.len();
        for (index, name) in def.params.optional.iter().enumerate() {
            env.bind(
                name.clone(),
                arg_values.get(optional_start + index)
                    .cloned()
                    .unwrap_or(MacroValue::Nil),
            );
        }
        if let Some(rest) = &def.params.rest {
            let rest_start = arg_values.len().min(optional_start + def.params.optional.len());
            env.bind(
                rest.clone(),
                MacroValue::list(arg_values[rest_start..].to_vec()),
            );
        }
        if let Some(environment) = &def.params.environment {
            env.bind(environment.clone(), MacroValue::Nil);
        }

        let mut macro_eval = MacroEval::new();
        match macro_eval.eval_progn(&def.body, &mut env) {
            Ok(result) => {
                self.diagnostics.extend(macro_eval.into_diagnostics());
                Some(value_to_surface(&result, def.span))
            }
            Err(()) => {
                self.diagnostics.extend(macro_eval.into_diagnostics());
                None
            }
        }
    }

    fn expand_push(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() != 3 {
            // Wrong arity — expand sub-forms, pass through
            let expanded: Vec<SurfaceForm> = items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let Some(place) = items[2].symbol_name().map(str::to_string) else {
            // Non-symbol place (e.g., list access) — expand sub-forms, pass through
            let expanded: Vec<SurfaceForm> = items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        };
        let value = items[1].clone();
        let expanded = list_form(
            vec![
                symbol_form("setq", span),
                symbol_form(&place, items[2].span),
                list_form(
                    vec![
                        symbol_form("cons", span),
                        value,
                        symbol_form(&place, items[2].span),
                    ],
                    span,
                ),
            ],
            span,
        );
        self.expand_form(expanded)
    }

    fn expand_pop(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() != 2 {
            // Wrong arity — expand sub-forms, pass through
            let expanded: Vec<SurfaceForm> = items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let Some(place) = items[1].symbol_name().map(str::to_string) else {
            // Non-symbol place — expand sub-forms, pass through
            let expanded: Vec<SurfaceForm> = items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        };
        let place_span = items[1].span;
        let expanded = list_form(
            vec![
                symbol_form("car-safe", span),
                list_form(
                    vec![
                        symbol_form("prog1", span),
                        symbol_form(&place, place_span),
                        list_form(
                            vec![
                                symbol_form("setq", span),
                                symbol_form(&place, place_span),
                                list_form(
                                    vec![symbol_form("cdr", span), symbol_form(&place, place_span)],
                                    span,
                                ),
                            ],
                            span,
                        ),
                    ],
                    span,
                ),
            ],
            span,
        );
        self.expand_form(expanded)
    }

    /// Simplify pcase-let* bindings: ((pattern expr) ...) -> ((sym expr) ...)
    /// For non-symbol patterns, extract the expression but bind to a gensym-like name.
    fn simplify_pcase_bindings(&mut self, form: &SurfaceForm) -> SurfaceForm {
        let SurfaceKind::List(items) = &form.kind else {
            return form.clone();
        };
        let span = form.span;
        let bindings: Vec<SurfaceForm> = items
            .iter()
            .filter_map(|binding| {
                let SurfaceKind::List(binding_items) = &binding.kind else {
                    return None;
                };
                if binding_items.len() == 2 {
                    let pat = &binding_items[0];
                    let expr = &binding_items[1];
                    if let Some(name) = pat.symbol_name() {
                        Some(list_form(vec![symbol_form(name, pat.span), expr.clone()], span))
                    } else {
                        Some(list_form(vec![symbol_form("_", pat.span), expr.clone()], span))
                    }
                } else if binding_items.len() == 1 {
                    Some(list_form(vec![symbol_form("_", binding_items[0].span), binding_items[0].clone()], span))
                } else {
                    None
                }
            })
            .collect();
        list_form(bindings, span)
    }

    fn expand_if_let(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() < 3 {
            self.error(span, "if-let* requires bindings and a then form");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        let Some(bindings) = self.parse_if_let_bindings(&items[1]) else {
            return SurfaceForm::new(SurfaceKind::List(items), span);
        };
        let then_form = items[2].clone();
        let else_forms = items[3..].to_vec();
        let expanded = build_if_let_form(bindings, then_form, else_forms, span);
        self.expand_form(expanded)
    }

    fn expand_when_let(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() < 2 {
            self.error(span, "when-let* requires bindings");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        let Some(bindings) = self.parse_if_let_bindings(&items[1]) else {
            return SurfaceForm::new(SurfaceKind::List(items), span);
        };
        let then_form = list_form(
            std::iter::once(symbol_form("progn", span))
                .chain(items[2..].iter().cloned())
                .collect(),
            span,
        );
        let expanded = build_if_let_form(bindings, then_form, Vec::new(), span);
        self.expand_form(expanded)
    }

    fn parse_if_let_bindings(&mut self, form: &SurfaceForm) -> Option<Vec<IfLetBinding>> {
        let SurfaceKind::List(items) = &form.kind else {
            if matches!(form.kind, SurfaceKind::Atom(SurfaceAtom::Nil)) {
                return Some(Vec::new());
            }
            self.error(form.span, "if-let* bindings must be a proper list");
            return None;
        };
        items
            .iter()
            .enumerate()
            .map(|(index, item)| self.parse_if_let_binding(item, index))
            .collect()
    }

    fn parse_if_let_binding(&mut self, form: &SurfaceForm, index: usize) -> Option<IfLetBinding> {
        if let Some(name) = form.symbol_name() {
            return Some(IfLetBinding {
                name: name.to_string(),
                value: form.clone(),
                span: form.span,
            });
        }
        let SurfaceKind::List(items) = &form.kind else {
            self.error(
                form.span,
                "if-let* binding must be SYMBOL, (SYMBOL VALUE), or (VALUE)",
            );
            return None;
        };
        match items.as_slice() {
            [value] => Some(IfLetBinding {
                name: generated_if_let_name(form.span, index),
                value: value.clone(),
                span: form.span,
            }),
            [name, value] => {
                let Some(name) = name.symbol_name() else {
                    self.error(name.span, "if-let* binding name must be a symbol");
                    return None;
                };
                let name = if name == "_" {
                    generated_if_let_name(form.span, index)
                } else {
                    name.to_string()
                };
                Some(IfLetBinding {
                    name,
                    value: value.clone(),
                    span: form.span,
                })
            }
            _ => {
                self.error(
                    form.span,
                    "if-let* binding must be SYMBOL, (SYMBOL VALUE), or (VALUE)",
                );
                None
            }
        }
    }

    fn parse_macro_params(&mut self, form: &SurfaceForm) -> Option<MacroParams> {
        let SurfaceKind::List(items) = &form.kind else {
            self.error(form.span, "defmacro parameter list must be a proper list");
            return None;
        };
        let mut params = MacroParams::default();
        let mut section = MacroParamSection::Required;
        let mut index = 0;
        while index < items.len() {
            let item = &items[index];
            let Some(name) = item.symbol_name() else {
                self.error(item.span, "defmacro parameter name must be a symbol");
                return None;
            };
            match name {
                "&optional" => {
                    if section != MacroParamSection::Required {
                        self.error(item.span, "&optional is out of order");
                        return None;
                    }
                    section = MacroParamSection::Optional;
                    index += 1;
                    continue;
                }
                "&rest" | "&body" => {
                    if section == MacroParamSection::Rest {
                        self.error(item.span, "duplicate rest parameter");
                        return None;
                    }
                    section = MacroParamSection::Rest;
                    index += 1;
                    continue;
                }
                "&environment" => {
                    let Some(next) = items.get(index + 1) else {
                        self.error(item.span, "&environment requires a parameter");
                        return None;
                    };
                    let Some(environment) = next.symbol_name() else {
                        self.error(next.span, "&environment parameter must be a symbol");
                        return None;
                    };
                    params.environment = Some(environment.to_string());
                    index += 2;
                    continue;
                }
                _ if name.starts_with('&') => {
                    self.error(
                        item.span,
                        "defmacro lambda-list keyword is not supported yet",
                    );
                    return None;
                }
                _ => {}
            }
            match section {
                MacroParamSection::Required => params.required.push(name.to_string()),
                MacroParamSection::Optional => params.optional.push(name.to_string()),
                MacroParamSection::Rest => {
                    if params.rest.is_some() {
                        self.error(item.span, "rest accepts only one parameter");
                        return None;
                    }
                    params.rest = Some(name.to_string());
                }
            }
            index += 1;
        }
        if section == MacroParamSection::Rest && params.rest.is_none() {
            self.error(form.span, "rest requires a parameter");
            return None;
        }
        Some(params)
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message.into()).with_span(span));
    }

    fn expand_destructuring_bind(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (destructuring-bind pattern expr body...)
        if items.len() < 4 {
            let expanded: Vec<SurfaceForm> = items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let pattern = &items[1];
        let expr = &items[2];
        let body: Vec<SurfaceForm> = items[3..].to_vec();
        let expanded = self.destructure_pattern(pattern, expr.clone(), body, span, 0);
        self.expand_form(expanded)
    }

    fn destructure_pattern(
        &self,
        pattern: &SurfaceForm,
        expr: SurfaceForm,
        body: Vec<SurfaceForm>,
        span: Span,
        depth: usize,
    ) -> SurfaceForm {
        match &pattern.kind {
            SurfaceKind::Atom(atom) => {
                if let Some(name) = match atom {
                    SurfaceAtom::Symbol(s) => Some(s.as_str()),
                    SurfaceAtom::Nil => Some("nil"),
                    SurfaceAtom::True => Some("t"),
                    _ => None,
                } {
                    if name == "nil" || name == "_" {
                        let mut forms = vec![expr];
                        forms.extend(body);
                        return list_form(vec![symbol_form("progn", span)].into_iter().chain(forms).collect(), span);
                    }
                    let binding = list_form(vec![symbol_form(name, pattern.span), expr], pattern.span);
                    let mut result = vec![symbol_form("let", span), list_form(vec![binding], span)];
                    result.extend(body);
                    list_form(result, span)
                } else {
                    let mut forms = vec![expr];
                    forms.extend(body);
                    list_form(vec![symbol_form("progn", span)].into_iter().chain(forms).collect(), span)
                }
            }
            SurfaceKind::Quote(_) | SurfaceKind::FunctionQuote(_) => {
                let mut forms = vec![expr];
                forms.extend(body);
                list_form(vec![symbol_form("progn", span)].into_iter().chain(forms).collect(), span)
            }
            SurfaceKind::List(patterns) => {
                self.destructure_list_pattern(patterns, expr, body, span, depth)
            }
            _ => {
                let mut forms = vec![expr];
                forms.extend(body);
                list_form(vec![symbol_form("progn", span)].into_iter().chain(forms).collect(), span)
            }
        }
    }

    fn destructure_list_pattern(
        &self,
        patterns: &[SurfaceForm],
        expr: SurfaceForm,
        body: Vec<SurfaceForm>,
        span: Span,
        depth: usize,
    ) -> SurfaceForm {
        let tmp = symbol_form(&format!("\0dsb.{}.{}", depth, span.start), span);

        let mut required: Vec<SurfaceForm> = Vec::new();
        let mut optional: Vec<SurfaceForm> = Vec::new();
        let mut rest_pattern: Option<SurfaceForm> = None;

        let mut mode = 0;
        for pat in patterns {
            if let Some(name) = pat.symbol_name() {
                if name == "&optional" { mode = 1; continue; }
                if name == "&rest" { mode = 2; continue; }
            }
            match mode {
                0 => required.push(pat.clone()),
                1 => optional.push(pat.clone()),
                2 => { rest_pattern = Some(pat.clone()); mode = 3; }
                _ => {}
            }
        }

        let mut bindings = vec![list_form(vec![tmp.clone(), expr], span)];
        let mut current_list = tmp.clone();

        for (i, pat) in required.iter().enumerate() {
            let car_form = list_form(vec![symbol_form("car", span), current_list.clone()], span);
            bindings.push(list_form(vec![pat.clone(), car_form], span));
            let next = if i + 1 < required.len() || !optional.is_empty() || rest_pattern.is_some() {
                let next_tmp = symbol_form(&format!("\0dsb.{}.{}.cdr", depth, i), span);
                let cdr_form = list_form(vec![symbol_form("cdr", span), current_list.clone()], span);
                bindings.push(list_form(vec![next_tmp.clone(), cdr_form], span));
                next_tmp
            } else {
                current_list.clone()
            };
            current_list = next;
        }

        for (i, pat) in optional.iter().enumerate() {
            let car_form = list_form(vec![symbol_form("car", span), current_list.clone()], span);
            let binding = list_form(vec![pat.clone(), car_form], span);
            bindings.push(binding);
            let next = if i + 1 < optional.len() || rest_pattern.is_some() {
                let next_tmp = symbol_form(&format!("\0dsb.{}.{}.opt", depth, i), span);
                let cdr_form = list_form(vec![symbol_form("cdr", span), current_list.clone()], span);
                bindings.push(list_form(vec![next_tmp.clone(), cdr_form], span));
                next_tmp
            } else {
                current_list.clone()
            };
            current_list = next;
        }

        if let Some(rest_pat) = rest_pattern {
            bindings.push(list_form(vec![rest_pat, current_list], span));
        }

        let mut result = vec![symbol_form("let", span), list_form(bindings, span)];
        result.extend(body);
        list_form(result, span)
    }

    fn expand_flet(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (flet ((name (params) body...) ...) body...)
        if items.len() < 3 {
            let expanded: Vec<SurfaceForm> = items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let bindings_form = &items[1];
        let body: Vec<SurfaceForm> = items[2..].to_vec();

        let bindings = self.parse_flet_bindings(bindings_form, span);
        let mut let_bindings = Vec::new();
        let mut prog_body = Vec::new();

        for (name, params, fbody) in bindings {
            let lambda = list_form(
                vec![
                    symbol_form("lambda", span),
                    params,
                ].into_iter().chain(fbody).collect(),
                span,
            );
            let binding = list_form(vec![symbol_form(&name, span), lambda], span);
            let_bindings.push(binding);
        }

        prog_body.extend(body);
        let mut result = vec![symbol_form("let", span), list_form(let_bindings, span)];
        result.extend(prog_body);
        self.expand_form(list_form(result, span))
    }

    fn expand_labels(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (labels ((name (params) body...) ...) body...)
        // Expand to: (let ((name1 nil) ...) (setq name1 (lambda ...)) ... body...)
        if items.len() < 3 {
            let expanded: Vec<SurfaceForm> = items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let bindings_form = &items[1];
        let body: Vec<SurfaceForm> = items[2..].to_vec();

        let bindings = self.parse_flet_bindings(bindings_form, span);
        let mut let_bindings = Vec::new();
        let mut setqs = Vec::new();

        for (name, params, fbody) in bindings {
            // (let ((name nil)) ...)
            let_bindings.push(list_form(vec![symbol_form(&name, span), nil_form(span)], span));
            // (setq name (lambda (params) body...))
            let lambda = list_form(
                vec![symbol_form("lambda", span), params].into_iter().chain(fbody).collect(),
                span,
            );
            setqs.push(list_form(vec![symbol_form("setq", span), symbol_form(&name, span), lambda], span));
        }

        let mut progn_body = setqs;
        progn_body.extend(body);
        let progn = list_form(vec![symbol_form("progn", span)].into_iter().chain(progn_body).collect(), span);
        let result = list_form(vec![symbol_form("let", span), list_form(let_bindings, span), progn], span);
        self.expand_form(result)
    }

    fn parse_flet_bindings(&self, bindings_form: &SurfaceForm, span: Span) -> Vec<(String, SurfaceForm, Vec<SurfaceForm>)> {
        let SurfaceKind::List(bindings) = &bindings_form.kind else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for binding in bindings {
            let SurfaceKind::List(items) = &binding.kind else { continue };
            if items.len() < 3 { continue; }
            let Some(name) = items[0].symbol_name().map(str::to_string) else { continue };
            let params = items[1].clone();
            let body: Vec<SurfaceForm> = items[2..].to_vec();
            result.push((name, params, body));
        }
        result
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct MacroParams {
    required: Vec<String>,
    optional: Vec<String>,
    rest: Option<String>,
    environment: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MacroParamSection {
    Required,
    Optional,
    Rest,
}

#[derive(Clone, Debug, PartialEq)]
struct MacroDef {
    params: MacroParams,
    body: Vec<SurfaceForm>,
    span: Span,
}

#[derive(Clone, Debug, PartialEq)]
struct IfLetBinding {
    name: String,
    value: SurfaceForm,
    span: Span,
}

fn build_if_let_form(
    bindings: Vec<IfLetBinding>,
    then_form: SurfaceForm,
    else_forms: Vec<SurfaceForm>,
    span: Span,
) -> SurfaceForm {
    if bindings.is_empty() {
        return list_form(
            vec![
                symbol_form("let*", span),
                list_form(Vec::new(), span),
                then_form,
            ],
            span,
        );
    }

    let mut previous = symbol_form("t", span);
    let mut binding_forms = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let current = symbol_form(&binding.name, binding.span);
        let value = list_form(
            vec![symbol_form("and", span), previous, binding.value],
            binding.span,
        );
        binding_forms.push(list_form(vec![current.clone(), value], binding.span));
        previous = current;
    }

    let mut if_items = vec![symbol_form("if", span), previous, then_form];
    if_items.extend(else_forms);
    list_form(
        vec![
            symbol_form("let*", span),
            list_form(binding_forms, span),
            list_form(if_items, span),
        ],
        span,
    )
}

fn generated_if_let_name(span: Span, index: usize) -> String {
    format!("\0if-let.{}.{}", span.start, index)
}

fn list_head_symbol(form: &SurfaceForm) -> Option<&str> {
    let SurfaceKind::List(items) = &form.kind else {
        return None;
    };
    items.first().and_then(SurfaceForm::symbol_name)
}

fn nil_form(span: Span) -> SurfaceForm {
    symbol_form("nil", span)
}

fn symbol_form(name: &str, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::symbol(name)), span)
}

fn quote_form(inner: SurfaceForm, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::Quote(Box::new(inner)), span)
}

fn function_quote_form(inner: SurfaceForm, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::FunctionQuote(Box::new(inner)), span)
}

fn list_form(items: Vec<SurfaceForm>, span: Span) -> SurfaceForm {
    SurfaceForm::new(SurfaceKind::List(items), span)
}

fn macro_defalias_form(name: &str, def: &MacroDef, span: Span) -> SurfaceForm {
    let body = if let Some(environment) = &def.params.environment {
        vec![list_form(
            vec![
                symbol_form("let", span),
                list_form(
                    vec![list_form(
                        vec![symbol_form(environment, span), nil_form(span)],
                        span,
                    )],
                    span,
                ),
                lower_macro_body(&def.body, span),
            ],
            span,
        )]
    } else {
        def.body.clone()
    };
    let lambda = list_form(
        std::iter::once(symbol_form("lambda", span))
            .chain(std::iter::once(macro_lambda_params_form(&def.params, span)))
            .chain(body)
            .collect(),
        span,
    );
    list_form(
        vec![
            symbol_form("defalias", span),
            quote_form(symbol_form(name, span), span),
            list_form(
                vec![
                    symbol_form("cons", span),
                    quote_form(symbol_form("macro", span), span),
                    function_quote_form(lambda, span),
                ],
                span,
            ),
        ],
        span,
    )
}

fn macro_lambda_params_form(params: &MacroParams, span: Span) -> SurfaceForm {
    let mut items = params
        .required
        .iter()
        .map(|name| symbol_form(name, span))
        .collect::<Vec<_>>();
    if !params.optional.is_empty() {
        items.push(symbol_form("&optional", span));
        items.extend(params.optional.iter().map(|name| symbol_form(name, span)));
    }
    if let Some(rest) = &params.rest {
        items.push(symbol_form("&rest", span));
        items.push(symbol_form(rest, span));
    }
    list_form(items, span)
}

fn lower_macro_body(body: &[SurfaceForm], span: Span) -> SurfaceForm {
    match body {
        [] => nil_form(span),
        [only] => only.clone(),
        _ => list_form(
            std::iter::once(symbol_form("progn", span))
                .chain(body.iter().cloned())
                .collect(),
            span,
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::compile_source;

    #[test]
    fn expands_push_and_pop_for_simple_symbol_places() {
        let artifact = compile_source(
            "push-pop.el",
            ";;; -*- lexical-binding: t; -*-\n(let ((xs nil)) (push 1 xs) (pop xs))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"setq\""));
        assert!(rendered.contains("\"car-safe\""));
    }

    #[test]
    fn expands_simple_if_let_and_when_let_star() {
        let artifact = compile_source(
            "if-let.el",
            ";;; -*- lexical-binding: t; -*-\n(progn (if-let* ((x 1) (_ x) ((+ x 1))) x 0) (when-let* ((y 2)) y))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"let*\""));
        assert!(rendered.contains("\"and\""));
        assert!(rendered.contains("\"if\""));
        assert!(rendered.contains("\"progn\""));
    }

    #[test]
    fn expands_top_level_defmacro_with_backquote() {
        let artifact = compile_source(
            "defmacro.el",
            ";;; -*- lexical-binding: t; -*-
(defmacro inc (var)
  `(setq ,var (1+ ,var)))
(let ((x 1)) (inc x) x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"inc\""));
        assert!(rendered.contains("\"defalias\""));
        assert!(rendered.contains("\"macro\""));
        assert!(rendered.contains("\"setq\""));
        assert!(!rendered.contains("\"defmacro\""));
    }

    #[test]
    fn expands_defmacro_body_using_list_functions() {
        let artifact = compile_source(
            "defmacro-list.el",
            ";;; -*- lexical-binding: t; -*-
(defmacro inc2 (var)
  (list 'setq var (list '1+ var)))
(let ((x 1)) (inc2 x) x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"setq\""));
        assert!(rendered.contains("\"1+\""));
        assert!(!rendered.contains("\"defmacro\""));
    }

    #[test]
    fn expands_defmacro_with_rest_arguments_and_splicing() {
        let artifact = compile_source(
            "defmacro-rest.el",
            ";;; -*- lexical-binding: t; -*-
(defmacro my-progn (&rest body)
  `(progn ,@body))
(my-progn 1 2 3)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"progn\""));
        assert!(!rendered.contains("\"defmacro\""));
    }

    #[test]
    fn expands_destructuring_bind_simple() {
        let artifact = compile_source(
            "dsb.el",
            ";;; -*- lexical-binding: t; -*-\n(destructuring-bind (a b) (list 1 2) (+ a b))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"car\""));
        assert!(rendered.contains("\"cdr\""));
    }

    #[test]
    fn expands_destructuring_bind_with_rest() {
        let artifact = compile_source(
            "dsb-rest.el",
            ";;; -*- lexical-binding: t; -*-\n(destructuring-bind (a &rest bs) (list 1 2 3) (list a bs))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"car\""));
        assert!(rendered.contains("\"cdr\""));
    }

    #[test]
    fn expands_destructuring_bind_with_optional() {
        let artifact = compile_source(
            "dsb-opt.el",
            ";;; -*- lexical-binding: t; -*-\n(destructuring-bind (a &optional b) (list 1) (list a b))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"car\""));
    }

    #[test]
    fn expands_flet_to_let_with_lambda() {
        let artifact = compile_source(
            "flet.el",
            ";;; -*- lexical-binding: t; -*-\n(flet ((add1 (x) (+ x 1))) (add1 5))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"lambda\""));
        assert!(rendered.contains("\"let\""));
    }

    #[test]
    fn expands_labels_to_let_with_setq() {
        let artifact = compile_source(
            "labels.el",
            ";;; -*- lexical-binding: t; -*-\n(labels ((even? (n) (if (= n 0) t (odd? (- n 1)))) (odd? (n) (if (= n 0) nil (even? (- n 1))))) (even? 4))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"lambda\""));
        assert!(rendered.contains("\"setq\""));
    }
}
