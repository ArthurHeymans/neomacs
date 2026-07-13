# Presentation Transaction Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace independently published display geometry, render state, and interaction metadata with an explicit presentation transaction whose specialized projections are prepared together, activated together, and retired safely.

**Architecture:** GNU-compatible `Frame`/`Window` objects remain the synchronous logical model. Redisplay builds a pending presentation identified by `PresentationId`; the render thread explicitly activates that presentation for drawing and hit testing, while physical presentation remains a later optional outcome. Render, geometry, source-map, and interaction projections may use different storage, but must share one identity and canonical transform/clip compilation.

**Tech Stack:** Rust, crossbeam channels, winit/wgpu, neovm evaluator, `FrameDisplayState`, typed geometry, `cargo nextest`.

---

## Constraints and confirmed seams

- Work directly on `main` in the current workspace; the user explicitly rejected a worktree.
- Run tests only with `cargo nextest`; never use `cargo test`.
- Keep commits small and behavior-oriented.
- Logical GNU queries cross the `Frame`/`Window` seam and never require a presentation.
- Exact visual queries cross an active-presentation seam and require `PresentationId`.
- The render thread hit-tests only its active presentation.
- Pointer events carry the identity of the presentation used for hit testing.
- Never introduce `geometry(window, prefer_presented)` or another implicit fallback interface.
- Resource caches are implementation details, not semantic presentation state.

## Target lifecycle

```text
logical revision R
        |
        v
Prepared presentation N
        |
        +-- send rejected/superseded --> Discarded N
        |
        v
Active presentation N
        |
        +-- optional platform feedback --> Presented N
        |
        v
Retired N
```

`Active` means the renderer uses N for both drawing and hit testing. It does
not claim physical scanout. A future Wayland presentation-feedback adapter may
produce `Presented` or `Discarded` timing outcomes without changing this seam.

### Task 1: Commit the researched architecture

**Files:**
- Create: `docs/research/gui-scene-pipeline-practices.md`
- Create: `docs/plans/2026-07-13-presentation-transaction-refactor.md`

**Step 1: Verify the documents name distinct logical, prepared, active, and presented states**

Run:

```bash
rg -n "Logical|Prepared|Active|Presented|Discarded" \
  docs/research/gui-scene-pipeline-practices.md \
  docs/plans/2026-07-13-presentation-transaction-refactor.md
```

Expected: both documents describe the lifecycle and frame-matched interaction invariant.

**Step 2: Commit**

```bash
git add docs/research/gui-scene-pipeline-practices.md \
  docs/plans/2026-07-13-presentation-transaction-refactor.md
git commit -m "docs(display): define transactional presentation lifecycle"
```

### Task 2: Add renderer-to-evaluator activation and discard events

**Files:**
- Modify: `neomacs-display-runtime/src/thread_comm.rs`
- Modify: `neomacs-bin/src/input_bridge.rs`
- Modify: `neovm-core/src/keyboard.rs`
- Modify: `neovm-core/src/frontend_events.rs`
- Test: `neomacs-display-runtime/src/thread_comm_test.rs`
- Test: `neovm-core/src/frontend_events.rs`
- Test: `neovm-core/src/keyboard_test.rs`

**Step 1: Write failing event-semantics tests**

Add tests specifying that:

```rust
InputEvent::PresentationActivated {
    presentation: 42,
    emacs_frame_id: 0x1_0000_0000,
}
```

and `PresentationDiscarded` are lossless, non-command internal events. Neither
event may enter the Lisp key sequence.

**Step 2: Run the focused tests and verify RED**

```bash
cargo nextest run -p neovm-core -p neomacs-display-runtime \
  'presentation_activated|presentation_discarded'
```

Expected: compile failure because the variants do not exist.

**Step 3: Implement the minimal event path**

Add:

```rust
PresentationActivated {
    presentation: u64,
    emacs_frame_id: u64,
},
PresentationDiscarded {
    presentation: u64,
    emacs_frame_id: u64,
},
```

to runtime and evaluator input enums, convert them in `input_bridge`, classify
them as internal non-command events, and initially route them to no-op context
methods. Do not emit either event yet.

**Step 4: Run focused and crate tests**

```bash
cargo nextest run -p neovm-core -p neomacs-display-runtime
```

Expected: PASS.

**Step 5: Commit**

```bash
git add neomacs-display-runtime/src/thread_comm.rs \
  neomacs-bin/src/input_bridge.rs neovm-core/src/keyboard.rs \
  neovm-core/src/frontend_events.rs
git commit -m "feat(display): carry presentation activation lifecycle events"
```

### Task 3: Introduce evaluator pending and active presentation state

**Files:**
- Modify: `neovm-core/src/window/geometry.rs`
- Modify: `neovm-core/src/window/mod.rs`
- Test: `neovm-core/src/window/window_test.rs`

**Step 1: Write a failing lifecycle test**

Through the public `Frame` interface, specify:

1. Preparing N does not change `active_presentation()`.
2. Activating N makes its geometry active atomically.
3. Preparing N+1 leaves N active.
4. Discarding N+1 leaves N active.
5. Activating an unknown/discarded identity returns a typed error.
6. Presentation identities cannot be reused even after discard or retirement.

**Step 2: Verify RED**

```bash
cargo nextest run -p neovm-core \
  'prepared_presentation_does_not_replace_active|discarded_presentation_cannot_activate'
```

Expected: compile failure because the lifecycle interface does not exist.

**Step 3: Implement `FramePresentationState`**

Replace the independent `presented_geometry`/`last_presentation` fields with a
private state holder conceptually shaped as:

```rust
struct FramePresentationState {
    prepared: BTreeMap<PresentationId, PresentedGeometry>,
    active: Option<PresentedGeometry>,
    last_identity: Option<PresentationId>,
}
```

Expose intent-specific operations:

```rust
fn prepare_display_presentation(...)
    -> Result<(), PresentationPrepareError>;
fn activate_display_presentation(PresentationId)
    -> Result<Option<PresentationId>, PresentationActivateError>;
fn discard_display_presentation(PresentationId) -> bool;
fn active_presentation(&self) -> Option<PresentationId>;
fn active_presented_geometry(&self) -> Option<&PresentedGeometry>;
```

Keep at most a small bounded set of in-flight prepared presentations. Do not
silently activate the newest one. Preserve the old compatibility getter only
until Task 7 migrates its callers.

**Step 4: Verify GREEN**

```bash
cargo nextest run -p neovm-core 'window::window_test'
```

Expected: PASS.

**Step 5: Commit**

```bash
git add neovm-core/src/window/geometry.rs neovm-core/src/window/mod.rs
git commit -m "refactor(window): separate prepared and active geometry"
```

### Task 4: Emit lifecycle events when the renderer changes active frames

**Files:**
- Modify: `neomacs-display-runtime/src/render_thread/frame_windows.rs`
- Modify: `neomacs-display-runtime/src/render_thread/frame_ingest.rs`
- Modify: `neomacs-display-runtime/src/render_thread/tests.rs`
- Modify: `neomacs-display-runtime/src/render_thread/input_test.rs`

**Step 1: Write failing transition tests**

Specify that installing a frame emits `PresentationActivated` exactly once;
replacing it activates the new frame and retires the old; a received frame
that is superseded before activation emits `PresentationDiscarded` rather than
`PresentationRetired`.

**Step 2: Verify RED**

```bash
cargo nextest run -p neomacs-display-runtime \
  'installing_frame_emits_activation|superseded_pending_frame_is_discarded'
```

**Step 3: Implement the transition result**

Make `set_current_frame` return a typed transition containing the activated and
replaced identities. `poll_frame` sends lifecycle events in channel order.
Pointer-capture retirement remains deferable, but activation is not deferred.

**Step 4: Verify GREEN**

```bash
cargo nextest run -p neomacs-display-runtime
```

**Step 5: Commit**

```bash
git add neomacs-display-runtime/src/render_thread
git commit -m "feat(runtime): report active presentation transitions"
```

### Task 5: Build and submit a prepared presentation atomically

**Files:**
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/lib.rs`
- Modify: `neomacs-bin/src/tty_layout.rs`
- Modify: `neomacs-bin/src/main.rs`
- Test: `neomacs-bin/src/main_test.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Step 1: Write the failing send-rejection test**

Fill or disconnect `frame_tx`, call `publish_gui_frame`, and assert that the
frame has no prepared or active geometry for the rejected presentation. Also
assert that successful submission leaves it prepared but not active.

**Step 2: Verify RED**

```bash
cargo nextest run -p neomacs-bin \
  'rejected_gui_frame_is_not_published_as_presented|submitted_gui_frame_is_only_prepared'
```

Expected: the first assertion fails because layout currently calls
`publish_display_snapshots` before `try_send`.

**Step 3: Introduce `PreparedFramePresentation`**

Have layout return or retain one value containing:

```rust
pub struct PreparedFramePresentation {
    pub display_state: FrameDisplayState,
    pub window_output: Vec<WindowDisplaySnapshot>,
}
```

Layout must not install geometry in `Frame`. `publish_gui_frame` prepares the
evaluator geometry and submits the matching `FrameDisplayState` as one
operation; failed sends immediately discard the prepared identity. TTY
rendering activates synchronously because layout and rasterization share one
thread.

**Step 4: Verify GREEN**

```bash
cargo nextest run -p neomacs-bin -p neomacs-layout-engine -p neovm-core
```

**Step 5: Commit**

```bash
git add neomacs-layout-engine neomacs-bin neovm-core
git commit -m "refactor(redisplay): submit one prepared presentation transaction"
```

### Task 6: Activate and discard evaluator geometry from internal events

**Files:**
- Modify: `neovm-core/src/keyboard.rs`
- Modify: `neovm-core/src/window/mod.rs`
- Test: `neovm-core/src/keyboard_test.rs`
- Test: `neovm-core/src/emacs_core/eval_test.rs`

**Step 1: Write failing ordered-event tests**

Prepare N and N+1, enqueue activation N followed by activation N+1 and
retirement N, and assert that exact visual queries switch only at activation.
Discarding a pending identity must not disturb the active one.

**Step 2: Verify RED**

```bash
cargo nextest run -p neovm-core \
  'activation_event_switches_active_geometry|discard_event_preserves_active_geometry'
```

**Step 3: Implement event handling**

Resolve `emacs_frame_id`, call the corresponding `Frame` lifecycle operation,
and log stale/unknown protocol events without panicking. Retirement releases
interaction registrations only after ordered pointer events have drained.

**Step 4: Verify GREEN and commit**

```bash
cargo nextest run -p neovm-core
git add neovm-core
git commit -m "feat(evaluator): follow renderer presentation activation"
```

### Task 7: Split redisplay cache from presentation geometry

**Files:**
- Modify: `neovm-core/src/window/mod.rs`
- Modify: `neomacs-layout-engine/src/incremental_layout.rs`
- Modify: `neomacs-layout-engine/src/window_output.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: `neovm-core/src/window/window_test.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Step 1: Write a failing cache-independence test**

Prove that replacing/recycling redisplay cache state cannot change active
presentation geometry, and retiring presentation geometry cannot erase the
cursor/row data required for incremental redisplay.

**Step 2: Verify RED**

```bash
cargo nextest run -p neovm-core -p neomacs-layout-engine \
  'redisplay_cache_is_independent_from_active_geometry'
```

**Step 3: Split the types**

Introduce evaluator-owned `WindowRedisplayOutput` for cursor progress and
incremental row reuse. Introduce a smaller immutable
`WindowPresentationSnapshot` for regions, body rows, and visible source
positions. Convert once when sealing the presentation transaction.

**Step 4: Verify and commit**

```bash
cargo nextest run -p neovm-core -p neomacs-layout-engine
git add neovm-core neomacs-layout-engine
git commit -m "refactor(redisplay): separate output cache from scene geometry"
```

### Task 8: Compile visual and interaction projections through one spatial plan

**Files:**
- Create: `neomacs-layout-engine/src/presentation_spatial.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-display-protocol/src/glyph_matrix.rs`
- Modify: `neomacs-display-protocol/src/presented_pointer.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`
- Test: `neomacs-display-protocol/src/presented_pointer_test.rs`

**Step 1: Write failing production-pipeline tests**

Cover nested child frames, side windows, margins, fringes, scrollbars, header
and tab lines. For each fixture, assert that the rendered rectangle and
hit-test/source rectangle are projections of the same canonical transform and
clip chain.

**Step 2: Verify RED**

```bash
cargo nextest run -p neomacs-layout-engine -p neomacs-display-protocol \
  'presentation_spatial'
```

**Step 3: Implement one builder with specialized outputs**

The builder owns frame hierarchy, local-to-parent transforms, window regions,
and ancestor clips. Render, hit-test, and source-map projections consume its
sealed output. They may retain different indexes and storage.

**Step 4: Verify and commit**

```bash
cargo nextest run -p neomacs-layout-engine -p neomacs-display-protocol
git add neomacs-layout-engine neomacs-display-protocol
git commit -m "refactor(display): compile scene projections from one spatial plan"
```

### Task 9: Migrate visual queries and semantic popup anchors

**Files:**
- Modify: `neovm-core/src/emacs_core/window_cmds/mod.rs`
- Modify: `neovm-core/src/emacs_core/xdisp.rs`
- Modify: `neovm-core/src/window/geometry.rs`
- Modify: `neomacs-display-runtime/src/thread_comm.rs`
- Modify: child-frame placement code in `neomacs-layout-engine` and `neomacs-display-runtime`
- Test: `neovm-core/src/emacs_core/window_cmds_test.rs`
- Test: `neovm-core/src/emacs_core/xdisp_test.rs`
- Test: GUI geometry integration tests

**Step 1: Write failing seam tests**

Logical `window-pixel-*` queries must succeed before any presentation. Exact
`posn-at-point`, hit testing, and visual anchors must either resolve against an
explicit active presentation or report `NotYetActive`/`StalePresentation`.

**Step 2: Introduce semantic anchors**

```rust
enum VisualAnchor {
    CursorBottom { window: WindowId },
    BufferPositionBottom { window: WindowId, position: LispCharPos1 },
    WindowRegionEdge { window: WindowId, region: WindowRegion, edge: Edge },
}
```

Popup requests also carry popup anchor, preferred side, offset, and constraint
policy. The active spatial projection resolves parent-relative placement.

**Step 3: Remove compatibility fallback getters**

All callers must choose the logical or active interface explicitly. Delete the
legacy `presented_geometry()` compatibility path once `rg` finds no callers.

**Step 4: Verify and commit**

```bash
cargo nextest run -p neovm-core -p neomacs-layout-engine \
  -p neomacs-display-runtime
git add neovm-core neomacs-layout-engine neomacs-display-runtime
git commit -m "refactor(geometry): make logical and active visual intent explicit"
```

### Task 10: Remove transitional snapshot publication and verify end to end

**Files:**
- Modify: all remaining callers found by `rg 'display_snapshots|replace_display_snapshots|set_display_snapshots|publish_display_snapshots'`
- Modify: `docs/rust-display-engine.md`
- Modify: `docs/research/gui-scene-pipeline-practices.md`

**Step 1: Delete obsolete interfaces**

Remove the compatibility snapshot map and independent publication helpers.
Retain only redisplay cache and presentation projection types with explicit
ownership.

**Step 2: Run formatting and broad verification**

```bash
cargo fmt --all
cargo nextest run --no-fail-fast
```

Expected: all tests pass. Do not use `--release` and do not run `cargo test`.

**Step 3: Run GUI scenarios**

Use headless Weston and the existing GUI harness to verify:

- startup before the first active presentation;
- tab-bar hover/click;
- main-buffer blank-line click placement;
- Treemacs side window plus Corfu child-frame anchor;
- minibuffer/Fido layout;
- child-frame retirement during pointer capture.

**Step 4: Final commit**

```bash
git add -A
git commit -m "refactor(display): complete transactional presentation migration"
```

## Completion criteria

- No evaluator state is called presented before renderer activation.
- A rejected or superseded submission cannot become active geometry.
- Render and hit-test projections always share presentation identity and
  spatial semantics.
- Logical GNU geometry queries work without a presentation.
- Exact visual operations require active presentation state.
- Popup placement is semantic and parent-relative.
- No compatibility `display_snapshots` publication remains.
- All relevant tests pass under `cargo nextest`.
