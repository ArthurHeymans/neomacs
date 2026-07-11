# Presented Tab-Bar Interactions Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make Neomacs tab-bar mouse input resolve against the exact displayed Lisp snapshot, with GNU-compatible `posn-string`, and relayout when asynchronous image metrics become available.

**Architecture:** Keep `FrameChrome` as the sole geometry owner. Replace tab-index actions with opaque presentation-scoped interaction references; the evaluator retains and GC-traces the Lisp captions, keys, and bindings that those references name. The runtime reports pointer observations, while evaluator-side resolution synthesizes GNU-shaped mouse events. Image dimension completion becomes a layout invalidation that schedules redisplay.

**Tech Stack:** Rust 2024, NeoVM tagged Lisp values and GC tracing, serde display protocol, crossbeam runtime input, Neomacs layout engine, `cargo nextest`.

---

### Task 1: Introduce presentation-scoped hit references

**Files:**
- Modify: `neomacs-display-protocol/src/frame_chrome.rs`
- Modify: `neomacs-display-protocol/src/frame_chrome_test.rs`
- Modify: `neomacs-display-protocol/src/glyph_matrix.rs`
- Modify: `neomacs-display-protocol/src/glyph_matrix_test.rs`

1. Write failing tests proving a frame snapshot carries a `PresentationId` and tab hit regions carry only an opaque `InteractionId`.
2. Run `cargo nextest run -p neomacs-display-protocol frame_chrome` and confirm the new interface is missing.
3. Add checked transparent `PresentationId` and `InteractionId` newtypes, replace `ChromeAction::SelectTab`, and publish the presentation ID with the frame state.
4. Re-run the focused protocol tests and keep materialization/coordinate tests green.

### Task 2: Retain GNU-shaped tab targets in the evaluator

**Files:**
- Modify: `neovm-core/src/keyboard.rs`
- Modify: `neovm-core/src/keyboard_test.rs`
- Modify: `neovm-core/src/emacs_core/eval.rs`
- Modify: `neomacs-layout-engine/src/display_status_line.rs`
- Modify: `neomacs-layout-engine/src/display_status_line_test.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`

1. Write failing tests for an evaluator-owned interaction presentation that GC-traces its values and resolves `(presentation, interaction)` to `(CAPTION . 0)` with `menu-item = (KEY BINDING CLOSE-P)`.
2. Run the focused NeoVM tests with `cargo nextest` and verify the expected failures.
3. Implement the small `PresentedInteractions` registry and include it in `Context::trace_roots`.
4. Extend tab-bar source construction to retain caption/key/binding values and source character ranges.
5. Write a failing layout test proving the close image source slot and tab body receive different opaque targets, while the plus caption retains its `add-tab` binding.
6. Replace whole-caption hit regions with coalesced rendered-source runs and register their GNU-shaped targets before frame publication.
7. Run focused NeoVM and layout-engine tests.

### Task 3: Transport pointer phase and resolve ordinary Lisp events

**Files:**
- Modify: `neomacs-display-runtime/src/thread_comm.rs`
- Modify: `neomacs-display-runtime/src/render_thread/input.rs`
- Modify: `neomacs-display-runtime/src/render_thread/pointer_events.rs`
- Modify: `neomacs-display-runtime/src/render_thread/input_test.rs`
- Modify: `neomacs-bin/src/input_bridge.rs`
- Modify: `neomacs-bin/src/input_bridge_test.rs`
- Modify: `neovm-core/src/keyboard.rs`
- Modify: `neovm-core/src/keyboard_test.rs`

1. Write failing runtime and bridge tests for `PresentedPointer` carrying frame, presentation, interaction, button phase, and coordinates.
2. Verify the tests fail because `TabBarClick { index }` is still the transport.
3. Replace the tab-specific index event across runtime and bridge.
4. Write failing keyboard tests proving press becomes `down-mouse-1`, release becomes `mouse-1`, and position slot four is the retained `(CAPTION . 0)`.
5. Implement resolution through `PresentedInteractions`; do not perform select/close/new policy in Rust.
6. Run all focused runtime, bridge, and keyboard tests.

### Task 4: Retire presentations safely

**Files:**
- Modify: `neomacs-display-runtime/src/thread_comm.rs`
- Modify: `neomacs-display-runtime/src/render_thread/state.rs`
- Modify: `neomacs-bin/src/input_bridge.rs`
- Modify: `neovm-core/src/keyboard.rs`

1. Write a failing registry test showing an older presentation remains resolvable until an explicit retirement event, then releases its roots and safely rejects stale hits.
2. Add `PresentationRetired` after the runtime replaces a displayed snapshot; preserve queue ordering after already-generated pointer events.
3. Route retirement to the evaluator and remove only the named presentation.
4. Run the focused lifecycle tests.

### Task 5: Turn image dimension completion into relayout

**Files:**
- Modify: `neomacs-bin/src/input_bridge.rs`
- Modify: `neomacs-bin/src/input_bridge_test.rs`
- Modify: `neovm-core/src/keyboard.rs`
- Modify: `neovm-core/src/keyboard_test.rs`

1. Write a failing bridge/keyboard test proving `ImageDimensionsReady` is not dropped and is consumed as a non-command display invalidation.
2. Translate the asset-specific runtime notification into a parameterless evaluator `LayoutInvalidated` wakeup. Consuming it returns no Lisp event and explicitly invalidates the redisplay signature, guaranteeing the normal command-loop redisplay pass runs; final metrics remain owned by the runtime asset cache.
3. Verify the next layout request observes the host’s final cached dimensions and publishes matching image and hit geometry.
4. Run focused tests.

### Task 6: Documentation, regression verification, review, and commit

**Files:**
- Modify: `docs/plans/2026-07-11-frame-chrome-design.md`
- Modify: `docs/plans/2026-07-11-frame-chrome-implementation.md`
- Add: `docs/audit/2026-07-11-gnu-emacs-tab-bar-rendering-input.md`

1. Amend the frame-chrome design so hit regions contain opaque interaction references rather than semantic tab commands.
2. Run `cargo fmt --all --check`.
3. Run focused suites with `cargo nextest`, then `cargo nextest run --workspace` if practical.
4. Run `cargo check --workspace`.
5. Reproduce `neomacs -Q`, enable `tab-bar-mode`, and inspect a fresh glyph dump when GUI execution is available.
6. Review the final diff against the GNU audit and accepted design.
7. Commit the cohesive change on the current branch.
