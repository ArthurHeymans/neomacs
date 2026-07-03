# Task 02 — JIT intrinsics round 2 (profile-first, from REAL interactive sessions)

Status: PROTOCOL READY. Risk: MEDIUM (JIT lowering; the round-1 machinery is
battle-tested and the pattern is proven). Effort: 1 session for profiling +
adjudication, 1-2 for implementation. Prerequisite reading: the round-1 work
(commits "feat(jit): speculative direct-subr calls + predicate fast-path
shims" + its test commit) and this doc's §3 architecture recap.

## 1. Why (evidence and the honest frame)

Round 1 (direct-subr spec calls + predicate shims) delivered:
- `jit_bench_pred` (2M `recordp` calls via generic `Op::Call`): 1.41x -> 7.9x
  vs interpreter (native 35ms -> 7ms).
- `jit_bench_subr` (2M `length` calls): 1.38x -> ~1.6x (the generic path
  already had an in-place subr fast path; the win is resolution+dispatch
  elimination, which is real but bounded).
- Profitability gate re-measure: compiling call-DOMINATED bodies is STILL
  net-negative (+7% on the real bootstrap-load workload) even with direct
  calls — `body_is_jit_profitable` stays. (Release, 3-config A/B: gate-ON
  19.5s / gate-OFF 20.9s / JIT-off 20.1s.)

The round-1 target list came from profiling the BYTE-COMPILER (the heaviest
in-process real-elisp workload available in tests):
`equal-including-properties` 38%, `symbol-with-pos-p` 15%, `fboundp` 12%,
`keywordp` 5.5%, `mapcar` 3.75%, `recordp`/`vectorp`/`functionp` ~2.5% each.
That workload is real but NOT an interactive session. The interactive
population is expected to differ substantially: the `CallBuiltinSym` opcode
population in production bytecode is buffer/point ops (GNU opcodes 96-127:
`point`, `bolp`, `eolp`, `following-char`, `insert`, `goto-char`,
`current-column`, plus `set-marker`/`match-beginning`/`match-end`/`upcase`/
`downcase` from the decoder's explicit list — see
`bytecode/decode.rs` opcode table), and command-loop/font-lock-support elisp
leans on `get`/`put`, `gethash`/`puthash`, `memq`, `assq`, `string=`, and
match-data accessors.

**Round 2 = run the profiler on REAL interactive traffic, then intrinsify the
top of THAT list.** Do not skip the profiling; round 1's list would have been
mis-guessed without it (e.g. `logand`/`ash` turned out to arrive as generic
`Op::Call`, not `CallBuiltinSym`, and `vectorp` turned out to be un-inlinable
as a tag test).

## 2. The profiler (already in-tree)

- `vm-profile` cargo feature: `vm_profile::bump(op)` per dispatched op
  (execution-weighted op histogram) + `bump_subr(SymId)` hooked at
  `subr_entry_from_value` (eval.rs — the single resolver every subr dispatch
  funnels through: tree-walk eval, `Op::Call` funcall, and `CallBuiltinSym`
  via `funcall_general`). `dump(label)` prints both the OP-MIX and the
  SUBR-MIX (per-builtin call ranking with names).
- Drivers: `vm_op_mix_loop`, `vm_op_mix_real_elisp`, `vm_subr_mix_byte_compile`
  (eval_test.rs, `#[ignore]`d, run with
  `cargo nextest run -p neovm-core --features vm-profile --release
  --run-ignored ignored-only --no-capture -E 'test(NAME)'`).
- IMPORTANT bias controls: set `NEOVM_JIT=0` for profiling runs (a tiered-up
  body bypasses the interpreter's bump sites, silently dropping counts), and
  note that the ~13 VM-special names in `dispatch_vm_builtin_unrooted`
  (call-interactively/kbd-macro/garbage-collect/mapatoms/maphash/...) bypass
  `subr_entry_from_value` — they are interactive plumbing, not intrinsic
  candidates, but remember the blind spot when reading the numbers.

## 3. Round-2 profiling protocol (the new work)

The gap: no in-test workload emulates interactive editing. Three options, do
at least (a)+(b):

(a) **Batch-scripted editing session** against the real binary: build via
`cargo xtask fresh-build --release` with the `vm-profile` feature threaded
into neomacs-bin (check whether the bin forwards the feature to neovm-core;
add a feature passthrough if not — small Cargo.toml change), then run
`neomacs --batch -l <script>` where the script: opens a large .el buffer,
performs N thousand mixed operations (forward-char/line, search-forward,
insert/delete, indent-for-tab-command, font-lock-ensure over regions,
query-replace loop), then dumps the profile (add an env-gated dump-at-exit
hook next to the existing dump fn, or expose a `neovm--dump-vm-profile`
debug subr — 20 lines, diagnostics-only).
(b) **jit-lock/font-lock chunked pass** in-test: the regex bench work built a
256KiB real-elisp buffer harness (`regex_test.rs` fontlock benches); wrap the
same buffer in a driver that runs `font-lock-fontify-region` chunk-by-chunk
(as jit-lock does) with the profiler on — this is the highest-value single
workload for editor-feel.
(c) If feasible, a real GUI session profile (needs the display; optional).

Deliverable: a merged ranking table (calls x est. per-call dispatch cost) with
the CallBuiltinSym-vs-Op::Call split per builtin (matters: they enter the JIT
through DIFFERENT lowerings — see §4).

## 4. Architecture recap (what round 1 built — reuse it all)

- Site detection lives in `find_spec_sites` (jit/compile.rs) which classifies
  constant-symbol `Op::Call` sites: `SpecCalleeKind::{Bytecode, SubrGeneral,
  PredRecordp, PredSymbolWithPos, EqInclProps}`. Constraints (all load-bearing,
  from the round-1 triple critique): fixed-arity A0..A8 only (never
  Many/ManySlice — the interpreter passes those the exact-length vector);
  `min <= n <= max` (arity signals must reach the generic path); excluded:
  `mutates_first_arg_name` writeback builtins (aset/fillarray + ALIASES
  through the function cell), `funcall`/`apply`/`eval`, SpecialForm /
  ContextCallable dispatch kinds, symbols while
  `compiler_function_overrides_active()`.
- Baked per site: sym, the stable subr-object VALUE bits (subrs are
  Box::leak'd and never move — NEVER bake the SubrFn pointer or a table
  index: subr entries are REWRITTEN IN PLACE keeping value bits identical),
  a SpecSlot (epoch), and the kind.
- Shims (all `maybe_quit()` FIRST): `neovm_jit_call_subr_spec` (general —
  fresh `subr_entry_from_value` read per call, full stack-protocol parity via
  `Vm::call_spec_subr_stack` incl. backtrace frame + depth guard + arity
  signal + A0..A8 nil-padding), `neovm_jit_pred_spec` (register args, NO
  rooting — the fast path must stay lisp-allocation-free; this is a standing
  reviewer invariant), `neovm_jit_eq_incl_props_spec` (bit-eq prefix).
  Epoch mismatch -> re-validate -> RE-ARM (one re-read per site per epoch
  bump; unrelated defuns do NOT permanently degrade sites). Non-subr rebind ->
  `STATUS_NEED_GENERIC` -> the per-site generated fallback block re-runs the
  original generic call.
- Harness: `NEOVM_JIT_FORCE_SLOW_SPEC=1` forces every spec shim's slow branch
  (run the differential gate under it); `SUBR_SPEC_{COUNT,FAST,GENERIC}`
  counters for engagement tests.
- The differential gate: `NEOVM_JIT_THRESHOLD=1` over
  `-E 'test(/jit|bytecode|eval::tests::/)'` (plus FORCE_SLOW_SPEC and
  FORCE_DEOPT variants; the FORCE_DEOPT failure set has ~29 documented
  expectation-tests — compare failure SETS against base, not zero).

## 5. Candidate treatments (decide per profiled winner)

Tier A — new Pred-style inline shims (only for provably-trivial semantics):
- Candidates from round-1 leftovers: NONE safely — `keywordp`/`symbolp`
  consult `symbols_with_pos_enabled` (excluded), `vectorp` has the
  bool-vector/char-table sentinel divergence (General-only, proven).
  Realistic new Tier-A: `bolp`/`eolp`/`point` IF the profile ranks them —
  they read current-buffer position state through ctx; a dedicated shim that
  skips resolution+dispatch (like pred shims) is the right shape; a true
  inline CLIF read of buffer fields requires stable vmctx offsets
  (NOT built — see doc 09; do not attempt casually).
- `gethash` fast path: guard on eq-test tables with fixnum/symbol keys ->
  inline `to_eq_key` + FxHashMap probe is NOT inlinable in CLIF (hashbrown),
  but a DEDICATED shim `neovm_jit_gethash_eq(ctx, tbl, key, dflt, out)` that
  skips funcall dispatch AND the generic hash-key boxing is a legitimate
  middle tier. Same for `puthash` (barrier-aware! it mutates — must fire
  note_heap_write exactly as the builtin does; route through the same
  internal fn, don't reimplement).
- `fboundp`: obarray function-cell read through ctx — shim-tier, trivial.
- `match-beginning`/`match-end`: read the match-data registers through ctx —
  shim-tier, trivial, likely high-frequency in font-lock support code.
- `memq`/`assq`: already have DEDICATED OPS (`Op::Memq`/`Op::Assq`) lowered
  via the direct builtin table when they come from the byte-compiler; they
  only reach the spec path via generic funcall — check the profile's split
  before spending effort.
- `insert`/`goto-char`/`forward-char`: CallBuiltinSym population — these
  enter through `Op::CallBuiltinSym` lowering (`neovm_jit_named_builtin`),
  NOT find_spec_sites. Intrinsifying them = a parallel classification in the
  CallBuiltinSym lowering (same shim architecture, sites keyed by the op's
  SymId — remember the AOT rule: op-SymIds must reloc by name under AOT; the
  named-builtin lowering already has the reloc_index-keyed pattern to copy).
  These are buffer-mutation ops — writeback/barrier semantics apply; treat
  each as General-tier (dispatch skip only) unless the per-op audit proves
  more.

Tier B — General-tier registration only (dispatch skip): anything hot that is
a plain Builtin with fixed arity and none of the exclusions.

## 6. Implementation plan

1. Profiling per §3; produce the ranking + the per-builtin op-entry split.
2. Adjudicate the top ~10: for each, classify Tier A shim / Tier B general /
   excluded-with-reason (use §5; write the per-builtin semantics audit like
   round 1 did — the vectorp/keywordp exclusions came from exactly this).
3. If any CallBuiltinSym-population op is targeted, design the CallBuiltinSym
   spec-site extension (mirror find_spec_sites' constraints; keep AOT
   exclusion automatic by gating on `Some(obarray)`).
4. TWO-CRITIC review of the adjudication + any new shim semantics (the round-1
   critics caught: in-place subr rewrites, writeback aliases, min-arity
   signals, Many-padding, SWP-flag dependence — expect the same class here,
   especially for buffer-state ops).
5. Implement in small commits (one builtin-group per commit), each with:
   engagement counter tests, parity tests incl. edge semantics, redefinition/
   re-arm tests, the differential gate matrix, and a dedicated microbench
   (`jit_bench_*` pattern) plus the (b) font-lock-chunk macro-bench
   before/after.
6. Re-run the profitability re-measure afterward (gate-ON vs gate-OFF vs
   JIT-off on the bootstrap-load workload): if gate-OFF finally wins, loosen
   `body_is_jit_profitable` per doc 09 §5; otherwise keep and record.

## 7. Gates

Per commit: the differential matrix (THRESHOLD=1 plain / +FORCE_SLOW_SPEC /
+FORCE_DEOPT set-compare) on `-E 'test(/jit|bytecode|eval::tests::/)'`;
`-E 'test(/jit/)'` plain; clippy-clean diff. Full suite at integration.
Honest reporting: per-bench table + the engagement counters from a real
workload run (a shim that never engages on real traffic is dead weight —
remove it).

## 8. Traps recorded from round 1 (do not relearn)

- Bake the subr VALUE, never the fn pointer; read the entry FRESH each call.
- The pred-shim fast path must never allocate on the lisp heap (call sites
  skip residual rooting — adding an allocation is a silent UAF; there is a
  reviewer invariant comment at the shims).
- `equal-including-properties`' in-shim fallback call needed rooting; the
  round-1 fix routes misses to NEED_GENERIC instead — keep that shape for any
  new shim that can call back into real builtins.
- STATUS codes are salted into the AOT ABI_TAG by explicit enumeration in
  `compute_abi_tag` — adding a status REQUIRES adding it there (round 1's
  "automatic" assumption was wrong).
- nextest `or` filtersets silently drop clauses — single-regex filters only.
