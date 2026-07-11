# Presented Pointer Map Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add GNU-compatible, snapshot-scoped pointer appearance for tab-bar close/add controls and main-buffer `mouse-face` without evaluator work or layout on pointer motion.

**Architecture:** Layout builds a validated `PresentedPointerMap` beside each immutable `FrameGlyphBuffer`. Hit regions reference click meaning and visual appearance independently; runtime selects snapshot-qualified hover/pressed state, and WGPU redraws existing primitives with a realized face or raised/sunken image mode while preserving geometry.

**Tech Stack:** Rust, serde, Neomacs display protocol/layout/runtime/WGPU crates, evaluator text properties and overlays, `cargo nextest` (never `cargo test`).

**Design:** `docs/plans/2026-07-11-presented-pointer-map-design.md`

**Workspace constraint:** Work directly in `/home/exec/Projects/github.com/eval-exec/neomacs`; do not create a worktree. Preserve unrelated user changes. Apply @superpowers:test-driven-development to every behavior change and @superpowers:verification-before-completion before each completion claim.

---

### Task 1: Define the validated protocol model

**Files:**
- Create: `neomacs-display-protocol/src/presented_pointer.rs`
- Create: `neomacs-display-protocol/src/presented_pointer_test.rs`
- Modify: `neomacs-display-protocol/src/lib.rs`
- Modify: `neomacs-display-protocol/src/frame_glyphs.rs`

**Step 1: Write failing protocol tests**

Add tests covering:

```rust
#[test]
fn pointer_regions_keep_input_and_appearance_independent() {
    let body = region(0.0..80.0, InteractionId::new(1), PointerAppearanceId::new(3));
    let close = region(80.0..96.0, InteractionId::new(2), PointerAppearanceId::new(3));
    let map = PresentedPointerMap::new(vec![body, close], vec![mouse_face_appearance()])
        .expect("valid map");

    assert_ne!(map.hit_test(point(20.0, 8.0)).unwrap().interaction,
               map.hit_test(point(88.0, 8.0)).unwrap().interaction);
    assert_eq!(map.hit_test(point(20.0, 8.0)).unwrap().appearance,
               map.hit_test(point(88.0, 8.0)).unwrap().appearance);
}

#[test]
fn pointer_map_rejects_stale_appearance_and_out_of_range_paint_spans() { /* ... */ }
```

Also assert serde round trips, finite/clipped rectangles, non-empty spans, and deterministic first/topmost hit behavior.

**Step 2: Run the tests and verify red**

Run:

```bash
cargo nextest run -p neomacs-display-protocol presented_pointer
```

Expected: compilation fails because `presented_pointer` types do not exist.

**Step 3: Implement the minimal protocol module**

Define checked transparent IDs and renderer-safe data:

```rust
pub struct PointerAppearanceId(u32);

pub enum PresentedPrimitiveKind {
    Glyph,
    Image,
}

pub struct PresentedPaintSpan {
    pub kind: PresentedPrimitiveKind,
    pub first: u32,
    pub len: u32,
    pub clip: FrameRect,
}

pub enum PointerDrawMode {
    Face(FaceId),
    ImageRaised,
    ImageSunken,
}

pub struct PresentedPointerAppearance {
    pub paint_spans: Vec<PresentedPaintSpan>,
    pub hover: PointerDrawMode,
    pub pressed: PointerDrawMode,
}

pub struct PresentedPointerRegion {
    pub bounds: FrameRect,
    pub interaction: InteractionId,
    pub appearance: Option<PointerAppearanceId>,
}

pub struct PresentedPointerMap { /* private validated vectors */ }
```

Keep native-control input out of the first implementation until the scroll-bar adapter exists; do not add a hypothetical port with one adapter. Add an empty map to every `FrameGlyphBuffer` constructor/default.

**Step 4: Run focused tests and formatting**

```bash
cargo nextest run -p neomacs-display-protocol presented_pointer
cargo fmt --all --check
```

Expected: all selected tests pass.

**Step 5: Commit**

```bash
git add neomacs-display-protocol/src
git commit -m "feat(protocol): describe presented pointer appearances"
```

### Task 2: Build the deep layout-side map builder

**Files:**
- Create: `neomacs-layout-engine/src/presented_pointer_map.rs`
- Create: `neomacs-layout-engine/src/presented_pointer_map_test.rs`
- Modify: `neomacs-layout-engine/src/lib.rs`
- Modify: `neomacs-layout-engine/src/display_row_render_state.rs`
- Modify: `neomacs-layout-engine/src/display_row_source_state.rs`

**Step 1: Write failing builder-interface tests**

Test only observable builder output:

```rust
#[test]
fn builder_coalesces_regions_and_deduplicates_visual_ranges() {
    let mut builder = PresentedPointerMapBuilder::new(presentation(), frame_bounds());
    builder.observe_run(body_run(interaction(1), mouse_face(7)));
    builder.observe_run(close_run(interaction(2), mouse_face(7)));
    let map = builder.finish().expect("valid map");

    assert_eq!(map.appearances().len(), 1);
    assert_eq!(map.regions().len(), 2);
}

#[test]
fn wrapped_source_property_becomes_one_appearance_with_two_paint_spans() { /* ... */ }
```

Cover adjacency coalescing, different click meanings sharing an appearance, clipping, an unresolved face becoming no appearance, and ordinary runs producing no records.

**Step 2: Verify red**

```bash
cargo nextest run -p neomacs-layout-engine presented_pointer_map
```

Expected: compilation fails because the builder does not exist.

**Step 3: Implement the builder**

Expose only:

```rust
impl PresentedPointerMapBuilder {
    pub(crate) fn new(presentation: PresentationId, frame: FrameRect) -> Self;
    pub(crate) fn observe_rendered_run(&mut self, run: RenderedPointerRun);
    pub(crate) fn finish(self) -> Result<PresentedPointerMap, PointerMapBuildError>;
}
```

Keep source-property lookup behind a private adapter. The pure `RenderedPointerRun` input contains final hit bounds, typed primitive span, evaluator interaction, optional realized mouse-face ID, and optional image-button fallback. Do not let callers deduplicate or coalesce themselves.

**Step 4: Verify green**

```bash
cargo nextest run -p neomacs-layout-engine presented_pointer_map
cargo nextest run -p neomacs-layout-engine display_row_source
```

Expected: all selected tests pass.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src
git commit -m "feat(layout): build presented pointer maps"
```

### Task 3: Replace tab-specific hover state with generic snapshot state

**Files:**
- Modify: `neomacs-display-runtime/src/render_thread/state.rs`
- Modify: `neomacs-display-runtime/src/render_thread/pointer_events.rs`
- Modify: `neomacs-display-runtime/src/render_thread/frame_ingest.rs`
- Modify: `neomacs-display-runtime/src/render_thread/input_test.rs`
- Modify: `neomacs-display-runtime/src/render_thread/frame_windows_test.rs`

**Step 1: Write failing transition tests**

Add a pure state transition seam:

```rust
#[test]
fn pointer_appearance_is_qualified_by_presentation_and_phase() { /* ... */ }

#[test]
fn replacing_a_frame_clears_hover_from_the_retired_presentation() { /* ... */ }

#[test]
fn press_capture_keeps_input_target_while_hover_follows_pointer() { /* ... */ }
```

Assert enter, movement within the same appearance, appearance switch, leave, press, release, and stale presentation behavior.

**Step 2: Verify red**

```bash
cargo nextest run -p neomacs-display-runtime pointer_appearance
```

Expected: tests fail because only `tab_bar_hovered: Option<u32>` exists.

**Step 3: Implement generic runtime state**

```rust
struct ActivePointerAppearance {
    presentation: PresentationId,
    appearance: PointerAppearanceId,
    phase: PointerAppearancePhase,
}

enum PointerAppearancePhase { Hover, Pressed }
```

Hit-test the displayed frame's pointer map. Preserve the existing evaluator `PresentedPointer` press/release transport. Keep captured input interaction separate from active visual appearance. Remove `tab_bar_hovered` only after all references compile against the generic state.

**Step 4: Verify green**

```bash
cargo nextest run -p neomacs-display-runtime pointer_appearance
cargo nextest run -p neomacs-display-runtime chrome_hit
```

Expected: all selected tests pass.

**Step 5: Commit**

```bash
git add neomacs-display-runtime/src/render_thread
git commit -m "refactor(input): track presented pointer appearance"
```

### Task 4: Add renderer draw-override selection

**Files:**
- Modify: `neomacs-renderer-wgpu/src/renderer/content.rs`
- Modify: `neomacs-renderer-wgpu/src/renderer/content_test.rs`
- Modify: `neomacs-renderer-wgpu/src/renderer/glyphs.rs`
- Modify: `neomacs-renderer-wgpu/src/renderer/glyphs_test.rs`
- Modify: `neomacs-renderer-wgpu/src/shaders/image.wgsl`
- Modify: `neomacs-display-runtime/src/render_thread/render_pass.rs`

**Step 1: Write failing command-selection tests**

Factor a pure resolver and test it before touching WGPU:

```rust
#[test]
fn pressed_override_precedes_hover_and_normal() { /* ... */ }

#[test]
fn face_override_changes_face_without_changing_geometry() { /* ... */ }

#[test]
fn image_override_selects_raised_and_sunken_relief() { /* ... */ }
```

The geometry test must compare x, y, width, advance, clip, row and source slot before and after selecting a weight-changing face.

**Step 2: Verify red**

```bash
cargo nextest run -p neomacs-renderer-wgpu pointer_override
```

Expected: compilation fails because renderer draw overrides do not exist.

**Step 3: Implement minimal renderer support**

Expose a renderer-safe selection value from `neomacs-display-protocol`, for example `PointerAppearanceSelection { appearance, phase }`. Runtime derives it from `ActivePointerAppearance` only after verifying the frame's `PresentationId`, so `neomacs-renderer-wgpu` never depends on runtime state types. Pass `Option<PointerAppearanceSelection>` into root and child frame content rendering. While iterating glyph/image primitives, resolve the applicable `PointerDrawMode`:

- `Face(id)`: use the alternate materialized face for background, glyph rasterization, decoration and box drawing while retaining original geometry.
- `ImageRaised`/`ImageSunken`: set an explicit relief uniform/vertex flag and draw GNU-compatible light/dark edges without changing the image box.

Do not create duplicate presentation layers and do not mutate `FrameGlyphBuffer`.

**Step 4: Verify green**

```bash
cargo nextest run -p neomacs-renderer-wgpu pointer_override
cargo nextest run -p neomacs-renderer-wgpu renderer::content
cargo nextest run -p neomacs-renderer-wgpu renderer::glyphs
```

Expected: all selected tests pass.

**Step 5: Commit**

```bash
git add neomacs-renderer-wgpu/src neomacs-display-runtime/src/render_thread/render_pass.rs
git commit -m "feat(renderer): apply transient pointer draw overrides"
```

### Task 5: Publish exact GNU tab-bar appearances

**Files:**
- Modify: `neomacs-layout-engine/src/display_status_line.rs`
- Modify: `neomacs-layout-engine/src/display_status_line_test.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/engine_test.rs`
- Modify: `neovm-core/src/keyboard.rs`
- Modify: `neovm-core/src/keyboard_test.rs`

**Step 1: Write failing GNU-semantic tests**

Use real propertized captions and replacement-image slots:

```rust
#[test]
fn tab_body_and_close_keep_different_clicks_but_share_whole_tab_mouse_face() { /* ... */ }

#[test]
fn add_tab_without_mouse_face_uses_raised_hover_and_sunken_press() { /* ... */ }

#[test]
fn tab_mouse_face_uses_realized_tab_bar_tab_highlight_face() { /* ... */ }
```

Assert that the close image's source slot is part of the whole caption appearance, while its interaction still resolves to `(KEY BINDING t)`. Assert add-tab resolves to `(add-tab tab-bar-new-tab nil)`.

**Step 2: Verify red**

```bash
cargo nextest run -p neomacs-layout-engine tab_bar_pointer_appearance
```

Expected: tests fail because current tab-bar hit regions publish no appearance.

**Step 3: Implement the tab-bar adapter**

Extend the existing tab source snapshot to retain effective `mouse-face` boundaries. Resolve the face through existing `FaceResolver`/`FrameFaceIdAllocator`. Feed rendered source runs into `PresentedPointerMapBuilder`:

```text
mouse-face property -> Face(realized face)
no mouse-face       -> ImageRaised
pressed             -> ImageSunken
```

Keep click `posn-string` construction in the evaluator registry. Delete tab-specific hover state only; do not duplicate Lisp bindings in the protocol.

**Step 4: Verify green**

```bash
cargo nextest run -p neomacs-layout-engine tab_bar_pointer_appearance
cargo nextest run -p neovm-core presented_pointer
cargo nextest run -p neomacs-display-runtime pointer_appearance
```

Expected: all selected tests pass.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src neovm-core/src neomacs-display-runtime/src
git commit -m "fix(tab-bar): reproduce GNU pointer feedback"
```

### Task 6: Publish main-buffer `mouse-face`

**Files:**
- Modify: `neomacs-layout-engine/src/display_source.rs`
- Modify: `neomacs-layout-engine/src/display_row_face_state.rs`
- Modify: `neomacs-layout-engine/src/display_row_render_state.rs`
- Modify: `neomacs-layout-engine/src/display_row_test.rs`
- Modify: `neomacs-layout-engine/src/display_source_test.rs`
- Modify: `neomacs-layout-engine/src/engine_test.rs`

**Step 1: Write failing display-row tests**

Cover the real call path:

```rust
#[test]
fn buffer_mouse_face_publishes_realized_pointer_appearance() { /* ... */ }

#[test]
fn overlay_mouse_face_priority_matches_effective_display_face_resolution() { /* ... */ }

#[test]
fn wrapped_mouse_face_range_publishes_multiple_spans_under_one_appearance() { /* ... */ }

#[test]
fn display_string_and_replacement_image_preserve_mouse_face_source_mapping() { /* ... */ }
```

Also assert ordinary buffer text publishes no pointer map records.

**Step 2: Verify red**

```bash
cargo nextest run -p neomacs-layout-engine buffer_mouse_face
```

Expected: tests fail because the display walk does not harvest `mouse-face`.

**Step 3: Implement through the shared property pipeline**

Harvest effective `mouse-face` beside existing `face` property processing so overlays, buffer text, Lisp strings and display replacements use one precedence implementation. After each rendered row is finalized, pass only interactive runs to `PresentedPointerMapBuilder`. Do not scan all glyphs a second time when the display walk already exposes property boundaries.

Use the realized alternate face only for painting; retain base layout metrics and source slots.

**Step 4: Verify green**

```bash
cargo nextest run -p neomacs-layout-engine buffer_mouse_face
cargo nextest run -p neomacs-layout-engine display_row
cargo nextest run -p neomacs-layout-engine display_source
```

Expected: all selected tests pass.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src
git commit -m "feat(display): publish buffer mouse-face appearances"
```

### Task 7: Add end-to-end pointer-motion regression coverage

**Files:**
- Modify: `neomacs-display-runtime/src/render_thread/input_test.rs`
- Modify: `neomacs-display-runtime/src/render_thread/frame_ingest.rs`
- Modify: `neomacs-bin/src/input_bridge_test.rs`
- Create or modify the existing GUI render-test harness selected by `rg --files | rg 'gui.*test|render.*snapshot'`

**Step 1: Add failing integration tests**

Inject pointer motion over a published frame and assert command/pixel state transitions:

1. Close hit activates the whole-tab face appearance.
2. Moving within the same appearance produces no redundant state change.
3. Plus hover selects raised and press selects sunken.
4. Main-buffer `mouse-face` changes pixels without a new presentation.
5. Leaving restores byte/pixel-equivalent base rendering.
6. Installing a new presentation rejects the old appearance.

Prefer command-buffer snapshots over GPU pixel readback where they prove the same behavior deterministically; retain at least one image comparison that catches shader/relief wiring.

**Step 2: Verify red**

```bash
cargo nextest run -p neomacs-display-runtime presented_pointer_integration
cargo nextest run -p neomacs-renderer-wgpu presented_pointer_integration
```

Expected: at least one integration assertion fails before final wiring.

**Step 3: Complete only missing wiring**

Connect frame ingestion, root/child rendering, dirty marking and bridge behavior needed by the tests. Do not add gesture, drag-and-drop, help-echo function evaluation, scroll-bar migration, or embedded-surface internal hit-testing.

**Step 4: Verify green**

```bash
cargo nextest run -p neomacs-display-runtime presented_pointer_integration
cargo nextest run -p neomacs-renderer-wgpu presented_pointer_integration
```

Expected: all selected tests pass deterministically.

**Step 5: Commit**

```bash
git add neomacs-display-runtime neomacs-renderer-wgpu neomacs-bin
git commit -m "test(pointer): cover presented hover pipeline"
```

### Task 8: Cleanup, documentation, full verification, and review

**Files:**
- Modify: `docs/plans/2026-07-11-presented-pointer-map-design.md` only if implementation discoveries change accepted details
- Modify: relevant Rust files to remove superseded tab-only hover helpers/tests

**Step 1: Remove obsolete shallow state**

Use `rg` to prove no dead tab-only hover path remains:

```bash
rg -n "tab_bar_hovered|TabBarHover|tab_bar.*brightness" neomacs-display-runtime neomacs-renderer-wgpu
```

Expected: no obsolete state; GNU-specific logic exists only in the tab-bar layout adapter.

**Step 2: Run formatting and static checks**

```bash
cargo fmt --all --check
cargo check --workspace
git diff --check
```

Expected: success, allowing only documented pre-existing warnings.

**Step 3: Run affected suites**

```bash
cargo nextest run -p neomacs-display-protocol
cargo nextest run -p neomacs-layout-engine
cargo nextest run -p neomacs-display-runtime
cargo nextest run -p neomacs-renderer-wgpu
cargo nextest run -p neovm-core keyboard
```

Expected: all affected tests pass. If a broader baseline test fails, rerun it alone and record evidence that it is unrelated; do not hide it or modify unrelated behavior.

**Step 4: Build the user reproduction binary**

```bash
cargo build --release -p neomacs
```

Expected: `target/release/neomacs` builds successfully.

**Step 5: Run the two-axis review**

Use @code-review against the fixed point preceding Task 1. Standards review checks module depth, duplicated state, primitive-ID invariants, and hot-path allocations. Spec review checks exact GNU close/plus semantics, main-buffer `mouse-face`, stale presentations, and absence of evaluator work on pointer motion. Address verified medium/high findings and repeat focused tests.

**Step 6: Commit final cleanup**

```bash
git add docs neomacs-display-protocol neomacs-layout-engine neomacs-display-runtime neomacs-renderer-wgpu neovm-core neomacs-bin
git commit -m "refactor(pointer): complete presented appearance pipeline"
```

**Step 7: Push**

```bash
git push origin main
```

Expected: `origin/main` advances to the verified final commit.
