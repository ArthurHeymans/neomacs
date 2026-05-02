# Macroexpander Stage 1: MacroValue + Surface-Form Evaluator

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the hand-rolled `eval_macro_expr` chain with a proper `MacroValue` type and clean mini-interpreter, enabling real macro bodies that use list operations, predicates, string/symbol ops, and quasiquote.

**Architecture:** Introduce `MacroValue` (a proper Lisp value type separate from `SurfaceForm` syntax) with bidirectional conversion functions. Build a `MacroEval` evaluator that takes `SurfaceForm` code and `MacroEnv` bindings, produces `MacroValue` results. Plug it into the existing `Expander.invoke_macro` call site. Delete the old `eval_macro_*` chain.

**Tech Stack:** Rust std only (`Rc`, `HashMap`). No new crate dependencies.

**Test command:** `cd neovm-compiler && cargo nextest run`

---

## File Structure

| File | Responsibility | Action |
|---|---|---|
| `neovm-compiler/src/expand_value.rs` | `MacroValue`, `MacroCons`, `surface_to_value`, `value_to_surface` | Create |
| `neovm-compiler/src/expand_eval.rs` | `MacroEval` evaluator, `MacroEnv` | Create |
| `neovm-compiler/src/expand.rs` | `Expander`, built-in rewrites, defmacro registration | Modify (delete ~650 lines of `eval_macro_*`, simplify `invoke_macro`) |
| `neovm-compiler/src/lib.rs` | Module declarations | Modify (add two new modules) |

---

### Task 1: MacroValue type and helpers

**Files:**
- Create: `neovm-compiler/src/expand_value.rs`

- [ ] **Step 1: Create expand_value.rs with MacroValue type**

```rust
use std::rc::Rc;

use crate::source::Span;
use crate::surface::{SurfaceAtom, SurfaceForm, SurfaceKind};

/// Lisp value for macro-time evaluation.
/// Separates computed values from parsed syntax (SurfaceForm).
#[derive(Clone, Debug, PartialEq)]
pub enum MacroValue {
    Nil,
    Int(i64),
    Symbol(String),
    String(String),
    Cons(Rc<MacroCons>),
    Vector(Vec<MacroValue>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MacroCons {
    pub car: MacroValue,
    pub cdr: MacroValue,
}

impl MacroValue {
    pub fn is_nil(&self) -> bool {
        matches!(self, MacroValue::Nil)
    }

    pub fn is_truthy(&self) -> bool {
        !self.is_nil()
    }

    pub fn is_cons(&self) -> bool {
        matches!(self, MacroValue::Cons(_))
    }

    pub fn is_list(&self) -> bool {
        self.is_nil() || self.is_cons()
    }

    pub fn is_symbol(&self) -> bool {
        matches!(self, MacroValue::Symbol(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, MacroValue::String(_))
    }

    pub fn is_int(&self) -> bool {
        matches!(self, MacroValue::Int(_))
    }

    pub fn cons(car: MacroValue, cdr: MacroValue) -> MacroValue {
        MacroValue::Cons(Rc::new(MacroCons { car, cdr }))
    }

    pub fn list(items: Vec<MacroValue>) -> MacroValue {
        let mut tail = MacroValue::Nil;
        for item in items.into_iter().rev() {
            tail = MacroValue::cons(item, tail);
        }
        tail
    }

    pub fn car(&self) -> MacroValue {
        match self {
            MacroValue::Cons(pair) => pair.car.clone(),
            _ => MacroValue::Nil,
        }
    }

    pub fn cdr(&self) -> MacroValue {
        match self {
            MacroValue::Cons(pair) => pair.cdr.clone(),
            _ => MacroValue::Nil,
        }
    }

    /// Collect proper list into Vec. Returns None if not a proper list.
    pub fn to_vec(&self) -> Option<Vec<MacroValue>> {
        let mut items = Vec::new();
        let mut current = self;
        loop {
            match current {
                MacroValue::Nil => return Some(items),
                MacroValue::Cons(pair) => {
                    items.push(pair.car.clone());
                    current = &pair.cdr;
                }
                _ => return None,
            }
        }
    }

    pub fn as_symbol_name(&self) -> Option<&str> {
        match self {
            MacroValue::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            MacroValue::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            MacroValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn from_bool(b: bool) -> MacroValue {
        if b {
            MacroValue::Symbol("t".into())
        } else {
            MacroValue::Nil
        }
    }
}

/// Convert unevaluated SurfaceForm syntax to a MacroValue.
/// No evaluation — purely a representation change.
pub fn surface_to_value(form: &SurfaceForm) -> MacroValue {
    match &form.kind {
        SurfaceKind::Atom(atom) => atom_to_value(atom),
        SurfaceKind::List(items) => {
            MacroValue::list(items.iter().map(surface_to_value).collect())
        }
        SurfaceKind::DottedList(items, tail) => {
            let mut result = surface_to_value(tail);
            for item in items.iter().rev() {
                result = MacroValue::cons(surface_to_value(item), result);
            }
            result
        }
        SurfaceKind::Vector(items) => {
            MacroValue::Vector(items.iter().map(surface_to_value).collect())
        }
        SurfaceKind::Quote(inner) => {
            MacroValue::list(vec![
                MacroValue::Symbol("quote".into()),
                surface_to_value(inner),
            ])
        }
        SurfaceKind::FunctionQuote(inner) => {
            MacroValue::list(vec![
                MacroValue::Symbol("function".into()),
                surface_to_value(inner),
            ])
        }
        SurfaceKind::Backquote(inner) => {
            MacroValue::list(vec![
                MacroValue::Symbol("backquote".into()),
                surface_to_value(inner),
            ])
        }
        SurfaceKind::Comma(inner) => {
            MacroValue::list(vec![
                MacroValue::Symbol("unquote".into()),
                surface_to_value(inner),
            ])
        }
        SurfaceKind::CommaAt(inner) => {
            MacroValue::list(vec![
                MacroValue::Symbol("splice-unquote".into()),
                surface_to_value(inner),
            ])
        }
    }
}

fn atom_to_value(atom: &SurfaceAtom) -> MacroValue {
    match atom {
        SurfaceAtom::Nil => MacroValue::Nil,
        SurfaceAtom::True => MacroValue::Symbol("t".into()),
        SurfaceAtom::Int(n) => MacroValue::Int(*n),
        SurfaceAtom::Float(_) => MacroValue::Nil, // no float in MacroValue
        SurfaceAtom::Symbol(s) => MacroValue::Symbol(s.clone()),
        SurfaceAtom::String(s) => MacroValue::String(s.clone()),
        SurfaceAtom::Char(c) => MacroValue::Int(*c),
    }
}

/// Convert a MacroValue back to SurfaceForm syntax for the compiler pipeline.
pub fn value_to_surface(value: &MacroValue, span: Span) -> SurfaceForm {
    match value {
        MacroValue::Nil => SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Nil), span),
        MacroValue::Int(n) => SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(*n)), span),
        MacroValue::Symbol(s) => {
            let atom = SurfaceAtom::symbol(s);
            SurfaceForm::new(SurfaceKind::Atom(atom), span)
        }
        MacroValue::String(s) => {
            SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::String(s.clone())), span)
        }
        MacroValue::Cons(pair) => {
            let mut items = Vec::new();
            let mut current = pair.as_ref();
            loop {
                items.push(value_to_surface(&current.car, span));
                match &current.cdr {
                    MacroValue::Nil => {
                        return SurfaceForm::new(SurfaceKind::List(items), span);
                    }
                    MacroValue::Cons(next) => {
                        current = next;
                    }
                    other => {
                        let tail = value_to_surface(other, span);
                        return SurfaceForm::new(
                            SurfaceKind::DottedList(items, Box::new(tail)),
                            span,
                        );
                    }
                }
            }
        }
        MacroValue::Vector(items) => {
            let forms: Vec<SurfaceForm> =
                items.iter().map(|v| value_to_surface(v, span)).collect();
            SurfaceForm::new(SurfaceKind::Vector(forms), span)
        }
    }
}
```

- [ ] **Step 2: Write conversion round-trip tests**

Add at the bottom of `expand_value.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceId;

    fn test_span() -> Span {
        Span::new(SourceId::new(0), 0, 1)
    }

    fn sym(name: &str) -> SurfaceForm {
        SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::symbol(name)), test_span())
    }

    fn int_form(n: i64) -> SurfaceForm {
        SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Int(n)), test_span())
    }

    fn str_form(s: &str) -> SurfaceForm {
        SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::String(s.into())), test_span())
    }

    fn nil_form() -> SurfaceForm {
        SurfaceForm::new(SurfaceKind::Atom(SurfaceAtom::Nil), test_span())
    }

    fn list_form(items: Vec<SurfaceForm>) -> SurfaceForm {
        SurfaceForm::new(SurfaceKind::List(items), test_span())
    }

    #[test]
    fn atom_round_trips() {
        let span = test_span();
        let cases: Vec<SurfaceForm> = vec![
            nil_form(),
            int_form(42),
            sym("foo"),
            str_form("hello"),
        ];
        for form in &cases {
            let value = surface_to_value(form);
            let back = value_to_surface(&value, span);
            assert_eq!(&back, form, "round-trip failed for {:?}", form.kind);
        }
    }

    #[test]
    fn proper_list_round_trips() {
        let span = test_span();
        let form = list_form(vec![sym("a"), int_form(1), sym("b")]);
        let value = surface_to_value(&form);
        let back = value_to_surface(&value, span);
        assert_eq!(back, form);
    }

    #[test]
    fn dotted_list_preserves_tail() {
        let span = test_span();
        let form = SurfaceForm::new(
            SurfaceKind::DottedList(vec![sym("a"), sym("b")], Box::new(sym("c"))),
            span,
        );
        let value = surface_to_value(&form);
        assert!(value.is_cons());
        let back = value_to_surface(&value, span);
        assert_eq!(back, form);
    }

    #[test]
    fn nested_list_round_trips() {
        let span = test_span();
        let inner = list_form(vec![int_form(1), int_form(2)]);
        let outer = list_form(vec![sym("foo"), inner]);
        let value = surface_to_value(&outer);
        let back = value_to_surface(&value, span);
        assert_eq!(back, outer);
    }

    #[test]
    fn vector_round_trips() {
        let span = test_span();
        let form = SurfaceForm::new(
            SurfaceKind::Vector(vec![int_form(1), sym("x")]),
            span,
        );
        let value = surface_to_value(&form);
        let back = value_to_surface(&value, span);
        assert_eq!(back, form);
    }

    #[test]
    fn nil_is_falsy() {
        assert!(!MacroValue::Nil.is_truthy());
        assert!(MacroValue::Nil.is_nil());
        assert!(MacroValue::Nil.is_list());
    }

    #[test]
    fn symbol_t_is_truthy() {
        let t = MacroValue::Symbol("t".into());
        assert!(t.is_truthy());
        assert!(t.is_symbol());
        assert!(!t.is_nil());
    }

    #[test]
    fn list_constructor() {
        let list = MacroValue::list(vec![
            MacroValue::Int(1),
            MacroValue::Int(2),
            MacroValue::Int(3),
        ]);
        assert!(list.is_cons());
        let vec = list.to_vec().unwrap();
        assert_eq!(vec.len(), 3);
    }

    #[test]
    fn cons_car_cdr() {
        let pair = MacroValue::cons(MacroValue::Int(1), MacroValue::Int(2));
        assert_eq!(pair.car(), MacroValue::Int(1));
        assert_eq!(pair.cdr(), MacroValue::Int(2));
    }
}
```

- [ ] **Step 3: Add module to lib.rs**

Add these two module declarations to `neovm-compiler/src/lib.rs` (after the existing `pub mod expand;` line):

```rust
pub mod expand_value;
pub mod expand_eval;
```

- [ ] **Step 4: Run tests**

Run: `cd neovm-compiler && cargo nextest run`
Expected: all existing 90 tests pass, plus ~8 new tests in `expand_value` pass.

- [ ] **Step 5: Commit**

```bash
git add neovm-compiler/src/expand_value.rs neovm-compiler/src/lib.rs
git commit -m "Add MacroValue type and surface form conversion"
```

---

### Task 2: MacroEnv and MacroEval core

**Files:**
- Create: `neovm-compiler/src/expand_eval.rs`

- [ ] **Step 1: Create expand_eval.rs with MacroEnv and core evaluator dispatch**

```rust
use std::collections::HashMap;

use crate::diagnostic::Diagnostic;
use crate::expand_value::{
    surface_to_value, value_to_surface, MacroValue,
};
use crate::source::Span;
use crate::surface::{SurfaceForm, SurfaceKind};

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

    /// Evaluate a sequence of forms, returning the last value.
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

    /// Evaluate a single form.
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

    fn eval_atom(&mut self, atom: &crate::surface::SurfaceAtom, env: &mut MacroEnv) -> MacroValue {
        use crate::surface::SurfaceAtom;
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

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message.into()).with_span(span));
    }
}
```

- [ ] **Step 2: Add eval_list_form dispatch**

Add to `impl MacroEval`:

```rust
    fn eval_list_form(
        &mut self,
        span: Span,
        items: &[SurfaceForm],
        env: &mut MacroEnv,
    ) -> Result<MacroValue, ()> {
        let head = items.first().and_then(|f| f.symbol_name());
        match head {
            // Special forms
            Some("quote") => {
                if items.len() != 2 {
                    self.error(span, "quote requires exactly one argument");
                    return Err(());
                }
                Ok(surface_to_value(&items[1]))
            }
            Some("if") => self.eval_if(span, items, env),
            Some("cond") => self.eval_cond(span, &items[1..], env),
            Some("and") => self.eval_and(span, &items[1..], env),
            Some("or") => self.eval_or(span, &items[1..], env),
            Some("let") => self.eval_let(span, &items[1..], env, false),
            Some("let*") => self.eval_let(span, &items[1..], env, true),
            Some("setq") => self.eval_setq(span, &items[1..], env),
            Some("progn") => self.eval_progn(&items[1..], env),

            // Named function calls
            Some("car") | Some("first") => {
                self.eval_unary(span, "car", &items[1..], env, |v| v.car())
            }
            Some("cdr") | Some("rest") => {
                self.eval_unary(span, "cdr", &items[1..], env, |v| v.cdr())
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

            // Predicates
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
                a == b || (a.is_int() && b.is_int() && a.as_int() == b.as_int())
            }),
            Some("equal") => self.eval_binary_pred(span, &items[1..], env, |a, b| a == b),

            // Arithmetic
            Some("+") => self.eval_fold(span, &items[1..], env, 0i64, |a, b| a.wrapping_add(b)),
            Some("-") => self.eval_sub(span, &items[1..], env),
            Some("*") => self.eval_fold(span, &items[1..], env, 1i64, |a, b| a.wrapping_mul(b)),

            // Comparison
            Some("=") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a == b),
            Some("<") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a < b),
            Some(">") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a > b),
            Some("<=") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a <= b),
            Some(">=") => self.eval_numeric_cmp(span, &items[1..], env, |a, b| a >= b),

            // Symbols and strings
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
            Some("concat") => self.eval_concat(span, &items[1..], env),
            Some("substring") => self.eval_substring(span, &items[1..], env),
            Some("string=") => self.eval_binary_pred(span, &items[1..], env, |a, b| {
                a.as_string() == b.as_string() && a.is_string() && b.is_string()
            }),
            Some("format") => self.eval_format(span, &items[1..], env),

            // Error signaling
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

            _ => {
                self.error(span, format!(
                    "cannot evaluate '{}' at macro expansion time",
                    head.unwrap_or("?")
                ));
                Err(())
            }
        }
    }
```

- [ ] **Step 3: Add special form implementations**

Add to `impl MacroEval`:

```rust
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
        _span: Span,
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
        _span: Span,
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
            SurfaceKind::List(items) if items.len() == 2 => {
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
                    crate::surface::SurfaceKind::Atom(crate::surface::SurfaceAtom::Nil),
                    form.span,
                )))
            }
            SurfaceKind::Atom(_) => {
                if let Some(name) = form.symbol_name() {
                    Ok((name.to_string(), SurfaceForm::new(
                        crate::surface::SurfaceKind::Atom(crate::surface::SurfaceAtom::Nil),
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
```

- [ ] **Step 4: Add function call helpers**

```rust
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
        span: Span,
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
```

Add the helper function at module level (outside `impl MacroEval`):

```rust
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
```

- [ ] **Step 5: Add quasiquote evaluator**

```rust
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
                    // splice is handled by list/vector processing below
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
                self.quasiquote_list(items, env, depth)
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

    fn quasiquote_list(
        &mut self,
        items: &[SurfaceForm],
        env: &mut MacroEnv,
        depth: usize,
    ) -> Result<MacroValue, ()> {
        let expanded = self.quasiquote_items(items, env, depth)?;
        Ok(MacroValue::list(expanded))
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
```

- [ ] **Step 6: Write evaluator tests**

Add at bottom of `expand_eval.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::expand_value::surface_to_value;
    use crate::source::SourceId;
    use crate::surface::{SurfaceAtom, SurfaceKind};

    fn test_span() -> Span {
        Span::new(SourceId::new(0), 0, 1)
    }

    fn parse_and_eval(source: &str) -> Result<MacroValue, ()> {
        // Use reader to get surface forms, bypassing expansion/compilation
        use crate::source::SourceFile;
        use crate::source::SourceId;
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
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n42");
        assert_eq!(v, MacroValue::Int(42));
    }

    #[test]
    fn evals_self_evaluating_string() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n\"hello\"");
        assert_eq!(v, MacroValue::String("hello".into()));
    }

    #[test]
    fn evals_nil() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\nnil");
        assert_eq!(v, MacroValue::Nil);
    }

    #[test]
    fn evals_if_then_branch() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(if t 1 2)");
        assert_eq!(v, MacroValue::Int(1));
    }

    #[test]
    fn evals_if_else_branch() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(if nil 1 2)");
        assert_eq!(v, MacroValue::Int(2));
    }

    #[test]
    fn evals_if_nil_else() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(if nil 1)");
        assert_eq!(v, MacroValue::Nil);
    }

    #[test]
    fn evals_let_binding() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(let ((x 10)) x)");
        assert_eq!(v, MacroValue::Int(10));
    }

    #[test]
    fn evals_let_star_sequential() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(let* ((x 1) (y (+ x 1))) y)");
        assert_eq!(v, MacroValue::Int(2));
    }

    #[test]
    fn evals_setq() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(let ((x 0)) (setq x 42) x)");
        assert_eq!(v, MacroValue::Int(42));
    }

    #[test]
    fn evals_cons_car_cdr() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(car (cons 1 2))");
        assert_eq!(v, MacroValue::Int(1));
    }

    #[test]
    fn evals_list_and_nth() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(nth 1 (list 10 20 30))");
        assert_eq!(v, MacroValue::Int(20));
    }

    #[test]
    fn evals_append() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(append (list 1 2) (list 3 4))");
        let vec = v.to_vec().unwrap();
        assert_eq!(vec.len(), 4);
        assert_eq!(vec[0], MacroValue::Int(1));
        assert_eq!(vec[3], MacroValue::Int(4));
    }

    #[test]
    fn evals_null_predicate() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(null nil)");
        assert!(v.is_truthy());
    }

    #[test]
    fn evals_consp_predicate() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(consp (list 1))");
        assert!(v.is_truthy());
    }

    #[test]
    fn evals_eq() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(eq 'foo 'foo)");
        assert!(v.is_truthy());
    }

    #[test]
    fn evals_arithmetic() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(+ 1 (* 2 3))");
        assert_eq!(v, MacroValue::Int(7));
    }

    #[test]
    fn evals_comparison() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(< 1 2)");
        assert!(v.is_truthy());
    }

    #[test]
    fn evals_and_short_circuit() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(and 1 2 3)");
        assert_eq!(v, MacroValue::Int(3));
    }

    #[test]
    fn evals_or_short_circuit() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(or nil nil 42)");
        assert_eq!(v, MacroValue::Int(42));
    }

    #[test]
    fn evals_quote() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(car '(1 2 3))");
        assert_eq!(v, MacroValue::Int(1));
    }

    #[test]
    fn reports_unknown_function() {
        let result = parse_and_eval(";;; -*- lexical-binding: t; -*-\n(some-unknown-fn 1)");
        assert!(result.is_err());
    }

    #[test]
    fn evals_symbol_name() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(symbol-name 'foo)");
        assert_eq!(v, MacroValue::String("foo".into()));
    }

    #[test]
    fn evals_length() {
        let v = eval_expr(";;; -*- lexical-binding: t; -*-\n(length (list 1 2 3))");
        assert_eq!(v, MacroValue::Int(3));
    }
}
```

- [ ] **Step 7: Run tests**

Run: `cd neovm-compiler && cargo nextest run`
Expected: all existing 90 tests pass, plus ~25 new tests pass.

- [ ] **Step 8: Commit**

```bash
git add neovm-compiler/src/expand_eval.rs
git commit -m "Add MacroEval evaluator with special forms and function calls"
```

---

### Task 3: Wire MacroEval into Expander, delete old eval chain

**Files:**
- Modify: `neovm-compiler/src/expand.rs`

This is the integration task. Replace `invoke_macro` to use `MacroEval`, then delete all the old `eval_macro_*` methods and unused helper functions.

- [ ] **Step 1: Replace invoke_macro implementation**

In `expand.rs`, replace the `invoke_macro` method (lines 161-220) with:

```rust
    fn invoke_macro(&mut self, def: &MacroDef, args: &[SurfaceForm]) -> Option<SurfaceForm> {
        use crate::expand_eval::{MacroEval, MacroEnv};
        use crate::expand_value::{surface_to_value, value_to_surface};

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
```

Add the `use` import at the top of the method or at the top of the file. Add `use crate::expand_value::MacroValue;` to the file-level imports.

- [ ] **Step 2: Delete old eval_macro_* methods**

Delete the following methods from `impl Expander`:
- `eval_macro_expr` (lines 222-245)
- `eval_macro_list` (lines 247-329)
- `eval_macro_if` (lines 331-350)
- `eval_macro_and` (lines 352-366)
- `eval_macro_or` (lines 368-381)
- `eval_macro_let` (lines 383-418)
- `eval_macro_binding` (lines 420-452)
- `eval_macro_setq` (lines 454-474)
- `eval_macro_car_cdr` (lines 476-496)
- `eval_macro_nth` (lines 498-524)
- `eval_quasiquote_form` (lines 526-563)
- `eval_quasiquote_prefixed` (lines 565-589)
- `eval_quasiquote_list` (lines 591-604)
- `eval_quasiquote_dotted_list` (lines 606-625)
- `eval_quasiquote_vector` (lines 627-644)
- `eval_quasiquote_list_parts` (lines 646-673)
- `append_forms` (the impl method, lines 675-683)

Also delete the following free functions (no longer used by any remaining code):
- `cons_form` (lines 1131-1144)
- `car_form` (lines 1146-1155)
- `cdr_form` (lines 1157-1171)
- `append_forms` (free function, lines 1173-1185)
- `proper_list_elements` (lines 1187-1193)
- `flush_quasiquote_segment` (lines 1195-1203)

Keep these functions (still used by built-in rewrites and other code):
- `list_head_symbol`, `is_nil`, `nil_form`, `symbol_form`, `quote_form`
- `function_quote_form`, `list_form`, `macro_defalias_form`
- `macro_lambda_params_form`, `lower_macro_body`, `fixnum_value`
- `build_if_let_form`, `generated_if_let_name`
- `parse_if_let_bindings`, `parse_if_let_binding`

Also delete the old `MacroEnv` struct and impl (lines 953-966) — it's replaced by the one in `expand_eval.rs`.

Note: Check that `nil_form` and `list_form` are not also deleted — they're still used by `expand_push`, `expand_pop`, and other built-in rewrites. Keep them.

- [ ] **Step 3: Remove unused import of compile_source from expand.rs**

The `use crate::compile_source;` import (line 5) was used by the old `eval_macro_list` for evaluating nested function calls. Check if it's still referenced anywhere in expand.rs after the deletions. If not, remove it.

- [ ] **Step 4: Run all tests**

Run: `cd neovm-compiler && cargo nextest run`
Expected: all tests pass — the 5 existing integration tests in `expand.rs` still pass because they test the public `expand_forms` API which hasn't changed. All new tests in `expand_value` and `expand_eval` pass.

- [ ] **Step 5: Commit**

```bash
git add neovm-compiler/src/expand.rs
git commit -m "Wire MacroEval into Expander, remove old eval_macro chain"
```

---

### Task 4: Verify end-to-end with existing integration tests

**Files:** None modified

- [ ] **Step 1: Run full test suite**

Run: `cd neovm-compiler && cargo nextest run`
Expected: all tests pass (90 original + ~33 new from Tasks 1-2).

- [ ] **Step 2: Run cargo check**

Run: `cd neovm-compiler && cargo check`
Expected: compiles without warnings.

- [ ] **Step 3: Final commit (if any cleanup needed)**

Only if there were compilation warnings or test issues to fix.
