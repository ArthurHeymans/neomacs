use std::collections::HashMap;

use crate::diagnostic::Diagnostic;
use crate::expand_eval::{MacroEnv, MacroEval};
use crate::expand_value::{surface_to_value, value_to_surface, MacroValue};
use crate::source::Span;
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

#[derive(Clone, Debug, PartialEq)]
pub struct ExpandOutput {
    pub forms: Vec<SurfaceForm>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn expand_forms(forms: Vec<SurfaceForm>) -> ExpandOutput {
    let mut expander = Expander {
        macros: HashMap::new(),
        diagnostics: Vec::new(),
        expansion_depth: 0,
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
    expansion_depth: usize,
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
        match form.kind {
            SurfaceKind::List(items) => self.expand_list(form.span, items),
            SurfaceKind::DottedList(items, tail) => SurfaceForm::new(
                SurfaceKind::DottedList(
                    items
                        .into_iter()
                        .map(|item| self.expand_form(item))
                        .collect(),
                    Box::new(self.expand_form(*tail)),
                ),
                form.span,
            ),
            SurfaceKind::Vector(_) => form,
            SurfaceKind::Quote(_)
            | SurfaceKind::FunctionQuote(_)
            | SurfaceKind::Backquote(_)
            | SurfaceKind::Comma(_)
            | SurfaceKind::CommaAt(_)
            | SurfaceKind::Atom(_) => form,
        }
    }

    fn expand_list(&mut self, span: Span, items: Vec<SurfaceForm>) -> SurfaceForm {
        let Some(head) = items.first().and_then(SurfaceForm::symbol_name) else {
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
        if self.expansion_depth >= 100 {
            self.error(span, "macro expansion exceeded recursion limit");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        self.expansion_depth += 1;
        let expanded = self
            .invoke_macro(&def, &items[1..])
            .unwrap_or_else(|| SurfaceForm::new(SurfaceKind::List(items), span));
        let result = self.expand_form(expanded);
        self.expansion_depth -= 1;
        result
    }

    fn invoke_macro(&mut self, def: &MacroDef, args: &[SurfaceForm]) -> Option<SurfaceForm> {
        let arg_values: Vec<MacroValue> = args.iter().map(surface_to_value).collect();

        if arg_values.len() < def.params.required.len() {
            self.error(
                def.span,
                format!(
                    "macro requires at least {} arguments, got {}",
                    def.params.required.len(),
                    arg_values.len()
                ),
            );
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
            self.error(span, "push requires a value and a symbol place");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        let Some(place) = items[2].symbol_name().map(str::to_string) else {
            self.error(
                items[2].span,
                "push supports only simple symbol places for now",
            );
            return SurfaceForm::new(SurfaceKind::List(items), span);
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
            self.error(span, "pop requires a symbol place");
            return SurfaceForm::new(SurfaceKind::List(items), span);
        }
        let Some(place) = items[1].symbol_name().map(str::to_string) else {
            self.error(
                items[1].span,
                "pop supports only simple symbol places for now",
            );
            return SurfaceForm::new(SurfaceKind::List(items), span);
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
}
