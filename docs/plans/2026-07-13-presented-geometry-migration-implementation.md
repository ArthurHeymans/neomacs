# Presented Geometry Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Finish the migration to one immutable, presentation-scoped geometry authority shared by GNU compatibility queries, pointer input, child frames, scrolling, rendering, and diagnostics, then verify the original Corfu/Treemacs failure in a real Weston session.

**Architecture:** Redisplay remains the sole producer of geometry. At the accepted redisplay commit it publishes one immutable `PresentedGeometry` containing frame ancestry, window outer and semantic region rectangles, rows, positions, cursors, interactions, and scale facts under the same `PresentationId` already sent to the renderer. Consumers use sealed semantic queries; compatibility adapters alone convert results to GNU Lisp values or backend device pixels. Transitional snapshot views are removed as their consumers migrate so this does not become a parallel geometry representation.

**Tech Stack:** Rust, neovm evaluator, Neomacs layout/display protocol/runtime, GNU Emacs Lisp compatibility layer, `cargo nextest`, Weston, `ydotool`, screenshot/log evidence.

---

## Confirmed public test seams

1. `GeometryStore::publish` plus sealed `resolve` queries.
2. Loaded GNU `lisp/window.el`, including `window-inside-pixel-edges`.
3. Presentation-scoped semantic hit testing from frame logical coordinates.
4. GNU immediate-parent-relative child placement and nested ancestry composition.
5. Pure viewport planning from presented visual rows.
6. Real Neomacs GUI behavior under Weston with Corfu and a left side window.

Every implementation task below is one vertical RED → GREEN slice and ends in a commit. Never use `cargo test`; use `cargo nextest`.

### Task 1: Publish immutable explicit window geometry

**Files:**
- Modify: `neovm-core/src/window/geometry.rs`
- Modify: `neovm-core/src/window/mod.rs`
- Modify: `neomacs-layout-engine/src/window_output.rs`
- Modify: `neomacs-layout-engine/src/display_buffer_source_tail_render.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: `neovm-core/src/window/window_test.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Steps:**

1. Add a failing presentation test using a left side window and a main window. Assert one publication contains independent stored cell origin plus frame-relative outer, text-body, margin, fringe, scrollbar, chrome, and divider rectangles.
2. Run the focused test with `cargo nextest`; confirm the current snapshot lacks these regions.
3. Introduce `PresentedGeometry`, `PresentedFrame`, `PresentedWindow`, and `WindowRegions` using the typed spaces already in `window/geometry.rs`. Store frame/window ownership and `PresentationId`; do not borrow mutable live `Window` geometry after publication.
4. Extend the accepted layout commit to publish all regions from measured layout facts. Encode margins as both columns and bounds; encode scrollbars by actual side; preserve fringes-inside/outside-margins ordering.
5. Assert renderer output and evaluator geometry use the same presentation and that a later layout cannot mutate the older presentation.
6. Run focused core/layout nextest suites and commit.

### Task 2: Add sealed semantic geometry queries and GNU adapter

**Files:**
- Modify: `neovm-core/src/window/geometry.rs`
- Modify: `neovm-core/src/emacs_core/window_cmds/mod.rs`
- Modify: `neovm-core/src/emacs_core/xdisp.rs`
- Test: `neovm-core/src/emacs_core/window_cmds_test.rs`
- Test: `neovm-core/src/emacs_core/xdisp_test.rs`

**Steps:**

1. Add a failing loaded-GNU-Lisp test with nonzero X and Y origins, margins, fringes, left/right scrollbars, header/tab/mode lines, and dividers. Assert `window-pixel-left/top`, `window-inside-pixel-edges`, and `posn-at-point` agree with known literal geometry.
2. Add sealed queries for window geometry, region bounds, and position geometry. Require the requested presentation and return structured stale/missing/position-not-visible errors.
3. Implement a thin GNU adapter: pixel primitives read outer bounds, cell primitives read stored cell facts, body queries read explicit `text_body`, and position queries return body-local coordinates.
4. Remove the migrated duplicate border/fringe/margin/chrome arithmetic and live-window/snapshot composition.
5. Run focused and complete window/xdisp nextest groups and commit.

### Task 3: Route pointer hit testing through presented regions

**Files:**
- Modify: `neomacs-display-protocol/src/presented_pointer.rs`
- Modify: `neomacs-layout-engine/src/hit_test.rs` or the current hit-data producer
- Modify: `neomacs-display-runtime/src/render_thread/state.rs`
- Modify: `neomacs-display-runtime/src/render_thread/frame_windows.rs`
- Modify: `neovm-core/src/keyboard.rs`
- Tests: corresponding protocol, layout, runtime, and keyboard test modules

**Steps:**

1. Add failing render→hit-test round-trip tests for text, left/right margins, fringes, both scrollbar sides, tab bar, mode/header lines, dividers, and child frames. Include stale-presentation rejection.
2. Convert device coordinates to root logical coordinates exactly once at the backend boundary.
3. Resolve `HitTest` against the same presentation regions, clips, z-order, position index, and interaction IDs used for rendering.
4. Preserve the canonical mixed-face pointer appearance behavior already developed in commit `a03732ecb` only if it is not already represented by equivalent commits on `main`; do not cherry-pick blindly.
5. Delete migrated parallel hit rectangles/maps once all consumers use the query result.
6. Run protocol/layout/runtime/core nextest groups and commit.

### Task 4: Route child frames and popup placement

**Files:**
- Modify: `neovm-core/src/window/geometry.rs`
- Modify: `neovm-core/src/window/mod.rs`
- Modify: `neomacs-layout-engine/src/neovm_bridge.rs`
- Modify: `neomacs-display-protocol/src/frame_glyphs.rs` and/or frame geometry types
- Modify: `neomacs-display-runtime/src/render_thread/frame_windows.rs`
- Tests: core/layout/protocol/runtime child-frame tests

**Steps:**

1. Add failing tests for a child under a root and a nested child. Assert Lisp `left/top` remain immediate-parent-relative and root composition applies each ancestor exactly once.
2. Add `PlaceChild`/frame ancestry queries returning parent-relative and derived root-relative rectangles from one presentation.
3. Route compositor and nested hit testing through these queries. Remove misleading fields that store root coordinates under parent-relative names.
4. Add a popup anchor invariant: presented body origin plus body-local glyph/cursor equals the frame anchor; Corfu’s requested child rectangle is then applied unchanged relative to its immediate parent.
5. Run child-frame nextest groups and commit.

### Task 5: Make logical/device scaling explicit

**Files:**
- Modify: geometry protocol/backend types selected by Task 4
- Modify: `neomacs-display-runtime` Wayland/X11 surface adapters
- Modify: renderer damage/clip conversion sites
- Tests: protocol/runtime scale and rounding tests

**Steps:**

1. Add failing fractional-scale tests proving edges are rounded first and size is derived as `right - left` / `bottom - top`.
2. Add typed logical-to-device conversion only at the native surface seam. Reject nonpositive/nonfinite scales.
3. Route Wayland and X11 through the adapter; ensure layout, Lisp, pointer, and popup policy remain logical.
4. Remove independent origin/size rounding at migrated call sites.
5. Run runtime/renderer nextest groups and commit.

### Task 6: Route scrolling and visibility through presented rows

**Files:**
- Modify: `neovm-core/src/window/geometry.rs`
- Modify: `neovm-core/src/emacs_core/window_cmds/mod.rs`
- Modify: `neovm-core/src/emacs_core/xdisp.rs`
- Modify: row publication in `neomacs-layout-engine`
- Tests: window command and xdisp scrolling/visibility tests

**Steps:**

1. Add failing previous/next-page and visibility tests using variable-height rows, clipping, continuation rows, and a side window.
2. Publish row frame/body bounds, buffer spans, continuation state, and clipping.
3. Add pure `PlanViewport` and visibility queries. The evaluator applies returned window-start/vscroll mutations; geometry remains immutable.
4. Route page movement, recenter, and visibility predicates through presented rows where authoritative geometry exists. Keep a named TTY/batch approximation adapter only where GNU has no GUI presentation.
5. Remove migrated row rescans and duplicate viewport geometry.
6. Run scrolling/xdisp nextest groups and commit.

### Task 7: Diagnostics and deletion pass

**Files:**
- Modify: `neovm-core/src/window/geometry.rs`
- Modify: `neomacs-layout-engine` dump output
- Modify/delete: obsolete hit maps, snapshot arithmetic, and child absolute-position helpers found by `rg`
- Tests: geometry explanation and stale-presentation tests

**Steps:**

1. Add a failing `explain` test with the known Treemacs/Corfu numbers and provenance labels.
2. Emit structured presentation/owner/space/provenance diagnostics into `NEOMACS_DUMP_FRAME_GLYPHS` output.
3. Delete superseded paths rather than retaining permanent dual authority.
4. Use `rg` to prove migrated consumers no longer reconstruct geometry from character metrics or unrelated scalar fields.
5. Run affected nextest groups and commit.

### Task 8: Review and automated verification

**Steps:**

1. Run two-axis spec and code-quality review; fix and re-review all findings.
2. Run focused package nextest suites after each fix.
3. Run the broad workspace `cargo nextest` suite. Record unrelated baseline failures with exact evidence; do not hide them.
4. Confirm `git diff --check` and a clean worktree after commits.

### Task 9: Weston/ydotool end-to-end verification

**Steps:**

1. Run exactly `cargo xtask fresh-build --release`.
2. Start an isolated Weston session and launch the freshly built release Neomacs with frame-glyph diagnostics enabled.
3. Use `ydotool` to create the original layouts and type commands. Capture screenshots and logs for:
   - Corfu without a side window;
   - Corfu with Treemacs/left side window;
   - margins/fringes/scrollbars and a window above;
   - pointer hits and hover changes;
   - nested child positioning where available;
   - page up/down behavior.
4. Compare cursor bottom and child top-left numerically from diagnostics; verify the side-window translation is applied equally.
5. Save concise verification evidence under `/tmp` and report exact commands, screenshots, logs, commits, and any remaining limitations.
