# Architecture

Neomacs is rewriting Emacs from C to Rust with clean module boundaries. The goal is a
layered architecture where the Elisp runtime is a self-contained core, editor subsystems
are independent modules communicating through defined APIs, and the rendering engine runs
on a separate GPU thread.

## Current state

The rendering engine, layout engine, and the core itself are in Rust: the Elisp
evaluator, bytecode VM, JIT, GC, reader, and the buffer/window/frame/process
subsystems all run in the `neovm-core` crate. The original C tree is retained
in-repo as the reference implementation and behavioral test oracle.

## Target architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Neomacs (Rust)                          │
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

**Design principles:**

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
  channels (`FrameGlyphBuffer` down, `InputEvent` up). Already implemented.

## Why Rust?

- **Memory safety** without garbage collection
- **Zero-cost abstractions** for high-performance rendering
- **Excellent FFI** with C (Emacs core)
- **Modern tooling** (Cargo, async, traits)
- **Growing ecosystem** for graphics (wgpu, winit, cosmic-text)

## Why wgpu?

- **Cross-platform** — single API for Vulkan, Metal, DX12, and OpenGL
- **Safe Rust API** — no unsafe Vulkan/Metal code in application
- **WebGPU standard** — future-proof API design
- **Active development** — used by Firefox, Bevy, and many others

For an in-depth analysis of the current Emacs C architecture, why it's hard to rewrite,
and why Elisp is slow, see [docs/elisp-core-analysis.md](docs/elisp-core-analysis.md).

---
