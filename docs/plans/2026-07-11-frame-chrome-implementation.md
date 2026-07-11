# Unified Frame Chrome Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the split tab/menu/tool/compact-bar geometry and hit-testing paths with one coordinate-safe `FrameChrome` module shared by GUI and TTY.

**Architecture:** The display protocol owns immutable frame-chrome bands. Each band has one absolute `FrameRect`; display rows, media, item rectangles, and hit regions are band-local. The layout engine builds semantic content and stacks bands once, while GUI and TTY adapters project that authoritative result without recalculating geometry.

**Tech Stack:** Rust 2024, serde display protocol, Neomacs layout engine, wgpu renderer, TTY RIF, cargo nextest run, proptest.

**Design reference:** `docs/plans/2026-07-11-frame-chrome-design.md`

---

### Task 1: Add coordinate-safe frame-chrome protocol types

**Files:**
- Create: `neomacs-display-protocol/src/frame_chrome.rs`
- Create: `neomacs-display-protocol/src/frame_chrome_test.rs`
- Modify: `neomacs-display-protocol/src/lib.rs`
- Modify: `neomacs-display-protocol/Cargo.toml`

**Step 1: Write failing coordinate and stacking tests**

Add tests for the interface, not helper implementation:

```rust
#[test]
fn layout_stacks_visible_bands_once() {
    let chrome = FrameChrome::layout(
        FrameSize::new(624.0, 648.0).unwrap(),
        vec![
            ChromeBandRequest::empty(FrameChromeKind::MenuBar, 18.0),
            ChromeBandRequest::empty(FrameChromeKind::ToolBar, 34.0),
            ChromeBandRequest::empty(FrameChromeKind::TabBar, 18.0),
        ],
    )
    .unwrap();

    assert_eq!(chrome.band(FrameChromeKind::MenuBar).unwrap().bounds().y(), 0.0);
    assert_eq!(chrome.band(FrameChromeKind::ToolBar).unwrap().bounds().y(), 18.0);
    assert_eq!(chrome.band(FrameChromeKind::TabBar).unwrap().bounds().y(), 52.0);
}

#[test]
fn frame_rect_places_band_local_rect_exactly_once() {
    let band = FrameRect::new(0.0, 52.0, 624.0, 18.0).unwrap();
    let local = BandRect::new(8.0, 0.0, 40.0, 18.0).unwrap();
    assert_eq!(band.place(local).unwrap().raw(), Rect::new(8.0, 52.0, 40.0, 18.0));
}
```

Also test rejection of NaN, infinity, negative sizes, overflowing bands, duplicate singleton kinds, and simultaneous `CompactBar` plus `MenuBar`/`ToolBar` requests.

Add a proptest asserting ordered, in-frame, non-overlapping output for generated valid heights.

**Step 2: Run tests and verify they fail**

Run:

```bash
cargo nextest run -p neomacs-display-protocol frame_chrome --lib
```

Expected: compilation fails because `frame_chrome` and its types do not exist.

**Step 3: Implement the minimal protocol module**

Define these public types with private fields and checked constructors:

```rust
pub struct FrameSize { width: f32, height: f32 }
pub struct FrameRect(Rect);
pub struct BandRect(Rect);

pub enum FrameChromeKind { MenuBar, ToolBar, CompactBar, TabBar }
pub struct ChromeBandId(u32);

pub enum ChromeAction {
    OpenMenu { index: u32, key: String },
    InvokeToolBarItem { index: u32 },
    Presented { interaction: InteractionId },
}

pub struct ChromeHitRegion {
    local_bounds: BandRect,
    action: ChromeAction,
}

pub struct FrameChrome {
    bands: Vec<FrameChromeBand>,
}
```

Initially use a private/test-only empty content variant so this commit establishes geometry without prematurely designing renderer payload details. `FrameChrome::layout` owns ordering and enforces the presentation policy:

```text
normal:  MenuBar -> ToolBar -> TabBar
compact: CompactBar -> TabBar
```

Export only read methods such as `bands()`, `band(kind)`, `bounds()`, `hit_regions()`, and `FrameRect::place`. Do not expose raw mutable rectangles.

Add `proptest.workspace = true` under protocol dev-dependencies and export the module from `lib.rs`.

**Step 4: Run tests and verify they pass**

Run:

```bash
cargo nextest run -p neomacs-display-protocol frame_chrome --lib
```

Expected: all frame-chrome unit and property tests pass.

**Step 5: Commit**

```bash
git add neomacs-display-protocol/Cargo.toml neomacs-display-protocol/src/lib.rs \
  neomacs-display-protocol/src/frame_chrome.rs \
  neomacs-display-protocol/src/frame_chrome_test.rs
git commit -m "feat(protocol): add coordinate-safe frame chrome"
```

### Task 2: Add typed band content and one GUI materialization path

**Files:**
- Modify: `neomacs-display-protocol/src/frame_chrome.rs`
- Modify: `neomacs-display-protocol/src/frame_chrome_test.rs`
- Modify: `neomacs-display-protocol/src/glyph_matrix.rs`
- Modify: `neomacs-display-protocol/src/glyph_matrix_test.rs`
- Modify: `neomacs-display-protocol/src/frame_glyphs.rs`

**Step 1: Write the nonzero-origin regression test**

Construct menu and tool bands totaling 52 pixels followed by a tab display-row band. Include a character, stretch, image, and hit region at local `y = 0`. Materialize and assert:

```rust
assert_eq!(tab_band.bounds().y(), 52.0);
assert_eq!(tab_char.y(), 52.0);
assert_eq!(tab_stretch.y(), 52.0);
assert_eq!(tab_image.y(), 52.0);
assert_eq!(tab_char.clip_rect(), Some(Rect::new(0.0, 52.0, 624.0, 18.0)));
assert_eq!(tab_hit.bounds().y(), 52.0);
```

This is the automated form of the `/tmp/debug.txt` failure: `104.0` must never appear.

**Step 2: Run the test and verify it fails**

Run:

```bash
cargo nextest run -p neomacs-display-protocol frame_chrome_materializes_nonzero_tab_origin_once --lib
```

Expected: failure because bands do not yet contain materializable content.

**Step 3: Add typed content**

Replace the temporary empty content with:

```rust
pub enum FrameChromeContent {
    DisplayRow(ChromeDisplayRow),
    MenuBar(MenuBarContent),
    ToolBar(ToolBarContent),
    CompactBar(CompactBarContent),
}

pub struct ChromeDisplayRow {
    row: GlyphRow,
    media: Vec<ChromeMedia>,
}

pub enum ChromeMedia {
    Image { local_bounds: BandRect, image_id: ImageId, slot: Option<DisplaySlotId> },
    Video { local_bounds: BandRect, video_id: VideoId, slot: Option<DisplaySlotId>, loop_count: i32, autoplay: bool },
    Xwidget { local_bounds: BandRect, xwidget_id: XwidgetId, slot: Option<DisplaySlotId> },
}
```

`ChromeDisplayRow::new` must normalize the contained row to `pixel_y = 0`; there is no public row-Y setter in this interface.

Menu/tool/compact content owns semantic items, resolved colors, and band-local item rectangles. Heights stay exclusively in `FrameChromeBand.bounds`.

**Step 4: Materialize frame chrome exactly once**

Add `frame_chrome: FrameChrome` to `FrameDisplayState` and `FrameGlyphBuffer` while temporarily retaining old fields for migration. In `FrameDisplayState::materialize`, iterate bands:

- `DisplayRow`: call the existing grid-row materializer with the band bounds as origin and clip.
- `ChromeMedia`: translate `BandRect` with `FrameRect::place` once.
- Menu/tool/compact: clone semantic content and absolute item/hit rectangles into `FrameGlyphBuffer.frame_chrome` for the runtime adapter.

Do not call `absolute_output_row()` anywhere in this path.

**Step 5: Run protocol tests**

Run:

```bash
cargo nextest run -p neomacs-display-protocol frame_chrome --lib
cargo nextest run -p neomacs-display-protocol frame_chrome_materializes_nonzero_tab_origin_once --lib
```

Expected: pass, with every tab output at `y = 52`.

**Step 6: Commit**

```bash
git add neomacs-display-protocol/src/frame_chrome.rs \
  neomacs-display-protocol/src/frame_chrome_test.rs \
  neomacs-display-protocol/src/glyph_matrix.rs \
  neomacs-display-protocol/src/glyph_matrix_test.rs \
  neomacs-display-protocol/src/frame_glyphs.rs
git commit -m "feat(protocol): materialize typed frame chrome bands"
```

### Task 3: Build exact semantic item geometry in the layout engine

**Files:**
- Modify: `neomacs-layout-engine/src/gui_chrome.rs`
- Modify: `neomacs-layout-engine/src/gui_chrome_test.rs`
- Modify: `neomacs-layout-engine/src/display_status_line.rs`
- Modify: `neomacs-layout-engine/src/display_status_line_test.rs`
- Modify: `neomacs-layout-engine/src/display_row_render_state.rs`

**Step 1: Write failing semantic-layout tests**

Test that menu, toolbar, compact, and tab builders return content with band-local item bounds and semantic actions. Important assertions:

```rust
assert_eq!(menu.items()[0].bounds().y(), 0.0);
assert_eq!(menu.items()[0].action(), &ChromeAction::OpenMenu { /* exact key */ });
assert_eq!(tool.items()[0].action(), &ChromeAction::InvokeToolBarItem { index: 0 });
assert!(matches!(tab.hit_regions()[0].action(), ChromeAction::Presented { .. }));
```

For tabs, retain each caption's key, binding, value, and source character range
before concatenation. Use `RenderedDisplayRow::source_slots()` to produce
contiguous semantic runs, resolving `close-tab` at the exact source character.
Register GNU-shaped evaluator targets and publish only opaque interaction
references. This supports proportional faces, replacement images, the plus
item, and custom bindings without moving Lisp policy into the renderer.

**Step 2: Run tests and verify they fail**

Run:

```bash
cargo nextest run -p neomacs-layout-engine gui_chrome --lib
cargo nextest run -p neomacs-layout-engine tab_bar_hit_regions_follow_rendered_caption_bounds --lib
```

Expected: failures because builders currently return bare items and tab caption ranges are discarded.

**Step 3: Implement semantic content builders**

- Change `TabBarDisplaySource` to preserve `(item, char_range)` alongside captions.
- Expose a crate-private read-only source-slot slice on `RenderedDisplayRow` outside `#[cfg(test)]`.
- Add one reducer from source character range to `BandRect`; keep it private to the tab-bar implementation.
- Move the current menu padding, toolbar separator width, icon-size/padding, and compact split calculations into layout-owned helpers that return positioned semantic items.
- Give renderers item rectangles; renderers must not recalculate widths later.

Use the frame's resolved font metrics for menu labels. Derive toolbar icon size/padding from the authoritative band height in one named policy function.

**Step 4: Run layout tests**

Run:

```bash
cargo nextest run -p neomacs-layout-engine gui_chrome --lib
cargo nextest run -p neomacs-layout-engine tab_bar --lib
```

Expected: pass.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/gui_chrome.rs \
  neomacs-layout-engine/src/gui_chrome_test.rs \
  neomacs-layout-engine/src/display_status_line.rs \
  neomacs-layout-engine/src/display_status_line_test.rs \
  neomacs-layout-engine/src/display_row_render_state.rs
git commit -m "refactor(layout): produce semantic frame chrome content"
```

### Task 4: Make `FrameChrome` the layout engine's sole frame-bar owner

**Files:**
- Modify: `neomacs-layout-engine/src/display_frame_output.rs`
- Modify: `neomacs-layout-engine/src/display_frame_output_test.rs`
- Modify: `neomacs-layout-engine/src/display_rendered_row_output_install.rs`
- Modify: `neomacs-layout-engine/src/display_row_measured_state.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/engine_test.rs`

**Step 1: Write failing end-to-end layout tests**

Add an engine test with menu height 18, toolbar height 34, and tab height 18. Assert one published `FrameChrome` with order `MenuBar`, `ToolBar`, `TabBar`, and tab origin `52`.

Materialize the resulting `FrameDisplayState` and assert tab characters, stretches, images, clip rectangles, and hit regions all use `52`, not `104`.

Add the compact policy case: `CompactBar`, `TabBar`, with no menu/tool bands.

**Step 2: Run tests and verify they fail**

Run:

```bash
cargo nextest run -p neomacs-layout-engine layout_frame_rust_publishes_authoritative_frame_chrome --lib
cargo nextest run -p neomacs-layout-engine layout_frame_rust_tab_bar_nonzero_origin_materializes_once --lib
```

Expected: fail because `FrameOutputOwner` still owns separate rows and tab metadata and `engine.rs` appends GUI states after finishing output.

**Step 3: Replace pending split state**

Replace:

```rust
pending_frame_chrome_rows: Vec<FrameChromeRow>
pending_tab_bar: Option<FrameTabBarState>
```

with one pending request collection finalized by `FrameChrome::layout` in `FrameOutputSession::finish`.

Render the tab display row at local `y = 0`. Replace `MeasuredDisplayRow::absolute_output_row` for frame chrome with a constructor that produces `ChromeDisplayRow`; retain `window_relative_output_row` for window chrome.

Move menu/tool/compact semantic collection before `FrameOutputOwner::finish`, submit visible requests, and delete `chrome_before_tab`, `tab_bar_y`, and all post-finish assignments to `gui_menu_bar`, `gui_tool_bar`, and `gui_compact_bar`.

If chrome layout fails, return a typed frame-layout failure to the publication call site and keep the previous valid snapshot; do not publish partial bands.

**Step 4: Run layout tests**

Run:

```bash
cargo nextest run -p neomacs-layout-engine frame_chrome --lib
cargo nextest run -p neomacs-layout-engine tab_bar --lib
```

Expected: pass.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/display_frame_output.rs \
  neomacs-layout-engine/src/display_frame_output_test.rs \
  neomacs-layout-engine/src/display_rendered_row_output_install.rs \
  neomacs-layout-engine/src/display_row_measured_state.rs \
  neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "refactor(layout): centralize frame chrome ownership"
```

### Task 5: Consume bands directly in the GUI renderer

**Files:**
- Modify: `neomacs-display-runtime/src/render_thread/frame_ingest.rs`
- Modify: `neomacs-display-runtime/src/render_thread/frame_windows.rs`
- Modify: `neomacs-display-runtime/src/render_thread/render_pass.rs`
- Modify: `neomacs-display-runtime/src/render_thread/input_test.rs`
- Modify: `neomacs-renderer-wgpu/src/renderer/ui_overlays.rs`
- Modify: `neomacs-renderer-wgpu/src/renderer/ui_overlays_test.rs`

**Step 1: Write failing GUI projection tests**

Test that ingestion receives only `FrameGlyphBuffer.frame_chrome`; no parallel menu/tool/compact arguments are needed. Test renderer overlay inputs use each band's absolute bounds and positioned item rectangles.

Include an assertion that toolbar origin comes from `ToolBar` band bounds and is not reconstructed from menu/tab heights.

**Step 2: Run tests and verify they fail**

Run:

```bash
cargo nextest run -p neomacs-display-runtime frame_chrome --lib
cargo nextest run -p neomacs-renderer-wgpu frame_chrome --lib
```

Expected: fail because ingestion and rendering still store and pass separate overlay states.

**Step 3: Migrate ingestion and rendering**

- Remove `menu_bar`, `tool_bar`, and `compact_bar` parameters from frame-ingest functions.
- Remove semantic bar fields from runtime `ChromeState`; keep only transient hover/pressed state and GPU texture resources.
- Change overlay render methods to accept `FrameRect` and positioned items. Delete `item_x`, `label.len() * char_width`, `frame_toolbar_y_origin`, and equivalent stacking calculations.
- Resolve toolbar textures from `ToolBarContent`/`CompactBarContent` during ingest, but keep texture caches runtime-owned.
- Render tab-bar display rows through ordinary materialized frame glyphs; do not restore a renderer-specific tab-bar method.

**Step 4: Run GUI tests**

Run:

```bash
cargo nextest run -p neomacs-display-runtime render_thread --lib
cargo nextest run -p neomacs-renderer-wgpu --lib
```

Expected: pass.

**Step 5: Commit**

```bash
git add neomacs-display-runtime/src/render_thread/frame_ingest.rs \
  neomacs-display-runtime/src/render_thread/frame_windows.rs \
  neomacs-display-runtime/src/render_thread/render_pass.rs \
  neomacs-display-runtime/src/render_thread/input_test.rs \
  neomacs-renderer-wgpu/src/renderer/ui_overlays.rs \
  neomacs-renderer-wgpu/src/renderer/ui_overlays_test.rs
git commit -m "refactor(display): render authoritative frame chrome bands"
```

### Task 6: Route all frame-chrome input through semantic hit regions

**Files:**
- Modify: `neomacs-display-runtime/src/render_thread/input.rs`
- Modify: `neomacs-display-runtime/src/render_thread/input_test.rs`
- Modify: `neomacs-display-runtime/src/render_thread/pointer_events.rs`
- Modify: `neomacs-display-runtime/src/render_thread/frame_windows.rs`
- Modify: `neomacs-display-runtime/src/render_thread/frame_windows_test.rs`

**Step 1: Write failing generic hit tests**

Create a frame with nonzero-Y menu, tool, and tab bands. Assert:

```rust
assert!(matches!(chrome.hit_test(FramePoint::new(20.0, 56.0)), Some(ChromeAction::Presented { .. })));
assert_eq!(chrome.hit_test(FramePoint::new(20.0, 30.0)), Some(&ChromeAction::InvokeToolBarItem { index: 0 }));
```

Test hover, press, release, and popup anchoring against the same absolute hit rectangle.

**Step 2: Run tests and verify they fail**

Run:

```bash
cargo nextest run -p neomacs-display-runtime chrome_hit --lib
```

Expected: fail because input still has independent label/icon width calculations and feature-specific Y checks.

**Step 3: Implement one action dispatcher**

Add one lookup:

```rust
fn frame_chrome_hit(frame: &FrameGlyphBuffer, point: FramePoint)
    -> Option<(&ChromeAction, FrameRect)>;
```

Map actions to existing `InputEvent` variants at the final dispatch edge. Preserve the exact menu key and use the authoritative hit rectangle as `PopupAnchorRect`.

Delete:

- `tab_bar_hit_test_items`
- `toolbar_hit_test_items`
- `menu_bar_hit_test_item`
- `compact_bar_menu_width`
- `toolbar_y_origin`, `tab_bar_y`, and bar-height reconstruction helpers
- duplicated primary-window and managed-frame-window chrome branches

Keep interaction visual state typed by action kind, but key it by semantic index/ID returned from the hit region.

**Step 4: Run runtime tests**

Run:

```bash
cargo nextest run -p neomacs-display-runtime render_thread --lib
```

Expected: pass.

**Step 5: Commit**

```bash
git add neomacs-display-runtime/src/render_thread/input.rs \
  neomacs-display-runtime/src/render_thread/input_test.rs \
  neomacs-display-runtime/src/render_thread/pointer_events.rs \
  neomacs-display-runtime/src/render_thread/frame_windows.rs \
  neomacs-display-runtime/src/render_thread/frame_windows_test.rs
git commit -m "refactor(input): dispatch semantic frame chrome hits"
```

### Task 7: Adapt TTY rendering to the same bands

**Files:**
- Modify: `neomacs-display-protocol/src/frame_chrome.rs`
- Modify: `neomacs-display-protocol/src/tty_rif.rs`
- Modify: `neomacs-display-protocol/src/tty_rif_test.rs`
- Modify: `neomacs-layout-engine/src/tty_menu_bar.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`

**Step 1: Write failing TTY adapter tests**

Build the same semantic `FrameChrome` used by GUI tests and assert TTY output:

- Menu content occupies the menu band's terminal rows.
- Tab display-row content occupies the tab band's terminal row.
- Compact presentation excludes separate menu/tool rows.
- Band pixel bounds convert to cells once using frame character metrics.

**Step 2: Run tests and verify they fail**

Run:

```bash
cargo nextest run -p neomacs-display-protocol tty_frame_chrome --lib
```

Expected: fail because `TtyRif` still reads `menu_bar` and `frame_chrome_rows` independently.

**Step 3: Implement the TTY adapter**

Add terminal face attributes needed by the TTY adapter to `MenuBarContent` rather than preserving `TtyMenuBarState` as a separate geometry owner.

In `TtyRif::rasterize`, iterate `state.frame_chrome.bands()`:

- Render menu semantic items with terminal attributes into the band's cell rows.
- Render `DisplayRow` through `rasterize_glyph_row` at the band-derived row.
- Ignore GUI-only tool images only where the existing TTY behavior already does so; do not create a second stacking policy.

**Step 4: Run TTY and layout tests**

Run:

```bash
cargo nextest run -p neomacs-display-protocol tty --lib
cargo nextest run -p neomacs-layout-engine tty_menu_bar --lib
```

Expected: pass.

**Step 5: Commit**

```bash
git add neomacs-display-protocol/src/frame_chrome.rs \
  neomacs-display-protocol/src/tty_rif.rs \
  neomacs-display-protocol/src/tty_rif_test.rs \
  neomacs-layout-engine/src/tty_menu_bar.rs \
  neomacs-layout-engine/src/engine.rs
git commit -m "refactor(tty): render shared frame chrome bands"
```

### Task 8: Delete the old protocol and verify the complete migration

**Files:**
- Modify: `neomacs-display-protocol/src/lib.rs`
- Modify: `neomacs-display-protocol/src/frame_glyphs.rs`
- Modify: `neomacs-display-protocol/src/glyph_matrix.rs`
- Modify: `neomacs-display-protocol/src/glyph_matrix_test.rs`
- Modify: `neomacs-display-protocol/src/snapshot_text.rs`
- Modify: `neomacs-layout-engine/src/display_rendered_row_output_install.rs`
- Modify: `neomacs-layout-engine/src/display_row_measured_state.rs`
- Modify: `neomacs-display-runtime/src/render_thread/frame_ingest.rs`
- Modify: `neomacs-display-runtime/src/render_thread/render_pass.rs`
- Modify: affected snapshot/golden files reported by tests

**Step 1: Delete compatibility fields and ambiguous constructors**

Remove:

- `FrameDisplayState.frame_chrome_rows`
- `FrameDisplayState.menu_bar`
- `FrameDisplayState.gui_menu_bar`
- `FrameDisplayState.gui_tool_bar`
- `FrameDisplayState.gui_compact_bar`
- `FrameDisplayState.tab_bar`
- `FrameGlyphBuffer.tab_bar`
- `FrameChromeRow`
- `FrameTabBarState`
- `GuiMenuBarState`, `GuiToolBarState`, `GuiCompactBarState`
- `MeasuredDisplayRow::absolute_output_row`

Keep only `frame_chrome` as the frame-bar protocol interface.

**Step 2: Replace and test the serialized protocol**

Replace the obsolete serialized fields directly and update round-trip tests for
the new `frame_chrome` shape. Neomacs does not maintain an internal protocol
version or compatibility path for obsolete pre-release snapshots.

Run:

```bash
cargo nextest run -p neomacs-display-protocol --lib
```

Expected: pass.

**Step 3: Run formatting and focused verification**

Run:

```bash
cargo fmt --all -- --check
cargo nextest run -p neomacs-display-protocol -p neomacs-layout-engine \
  -p neomacs-display-runtime -p neomacs-renderer-wgpu --lib
```

Expected: all tests pass.

**Step 4: Rebuild and run the original GUI reproduction**

Build:

```bash
cargo build --release -p neomacs
```

Run Neomacs with frame dumps, enable `tab-bar-mode`, and assert every tab glyph lies inside its clip rectangle. Use the captured-dump checker:

```bash
python3 - <<'PY'
import re
path = "/tmp/debug.txt"
found = 0
for line in open(path, errors="replace"):
    if "row_role: TabBar" not in line or "Char {" not in line:
        continue
    clip = re.search(r"clip_rect: Some\(Rect \{ x: [^,]+, y: ([^,]+), width: [^,]+, height: ([^ }]+)", line)
    glyph = re.search(r", y: ([^,]+), baseline:", line)
    if clip and glyph:
        y, h, gy = map(float, (clip.group(1), clip.group(2), glyph.group(1)))
        assert y <= gy < y + h, (gy, y, h)
        found += 1
assert found > 0
print(f"PASS: {found} tab-bar characters lie inside their band")
PY
```

Expected: `PASS`, with tab characters at the band's actual origin (`52` in the original environment), never `104`.

**Step 5: Check for stale abstractions**

Run:

```bash
rg "FrameChromeRow|FrameTabBarState|gui_menu_bar|gui_tool_bar|gui_compact_bar|absolute_output_row|tab_bar_hit_test_items|frame_toolbar_y_origin" \
  neomacs-display-protocol neomacs-layout-engine neomacs-display-runtime neomacs-renderer-wgpu
```

Expected: no production-code matches. Test migration comments may mention old names only when explaining rejected snapshots.

**Step 6: Commit**

```bash
git add neomacs-display-protocol neomacs-layout-engine neomacs-display-runtime \
  neomacs-renderer-wgpu
git commit -m "refactor(display): complete unified frame chrome migration"
```

### Task 9: Final review and full verification

**Files:**
- Review all files changed since `4bae623a1`

**Step 1: Inspect the complete diff**

Run:

```bash
git diff --check 4bae623a1..HEAD
git diff --stat 4bae623a1..HEAD
```

Expected: no whitespace errors; changes remain within frame-chrome scope.

**Step 2: Run the repository verification appropriate to the change**

Run:

```bash
cargo fmt --all -- --check
cargo clippy -p neomacs-display-protocol -p neomacs-layout-engine \
  -p neomacs-display-runtime -p neomacs-renderer-wgpu --all-targets -- -D warnings
cargo nextest run -p neomacs-display-protocol -p neomacs-layout-engine \
  -p neomacs-display-runtime -p neomacs-renderer-wgpu
```

Expected: all commands pass.

**Step 3: Request code review**

Use `@superpowers:requesting-code-review` against base commit `4bae623a1`. Review specifically for:

- any remaining second source of frame-chrome geometry;
- public access to raw mutable rectangles;
- local coordinates translated more than once;
- visuals and hit regions derived from different measurements;
- GUI/TTY adapter behavior divergence.

**Step 4: Apply review feedback test-first**

Use `@superpowers:receiving-code-review` for every actionable finding. Add or adjust a failing interface test before changing implementation.

**Step 5: Re-run final verification**

Repeat Step 2 and the original frame-glyph dump checker. Expected: all pass.

**Step 6: Commit review fixes if any**

```bash
git add <reviewed-files>
git commit -m "fix(display): address frame chrome review"
```
