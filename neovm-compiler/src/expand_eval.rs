use std::collections::HashMap;
use std::rc::Rc;

use crate::diagnostic::Diagnostic;
use crate::expand_value::{MacroValue, surface_to_value};
use crate::source::Span;
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

#[derive(Clone, Debug)]
pub struct MacroFunction {
    pub params: Vec<String>,
    pub body: Vec<SurfaceForm>,
}

#[derive(Clone, Debug, Default)]
pub struct MacroEnv {
    bindings: HashMap<String, MacroValue>,
    functions: HashMap<String, MacroFunction>,
}

impl MacroEnv {
    pub fn bind(&mut self, name: String, value: MacroValue) {
        self.bindings.insert(name, value);
    }

    pub fn lookup(&self, name: &str) -> Option<&MacroValue> {
        self.bindings.get(name)
    }

    /// Save the current value of a binding, returning the old value (or None if unbound).
    pub fn save(&mut self, name: &str) -> Option<MacroValue> {
        self.bindings.get(name).cloned()
    }

    /// Restore a binding to a previously saved value, or remove it if it was unbound.
    pub fn restore(&mut self, name: String, saved: Option<MacroValue>) {
        match saved {
            Some(v) => {
                self.bindings.insert(name, v);
            }
            None => {
                self.bindings.remove(&name);
            }
        }
    }

    pub fn define_function(&mut self, name: String, func: MacroFunction) {
        self.functions.insert(name, func);
    }

    pub fn lookup_function(&self, name: &str) -> Option<&MacroFunction> {
        self.functions.get(name)
    }
}

pub struct MacroEval {
    diagnostics: Vec<Diagnostic>,
    depth: usize,
}

const MAX_EVAL_DEPTH: usize = 10;

impl MacroEval {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            depth: 0,
        }
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    fn inc_depth(&mut self) -> Result<(), ()> {
        self.depth += 1;
        if self.depth > MAX_EVAL_DEPTH {
            return Err(());
        }
        Ok(())
    }

    fn dec_depth(&mut self) {
        self.depth = self.depth.saturating_sub(1);
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

    pub fn eval(&mut self, form: &SurfaceForm, env: &mut MacroEnv) -> Result<MacroValue, ()> {
        self.inc_depth()?;
        let result = self.eval_inner(form, env);
        self.dec_depth();
        result
    }

    fn eval_inner(&mut self, form: &SurfaceForm, env: &mut MacroEnv) -> Result<MacroValue, ()> {
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
            SurfaceKind::DottedList(_, _)
            | SurfaceKind::Vector(_)
            | SurfaceKind::HashList(_)
            | SurfaceKind::Record(..)
            | SurfaceKind::CharTable(_)
            | SurfaceKind::Labeled(..)
            | SurfaceKind::Ref(_) => Ok(surface_to_value(form)),
        }
    }

    fn eval_atom(&mut self, atom: &SurfaceAtom, env: &mut MacroEnv) -> MacroValue {
        match atom {
            SurfaceAtom::Nil => MacroValue::Nil,
            SurfaceAtom::True => MacroValue::Symbol("t".into()),
            SurfaceAtom::Int(n) => MacroValue::Int(*n),
            SurfaceAtom::Float(f) => MacroValue::Float(*f),
            SurfaceAtom::Symbol(name) => env
                .lookup(name)
                .cloned()
                .unwrap_or(MacroValue::Symbol(name.clone())),
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

            // Thread primitives — passed through to the HIR compiler.
            Some("make-thread") | Some("thread-yield") | Some("thread-join")
            | Some("thread-signal") | Some("current-thread") | Some("thread-alive-p") => {
                let form = SurfaceForm::new(SurfaceKind::List(items.to_vec()), span);
                Ok(surface_to_value(&form))
            }

            // Atom and Agent primitives — passed through to HIR.
            Some("make-atom") | Some("atom-swap!") | Some("atom-reset!")
            | Some("atom-deref") | Some("atom-compare-and-set!")
            | Some("make-agent") | Some("send") | Some("send-off")
            | Some("agent-await") | Some("agent-deref") | Some("agent-error")
            | Some("restart-agent") => {
                let form = SurfaceForm::new(SurfaceKind::List(items.to_vec()), span);
                Ok(surface_to_value(&form))
            }

            // Mutex and condition variable primitives.
            Some("make-mutex") | Some("with-mutex") | Some("mutex-lock")
            | Some("mutex-unlock") | Some("make-condition-variable")
            | Some("condition-wait") | Some("condition-notify")
            | Some("condition-notify-all") => {
                let form = SurfaceForm::new(SurfaceKind::List(items.to_vec()), span);
                Ok(surface_to_value(&form))
            }

            Some("car") | Some("first") => {
                self.eval_unary(span, "car", &items[1..], env, |v| v.car())
            }
            Some("car-safe") => self.eval_unary(span, "car-safe", &items[1..], env, |v| {
                if v.is_cons() {
                    v.car()
                } else {
                    MacroValue::Nil
                }
            }),
            Some("cdr") | Some("rest") => {
                self.eval_unary(span, "cdr", &items[1..], env, |v| v.cdr())
            }
            Some("cdr-safe") => self.eval_unary(span, "cdr-safe", &items[1..], env, |v| {
                if v.is_cons() {
                    v.cdr()
                } else {
                    MacroValue::Nil
                }
            }),
            Some("cadr") => self.eval_unary(span, "cadr", &items[1..], env, |v| v.cdr().car()),
            Some("caddr") => {
                self.eval_unary(span, "caddr", &items[1..], env, |v| v.cdr().cdr().car())
            }
            Some("cddr") => self.eval_unary(span, "cddr", &items[1..], env, |v| v.cdr().cdr()),
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
            Some("elt") => {
                // (elt seq n) — nth element of sequence (list or vector)
                if items.len() >= 3 {
                    let seq = self.eval(&items[1], env)?;
                    let n = self.eval(&items[2], env)?.as_int().unwrap_or(0) as usize;
                    match &seq {
                        MacroValue::Vector(vec) => {
                            Ok(vec.get(n).cloned().unwrap_or(MacroValue::Nil))
                        }
                        _ => {
                            // List path
                            let mut current = seq;
                            for _ in 0..n {
                                current = current.cdr();
                                if current.is_nil() {
                                    return Ok(MacroValue::Nil);
                                }
                            }
                            Ok(current.car())
                        }
                    }
                } else {
                    Ok(MacroValue::Nil)
                }
            }
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
                MacroValue::from_bool(v.is_int() || matches!(v, MacroValue::Float(..)))
            }),
            Some("eq") => self.eval_binary_pred(span, &items[1..], env, |a, b| a.eq(b)),
            Some("eql") => self.eval_binary_pred(span, &items[1..], env, |a, b| a.eql(b)),
            Some("equal") => self.eval_binary_pred(span, &items[1..], env, |a, b| a.equal(b)),

            Some("+") => self.eval_arithmetic(
                span,
                &items[1..],
                env,
                0i64,
                0.0,
                |a, b| a.wrapping_add(b),
                |a, b| a + b,
            ),
            Some("-") => self.eval_sub(span, &items[1..], env),
            Some("*") => self.eval_arithmetic(
                span,
                &items[1..],
                env,
                1i64,
                1.0,
                |a, b| a.wrapping_mul(b),
                |a, b| a * b,
            ),
            Some("/") => self.eval_divide(span, &items[1..], env),
            Some("%") => self.eval_rem(span, &items[1..], env, "%"),
            Some("mod") => self.eval_rem(span, &items[1..], env, "mod"),

            Some("=") => {
                self.eval_numeric_cmp(span, &items[1..], env, |a, b| a == b, |a, b| a == b)
            }
            Some("/=") => self.eval_ne(span, &items[1..], env),
            Some("<") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a < b, |a, b| a < b),
            Some(">") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a > b, |a, b| a > b),
            Some("<=") => {
                self.eval_numeric_cmp(span, &items[1..], env, |a, b| a <= b, |a, b| a <= b)
            }
            Some(">=") => {
                self.eval_numeric_cmp(span, &items[1..], env, |a, b| a >= b, |a, b| a >= b)
            }

            Some("symbol-name") => self.eval_unary(span, "symbol-name", &items[1..], env, |v| {
                v.as_symbol_name()
                    .map(|s| MacroValue::String(s.into()))
                    .unwrap_or(MacroValue::Nil)
            }),
            Some("intern") => self.eval_unary(span, "intern", &items[1..], env, |v| match v {
                MacroValue::String(s) => MacroValue::Symbol(s.clone()),
                other => other.clone(),
            }),
            Some("make-symbol") => {
                self.eval_unary(span, "make-symbol", &items[1..], env, |v| match v {
                    MacroValue::String(s) => MacroValue::Symbol(format!(" {}", s)),
                    other => other.clone(),
                })
            }
            Some("downcase") => self.eval_unary(span, "downcase", &items[1..], env, |v| match v {
                MacroValue::String(s) => MacroValue::String(s.to_lowercase()),
                MacroValue::Symbol(s) => MacroValue::Symbol(s.to_lowercase()),
                other => other.clone(),
            }),
            Some("upcase") => self.eval_unary(span, "upcase", &items[1..], env, |v| match v {
                MacroValue::String(s) => MacroValue::String(s.to_uppercase()),
                MacroValue::Symbol(s) => MacroValue::Symbol(s.to_uppercase()),
                other => other.clone(),
            }),
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
                // Evaluate body, return last value; swallow errors
                let mut result = MacroValue::Nil;
                for form in &items[1..] {
                    match self.eval(form, env) {
                        Ok(v) => result = v,
                        Err(_) => return Ok(MacroValue::Nil),
                    }
                }
                Ok(result)
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
                            let saved = env.save(name);
                            env.bind(name.to_string(), sym_val);
                            let result = self.eval(&items[3], env);
                            env.restore(name.to_string(), saved);
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

            Some("keywordp") => self.eval_unary(span, "keywordp", &items[1..], env, |v| match v {
                MacroValue::Symbol(s) => MacroValue::from_bool(s.starts_with(':')),
                _ => MacroValue::from_bool(false),
            }),

            Some("evenp") => self.eval_unary(span, "evenp", &items[1..], env, |v| match v {
                MacroValue::Int(n) => MacroValue::from_bool(n % 2 == 0),
                _ => MacroValue::from_bool(false),
            }),

            Some("zerop") => self.eval_unary(span, "zerop", &items[1..], env, |v| match v {
                MacroValue::Int(n) => MacroValue::from_bool(n == 0),
                _ => MacroValue::from_bool(false),
            }),

            Some("capitalize") => {
                self.eval_unary(span, "capitalize", &items[1..], env, |v| match v {
                    MacroValue::String(s) => {
                        let cap: String = s
                            .chars()
                            .enumerate()
                            .map(|(i, c)| {
                                if i == 0 {
                                    c.to_uppercase().to_string()
                                } else {
                                    c.to_lowercase().to_string()
                                }
                            })
                            .collect();
                        MacroValue::String(cap)
                    }
                    other => other.clone(),
                })
            }

            Some("macroexp--fgrep") => {
                // (macroexp--fgrep BINDINGS FORM) — check if form references any binding
                // Returns the subset of BINDINGS referenced in FORM, or nil.
                // Simplified: check if any binding name appears in the form,
                // return the bindings list if so, nil otherwise.
                if items.len() >= 3 {
                    if let Ok(binding_vals) = self.eval(&items[1], env) {
                        if let Some(binding_pairs) = binding_vals.to_vec() {
                            let binding_names: Vec<String> = binding_pairs
                                .iter()
                                .filter_map(|pair| {
                                    let car = pair.car();
                                    if let MacroValue::Symbol(s) = car {
                                        Some(s)
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            if form_references_bindings(&items[2], &binding_names) {
                                return Ok(binding_vals);
                            }
                        }
                    }
                }
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
                        let saved = env.save(name);
                        env.bind(name.to_string(), place_val);
                        let result = self.eval(&items[3], env);
                        env.restore(name.to_string(), saved);
                        result
                    } else {
                        self.eval(&items[3], env)
                    }
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("pcase-let") | Some("pcase-let*") => {
                // (pcase-let BINDINGS BODY...) — pattern binding
                // Simplified: treat as let*
                if items.len() >= 3 {
                    let bindings_form = &items[1];
                    let bindings = match &bindings_form.kind {
                        crate::surface::SurfaceKind::List(b) => b,
                        _ => {
                            // Non-list bindings — skip
                            return self.eval_progn(&items[2..], env);
                        }
                    };
                    // First pass: evaluate all values and collect names
                    let mut bindings_to_apply: Vec<(String, MacroValue)> = Vec::new();
                    for binding in bindings {
                        match &binding.kind {
                            crate::surface::SurfaceKind::List(pair) if pair.len() == 2 => {
                                if let Some(name) = pair[0].symbol_name() {
                                    let val = self.eval(&pair[1], env)?;
                                    bindings_to_apply.push((name.to_string(), val));
                                }
                            }
                            _ => {}
                        }
                    }
                    // Save old values BEFORE binding any new ones
                    let saved: Vec<(String, Option<MacroValue>)> = bindings_to_apply
                        .iter()
                        .map(|(n, _)| (n.clone(), env.save(n)))
                        .collect();
                    // Now bind all new values
                    for (name, val) in &bindings_to_apply {
                        env.bind(name.clone(), val.clone());
                    }
                    let result = self.eval_progn(&items[2..], env);
                    for (name, old) in saved {
                        env.restore(name, old);
                    }
                    result
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("nreverse") => {
                self.eval_unary(span, "nreverse", &items[1..], env, |v| match v.to_vec() {
                    Some(mut vals) => {
                        vals.reverse();
                        MacroValue::list(vals)
                    }
                    None => MacroValue::Nil,
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
                                    crate::surface::SurfaceAtom::Symbol(s),
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
                // (push VAL PLACE) — prepend VAL to PLACE list
                if items.len() >= 3 {
                    let val = self.eval(&items[1], env)?;
                    if let Some(place_name) = items[2].symbol_name() {
                        let current = env.lookup(place_name).cloned().unwrap_or(MacroValue::Nil);
                        let new_list = MacroValue::cons(val, current);
                        env.bind(place_name.to_string(), new_list.clone());
                        Ok(new_list)
                    } else {
                        // Non-symbol place — just return val
                        Ok(val)
                    }
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("pop") => {
                // (pop PLACE) — remove first element, return it, update PLACE
                if items.len() >= 2 {
                    if let Some(place_name) = items[1].symbol_name() {
                        let current = env.lookup(place_name).cloned().unwrap_or(MacroValue::Nil);
                        let first = current.car();
                        let rest = current.cdr();
                        env.bind(place_name.to_string(), rest);
                        Ok(first)
                    } else {
                        // Non-symbol place — just evaluate and return car
                        let val = self.eval(&items[1], env)?;
                        Ok(val.car())
                    }
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("assq") => {
                // (assq key alist) — find first pair whose car is eq to key
                if items.len() >= 3 {
                    let key = self.eval(&items[1], env)?;
                    let alist = self.eval(&items[2], env)?;
                    Ok(alist.assq(&key))
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("get") => {
                // (get symbol prop) — get property from symbol's plist
                // At macro time we don't have a real plist, return nil
                Ok(MacroValue::Nil)
            }

            Some("intern-soft") => {
                // (intern-soft name) — look up existing symbol, return nil if not found
                // At macro time we return the symbol if it's a string arg
                if items.len() >= 2 {
                    let name = self.eval(&items[1], env)?;
                    match &name {
                        MacroValue::String(s) => Ok(MacroValue::Symbol(s.clone())),
                        MacroValue::Symbol(s) => Ok(MacroValue::Symbol(s.clone())),
                        _ => Ok(MacroValue::Nil),
                    }
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("fboundp") | Some("boundp") | Some("facep") => {
                // Runtime predicates — at macro time, return nil (unknown)
                Ok(MacroValue::Nil)
            }

            Some("butlast") => {
                // (butlast list &optional n) — return list without last n elements
                if items.len() >= 2 {
                    let list = self.eval(&items[1], env)?;
                    let n = if items.len() >= 3 {
                        self.eval(&items[2], env)?.as_int().unwrap_or(1)
                    } else {
                        1
                    };
                    Ok(list.butlast(n as usize))
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("delq") => {
                // (delq element list) — delete elements by eq
                // Returns a new list (we don't mutate at macro time)
                if items.len() >= 3 {
                    let el = self.eval(&items[1], env)?;
                    let list = self.eval(&items[2], env)?;
                    Ok(list.delq(&el))
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("prog1") => {
                // (prog1 first &rest body) — evaluate all, return first value
                if items.is_empty() {
                    Ok(MacroValue::Nil)
                } else {
                    let first = self.eval(&items[1], env)?;
                    for form in &items[2..] {
                        self.eval(form, env)?;
                    }
                    Ok(first)
                }
            }

            Some("mapcar") => {
                // (mapcar function sequence) — map over list
                if items.len() >= 3 {
                    let func_val = self.eval(&items[1], env)?;
                    let seq = self.eval(&items[2], env)?;
                    if seq.is_nil() {
                        Ok(MacroValue::Nil)
                    } else {
                        let list_items = seq.to_vec().unwrap_or_default();
                        let mut results = Vec::new();
                        for item in &list_items {
                            results.push(self.call_function(
                                span,
                                &func_val,
                                &[item.clone()],
                                env,
                            )?);
                        }
                        Ok(MacroValue::list(results))
                    }
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("memq") => {
                // (memq element list) — find element in list by eq
                if items.len() >= 3 {
                    let el = self.eval(&items[1], env)?;
                    let list = self.eval(&items[2], env)?;
                    Ok(list.memq(&el))
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("eval") => {
                // (eval form) — evaluate a form
                // At macro time, evaluate the argument as a form
                if items.len() >= 2 {
                    let form = self.eval(&items[1], env)?;
                    // The result should be a form to evaluate — but we'd need
                    // to convert MacroValue back to SurfaceForm. For now, just
                    // return the value as-is.
                    Ok(form)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("setcdr") => {
                // (setcdr cell new-cdr) — mutate cdr of a cons cell
                // At macro time, we don't mutate — return new-cdr
                if items.len() >= 3 {
                    self.eval(&items[2], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("put") => {
                // (put symbol prop value) — set symbol property
                // At macro time, return the value
                if items.len() >= 4 {
                    self.eval(&items[3], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("string-match") => {
                // (string-match regexp string &optional start)
                // Conservative default: return nil (no match)
                Ok(MacroValue::Nil)
            }

            Some("plist-get") => {
                // (plist-get plist prop) — get value from property list
                // plist is (prop1 val1 prop2 val2 ...)
                if items.len() >= 3 {
                    let prop = self.eval(&items[2], env)?;
                    let plist = self.eval(&items[1], env)?;
                    Ok(plist.plist_get(&prop))
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("last") => {
                // (last list &optional n) — return last n elements (default 1)
                if items.len() >= 2 {
                    let list = self.eval(&items[1], env)?;
                    let n = if items.len() >= 3 {
                        self.eval(&items[2], env)?.as_int().unwrap_or(1) as usize
                    } else {
                        1
                    };
                    Ok(list.last(n))
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("copy-sequence") | Some("cl-copy-list") => {
                // (copy-sequence seq) — shallow copy
                // Our MacroValues are Rc-based, so clone is already a shallow copy
                if items.len() >= 2 {
                    self.eval(&items[1], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("remove") => {
                // (remove element list) — remove by equal (not eq)
                if items.len() >= 3 {
                    let el = self.eval(&items[1], env)?;
                    let list = self.eval(&items[2], env)?;
                    Ok(list.remove(&el))
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("make-vector") => {
                // (make-vector length initial-value) — create a vector
                // Return a vector of the specified length
                if items.len() >= 2 {
                    let len = self.eval(&items[1], env)?.as_int().unwrap_or(0);
                    let init = if items.len() >= 3 {
                        self.eval(&items[2], env)?
                    } else {
                        MacroValue::Nil
                    };
                    Ok(MacroValue::Vector(Rc::new(vec![init; len.max(0) as usize])))
                } else {
                    Ok(MacroValue::Vector(Rc::new(Vec::new())))
                }
            }

            Some("vector") => {
                // (vector &rest args) — create vector from args
                let mut results = Vec::new();
                for arg in &items[1..] {
                    results.push(self.eval(arg, env)?);
                }
                Ok(MacroValue::Vector(Rc::new(results)))
            }

            Some("vconcat") => {
                // (vconcat &rest sequences) — concatenate into vector
                let mut all = Vec::new();
                for arg in &items[1..] {
                    let val = self.eval(arg, env)?;
                    match val {
                        MacroValue::Vector(vec) => all.extend(vec.iter().cloned()),
                        MacroValue::Nil => {}
                        other => {
                            if let Some(vec) = other.to_vec() {
                                all.extend(vec);
                            }
                        }
                    }
                }
                Ok(MacroValue::Vector(Rc::new(all)))
            }

            Some("plist-put") => {
                // (plist-put plist prop val) — set property in plist
                // At macro time, return the modified plist
                if items.len() >= 4 {
                    let plist = self.eval(&items[1], env)?;
                    let prop = self.eval(&items[2], env)?;
                    let val = self.eval(&items[3], env)?;
                    Ok(plist.plist_put(&prop, val))
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("reverse") => {
                // (reverse list) — reverse a list
                if items.len() >= 2 {
                    let list = self.eval(&items[1], env)?;
                    Ok(list.reverse())
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("aset") => {
                // (aset array idx newelt) — set element in array/vector
                // At macro time, return the new value
                if items.len() >= 4 {
                    self.eval(&items[3], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("lambda") => {
                // (lambda (args) body...) — store as a callable closure value
                if items.len() < 3 {
                    return Ok(MacroValue::Nil);
                }
                let params = parse_lambda_params(&items[1]);
                let body: Vec<SurfaceForm> = items[2..].to_vec();
                // Store as a list (lambda (params...) body...) so it can be passed around
                let mut parts = vec![
                    MacroValue::Symbol("lambda".into()),
                    MacroValue::list(
                        params
                            .iter()
                            .map(|s| MacroValue::Symbol(s.clone()))
                            .collect(),
                    ),
                ];
                for b in &body {
                    parts.push(surface_to_value(b));
                }
                Ok(MacroValue::list(parts))
            }

            Some("funcall") => {
                // (funcall function &rest args) — call a function
                if items.len() < 2 {
                    return Ok(MacroValue::Nil);
                }
                let func_val = self.eval(&items[1], env)?;
                let args: Vec<MacroValue> = items[2..]
                    .iter()
                    .map(|a| self.eval(a, env))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call_function(span, &func_val, &args, env)
            }

            Some("aref") => {
                // (aref array idx) — get element from array/vector
                if items.len() >= 3 {
                    let array = self.eval(&items[1], env)?;
                    let idx = self.eval(&items[2], env)?;
                    match (&array, idx.as_int()) {
                        (MacroValue::Vector(vec), Some(i)) => {
                            let i = i as usize;
                            if i < vec.len() {
                                Ok(vec[i].clone())
                            } else {
                                Ok(MacroValue::Nil)
                            }
                        }
                        (MacroValue::Cons(pair), Some(i)) => {
                            // List aref: nth element
                            let mut current = MacroValue::Cons(pair.clone());
                            for _ in 0..i {
                                match current.cdr() {
                                    cdr if cdr.is_nil() => return Ok(MacroValue::Nil),
                                    cdr => current = cdr,
                                }
                            }
                            Ok(current.car())
                        }
                        _ => Ok(MacroValue::Nil),
                    }
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("match-string") => {
                // (match-string num &optional string) — return matched substring
                Ok(MacroValue::Nil)
            }

            Some("replace-match") => {
                // (replace-match newtext ...) — replace matched text
                if items.len() >= 2 {
                    self.eval(&items[1], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("apply-partially") => {
                // (apply-partially fun &rest args) — partial application
                Ok(MacroValue::Nil)
            }

            Some("cl--generic-predicate") => {
                // Generic function predicate — return nil at macro time
                Ok(MacroValue::Nil)
            }

            Some("macroexp-parse-binding") => {
                // Helper for macro expansion — return nil
                Ok(MacroValue::Nil)
            }

            Some("cl-generic--method-qualifier-p") => {
                // Check if arg is a method qualifier (not a list)
                if items.len() >= 2 {
                    let val = self.eval(&items[1], env)?;
                    Ok(MacroValue::from_bool(!val.is_cons()))
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("advice--normalize-place") => {
                // Normalize advice place — just return the place symbol
                if items.len() >= 2 {
                    self.eval(&items[1], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("cl--find-class") => {
                // EIEIO class lookup — return nil (no class info at macro time)
                Ok(MacroValue::Nil)
            }

            Some("byte-run--parse-body") => {
                // Parse function body for declarations — return empty list
                // Real impl returns (declarations interactive-form docstring rest-body)
                Ok(MacroValue::list(vec![
                    MacroValue::Nil, // declarations
                    MacroValue::Nil, // interactive-form
                    MacroValue::Nil, // docstring
                    MacroValue::Nil, // rest-body
                    MacroValue::Nil, // define-widget
                ]))
            }

            Some("rx--to-expr") => {
                // RX pattern translation — return nil
                Ok(MacroValue::Nil)
            }

            Some("letrec") => {
                // letrec: evaluate bindings sequentially with mutual recursion
                // Simplified: evaluate body with all bindings set to nil
                if items.len() <= 2 {
                    return Ok(MacroValue::Nil);
                }
                let mut result = MacroValue::Nil;
                for form in &items[2..] {
                    result = self.eval(form, env)?;
                }
                Ok(result)
            }

            Some("cl-with-gensyms") => {
                // (cl-with-gensyms (names...) body...) -> evaluate body
                if items.len() >= 3 {
                    self.eval_progn(&items[2..], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("cl-check-type") => {
                // (cl-check-type form type) -> evaluate form
                if items.len() >= 2 {
                    self.eval(&items[1], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("cl-assert") => {
                // (cl-assert form) -> evaluate form
                if items.len() >= 2 {
                    self.eval(&items[1], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("declare-function") => {
                // Compile-time declaration — discard
                Ok(MacroValue::Nil)
            }

            Some("condition-case") | Some("condition-case-unless-debug") => {
                // (condition-case var body-form (condition body)...) — try body,
                // catch errors, and match handler conditions.  Simplified: we try
                // the body and fall through to the first handler on any error.
                if items.len() < 3 {
                    return Ok(MacroValue::Nil);
                }
                let var_sym = items[1].symbol_name().unwrap_or_default();
                let body_form = &items[2];
                let handlers = &items[3..];
                match self.eval(body_form, env) {
                    Ok(value) => Ok(value),
                    Err(_) => {
                        // Body signalled an error — try each handler pair.
                        for chunk in handlers.chunks(2) {
                            if chunk.len() < 2 { break; }
                            // Evaluate handler body (skip condition check —
                            // in macro expansion any error triggers the first handler).
                            let handler_body = &chunk[1];
                            let mut handler_env = env.clone();
                            if !var_sym.is_empty() {
                                handler_env.bind(var_sym.to_string(), MacroValue::Nil);
                            }
                            if let Ok(val) = self.eval(handler_body, &mut handler_env) {
                                return Ok(val);
                            }
                        }
                        Ok(MacroValue::Nil)
                    }
                }
            }

            Some("oclosure--class-slots") => {
                // EIEIO slot access — return nil
                Ok(MacroValue::Nil)
            }

            Some("eieio--class-option-assoc") => {
                // EIEIO class option — return nil
                Ok(MacroValue::Nil)
            }

            Some("nconc") => {
                // (nconc &rest lists) — concatenate lists by appending
                let mut lists = Vec::new();
                for item in &items[1..] {
                    let val = self.eval(item, env)?;
                    lists.push(val);
                }
                Ok(append_values(&lists))
            }

            Some("cl-pushnew") => {
                // (cl-pushnew ITEM PLACE [KEYWORD ARGS]) — push if not already present
                // Simplified: just return nil (don't modify place at macro time)
                Ok(MacroValue::Nil)
            }

            Some("member") => {
                // (member ELT LIST) — find ELT in LIST using equal
                if items.len() >= 3 {
                    let elt = self.eval(&items[1], env)?;
                    let list = self.eval(&items[2], env)?;
                    Ok(list.member(&elt))
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("macroexp-const-p") => {
                // (macroexp-const-p FORM) — is form a constant?
                // Return nil (assume not constant at macro time)
                Ok(MacroValue::Nil)
            }

            // EIEIO, oclosure, and other object system functions
            // that return nil at macro expansion time
            Some("eieio--eval-default-p") => Ok(MacroValue::Nil),
            Some("eieio--class-children") => Ok(MacroValue::Nil),
            Some("eieio-class-parents") => Ok(MacroValue::Nil),
            Some("eieio--class-precedence-list") => Ok(MacroValue::Nil),
            Some("eieio--slot-name-at-point") => Ok(MacroValue::Nil),
            Some("cl--struct-slot-offset") => Ok(MacroValue::Nil),
            Some("cl--struct-slot-value") => Ok(MacroValue::Nil),
            Some("cl--struct-slot-mutable") => Ok(MacroValue::Nil),
            Some("oclosure--slot-names") => Ok(MacroValue::Nil),
            Some("oclosure--type-definitions") => Ok(MacroValue::Nil),
            Some("gv-expander") => Ok(MacroValue::Nil),
            Some("gv-get") => Ok(MacroValue::Nil),
            Some("gv-set") => Ok(MacroValue::Nil),
            Some("macroexp--expand-all") => Ok(MacroValue::Nil),
            Some("macroexp--accumulate-vars") => Ok(MacroValue::Nil),
            Some("macroexp-unwrap-cookie") => Ok(MacroValue::Nil),
            Some("cl--defsubst-expander") => Ok(MacroValue::Nil),
            Some("internal--format-docstring-line") => Ok(MacroValue::Nil),
            Some("internal--format-docstring") => Ok(MacroValue::Nil),
            Some("remq") => Ok(MacroValue::Nil),
            Some("cl-flet") => Ok(MacroValue::Nil),
            Some("c--mapcan") => Ok(MacroValue::Nil),

            Some("signal") | Some("user-error") => {
                // Error signaling at macro time — return nil to allow expansion to continue
                Ok(MacroValue::Nil)
            }

            Some("cl--transform-lambda") => {
                // CL lambda transformer — return nil (simplified)
                Ok(MacroValue::Nil)
            }

            Some("oclosure--defstruct-make-copiers") => {
                // EIEIO copier generation — return nil
                Ok(MacroValue::Nil)
            }

            Some("pcase-dolist") => {
                // Pattern matching dolist — evaluate body simply
                if items.len() >= 3 {
                    self.eval_progn(&items[2..], env)
                } else {
                    Ok(MacroValue::Nil)
                }
            }

            Some("cl-loop") => self.eval_cl_loop(span, &items[1..], env),

            _ => {
                // Pass through: unknown function calls are handled by the
                // compiler at HIR level, not the macro expander.
                let form = SurfaceForm::new(SurfaceKind::List(items.to_vec()), span);
                Ok(surface_to_value(&form))
            }
        }
    }

    // --- cl-loop evaluator for macro expansion ---

    fn eval_cl_loop(
        &mut self,
        span: Span,
        items: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        // Minimal cl-loop evaluator supporting common patterns:
        // for VAR in LIST, for VAR on LIST by FN, for (DESTRUCTURE) in/on LIST
        // collect EXPR, sum EXPR, count EXPR, if/when, do, finally return
        let mut pos = 0;
        let mut for_var: Option<String> = None;
        let mut for_destructure: Option<SurfaceForm> = None;
        let mut for_list: Option<SurfaceForm> = None;
        let mut for_on = false;
        let mut step_fn: Option<SurfaceForm> = None;
        let mut collect_exprs: Vec<SurfaceForm> = Vec::new();
        let mut sum_exprs: Vec<SurfaceForm> = Vec::new();
        let mut sum_vars: Vec<Option<String>> = Vec::new();
        let mut count_exprs: Vec<SurfaceForm> = Vec::new();
        let mut count_vars: Vec<Option<String>> = Vec::new();
        let mut do_body: Vec<SurfaceForm> = Vec::new();
        let mut while_conds: Vec<SurfaceForm> = Vec::new();
        let mut until_conds: Vec<SurfaceForm> = Vec::new();
        let mut finally_return: Option<SurfaceForm> = None;
        let mut default_into: Option<String> = None;

        while pos < items.len() {
            match items[pos].symbol_name() {
                Some("for") => {
                    pos += 1;
                    // Check for destructuring pattern
                    if let Some(name) = items[pos].symbol_name() {
                        for_var = Some(name.to_string());
                        for_destructure = None;
                    } else {
                        // Destructuring pattern like (key . val)
                        let pattern = items[pos].clone();
                        for_var = Some(format!("--cl-dst-{}--", pos));
                        for_destructure = Some(pattern);
                    }
                    pos += 1;
                    if pos < items.len() && items[pos].symbol_name() == Some("in") {
                        pos += 1;
                        for_list = Some(items[pos].clone());
                        pos += 1;
                        for_on = false;
                    } else if pos < items.len() && items[pos].symbol_name() == Some("on") {
                        pos += 1;
                        for_list = Some(items[pos].clone());
                        pos += 1;
                        for_on = true;
                    }
                    if pos < items.len() && items[pos].symbol_name() == Some("by") {
                        pos += 1;
                        step_fn = Some(items[pos].clone());
                        pos += 1;
                    }
                    // Handle for-equals: for x = expr [then step]
                    if pos > 0 && pos < items.len() && items[pos].symbol_name() == Some("=") {
                        // Already handled in/on above, skip
                    }
                }
                Some("from") | Some("upto") | Some("to") | Some("below") => {
                    pos += 1;
                    if pos < items.len() {
                        pos += 1;
                    }
                }
                Some("collect") => {
                    pos += 1;
                    if pos < items.len() {
                        collect_exprs.push(items[pos].clone());
                        pos += 1;
                        // Check for into
                        if pos < items.len() && items[pos].symbol_name() == Some("into") {
                            pos += 1;
                            // named accumulator — skip for now
                            if pos < items.len() {
                                pos += 1;
                            }
                        }
                    }
                }
                Some("sum") => {
                    pos += 1;
                    if pos < items.len() {
                        sum_exprs.push(items[pos].clone());
                        pos += 1;
                        if pos < items.len() && items[pos].symbol_name() == Some("into") {
                            pos += 1;
                            let name = if pos < items.len() {
                                let n = items[pos].symbol_name().map(|s| s.to_string());
                                pos += 1;
                                n
                            } else {
                                None
                            };
                            sum_vars.push(name);
                        } else {
                            sum_vars.push(None);
                            if default_into.is_none() {
                                default_into = Some("--cl-sum--".to_string());
                            }
                        }
                    }
                }
                Some("count") => {
                    pos += 1;
                    if pos < items.len() {
                        count_exprs.push(items[pos].clone());
                        pos += 1;
                        if pos < items.len() && items[pos].symbol_name() == Some("into") {
                            pos += 1;
                            let name = if pos < items.len() {
                                let n = items[pos].symbol_name().map(|s| s.to_string());
                                pos += 1;
                                n
                            } else {
                                None
                            };
                            count_vars.push(name);
                        } else {
                            count_vars.push(None);
                        }
                    }
                }
                Some("while") => {
                    pos += 1;
                    if pos < items.len() {
                        while_conds.push(items[pos].clone());
                        pos += 1;
                    }
                }
                Some("until") => {
                    pos += 1;
                    if pos < items.len() {
                        until_conds.push(items[pos].clone());
                        pos += 1;
                    }
                }
                Some("do") => {
                    pos += 1;
                    while pos < items.len() {
                        let kw = items[pos].symbol_name().unwrap_or("");
                        if matches!(
                            kw,
                            "collect"
                                | "sum"
                                | "count"
                                | "do"
                                | "finally"
                                | "while"
                                | "until"
                                | "if"
                                | "when"
                                | "return"
                                | "for"
                                | "with"
                                | "append"
                                | "nconc"
                                | "minimize"
                                | "maximize"
                                | "always"
                                | "never"
                                | "thereis"
                                | "initially"
                                | "repeat"
                        ) {
                            break;
                        }
                        do_body.push(items[pos].clone());
                        pos += 1;
                    }
                }
                Some("finally") => {
                    pos += 1;
                    // Handle: finally return EXPR, or finally (return EXPR)
                    if pos < items.len() {
                        if items[pos].symbol_name() == Some("return") {
                            pos += 1;
                            if pos < items.len() {
                                finally_return = Some(items[pos].clone());
                                pos += 1;
                            }
                        } else if let SurfaceKind::List(inner) = &items[pos].kind {
                            if inner.first().and_then(|i| i.symbol_name()) == Some("return") {
                                if inner.len() > 1 {
                                    finally_return = Some(inner[1].clone());
                                }
                                pos += 1;
                            } else {
                                // skip finally body
                                pos += 1;
                            }
                        } else {
                            pos += 1;
                        }
                    }
                }
                Some("return") => {
                    pos += 1;
                    if pos < items.len() {
                        finally_return = Some(items[pos].clone());
                        pos += 1;
                    }
                }
                _ => {
                    pos += 1;
                }
            }
        }

        // Execute the loop
        let Some(list_expr) = for_list else {
            // No iteration — evaluate body once
            if let Some(ret) = &finally_return {
                return self.eval(ret, env);
            }
            return Ok(MacroValue::Nil);
        };

        let list_val = self.eval(&list_expr, env)?;
        let step_closure = |v: &MacroValue| -> MacroValue {
            if let Some(ref step) = step_fn {
                // Only support #'cddr and #'cdr for now
                if let SurfaceKind::FunctionQuote(inner) = &step.kind {
                    if inner.symbol_name() == Some("cddr") {
                        return v.cdr().cdr();
                    } else if inner.symbol_name() == Some("cdr") {
                        return v.cdr();
                    }
                }
            }
            v.cdr()
        };

        let var_name = for_var.as_deref().unwrap_or("--cl-it--");
        let mut current = list_val;
        let mut results: Vec<MacroValue> = Vec::new();
        let mut sum_result: i64 = 0;
        let mut named_sums: HashMap<String, i64> = HashMap::new();
        let mut count_result: i64 = 0;
        let mut named_counts: HashMap<String, i64> = HashMap::new();

        while current.is_truthy() {
            // Bind iteration variable
            let val = if for_on {
                current.clone()
            } else {
                current.car()
            };
            env.bind(var_name.to_string(), val.clone());

            // Destructuring
            if let Some(ref pattern) = for_destructure {
                self.bind_destructure(pattern, &val, env);
            }

            // while/until conditions: break if any while is nil or any until is truthy
            let mut should_break = false;
            for cond in &while_conds {
                if !self.eval(cond, env)?.is_truthy() { should_break = true; break; }
            }
            if !should_break {
                for cond in &until_conds {
                    if self.eval(cond, env)?.is_truthy() { should_break = true; break; }
                }
            }
            if should_break { break; }

            // Evaluate body clauses
            for expr in &collect_exprs {
                let v = self.eval(expr, env)?;
                results.push(v);
            }
            for (i, expr) in sum_exprs.iter().enumerate() {
                let v = self.eval(expr, env)?;
                let n = v.as_int().unwrap_or(0);
                if let Some(ref name) = sum_vars[i] {
                    *named_sums.entry(name.clone()).or_insert(0) += n;
                } else {
                    sum_result += n;
                }
            }
            for (i, expr) in count_exprs.iter().enumerate() {
                let v = self.eval(expr, env)?;
                if v.is_truthy() {
                    if let Some(ref name) = count_vars[i] {
                        *named_counts.entry(name.clone()).or_insert(0) += 1;
                    } else {
                        count_result += 1;
                    }
                }
            }
            for expr in &do_body {
                let _ = self.eval(expr, env);
            }

            // Advance
            current = step_closure(&current);
        }

        // Handle finally return
        if let Some(ret_expr) = &finally_return {
            return self.eval(ret_expr, env);
        }

        // Return default accumulator
        if !collect_exprs.is_empty() {
            // nreverse results
            results.reverse();
            Ok(MacroValue::list(results))
        } else if !sum_exprs.is_empty() {
            Ok(MacroValue::Int(sum_result))
        } else if !count_exprs.is_empty() {
            Ok(MacroValue::Int(count_result))
        } else {
            Ok(MacroValue::Nil)
        }
    }

    fn bind_destructure(&self, pattern: &SurfaceForm, source: &MacroValue, env: &mut MacroEnv) {
        match &pattern.kind {
            SurfaceKind::DottedList(items, tail) => {
                let mut current = source.clone();
                for item in items {
                    if let Some(name) = item.symbol_name() {
                        env.bind(name.to_string(), current.car());
                    }
                    current = current.cdr();
                }
                if let Some(tail_name) = tail.symbol_name() {
                    env.bind(tail_name.to_string(), current);
                }
            }
            SurfaceKind::List(items) => {
                let mut current = source.clone();
                for item in items {
                    if let Some(name) = item.symbol_name() {
                        env.bind(name.to_string(), current.car());
                    }
                    current = current.cdr();
                }
            }
            _ => {}
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

    fn eval_and(&mut self, forms: &[SurfaceForm], env: &mut MacroEnv) -> Result<MacroValue, ()> {
        let mut result = MacroValue::Symbol("t".into());
        for form in forms {
            result = self.eval(form, env)?;
            if !result.is_truthy() {
                return Ok(MacroValue::Nil);
            }
        }
        Ok(result)
    }

    fn eval_or(&mut self, forms: &[SurfaceForm], env: &mut MacroEnv) -> Result<MacroValue, ()> {
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

        let saved: Vec<(String, Option<MacroValue>)> = bindings
            .iter()
            .map(|(n, _)| (n.clone(), env.save(n)))
            .collect();

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

        for (name, old_value) in saved {
            env.restore(name, old_value);
        }

        result
    }

    fn parse_let_bindings(&mut self, form: &SurfaceForm) -> Result<Vec<(String, SurfaceForm)>, ()> {
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

    fn parse_let_binding(&mut self, form: &SurfaceForm) -> Result<(String, SurfaceForm), ()> {
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
                Ok((
                    name.to_string(),
                    SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Nil), form.span),
                ))
            }
            SurfaceKind::Atom(_) => {
                if let Some(name) = form.symbol_name() {
                    Ok((
                        name.to_string(),
                        SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Nil), form.span),
                    ))
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
            if iterations > 1000 {
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

    fn eval_arithmetic(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
        init_i: i64,
        init_f: f64,
        op_i: impl Fn(i64, i64) -> i64,
        op_f: impl Fn(f64, f64) -> f64,
    ) -> Result<MacroValue, ()> {
        let values: Vec<MacroValue> = args
            .iter()
            .map(|arg| self.eval(arg, env))
            .collect::<Result<Vec<_>, _>>()?;
        // Float contagion: if any arg is a float, use float arithmetic
        let has_float = values.iter().any(|v| matches!(v, MacroValue::Float(..)));
        if has_float {
            let mut result = init_f;
            for val in &values {
                let n = match val {
                    MacroValue::Int(n) => *n as f64,
                    MacroValue::Float(n) => *n,
                    _ => {
                        self.error(span, "arithmetic requires number arguments");
                        return Err(());
                    }
                };
                result = op_f(result, n);
            }
            return Ok(MacroValue::Float(result));
        }
        let mut result = init_i;
        for val in &values {
            let n = match val {
                MacroValue::Int(n) => *n,
                _ => {
                    self.error(span, "arithmetic requires integer arguments");
                    return Err(());
                }
            };
            result = op_i(result, n);
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
        let values: Vec<MacroValue> = args
            .iter()
            .map(|arg| self.eval(arg, env))
            .collect::<Result<Vec<_>, _>>()?;
        let has_float = values.iter().any(|v| matches!(v, MacroValue::Float(..)));
        if has_float {
            if values.len() == 1 {
                let n = match &values[0] {
                    MacroValue::Int(n) => -(*n as f64),
                    MacroValue::Float(n) => -n,
                    _ => {
                        self.error(span, "arithmetic requires number arguments");
                        return Err(());
                    }
                };
                return Ok(MacroValue::Float(n));
            }
            let mut result = match &values[0] {
                MacroValue::Int(n) => *n as f64,
                MacroValue::Float(n) => *n,
                _ => {
                    self.error(span, "arithmetic requires number arguments");
                    return Err(());
                }
            };
            for val in &values[1..] {
                let n = match val {
                    MacroValue::Int(n) => *n as f64,
                    MacroValue::Float(n) => *n,
                    _ => {
                        self.error(span, "arithmetic requires number arguments");
                        return Err(());
                    }
                };
                result -= n;
            }
            return Ok(MacroValue::Float(result));
        }
        // Integer path
        if values.len() == 1 {
            let n = match &values[0] {
                MacroValue::Int(n) => -n,
                _ => {
                    self.error(span, "arithmetic requires integer arguments");
                    return Err(());
                }
            };
            return Ok(MacroValue::Int(n));
        }
        let mut result = match &values[0] {
            MacroValue::Int(n) => *n,
            _ => {
                self.error(span, "arithmetic requires integer arguments");
                return Err(());
            }
        };
        for val in &values[1..] {
            let n = match val {
                MacroValue::Int(n) => *n,
                _ => {
                    self.error(span, "arithmetic requires integer arguments");
                    return Err(());
                }
            };
            result = result.wrapping_sub(n);
        }
        Ok(MacroValue::Int(result))
    }

    fn eval_divide(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if args.is_empty() {
            self.error(span, "/ requires at least one argument");
            return Err(());
        }
        let values: Vec<MacroValue> = args
            .iter()
            .map(|arg| self.eval(arg, env))
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() == 1 {
            // (/ x) → 1/x  (Emacs behavior)
            match &values[0] {
                MacroValue::Int(n) => {
                    if *n == 0 {
                        self.error(span, "arith-error");
                        return Err(());
                    }
                    if *n == 1 || *n == -1 {
                        return Ok(MacroValue::Int(1 / n));
                    }
                    // 1 / n for |n| > 1 → 0 (integer division)
                    return Ok(MacroValue::Int(0));
                }
                MacroValue::Float(f) => {
                    if *f == 0.0 {
                        self.error(span, "arith-error");
                        return Err(());
                    }
                    return Ok(MacroValue::Float(1.0 / f));
                }
                _ => {
                    self.error(span, "arithmetic requires number arguments");
                    return Err(());
                }
            }
        }
        let has_float = values.iter().any(|v| matches!(v, MacroValue::Float(..)));
        if has_float {
            let mut result = match &values[0] {
                MacroValue::Int(n) => *n as f64,
                MacroValue::Float(n) => *n,
                _ => {
                    self.error(span, "arithmetic requires number arguments");
                    return Err(());
                }
            };
            for val in &values[1..] {
                let n = match val {
                    MacroValue::Int(n) => *n as f64,
                    MacroValue::Float(n) => *n,
                    _ => {
                        self.error(span, "arithmetic requires number arguments");
                        return Err(());
                    }
                };
                if n == 0.0 {
                    self.error(span, "arith-error");
                    return Err(());
                }
                result /= n;
            }
            return Ok(MacroValue::Float(result));
        }
        let mut result = match &values[0] {
            MacroValue::Int(n) => *n,
            _ => {
                self.error(span, "arithmetic requires integer arguments");
                return Err(());
            }
        };
        for val in &values[1..] {
            let n = match val {
                MacroValue::Int(n) => *n,
                _ => {
                    self.error(span, "arithmetic requires integer arguments");
                    return Err(());
                }
            };
            if n == 0 {
                self.error(span, "arith-error");
                return Err(());
            }
            result /= n;
        }
        Ok(MacroValue::Int(result))
    }

    fn eval_rem(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
        name: &str,
    ) -> Result<MacroValue, ()> {
        if args.len() != 2 {
            self.error(span, &format!("{name} requires exactly 2 arguments"));
            return Err(());
        }
        let a = self.eval(&args[0], env)?;
        let b = self.eval(&args[1], env)?;
        match (&a, &b) {
            (MacroValue::Int(x), MacroValue::Int(y)) => {
                if *y == 0 {
                    self.error(span, "arith-error");
                    return Err(());
                }
                if name == "mod" {
                    let r = x % y;
                    Ok(MacroValue::Int(if r == 0 || (y ^ r) >= 0 {
                        r
                    } else {
                        r + y
                    }))
                } else {
                    Ok(MacroValue::Int(x % y))
                }
            }
            (MacroValue::Int(x), MacroValue::Float(y)) => {
                let xf = *x as f64;
                if *y == 0.0 {
                    self.error(span, "arith-error");
                    return Err(());
                }
                Ok(MacroValue::Float(xf % y))
            }
            (MacroValue::Float(x), MacroValue::Int(y)) => {
                let yf = *y as f64;
                if yf == 0.0 {
                    self.error(span, "arith-error");
                    return Err(());
                }
                Ok(MacroValue::Float(x % yf))
            }
            (MacroValue::Float(x), MacroValue::Float(y)) => {
                if *y == 0.0 {
                    self.error(span, "arith-error");
                    return Err(());
                }
                Ok(MacroValue::Float(x % y))
            }
            _ => {
                self.error(span, "arithmetic requires integer or float arguments");
                Err(())
            }
        }
    }

    fn eval_ne(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        if args.len() != 2 {
            self.error(span, "/= requires exactly 2 arguments");
            return Err(());
        }
        let a = self.eval(&args[0], env)?;
        let b = self.eval(&args[1], env)?;
        let has_float = matches!(&a, MacroValue::Float(..)) || matches!(&b, MacroValue::Float(..));
        if has_float {
            let af = match &a {
                MacroValue::Int(n) => *n as f64,
                MacroValue::Float(n) => *n,
                _ => {
                    self.error(span, "arithmetic requires number arguments");
                    return Err(());
                }
            };
            let bf = match &b {
                MacroValue::Int(n) => *n as f64,
                MacroValue::Float(n) => *n,
                _ => {
                    self.error(span, "arithmetic requires number arguments");
                    return Err(());
                }
            };
            return Ok(MacroValue::from_bool(af != bf));
        }
        let ai = match a {
            MacroValue::Int(n) => n,
            _ => {
                self.error(span, "arithmetic requires integer arguments");
                return Err(());
            }
        };
        let bi = match b {
            MacroValue::Int(n) => n,
            _ => {
                self.error(span, "arithmetic requires integer arguments");
                return Err(());
            }
        };
        Ok(MacroValue::from_bool(ai != bi))
    }

    fn eval_numeric_cmp(
        &mut self,
        span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
        pred_i: impl Fn(i64, i64) -> bool,
        pred_f: impl Fn(f64, f64) -> bool,
    ) -> Result<MacroValue, ()> {
        if args.len() < 2 {
            self.error(span, "comparison requires at least two arguments");
            return Err(());
        }
        let values: Vec<MacroValue> = args
            .iter()
            .map(|arg| self.eval(arg, env))
            .collect::<Result<Vec<_>, _>>()?;
        let has_float = values.iter().any(|v| matches!(v, MacroValue::Float(..)));
        if has_float {
            let first = match &values[0] {
                MacroValue::Int(n) => *n as f64,
                MacroValue::Float(n) => *n,
                _ => return Err(()),
            };
            for val in &values[1..] {
                let n = match val {
                    MacroValue::Int(n) => *n as f64,
                    MacroValue::Float(n) => *n,
                    _ => return Err(()),
                };
                if !pred_f(first, n) {
                    return Ok(MacroValue::Nil);
                }
            }
            return Ok(MacroValue::Symbol("t".into()));
        }
        let first = match &values[0] {
            MacroValue::Int(n) => *n,
            _ => return Err(()),
        };
        for val in &values[1..] {
            let n = match val {
                MacroValue::Int(n) => *n,
                _ => return Err(()),
            };
            if !pred_i(first, n) {
                return Ok(MacroValue::Nil);
            }
        }
        Ok(MacroValue::Symbol("t".into()))
    }

    fn eval_concat(
        &mut self,
        _span: Span,
        args: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        let mut result = String::new();
        for arg in args {
            let val = self.eval(arg, env)?;
            match val {
                MacroValue::String(s) => result.push_str(&s),
                MacroValue::Int(n) => {
                    if let Some(c) = u32::try_from(n).ok().and_then(char::from_u32) {
                        result.push(c);
                    }
                }
                MacroValue::Nil => {}
                _ => {}
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
                    s_text.len().saturating_sub((-n) as usize)
                } else {
                    (n as usize).min(s_text.len())
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
                        s_text.len().saturating_sub((-n) as usize)
                    } else {
                        (n as usize).min(s_text.len())
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
        Ok(MacroValue::String(
            s_text[from_idx.min(to_idx)..to_idx].to_string(),
        ))
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
                let spec = chars.next();
                match spec {
                    Some('s') | Some('S') => {
                        if arg_idx < format_args.len() {
                            result.push_str(&format_value_as_string(&format_args[arg_idx]));
                            arg_idx += 1;
                        }
                    }
                    Some('d') => {
                        if arg_idx < format_args.len() {
                            result
                                .push_str(&format_args[arg_idx].as_int().unwrap_or(0).to_string());
                            arg_idx += 1;
                        }
                    }
                    Some('c') => {
                        if arg_idx < format_args.len() {
                            let n = format_args[arg_idx].as_int().unwrap_or(0);
                            if let Some(c) = u32::try_from(n).ok().and_then(char::from_u32) {
                                result.push(c);
                            }
                            arg_idx += 1;
                        }
                    }
                    Some('e') | Some('E') | Some('f') | Some('g') | Some('G') => {
                        if arg_idx < format_args.len() {
                            let val = &format_args[arg_idx];
                            if let MacroValue::Float(f) = val {
                                result.push_str(&format!("{f}"));
                            } else {
                                result.push_str(&val.as_int().unwrap_or(0).to_string());
                                result.push_str(".0");
                            }
                            arg_idx += 1;
                        }
                    }
                    Some(spec @ 'x') | Some(spec @ 'X') | Some(spec @ 'o') => {
                        if arg_idx < format_args.len() {
                            let n = format_args[arg_idx].as_int().unwrap_or(0);
                            if n == 0 {
                                result.push('0');
                            } else {
                                match spec {
                                    'o' => result.push_str(&format!("{n:o}")),
                                    'x' => result.push_str(&format!("{n:x}")),
                                    'X' => result.push_str(&format!("{n:X}")),
                                    _ => {}
                                }
                            }
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
        if depth > 10 {
            return Err(());
        }
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
                Ok(MacroValue::Vector(Rc::new(result)))
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

    fn call_function(
        &mut self,
        span: Span,
        func_val: &MacroValue,
        args: &[MacroValue],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        // Direct symbol: look up named function
        if let MacroValue::Symbol(name) = func_val {
            if let Some(func) = env.lookup_function(name).cloned() {
                return self.call_macro_function(&func, args, env);
            }
            // Handle built-in predicates that mapcar/mapc/remove-if use
            return self.call_builtin_predicate(span, name, args);
        }
        // (function name) — function-quoted symbol: extract the name
        if let MacroValue::Cons(pair) = func_val {
            if let MacroValue::Symbol(ref fn_sym) = pair.car {
                if fn_sym == "function" {
                    if let MacroValue::Cons(rest) = &pair.cdr {
                        if let MacroValue::Symbol(ref name) = rest.car {
                            if let Some(func) = env.lookup_function(name).cloned() {
                                return self.call_macro_function(&func, args, env);
                            }
                            return self.call_builtin_predicate(span, name, args);
                        }
                    }
                }
            }
        }
        // Try lambda value
        if let Some((params, body_forms)) = extract_lambda(func_val) {
            let func = MacroFunction {
                params,
                body: body_forms,
            };
            return self.call_macro_function(&func, args, env);
        }
        // Unknown function — return nil
        Ok(MacroValue::Nil)
    }

    fn call_builtin_predicate(
        &mut self,
        span: Span,
        name: &str,
        args: &[MacroValue],
    ) -> Result<MacroValue, ()> {
        if args.is_empty() {
            return Ok(MacroValue::Nil);
        }
        match name {
            "symbolp" => Ok(MacroValue::from_bool(args[0].is_symbol())),
            "listp" => Ok(MacroValue::from_bool(args[0].is_list())),
            "consp" => Ok(MacroValue::from_bool(args[0].is_cons())),
            "stringp" => Ok(MacroValue::from_bool(args[0].is_string())),
            "numberp" => Ok(MacroValue::from_bool(
                args[0].is_int() || matches!(args[0], MacroValue::Float(..)),
            )),
            "integerp" => Ok(MacroValue::from_bool(args[0].is_int())),
            "floatp" => Ok(MacroValue::from_bool(matches!(
                args[0],
                MacroValue::Float(..)
            ))),
            "null" | "not" => Ok(MacroValue::from_bool(args[0].is_nil())),
            "atom" => Ok(MacroValue::from_bool(!args[0].is_cons())),
            "identity" => Ok(args[0].clone()),
            _ => Ok(MacroValue::Nil),
        }
    }

    fn call_macro_function(
        &mut self,
        func: &MacroFunction,
        args: &[MacroValue],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        let saved: Vec<(String, Option<MacroValue>)> = func
            .params
            .iter()
            .map(|p| (p.clone(), env.save(p)))
            .collect();
        for (i, param) in func.params.iter().enumerate() {
            let val = args.get(i).cloned().unwrap_or(MacroValue::Nil);
            env.bind(param.clone(), val);
        }
        let result = self.eval_progn(&func.body, env);
        for (name, old) in saved {
            env.restore(name, old);
        }
        result
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
        MacroValue::Float(f) => f.to_string(),
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

fn parse_lambda_params(form: &SurfaceForm) -> Vec<String> {
    match &form.kind {
        SurfaceKind::List(items) => items
            .iter()
            .filter_map(|f| f.symbol_name().map(|s| s.to_string()))
            .collect(),
        SurfaceKind::Atom(SurfaceAtom::Nil) => Vec::new(),
        _ => Vec::new(),
    }
}

fn extract_lambda(val: &MacroValue) -> Option<(Vec<String>, Vec<SurfaceForm>)> {
    let items = val.to_vec()?;
    if items.is_empty() {
        return None;
    }
    if items[0] != MacroValue::Symbol("lambda".into()) {
        return None;
    }
    let params: Vec<String> = items
        .get(1)?
        .to_vec()?
        .into_iter()
        .filter_map(|v| v.as_symbol_name().map(|s| s.to_string()))
        .collect();
    let body: Vec<SurfaceForm> = items[2..]
        .iter()
        .filter_map(|v| value_to_surface_form(v))
        .collect();
    if body.is_empty() {
        return None;
    }
    Some((params, body))
}

fn value_to_surface_form(val: &MacroValue) -> Option<SurfaceForm> {
    use crate::source::SourceId;
    let span = Span::new(SourceId::new(0), 0, 0);
    Some(crate::expand_value::value_to_surface(val, span))
}

fn append_values(values: &[MacroValue]) -> MacroValue {
    use std::rc::Rc;
    let mut all_elements: Vec<MacroValue> = Vec::new();
    for val in values {
        if val.is_nil() {
            continue;
        }
        let mut cur = val.clone();
        while let MacroValue::Cons(pair) = cur {
            all_elements.push(pair.car.clone());
            cur = pair.cdr.clone();
        }
    }
    let mut result = MacroValue::Nil;
    for item in all_elements.into_iter().rev() {
        result = MacroValue::Cons(Rc::new(crate::expand_value::MacroCons {
            car: item,
            cdr: result,
        }));
    }
    result
}

/// Check if a SurfaceForm references any of the given binding names.
/// Recursively walks lists, dotted lists, quotes, etc.
fn form_references_bindings(form: &SurfaceForm, names: &[String]) -> bool {
    match &form.kind {
        SurfaceKind::Atom(atom) => {
            if let SurfaceAtom::Symbol(s) = atom {
                names.iter().any(|n| n == s)
            } else {
                false
            }
        }
        SurfaceKind::List(items) => items.iter().any(|f| form_references_bindings(f, names)),
        SurfaceKind::DottedList(items, tail) => {
            items.iter().any(|f| form_references_bindings(f, names))
                || form_references_bindings(tail, names)
        }
        SurfaceKind::Quote(f) | SurfaceKind::FunctionQuote(f) => form_references_bindings(f, names),
        SurfaceKind::Backquote(f) => form_references_bindings(f, names),
        SurfaceKind::Comma(f) | SurfaceKind::CommaAt(f) => form_references_bindings(f, names),
        SurfaceKind::Vector(items) | SurfaceKind::HashList(items) => {
            items.iter().any(|f| form_references_bindings(f, names))
        }
        SurfaceKind::Record(type_name, items) => {
            form_references_bindings(type_name, names)
                || items.iter().any(|f| form_references_bindings(f, names))
        }
        SurfaceKind::CharTable(items) => items.iter().any(|f| form_references_bindings(f, names)),
        SurfaceKind::Labeled(_, f) => form_references_bindings(f, names),
        SurfaceKind::Ref(_) => false,
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
        // Unknown function calls now pass through to the compiler
        // instead of erroring at macro expansion time
        let result = parse_and_eval("(some-unknown-fn 1)");
        assert!(result.is_ok());
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

    #[test]
    fn funcall_lambda() {
        let v = eval_expr("(funcall (lambda (x) (+ x 1)) 41)");
        assert_eq!(v, MacroValue::Int(42));
    }

    #[test]
    fn funcall_named_function() {
        let mut eval = MacroEval::new();
        let mut env = MacroEnv::default();
        // Define add1 as a named function in the env
        let body_src = SourceFile::new(SourceId::new(0), Some("test.el".into()), "(+ x 1)".into());
        let body_output = crate::reader::read_source(&body_src);
        env.define_function(
            "add1".into(),
            MacroFunction {
                params: vec!["x".into()],
                body: body_output.forms,
            },
        );
        // Call (funcall 'add1 10)
        let call_src = SourceFile::new(
            SourceId::new(0),
            Some("test.el".into()),
            "(funcall 'add1 10)".into(),
        );
        let call_output = crate::reader::read_source(&call_src);
        let result = eval.eval(&call_output.forms[0], &mut env).unwrap();
        assert_eq!(result, MacroValue::Int(11));
    }

    #[test]
    fn funcall_calls_lambda_with_multiple_args() {
        let v = eval_expr("(funcall (lambda (a b) (+ a b)) 3 4)");
        assert_eq!(v, MacroValue::Int(7));
    }

    #[test]
    fn mapcar_with_lambda() {
        let v = eval_expr("(mapcar (lambda (x) (+ x 10)) (list 1 2 3))");
        let vec = v.to_vec().unwrap();
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], MacroValue::Int(11));
        assert_eq!(vec[1], MacroValue::Int(12));
        assert_eq!(vec[2], MacroValue::Int(13));
    }

    #[test]
    fn mapcar_empty_list() {
        let v = eval_expr("(mapcar (lambda (x) x) nil)");
        assert_eq!(v, MacroValue::Nil);
    }

    #[test]
    fn backquote_simple_list() {
        let v = eval_expr("`(a 3 b)");
        let vec = v.to_vec().unwrap();
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], MacroValue::Symbol("a".into()));
        assert_eq!(vec[1], MacroValue::Int(3));
        assert_eq!(vec[2], MacroValue::Symbol("b".into()));
    }

    #[test]
    fn backquote_with_comma() {
        let v = eval_expr("(let ((x 3)) `(a ,x b))");
        let vec = v.to_vec().unwrap();
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], MacroValue::Symbol("a".into()));
        assert_eq!(vec[1], MacroValue::Int(3));
        assert_eq!(vec[2], MacroValue::Symbol("b".into()));
    }

    #[test]
    fn backquote_with_splice() {
        let v = eval_expr("(let ((xs (list 1 2 3))) `(a ,@xs b))");
        let vec = v.to_vec().unwrap();
        assert_eq!(vec.len(), 5);
        assert_eq!(vec[0], MacroValue::Symbol("a".into()));
        assert_eq!(vec[1], MacroValue::Int(1));
        assert_eq!(vec[2], MacroValue::Int(2));
        assert_eq!(vec[3], MacroValue::Int(3));
        assert_eq!(vec[4], MacroValue::Symbol("b".into()));
    }

    #[test]
    fn backquote_splice_over_list_quote() {
        // Simulating (let ((xs '(1 2 3))) `(a ,@xs b))
        let v = eval_expr("(let ((xs '(1 2 3))) `(a ,@xs b))");
        let vec = v.to_vec().unwrap();
        assert_eq!(vec.len(), 5);
        assert_eq!(vec[0], MacroValue::Symbol("a".into()));
        assert_eq!(vec[1], MacroValue::Int(1));
        assert_eq!(vec[2], MacroValue::Int(2));
        assert_eq!(vec[3], MacroValue::Int(3));
        assert_eq!(vec[4], MacroValue::Symbol("b".into()));
    }

    #[test]
    fn assq_finds_key() {
        // assq returns the pair (b . c)
        let v = eval_expr("(assq 'b '((a . 1) (b . c)))");
        assert!(v.is_cons());
        let car = v.car();
        let cdr = v.cdr();
        assert_eq!(car, MacroValue::Symbol("b".into()));
        assert_eq!(cdr, MacroValue::Symbol("c".into()));
    }

    #[test]
    fn concat_returns_string() {
        let v = eval_expr("(concat \"hello\" \" \" \"world\")");
        assert_eq!(v, MacroValue::String("hello world".into()));
    }
}
