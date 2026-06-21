# Baseline cross-op fixnum unboxing (path B) — adversarially-verified plan

Goal: in `lower_leaf_full` (neovm-core/src/emacs_core/jit/compile.rs, the LIVE baseline
JIT), keep fixnum values RAW (untagged i64) across consecutive arithmetic ops within a
block, retagging only at boundaries — eliminating intermediate retag+untag pairs. Mirrors
`lower_mir_pure`'s unboxing but on the baseline's per-slot model + (the novel part) its
PRECISE-deopt framestate. Source: design+critique workflow wf_0f59e651-3fa.

## Core invariant (REVISED per critique — the load-bearing rule)
RAW is allowed ONLY on the in-flight model `stack` between consecutive arithmetic ops in
the SAME block, consumed ONLY by arithmetic. **Everything that leaves the block, is
GC-rooted (`gc_push`), is tag-inspected, is passed to a shim/call, is written to `vars`,
or is captured in a deopt/signal snapshot MUST be TAGGED.** Raw resets to all-tagged at
every block entry (no cross-block raw in this increment).

## Representation
- Parallel `stack_raw: Vec<bool>` 1:1 with the model `stack`, indexed BY POSITION (NOT by
  ClifValue — critique H5: by-value would be wrong under Dup/StackRef aliasing). `true` =
  slot holds an untagged i64.
- Seed `stack_raw = vec![false; depth]` where the per-block `stack` is materialized.
- MUST be kept in lockstep with EVERY `stack.push/pop/move` — in `lower_simple_op` AND in
  the `lower_leaf_full` terminator arms (critique H4: terminators pop `stack` too).

## Helpers (near mir_as_raw, ~2204)
- `stack_as_raw(fb, deopt, stack, stack_raw, k, known) -> ClifValue`: if `stack_raw[k]`
  return `stack[k]`; else `guard_fixnum(fb,deopt,stack[k],known)` + `sshr_imm(.., FIXNUM_SHIFT)`.
- `stack_force_tagged(fb, stack, stack_raw, k)`: if raw, `stack[k]=retag_fixnum(..)`, clear flag.
- `retag_all_raw(fb, stack, stack_raw)`: force-tag every slot. Call before EVERY boundary.

## Hot path (arithmetic arms: Add/Sub, Mul, Div/Rem, Add1/Sub1/Negate, Max/Min)
Consume operands via `stack_as_raw`, push raw result via the existing `raw_fixnum_*`
helpers (already present + tested), push `stack_raw=true`. `deopt_site` is called BEFORE
popping (captures pre-op stack) — keep that order. Comparison arm (=,<,>,<=,>=): consume
operands raw, push TAGGED t/nil (stack_raw=false).

## Retag sites (force-tag before the boundary)
1. **CRITICAL (H1+H2): full-stack `retag_all_raw` at the TOP of every arm that calls
   `gc_push` / a call/apply / a shim / `stack_store` to the call buffer / `signal_target_for_site`.**
   That is: VarRef, VarSet, Call/Apply, Cons, VarBind, Unbind, SaveWindowExcursion,
   CallBuiltin*/Aset, List, Switch, Throw. These iterate `stack.iter()` and root the
   NON-operand survivors, which can be raw → UAF. This single rule closes H1 and makes the
   "dispatch gc_push tagged by construction" claim true (H2).
2. **Bit-inspecting ops MUST force-tag, never raw-consume (H3):** Eq, Null/Not,
   Consp/Stringp/Listp, Symbolp/Integerp/Numberp, all GotoIfNil*/ElsePop conditions,
   Switch dispatch/table. (A raw vs tagged copy of the same fixnum have different bits.)
3. **Car/Cdr/CarSafe/CdrSafe:** force-tag operand (tag inspection).
4. **Every terminator:** `retag_all_raw` before `write_stack_to_vars` (Return, Goto incl.
   backedge, GotoIfNil*, *ElsePop, Switch, Push*Handler, fall-through). Then `vars` are
   provably always tagged → no gc_push/backedge/dispatch changes needed.

## Cold-path deopt-framestate retag (the subtle novel part)
- Extend `PendingDeopt` with `stack_raw: Vec<bool>`; `deopt_site` stores `stack_raw.to_vec()`
  alongside `stack`. Update all 7 `deopt_site` callers (arith + car/cdr + max/min).
- In `emit_pending_deopts` (the single framestate store loop, ~2948): before storing each
  slot, `let tagged = if pd.stack_raw[j] { retag_fixnum(fb, v) } else { v };`. This runs in
  the COLD deopt block (after the terminator) → zero hot-path cost. `run_resumed_frame`
  (vm.rs:452) re-pushes these as GC-traced tagged Values → must be tagged. Verified the
  spill is type-confusion-prone if left raw (critique sound-list #1).

## Composition with known-fixnum guard elimination (unchanged)
`stack_as_raw`'s else-branch still passes `known` to `guard_fixnum`, so cross-block
proven-fixnum slots skip their guard AND get untagged once — both wins stack. Do NOT modify
`compute_known_fixnum_slots`/`apply_known_fixnum_op`. `is_known_fixnum` cannot misfire on
raw values (only matches iconst/retag patterns) — verified safe (critique H4 sound dir).

## Verification (all at NEOVM_JIT_THRESHOLD=1, exclude module/cache/compile/env tests)
1. Differential gate (JIT==interp) — catches hot-path mis-tag.
2. Force-deopt (NEOVM_JIT_FORCE_DEOPT=1) — catches a missing/incorrect framestate retag.
3. **Force-SIGNAL test (NEW, critique H2):** raw slot below a protected Call/VarRef +
   trigger a signal → exercises emit_pending_dispatches snapshot. Force-deopt alone does
   NOT cover the dispatch path.
4. Targeted: `[Const a, Const b, Add, Const c, Add, Return]` and chains with a mid-chain
   deopt so the snapshot contains a raw slot below the current op.

## REFINEMENT (tractable + safe via a central chokepoint) — implement this way
Instead of per-arm lockstep across all ~40 arms (error-prone), classify each op:
- RAW-PRESERVING set = arithmetic (Add/Sub/Mul/Div/Rem/Add1/Sub1/Negate/Max/Min) +
  comparison (Eqlsign/Lss/Gtr/Leq/Geq) + shuffles (Constant/Nil/True/Pop/Dup/StackRef/
  StackSet/DiscardN). These ~14 arms handle `stack_raw` EXPLICITLY (push true for fixnum
  arith results, push false for tagged, move/copy flags for shuffles, pop in lockstep).
- EVERYTHING ELSE: at the TOP of lower_simple_op `if !raw_preserving { retag_all_raw(fb,
  stack, stack_raw) }` (now its gc_push/signal-snapshot see only tagged — closes H1+H2 in
  ONE place), and at the BOTTOM `if !raw_preserving { stack_raw.resize(stack.len(), false) }`
  (re-syncs length; all tagged since the arm pushed tagged results). These arms need NO
  per-arm stack_raw edits.
- `debug_assert_eq!(stack.len(), stack_raw.len())` after the match catches any missed lockstep.
- Shuffles MUST be raw-preserving (Constant/StackRef between two Adds is the common case —
  retagging there would defeat the unboxing).
- Only `deopt_site` needs the `stack_raw` param (arith precise-deopt with raw slots below);
  signal snapshots are post-retag_all_raw → already tagged. car/cdr (non-raw) calls deopt_site
  with all-false stack_raw → cold retag is a no-op there.

## Change list (functions / approx lines — re-verify, shifted by recent edits)
helpers ~2204 | PendingDeopt ~2903 | deopt_site ~2912 (+stack_raw) | emit_pending_deopts
~2948 (cold retag) | lower_simple_op sig ~3076 (+ &mut stack_raw) | shuffle arms lockstep
~3094 | arith arms raw ~3146-3293 | force-tag arms (gc/call/bit-inspect) throughout |
seed stack_raw ~5000 | terminator lockstep pops + retag_all_raw ~5028-5326 | call site ~5306.
