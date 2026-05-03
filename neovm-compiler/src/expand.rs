use std::collections::HashMap;

use crate::diagnostic::Diagnostic;
use crate::expand_eval::{MacroEnv, MacroEval};
use crate::expand_value::{MacroValue, surface_to_value, value_to_surface};
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

        results.pop().unwrap_or_else(|| {
            SurfaceForm::new(
                SurfaceKind::Atom(SurfaceAtom::Nil),
                Span::new(SourceId::new(0), 0, 0),
            )
        })
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
                    let body: Vec<SurfaceForm> = items[2..]
                        .iter()
                        .map(|f| self.expand_form(f.clone()))
                        .collect();
                    // Convert pcase bindings to let* bindings: ((pat expr) ...) -> ((sym expr) ...)
                    let simple_bindings = self.simplify_pcase_bindings(bindings_form);
                    let mut result = vec![symbol_form("let*", span), simple_bindings];
                    result.extend(body);
                    list_form(result, span)
                } else {
                    let expanded: Vec<SurfaceForm> =
                        items.into_iter().map(|f| self.expand_form(f)).collect();
                    SurfaceForm::new(SurfaceKind::List(expanded), span)
                }
            }
            // cl-with-gensyms -> let (simplified: uses symbol names as-is)
            "cl-with-gensyms" => {
                if items.len() >= 3 {
                    let bindings = items[1].clone();
                    let body: Vec<SurfaceForm> = items[2..]
                        .iter()
                        .map(|f| self.expand_form(f.clone()))
                        .collect();
                    list_form(
                        vec![symbol_form("let", span), bindings.clone()]
                            .into_iter()
                            .chain(body)
                            .collect(),
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
            "cl-defun" => self.expand_cl_defun(span, items),
            "cl-macrolet" => self.expand_cl_macrolet(span, items),
            "cl-symbol-macrolet" => self.expand_cl_symbol_macrolet(span, items),
            "cl-loop" => self.expand_cl_loop(span, items),
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
            let expansion_items =
                match std::mem::replace(&mut form.kind, SurfaceKind::Atom(SurfaceAtom::Nil)) {
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
                arg_values
                    .get(optional_start + index)
                    .cloned()
                    .unwrap_or(MacroValue::Nil),
            );
        }
        if let Some(rest) = &def.params.rest {
            let rest_start = arg_values
                .len()
                .min(optional_start + def.params.optional.len());
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
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let Some(place) = items[2].symbol_name().map(str::to_string) else {
            // Non-symbol place (e.g., list access) — expand sub-forms, pass through
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
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
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let Some(place) = items[1].symbol_name().map(str::to_string) else {
            // Non-symbol place — expand sub-forms, pass through
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
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
                        Some(list_form(
                            vec![symbol_form(name, pat.span), expr.clone()],
                            span,
                        ))
                    } else {
                        Some(list_form(
                            vec![symbol_form("_", pat.span), expr.clone()],
                            span,
                        ))
                    }
                } else if binding_items.len() == 1 {
                    Some(list_form(
                        vec![
                            symbol_form("_", binding_items[0].span),
                            binding_items[0].clone(),
                        ],
                        span,
                    ))
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
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
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
                        return list_form(
                            vec![symbol_form("progn", span)]
                                .into_iter()
                                .chain(forms)
                                .collect(),
                            span,
                        );
                    }
                    let binding =
                        list_form(vec![symbol_form(name, pattern.span), expr], pattern.span);
                    let mut result = vec![symbol_form("let", span), list_form(vec![binding], span)];
                    result.extend(body);
                    list_form(result, span)
                } else {
                    let mut forms = vec![expr];
                    forms.extend(body);
                    list_form(
                        vec![symbol_form("progn", span)]
                            .into_iter()
                            .chain(forms)
                            .collect(),
                        span,
                    )
                }
            }
            SurfaceKind::Quote(_) | SurfaceKind::FunctionQuote(_) => {
                let mut forms = vec![expr];
                forms.extend(body);
                list_form(
                    vec![symbol_form("progn", span)]
                        .into_iter()
                        .chain(forms)
                        .collect(),
                    span,
                )
            }
            SurfaceKind::List(patterns) => {
                self.destructure_list_pattern(patterns, expr, body, span, depth)
            }
            _ => {
                let mut forms = vec![expr];
                forms.extend(body);
                list_form(
                    vec![symbol_form("progn", span)]
                        .into_iter()
                        .chain(forms)
                        .collect(),
                    span,
                )
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
                if name == "&optional" {
                    mode = 1;
                    continue;
                }
                if name == "&rest" {
                    mode = 2;
                    continue;
                }
            }
            match mode {
                0 => required.push(pat.clone()),
                1 => optional.push(pat.clone()),
                2 => {
                    rest_pattern = Some(pat.clone());
                    mode = 3;
                }
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
                let cdr_form =
                    list_form(vec![symbol_form("cdr", span), current_list.clone()], span);
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
                let cdr_form =
                    list_form(vec![symbol_form("cdr", span), current_list.clone()], span);
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
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let bindings_form = &items[1];
        let body: Vec<SurfaceForm> = items[2..].to_vec();

        let bindings = self.parse_flet_bindings(bindings_form, span);
        let mut let_bindings = Vec::new();
        let mut prog_body = Vec::new();

        for (name, params, fbody) in bindings {
            let lambda = list_form(
                vec![symbol_form("lambda", span), params]
                    .into_iter()
                    .chain(fbody)
                    .collect(),
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
            let expanded: Vec<SurfaceForm> =
                items.into_iter().map(|f| self.expand_form(f)).collect();
            return SurfaceForm::new(SurfaceKind::List(expanded), span);
        }
        let bindings_form = &items[1];
        let body: Vec<SurfaceForm> = items[2..].to_vec();

        let bindings = self.parse_flet_bindings(bindings_form, span);
        let label_names: Vec<String> = bindings.iter().map(|(n, _, _)| n.clone()).collect();
        let mut let_bindings = Vec::new();
        let mut setqs = Vec::new();

        for (name, params, fbody) in bindings {
            // (let ((name nil)) ...)
            let_bindings.push(list_form(
                vec![symbol_form(&name, span), nil_form(span)],
                span,
            ));
            // (setq name (lambda (params) body...))
            let lambda = list_form(
                vec![symbol_form("lambda", span), params]
                    .into_iter()
                    .chain(fbody)
                    .collect(),
                span,
            );
            setqs.push(list_form(
                vec![symbol_form("setq", span), symbol_form(&name, span), lambda],
                span,
            ));
        }

        // Rewrite function calls to label names in the body AND lambda bodies to use funcall
        let rewritten_body: Vec<SurfaceForm> = body
            .into_iter()
            .map(|f| Self::rewrite_labels_calls(&f, &label_names))
            .collect();
        let rewritten_setqs: Vec<SurfaceForm> = setqs
            .into_iter()
            .map(|f| Self::rewrite_labels_calls(&f, &label_names))
            .collect();

        let mut progn_body = rewritten_setqs;
        progn_body.extend(rewritten_body);
        let progn = list_form(
            vec![symbol_form("progn", span)]
                .into_iter()
                .chain(progn_body)
                .collect(),
            span,
        );
        let result = list_form(
            vec![
                symbol_form("let", span),
                list_form(let_bindings, span),
                progn,
            ],
            span,
        );
        self.expand_form(result)
    }

    /// Rewrite (name args...) to (funcall name args...) when name is a label.
    fn rewrite_labels_calls(form: &SurfaceForm, label_names: &[String]) -> SurfaceForm {
        match &form.kind {
            SurfaceKind::List(items) if !items.is_empty() => {
                if let SurfaceKind::Atom(SurfaceAtom::Symbol(name)) = &items[0].kind {
                    if label_names.contains(name) {
                        // (name args...) -> (funcall name args...)
                        let mut new_items = vec![
                            symbol_form("funcall", form.span),
                            symbol_form(name, form.span),
                        ];
                        new_items.extend(
                            items[1..]
                                .iter()
                                .cloned()
                                .map(|arg| Self::rewrite_labels_calls(&arg, label_names)),
                        );
                        return list_form(new_items, form.span);
                    }
                }
                let rewritten: Vec<SurfaceForm> = items
                    .iter()
                    .map(|item| Self::rewrite_labels_calls(item, label_names))
                    .collect();
                SurfaceForm::new(SurfaceKind::List(rewritten), form.span)
            }
            SurfaceKind::List(items) => form.clone(),
            _ => form.clone(),
        }
    }

    fn parse_flet_bindings(
        &self,
        bindings_form: &SurfaceForm,
        _span: Span,
    ) -> Vec<(String, SurfaceForm, Vec<SurfaceForm>)> {
        let SurfaceKind::List(bindings) = &bindings_form.kind else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for binding in bindings {
            let SurfaceKind::List(items) = &binding.kind else {
                continue;
            };
            if items.len() < 3 {
                continue;
            }
            let Some(name) = items[0].symbol_name().map(str::to_string) else {
                continue;
            };
            let params = items[1].clone();
            let body: Vec<SurfaceForm> = items[2..].to_vec();
            result.push((name, params, body));
        }
        result
    }

    fn expand_cl_defun(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (cl-defun name (args &optional opt &rest rest) body...)
        // Expand to (defun name (required-args) (destructuring-bind (...) (list required-args...) body...))
        // Simplified: if params are all required, just produce (defun name (params) body...)
        // If &optional/&rest present, use a wrapper approach.
        if items.len() < 4 {
            return nil_form(span);
        }
        let Some(name) = items[1].symbol_name().map(str::to_string) else {
            return nil_form(span);
        };
        let params_form = &items[2];
        let body: Vec<SurfaceForm> = items[3..].to_vec();

        let (required, optional, rest) = self.parse_cl_lambda_list(params_form);

        if optional.is_empty() && rest.is_none() {
            // Simple case: all required params, expand to plain defun
            let mut result = vec![
                symbol_form("defun", span),
                symbol_form(&name, span),
                params_form.clone(),
            ];
            result.extend(body.into_iter().map(|f| self.expand_form(f)));
            return list_form(result, span);
        }

        // Complex case: use a single &rest arg and destructuring-bind
        // (cl-defun foo (a &optional b &rest c) body...)
        // -> (defun foo (&rest --cl-rest--) (destructuring-bind (a &optional b &rest c) --cl-rest-- body...))
        let rest_arg = symbol_form("--cl-rest--", span);
        let rest_params = list_form(vec![symbol_form("&rest", span), rest_arg.clone()], span);

        let mut dbind_params: Vec<SurfaceForm> =
            required.iter().map(|s| symbol_form(s, span)).collect();
        if !optional.is_empty() {
            dbind_params.push(symbol_form("&optional", span));
            for s in &optional {
                dbind_params.push(symbol_form(s, span));
            }
        }
        if let Some(ref r) = rest {
            dbind_params.push(symbol_form("&rest", span));
            dbind_params.push(symbol_form(r, span));
        }
        let dbind_pattern = list_form(dbind_params, span);

        let mut dbind_body = vec![
            symbol_form("destructuring-bind", span),
            dbind_pattern,
            rest_arg,
        ];
        dbind_body.extend(body.into_iter().map(|f| self.expand_form(f)));

        let defun_body = vec![
            symbol_form("defun", span),
            symbol_form(&name, span),
            rest_params,
            list_form(dbind_body, span),
        ];
        list_form(defun_body, span)
    }

    fn parse_cl_lambda_list(
        &self,
        params_form: &SurfaceForm,
    ) -> (Vec<String>, Vec<String>, Option<String>) {
        let items = match &params_form.kind {
            SurfaceKind::List(items) => items,
            _ => return (Vec::new(), Vec::new(), None),
        };
        let mut required = Vec::new();
        let mut optional = Vec::new();
        let mut rest = None;
        let mut section = 0; // 0=required, 1=optional, 2=rest

        for item in items {
            let Some(name) = item.symbol_name() else {
                continue;
            };
            match name {
                "&optional" => section = 1,
                "&rest" => section = 2,
                "&key" | "&allow-other-keys" | "&aux" => break, // not supported yet
                _ => match section {
                    0 => required.push(name.to_string()),
                    1 => optional.push(name.to_string()),
                    2 if rest.is_none() => {
                        rest = Some(name.to_string());
                        section = 3; // done
                    }
                    _ => {}
                },
            }
        }
        (required, optional, rest)
    }

    fn expand_cl_macrolet(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (cl-macrolet ((name (args) body...) ...) body...)
        // Register each binding as a macro, expand the body, then unregister.
        if items.len() < 3 {
            return nil_form(span);
        }
        let bindings_form = &items[1];
        let body = &items[2..];

        let SurfaceKind::List(bindings) = &bindings_form.kind else {
            return self.expand_progn(span, body.to_vec());
        };

        let mut defined_names: Vec<String> = Vec::new();
        for binding in bindings {
            let SurfaceKind::List(bitems) = &binding.kind else {
                continue;
            };
            if bitems.len() < 3 {
                continue;
            }
            let Some(name) = bitems[0].symbol_name().map(str::to_string) else {
                continue;
            };
            let params_form = &bitems[1];
            let macro_body: Vec<SurfaceForm> = bitems[2..].to_vec();
            let macro_params = self.parse_macro_params(params_form).unwrap_or_default();
            let def = MacroDef {
                params: macro_params,
                body: macro_body,
                span: binding.span,
            };
            self.macros.insert(name.clone(), def);
            defined_names.push(name);
        }

        let result = self.expand_progn(span, body.to_vec());

        for name in &defined_names {
            self.macros.remove(name);
        }

        result
    }

    fn expand_cl_symbol_macrolet(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        // (cl-symbol-macrolet ((name expansion) ...) body...)
        // Register each binding as a value in the symbol-macro table.
        // Simplified: expand body, replacing symbol occurrences.
        if items.len() < 3 {
            return nil_form(span);
        }
        // For now, just expand the body forms — symbol macros would need
        // a full substitution pass which is complex. Most real uses of
        // cl-symbol-macrolet are for macros that we handle differently.
        let body: Vec<SurfaceForm> = items[2..].to_vec();
        self.expand_progn(span, body)
    }

    fn expand_progn(&mut self, span: Span, forms: Vec<SurfaceForm>) -> SurfaceForm {
        let expanded: Vec<SurfaceForm> = forms.into_iter().map(|f| self.expand_form(f)).collect();
        match expanded.len() {
            0 => nil_form(span),
            1 => expanded.into_iter().next().unwrap(),
            _ => list_form(
                std::iter::once(symbol_form("progn", span))
                    .chain(expanded)
                    .collect(),
                span,
            ),
        }
    }

    // ── cl-loop expansion ──────────────────────────────────────────────

    fn expand_cl_loop(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        if items.len() < 2 {
            return nil_form(span);
        }
        let clauses = match self.parse_loop_clauses(span, &items[1..]) {
            Some(c) => c,
            None => return nil_form(span),
        };
        if clauses.is_empty() {
            return nil_form(span);
        }
        let result = self.build_loop_expansion(span, &clauses);
        self.expand_form(result)
    }

    fn build_loop_expansion(&self, span: Span, clauses: &[LoopClause]) -> SurfaceForm {
        let mut acc_counter = 0usize;
        let mut list_counter = 0usize;
        let mut has_return = false;

        // Classify clauses
        let mut for_clauses: Vec<&LoopClause> = Vec::new();
        let mut body_clauses: Vec<&LoopClause> = Vec::new();
        let mut while_conds: Vec<SurfaceForm> = Vec::new();
        let mut initially_body: Vec<SurfaceForm> = Vec::new();
        let mut finally_body: Vec<SurfaceForm> = Vec::new();
        let mut with_bindings: Vec<(String, SurfaceForm)> = Vec::new();
        let mut accums: Vec<(AccumKind, String, SurfaceForm)> = Vec::new(); // (kind, var, init)
        let mut has_repeat = false;
        let mut repeat_count: Option<SurfaceForm> = None;
        let mut has_always_never = false;
        let mut has_thereis = false;

        for clause in clauses {
            match clause {
                LoopClause::ForFrom { .. }
                | LoopClause::ForIn { .. }
                | LoopClause::ForOn { .. }
                | LoopClause::ForEquals { .. } => for_clauses.push(clause),
                LoopClause::While { cond } => while_conds.push(cond.clone()),
                LoopClause::Until { cond } => while_conds.push(list_form(
                    vec![symbol_form("null", span), cond.clone()],
                    span,
                )),
                LoopClause::With { var, expr } => with_bindings.push((var.clone(), expr.clone())),
                LoopClause::Repeat { count } => {
                    // repeat N → counter var, checked in while test, decremented in body
                    let counter_var = "--cl-repeat--";
                    while_conds.push(list_form(
                        vec![
                            symbol_form(">", span),
                            symbol_form(counter_var, span),
                            SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(0)), span),
                        ],
                        span,
                    ));
                    has_repeat = true;
                    repeat_count = Some(count.clone());
                }
                LoopClause::Initially { body } => initially_body.extend(body.iter().cloned()),
                LoopClause::Finally { body } => {
                    // If finally body contains (throw '--cl-loop-tag-- ...), need catch wrapper
                    if body.first().map_or(false, |f| {
                        matches!(&f.kind, SurfaceKind::List(items) if items.first().and_then(|i| i.symbol_name()) == Some("throw"))
                    }) {
                        has_return = true;
                    }
                    finally_body.extend(body.iter().cloned());
                }
                LoopClause::Return { .. } => {
                    has_return = true;
                    body_clauses.push(clause);
                }
                LoopClause::Always { .. } | LoopClause::Never { .. } => {
                    has_always_never = true;
                    body_clauses.push(clause);
                }
                LoopClause::Thereis { .. } => {
                    has_thereis = true;
                    body_clauses.push(clause);
                }
                LoopClause::If {
                    then_clauses,
                    else_clauses,
                    ..
                } => {
                    if clauses_contain_return(then_clauses)
                        || else_clauses
                            .as_ref()
                            .map_or(false, |ec| clauses_contain_return(ec))
                    {
                        has_return = true;
                    }
                    if !has_always_never {
                        has_always_never = clauses.iter().any(|c| {
                            matches!(c, LoopClause::Always { .. } | LoopClause::Never { .. })
                        });
                    }
                    if !has_thereis {
                        has_thereis = clauses
                            .iter()
                            .any(|c| matches!(c, LoopClause::Thereis { .. }));
                    }
                    body_clauses.push(clause);
                }
                _ => body_clauses.push(clause),
            }
        }

        // Allocate accumulators for collection/aggregation clauses (including nested in when/if)
        let mut accum_map: Vec<(AccumKind, Option<String>, String)> = Vec::new(); // (kind, into_name, var_name)
        let all_accum_clauses = Self::collect_accum_kinds(body_clauses.iter().copied());
        for (kind, into_name) in all_accum_clauses {
            // Reuse existing accumulator of same kind + into name
            if accum_map
                .iter()
                .any(|(k, n, _)| *k == kind && n.as_ref() == into_name.as_ref())
            {
                continue;
            }
            let var_name = into_name
                .clone()
                .unwrap_or_else(|| format!("--cl-acc-{}--", acc_counter));
            let init = match kind {
                AccumKind::Collect | AccumKind::Append | AccumKind::Nconc => nil_form(span),
                AccumKind::Sum | AccumKind::Count => {
                    SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(0)), span)
                }
                AccumKind::Minimize | AccumKind::Maximize => nil_form(span),
            };
            accums.push((kind, var_name.clone(), init));
            accum_map.push((kind, into_name, var_name));
            acc_counter += 1;
        }

        // Build let-bindings
        let mut let_bindings: Vec<SurfaceForm> = Vec::new();

        // Accumulator bindings
        for (_, name, init) in &accums {
            let_bindings.push(list_form(vec![symbol_form(name, span), init.clone()], span));
        }

        // For-from bindings and list temp bindings
        let mut for_from_info: Vec<(
            String,
            SurfaceForm,
            Option<SurfaceForm>,
            Option<SurfaceForm>,
        )> = Vec::new(); // var, start, end, step
        let mut for_in_info: Vec<(String, String, SurfaceForm)> = Vec::new(); // var, list-temp, list-expr
        let mut for_on_info: Vec<(String, String, SurfaceForm)> = Vec::new(); // var, list-temp, list-expr
        let mut for_eq_info: Vec<(String, SurfaceForm)> = Vec::new(); // var, expr (no then)
        let mut for_eq_step: Vec<(String, SurfaceForm)> = Vec::new(); // var, step (has then)

        for clause in &for_clauses {
            match clause {
                LoopClause::ForFrom {
                    var,
                    start,
                    end,
                    step,
                } => {
                    let_bindings.push(list_form(vec![symbol_form(var, span), start.clone()], span));
                    for_from_info.push((var.clone(), start.clone(), end.clone(), step.clone()));
                }
                LoopClause::ForIn { var, list_expr } => {
                    let list_temp = format!("--cl-list-{}--", list_counter);
                    list_counter += 1;
                    let_bindings.push(list_form(
                        vec![symbol_form(&list_temp, span), list_expr.clone()],
                        span,
                    ));
                    let_bindings.push(list_form(
                        vec![symbol_form(var, span), nil_form(span)],
                        span,
                    ));
                    for_in_info.push((var.clone(), list_temp, list_expr.clone()));
                }
                LoopClause::ForOn { var, list_expr } => {
                    let list_temp = format!("--cl-list-{}--", list_counter);
                    list_counter += 1;
                    let_bindings.push(list_form(
                        vec![symbol_form(&list_temp, span), list_expr.clone()],
                        span,
                    ));
                    for_on_info.push((var.clone(), list_temp, list_expr.clone()));
                }
                LoopClause::ForEquals {
                    var,
                    expr,
                    then_expr,
                } => {
                    if let Some(step) = then_expr {
                        // for x = init then step: bind init, step goes after body
                        let_bindings
                            .push(list_form(vec![symbol_form(var, span), expr.clone()], span));
                        for_eq_step.push((var.clone(), step.clone()));
                    } else {
                        // for x = expr: bind nil, setq expr at body start
                        let_bindings.push(list_form(
                            vec![symbol_form(var, span), nil_form(span)],
                            span,
                        ));
                        for_eq_info.push((var.clone(), expr.clone()));
                    }
                }
                _ => {}
            }
        }

        // With bindings
        for (var, expr) in &with_bindings {
            let_bindings.push(list_form(vec![symbol_form(var, span), expr.clone()], span));
        }

        // Repeat binding
        if has_repeat {
            if let Some(count) = &repeat_count {
                let_bindings.push(list_form(
                    vec![symbol_form("--cl-repeat--", span), count.clone()],
                    span,
                ));
            }
        }

        // Always/never flag variable
        if has_always_never {
            let_bindings.push(list_form(
                vec![symbol_form("--cl-always--", span), symbol_form("t", span)],
                span,
            ));
        }

        // Thereis result variable
        if has_thereis {
            let_bindings.push(list_form(
                vec![symbol_form("--cl-thereis--", span), nil_form(span)],
                span,
            ));
        }

        // Build while test
        let mut while_tests: Vec<SurfaceForm> = Vec::new();

        // For-from: (<= var end) when end is specified
        for (var, start, end, _) in &for_from_info {
            let _ = start;
            if let Some(end_val) = end {
                while_tests.push(list_form(
                    vec![
                        symbol_form("<=", span),
                        symbol_form(var, span),
                        end_val.clone(),
                    ],
                    span,
                ));
            }
            // No end means open-ended loop — only while/until conditions control termination
        }

        // For-in/for-on: list-temp truthiness
        for (_, list_temp, _) in &for_in_info {
            while_tests.push(symbol_form(list_temp, span));
        }
        for (_, list_temp, _) in &for_on_info {
            while_tests.push(symbol_form(list_temp, span));
        }

        // Explicit while/until conditions
        while_tests.extend(while_conds);

        // always/never short-circuit: stop the loop when the flag becomes nil
        if has_always_never {
            while_tests.push(symbol_form("--cl-always--", span));
        }

        let while_test = if while_tests.is_empty() {
            symbol_form("t", span)
        } else if while_tests.len() == 1 {
            while_tests.into_iter().next().unwrap()
        } else {
            list_form(
                std::iter::once(symbol_form("and", span))
                    .chain(while_tests.into_iter())
                    .collect(),
                span,
            )
        };

        // Build while body
        let mut while_body: Vec<SurfaceForm> = Vec::new();

        // For-equals: setq at body start
        for (var, expr) in &for_eq_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    expr.clone(),
                ],
                span,
            ));
        }

        // For-in: setq var (car --list--)
        for (var, list_temp, _) in &for_in_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    list_form(
                        vec![symbol_form("car", span), symbol_form(list_temp, span)],
                        span,
                    ),
                ],
                span,
            ));
        }

        // For-on: setq var --list--
        for (var, list_temp, _) in &for_on_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    symbol_form(list_temp, span),
                ],
                span,
            ));
        }

        // Body clauses (collect, sum, do, return, if, etc.)
        for clause in &body_clauses {
            match clause {
                LoopClause::Collect { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Collect, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("cons", span),
                                    expr.clone(),
                                    symbol_form(&acc_var, span),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Append { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Append, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("append", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Nconc { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Nconc, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("nconc", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Sum { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Sum, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("+", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Count { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Count, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("+", span),
                                    symbol_form(&acc_var, span),
                                    list_form(
                                        vec![
                                            symbol_form("if", span),
                                            expr.clone(),
                                            SurfaceForm::new(
                                                SurfaceKind::Atom(SurfaceAtom::Int(1)),
                                                span,
                                            ),
                                            SurfaceForm::new(
                                                SurfaceKind::Atom(SurfaceAtom::Int(0)),
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
                    ));
                }
                LoopClause::Do { body } => {
                    while_body.extend(body.iter().cloned());
                }
                LoopClause::Return { expr } => {
                    while_body.push(list_form(
                        vec![
                            symbol_form("throw", span),
                            list_form(
                                vec![
                                    symbol_form("quote", span),
                                    symbol_form("--cl-loop-tag--", span),
                                ],
                                span,
                            ),
                            expr.clone(),
                        ],
                        span,
                    ));
                }
                LoopClause::If {
                    cond,
                    then_clauses,
                    else_clauses,
                } => {
                    let then_body = self.build_if_body(span, then_clauses, &accum_map);
                    let if_form = if let Some(else_cls) = else_clauses {
                        let else_body = self.build_if_body(span, else_cls, &accum_map);
                        list_form(
                            vec![
                                symbol_form("if", span),
                                cond.clone(),
                                self.wrap_progn(then_body, span),
                                self.wrap_progn(else_body, span),
                            ],
                            span,
                        )
                    } else {
                        list_form(
                            vec![
                                symbol_form("if", span),
                                cond.clone(),
                                self.wrap_progn(then_body, span),
                            ],
                            span,
                        )
                    };
                    while_body.push(if_form);
                }
                LoopClause::Always { expr } => {
                    // (if (null expr) (setq --cl-always-- nil))
                    while_body.push(list_form(
                        vec![
                            symbol_form("if", span),
                            list_form(vec![symbol_form("null", span), expr.clone()], span),
                            list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form("--cl-always--", span),
                                    nil_form(span),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Never { expr } => {
                    // (if expr (setq --cl-always-- nil))
                    while_body.push(list_form(
                        vec![
                            symbol_form("if", span),
                            expr.clone(),
                            list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form("--cl-always--", span),
                                    nil_form(span),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Thereis { expr } => {
                    // (if (and (null --cl-thereis--) expr)
                    //     (setq --cl-thereis-- expr))
                    while_body.push(list_form(
                        vec![
                            symbol_form("if", span),
                            list_form(
                                vec![
                                    symbol_form("and", span),
                                    list_form(
                                        vec![
                                            symbol_form("null", span),
                                            symbol_form("--cl-thereis--", span),
                                        ],
                                        span,
                                    ),
                                    expr.clone(),
                                ],
                                span,
                            ),
                            list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form("--cl-thereis--", span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Minimize { expr, into } => {
                    // (if (or (null acc) (< expr acc)) (setq acc expr))
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Minimize, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("if", span),
                            list_form(
                                vec![
                                    symbol_form("or", span),
                                    list_form(
                                        vec![
                                            symbol_form("null", span),
                                            symbol_form(&acc_var, span),
                                        ],
                                        span,
                                    ),
                                    list_form(
                                        vec![
                                            symbol_form("<", span),
                                            expr.clone(),
                                            symbol_form(&acc_var, span),
                                        ],
                                        span,
                                    ),
                                ],
                                span,
                            ),
                            list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Maximize { expr, into } => {
                    let acc_var = self.find_accum_var(&accum_map, AccumKind::Maximize, into);
                    while_body.push(list_form(
                        vec![
                            symbol_form("if", span),
                            list_form(
                                vec![
                                    symbol_form("or", span),
                                    list_form(
                                        vec![
                                            symbol_form("null", span),
                                            symbol_form(&acc_var, span),
                                        ],
                                        span,
                                    ),
                                    list_form(
                                        vec![
                                            symbol_form(">", span),
                                            expr.clone(),
                                            symbol_form(&acc_var, span),
                                        ],
                                        span,
                                    ),
                                ],
                                span,
                            ),
                            list_form(
                                vec![
                                    symbol_form("setq", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                _ => {}
            }
        }

        // For-in advance: setq --list-- (cdr --list--)
        for (_, list_temp, _) in &for_in_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(list_temp, span),
                    list_form(
                        vec![symbol_form("cdr", span), symbol_form(list_temp, span)],
                        span,
                    ),
                ],
                span,
            ));
        }

        // For-on advance: setq --list-- (cdr --list--)
        for (_, list_temp, _) in &for_on_info {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(list_temp, span),
                    list_form(
                        vec![symbol_form("cdr", span), symbol_form(list_temp, span)],
                        span,
                    ),
                ],
                span,
            ));
        }

        // For-from advance: setq var (+ var step)
        for (var, _, _, step) in &for_from_info {
            let step_val = step
                .clone()
                .unwrap_or_else(|| SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(1)), span));
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    list_form(
                        vec![symbol_form("+", span), symbol_form(var, span), step_val],
                        span,
                    ),
                ],
                span,
            ));
        }

        // For-equals step (for x = init then step): update at end of iteration
        for (var, step) in &for_eq_step {
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(var, span),
                    step.clone(),
                ],
                span,
            ));
        }

        // Repeat counter decrement
        if has_repeat {
            let counter_var = "--cl-repeat--";
            while_body.push(list_form(
                vec![
                    symbol_form("setq", span),
                    symbol_form(counter_var, span),
                    list_form(
                        vec![
                            symbol_form("-", span),
                            symbol_form(counter_var, span),
                            SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(1)), span),
                        ],
                        span,
                    ),
                ],
                span,
            ));
        }

        // Build result expression (after while loop)
        let mut after_while: Vec<SurfaceForm> = Vec::new();

        // nreverse for collect accumulators
        for (kind, name, _) in &accums {
            if *kind == AccumKind::Collect {
                after_while.push(list_form(
                    vec![
                        symbol_form("setq", span),
                        symbol_form(name, span),
                        list_form(
                            vec![symbol_form("nreverse", span), symbol_form(name, span)],
                            span,
                        ),
                    ],
                    span,
                ));
            }
        }

        // Finally body
        after_while.extend(finally_body.iter().cloned());

        // Result expression
        if !accums.is_empty() {
            let (_, name, _) = &accums[0];
            after_while.push(symbol_form(name, span));
        } else if has_always_never {
            after_while.push(symbol_form("--cl-always--", span));
        } else if has_thereis {
            after_while.push(symbol_form("--cl-thereis--", span));
        } else if finally_body.is_empty() {
            after_while.push(nil_form(span));
        }

        // Build the let body
        let mut let_body: Vec<SurfaceForm> = Vec::new();
        let_body.extend(initially_body.iter().cloned());
        let_body.push(list_form(
            std::iter::once(symbol_form("while", span))
                .chain(std::iter::once(while_test))
                .chain(while_body.into_iter())
                .collect(),
            span,
        ));
        let_body.extend(after_while);

        let let_form = list_form(
            std::iter::once(symbol_form("let", span))
                .chain(std::iter::once(list_form(let_bindings, span)))
                .chain(let_body.into_iter())
                .collect(),
            span,
        );

        // Wrap in catch/throw if return clause present
        if has_return {
            list_form(
                vec![
                    symbol_form("catch", span),
                    list_form(
                        vec![
                            symbol_form("quote", span),
                            symbol_form("--cl-loop-tag--", span),
                        ],
                        span,
                    ),
                    let_form,
                ],
                span,
            )
        } else {
            let_form
        }
    }

    fn find_accum_var(
        &self,
        accum_map: &[(AccumKind, Option<String>, String)],
        kind: AccumKind,
        into: &Option<String>,
    ) -> String {
        accum_map
            .iter()
            .find(|(k, n, _)| *k == kind && n.as_ref() == into.as_ref())
            .map(|(_, _, name)| name.clone())
            .unwrap_or_else(|| "--cl-acc-unknown--".into())
    }

    /// Collect all AccumKinds with their into-names from clauses, recursing into when/if branches.
    fn collect_accum_kinds<'a>(
        clauses: impl IntoIterator<Item = &'a LoopClause>,
    ) -> Vec<(AccumKind, Option<String>)> {
        let mut kinds = Vec::new();
        for clause in clauses {
            match clause {
                LoopClause::Collect { into, .. } => kinds.push((AccumKind::Collect, into.clone())),
                LoopClause::Append { into, .. } => kinds.push((AccumKind::Append, into.clone())),
                LoopClause::Nconc { into, .. } => kinds.push((AccumKind::Nconc, into.clone())),
                LoopClause::Sum { into, .. } => kinds.push((AccumKind::Sum, into.clone())),
                LoopClause::Count { into, .. } => kinds.push((AccumKind::Count, into.clone())),
                LoopClause::Minimize { into, .. } => {
                    kinds.push((AccumKind::Minimize, into.clone()))
                }
                LoopClause::Maximize { into, .. } => {
                    kinds.push((AccumKind::Maximize, into.clone()))
                }
                LoopClause::If {
                    then_clauses,
                    else_clauses,
                    ..
                } => {
                    kinds.extend(Self::collect_accum_kinds(then_clauses.iter()));
                    if let Some(else_cls) = else_clauses {
                        kinds.extend(Self::collect_accum_kinds(else_cls.iter()));
                    }
                }
                _ => {}
            }
        }
        kinds
    }

    fn build_if_body(
        &self,
        span: Span,
        clauses: &[LoopClause],
        accum_map: &[(AccumKind, Option<String>, String)],
    ) -> Vec<SurfaceForm> {
        let mut body: Vec<SurfaceForm> = Vec::new();
        for clause in clauses {
            match clause {
                LoopClause::Collect { expr, into } => {
                    let acc_var = self.find_accum_var(accum_map, AccumKind::Collect, into);
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("cons", span),
                                    expr.clone(),
                                    symbol_form(&acc_var, span),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Append { expr, into } => {
                    let acc_var = self.find_accum_var(accum_map, AccumKind::Append, into);
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("append", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Nconc { expr, into } => {
                    let acc_var = self.find_accum_var(accum_map, AccumKind::Nconc, into);
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("nconc", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Sum { expr, into } => {
                    let acc_var = self.find_accum_var(accum_map, AccumKind::Sum, into);
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("+", span),
                                    symbol_form(&acc_var, span),
                                    expr.clone(),
                                ],
                                span,
                            ),
                        ],
                        span,
                    ));
                }
                LoopClause::Count { expr, into } => {
                    let acc_var = self.find_accum_var(accum_map, AccumKind::Count, into);
                    body.push(list_form(
                        vec![
                            symbol_form("setq", span),
                            symbol_form(&acc_var, span),
                            list_form(
                                vec![
                                    symbol_form("+", span),
                                    symbol_form(&acc_var, span),
                                    list_form(
                                        vec![
                                            symbol_form("if", span),
                                            expr.clone(),
                                            SurfaceForm::new(
                                                SurfaceKind::Atom(SurfaceAtom::Int(1)),
                                                span,
                                            ),
                                            SurfaceForm::new(
                                                SurfaceKind::Atom(SurfaceAtom::Int(0)),
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
                    ));
                }
                LoopClause::Do { body: b } => body.extend(b.iter().cloned()),
                LoopClause::Return { expr } => {
                    body.push(list_form(
                        vec![
                            symbol_form("throw", span),
                            list_form(
                                vec![
                                    symbol_form("quote", span),
                                    symbol_form("--cl-loop-tag--", span),
                                ],
                                span,
                            ),
                            expr.clone(),
                        ],
                        span,
                    ));
                }
                _ => {}
            }
        }
        body
    }

    fn wrap_progn(&self, forms: Vec<SurfaceForm>, span: Span) -> SurfaceForm {
        match forms.len() {
            0 => nil_form(span),
            1 => forms.into_iter().next().unwrap(),
            _ => list_form(
                std::iter::once(symbol_form("progn", span))
                    .chain(forms)
                    .collect(),
                span,
            ),
        }
    }

    // ── cl-loop clause parser ──────────────────────────────────────────

    fn parse_into_keyword(items: &[SurfaceForm], pos: &mut usize) -> Option<String> {
        if *pos < items.len() && items[*pos].symbol_name() == Some("into") {
            *pos += 1;
            if *pos < items.len() {
                let name = items[*pos].symbol_name().map(str::to_string);
                *pos += 1;
                return name;
            }
        }
        None
    }

    fn parse_loop_clauses(&self, span: Span, items: &[SurfaceForm]) -> Option<Vec<LoopClause>> {
        let mut clauses = Vec::new();
        let mut pos = 0;
        while pos < items.len() {
            // Non-keyword forms at the top level are treated as implicit do body
            let keyword = match items[pos].symbol_name() {
                Some(kw) => kw,
                None => {
                    // Implicit do: treat this form as a body expression
                    let body_form = items[pos].clone();
                    pos += 1;
                    clauses.push(LoopClause::Do {
                        body: vec![body_form],
                    });
                    continue;
                }
            };
            match keyword {
                "for" => {
                    pos += 1;
                    let clause = self.parse_for_clause(span, items, &mut pos)?;
                    clauses.push(clause);
                }
                "collect" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(&items, &mut pos);
                    clauses.push(LoopClause::Collect { expr, into });
                }
                "append" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(&items, &mut pos);
                    clauses.push(LoopClause::Append { expr, into });
                }
                "nconc" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(&items, &mut pos);
                    clauses.push(LoopClause::Nconc { expr, into });
                }
                "sum" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(&items, &mut pos);
                    clauses.push(LoopClause::Sum { expr, into });
                }
                "count" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(&items, &mut pos);
                    clauses.push(LoopClause::Count { expr, into });
                }
                "minimize" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(&items, &mut pos);
                    clauses.push(LoopClause::Minimize { expr, into });
                }
                "maximize" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let expr = items[pos].clone();
                    pos += 1;
                    let into = Self::parse_into_keyword(&items, &mut pos);
                    clauses.push(LoopClause::Maximize { expr, into });
                }
                "thereis" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::Thereis {
                        expr: items[pos].clone(),
                    });
                    pos += 1;
                }
                "do" => {
                    pos += 1;
                    let body = self.collect_until_keyword(items, &mut pos);
                    clauses.push(LoopClause::Do { body });
                }
                "while" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::While {
                        cond: items[pos].clone(),
                    });
                    pos += 1;
                }
                "until" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::Until {
                        cond: items[pos].clone(),
                    });
                    pos += 1;
                }
                "return" => {
                    pos += 1;
                    let expr = if pos < items.len() {
                        let e = items[pos].clone();
                        pos += 1;
                        e
                    } else {
                        nil_form(span)
                    };
                    clauses.push(LoopClause::Return { expr });
                }
                "if" | "when" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let cond = items[pos].clone();
                    pos += 1;
                    let then_clauses = self.parse_sub_clauses(span, items, &mut pos)?;
                    let else_clauses =
                        if pos < items.len() && items[pos].symbol_name() == Some("else") {
                            pos += 1;
                            Some(self.parse_sub_clauses(span, items, &mut pos)?)
                        } else {
                            None
                        };
                    if pos < items.len() && items[pos].symbol_name() == Some("end") {
                        pos += 1;
                    }
                    clauses.push(LoopClause::If {
                        cond,
                        then_clauses,
                        else_clauses,
                    });
                }
                "with" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    let var = items[pos].symbol_name()?.to_string();
                    pos += 1;
                    let expr = if pos < items.len() && items[pos].symbol_name() == Some("=") {
                        pos += 1;
                        if pos >= items.len() {
                            return None;
                        }
                        let e = items[pos].clone();
                        pos += 1;
                        e
                    } else {
                        nil_form(span)
                    };
                    clauses.push(LoopClause::With { var, expr });
                }
                "initially" => {
                    pos += 1;
                    let body = self.collect_until_keyword(items, &mut pos);
                    clauses.push(LoopClause::Initially { body });
                }
                "finally" => {
                    pos += 1;
                    // Handle "finally return expr" — generate a throw in the finally body
                    if pos < items.len() && items[pos].symbol_name() == Some("return") {
                        pos += 1;
                        if pos < items.len() {
                            let expr = items[pos].clone();
                            pos += 1;
                            clauses.push(LoopClause::Finally {
                                body: vec![list_form(
                                    vec![
                                        symbol_form("throw", span),
                                        list_form(
                                            vec![
                                                symbol_form("quote", span),
                                                symbol_form("--cl-loop-tag--", span),
                                            ],
                                            span,
                                        ),
                                        expr,
                                    ],
                                    span,
                                )],
                            });
                        } else {
                            let body = self.collect_until_keyword(items, &mut pos);
                            clauses.push(LoopClause::Finally { body });
                        }
                    } else {
                        let body = self.collect_until_keyword(items, &mut pos);
                        clauses.push(LoopClause::Finally { body });
                    }
                }
                "always" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::Always {
                        expr: items[pos].clone(),
                    });
                    pos += 1;
                }
                "never" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::Never {
                        expr: items[pos].clone(),
                    });
                    pos += 1;
                }
                "repeat" => {
                    pos += 1;
                    if pos >= items.len() {
                        return None;
                    }
                    clauses.push(LoopClause::Repeat {
                        count: items[pos].clone(),
                    });
                    pos += 1;
                }
                _ => {
                    // Unknown keyword — treat as implicit do body expression
                    let body_form = items[pos].clone();
                    pos += 1;
                    clauses.push(LoopClause::Do {
                        body: vec![body_form],
                    });
                }
            }
        }
        Some(clauses)
    }

    fn parse_for_clause(
        &self,
        span: Span,
        items: &[SurfaceForm],
        pos: &mut usize,
    ) -> Option<LoopClause> {
        if *pos >= items.len() {
            return None;
        }
        let var = items[*pos].symbol_name()?.to_string();
        *pos += 1;

        // Peek at next keyword to determine sub-type
        if *pos >= items.len() {
            // bare: (for var) — treat as for-equals nil
            return Some(LoopClause::ForEquals {
                var,
                expr: nil_form(span),
                then_expr: None,
            });
        }

        let next_kw = items[*pos].symbol_name().unwrap_or("");
        match next_kw {
            "from" => {
                *pos += 1;
                let start = items.get(*pos)?.clone();
                *pos += 1;
                // Optional "to" and "by"
                let mut end = None;
                let mut step = None;
                while *pos < items.len() {
                    let kw = items[*pos].symbol_name().unwrap_or("");
                    match kw {
                        "to" => {
                            *pos += 1;
                            end = Some(items.get(*pos)?.clone());
                            *pos += 1;
                        }
                        "by" => {
                            *pos += 1;
                            step = Some(items.get(*pos)?.clone());
                            *pos += 1;
                        }
                        _ => break,
                    }
                }
                Some(LoopClause::ForFrom {
                    var,
                    start,
                    end,
                    step,
                })
            }
            "in" => {
                *pos += 1;
                let list_expr = items.get(*pos)?.clone();
                *pos += 1;
                Some(LoopClause::ForIn { var, list_expr })
            }
            "on" => {
                *pos += 1;
                let list_expr = items.get(*pos)?.clone();
                *pos += 1;
                Some(LoopClause::ForOn { var, list_expr })
            }
            "=" => {
                *pos += 1;
                let expr = items.get(*pos)?.clone();
                *pos += 1;
                // Optional "then step-expr"
                let then_expr = if *pos < items.len() && items[*pos].symbol_name() == Some("then") {
                    *pos += 1;
                    if *pos < items.len() {
                        let step = items[*pos].clone();
                        *pos += 1;
                        Some(step)
                    } else {
                        None
                    }
                } else {
                    None
                };
                Some(LoopClause::ForEquals {
                    var,
                    expr,
                    then_expr,
                })
            }
            _ => {
                // No recognized sub-keyword: treat as for-equals with the next form
                Some(LoopClause::ForEquals {
                    var,
                    expr: nil_form(span),
                    then_expr: None,
                })
            }
        }
    }

    fn parse_sub_clauses(
        &self,
        span: Span,
        items: &[SurfaceForm],
        pos: &mut usize,
    ) -> Option<Vec<LoopClause>> {
        let mut clauses = Vec::new();
        while *pos < items.len() {
            let kw = items[*pos].symbol_name().unwrap_or("");
            match kw {
                "else" | "end" => break,
                "collect" => {
                    *pos += 1;
                    if *pos >= items.len() {
                        break;
                    }
                    clauses.push(LoopClause::Collect {
                        expr: items[*pos].clone(),
                        into: None,
                    });
                    *pos += 1;
                }
                "sum" => {
                    *pos += 1;
                    if *pos >= items.len() {
                        break;
                    }
                    clauses.push(LoopClause::Sum {
                        expr: items[*pos].clone(),
                        into: None,
                    });
                    *pos += 1;
                }
                "count" => {
                    *pos += 1;
                    if *pos >= items.len() {
                        break;
                    }
                    clauses.push(LoopClause::Count {
                        expr: items[*pos].clone(),
                        into: None,
                    });
                    *pos += 1;
                }
                "do" => {
                    *pos += 1;
                    let body = self.collect_until_keyword(items, pos);
                    clauses.push(LoopClause::Do { body });
                }
                "return" => {
                    *pos += 1;
                    let expr = if *pos < items.len() {
                        let e = items[*pos].clone();
                        *pos += 1;
                        e
                    } else {
                        nil_form(span)
                    };
                    clauses.push(LoopClause::Return { expr });
                }
                "append" => {
                    *pos += 1;
                    if *pos >= items.len() {
                        break;
                    }
                    clauses.push(LoopClause::Append {
                        into: None,
                        expr: items[*pos].clone(),
                    });
                    *pos += 1;
                }
                "nconc" => {
                    *pos += 1;
                    if *pos >= items.len() {
                        break;
                    }
                    clauses.push(LoopClause::Nconc {
                        into: None,
                        expr: items[*pos].clone(),
                    });
                    *pos += 1;
                }
                _ => break,
            }
        }
        Some(clauses)
    }

    fn collect_until_keyword(&self, items: &[SurfaceForm], pos: &mut usize) -> Vec<SurfaceForm> {
        let mut body = Vec::new();
        while *pos < items.len() {
            let kw = items[*pos].symbol_name().unwrap_or("");
            if is_loop_keyword(kw) {
                break;
            }
            body.push(items[*pos].clone());
            *pos += 1;
        }
        body
    }
}

// ── cl-loop data structures ────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum LoopClause {
    ForFrom {
        var: String,
        start: SurfaceForm,
        end: Option<SurfaceForm>,
        step: Option<SurfaceForm>,
    },
    ForIn {
        var: String,
        list_expr: SurfaceForm,
    },
    ForOn {
        var: String,
        list_expr: SurfaceForm,
    },
    ForEquals {
        var: String,
        expr: SurfaceForm,
        then_expr: Option<SurfaceForm>,
    },
    Collect {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Append {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Nconc {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Sum {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Count {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Do {
        body: Vec<SurfaceForm>,
    },
    While {
        cond: SurfaceForm,
    },
    Until {
        cond: SurfaceForm,
    },
    Return {
        expr: SurfaceForm,
    },
    If {
        cond: SurfaceForm,
        then_clauses: Vec<LoopClause>,
        else_clauses: Option<Vec<LoopClause>>,
    },
    With {
        var: String,
        expr: SurfaceForm,
    },
    Finally {
        body: Vec<SurfaceForm>,
    },
    Initially {
        body: Vec<SurfaceForm>,
    },
    Always {
        expr: SurfaceForm,
    },
    Never {
        expr: SurfaceForm,
    },
    Thereis {
        expr: SurfaceForm,
    },
    Minimize {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Maximize {
        expr: SurfaceForm,
        into: Option<String>,
    },
    Repeat {
        count: SurfaceForm,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccumKind {
    Collect,
    Append,
    Nconc,
    Sum,
    Count,
    Minimize,
    Maximize,
}

fn is_loop_keyword(kw: &str) -> bool {
    matches!(
        kw,
        "for"
            | "collect"
            | "append"
            | "nconc"
            | "sum"
            | "count"
            | "do"
            | "while"
            | "until"
            | "return"
            | "if"
            | "when"
            | "with"
            | "initially"
            | "finally"
            | "else"
            | "end"
            | "minimize"
            | "maximize"
            | "always"
            | "never"
            | "thereis"
            | "repeat"
            | "into"
    )
}

fn clauses_contain_return(clauses: &[LoopClause]) -> bool {
    clauses.iter().any(|c| match c {
        LoopClause::Return { .. } => true,
        LoopClause::If {
            then_clauses,
            else_clauses,
            ..
        } => {
            clauses_contain_return(then_clauses)
                || else_clauses
                    .as_ref()
                    .map_or(false, |ec| clauses_contain_return(ec))
        }
        _ => false,
    })
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

    #[test]
    fn expands_cl_defun_simple_params() {
        let artifact = compile_source(
            "cl-defun-simple.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-defun add (x y) (+ x y))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"defun\""));
        assert!(rendered.contains("\"add\""));
    }

    #[test]
    fn expands_cl_defun_with_optional() {
        let artifact = compile_source(
            "cl-defun-opt.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-defun foo (a &optional b) (list a b))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"defun\""));
        assert!(rendered.contains("\"destructuring-bind\""));
        assert!(rendered.contains("&optional"));
    }

    #[test]
    fn expands_cl_macrolet_and_uses_macro() {
        let artifact = compile_source(
            "cl-macrolet.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-macrolet ((double (x) (list '+ x x))) (double 5))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        // The macro should have been expanded: (double 5) -> (+ 5 5)
        assert!(rendered.contains("\"+\""));
        assert!(rendered.contains("5"));
    }

    // ── cl-loop tests ─────────────────────────────────────────────

    #[test]
    fn cl_loop_for_from_collect() {
        let artifact = compile_source(
            "cl-loop-1.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 to 5 collect (* x x))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
        assert!(
            !rendered.contains("\"collect\""),
            "collect should be expanded away"
        );
    }

    #[test]
    fn cl_loop_for_in_collect() {
        let artifact = compile_source(
            "cl-loop-2.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3) collect (* x 2))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
        assert!(rendered.contains("\"car\""));
        assert!(rendered.contains("\"cdr\""));
    }

    #[test]
    fn cl_loop_sum() {
        let artifact = compile_source(
            "cl-loop-3.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 to 10 sum x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"+\""));
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_count() {
        let artifact = compile_source(
            "cl-loop-4.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3 4 5) count (> x 3))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"if\""));
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_do_body() {
        let artifact = compile_source(
            "cl-loop-5.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 to 3 do (foo x))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"foo\""));
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_with_binding() {
        let artifact = compile_source(
            "cl-loop-6.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop with y = 10 for x from 1 to 3 collect (+ x y))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_while_termination() {
        let artifact = compile_source(
            "cl-loop-7.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3 4 5) while (< x 4) collect x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
        assert!(rendered.contains("\"and\""));
    }

    #[test]
    fn cl_loop_return() {
        let artifact = compile_source(
            "cl-loop-8.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 to 100 if (> x 5) return x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"catch\""));
        assert!(rendered.contains("\"throw\""));
    }

    #[test]
    fn cl_loop_by_step() {
        let artifact = compile_source(
            "cl-loop-9.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 0 to 10 by 2 collect x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_initially_finally() {
        let artifact = compile_source(
            "cl-loop-10.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop initially (bar) for x from 1 to 3 collect x finally (baz))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"bar\""));
        assert!(rendered.contains("\"baz\""));
        assert!(rendered.contains("\"nreverse\""));
    }

    #[test]
    fn cl_loop_append_accumulation() {
        let artifact = compile_source(
            "cl-loop-11.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list (list 1 2) (list 3 4)) append x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"append\""));
    }

    #[test]
    fn cl_loop_for_on() {
        let artifact = compile_source(
            "cl-loop-12.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x on (list 1 2 3) collect (car x))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"car\""));
        assert!(rendered.contains("\"cdr\""));
    }

    #[test]
    fn cl_loop_empty() {
        let artifact = compile_source(
            "cl-loop-empty.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
    }

    #[test]
    fn cl_loop_always_short_circuit() {
        let artifact = compile_source(
            "cl-loop-always.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3) always (> x 0))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(
            rendered.contains("\"--cl-always--\""),
            "should have --cl-always-- flag for always clause"
        );
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_never_short_circuit() {
        let artifact = compile_source(
            "cl-loop-never.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3) never (< x 0))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(
            rendered.contains("\"--cl-always--\""),
            "never clause should use --cl-always-- flag"
        );
    }

    #[test]
    fn cl_loop_sum_accumulation() {
        let artifact = compile_source(
            "cl-loop-sum.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x in (list 1 2 3) sum x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"+\""), "sum should use + operator");
        assert!(rendered.contains("\"--cl-acc-"));
    }

    #[test]
    fn cl_loop_with_and_finally() {
        let artifact = compile_source(
            "cl-loop-with2.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop with total = 0 for x in (list 1 2 3) do (setq total (+ total x)) finally return total)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"total\""));
    }

    #[test]
    fn cl_loop_do_and_message() {
        let artifact = compile_source(
            "cl-loop-do2.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 to 3 do (message \"%d\" x))",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"message\""));
        assert!(rendered.contains("\"while\""));
    }

    #[test]
    fn cl_loop_for_from_no_end() {
        // for x from 1 (no end) — should create infinite loop with no while test
        let artifact = compile_source(
            "cl-loop-noend.el",
            ";;; -*- lexical-binding: t; -*-\n(cl-loop for x from 1 while (< x 5) collect x)",
        );
        assert_eq!(artifact.diagnostics, Vec::new());
        let rendered = format!("{:?}", artifact.surface);
        assert!(rendered.contains("\"while\""));
        assert!(rendered.contains("\"<\""));
    }
}
