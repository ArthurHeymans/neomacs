use std::collections::HashMap;

use crate::diagnostic::Diagnostic;
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
        self.expansion_depth -= 1;
        self.expand_form(expanded)
    }

    fn invoke_macro(&mut self, def: &MacroDef, args: &[SurfaceForm]) -> Option<SurfaceForm> {
        let mut env = MacroEnv::default();
        if args.len() < def.params.required.len() {
            self.error(
                def.span,
                format!(
                    "macro requires at least {} arguments, got {}",
                    def.params.required.len(),
                    args.len()
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
            && args.len() > max_arity
        {
            self.error(
                def.span,
                format!(
                    "macro requires at most {max_arity} arguments, got {}",
                    args.len()
                ),
            );
            return None;
        }

        for (name, arg) in def.params.required.iter().zip(args.iter()) {
            env.bind(name.clone(), arg.clone());
        }
        let optional_start = def.params.required.len();
        for (index, name) in def.params.optional.iter().enumerate() {
            env.bind(
                name.clone(),
                args.get(optional_start + index)
                    .cloned()
                    .unwrap_or_else(|| nil_form(def.span)),
            );
        }
        if let Some(rest) = &def.params.rest {
            let rest_start = args.len().min(optional_start + def.params.optional.len());
            env.bind(
                rest.clone(),
                list_form(args[rest_start..].to_vec(), def.span),
            );
        }
        if let Some(environment) = &def.params.environment {
            env.bind(environment.clone(), nil_form(def.span));
        }

        let mut value = nil_form(def.span);
        for form in &def.body {
            value = self.eval_macro_expr(form, &mut env)?;
        }
        Some(value)
    }

    fn eval_macro_expr(&mut self, form: &SurfaceForm, env: &mut MacroEnv) -> Option<SurfaceForm> {
        match &form.kind {
            SurfaceKind::Atom(SurfaceAtom::Symbol(name)) => {
                Some(env.lookup(name).cloned().unwrap_or_else(|| form.clone()))
            }
            SurfaceKind::Atom(_) => Some(form.clone()),
            SurfaceKind::Quote(inner) => Some((**inner).clone()),
            SurfaceKind::FunctionQuote(inner) => Some((**inner).clone()),
            SurfaceKind::Backquote(inner) => self.eval_quasiquote_form(inner, env, 1),
            SurfaceKind::Comma(_) => {
                self.error(form.span, "comma is only valid inside backquote");
                None
            }
            SurfaceKind::CommaAt(_) => {
                self.error(
                    form.span,
                    "unquote-splicing is only valid inside a backquote list or vector",
                );
                None
            }
            SurfaceKind::Vector(_) | SurfaceKind::DottedList(_, _) => Some(form.clone()),
            SurfaceKind::List(items) => self.eval_macro_list(form.span, items, env),
        }
    }

    fn eval_macro_list(
        &mut self,
        span: Span,
        items: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Option<SurfaceForm> {
        let Some(head) = items.first().and_then(SurfaceForm::symbol_name) else {
            return Some(SurfaceForm::new(SurfaceKind::List(Vec::new()), span));
        };
        match head {
            "quote" => {
                if items.len() != 2 {
                    self.error(span, "quote requires exactly one argument");
                    return None;
                }
                Some(items[1].clone())
            }
            "function" => {
                if items.len() != 2 {
                    self.error(span, "function requires exactly one argument");
                    return None;
                }
                Some(items[1].clone())
            }
            "progn" => {
                let mut value = nil_form(span);
                for form in &items[1..] {
                    value = self.eval_macro_expr(form, env)?;
                }
                Some(value)
            }
            "if" => self.eval_macro_if(span, &items[1..], env),
            "and" => self.eval_macro_and(span, &items[1..], env),
            "or" => self.eval_macro_or(span, &items[1..], env),
            "let" => self.eval_macro_let(span, &items[1..], false, env),
            "let*" => self.eval_macro_let(span, &items[1..], true, env),
            "setq" => self.eval_macro_setq(span, &items[1..], env),
            "list" => items[1..]
                .iter()
                .map(|item| self.eval_macro_expr(item, env))
                .collect::<Option<Vec<_>>>()
                .map(|items| list_form(items, span)),
            "cons" => {
                if items.len() != 3 {
                    self.error(span, "cons requires two arguments");
                    return None;
                }
                let car = self.eval_macro_expr(&items[1], env)?;
                let cdr = self.eval_macro_expr(&items[2], env)?;
                Some(cons_form(car, cdr, span))
            }
            "append" => {
                let parts = items[1..]
                    .iter()
                    .map(|item| self.eval_macro_expr(item, env))
                    .collect::<Option<Vec<_>>>()?;
                self.append_forms(parts, span)
            }
            "car" | "car-safe" => {
                if items.len() != 2 {
                    self.error(span, "car requires one argument");
                    return None;
                }
                let value = self.eval_macro_expr(&items[1], env)?;
                Some(car_form(&value).unwrap_or_else(|| nil_form(span)))
            }
            "cdr" | "cdr-safe" => {
                if items.len() != 2 {
                    self.error(span, "cdr requires one argument");
                    return None;
                }
                let value = self.eval_macro_expr(&items[1], env)?;
                Some(cdr_form(&value, span).unwrap_or_else(|| nil_form(span)))
            }
            "cadr" => self.eval_macro_car_cdr(span, &items[1..], env, &["cdr", "car"]),
            "caddr" => self.eval_macro_car_cdr(span, &items[1..], env, &["cdr", "cdr", "car"]),
            "nth" => self.eval_macro_nth(span, &items[1..], env),
            _ => {
                self.error(span, format!("unsupported macro-time call `{head}`"));
                None
            }
        }
    }

    fn eval_macro_if(
        &mut self,
        span: Span,
        tail: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Option<SurfaceForm> {
        if tail.len() < 2 {
            self.error(span, "if requires a test and then form");
            return None;
        }
        let test = self.eval_macro_expr(&tail[0], env)?;
        if !is_nil(&test) {
            return self.eval_macro_expr(&tail[1], env);
        }
        let mut value = nil_form(span);
        for form in &tail[2..] {
            value = self.eval_macro_expr(form, env)?;
        }
        Some(value)
    }

    fn eval_macro_and(
        &mut self,
        span: Span,
        forms: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Option<SurfaceForm> {
        let mut value = symbol_form("t", span);
        for form in forms {
            value = self.eval_macro_expr(form, env)?;
            if is_nil(&value) {
                return Some(value);
            }
        }
        Some(value)
    }

    fn eval_macro_or(
        &mut self,
        span: Span,
        forms: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Option<SurfaceForm> {
        for form in forms {
            let value = self.eval_macro_expr(form, env)?;
            if !is_nil(&value) {
                return Some(value);
            }
        }
        Some(nil_form(span))
    }

    fn eval_macro_let(
        &mut self,
        span: Span,
        tail: &[SurfaceForm],
        sequential: bool,
        env: &mut MacroEnv,
    ) -> Option<SurfaceForm> {
        if tail.len() < 2 {
            self.error(span, "let requires bindings and body");
            return None;
        }
        let SurfaceKind::List(bindings) = &tail[0].kind else {
            self.error(tail[0].span, "let bindings must be a proper list");
            return None;
        };
        let mut child = env.clone();
        if sequential {
            for binding in bindings {
                let (name, value) = self.eval_macro_binding(binding, &mut child)?;
                child.bind(name, value);
            }
        } else {
            let mut values = Vec::new();
            for binding in bindings {
                values.push(self.eval_macro_binding(binding, env)?);
            }
            for (name, value) in values {
                child.bind(name, value);
            }
        }
        let mut value = nil_form(span);
        for form in &tail[1..] {
            value = self.eval_macro_expr(form, &mut child)?;
        }
        Some(value)
    }

    fn eval_macro_binding(
        &mut self,
        binding: &SurfaceForm,
        env: &mut MacroEnv,
    ) -> Option<(String, SurfaceForm)> {
        if let Some(name) = binding.symbol_name() {
            return Some((name.to_string(), nil_form(binding.span)));
        }
        let SurfaceKind::List(items) = &binding.kind else {
            self.error(
                binding.span,
                "let binding must be a symbol or (symbol init)",
            );
            return None;
        };
        if items.is_empty() || items.len() > 2 {
            self.error(
                binding.span,
                "let binding must be a symbol or (symbol init)",
            );
            return None;
        }
        let Some(name) = items[0].symbol_name().map(str::to_string) else {
            self.error(items[0].span, "let binding name must be a symbol");
            return None;
        };
        let value = if let Some(init) = items.get(1) {
            self.eval_macro_expr(init, env)?
        } else {
            nil_form(binding.span)
        };
        Some((name, value))
    }

    fn eval_macro_setq(
        &mut self,
        span: Span,
        pairs: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Option<SurfaceForm> {
        if pairs.is_empty() || !pairs.len().is_multiple_of(2) {
            self.error(span, "setq requires symbol/value pairs");
            return None;
        }
        let mut value = nil_form(span);
        for pair in pairs.chunks_exact(2) {
            let Some(name) = pair[0].symbol_name().map(str::to_string) else {
                self.error(pair[0].span, "setq target must be a symbol");
                return None;
            };
            value = self.eval_macro_expr(&pair[1], env)?;
            env.bind(name, value.clone());
        }
        Some(value)
    }

    fn eval_macro_car_cdr(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
        ops: &[&str],
    ) -> Option<SurfaceForm> {
        if args.len() != 1 {
            self.error(span, "list accessor requires one argument");
            return None;
        }
        let mut value = self.eval_macro_expr(&args[0], env)?;
        for op in ops {
            value = match *op {
                "car" => car_form(&value).unwrap_or_else(|| nil_form(span)),
                "cdr" => cdr_form(&value, span).unwrap_or_else(|| nil_form(span)),
                _ => unreachable!("known accessor op"),
            };
        }
        Some(value)
    }

    fn eval_macro_nth(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Option<SurfaceForm> {
        if args.len() != 2 {
            self.error(span, "nth requires an index and list");
            return None;
        }
        let index = self.eval_macro_expr(&args[0], env)?;
        let Some(index) = fixnum_value(&index) else {
            self.error(args[0].span, "nth index must be an integer");
            return None;
        };
        if index < 0 {
            return Some(nil_form(span));
        }
        let values = self.eval_macro_expr(&args[1], env)?;
        let values = proper_list_elements(&values)?;
        Some(
            values
                .get(index as usize)
                .cloned()
                .unwrap_or_else(|| nil_form(span)),
        )
    }

    fn eval_quasiquote_form(
        &mut self,
        form: &SurfaceForm,
        env: &mut MacroEnv,
        depth: usize,
    ) -> Option<SurfaceForm> {
        match &form.kind {
            SurfaceKind::Comma(inner) if depth == 1 => self.eval_macro_expr(inner, env),
            SurfaceKind::Comma(inner) => {
                self.eval_quasiquote_prefixed("unquote", inner, env, depth - 1, form.span)
            }
            SurfaceKind::CommaAt(_) if depth == 1 => {
                self.error(
                    form.span,
                    "unquote-splicing is only valid inside a backquote list or vector",
                );
                None
            }
            SurfaceKind::CommaAt(inner) => {
                self.eval_quasiquote_prefixed("unquote-splicing", inner, env, depth - 1, form.span)
            }
            SurfaceKind::Backquote(inner) => {
                self.eval_quasiquote_prefixed("quasiquote", inner, env, depth + 1, form.span)
            }
            SurfaceKind::Quote(inner) => {
                self.eval_quasiquote_prefixed("quote", inner, env, depth, form.span)
            }
            SurfaceKind::FunctionQuote(inner) => {
                self.eval_quasiquote_prefixed("function", inner, env, depth, form.span)
            }
            SurfaceKind::List(items) => self.eval_quasiquote_list(form.span, items, env, depth),
            SurfaceKind::DottedList(items, tail) => {
                self.eval_quasiquote_dotted_list(form.span, items, tail, env, depth)
            }
            SurfaceKind::Vector(items) => self.eval_quasiquote_vector(form.span, items, env, depth),
            SurfaceKind::Atom(_) => Some(form.clone()),
        }
    }

    fn eval_quasiquote_prefixed(
        &mut self,
        name: &str,
        inner: &SurfaceForm,
        env: &mut MacroEnv,
        depth: usize,
        span: Span,
    ) -> Option<SurfaceForm> {
        if let SurfaceKind::CommaAt(splice) = &inner.kind
            && depth == 1
        {
            let spliced = self.eval_macro_expr(splice, env)?;
            return self.append_forms(
                vec![list_form(vec![symbol_form(name, span)], span), spliced],
                span,
            );
        }
        Some(list_form(
            vec![
                symbol_form(name, span),
                self.eval_quasiquote_form(inner, env, depth)?,
            ],
            span,
        ))
    }

    fn eval_quasiquote_list(
        &mut self,
        span: Span,
        items: &[SurfaceForm],
        env: &mut MacroEnv,
        depth: usize,
    ) -> Option<SurfaceForm> {
        let (parts, has_splice) = self.eval_quasiquote_list_parts(items, env, depth, span)?;
        if has_splice {
            self.append_forms(parts, span)
        } else {
            Some(list_form(parts, span))
        }
    }

    fn eval_quasiquote_dotted_list(
        &mut self,
        span: Span,
        items: &[SurfaceForm],
        tail: &SurfaceForm,
        env: &mut MacroEnv,
        depth: usize,
    ) -> Option<SurfaceForm> {
        let (mut parts, has_splice) = self.eval_quasiquote_list_parts(items, env, depth, span)?;
        let tail = self.eval_quasiquote_form(tail, env, depth)?;
        if has_splice {
            parts.push(tail);
            return self.append_forms(parts, span);
        }
        let mut result = tail;
        for item in parts.into_iter().rev() {
            result = cons_form(item, result, span);
        }
        Some(result)
    }

    fn eval_quasiquote_vector(
        &mut self,
        span: Span,
        items: &[SurfaceForm],
        env: &mut MacroEnv,
        depth: usize,
    ) -> Option<SurfaceForm> {
        let (parts, has_splice) = self.eval_quasiquote_list_parts(items, env, depth, span)?;
        if !has_splice {
            return Some(SurfaceForm::new(SurfaceKind::Vector(parts), span));
        }
        let list = self.append_forms(parts, span)?;
        let Some(items) = proper_list_elements(&list) else {
            self.error(span, "vector unquote-splicing requires a proper list");
            return None;
        };
        Some(SurfaceForm::new(SurfaceKind::Vector(items), span))
    }

    fn eval_quasiquote_list_parts(
        &mut self,
        items: &[SurfaceForm],
        env: &mut MacroEnv,
        depth: usize,
        span: Span,
    ) -> Option<(Vec<SurfaceForm>, bool)> {
        let mut parts = Vec::new();
        let mut segment = Vec::new();
        let mut has_splice = false;
        for item in items {
            if let SurfaceKind::CommaAt(inner) = &item.kind
                && depth == 1
            {
                flush_quasiquote_segment(&mut parts, &mut segment, span);
                parts.push(self.eval_macro_expr(inner, env)?);
                has_splice = true;
                continue;
            }
            segment.push(self.eval_quasiquote_form(item, env, depth)?);
        }
        if has_splice {
            flush_quasiquote_segment(&mut parts, &mut segment, span);
        } else {
            parts = segment;
        }
        Some((parts, has_splice))
    }

    fn append_forms(&mut self, parts: Vec<SurfaceForm>, span: Span) -> Option<SurfaceForm> {
        append_forms(parts, span).or_else(|| {
            self.error(
                span,
                "append requires proper lists before the final argument",
            );
            None
        })
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

#[derive(Clone, Debug, Default, PartialEq)]
struct MacroEnv {
    bindings: HashMap<String, SurfaceForm>,
}

impl MacroEnv {
    fn bind(&mut self, name: String, value: SurfaceForm) {
        self.bindings.insert(name, value);
    }

    fn lookup(&self, name: &str) -> Option<&SurfaceForm> {
        self.bindings.get(name)
    }
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

fn is_nil(form: &SurfaceForm) -> bool {
    matches!(form.kind, SurfaceKind::Atom(SurfaceAtom::Nil))
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

fn fixnum_value(form: &SurfaceForm) -> Option<i64> {
    match form.kind {
        SurfaceKind::Atom(SurfaceAtom::Int(value)) => Some(value),
        _ => None,
    }
}

fn cons_form(car: SurfaceForm, cdr: SurfaceForm, span: Span) -> SurfaceForm {
    match cdr.kind {
        SurfaceKind::Atom(SurfaceAtom::Nil) => list_form(vec![car], span),
        SurfaceKind::List(mut items) => {
            items.insert(0, car);
            list_form(items, span)
        }
        SurfaceKind::DottedList(mut items, tail) => {
            items.insert(0, car);
            SurfaceForm::new(SurfaceKind::DottedList(items, tail), span)
        }
        _ => SurfaceForm::new(SurfaceKind::DottedList(vec![car], Box::new(cdr)), span),
    }
}

fn car_form(form: &SurfaceForm) -> Option<SurfaceForm> {
    match &form.kind {
        SurfaceKind::Atom(SurfaceAtom::Nil) => None,
        SurfaceKind::List(items) => items.first().cloned(),
        SurfaceKind::DottedList(items, tail) => {
            items.first().cloned().or_else(|| Some((**tail).clone()))
        }
        _ => None,
    }
}

fn cdr_form(form: &SurfaceForm, span: Span) -> Option<SurfaceForm> {
    match &form.kind {
        SurfaceKind::Atom(SurfaceAtom::Nil) => None,
        SurfaceKind::List(items) => Some(list_form(items.iter().skip(1).cloned().collect(), span)),
        SurfaceKind::DottedList(items, tail) => match items.len() {
            0 => Some((**tail).clone()),
            1 => Some((**tail).clone()),
            _ => Some(SurfaceForm::new(
                SurfaceKind::DottedList(items.iter().skip(1).cloned().collect(), tail.clone()),
                span,
            )),
        },
        _ => None,
    }
}

fn append_forms(parts: Vec<SurfaceForm>, span: Span) -> Option<SurfaceForm> {
    let Some((last, prefixes)) = parts.split_last() else {
        return Some(nil_form(span));
    };
    let mut result = last.clone();
    for prefix in prefixes.iter().rev() {
        let values = proper_list_elements(prefix)?;
        for value in values.into_iter().rev() {
            result = cons_form(value, result, span);
        }
    }
    Some(result)
}

fn proper_list_elements(form: &SurfaceForm) -> Option<Vec<SurfaceForm>> {
    match &form.kind {
        SurfaceKind::Atom(SurfaceAtom::Nil) => Some(Vec::new()),
        SurfaceKind::List(items) => Some(items.clone()),
        _ => None,
    }
}

fn flush_quasiquote_segment(
    parts: &mut Vec<SurfaceForm>,
    segment: &mut Vec<SurfaceForm>,
    span: Span,
) {
    if !segment.is_empty() {
        parts.push(list_form(std::mem::take(segment), span));
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
