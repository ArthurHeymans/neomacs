//! Localhost HTTP diagnostics server for neomacs performance introspection.
//!
//! Runs on a dedicated OS thread with a tokio current-thread runtime. Reads
//! metrics through a [`MetricsProvider`] supplied by the host binary; it never
//! touches VM state directly — only lock-free published atomics reached through
//! the provider closure. This keeps the Lisp VM synchronous and confines tokio
//! to this crate (the "IO-reactor edge").

pub mod metrics;
pub mod server;

pub use metrics::MetricsSnapshot;
pub use server::{DiagnosticsConfig, MetricsProvider, router, spawn};

#[cfg(test)]
mod metrics_test;
#[cfg(test)]
mod server_test;
