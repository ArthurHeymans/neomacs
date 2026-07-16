//! Quarantined lisp shims — population D of the four-population layout
//! (docs/design/neovm-core-layout.md).
//!
//! Every module here is a Rust reimplementation of functionality GNU Emacs
//! defines in `lisp/**/*.el`. The project parity rule is that Lisp
//! functionality comes from loading the real `.el` file, never from a Rust
//! rewrite, so this directory exists to make that debt visible and shrinking:
//! nothing may be added here, and each file's header names the `.el` it
//! shadows and what still keeps it alive. A shim whose runtime state is no
//! longer referenced gets deleted outright.
//!
//! Runtime-probe status (2026-07-16): none of these register subrs; the
//! el-shadow subrs that DO exist elsewhere are overwritten by the preloaded
//! `.el` during loadup (48/52 names), so the shims are bootstrap scaffolding,
//! not user-visible divergences.

pub mod abbrev;
pub mod bookmark;
pub mod cl_lib;
pub mod isearch;
pub mod rect;
pub mod register;
