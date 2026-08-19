//! Tiered execution subsystem for the Emacs-Lisp VM — the foundation of the
//! modern JIT path. See `bytecode/ELISP_VM_MODERNIZATION.md` for the full
//! design + phased roadmap.
//!
//! Gated behind the `jit` cargo feature, which is **default-ON** (`default =
//! ["jit"]`): both the baseline (Tier-1) Cranelift JIT and the optimizing
//! typed-MIR Tier-2 above it are qualified and shipping. The bytecode
//! interpreter (`bytecode::Vm`) is always the **Tier 0** engine — the
//! correctness oracle that mirrors GNU Emacs 31.0.90 and the deoptimization
//! landing pad. It is never removed.
//!
//! Design rule (carried over from the GC work): every dispatch over an
//! execution tier is an **exhaustive `match`** with no catch-all arm, so adding
//! a tier fails to compile until every site handles it. That is the same
//! compiler-enforced completeness that caught the GC `trace_veclike`
//! use-after-free (an incomplete duplicate with a `_ => {}` arm).
//!
//! # Environment-knob inventory (the authoritative list)
//!
//! Every runtime toggle this subsystem reads, with default and status. Grep
//! anchor: `env::var("NEOVM_`. Keep this table in sync when adding a knob, and
//! give every OPT-IN knob a graduation plan — soak → default-on, or delete —
//! so the surface doesn't accumulate permanently-dead branches.
//!
//! ## Runtime switches (shipping, default-on)
//! | Knob | Default | Meaning |
//! |---|---|---|
//! | `NEOVM_JIT` | on | Kill switch: `0`/`off`/`false`/`no` forces the pure interpreter (the A/B baseline). |
//! | `NEOVM_JIT_THRESHOLD` | 256 | Tier-up heat threshold; `=1` compiles every compilable function — the differential soak and the strictest oracle configuration. |
//! | `NEOVM_JIT_LOOP_HEAT` | 1 | Heat credited per 256-iteration back-edge wrap (loop-aware tier-up); `=0` disables loop heat — the pre-loop-heat baseline. |
//! | `NEOVM_JIT_LEVER1` | on | Residual-rooting non-heap skip; `=off` reverts to an unconditional gc_push per residual (single-build A/B). |
//! | `NEOVM_JIT_PROFIT` | on | Profitability gate (calls ≤ arith); `=off` also compiles call-heavy bodies. |
//!
//! ## Opt-in features (default-OFF, pending a graduation decision)
//! | Knob | Enable | Meaning / graduation blocker |
//! |---|---|---|
//! | `NEOVM_JIT_OSR` | `=on` | Mid-loop interpreter→native transfer (on-stack replacement). Blocker: soak the live-stack marshalling path under real workloads before default-on. |
//! | `NEOVM_JIT_INLINE_ARITH` | `=on` | Level-B native bit-ops (logand/logior/logxor/lognot) with fixnum-guard deopt. Blocker: skips the compiler-macro bounce; a mixed-type loop falls back to the interpreter ungracefully. |
//! | `NEOVM_JIT_GATE_RELAX` | `=on` | Relax the calls ≤ arith profit gate. Default-on was tried and REVERTED (regressed byte-compile 21%) — measure byte-compile before ever re-flipping. |
//!
//! ## Measurement / bisection
//! | Knob | Meaning |
//! |---|---|
//! | `NEOVM_JIT_MAX_ID` | Compile only functions with id ≤ N (ids assigned in first-hot order) — clean prefix bisection of a misbehaving workload. |
//! | `NEOVM_JIT_DEBUG_ID` | Dump the bytecode body of the one compiled function with this id. |
//! | `NEOVM_JIT_PROFILE` | Append per-function workload-characterization records to this file path. |
//! | `NEOVM_JIT_COMPILE_STATS` | `=1`: print a running compile-stall summary line every 64 compiles. |
//!
//! ## Verification harnesses (force the cold path everywhere; run the suite with each ON)
//! | Knob | Forces |
//! |---|---|
//! | `NEOVM_JIT_FORCE_DEOPT=1` | Every speculation guard fails → every deopt path executes. |
//! | `NEOVM_JIT_FORCE_SLOW_SPEC=1` | Every spec-call shim takes its stale-epoch re-validate branch on every call. |
//! | `NEOVM_JIT_FORCE_CBSYM_GENERIC=1` | Every CallBuiltinSym intrinsic bounces to its generic fallback. |
//!
//! ## AOT (`jit/aot.rs`)
//! | Knob | Meaning |
//! |---|---|
//! | `NEOVM_AOT` | `1`/`on`/`force` enables the AOT preload; `force` additionally warns when no usable preload loaded. |
//! | `NEOVM_AOT_PGO` | `1`/`on`/`force` enables PGO collection for the AOT function set. |

#![cfg_attr(not(feature = "jit"), allow(dead_code))]

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::emacs_core::intern::SymId;

/// Cranelift codegen backend (Phase 3+). Only compiled with the `jit` feature,
/// since it links Cranelift. Today it exposes a self-contained smoke path that
/// proves the codegen toolchain works inside neovm-core's own build before any
/// bytecode is lowered onto it — the same "prove the tool, then build on it"
/// discipline used to validate TSan before trusting the concurrent GC.
#[cfg(feature = "jit")]
pub mod backend;

/// Baseline bytecode → native lowering (Phase 3b+). Compiles the leaf,
/// straight-line opcode subset to machine code and bails to the interpreter on
/// anything else. Only built with the `jit` feature. See `jit/compile.rs`.
#[cfg(feature = "jit")]
pub mod compile;

/// Per-thread compiled-code cache + the tier-up entry point the dispatch seam
/// calls ([`cache::try_run_compiled`]). Only built with the `jit` feature.
#[cfg(feature = "jit")]
pub mod cache;

/// MIR: typed SSA IR for the optimizing Tier-2 (above the baseline `compile`).
/// Live: `compile::compile_bytecode_function_inner` builds the MIR for pure
/// required-only bodies, runs the pure inliner + type/unboxing/guard-elision and
/// cons-escape passes, and lowers it via `compile::lower_mir_pure`, falling back
/// to the baseline tier otherwise. Only built with the `jit` feature. See
/// `jit/mir.rs`.
#[cfg(feature = "jit")]
pub mod mir;

/// AOT (ahead-of-time) object emission (Phase R1c): emit the same CLIF the JIT
/// does, but through Cranelift's `ObjectModule`, producing a relocatable `.o`
/// that is linked to a `.so`, `dlopen`'d, and inserted as a pre-warmed
/// `CompiledLeaf`. Only built with the `jit` feature. See `jit/aot.rs`.
#[cfg(feature = "jit")]
pub mod aot;

/// Always-on metering of the synchronous compile stalls the cache-miss path
/// pays on the eval thread — the evidence base for background compilation.
/// Only built with the `jit` feature. See `jit/stats.rs`.
#[cfg(feature = "jit")]
pub mod stats;

#[cfg(feature = "jit")]
pub use cache::try_run_compiled;

/// Which execution tier currently backs a compiled function.
///
/// This enum models only the interpreter tier; the compiled tiers — the
/// baseline Cranelift JIT and the optimizing typed-MIR Tier-2 — live in
/// `compile.rs` and are selected there, reached via [`Plan::Compiled`]. Do NOT
/// add a catch-all when matching on this — let the compiler enforce that each
/// new tier is handled everywhere.
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
    /// The function is hot — consult the JIT (the baseline tier or the
    /// optimizing typed-MIR Tier-2, selected in `compile.rs`): compile-on-first-
    /// use and run native, or fall back to the interpreter on a deopt /
    /// non-compilable body. See [`cache::try_run_compiled`].
    Compiled,
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
    /// Process-unique identity assigned on first JIT compilation attempt (0 =
    /// unassigned). Keys this function's entry in the per-thread compiled-code
    /// cache ([`cache`]). Monotonic and never reused, so a freed function's
    /// stale cache entry can never be mis-looked-up after the (non-moving) GC
    /// reuses its address — a new function gets a new id. Reset to 0 on clone.
    compiled_id: AtomicU64,
    /// Set by the AOT preload prepopulate (R2-C3) when this function's leaf was
    /// inserted into the compiled cache at startup: `dispatch` then serves the
    /// prewarmed native leaf FROM CALL 1 instead of interpreting until the heat
    /// threshold — the piece that lets the AOT preload cover ONE-SHOT startup
    /// elisp, which never gets hot. One relaxed load on the dispatch path,
    /// never set in the default (AOT-off) configuration.
    aot_prewarmed: std::sync::atomic::AtomicBool,
    /// Test-only: pin this function to the Tier-0 interpreter regardless of
    /// hotness (the benchmark harness measures native vs interpreter in ONE
    /// process — a hot copy and a forced-cold copy — to cancel the
    /// cross-process CPU-frequency variance that wrecks a two-process A/B).
    /// Absent from the production library (only the test binary carries it).
    #[cfg(test)]
    force_interpret: std::sync::atomic::AtomicBool,
}

/// Source of process-unique [`Runtime::compiled_id`] values. Ids are
/// `fetch_add + 1` so 0 stays reserved for "unassigned".
static NEXT_COMPILED_ID: AtomicU64 = AtomicU64::new(0);

/// Invocations before a function tiers up to the JIT. Defaults to
/// [`Runtime::HOT_THRESHOLD`]; the `NEOVM_JIT_THRESHOLD` environment variable
/// overrides it — e.g. `=1` runs every compilable function through the JIT,
/// the every-function differential soak used to qualify default-on (Phase 9).
pub fn hot_threshold() -> u32 {
    static THRESHOLD: OnceLock<u32> = OnceLock::new();
    *THRESHOLD.get_or_init(|| {
        std::env::var("NEOVM_JIT_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(Runtime::HOT_THRESHOLD)
    })
}

/// Heat credited per backward-branch *wrap* — one wrap is
/// [`LOOP_BACKEDGES_PER_WRAP`] (256) loop iterations, so this weights 256 loop
/// iterations as ≈ one function invocation (`dispatch` credits +1 per call).
/// This is the tier-up signal for a body dominated by a long INNER LOOP but
/// called only a handful of times: `dispatch` alone would never make it hot
/// (heat counts calls), so a hot loop in a rarely-called function stayed in the
/// interpreter forever. `NEOVM_JIT_LOOP_HEAT` overrides it; **`=0` disables
/// loop heat** — the pre-loop-heat behavior and the A/B baseline.
pub fn loop_heat_per_wrap() -> u32 {
    static V: OnceLock<u32> = OnceLock::new();
    *V.get_or_init(|| {
        std::env::var("NEOVM_JIT_LOOP_HEAT")
            .ok()
            .and_then(|s| s.parse().ok())
            // 8 ⇒ 32 loop iterations ≈ one invocation ⇒ a loop tiers up (and
            // OSR fires) near 32k total iterations. The old credit of 1
            // needed 256k iterations — a 60k-iteration hot loop in a
            // once-called function (the realworld buffer bench's insert and
            // scan loops) never left the interpreter.
            .unwrap_or(8)
    })
}

/// Whether the JIT is active at runtime. The `jit` cargo feature compiles the
/// JIT *in*; this switch turns tier-up on/off WITHOUT a recompile, so a single
/// binary can run pure-interpreter or JIT-backed. Default on; `NEOVM_JIT=0`
/// (also `off`/`false`/`no`) forces the interpreter — a kill switch and the
/// A/B-measurement knob (no more `NEOVM_JIT_THRESHOLD=<huge>` hack).
pub fn jit_runtime_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        !matches!(
            std::env::var("NEOVM_JIT").ok().as_deref(),
            Some("0" | "off" | "false" | "no")
        )
    })
}

/// OSR (on-stack replacement): transfer a hot loop in a rarely-/once-called
/// function into native code MID-execution (the case loop-heat's next-entry
/// tier-up cannot reach). Default ON since the `mod` arith-intrinsic made the
/// transferred loop a measured win on builtin-call-bearing bodies (list
/// workload −25% wall; the shimmed-builtin overhead previously ate the
/// transfer's gain — the reason this started life opt-in). Kill switch:
/// `NEOVM_JIT_OSR=off` (same spelling family as `NEOVM_JIT`); the interpreter
/// marshals its live operand stack into a native OSR entry, restricted to
/// functions with no dynamic bind/handler/save ops (nothing to transfer).
/// Off ⇒ the back-edge stays a pure interpreter loop, zero added cost.
pub fn jit_osr_on() -> bool {
    #[cfg(test)]
    if let Some(o) = OSR_TEST_OVERRIDE.with(|c| c.get()) {
        return o;
    }
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        !matches!(
            std::env::var("NEOVM_JIT_OSR").ok().as_deref(),
            Some("0" | "off" | "false" | "no")
        )
    })
}

#[cfg(test)]
thread_local! {
    static OSR_TEST_OVERRIDE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Force OSR on/off on the current thread (tests only), overriding the env gate.
#[cfg(test)]
pub fn force_osr_for_test(on: bool) {
    OSR_TEST_OVERRIDE.with(|c| c.set(Some(on)));
}

impl Runtime {
    /// Invocations before a function is "hot" enough to tier up.
    ///
    /// Tuned via `jit_bench_threshold_economics` (eval_test.rs), an
    /// interleaved debug-build A/B of 1_000 vs the previous placeholder
    /// 10_000 across 1.2k/3k/20k-call workloads: 1_000 halves end-to-end
    /// wall time for the 3k-20k call population (the functions a 10_000
    /// threshold strands in the interpreter forever) and only regresses
    /// ~1.2k-call functions by the one-time compile cost — which a debug
    /// build heavily inflates, so the release regression is smaller still.
    /// Compilation is the only cost lowering adds; going far lower (100)
    /// starts compiling barely-warm functions for no amortized win.
    /// `NEOVM_JIT_THRESHOLD` still overrides per process.
    pub const HOT_THRESHOLD: u32 = 1_000;

    #[inline]
    pub const fn new() -> Self {
        Self {
            heat: AtomicU32::new(0),
            feedback: FeedbackVec::new(),
            compiled_id: AtomicU64::new(0),
            aot_prewarmed: std::sync::atomic::AtomicBool::new(false),
            #[cfg(test)]
            force_interpret: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Record one invocation and decide how to run it. The caller MUST handle
    /// the returned [`Plan`] exhaustively.
    ///
    /// Counts the invocation and returns [`Plan::Compiled`] once the function
    /// crosses [`hot_threshold`] (default [`Runtime::HOT_THRESHOLD`]), else
    /// [`Plan::Interpret`]. The compiled plan only means "the JIT may run this"
    /// — the cache still falls back to the interpreter for non-compilable
    /// bodies and on deopt.
    #[inline]
    pub fn dispatch(&self) -> Plan {
        // Runtime kill switch (NEOVM_JIT=0): never tier up — pure interpreter,
        // no recompile. The early return also skips the heat bump, so a disabled
        // JIT is strictly cheaper than an enabled-but-cold one.
        if !jit_runtime_enabled() {
            return Plan::Interpret;
        }
        // Test-only: a forced-cold function never tiers up (benchmark A/B).
        #[cfg(test)]
        if self.force_interpret.load(Ordering::Relaxed) {
            return Plan::Interpret;
        }
        // Saturating bump — a long-lived hot function must never wrap to cold.
        let prev = self.heat.load(Ordering::Relaxed);
        let now = prev.saturating_add(1);
        self.heat.store(now, Ordering::Relaxed);
        if now >= hot_threshold() || self.aot_prewarmed.load(Ordering::Relaxed) {
            Plan::Compiled
        } else {
            Plan::Interpret
        }
    }

    /// This function's compiled-cache id, assigning a fresh process-unique one
    /// on first call (idempotent under races). Used only by [`cache`].
    #[inline]
    pub fn compiled_id_or_assign(&self) -> u64 {
        let cur = self.compiled_id.load(Ordering::Acquire);
        if cur != 0 {
            return cur;
        }
        // `+ 1` keeps 0 reserved for "unassigned".
        let fresh = NEXT_COMPILED_ID.fetch_add(1, Ordering::Relaxed) + 1;
        match self
            .compiled_id
            .compare_exchange(0, fresh, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => fresh,
            // Another thread won the race; adopt its id, discard ours.
            Err(actual) => actual,
        }
    }

    /// This function's compiled-cache id if one was ALREADY assigned (it has been
    /// compiled/hot), else `None` — WITHOUT assigning a fresh one. The AOT-PGO drain
    /// uses this to intersect the obarray walk with the hot set without minting ids
    /// for the many never-compiled bound functions it walks past.
    #[inline]
    pub fn compiled_id(&self) -> Option<u64> {
        let cur = self.compiled_id.load(Ordering::Acquire);
        (cur != 0).then_some(cur)
    }

    /// True once this function has crossed the tier-up threshold.
    #[inline]
    pub fn is_hot(&self) -> bool {
        self.heat.load(Ordering::Relaxed) >= hot_threshold()
    }

    /// Current invocation count.
    #[inline]
    pub fn heat(&self) -> u32 {
        self.heat.load(Ordering::Relaxed)
    }

    /// Backward branches per loop-heat wrap — the interpreter's `branch_to!`
    /// quit counter is a `u8`, so it wraps (and calls [`note_loop_work`]) once
    /// per 256 backward branches. Documented here so the loop-heat weighting
    /// (256 iterations ≈ one call) is discoverable from the `Runtime` API.
    ///
    /// [`note_loop_work`]: Self::note_loop_work
    pub const LOOP_BACKEDGES_PER_WRAP: u32 = 256;

    /// Credit loop work toward tier-up: called from the interpreter's
    /// backward-branch quit-counter wrap (`bytecode/vm.rs`), i.e. once per
    /// [`LOOP_BACKEDGES_PER_WRAP`](Self::LOOP_BACKEDGES_PER_WRAP) iterations, so
    /// the per-iteration cost is amortized to ~nothing. A function whose body is
    /// a long INNER LOOP but which is CALLED only a few times never crosses
    /// [`hot_threshold`] on `dispatch`'s per-call bump alone; this accumulates
    /// heat from the loop itself so the NEXT entry tiers it up. The CURRENT
    /// interpreted call still runs to completion in Tier 0 — there is no
    /// on-stack replacement, so a body called exactly once sees no benefit
    /// (that is the OSR follow-up). Saturating: a long-lived loop must never
    /// wrap heat back to cold. Respects the [`jit_runtime_enabled`] kill switch
    /// and the `NEOVM_JIT_LOOP_HEAT=0` off knob.
    #[inline]
    pub fn note_loop_work(&self) {
        let credit = loop_heat_per_wrap();
        if credit == 0 || !jit_runtime_enabled() {
            return;
        }
        let prev = self.heat.load(Ordering::Relaxed);
        self.heat
            .store(prev.saturating_add(credit), Ordering::Relaxed);
    }

    /// Test-only: force this function "hot" so the next [`dispatch`](Self::dispatch)
    /// tiers it up, without driving `HOT_THRESHOLD` real invocations.
    /// Mark this function as served by a prepopulated AOT leaf (see the
    /// field doc): `dispatch` returns `Plan::Compiled` from call 1.
    pub(crate) fn mark_aot_prewarmed(&self) {
        self.aot_prewarmed
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    #[cfg(test)]
    pub(crate) fn set_hot_for_test(&self) {
        self.heat.store(Self::HOT_THRESHOLD, Ordering::Relaxed);
    }

    /// Test-only: pin this function to the Tier-0 interpreter forever (the
    /// forced-cold half of the benchmark A/B; see `force_interpret`).
    #[cfg(test)]
    pub(crate) fn set_cold_for_test(&self) {
        self.force_interpret
            .store(true, std::sync::atomic::Ordering::Relaxed);
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
        // Threshold-aware so the test also holds under a NEOVM_JIT_THRESHOLD
        // override (e.g. the =1 every-function soak).
        let threshold = hot_threshold();
        let rt = Runtime::new();
        assert_eq!(rt.heat(), 0);
        assert!(!rt.is_hot());
        for i in 1..=5u32 {
            let plan = rt.dispatch();
            if i >= threshold {
                assert!(
                    matches!(plan, Plan::Compiled),
                    "hot at {i} (>= {threshold})"
                );
            } else {
                assert!(
                    matches!(plan, Plan::Interpret),
                    "cold at {i} (< {threshold})"
                );
            }
            assert_eq!(rt.heat(), i);
        }
        assert_eq!(rt.is_hot(), 5 >= threshold);
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
    fn loop_work_tiers_up_a_rarely_called_body() {
        // A body CALLED only a few times (dispatch < threshold) but running a
        // hot INNER LOOP must still tier up: note_loop_work credits the loop.
        let credit = loop_heat_per_wrap();
        if credit == 0 || !jit_runtime_enabled() {
            return; // loop heat disabled in this env — nothing to assert
        }
        let threshold = hot_threshold();
        let rt = Runtime::new();
        // Called a handful of times: nowhere near hot on call count alone.
        for _ in 0..5 {
            assert!(matches!(rt.dispatch(), Plan::Interpret));
        }
        let after_calls = rt.heat();
        assert!(!rt.is_hot(), "5 calls must be cold (threshold {threshold})");
        // Now credit loop work (each wrap = 256 iterations); cross the threshold.
        let wraps_needed = threshold.div_ceil(credit) + 1;
        for _ in 0..wraps_needed {
            rt.note_loop_work();
        }
        assert!(rt.is_hot(), "hot after {wraps_needed} loop wraps");
        assert!(matches!(rt.dispatch(), Plan::Compiled));
        assert!(rt.heat() > after_calls);
    }

    #[test]
    fn loop_work_saturates_without_wrapping() {
        if loop_heat_per_wrap() == 0 || !jit_runtime_enabled() {
            return;
        }
        let rt = Runtime::new();
        rt.heat
            .store(u32::MAX - 1, std::sync::atomic::Ordering::Relaxed);
        rt.note_loop_work();
        assert_eq!(rt.heat(), u32::MAX);
        rt.note_loop_work();
        assert_eq!(rt.heat(), u32::MAX);
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
