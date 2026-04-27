# neovm-compiler Design

`neovm-compiler` is the long-term standalone Elisp compiler and VM pipeline
for NeoVM. Its first responsibility is to compile `.el` source files into
inspectable intermediate representations. Runtime integration with the current
NeoMacs evaluator is deliberately out of scope for the initial crate.

The design target is:

```text
.el source
 ↓
Reader / parser
 ↓
Surface AST: raw S-expressions with source spans
 ↓
Expander: macroexpand and compiler-macro boundary
 ↓
HIR: semantic Elisp AST
 ↓
SSA CFG: optimization IR with basic blocks, values, and effects
 ↓
Register IR: executable low-level VM/JIT IR
 ↓
Execution: register interpreter and Cranelift native backend
```

## Goals

- Compile `.el` files through a clean, testable IR pipeline.
- Preserve GNU Emacs semantics instead of optimizing around them.
- Keep the compiler independent from `neovm-core` while the architecture is
  being built.
- Make each stage inspectable through stable pretty-printers and verifiers.
- Model lexical binding, dynamic binding, buffer-sensitive symbol access,
  nonlocal control flow, effects, and GC safepoints explicitly.
- Keep Register IR backend-independent while using Cranelift for native
  codegen rather than hand-writing a machine-code backend.

## Non-Goals

- No dependency on `neovm-core` in the initial crate.
- No direct integration with the current NeoMacs evaluator in the first
  milestones.
- No broad Elisp support by silently falling back inside the compiler. Unknown
  forms should produce explicit diagnostics.
- No direct machine-code backend before the Register IR and safepoint metadata
  are stable. Native JIT work should go through Cranelift first.
- No attempt to treat dynamic variables or buffer-local variables as ordinary
  SSA locals.

## GNU Emacs Oracle

GNU Emacs remains the semantic reference:

```text
/home/exec/Projects/github.com/emacs-mirror/emacs
/home/exec/Projects/github.com/emacs-mirror/emacs/src/emacs
```

Important source files:

```text
src/eval.c
src/bytecode.c
src/lread.c
src/data.c
src/fns.c
src/buffer.c
lisp/emacs-lisp/bytecomp.el
lisp/emacs-lisp/cconv.el
lisp/emacs-lisp/macroexp.el
```

The compiler should encode behavior learned from GNU, but oracle execution
should live in tests or tooling around the compiler rather than in the core IR
types.

## Crate Boundary

`neovm-compiler` owns compiler vocabulary:

```text
SourceFile
Span
SymbolId
ConstId
LocalId
BlockId
ValueId
RegId
SafepointId
Diagnostic
SurfaceForm
HirExpr
SsaModule
SsaFunction
RegModule
RegFunction
```

The crate should use compiler-owned identifiers and constants instead of
depending on a runtime value representation. Runtime adapters can be added
later outside this boundary.

## Proposed Modules

```text
src/lib.rs
src/ids.rs
src/source.rs
src/diagnostic.rs
src/surface.rs
src/reader.rs
src/expand.rs
src/hir.rs
src/effects.rs
src/ssa.rs
src/liveness.rs
src/regir.rs
src/safepoint.rs
src/lower.rs
src/verify.rs
src/pretty.rs
src/interp.rs
```

Module responsibilities:

- `ids.rs`: typed IDs for symbols, constants, locals, blocks, SSA values,
  registers, and safepoints.
- `source.rs`: source files, byte offsets, spans, and file-local metadata.
- `diagnostic.rs`: errors, warnings, notes, and span labels.
- `surface.rs`: raw Lisp forms produced by the reader.
- `reader.rs`: `.el` text to surface forms.
- `expand.rs`: macroexpansion boundary. Initially conservative or stubbed.
- `hir.rs`: Elisp semantic AST after expansion.
- `effects.rs`: effect classification for calls and runtime operations.
- `ssa.rs`: CFG-shaped SSA IR.
- `liveness.rs`: SSA value liveness for precise safepoint root sets.
- `regir.rs`: low-level register execution IR.
- `safepoint.rs`: safepoint IDs, live root maps, and future stack-map metadata.
- `lower.rs`: lowering passes between IR layers.
- `verify.rs`: invariants for every IR stage.
- `pretty.rs`: stable textual dumps for tests and debugging.
- `interp.rs`: small Register IR interpreter for runtime-free lexical/control
  flow subsets, with explicit diagnostics for runtime-dependent operations.

## Reader And Surface AST

The reader handles syntax only. It must not classify lexical vs dynamic
variables or decide call semantics.

Initial reader responsibilities:

- Symbols, `nil`, and `t`.
- Integers, floats, strings, and characters.
- Lists and dotted lists.
- Vectors.
- Quote syntax: `'x`.
- Function quote syntax: `#'x`.
- Backquote, comma, and comma-at as surface forms.
- Comments.
- Source spans.
- File-local `lexical-binding` detection.

Example surface shape:

```text
SurfaceForm {
  kind: List([Symbol("if"), Symbol("x"), Int(1), Int(2)]),
  span: ...
}
```

## Macroexpansion Boundary

Macroexpansion must happen before final semantic HIR. Macros can introduce
special forms, declarations, control flow, dynamic binding, and arbitrary
calls.

The first implementation can use a conservative `expand` stage:

```text
Surface AST
 ↓
ExpansionInput
 ↓
ExpandedForm
```

Early milestones may leave user macros unsupported and lower only already-core
forms. That is acceptable if unsupported cases produce diagnostics. Silent
miscompilation is not acceptable.

## HIR

HIR is the semantic Elisp AST. It should be structured and close enough to
Elisp to preserve meaning, but explicit enough that lowerings do not rediscover
semantics from raw lists.

HIR should encode:

- Constants and quoted forms.
- Lexical variable reads and writes.
- Dynamic/special variable operations.
- Symbol value operations.
- Function namespace vs value namespace.
- Calls by value and calls by name.
- `if`, `progn`, `let`, `let*`, `lambda`, function-quoted lambda,
  `defun`, and `setq`.
- `catch`, `throw`, `condition-case`, and `unwind-protect`.
- Declaration effects such as special variables.
- Source spans.

Important rule:

```text
Lexical locals may become SSA values.
Dynamic variables and buffer-sensitive symbol values remain runtime operations.
```

Example:

```elisp
(let ((x 1)) (+ x 2))
```

Possible HIR:

```text
Let {
  mode: Lexical,
  bindings: [(x, Const(1))],
  body: CallNamed("+", [LexicalGet(x), Const(2)])
}
```

Special or dynamic variables must lower differently:

```text
DynGet(symbol)
DynSet(symbol, value)
DynBind(symbol, value, body)
```

Buffer-sensitive symbol access should stay explicit:

```text
SymGet(symbol)
SymSet(symbol, value)
DefaultSymGet(symbol)
DefaultSymSet(symbol, value)
```

Those operations may observe dynamic bindings, buffer-local bindings, symbol
aliases, forwarded variables, watchers, and default values.

## Effects

The optimizer must know when it is unsafe to reorder, eliminate, or speculate.
Effects should be explicit and conservative.

Initial effect classes:

```text
Pure
ReadLexical
ReadSymbol
WriteSymbol
BindDynamic
Allocate
Call
MayGc
MaySignal
MayThrow
MayQuit
MayReenterElisp
BlockingIo
Unknown
```

Calls default to conservative effects unless the compiler has trustworthy
metadata.

## SSA CFG

SSA is represented as a control-flow graph. Use block parameters instead of
separate phi instructions.

```text
Module {
  functions: Vec<Function>
  entry: FunctionId
}

Function {
  blocks: Vec<Block>
}

Block {
  params: Vec<ValueId>
  instructions: Vec<Inst>
  terminator: Terminator
}
```

Branch arguments replace phi nodes:

```text
entry:
  v0 = ...
  branch_if_nil v0, else_block(), then_block()

then_block:
  v1 = ...
  jump merge(v1)

else_block:
  v2 = ...
  jump merge(v2)

merge(v3):
  return v3
```

SSA must model exceptional and nonlocal control flow:

- `throw`.
- `signal`.
- `condition-case`.
- `unwind-protect`.
- quit checks.
- calls that can reenter Elisp.
- calls that can trigger GC.

This matters for both correctness and precise root maps.

## Register IR

Register IR is the execution contract for the interpreter and backend lowering.
It is lower-level than SSA and should make VM state explicit.

Register IR should contain:

- Virtual registers. Physical register allocation belongs to Cranelift unless
  we later build a custom native backend.
- Constants.
- Direct jumps.
- Calls.
- Runtime operations for symbol access and dynamic binding.
- Explicit safepoints.
- Live register metadata at safepoints.
- Frame layout metadata.
- Deopt or reconstruction metadata later.

Register IR has a module container parallel to SSA so every top-level function
can keep a stable function ID through interpreter and backend-oriented lowering.

The register interpreter remains useful for portability, debugging, and
semantic validation. Native execution should lower to Cranelift IR, letting
Cranelift own instruction selection, physical register allocation, verification,
and machine-code emission.

## Cranelift Backend

Cranelift is the preferred native backend. The compiler should lower the safe,
explicit subset of SSA or Register IR into Cranelift IR, then use Cranelift for
optimization, register allocation, stack maps, and eventual JIT/AOT emission.

Initial Cranelift lowering is intentionally conservative:

- Treat Lisp values as an opaque `i64` carrier until the runtime value ABI is
  finalized.
- Lower constants, lexical block parameters, lexical mutation through Cranelift
  frontend variables, direct jumps, conditional branches, and returns.
- Lower unreachable SSA terminators to explicit Cranelift traps instead of
  silently assuming control can continue.
- Lower direct named calls through a declared runtime ABI first. Primitive
  inlining can come later after type and semantic metadata exist.
- Lower symbol value reads and writes through declared runtime ABI calls so
  dynamic and buffer-local semantics stay in the runtime.
- Lower `funcall` and `apply` through declared runtime ABI calls so indirect
  function semantics stay in the runtime.
- Lower string/float constants and quoted forms through runtime materialization
  calls so allocation, interning, and object identity stay runtime-owned.
- Lower lambda values through runtime materialization calls over compiler-owned
  lambda template tables and explicit capture arguments. Callable code
  registration stays runtime-owned until the execution ABI is connected.
- Lower lambda templates into standalone SSA/Cranelift functions with entry
  parameters for captures followed by lambda call parameters.
- Lower SSA modules into Cranelift modules function-by-function while retaining
  per-function runtime ABI metadata.
- Lower scoped dynamic bindings through runtime push/pop calls. Parallel `let`
  evaluates all initializers before installing bindings; `let*` installs each
  binding before the next initializer.
- Lower captured mutable lexical bindings through runtime lexical-cell
  allocation and get/set calls. Lambda materialization receives the cell
  payload so mutation remains visible through closures.
- Reject nonlocal exits until the runtime ABI and precise safepoint/stack-map
  contract exist.
- Do not depend on `regalloc2` directly while using Cranelift; Cranelift owns
  physical register allocation internally.

The initial generated function ABI is:

```text
compiled_elisp_fn(vmctx: i64, arg0: i64, ...) -> i64
```

Direct named calls use arity-specialized imports:

```text
__neomacs_rt_call_named_N(vmctx: i64, symbol_id: i64, arg0: i64, ...) -> i64
```

Symbol value access uses fixed imports:

```text
__neomacs_rt_symbol_get(vmctx: i64, symbol_id: i64) -> i64
__neomacs_rt_symbol_set(vmctx: i64, symbol_id: i64, value: i64) -> i64
```

Indirect calls use arity-specialized imports:

```text
__neomacs_rt_funcall_N(vmctx: i64, callee: i64, arg0: i64, ...) -> i64
__neomacs_rt_apply_N(vmctx: i64, callee: i64, arg0: i64, ...) -> i64
```

Runtime materialization uses compiler-owned table indexes or immediate payloads:

```text
__neomacs_rt_string_const(vmctx: i64, string_id: i64) -> i64
__neomacs_rt_float_const(vmctx: i64, bits: i64) -> i64
__neomacs_rt_quote(vmctx: i64, quote_id: i64) -> i64
__neomacs_rt_function_quote(vmctx: i64, quote_id: i64) -> i64
__neomacs_rt_lambda_N(vmctx: i64, lambda_id: i64, capture0: i64, ...) -> i64
__neomacs_rt_make_lexical_cell(vmctx: i64, initial: i64) -> i64
__neomacs_rt_lexical_cell_get(vmctx: i64, cell: i64) -> i64
__neomacs_rt_lexical_cell_set(vmctx: i64, cell: i64, value: i64) -> i64
```

`symbol_id` is a compiler-owned interned symbol key. This is not the final
runtime object representation; it is the bridge needed before runtime symbol
tables, precise stack maps, and JIT execution are connected.

`string_id` and `quote_id` are also compiler-owned keys into per-function
metadata tables. Float constants pass the exact IEEE-754 bit pattern as an
opaque payload for the runtime to materialize.

`lambda_id` is a compiler-owned key into a lambda template table containing the
lambda parameters, capture names, capture modes, declarations, and HIR body.
Capture mode is `value` for immutable payloads and `cell` for mutable lexical
bindings that must preserve GNU Emacs closure mutation semantics. The arity
suffix is the number of captured lexical payloads passed after `lambda_id`.
This records semantic closure shape and makes capture roots visible to
safepoint metadata without pretending the compiler can already register
callable machine code.

Captured mutable lexical variables now lower through compiler-visible cell
operations. JIT execution still needs runtime implementations for those imports,
but the compiler-side ABI is already shaped so `cell` payloads are binding-cell
pointers without changing safepoint/root visibility.

Dynamic binding uses fixed imports:

```text
__neomacs_rt_bind_dynamic(vmctx: i64, symbol_id: i64, value: i64)
__neomacs_rt_unbind_dynamic(vmctx: i64, count: i64)
```

The compiler emits `unbind_dynamic` on normal scope exit. Nonlocal exits still
need explicit unwind-edge lowering before Cranelift can support `throw`,
`condition-case`, or `unwind-protect`.

Mutable lexical locals lower to Cranelift frontend variables, not ad-hoc
compiler-side value maps. Each declared lexical variable is marked as needing
Cranelift stack-map metadata so future GC integration can spill/reload roots
around safepoints using Cranelift's supported mechanism.

Every Cranelift runtime ABI call is also a compiler safepoint. The current
metadata records:

- The Cranelift call instruction.
- The runtime call kind.
- Liveness-pruned SSA/Cranelift Lisp-value roots used by or live across that
  call.

This is still conservative about value types because all Lisp values currently
use an opaque `i64` carrier, but dead SSA values are no longer retained in
safepoint root metadata. The Cranelift lowerer also marks each liveness-pruned
CLIF root with `declare_value_needs_stack_map`, allowing Cranelift's safepoint
spiller to attach `stack_map=[...]` metadata to runtime calls. Later work should
thread those emitted stack maps into the runtime/JIT registration path.

## Safepoints And GC Metadata

Modern GC support depends on compiler metadata. The compiler must eventually
emit precise root maps for every place execution can allocate, call out, block,
or poll.

Safepoints should be attached to:

- Calls that may allocate or run Lisp.
- Runtime operations that may allocate.
- Backward branches or loop polls.
- Explicit interrupt/quit checks.
- Places where the interpreter may block.

Metadata should answer:

```text
At safepoint S:
  Which registers contain Lisp roots?
  Which frame slots contain Lisp roots?
  Which values reconstruct source-level state?
  Which dynamic/unwind state is active?
```

This is the compiler-side foundation for low-pause, precise GC.

## Correctness Constraints From Elisp

The compiler must preserve these semantics:

- Lexical binding is file/function dependent.
- Dynamic variables are visible through symbol-value operations.
- Special declarations can force dynamic binding under lexical mode.
- Buffer-local variables depend on current buffer.
- Default values differ from current buffer-local values.
- Symbols have separate value and function namespaces.
- Function cells can change at runtime.
- Advice and autoloads can affect call behavior.
- `unwind-protect` cleanup must run across local and nonlocal exits.
- `catch`/`throw` and `condition-case` are not ordinary branches.
- Calls may signal, throw, quit, allocate, and reenter Lisp.
- Macroexpansion can run arbitrary Lisp and produce arbitrary forms.

## Initial Supported Subset

The first milestone should compile only a small subset:

```elisp
;;; -*- lexical-binding: t; -*-

(defun add2 (x y)
  (+ x y))

(defun fact (n)
  (if (<= n 1)
      1
    (* n (fact (1- n)))))
```

Initial supported forms:

- constants.
- symbols.
- `quote`.
- `function`.
- `if`.
- `progn`.
- `let` and `let*`.
- `lambda`.
- `defun`.
- `setq`.
- ordinary function calls.
- file-local `lexical-binding`.

Unsupported forms should return diagnostics.

## Verification

Each IR stage should have a verifier.

Surface verifier:

- Spans are valid.
- Dotted lists are represented explicitly.
- Reader-produced forms are structurally valid.

HIR verifier:

- Lexical locals are declared before use.
- Dynamic and symbol operations are explicit.
- HIR nodes carry source spans.
- Unsupported forms are diagnostics, not placeholder nodes.

SSA verifier:

- Block IDs exist.
- Terminator targets exist.
- Branch argument counts match target block parameter counts.
- Values dominate uses.
- Effectful instructions are ordered.
- Exceptional edges are represented.

Register IR verifier:

- Registers are defined before use.
- Jumps target valid labels.
- Calls and safepoints have liveness-pruned live-root metadata.
- Frame metadata is internally consistent.

## Pretty Dumps

Stable textual dumps are required for development. Tests should compare
expected output for small examples.

Suggested API:

```text
compile_source(source) -> CompileArtifact

CompileArtifact {
  surface,
  hir,
  ssa,
  regir,
  diagnostics,
}
```

Suggested dump modes:

```text
--dump-surface
--dump-hir
--dump-ssa
--dump-regir
--dump-all
```

## Milestones

1. Create standalone crate skeleton with IDs, diagnostics, source spans, and
   pretty-print infrastructure.
2. Implement reader for the initial `.el` subset.
3. Implement HIR lowering for core forms under lexical binding.
4. Implement HIR verifier and golden pretty-printer tests.
5. Lower HIR to CFG-shaped SSA with block parameters.
6. Implement SSA verifier and simple optimization passes.
7. Lower SSA to Register IR.
8. Implement Register IR verifier and safepoint metadata skeleton.
9. Add conservative Cranelift IR lowering for the safe SSA/Register IR subset.
10. Add a register interpreter for the supported subset.
11. Add GNU oracle tests around behavior, initially outside the core compiler.

## Design Principle

The compiler should make hard Elisp semantics explicit early, then optimize
only what is safe. Fast lexical code should become SSA/register code. Dynamic,
buffer-local, reflective, and reentrant behavior should remain explicit runtime
operations until the compiler can prove stronger facts.
