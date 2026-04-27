# neovm-compiler Crate Choices

`neovm-compiler` should prefer mature open-source infrastructure for generic
compiler mechanics. We should hand-write only the Elisp-specific semantic
rules that no general Rust crate can know.

## Current Dependencies

### `logos`

Used for lexing `.el` source into reader tokens.

Why:

- Mature token enum derive.
- Fast lexer generation.
- Keeps tokenization separate from parser semantics.

Source: <https://docs.rs/logos/latest/logos/>

### `rowan`

Used for the lossless syntax tree.

Why:

- Mature lossless green/red tree implementation from the rust-analyzer
  ecosystem.
- Preserves comments, whitespace, and exact token text.
- Lets us keep a lossless syntax tree while also building typed surface forms.

Source: <https://docs.rs/rowan/latest/rowan/>

### `num_enum`

Used for `SyntaxKind` conversion to and from rowan raw syntax kinds.

Why:

- Avoids handwritten discriminant conversion tables.
- Provides safe fallback with `#[num_enum(default)]`.

Source: <https://docs.rs/num_enum/latest/num_enum/>

### `cranelift-entity`

Used for typed compiler IDs and dense maps.

Why:

- Mature compiler-oriented entity reference infrastructure.
- `PrimaryMap` and `SecondaryMap` prevent accidental cross-indexing.
- Better long-term fit for SSA blocks, values, registers, and safepoints than
  hand-rolled integer IDs.

Source: <https://docs.rs/cranelift-entity/latest/cranelift_entity/>

### `cranelift-codegen` and `cranelift-frontend`

Used for the planned native backend.

Why:

- Mature Rust compiler backend from the Bytecode Alliance.
- Owns Cranelift IR verification, instruction selection, machine lowering, and
  physical register allocation.
- Lets NeoVM avoid a custom native code generator while keeping Elisp semantics
  explicit in HIR/SSA/Register IR.

Source: <https://cranelift.dev/>
Source: <https://docs.rs/cranelift-frontend/latest/cranelift_frontend/>

### `indexmap`

Used for deterministic semantic maps such as scopes.

Why:

- Stable iteration order.
- Useful for reproducible IR dumps and tests.

Source: <https://docs.rs/indexmap/latest/indexmap/>

### `ariadne`

Used for diagnostic rendering.

Why:

- Mature compiler diagnostic renderer.
- Supports spans, labels, notes, and multi-file rendering.

Source: <https://docs.rs/ariadne/latest/ariadne/>

### `insta`

Reserved for snapshot/golden tests.

Why:

- Mature snapshot testing workflow.
- Good fit for stable syntax/HIR/SSA/Register IR dumps.

Source: <https://docs.rs/insta/latest/insta/>

## Later Candidates

### `salsa`

Use later for incremental compilation and IDE-like recomputation.

Do not add until the compiler has stable query boundaries.

Source: <https://rustc-dev-guide.rust-lang.org/queries/salsa.html>

### `cranelift-jit`

Use later when native-code JIT work starts.

Do not add during front-end/IR bootstrap.

Source: <https://docs.rs/crate/cranelift-jit/latest>

### `regalloc2`

Do not add directly while the native backend is Cranelift.

Reason:

- Cranelift already uses register allocation internally.
- A direct `regalloc2` dependency is useful only if NeoVM later builds a custom
  machine-code backend that bypasses Cranelift.

Source: <https://docs.rs/regalloc2/latest/regalloc2/>

### `chumsky` or `winnow`

Possible parser-combinator choices if the reader parser grows complex.

Current stance:

- Use `logos` plus a small parser for S-expressions now.
- Revisit parser combinators only if error recovery becomes painful.

Sources:

- <https://docs.rs/chumsky/latest/chumsky/>
- <https://docs.rs/winnow/latest/winnow/>

### `tree-sitter`

Possible later for editor-grade incremental parsing.

Current stance:

- Not needed for compiler bootstrap.
- `rowan` gives us lossless syntax without maintaining a tree-sitter grammar.

Source: <https://docs.rs/tree-sitter/latest/tree_sitter/>

## What We Still Own

No mature Rust crate can provide GNU Emacs Elisp semantics for us. We must own:

- Dynamic vs lexical binding.
- Special declarations.
- Buffer-local and default-value symbol access.
- Separate value and function namespaces.
- Macroexpansion semantics.
- `catch`, `throw`, `condition-case`, and `unwind-protect`.
- Calls that may signal, quit, allocate, GC, or reenter Lisp.
- Precise root metadata and safepoint meaning.

The rule is:

```text
Use crates for infrastructure.
Own the Elisp semantics.
```
