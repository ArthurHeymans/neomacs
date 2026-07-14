//! Localhost HTTP diagnostics server for neomacs performance introspection.
//!
//! Runs on a dedicated OS thread with a tokio current-thread runtime. Reads
//! metrics through a [`MetricsProvider`] supplied by the host binary; it never
//! touches VM state directly — only lock-free published atomics reached through
//! the provider closure. This keeps the Lisp VM synchronous and confines tokio
//! to this crate (the "IO-reactor edge").

pub mod flamegraph;
pub mod metrics;
pub mod report;
pub mod server;

pub use flamegraph::folded_to_svg;
pub use metrics::MetricsSnapshot;
pub use report::{CpuReport, Hotspot, cpu_report_from_folded};
pub use server::{DiagnosticsConfig, MetricsProvider, port_from_str, router, spawn};

#[cfg(test)]
mod flamegraph_test;
#[cfg(test)]
mod metrics_test;
#[cfg(test)]
mod report_test;
#[cfg(test)]
mod server_test;
