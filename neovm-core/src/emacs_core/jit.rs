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

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::emacs_core::intern::SymId;

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

// ---------------------------------------------------------------------------
// Phase 1 — feedback. The runtime-observed information later tiers speculate on.
// ---------------------------------------------------------------------------

/// Type/target feedback observed at one CALL site (the JIT's most important
/// speculation input — it enables direct-call inlining).
///
/// Holds a [`SymId`], NOT a function `Value`: a `SymId` is a stable runtime
/// index, never a heap pointer, so feedback is **GC-safe** — the collector never
/// has to trace it, and it never dangles. The optimizing tier turns
/// `Monomorphic(sym)` into a direct/inlined call guarded by a dependency on that
/// symbol's function cell (Phase 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallFeedback {
    /// This site has not executed yet.
    Uninit,
    /// Every observed call so far went to the same named function.
    Monomorphic(SymId),
    /// Conflicting / non-symbol callees seen — no useful speculation.
    Megamorphic,
}

impl CallFeedback {
    /// Pack into one `u64` for lock-free atomic storage. Low 2 bits tag the
    /// variant; a `SymId`'s `u32` rides in the upper bits.
    #[inline]
    const fn pack(self) -> u64 {
        match self {
            CallFeedback::Uninit => 0b00,
            CallFeedback::Monomorphic(SymId(n)) => ((n as u64) << 2) | 0b01,
            CallFeedback::Megamorphic => 0b10,
        }
    }

    #[inline]
    fn unpack(bits: u64) -> Self {
        match bits & 0b11 {
            0b00 => CallFeedback::Uninit,
            0b01 => CallFeedback::Monomorphic(SymId((bits >> 2) as u32)),
            0b10 => CallFeedback::Megamorphic,
            // The mask yields only 0..=3 and 0b11 is a reserved (unused) tag;
            // treat it as the safe over-approximation rather than panicking.
            _ => CallFeedback::Megamorphic,
        }
    }
}

/// A per-function feedback vector — one slot per bytecode instruction, lazily
/// allocated on first use (when the instruction count is known). Slots for
/// non-call instructions stay [`CallFeedback::Uninit`]. Lock-free
/// (`AtomicU64`), `Send + Sync` — sound to hold inline on a GC-managed function
/// alongside the concurrent collector (the mutator is the only writer).
#[derive(Debug, Default)]
pub struct FeedbackVec {
    slots: OnceLock<Box<[AtomicU64]>>,
}

impl FeedbackVec {
    #[inline]
    pub const fn new() -> Self {
        Self {
            slots: OnceLock::new(),
        }
    }

    /// Allocate (once) `len` zeroed slots. Idempotent; a benign race just keeps
    /// whichever allocation wins.
    #[inline]
    fn slots(&self, len: usize) -> &[AtomicU64] {
        self.slots
            .get_or_init(|| (0..len).map(|_| AtomicU64::new(0)).collect())
    }

    /// Record an observed callee `sym` at call-site `pc` (instruction index);
    /// `ops_len` is the function's instruction count, for lazy sizing. Drives
    /// the `Uninit -> Monomorphic -> Megamorphic` lattice.
    #[inline]
    pub fn record_call(&self, pc: usize, ops_len: usize, sym: SymId) {
        let slots = self.slots(ops_len);
        let Some(slot) = slots.get(pc) else { return };
        let next = match CallFeedback::unpack(slot.load(Ordering::Relaxed)) {
            CallFeedback::Uninit => CallFeedback::Monomorphic(sym),
            // Unchanged target — no store needed (stays monomorphic).
            CallFeedback::Monomorphic(seen) if seen == sym => return,
            CallFeedback::Monomorphic(_) => CallFeedback::Megamorphic,
            CallFeedback::Megamorphic => return,
        };
        slot.store(next.pack(), Ordering::Relaxed);
    }

    /// Feedback at call-site `pc` (or `Uninit` if unallocated / out of range).
    #[inline]
    pub fn call_at(&self, pc: usize) -> CallFeedback {
        match self.slots.get() {
            None => CallFeedback::Uninit,
            Some(slots) => slots.get(pc).map_or(CallFeedback::Uninit, |s| {
                CallFeedback::unpack(s.load(Ordering::Relaxed))
            }),
        }
    }
}

impl Clone for FeedbackVec {
    /// A clone starts with no feedback (per-instance, like the heat counter).
    fn clone(&self) -> Self {
        Self::new()
    }
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
    /// Per-call-site type/target feedback (Phase 1). The optimizing tier reads
    /// this to speculate direct/inlined calls.
    feedback: FeedbackVec,
}

impl Runtime {
    /// Invocations before a function is "hot" enough to tier up. Placeholder —
    /// tuned against the benchmark harness in Phase 8.
    pub const HOT_THRESHOLD: u32 = 10_000;

    #[inline]
    pub const fn new() -> Self {
        Self {
            heat: AtomicU32::new(0),
            feedback: FeedbackVec::new(),
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

    /// Record an observed callee `sym` at the call site at instruction `pc`
    /// (`ops_len` = the function's instruction count, for lazy sizing).
    #[inline]
    pub fn record_call(&self, pc: usize, ops_len: usize, sym: SymId) {
        self.feedback.record_call(pc, ops_len, sym);
    }

    /// Call-site feedback observed at instruction `pc`.
    #[inline]
    pub fn call_feedback(&self, pc: usize) -> CallFeedback {
        self.feedback.call_at(pc)
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

    #[test]
    fn call_feedback_packs_and_unpacks() {
        for fb in [
            CallFeedback::Uninit,
            CallFeedback::Monomorphic(SymId(0)),
            CallFeedback::Monomorphic(SymId(1)),
            CallFeedback::Monomorphic(SymId(u32::MAX)),
            CallFeedback::Megamorphic,
        ] {
            assert_eq!(CallFeedback::unpack(fb.pack()), fb);
        }
        // Uninit and Monomorphic(0) must be distinct despite the zero SymId.
        assert_ne!(
            CallFeedback::Uninit.pack(),
            CallFeedback::Monomorphic(SymId(0)).pack()
        );
    }

    #[test]
    fn feedback_lattice_uninit_mono_mega() {
        let rt = Runtime::new();
        let ops_len = 8;
        let pc = 3;
        assert_eq!(rt.call_feedback(pc), CallFeedback::Uninit);

        // First observation -> Monomorphic.
        rt.record_call(pc, ops_len, SymId(42));
        assert_eq!(rt.call_feedback(pc), CallFeedback::Monomorphic(SymId(42)));

        // Same target -> still Monomorphic.
        rt.record_call(pc, ops_len, SymId(42));
        assert_eq!(rt.call_feedback(pc), CallFeedback::Monomorphic(SymId(42)));

        // Different target -> Megamorphic, and it sticks.
        rt.record_call(pc, ops_len, SymId(7));
        assert_eq!(rt.call_feedback(pc), CallFeedback::Megamorphic);
        rt.record_call(pc, ops_len, SymId(7));
        assert_eq!(rt.call_feedback(pc), CallFeedback::Megamorphic);
    }

    #[test]
    fn feedback_is_per_site() {
        let rt = Runtime::new();
        rt.record_call(1, 8, SymId(10));
        rt.record_call(5, 8, SymId(20));
        assert_eq!(rt.call_feedback(0), CallFeedback::Uninit);
        assert_eq!(rt.call_feedback(1), CallFeedback::Monomorphic(SymId(10)));
        assert_eq!(rt.call_feedback(5), CallFeedback::Monomorphic(SymId(20)));
        // Out-of-range pc is Uninit, never a panic.
        assert_eq!(rt.call_feedback(99), CallFeedback::Uninit);
    }

    #[test]
    fn out_of_range_record_is_ignored() {
        let rt = Runtime::new();
        rt.record_call(100, 8, SymId(1)); // pc >= ops_len: no-op, no panic
        assert_eq!(rt.call_feedback(100), CallFeedback::Uninit);
    }

    #[test]
    fn clone_clears_feedback() {
        let rt = Runtime::new();
        rt.record_call(2, 8, SymId(5));
        assert_eq!(rt.call_feedback(2), CallFeedback::Monomorphic(SymId(5)));
        assert_eq!(rt.clone().call_feedback(2), CallFeedback::Uninit);
    }
}
