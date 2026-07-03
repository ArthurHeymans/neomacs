# Task 09 — JIT standing defers (consolidated go-criteria)

Status: ALL DEFERRED ON EVIDENCE. This doc consolidates the five standing
JIT items so their go-criteria live in one place and nobody re-assesses from
scratch. Each was individually assessed (some with multi-agent feasibility
workflows) — the verdicts and their reasoning are summarized with what would
FLIP each to GO.

## 1. OSR (on-stack replacement) — DEFER (net-negative-leaning)

Assessment summary: codegen is surprisingly cheap here (interpreter locals ==
operand-stack slice; loop headers are already CLIF blocks with params ==
entry stack; the deopt resume machinery `run_resumed_frame` is the inverse
transfer). But the VALUE is near-absent: heat is per-FUNCTION (bumped once
per call at dispatch), so every hot loop in a REPEATEDLY-CALLED function
tiers at call-entry; the only beneficiary is a long loop in a ONCE-called
function — and those are typically call/IO-heavy (outside the compilable
subset). Emacs's main loop is native Rust, not bytecode. COSTS: a per-loop
back-edge counter taxing the interpreter's hottest path; a SECOND cache
dimension (per (fn, loop-header) artifacts) with its own eviction; a
mid-run_loop suspend/enter-native control point; all-precise-deopt-only MIR;
silent-miscompile risk class (mid-function native entry with mis-mapped
state).
GO-CRITERIA: a profiled real workload showing a hot, compilable,
never-call-entry-tiered once-called bytecode loop. Cheap no-regret precursor
if suspected: an env-gated measurement-only per-loop back-edge profiler.

## 2. Tier-0 quickening / call ICs — DEFER (go-but-defer)

The classic quickening wins ALREADY exist in the interpreter: pre-decoded
Vec<Op>, inline fixnum fast paths, a fused Dup+StackRef+Lss+GotoIfNil
superinstruction, O(1) stack refs. In-place op MUTATION is UNSOUND here
(run_loop dispatches over a shared borrow of GC-managed, concurrently-scanned
ops) — any quickening must be a side table. The one defensible piece: a
side-table monomorphic CALL IC for bytecode `Op::Call` reusing the
FeedbackVec tier-0 already POPULATES (record_call) but nothing reads.
GO-CRITERIA: vm-profile op-mix data showing a real sub-threshold workload
spending material CPU in Op::Call re-resolution, plus a warm-up A/B (pin
tier-0 via the test seam) showing the IC wins without regressing the hot
path. NOTE: task 02's real-session profiling produces exactly the needed
data as a side effect — re-check this defer after task 02's measurements.

## 3. Float unboxing — DEFER

Floats are heap-boxed; reboxing allocates -> GC-safepoint entanglement (raw
values cannot live across safepoints — the load-bearing unboxing rule).
Elisp floats are rare in hot paths (the drain classifier measured ~437 float
allocs at startup; churn workloads allocate them only when asked).
GO-CRITERIA: a profiled float-heavy real workload (audio? calc?). If it ever
fires, the design must confine raw f64 to safepoint-free regions exactly like
the fixnum path-B rule.

## 4. Feedback-driven speculation — DEFER (partially superseded)

The FeedbackVec records call targets (Uninit -> Monomorphic(SymId) ->
Megamorphic) and nothing reads it. The original idea — feedback-selected
guards — was partially superseded: static obarray-resolved spec sites +
epoch re-arm (round-1 intrinsics) already cover the monomorphic-call case
for constant-symbol sites. The remaining feedback value: (a) NON-constant
callee sites (funcall through a variable) — feedback could arm a
monomorphic guess with the same SpecSlot machinery; (b) type feedback for
arithmetic beyond the static fixnum speculation. Both need evidence that the
populations exist in real code.
GO-CRITERIA: task 02's real-session profile showing hot variable-callee
sites (measurable: count Op::Call sites whose callee is NOT a constant
symbol, weighted by execution) or guard-failure counters showing the static
fixnum speculation thrashing anywhere.

## 5. body_is_jit_profitable loosening — DEFER (re-measured, still negative)

The gate rejects call-dominated bodies (calls > arith). History: compiling
them measured ~32% SLOWER than interpreting (the shim-call + rooting
overhead). Re-measured AFTER round-1 intrinsics (release, real bootstrap
load, 3-config A/B): gate-ON 19.5s / gate-OFF 20.9s / JIT-off 20.1s — still
+7% net-negative. The gate stays.
GO-CRITERIA: re-run the same 3-config A/B after task 02 lands. If gate-OFF
finally beats gate-ON, don't just delete the gate — make it smarter: count
DIRECT-ABLE calls (spec-site-classifiable) separately from generic calls in
the ops scan (the gate's signature currently takes only &[Op]; threading the
obarray in changes AOT determinism — at obarray=None the gate must behave
identically, so key the refinement on op SHAPE (constant-symbol callee
patterns), not on resolution).

## 6. Shared harness notes for any of these

The differential gate (`NEOVM_JIT_THRESHOLD=1`), FORCE_DEOPT (expectation-set
compare, ~29 documented incompatible tests), FORCE_SLOW_SPEC, the engagement
counters, and the bench suite (`jit_bench_pred/subr/cbsym/fib/loop` +
`run-jit-bench.sh`) are the verification vocabulary. Any change here also
interacts with AOT's ABI_TAG (STATUS codes and shim sets are explicitly
enumerated in `compute_abi_tag` — extend it when adding either).
