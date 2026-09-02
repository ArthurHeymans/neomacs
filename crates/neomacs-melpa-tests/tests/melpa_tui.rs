//! Interactive package parity through the shared PTY/grid adapter.
//!
//! This is a distinct Cargo test target so package-screen compatibility can
//! be selected without compiling test-name conventions into CI shell code.

#![cfg(unix)]

use neomacs_melpa_tests::*;

#[path = "../src/tui_parity_tests/mod.rs"]
mod tui_parity_tests;
