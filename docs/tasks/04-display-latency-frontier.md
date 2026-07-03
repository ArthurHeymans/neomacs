# Task 04 — Display-side latency (the true keystroke frontier) — pointer doc

Status: POINTER DOC — the work lives OUTSIDE neovm-core (neomacs-layout-engine,
neomacs-renderer-wgpu, neomacs-bin, display-protocol). This file exists so the
VM roadmap directory tells the whole truth: after the 2026-07 VM work, the VM
is sub-millisecond everywhere that matters, and keystroke-to-pixel latency is
now decided by layout + GPU presentation, not by elisp execution or GC.

## 1. The honest system-level frame (evidence)

- VM/JIT: ~us per keystroke; predicate-class builtin calls 7.9x; font-lock's
  regex pass 10.1x faster (14.5ms per FULL 256KiB pass — and real
  fontification is jit-lock CHUNKED, so per-keystroke regex cost is a small
  fraction of that).
- GC: start handshake 134us (pdump), termination lump ~134-210us, drain
  ~180-220us, zero full-STW after bootstrap, adaptive pacer. GC cannot drop a
  frame anymore in the shipping config.
- Layout: the redisplay layout cycle was measured at ~1.2ms/cycle
  full-rebuild before the incremental-layout project; incremental phases
  0a/0b/1/2/3/#45/parts-of-4/5-step-1 are DONE on main (cursor-only,
  pure-scroll, localized-edit fast paths; per-row damage tags in the
  protocol). What remains blocked/open there:
  - **Phase 5 step 2**: diff-aware materialize + wgpu area-blit (GPU-resource
    reuse). BLOCKED HEADLESS — needs a live display to verify honestly.
  - **Smooth-scroll vscroll render gap (task #64 in that roadmap)**: neomacs
    vscroll only shrinks text_height and never offsets content-Y, so
    sub-line scroll motion isn't actually rendered — a pre-existing gap that
    the smooth-scroll input pipeline (landed) exposes.
  - Parallel window layout (evidence-gated; rayon-style true parallelism was
    assessed DEFER previously).

## 2. What a session on this should do

1. Read the incremental-layout memory notes (project memory:
   `incremental-layout-*` entries) and the layout-engine test goldens — the
   working method there (byte-identical goldens + relaid-row-count
   instrumentation) is established and non-negotiable.
2. Requires a GUI-capable environment (a real display or the GUI test harness
   with frame-snapshot artifacts that landed upstream in the
   `feat(gui-tests): frame-snapshot artifacts...` commits — those may make
   parts of Phase-5-step-2 verifiable without a physical display now; check
   `docs/` GUI agent testing guide referenced by commit `5c0205ae0`).
3. Priority within display work, per the measurements: vscroll render gap
   (#64) first (it blocks the already-landed smooth-scroll feature from being
   visible), then Phase 5 step 2 (GPU blit), then parallel layout only with
   fresh profiling.

## 3. Why this doc is in the VM roadmap

Every future "make neomacs feel faster" request should check this frontier
FIRST. The VM-side backlog (tasks 01-03, 05-12) improves throughput,
robustness, and tail latencies — but the median keystroke's critical path is
now: input -> (VM: ~us) -> layout classify/fast-path (~0.1-1.2ms) -> protocol
-> GPU materialize/present. The biggest single remaining win in that chain is
render-side damage reuse, not anything in neovm-core.
