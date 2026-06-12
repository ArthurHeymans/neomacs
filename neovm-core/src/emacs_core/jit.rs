//! Tiered execution subsystem for the Emacs-Lisp VM — the foundation of the
//! modern JIT path. See `bytecode/ELISP_VM_MODERNIZATION.md` for the full
//! design + phased roadmap.
//!
//! **Phase 0.** Gated behind the `jit` cargo feature (default OFF): production
//! builds are byte-for-byte unchanged while this is built out. The bytecode
//! interpreter (`bytecode::Vm`) is always the **Tier 0** engine — the
//! correctness oracle that mirrors GNU Emacs 31.0.90 and the deoptimization
//! landing pad. It is never removed.
//!
//! Design rule (carried over from the GC work): every dispatch over an
//! execution tier is an **exhaustive `match`** with no catch-all arm, so adding
//! a tier fails to compile until every site handles it. That is the same
//! compiler-enforced completeness that caught the GC `trace_veclike`
//! use-after-free (an incomplete duplicate with a `_ => {}` arm).

#![cfg_attr(not(feature = "jit"), allow(dead_code))]

use std::sync::atomic::{AtomicU32, Ordering};

/// Which execution tier currently backs a compiled function.
///
/// Only [`Tier::Bytecode`] exists today. Later phases add `Baseline`
/// (copy-and-patch, `dynasmrt`) and `Optimized` (Cranelift). Do NOT add a
/// catch-all when matching on this — let the compiler enforce that each new
/// tier is handled everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tier {
    /// Tier 0 — interpret the function's bytecode `ops` via `bytecode::Vm`.
    #[default]
    Bytecode,
}

/// The action the dispatcher takes for one invocation of a compiled function.
/// Exhaustive by design (mirrors [`Tier`]).
#[derive(Debug)]
pub enum Plan {
    /// Run the Tier-0 bytecode interpreter.
    Interpret,
    // Phase 3+: RunBaseline(BaselineCode), RunOptimized(OptimizedCode),
}

/// Per-function runtime tiering + profiling state.
///
/// Lives inline on `ByteCodeFunction` (only when the `jit` feature is on) but is
/// NOT part of the dumped representation (`DumpByteCodeFunction`) — it is pure
/// runtime state, started cold each session and on each clone. Relaxed atomics:
/// the mutator is the only writer today, and being `Sync` keeps the heap object
/// sound alongside the concurrent collector.
#[derive(Debug)]
pub struct Runtime {
    /// Coarse invocation hotness (saturating at `u32::MAX`). The feedback that
    /// later phases use to decide when to tier a function up.
    heat: AtomicU32,
}

impl Runtime {
    /// Invocations before a function is "hot" enough to tier up. Placeholder —
    /// tuned against the benchmark harness in Phase 8.
    pub const HOT_THRESHOLD: u32 = 10_000;

    #[inline]
    pub const fn new() -> Self {
        Self {
            heat: AtomicU32::new(0),
        }
    }

    /// Record one invocation and decide how to run it. The caller MUST handle
    /// the returned [`Plan`] exhaustively.
    ///
    /// Today this only counts and returns [`Plan::Interpret`]; once a compiled
    /// tier exists (Phase 3+) it branches there when [`Runtime::is_hot`].
    #[inline]
    pub fn dispatch(&self) -> Plan {
        // Saturating bump — a long-lived hot function must never wrap to cold.
        let prev = self.heat.load(Ordering::Relaxed);
        self.heat.store(prev.saturating_add(1), Ordering::Relaxed);
        Plan::Interpret
    }

    /// True once this function has crossed the tier-up threshold.
    #[inline]
    pub fn is_hot(&self) -> bool {
        self.heat.load(Ordering::Relaxed) >= Self::HOT_THRESHOLD
    }

    /// Current invocation count.
    #[inline]
    pub fn heat(&self) -> u32 {
        self.heat.load(Ordering::Relaxed)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Runtime {
    /// A clone of a function starts COLD — profiling is per-instance.
    fn clone(&self) -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_counts_and_plans_interpret() {
        let rt = Runtime::new();
        assert_eq!(rt.heat(), 0);
        assert!(!rt.is_hot());
        for i in 1..=5 {
            assert!(matches!(rt.dispatch(), Plan::Interpret));
            assert_eq!(rt.heat(), i);
        }
        assert!(!rt.is_hot());
    }

    #[test]
    fn becomes_hot_at_threshold() {
        let rt = Runtime::new();
        for _ in 0..Runtime::HOT_THRESHOLD {
            let _ = rt.dispatch();
        }
        assert!(rt.is_hot());
    }

    #[test]
    fn heat_saturates_without_wrapping() {
        let rt = Runtime::new();
        // Seed near the ceiling, then bump past it; must clamp, not wrap to cold.
        for _ in 0..3 {
            rt.heat
                .store(u32::MAX - 1, std::sync::atomic::Ordering::Relaxed);
            let _ = rt.dispatch();
            assert_eq!(rt.heat(), u32::MAX);
            let _ = rt.dispatch();
            assert_eq!(rt.heat(), u32::MAX);
        }
    }

    #[test]
    fn clone_starts_cold() {
        let rt = Runtime::new();
        for _ in 0..100 {
            let _ = rt.dispatch();
        }
        assert_eq!(rt.heat(), 100);
        assert_eq!(rt.clone().heat(), 0);
    }
}
