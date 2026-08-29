# Architecture

NEO Emacs rebuilds GNU Emacs as a layered Rust system: the Elisp runtime is a
self-contained core, editor subsystems are independent modules communicating through
defined APIs, and the rendering engine runs on a separate GPU thread.

## Current State

The shipped `neomacs` binary contains no C core. The Elisp runtime (evaluator,
bytecode VM, GC, portable dump), the editor subsystems (buffers, windows, frames,
keyboard, processes), the layout engine, and the wgpu rendering engine all run in
Rust. GNU Emacs serves as the behavioral test oracle: oracle suites, TUI grid
comparison tests, and GUI parity checks continuously diff NEO Emacs against GNU Emacs
to keep the rewrite honest.

## Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        NEO Emacs (Rust)                         │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  Elisp Runtime Core                      │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │ Evaluator   │  │ Bytecode VM │  │ GC/Allocator│       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │ LispObject  │  │Symbol Table │  │ Type System │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                             │                                   │
│                             ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                       Runtime API                        │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │register_type│  │register_root│  │define_func  │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │  run_hook   │  │  specbind   │  │signal_error │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                             │                                   │
│                             ▼                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                     Editor Modules                        │  │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  ┌───────┐│  │
│  │  │ Buffer │  │ Window │  │ Frame  │  │Keyboard│  │Process││  │
│  │  └────────┘  └────────┘  └────────┘  └────────┘  └───────┘│  │
│  │  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  ┌───────┐│  │
│  │  │ Font   │  │ Image  │  │File IO │  │ Reader │  │ Data  ││  │
│  │  └────────┘  └────────┘  └────────┘  └────────┘  └───────┘│  │
│  └───────────────────────────────────────────────────────────┘  │
│                             │                                   │
│                             ▼                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Rendering Engine                       │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │  │
│  │  │Layout Engine│  │wgpu Renderer│  │ Animations  │        │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │  │
│  │  │    winit    │  │   WebKit    │  │ GStreamer   │        │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘        │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                      Threading                            │  │
│  │   ┌────────────┐                       ┌────────────┐     │  │
│  │   │EmacsThread │                       │RenderThread│     │  │
│  │   └────────────┘                       └────────────┘     │  │
│  │      │                                      ▲             │  │
│  │      ├── FrameGlyphBuffer (crossbeam) ──────┘             │  │
│  │      └── InputEvent (crossbeam) ────────────────────┐     │  │
│  │                                                     │     │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                       Backends                            │  │
│  │   ┌──────────┐         ┌──────────┐       ┌──────────┐    │  │
│  │   │  Vulkan  │         │  Metal   │       │ DX12/GL  │    │  │
│  │   └──────────┘         └──────────┘       └──────────┘    │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Design Principles

- **Elisp Runtime Core** is a self-contained Rust crate. It owns LispObject, the
  evaluator, bytecode VM, GC, specpdl, and symbol table. It does NOT know about
  buffers, windows, frames, or any editor concept.
- **Runtime API** is a trait-based interface. Editor modules register their types
  (with GC trace descriptors), roots, and primitives. The GC traces registered
  types generically — no hardcoded `mark_kboards()` or `mark_terminals()`.
- **Editor Modules** are independent. Each owns its data structures and exposes
  them to Lisp through the Runtime API. Modules do not reach into each other's
  internals.
- **Rendering Engine** runs on a separate GPU thread, communicating via crossbeam
  channels (`FrameGlyphBuffer` down, `InputEvent` up).

### Elisp core source ownership

GNU Emacs' `src/` layout reflects its history as a collection of large C
translation units. Neomacs keeps the behavioral boundaries but uses a directory
per Rust subsystem:

```text
neovm-core/src/emacs_core/
├── commands/  ├── display/  ├── editing/  ├── lisp/
├── runtime/   ├── system/   ├── text/     └── tests/
```

Each subsystem directory owns its implementation, private helpers, and tests.
The root `mod.rs` is the stable facade: it maps physical ownership to existing
paths such as `crate::emacs_core::eval`, so reorganizing files does not create a
workspace-wide API migration. New production Rust files must live below an
owning subsystem directory; architectural tests reject loose root and domain
files. See `neovm-core/src/emacs_core/README.md` for the complete rules.

### Rust-backed Elisp functions

Rust-backed Elisp functions are declared with `SubrSpec`. A declaration keeps
the Lisp name, Rust function shape, observable arity, evaluator dispatch kind,
interactive contract, and startup policy together. `Context::register_subr` is
the only installation path into the static `SymId` registry used by the
evaluator, bytecode VM, JIT, and portable dumps.

Subsystem-owned declarations live beside their implementations and are exposed
by a consistently named `register_subrs` function. Startup calls those
registrars explicitly so ordering remains reviewable. The not-yet-localized GNU
compatibility surface is isolated as a declaration-only manifest in
`builtins/subrs/mod.rs`; new subsystem work does not add registrations there.

Private adapters in localized subsystems use names from their Rust domain
vocabulary. They do not repeat the Lisp identity with a `builtin_` prefix: the
descriptor already carries the Lisp-visible name. GNU-port names in the legacy
manifest remain unchanged until their declarations move to the owning module.

## Why Rust?

- **Memory safety** without garbage collection
- **Zero-cost abstractions** for high-performance rendering
- **Excellent FFI** with C libraries (GStreamer, WebKit, VA-API)
- **Modern tooling** (Cargo, async, traits)
- **Growing ecosystem** for graphics (wgpu, winit, cosmic-text)

## Why wgpu?

- **Cross-platform** — single API for Vulkan, Metal, DX12, and OpenGL
- **Safe Rust API** — no unsafe Vulkan/Metal code in application
- **WebGPU standard** — future-proof API design
- **Active development** — used by Firefox, Bevy, and many others

## Further Reading

- [The Rust display engine](rust-display-engine.md) — design document for the
  layout/rendering rewrite that replaced `xdisp.c`
- [Elisp core analysis](elisp-core-analysis.md) — in-depth analysis of the GNU Emacs
  C architecture, why it's hard to rewrite, and why Elisp is slow
- [Elisp VM design](elisp-vm-design.md) — the Rust Elisp virtual machine
- [GC design](neovm-gc-design.md) — the Rust garbage collector
