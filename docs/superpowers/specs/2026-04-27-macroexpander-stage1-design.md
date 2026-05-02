# Macroexpander Stage 1: MacroValue + Surface-Form Evaluator

## Problem

The current `expand.rs` has a hand-rolled `eval_macro_expr` that operates on
`SurfaceForm` directly — conflating "parsed syntax" with "computed value." This
makes the evaluator grow harder with each new form and prevents it from handling
the operations real macro bodies need (list reconstruction, string ops,
predicates on computed values).

## Approach

Introduce a `MacroValue` type that separates code-to-evaluate from
computed-values, then rewrite the macro-time evaluator to operate on it.

This is Stage 1 of a three-stage bootstrap:

- Stage 1 (this spec): MacroValue type + surface-form evaluator
- Stage 2 (future): Extend the compiler pipeline value system beyond i64
- Stage 3 (future): Self-hosting — compile macro bodies through the pipeline

The `MacroValue` type carries forward through all three stages.

## MacroValue Type

```rust
enum MacroValue {
    Nil,
    Int(i64),
    Symbol(String),
    String(String),
    Cons(Rc<MacroCons>),
    Vector(Vec<MacroValue>),
}

struct MacroCons {
    car: MacroValue,
    cdr: MacroValue,
}
```

Design decisions:

- **No Bool variant.** Elisp `t` is the symbol `t`, `nil` is both false and
  empty list. `is_truthy()` returns `!is_nil()`.
- **Rc not Box.** `(append a b)` shares b's tail. Cheap cloning without
  deep-copying list spines.
- **No RefCell.** Macro-time values are immutable. No setcar/setcdr at
  macro time.
- **No Float.** Macros don't do float arithmetic in practice.

Helper methods: `is_nil`, `is_truthy`, `is_cons`, `is_list`, `is_symbol`,
`is_string`, `is_int`, `cons`, `list`, `car`, `cdr`, `to_vec`,
`as_symbol_name`, `as_int`.

## Conversion Boundary

Two functions at the edge between syntax and values:

**`surface_to_value(form: &SurfaceForm) -> MacroValue`**

Converts macro arguments (unevaluated syntax) to MacroValues. No evaluation —
purely a representation change. Symbols become `MacroValue::Symbol`, lists
become `MacroValue::Cons` chains, nil becomes `MacroValue::Nil`.

**`value_to_surface(value: &MacroValue, span: Span) -> SurfaceForm`**

Converts the expansion result back to syntax for the compiler pipeline.
Proper lists become `SurfaceForm::List`, dotted lists become
`SurfaceForm::DottedList`, vectors become `(vector ...)` list forms.

Data flow:

```
macro args (SurfaceForm)
  → surface_to_value
  → MacroValue args
  → eval(macro_body, env)
  → MacroValue result
  → value_to_surface
  → SurfaceForm (expanded code, fed back into expansion loop)
```

## Evaluator Architecture

`MacroEval` struct with `eval(&mut self, form: &SurfaceForm, env: &mut MacroEnv) -> Result<MacroValue, ()>`.

Dispatch pattern:

1. **Atoms**: self-evaluating (numbers, strings, keywords) or symbol lookup in
   `MacroEnv`.
2. **Special forms**: `quote`, `if`, `cond`, `and`, `or`, `let`, `let*`,
   `setq`, `progn`, `while`, `backquote`/quasiquote.
3. **Named function calls**: dispatch on head symbol to the appropriate
   handler. Returns `Err(())` with a diagnostic for unknown operations.

### Supported operations

| Category | Operations |
|---|---|
| List construction | `cons`, `list`, `append`, `list*` |
| List access | `car`, `cdr`, `nth`, `cadr`, `caddr`, `last`, `butlast` |
| Predicates | `null`/`not`, `consp`, `listp`, `symbolp`, `stringp`, `numberp`, `eq`/`eql`, `equal` |
| Conditionals | `if`, `cond`, `and`, `or` |
| Binding | `let`, `let*`, `setq` |
| Arithmetic | `+`, `-`, `*`, `=`, `<`, `>`, `<=`, `>=`, `/=` |
| Symbols | `symbol-name`, `intern`, `make-symbol` |
| Strings | `concat`, `substring`, `string=`, `format`, `length` |
| Quoting | `quote`, `backquote`/quasiquote with `,` and `,@` |
| Misc | `progn`, `while`, `length`, `error` |

### Out of scope for Stage 1

- Closures / lambda / mapcar / mapc
- Buffer operations
- cl-lib (handled by expanding cl macros in a prior pass)
- Destructuring bind (added incrementally as needed)

## MacroEnv

```rust
struct MacroEnv {
    bindings: HashMap<String, MacroValue>,
}
```

Methods: `bind`, `lookup`, `unbind` (or use scoped child envs for let).

The `macros` field (currently on `Expander`) stays on `Expander` — the
evaluator doesn't own macro definitions, it just evaluates bodies.

## Quasiquote Simplification

The current quasiquote code (~180 lines of SurfaceForm manipulation) becomes
straightforward: produce `MacroValue::Cons` trees directly, let
`value_to_surface` convert the result. No more `append_forms`, `cons_form`,
`flush_quasiquote_segment` helpers.

## Integration with Expander

`Expander.invoke_macro` changes from calling the `eval_macro_*` chain to:

```rust
fn invoke_macro(&mut self, def: &MacroDef, args: &[SurfaceForm], span: Span) -> Option<SurfaceForm> {
    let arg_values: Vec<MacroValue> = args.iter().map(surface_to_value).collect();
    let mut env = self.bind_macro_params(&def.params, &arg_values);
    match self.macro_eval.eval_progn(&def.body, &mut env) {
        Ok(result) => Some(value_to_surface(&result, span)),
        Err(()) => None,
    }
}
```

`MacroDef` struct unchanged. `bind_macro_params` rewritten for `MacroValue`.

### Deleted code (~650 lines)

All `eval_macro_*` methods and surface-construction helpers:
`eval_macro_expr`, `eval_macro_list`, `eval_macro_if`, `eval_macro_and`,
`eval_macro_or`, `eval_macro_let`, `eval_macro_binding`, `eval_macro_setq`,
`eval_macro_car_cdr`, `eval_macro_nth`, all `eval_quasiquote_*`,
`cons_form`, `car_form`, `cdr_form`, `append_forms` (module-level),
`flush_quasiquote_segment`, `proper_list_elements`.

### Net size change

~650 lines deleted from expand.rs, ~800 lines added in new modules.
expand.rs net shrinks by ~200 lines.

## Module Structure

```
neovm-compiler/src/
  expand.rs          — Expander struct, built-in rewrites, defmacro registration (shrinks)
  expand_value.rs    — MacroValue, MacroCons, surface_to_value, value_to_surface
  expand_eval.rs     — MacroEval evaluator, MacroEnv
```

## Testing

### expand_value.rs

- `surface_atom_round_trips` — each atom type converts and converts back
- `surface_list_to_value_and_back` — proper list round-trip
- `dotted_list_preserves_tail` — dotted list ↔ Cons with non-Nil cdr
- `nested_list_round_trip` — deeply nested structure
- `vector_round_trip` — vector converts to (vector ...) surface form

### expand_eval.rs

- `evals_self_evaluating_atoms` — numbers, strings, nil, t, keywords
- `evals_symbol_lookup` — bound and unbound symbols
- `evals_if_branches` — truthy and falsy conditions, else-optional
- `evals_let_bindings` — parallel and sequential
- `evals_setq` — variable mutation
- `evals_while` — loop termination
- `evals_list_operations` — cons, car, cdr, list, append, nth
- `evals_predicates` — null, consp, symbolp, eq, equal
- `evals_arithmetic` — +, -, *, comparisons
- `evals_quasiquote_with_unquote` — `,expr` substitution
- `evals_quasiquote_with_splice` — `,@expr` in list context
- `evals_nested_quasiquote` — depth tracking
- `evals_string_operations` — concat, substring, string=
- `evals_format` — basic format strings
- `evals_symbol_operations` — symbol-name, intern
- `reports_unknown_function` — diagnostic for unsupported operations
- `reports_error_call` — `(error "msg")` produces diagnostic

### expand.rs (integration, existing tests updated)

All 5 existing tests pass unchanged — they test `expand_forms` public API.

## Dependencies

No new crate dependencies. `Rc` and `RefCell` from `std`.

## Stages 2 and 3 Connection

The `MacroValue` type designed here is the same type that becomes
`CompileValue` in Stage 2 (extending the pipeline beyond i64) and the runtime
value type in Stage 3. No throwaway work.

Stage 2 widens `RegInstKind::ConstI64(i64)` to `RegInstKind::Const(CompileValue)`
and updates the RegIR interpreter to handle cons/symbol/string operations.

Stage 3 compiles macro bodies through the full pipeline and executes them via
the RegIR interpreter instead of the surface-form evaluator. The evaluator from
this spec gets retired but `MacroValue`/`CompileValue` carries forward.
