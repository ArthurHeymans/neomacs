use std::collections::HashMap;

use crate::diagnostic::Diagnostic;
use crate::expand_value::{surface_to_value, MacroValue};
use crate::source::Span;
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

#[derive(Clone, Debug, Default)]
pub struct MacroEnv {
    bindings: HashMap<String, MacroValue>,
}

impl MacroEnv {
    pub fn bind(&mut self, name: String, value: MacroValue) {
        self.bindings.insert(name, value);
    }

    pub fn lookup(&self, name: &str) -> Option<&MacroValue> {
        self.bindings.get(name)
    }

    pub fn remove(&mut self, name: &str) {
        self.bindings.remove(name);
    }
}

pub struct MacroEval {
    diagnostics: Vec<Diagnostic>,
}

impl MacroEval {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    pub fn eval_progn(
        &mut self,
        forms: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        let mut result = MacroValue::Nil;
        for form in forms {
            result = self.eval(form, env)?;
        }
        Ok(result)
    }

    pub fn eval(
        &mut self,
        form: &SurfaceForm,
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        match &form.kind {
            SurfaceKind::Atom(atom) => Ok(self.eval_atom(atom, env)),
            SurfaceKind::List(items) => {
                if items.is_empty() {
                    return Ok(MacroValue::Nil);
                }
                self.eval_list_form(form.span, items, env)
            }
            SurfaceKind::Quote(inner) => Ok(surface_to_value(inner)),
            SurfaceKind::FunctionQuote(inner) => Ok(surface_to_value(inner)),
            SurfaceKind::Backquote(inner) => self.eval_quasiquote(inner, env, 1),
            SurfaceKind::Comma(_) => {
                self.error(form.span, "comma is only valid inside backquote");
                Err(())
            }
            SurfaceKind::CommaAt(_) => {
                self.error(
                    form.span,
                    "unquote-splicing is only valid inside a backquote list or vector",
                );
                Err(())
            }
            SurfaceKind::DottedList(_, _) | SurfaceKind::Vector(_) => {
                Ok(surface_to_value(form))
            }
        }
    }

    fn eval_atom(&mut self, atom: &SurfaceAtom, env: &mut MacroEnv) -> MacroValue {
        match atom {
            SurfaceAtom::Nil => MacroValue::Nil,
            SurfaceAtom::True => MacroValue::Symbol("t".into()),
            SurfaceAtom::Int(n) => MacroValue::Int(*n),
            SurfaceAtom::Float(_) => MacroValue::Nil,
            SurfaceAtom::Symbol(name) => {
                env.lookup(name).cloned().unwrap_or(MacroValue::Symbol(name.clone()))
            }
            SurfaceAtom::String(s) => MacroValue::String(s.clone()),
            SurfaceAtom::Char(c) => MacroValue::Int(*c),
        }
    }

    fn eval_list_form(
        &mut self,
        span: Span,
        items: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        let head = items.first().and_then(|f| f.symbol_name());
        match head {
            Some("quote") => {
                if items.len() != 2 {
                    self.error(span, "quote requires exactly one argument");
                    return Err(());
                }
                Ok(surface_to_value(&items[1]))
            }
            Some("if") => self.eval_if(span, items, env),
            Some("cond") => self.eval_cond(span, &items[1..], env),
            Some("and") => self.eval_and(&items[1..], env),
            Some("or") => self.eval_or(&items[1..], env),
            Some("when") => {
                // (when cond body...) => (if cond (progn body...))
                let cond_val = self.eval(&items[1], env)?;
                if cond_val.is_truthy() {
                    self.eval_progn(&items[2..], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }
            Some("unless") => {
                // (unless cond body...) => (if (not cond) (progn body...))
                let cond_val = self.eval(&items[1], env)?;
                if cond_val.is_nil() {
                    self.eval_progn(&items[2..], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }
            Some("let") => self.eval_let(span, &items[1..], env, false),
            Some("let*") => self.eval_let(span, &items[1..], env, true),
            Some("setq") => self.eval_setq(span, &items[1..], env),
            Some("progn") => self.eval_progn(&items[1..], env),
            Some("while") => self.eval_while(span, &items[1..], env),

            Some("car") | Some("first") => {
                self.eval_unary(span, "car", &items[1..], env, |v| v.car())
            }
            Some("car-safe") => {
                self.eval_unary(span, "car-safe", &items[1..], env, |v| {
                    if v.is_cons() { v.car() } else { MacroValue::Nil }
                })
            }
            Some("cdr") | Some("rest") => {
                self.eval_unary(span, "cdr", &items[1..], env, |v| v.cdr())
            }
            Some("cdr-safe") => {
                self.eval_unary(span, "cdr-safe", &items[1..], env, |v| {
                    if v.is_cons() { v.cdr() } else { MacroValue::Nil }
                })
            }
            Some("cadr") => {
                self.eval_unary(span, "cadr", &items[1..], env, |v| v.cdr().car())
            }
            Some("caddr") => {
                self.eval_unary(span, "caddr", &items[1..], env, |v| v.cdr().cdr().car())
            }
            Some("cddr") => {
                self.eval_unary(span, "cddr", &items[1..], env, |v| v.cdr().cdr())
            }
            Some("cons") => self.eval_binary(span, &items[1..], env, MacroValue::cons),
            Some("list") => {
                let mut results = Vec::new();
                for arg in &items[1..] {
                    results.push(self.eval(arg, env)?);
                }
                Ok(MacroValue::list(results))
            }
            Some("append") => self.eval_append(span, &items[1..], env),
            Some("nth") => self.eval_nth(span, &items[1..], env),
            Some("length") => self.eval_length(span, &items[1..], env),

            Some("null") | Some("not") => self.eval_unary(span, "null", &items[1..], env, |v| {
                MacroValue::from_bool(v.is_nil())
            }),
            Some("consp") => self.eval_unary(span, "consp", &items[1..], env, |v| {
                MacroValue::from_bool(v.is_cons())
            }),
            Some("listp") => self.eval_unary(span, "listp", &items[1..], env, |v| {
                MacroValue::from_bool(v.is_list())
            }),
            Some("symbolp") => self.eval_unary(span, "symbolp", &items[1..], env, |v| {
                MacroValue::from_bool(v.is_symbol())
            }),
            Some("stringp") => self.eval_unary(span, "stringp", &items[1..], env, |v| {
                MacroValue::from_bool(v.is_string())
            }),
            Some("numberp") => self.eval_unary(span, "numberp", &items[1..], env, |v| {
                MacroValue::from_bool(v.is_int())
            }),
            Some("eq") | Some("eql") => self.eval_binary_pred(span, &items[1..], env, |a, b| {
                a == b
            }),
            Some("equal") => self.eval_binary_pred(span, &items[1..], env, |a, b| a == b),

            Some("+") => self.eval_fold(span, &items[1..], env, 0i64, |a, b| a.wrapping_add(b)),
            Some("-") => self.eval_sub(span, &items[1..], env),
            Some("*") => self.eval_fold(span, &items[1..], env, 1i64, |a, b| a.wrapping_mul(b)),

            Some("=") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a == b),
            Some("<") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a < b),
            Some(">") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a > b),
            Some("<=") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a <= b),
            Some(">=") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a >= b),

            Some("symbol-name") => {
                self.eval_unary(span, "symbol-name", &items[1..], env, |v| {
                    v.as_symbol_name()
                        .map(|s| MacroValue::String(s.into()))
                        .unwrap_or(MacroValue::Nil)
                })
            }
            Some("intern") => {
                self.eval_unary(span, "intern", &items[1..], env, |v| {
                    match v {
                        MacroValue::String(s) => MacroValue::Symbol(s.clone()),
                        other => other.clone(),
                    }
                })
            }
            Some("make-symbol") => {
                self.eval_unary(span, "make-symbol", &items[1..], env, |v| {
                    match v {
                        MacroValue::String(s) => MacroValue::Symbol(format!(" {}", s)),
                        other => other.clone(),
                    }
                })
            }
            Some("downcase") => {
                self.eval_unary(span, "downcase", &items[1..], env, |v| {
                    match v {
                        MacroValue::String(s) => MacroValue::String(s.to_lowercase()),
                        MacroValue::Symbol(s) => MacroValue::Symbol(s.to_lowercase()),
                        other => other.clone(),
                    }
                })
            }
            Some("upcase") => {
                self.eval_unary(span, "upcase", &items[1..], env, |v| {
                    match v {
                        MacroValue::String(s) => MacroValue::String(s.to_uppercase()),
                        MacroValue::Symbol(s) => MacroValue::Symbol(s.to_uppercase()),
                        other => other.clone(),
                    }
                })
            }
            Some("concat") => self.eval_concat(span, &items[1..], env),
            Some("substring") => self.eval_substring(span, &items[1..], env),
            Some("string=") => self.eval_binary_pred(span, &items[1..], env, |a, b| {
                a.as_string() == b.as_string() && a.is_string() && b.is_string()
            }),
            Some("format") => self.eval_format(span, &items[1..], env),

            Some("error") => {
                let msg = self.eval(&items[1], env);
                let msg_text = match &msg {
                    Ok(MacroValue::String(s)) => s.clone(),
                    Ok(_) => "unknown error".into(),
                    Err(_) => "error during error evaluation".into(),
                };
                self.error(span, format!("macro expansion error: {}", msg_text));
                Err(())
            }

            Some("ignore-errors") => {
                // Best-effort: evaluate body, swallow errors
                for form in &items[1..] {
                    let _ = self.eval(form, env);
                }
                Ok(MacroValue::Nil)
            }

            Some("require") => {
                // At macro expansion time, just return nil (feature already loaded)
                Ok(MacroValue::Nil)
            }

            Some("macroexp-warn-and-return") => {
                // (macroexp-warn-and-return MSG FORM &optional CATEGORY FILE)
                // Return the form, ignore the warning at macro expansion time.
                if items.len() >= 3 {
                    self.eval(&items[2], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("macroexp-let2") => {
                // (macroexp-let2 VAR SYM BODY)
                // Bind VAR to SYM and evaluate BODY. If VAR is nil, use SYM directly.
                if items.len() >= 4 {
                    let var_name = items[1].symbol_name();
                    let sym_val = self.eval(&items[2], env)?;
                    match var_name {
                        Some(name) if !name.is_empty() => {
                            env.bind(name.to_string(), sym_val);
                            let result = self.eval(&items[3], env);
                            env.remove(name);
                            result
                        }
                        _ => self.eval(&items[3], env),
                    }
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("called-interactively-p") => {
                // Always return nil at macro expansion time
                Ok(MacroValue::Nil)
            }

            Some("keywordp") => {
                self.eval_unary(span, "keywordp", &items[1..], env, |v| {
                    match v {
                        MacroValue::Symbol(s) => MacroValue::from_bool(s.starts_with(':')),
                        _ => MacroValue::from_bool(false),
                    }
                })
            }

            Some("evenp") => {
                self.eval_unary(span, "evenp", &items[1..], env, |v| {
                    match v {
                        MacroValue::Int(n) => MacroValue::from_bool(n % 2 == 0),
                        _ => MacroValue::from_bool(false),
                    }
                })
            }

            Some("zerop") => {
                self.eval_unary(span, "zerop", &items[1..], env, |v| {
                    match v {
                        MacroValue::Int(n) => MacroValue::from_bool(n == 0),
                        _ => MacroValue::from_bool(false),
                    }
                })
            }

            Some("capitalize") => {
                self.eval_unary(span, "capitalize", &items[1..], env, |v| {
                    match v {
                        MacroValue::String(s) => {
                            let cap: String = s.chars().enumerate().map(|(i, c)| {
                                if i == 0 { c.to_uppercase().to_string() } else { c.to_lowercase().to_string() }
                            }).collect();
                            MacroValue::String(cap)
                        }
                        other => other.clone(),
                    }
                })
            }

            Some("macroexp--fgrep") => {
                // (macroexp--fgrep BINDINGS FORM) — check if form references any binding
                // Simplified: return nil
                Ok(MacroValue::Nil)
            }

            Some("macroexp-progn") => {
                // (macroexp-progn FORMS) — return a progn form
                let mut results = Vec::new();
                for form in &items[1..] {
                    results.push(self.eval(form, env)?);
                }
                Ok(MacroValue::list(results))
            }

            Some("replace-regexp-in-string") => {
                // Simplified: return empty string
                Ok(MacroValue::String(String::new()))
            }

            Some("gv-letplace") => {
                // (gv-letplace SYM PLACE BODY) — general place macro
                // Simplified: evaluate body with PLACE's value bound to SYM
                if items.len() >= 4 {
                    let sym_name = items[1].symbol_name();
                    let place_val = self.eval(&items[2], env)?;
                    if let Some(name) = sym_name {
                        env.bind(name.to_string(), place_val);
                        let result = self.eval(&items[3], env);
                        env.remove(name);
                        result
                    } else {
                        self.eval(&items[3], env)
                    }
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("pcase-let") => {
                // (pcase-let BINDINGS BODY...) — pattern binding
                // Simplified: treat as let*
                if items.len() >= 3 {
                    let bindings_form = &items[1];
                    let bindings = match &bindings_form.kind {
                        crate::surface::SurfaceKind::List(b) => b,
                        _ => {
                            self.error(span, "pcase-let expects bindings list");
                            return Err(());
                        }
                    };
                    let mut bound_names = Vec::new();
                    for binding in bindings {
                        match &binding.kind {
                            crate::surface::SurfaceKind::List(pair) if pair.len() == 2 => {
                                if let Some(name) = pair[0].symbol_name() {
                                    let val = self.eval(&pair[1], env)?;
                                    env.bind(name.to_string(), val);
                                    bound_names.push(name.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                    let result = self.eval_progn(&items[2..], env);
                    for name in bound_names {
                        env.remove(&name);
                    }
                    result
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("nreverse") => {
                self.eval_unary(span, "nreverse", &items[1..], env, |v| {
                    match v.to_vec() {
                        Some(mut vals) => {
                            vals.reverse();
                            MacroValue::list(vals)
                        }
                        None => MacroValue::Nil,
                    }
                })
            }

            Some("pcase") => {
                // (pcase EXP CLAUSES...) — pattern matching
                // Simplified: evaluate EXP, try each clause as (PATTERN BODY)
                // Only handle simple (SYMBOL BODY) and (`',VALUE BODY) patterns
                let exp_val = self.eval(&items[1], env)?;
                for clause in &items[2..] {
                    match &clause.kind {
                        crate::surface::SurfaceKind::List(parts) if parts.len() >= 2 => {
                            // Check if pattern matches
                            let pattern = &parts[0];
                            let matches = match &pattern.kind {
                                crate::surface::SurfaceKind::Atom(
                                    crate::surface::SurfaceAtom::Symbol(s)
                                ) if s == "_" => true,
                                crate::surface::SurfaceKind::Quote(q) => {
                                    let qval = surface_to_value(q);
                                    qval == exp_val
                                }
                                _ => true, // Best-effort: assume match
                            };
                            if matches {
                                return self.eval_progn(&parts[1..], env);
                            }
                        }
                        _ => continue,
                    }
                }
                Ok(MacroValue::Nil)
            }

            Some("push") => {
                // (push VAL PLACE) — macro expansion time, just return nil
                Ok(MacroValue::Nil)
            }

            Some("pop") => {
                // (pop PLACE) — macro expansion time, return car of place value
                if items.len() >= 2 {
                    let val = self.eval(&items[1], env)?;
                    Ok(val.car())
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            _ => {
                self.error(span, format!(
                    "cannot evaluate '{}' at macro expansion time",
                    head.unwrap_or("?")
                ));
                Err(())
            }
        }
    }

    // --- Special forms ---

    fn eval_if(
        &mut self,
        span: Span,
        items: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if items.len() < 3 {
            self.error(span, "if requires at least condition and then-branch");
            return Err(());
        }
        let condition = self.eval(&items[1], env)?;
        if condition.is_truthy() {
            self.eval(&items[2], env)
        } else {
            self.eval_progn(&items[3..], env)
        }
    }

    fn eval_cond(
        &mut self,
        span: Span,
        clauses: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        for clause in clauses {
            match &clause.kind {
                SurfaceKind::List(items) if !items.is_empty() => {
                    let test = self.eval(&items[0], env)?;
                    if test.is_truthy() {
                        if items.len() == 1 {
                            return Ok(test);
                        }
                        return self.eval_progn(&items[1..], env);
                    }
                }
                _ => {
                    self.error(span, "cond clause must be a non-empty list");
                    return Err(());
                }
            }
        }
        Ok(MacroValue::Nil)
    }

    fn eval_and(
        &mut self,
        forms: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        let mut result = MacroValue::Symbol("t".into());
        for form in forms {
            result = self.eval(form, env)?;
            if !result.is_truthy() {
                return Ok(MacroValue::Nil);
            }
        }
        Ok(result)
    }

    fn eval_or(
        &mut self,
        forms: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        for form in forms {
            let result = self.eval(form, env)?;
            if result.is_truthy() {
                return Ok(result);
            }
        }
        Ok(MacroValue::Nil)
    }

    fn eval_let(
        &mut self,
        span: Span,
        tail: &[SurfaceForm],
        env: &mut MacroEnv,
        sequential: bool,
    ) -> Result<MacroValue, ()> {
        if tail.is_empty() {
            self.error(span, "let requires bindings");
            return Err(());
        }
        let bindings_form = &tail[0];
        let body = &tail[1..];
        let bindings = self.parse_let_bindings(bindings_form)?;

        let saved: Vec<String> = bindings.iter().map(|(n, _)| n.clone()).collect();

        if sequential {
            for (name, val_form) in &bindings {
                let val = self.eval(val_form, env)?;
                env.bind(name.clone(), val);
            }
        } else {
            let mut values = Vec::new();
            for (_, val_form) in &bindings {
                values.push(self.eval(val_form, env)?);
            }
            for ((name, _), val) in bindings.iter().zip(values) {
                env.bind(name.clone(), val);
            }
        }

        let result = self.eval_progn(body, env);

        for name in &saved {
            env.remove(name);
        }

        result
    }

    fn parse_let_bindings(
        &mut self,
        form: &SurfaceForm,
    ) -> Result<Vec<(String, SurfaceForm)>, ()> {
        match &form.kind {
            SurfaceKind::List(items) => {
                let mut bindings = Vec::new();
                for item in items {
                    bindings.push(self.parse_let_binding(item)?);
                }
                Ok(bindings)
            }
            _ => {
                self.error(form.span, "let bindings must be a list");
                Err(())
            }
        }
    }

    fn parse_let_binding(
        &mut self,
        form: &SurfaceForm,
    ) -> Result<(String, SurfaceForm), ()> {
        match &form.kind {
            SurfaceKind::List(items) if items.len() >= 2 => {
                let name = items[0].symbol_name().ok_or_else(|| {
                    self.error(items[0].span, "binding name must be a symbol");
                })?;
                Ok((name.to_string(), items[1].clone()))
            }
            SurfaceKind::List(items) if items.len() == 1 => {
                let name = items[0].symbol_name().ok_or_else(|| {
                    self.error(items[0].span, "binding name must be a symbol");
                })?;
                Ok((name.to_string(), SurfaceForm::new(
                    SurfaceKind::Atom(SurfaceAtom::Nil),
                    form.span,
                )))
            }
            SurfaceKind::Atom(_) => {
                if let Some(name) = form.symbol_name() {
                    Ok((name.to_string(), SurfaceForm::new(
                        SurfaceKind::Atom(SurfaceAtom::Nil),
                        form.span,
                    )))
                } else {
                    self.error(form.span, "binding must be a symbol or (symbol value) list");
                    Err(())
                }
            }
            _ => {
                self.error(form.span, "binding must be a symbol or (symbol value) list");
                Err(())
            }
        }
    }

    fn eval_setq(
        &mut self,
        span: Span,
        pairs: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if pairs.len() % 2 != 0 {
            self.error(span, "setq requires pairs of variable and value");
            return Err(());
        }
        let mut result = MacroValue::Nil;
        let mut i = 0;
        while i + 1 < pairs.len() {
            let name = pairs[i].symbol_name().ok_or_else(|| {
                self.error(pairs[i].span, "setq variable must be a symbol");
            })?;
            let val = self.eval(&pairs[i + 1], env)?;
            env.bind(name.to_string(), val.clone());
            result = val;
            i += 2;
        }
        Ok(result)
    }

    fn eval_while(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if args.is_empty() {
            return Ok(MacroValue::Nil);
        }
        let mut iterations = 0;
        loop {
            let cond = self.eval(&args[0], env)?;
            if cond.is_nil() {
                return Ok(MacroValue::Nil);
            }
            for body_form in &args[1..] {
                self.eval(body_form, env)?;
            }
            iterations += 1;
            if iterations > 10000 {
                self.error(span, "while loop exceeded iteration limit");
                return Err(());
            }
        }
    }

    // --- Function call helpers ---

    fn eval_unary(
        &mut self,
        span: Span,
        name: &str,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
        f: impl Fn(MacroValue) -> MacroValue,
    ) -> Result<MacroValue, ()> {
        if args.len() != 1 {
            self.error(span, format!("{} requires exactly one argument", name));
            return Err(());
        }
        let val = self.eval(&args[0], env)?;
        Ok(f(val))
    }

    fn eval_binary(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
        f: impl Fn(MacroValue, MacroValue) -> MacroValue,
    ) -> Result<MacroValue, ()> {
        if args.len() != 2 {
            self.error(span, "requires exactly two arguments");
            return Err(());
        }
        let a = self.eval(&args[0], env)?;
        let b = self.eval(&args[1], env)?;
        Ok(f(a, b))
    }

    fn eval_binary_pred(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
        pred: impl Fn(&MacroValue, &MacroValue) -> bool,
    ) -> Result<MacroValue, ()> {
        if args.len() != 2 {
            self.error(span, "requires exactly two arguments");
            return Err(());
        }
        let a = self.eval(&args[0], env)?;
        let b = self.eval(&args[1], env)?;
        Ok(MacroValue::from_bool(pred(&a, &b)))
    }

    fn eval_append(
        &mut self,
        _span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if args.is_empty() {
            return Ok(MacroValue::Nil);
        }
        let mut lists: Vec<MacroValue> = Vec::new();
        for arg in &args[..args.len() - 1] {
            lists.push(self.eval(arg, env)?);
        }
        let last = self.eval(&args[args.len() - 1], env)?;
        let mut result = last;
        for list in lists.into_iter().rev() {
            let items = list.to_vec().unwrap_or_default();
            for item in items.into_iter().rev() {
                result = MacroValue::cons(item, result);
            }
        }
        Ok(result)
    }

    fn eval_nth(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if args.len() != 2 {
            self.error(span, "nth requires exactly two arguments");
            return Err(());
        }
        let n = self.eval(&args[0], env)?.as_int().unwrap_or(0);
        let list = self.eval(&args[1], env)?;
        let mut current = list;
        for _ in 0..n {
            current = current.cdr();
        }
        Ok(current.car())
    }

    fn eval_length(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if args.len() != 1 {
            self.error(span, "length requires exactly one argument");
            return Err(());
        }
        let val = self.eval(&args[0], env)?;
        let len = match &val {
            MacroValue::Nil => 0,
            MacroValue::Cons(_) => {
                let mut count = 0;
                let mut cur = &val;
                while let MacroValue::Cons(pair) = cur {
                    count += 1;
                    cur = &pair.cdr;
                }
                count
            }
            MacroValue::Vector(items) => items.len(),
            MacroValue::String(s) => s.len(),
            _ => 0,
        };
        Ok(MacroValue::Int(len as i64))
    }

    fn eval_fold(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
        init: i64,
        f: impl Fn(i64, i64) -> i64,
    ) -> Result<MacroValue, ()> {
        let mut result = init;
        for arg in args {
            let val = self.eval(arg, env)?;
            match val {
                MacroValue::Int(n) => result = f(result, n),
                _ => {
                    self.error(span, "arithmetic requires integer arguments");
                    return Err(());
                }
            }
        }
        Ok(MacroValue::Int(result))
    }

    fn eval_sub(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if args.is_empty() {
            return Ok(MacroValue::Int(0));
        }
        if args.len() == 1 {
            let val = self.eval(&args[0], env)?;
            match val {
                MacroValue::Int(n) => return Ok(MacroValue::Int(-n)),
                _ => {
                    self.error(span, "arithmetic requires integer arguments");
                    return Err(());
                }
            }
        }
        let first = self.eval(&args[0], env)?;
        let mut result = match first {
            MacroValue::Int(n) => n,
            _ => {
                self.error(span, "arithmetic requires integer arguments");
                return Err(());
            }
        };
        for arg in &args[1..] {
            let val = self.eval(arg, env)?;
            match val {
                MacroValue::Int(n) => result = result.wrapping_sub(n),
                _ => {
                    self.error(span, "arithmetic requires integer arguments");
                    return Err(());
                }
            }
        }
        Ok(MacroValue::Int(result))
    }

    fn eval_numeric_cmp(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
        pred: impl Fn(i64, i64) -> bool,
    ) -> Result<MacroValue, ()> {
        if args.len() < 2 {
            self.error(span, "comparison requires at least two arguments");
            return Err(());
        }
        let first = self.eval(&args[0], env)?;
        let first_int = match first {
            MacroValue::Int(n) => n,
            _ => {
                self.error(span, "comparison requires integer arguments");
                return Err(());
            }
        };
        for arg in &args[1..] {
            let val = self.eval(arg, env)?;
            match val {
                MacroValue::Int(n) => {
                    if !pred(first_int, n) {
                        return Ok(MacroValue::Nil);
                    }
                }
                _ => {
                    self.error(span, "comparison requires integer arguments");
                    return Err(());
                }
            }
        }
        Ok(MacroValue::Symbol("t".into()))
    }

    fn eval_concat(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        let mut result = String::new();
        for arg in args {
            let val = self.eval(arg, env)?;
            match val {
                MacroValue::String(s) => result.push_str(&s),
                MacroValue::Int(n) => result.push_str(&n.to_string()),
                MacroValue::Symbol(s) => result.push_str(&s),
                _ => {
                    self.error(span, "concat requires string arguments");
                    return Err(());
                }
            }
        }
        Ok(MacroValue::String(result))
    }

    fn eval_substring(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if args.len() < 2 || args.len() > 3 {
            self.error(span, "substring requires 2 or 3 arguments");
            return Err(());
        }
        let s = self.eval(&args[0], env)?;
        let from = self.eval(&args[1], env)?;
        let s_text = match &s {
            MacroValue::String(t) => t.clone(),
            _ => {
                self.error(span, "substring first argument must be a string");
                return Err(());
            }
        };
        let from_idx = match from {
            MacroValue::Int(n) => {
                if n < 0 {
                    (s_text.len() as i64 + n) as usize
                } else {
                    n as usize
                }
            }
            _ => {
                self.error(span, "substring index must be an integer");
                return Err(());
            }
        };
        let to_idx = if args.len() == 3 {
            let to = self.eval(&args[2], env)?;
            match to {
                MacroValue::Int(n) => {
                    if n < 0 {
                        (s_text.len() as i64 + n) as usize
                    } else {
                        n as usize
                    }
                }
                _ => {
                    self.error(span, "substring index must be an integer");
                    return Err(());
                }
            }
        } else {
            s_text.len()
        };
        Ok(MacroValue::String(s_text[from_idx..to_idx].into()))
    }

    fn eval_format(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if args.is_empty() {
            self.error(span, "format requires at least one argument");
            return Err(());
        }
        let fmt = self.eval(&args[0], env)?;
        let fmt_text = match &fmt {
            MacroValue::String(s) => s.clone(),
            _ => {
                self.error(span, "format first argument must be a string");
                return Err(());
            }
        };
        let mut format_args: Vec<MacroValue> = Vec::new();
        for arg in &args[1..] {
            format_args.push(self.eval(arg, env)?);
        }
        let mut result = String::new();
        let mut arg_idx = 0;
        let mut chars = fmt_text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '%' {
                match chars.next() {
                    Some('s') | Some('S') => {
                        if arg_idx < format_args.len() {
                            result.push_str(&format_value_as_string(&format_args[arg_idx]));
                            arg_idx += 1;
                        }
                    }
                    Some('d') => {
                        if arg_idx < format_args.len() {
                            result.push_str(&format_args[arg_idx].as_int().unwrap_or(0).to_string());
                            arg_idx += 1;
                        }
                    }
                    Some('%') => result.push('%'),
                    _ => result.push('%'),
                }
            } else {
                result.push(c);
            }
        }
        Ok(MacroValue::String(result))
    }

    // --- Quasiquote ---

    fn eval_quasiquote(
        &mut self,
        form: &SurfaceForm,
        env: &mut MacroEnv,
        depth: usize,
    ) -> Result<MacroValue, ()> {
        match &form.kind {
            SurfaceKind::Comma(inner) => {
                if depth == 1 {
                    self.eval(inner, env)
                } else {
                    Ok(MacroValue::list(vec![
                        MacroValue::Symbol("unquote".into()),
                        self.eval_quasiquote(inner, env, depth - 1)?,
                    ]))
                }
            }
            SurfaceKind::CommaAt(inner) => {
                if depth == 1 {
                    self.eval(inner, env)
                } else {
                    Ok(MacroValue::list(vec![
                        MacroValue::Symbol("splice-unquote".into()),
                        self.eval_quasiquote(inner, env, depth - 1)?,
                    ]))
                }
            }
            SurfaceKind::Backquote(inner) => {
                let inner_val = self.eval_quasiquote(inner, env, depth + 1)?;
                Ok(MacroValue::list(vec![
                    MacroValue::Symbol("backquote".into()),
                    inner_val,
                ]))
            }
            SurfaceKind::List(items) => {
                let expanded = self.quasiquote_items(items, env, depth)?;
                Ok(MacroValue::list(expanded))
            }
            SurfaceKind::DottedList(items, tail) => {
                let items_val = self.quasiquote_items(items, env, depth)?;
                let tail_val = self.eval_quasiquote(tail, env, depth)?;
                let mut result = tail_val;
                for item in items_val.into_iter().rev() {
                    result = MacroValue::cons(item, result);
                }
                Ok(result)
            }
            SurfaceKind::Vector(items) => {
                let mut result = Vec::new();
                for item in items {
                    if let SurfaceKind::CommaAt(inner) = &item.kind {
                        let spliced = self.eval(inner, env)?;
                        if let Some(vec) = spliced.to_vec() {
                            result.extend(vec);
                        }
                    } else {
                        result.push(self.eval_quasiquote(item, env, depth)?);
                    }
                }
                Ok(MacroValue::Vector(result))
            }
            _ => Ok(surface_to_value(form)),
        }
    }

    fn quasiquote_items(
        &mut self,
        items: &[SurfaceForm],
        env: &mut MacroEnv,
        depth: usize,
    ) -> Result<Vec<MacroValue>, ()> {
        let mut result = Vec::new();
        for item in items {
            if let SurfaceKind::CommaAt(inner) = &item.kind {
                let spliced = self.eval(inner, env)?;
                if let Some(vec) = spliced.to_vec() {
                    for val in vec {
                        result.push(val);
                    }
                }
            } else {
                result.push(self.eval_quasiquote(item, env, depth)?);
            }
        }
        Ok(result)
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message.into()).with_span(span));
    }
}

fn format_value_as_string(val: &MacroValue) -> String {
    match val {
        MacroValue::Nil => "nil".into(),
        MacroValue::Int(n) => n.to_string(),
        MacroValue::Symbol(s) => s.clone(),
        MacroValue::String(s) => format!("\"{}\"", s),
        MacroValue::Cons(_) => {
            let items = val.to_vec().unwrap_or_default();
            let parts: Vec<String> = items.iter().map(format_value_as_string).collect();
            format!("({})", parts.join(" "))
        }
        MacroValue::Vector(items) => {
            let parts: Vec<String> = items.iter().map(format_value_as_string).collect();
            format!("[{}]", parts.join(" "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceFile, SourceId};

    fn parse_and_eval(source: &str) -> Result<MacroValue, ()> {
        let src = SourceFile::new(SourceId::new(0), Some("test.el".into()), source.into());
        let output = crate::reader::read_source(&src);
        if !output.diagnostics.is_empty() {
            return Err(());
        }
        if output.forms.is_empty() {
            return Err(());
        }
        let form = &output.forms[0];
        let mut eval = MacroEval::new();
        let mut env = MacroEnv::default();
        eval.eval(form, &mut env)
    }

    fn eval_expr(source: &str) -> MacroValue {
        parse_and_eval(source).unwrap()
    }

    #[test]
    fn evals_self_evaluating_integer() {
        let v = eval_expr("42");
        assert_eq!(v, MacroValue::Int(42));
    }

    #[test]
    fn evals_self_evaluating_string() {
        let v = eval_expr("\"hello\"");
        assert_eq!(v, MacroValue::String("hello".into()));
    }

    #[test]
    fn evals_nil() {
        let v = eval_expr("nil");
        assert_eq!(v, MacroValue::Nil);
    }

    #[test]
    fn evals_if_then_branch() {
        let v = eval_expr("(if t 1 2)");
        assert_eq!(v, MacroValue::Int(1));
    }

    #[test]
    fn evals_if_else_branch() {
        let v = eval_expr("(if nil 1 2)");
        assert_eq!(v, MacroValue::Int(2));
    }

    #[test]
    fn evals_if_nil_else() {
        let v = eval_expr("(if nil 1)");
        assert_eq!(v, MacroValue::Nil);
    }

    #[test]
    fn evals_let_binding() {
        let v = eval_expr("(let ((x 10)) x)");
        assert_eq!(v, MacroValue::Int(10));
    }

    #[test]
    fn evals_let_star_sequential() {
        let v = eval_expr("(let* ((x 1) (y (+ x 1))) y)");
        assert_eq!(v, MacroValue::Int(2));
    }

    #[test]
    fn evals_setq() {
        let v = eval_expr("(let ((x 0)) (setq x 42) x)");
        assert_eq!(v, MacroValue::Int(42));
    }

    #[test]
    fn evals_cons_car_cdr() {
        let v = eval_expr("(car (cons 1 2))");
        assert_eq!(v, MacroValue::Int(1));
    }

    #[test]
    fn evals_list_and_nth() {
        let v = eval_expr("(nth 1 (list 10 20 30))");
        assert_eq!(v, MacroValue::Int(20));
    }

    #[test]
    fn evals_append() {
        let v = eval_expr("(append (list 1 2) (list 3 4))");
        let vec = v.to_vec().unwrap();
        assert_eq!(vec.len(), 4);
        assert_eq!(vec[0], MacroValue::Int(1));
        assert_eq!(vec[3], MacroValue::Int(4));
    }

    #[test]
    fn evals_null_predicate() {
        let v = eval_expr("(null nil)");
        assert!(v.is_truthy());
    }

    #[test]
    fn evals_consp_predicate() {
        let v = eval_expr("(consp (list 1))");
        assert!(v.is_truthy());
    }

    #[test]
    fn evals_eq() {
        let v = eval_expr("(eq 'foo 'foo)");
        assert!(v.is_truthy());
    }

    #[test]
    fn evals_arithmetic() {
        let v = eval_expr("(+ 1 (* 2 3))");
        assert_eq!(v, MacroValue::Int(7));
    }

    #[test]
    fn evals_comparison() {
        let v = eval_expr("(< 1 2)");
        assert!(v.is_truthy());
    }

    #[test]
    fn evals_and_short_circuit() {
        let v = eval_expr("(and 1 2 3)");
        assert_eq!(v, MacroValue::Int(3));
    }

    #[test]
    fn evals_or_short_circuit() {
        let v = eval_expr("(or nil nil 42)");
        assert_eq!(v, MacroValue::Int(42));
    }

    #[test]
    fn evals_quote() {
        let v = eval_expr("(car '(1 2 3))");
        assert_eq!(v, MacroValue::Int(1));
    }

    #[test]
    fn reports_unknown_function() {
        let result = parse_and_eval("(some-unknown-fn 1)");
        assert!(result.is_err());
    }

    #[test]
    fn evals_symbol_name() {
        let v = eval_expr("(symbol-name 'foo)");
        assert_eq!(v, MacroValue::String("foo".into()));
    }

    #[test]
    fn evals_length() {
        let v = eval_expr("(length (list 1 2 3))");
        assert_eq!(v, MacroValue::Int(3));
    }
}
