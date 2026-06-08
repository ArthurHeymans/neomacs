# Display Item Stream Unification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make buffer text, mode-line, header-line, tab-line, tab-bar, minibuffer, and echo-area text share one display-item-to-glyph-row pipeline, so every row type can render the same display semantics where GNU Emacs supports them.

**Architecture:** Keep source acquisition source-specific: buffer text reads buffer intervals, overlays, invisibility, window state, and cursor positions; Lisp-string rows read already-evaluated strings and their text properties. Unify below that boundary by converting both sources into a typed `DisplayItemStream`, then using one `DisplayRowBuilder` to handle faces, display properties, glyphless/control characters, clusters, wide chars, bidi, stretch/image/xwidget/video items, wrapping/truncation, and glyph row geometry.

**Tech Stack:** Rust, `neomacs-layout-engine`, `neomacs-display-protocol`, `neovm_core`, GNU Emacs source reference in `/home/exec/Projects/github.com/emacs-mirror/emacs`, `cargo nextest`.

---

## GNU Reference Model

GNU Emacs does not have a clean Rust-style abstraction, but it has a strong conceptual unification point in `src/xdisp.c`:

- `get_next_display_element` fills a mutable redisplay iterator.
- `handle_display_prop` handles `display` properties for both Lisp strings and buffer text.
- `PRODUCE_GLYPHS` turns the iterator element into row glyphs.
- `display_string` is used by mode-line/menu/tab-bar-like string rendering and drives the same iterator/glyph production loop.
- `display_tab_bar_line` also uses `get_next_display_element` and `PRODUCE_GLYPHS` for non-toolkit tab-bar rows.

The useful lesson is not to copy GNU's mutable `struct it` directly. The useful lesson is that source walking is separate from glyph production, and string rows should not have a weaker display language than buffer rows.

## Current Neomacs Pipeline

```text
Buffer text path
----------------
Context + Window + Buffer
        |
        v
engine.rs giant buffer walker
  - buffer chars
  - text properties
  - overlays / before-string / after-string
  - display properties
  - glyphless/control display
  - tabs/wide chars/clusters/complex runs
  - bidi via GlyphMatrixBuilder
  - cursor/window-end/hit-test metadata
        |
        v
direct GlyphMatrixBuilder push_* calls
        |
        v
GlyphRow in window matrix
        |
        v
FrameDisplayState -> renderer


Mode/header/tab-line/tab-bar/echo path
--------------------------------------
Lisp evaluation produces Value string
        |
        v
DisplayRowRequest { LispStringSource, geometry, role }
        |
        v
DisplayRowSpec
  - bytes
  - face runs
  - display prop records
  - align records
        |
        v
render_display_row_spec_to_glyph_row
  - simpler per-byte/per-char walker
  - no full buffer display semantics
        |
        v
GlyphRow installed as window row or frame chrome row
        |
        v
FrameDisplayState -> renderer
```

The two paths only unify after glyph rows exist. That is too late. By then the chrome path has already lost semantics the buffer path knows how to render.

## Ideal Pipeline

```text
                         VM-aware/source-specific side
                         -----------------------------

BufferTextSourceCursor                         LispStringSourceCursor
  - buffer chars                                - string chars
  - buffer text props                           - string text props
  - overlays                                    - face/display props
  - invisibility                                - nested display strings
  - window/cursor state                         - echo/mode/tab row role
          \                                      /
           \                                    /
            v                                  v
              DisplayItemStream / SourceStack
              - source spans
              - resolved faces or face specs
              - typed display property items
              - cursor/hit-test anchors
              - row break candidates

                         Pure layout/render side
                         -----------------------

                    DisplayRowBuilder
                    - font metrics
                    - tab stops
                    - glyphless/control display
                    - grapheme clusters
                    - complex runs
                    - wide chars and padding cells
                    - stretch/image/video/xwidget slots
                    - bidi row reordering
                    - wrap/truncate outcomes
                             |
                             v
             GlyphRow + DisplayRowMetadata + SideItems
                             |
                             v
      WindowMatrixInstaller or FrameChromeInstaller
                             |
                             v
                  FrameDisplayState -> renderer
```

The important boundary is `DisplayItemStream`. Code above it may inspect `Value`, buffers, overlays, evaluator state, and frame/window parameters. Code below it should consume typed Rust data and should not evaluate Lisp or ask a buffer for raw text properties.

## Core Types

Create these modules gradually:

- `neomacs-layout-engine/src/display_item.rs`
- `neomacs-layout-engine/src/display_source.rs`
- `neomacs-layout-engine/src/display_row_builder.rs`
- `neomacs-layout-engine/src/display_row_install.rs`

Target shape:

```rust
pub(crate) trait DisplayItemSource {
    fn next_item(&mut self, cx: &mut DisplaySourceContext<'_>) -> Option<DisplayItem>;
    fn source_position(&self) -> DisplaySourcePosition;
}

pub(crate) struct DisplaySourceContext<'a> {
    pub evaluator: Option<&'a mut neovm_core::emacs_core::Context>,
    pub face_resolver: &'a FaceResolver,
    pub font_metrics: &'a mut Option<FontMetricsService>,
    pub next_face_id: &'a mut u32,
}

pub(crate) struct DisplayItem {
    pub span: SourceSpan,
    pub face: RenderFaceRef,
    pub kind: DisplayItemKind,
}

pub(crate) enum DisplayItemKind {
    TextRun(DisplayTextRun),
    ControlChar { ch: char },
    Glyphless { ch: char, method: GlyphlessMethod },
    Stretch(DisplayStretch),
    Image(DisplayImageItem),
    Video(DisplayVideoItem),
    Xwidget(DisplayXwidgetItem),
    RowBreak(DisplayRowBreak),
    CursorAnchor(CursorAnchor),
    HitTestAnchor(HitTestAnchor),
}

pub(crate) struct DisplayRowLayout {
    pub role: GlyphRowRole,
    pub target: DisplayRowTarget,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
    pub base_face: RenderFaceRef,
    pub tab_width: usize,
    pub wrap: DisplayWrapMode,
}

pub(crate) enum DisplayRowTarget {
    WindowMatrix { window_id: i64, matrix_row: usize },
    WindowChrome { window_id: i64 },
    FrameChrome,
    Detached,
}

pub(crate) struct DisplayRowBuilder<'a> {
    layout: DisplayRowLayout,
    metrics: &'a mut DisplayMetrics,
    row: GlyphRow,
    metadata: DisplayRowMetadata,
}
```

Notes:

- `DisplayRowLayout` replaces the scattered row geometry locals for buffer rows and the current `DisplayRowRequest` for chrome rows.
- `DisplayItemKind` is not a rendering artifact; it is the source-neutral display language.
- `Value` is not allowed in `DisplayRowBuilder`. Parse Lisp display properties before producing items.
- Use concrete order enums, not bare bools. For example:

```rust
pub(crate) enum OverlayStringInsertion {
    BeforeBufferText,
    AfterBufferText,
}

pub(crate) enum SamePositionOverlayOrder {
    HigherPriorityFirst,
    LowerPriorityFirst,
}
```

## Required Invariants

1. Buffer and chrome rows must use the same glyph production code after `DisplayItemStream`.
2. Mode-line, header-line, tab-line, tab-bar, minibuffer, and echo-area strings must support the same string text-property display semantics as buffer display strings, except where GNU has source-specific limits.
3. `GlyphMatrixBuilder` must stop being the place where display semantics live. Long-term it should install already-built rows and own frame/window matrix bookkeeping.
4. The renderer must stay dumb: it consumes glyph rows, images, videos, xwidgets, and face tables. It must not know whether a row came from a buffer, mode-line, tab-bar, or echo-area.
5. No new untyped bool configuration for semantic choices. Use small enums with domain names.
6. Use `cargo nextest`, not `cargo test`.
7. One phase per commit; do not combine cleanup and behavior changes.

## Hard Parts

### Source Stack

GNU display has nested sources: buffer text can enter overlay strings, display strings, before/after strings, and replacement strings. Lisp strings can contain `display` properties that point at other strings or images. A flat iterator is not enough.

Use a stack:

```text
SourceStack
  top -> LispStringSourceCursor(display replacement string)
         BufferTextSourceCursor(original buffer)
```

Each source frame must carry source relation metadata: replacement, before-string, after-string, display-string, composition string, etc. This is how cursor and hit-test positions remain attached to the original buffer position when a display string replaces a character.

### Cursor, Window-End, And Hit Testing

The buffer path currently computes cursor position, display positions, window-end, and hit-test rows while it emits glyphs. Unification will fail if those are treated as renderer details.

Represent them as metadata items or row metadata updates:

- `CursorAnchor`
- `DisplayPointAnchor`
- `HitTestAnchor`
- `WindowEndCandidate`

Chrome rows can ignore buffer-only metadata; buffer rows must preserve it.

### Faces

Face resolution is source-specific because buffer-local face remapping, string text properties, overlays, and `font-lock-face` all enter differently. But after resolution, row building needs one face reference model.

Target:

```text
Value face specs / overlay face / text property face
        |
        v
FaceResolver + frame face-id allocator
        |
        v
RenderFaceRef { face_id, metrics_key, render_face }
```

### Widths And Metrics

GUI rows must carry real pixel widths. TTY rows must carry cell-consistent widths. This belongs in `DisplayRowBuilder` and `DisplayMetrics`, not in source cursors and not in final renderer code.

### Bidi, Clusters, Wide Chars

These are currently concentrated in `GlyphMatrixBuilder` and the main buffer walker. Move them behind row building:

- grapheme cluster continuation
- complex run continuation
- wide char padding cells
- composed glyphs
- bidi levels and visual reordering
- cursor column mapping after bidi

### Display Properties

Implement typed display property parsing once and use it from both source cursors:

```rust
pub(crate) enum DisplayProperty {
    ReplacementString(LispStringSourceCursor),
    Space(DisplaySpace),
    Image(DisplayImageSpec),
    Video(DisplayVideoSpec),
    Xwidget(DisplayXwidgetSpec),
    Ignore,
    Unsupported(ValueForDebugOnly),
}
```

The final form should avoid leaking `Value` below source parsing. During migration, an `Unsupported(ValueForDebugOnly)` diagnostic wrapper is acceptable only above the item stream boundary.

## Phase 0: Lock Current Behavior And GNU Oracles

**Files:**

- Modify: `neomacs-layout-engine/src/display_row_test.rs`
- Modify: `neomacs-layout-engine/src/engine_test.rs`
- Create: `docs/testing/display-unification-gnu-oracles.md`

**Steps:**

1. Add lock tests for current chrome rows:
   - propertized echo text
   - propertized tab-bar text
   - mode-line `display` space
   - tab-line CJK wide text
   - tab-line emoji cluster text
   - RTL tab-line text
2. Add lock tests for current buffer rows with the same semantic inputs where possible.
3. Add a short GNU oracle doc with exact one-liner commands using `/home/exec/.local/bin/emacs` and the nix Emacs path when available.
4. Run:

```bash
cargo nextest run -p neomacs-layout-engine display_row display_status_line layout_frame_rust
```

5. Commit:

```bash
git add docs/testing/display-unification-gnu-oracles.md neomacs-layout-engine/src/display_row_test.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "test(layout): lock display row unification baseline"
```

## Phase 1: Introduce Display Items Without Routing Callers

**Files:**

- Create: `neomacs-layout-engine/src/display_item.rs`
- Create: `neomacs-layout-engine/src/display_source.rs`
- Modify: `neomacs-layout-engine/src/lib.rs`
- Test: `neomacs-layout-engine/src/display_item_test.rs`

**Steps:**

1. Add `DisplayItem`, `DisplayItemKind`, `SourceSpan`, `DisplaySourcePosition`, and `RenderFaceRef`.
2. Add `DisplayItemSource` trait.
3. Add data-only tests for text run, stretch, image, row break, cursor anchor, and hit-test anchor items.
4. Run:

```bash
cargo nextest run -p neomacs-layout-engine display_item
```

5. Commit:

```bash
git add neomacs-layout-engine/src/display_item.rs neomacs-layout-engine/src/display_item_test.rs neomacs-layout-engine/src/display_source.rs neomacs-layout-engine/src/lib.rs
git commit -m "refactor(layout): add display item model"
```

## Phase 2: Add Lisp String Item Source

**Files:**

- Modify: `neomacs-layout-engine/src/display_source.rs`
- Modify: `neomacs-layout-engine/src/display_row.rs`
- Test: `neomacs-layout-engine/src/display_source_test.rs`

**Steps:**

1. Implement `LispStringSourceCursor` that walks a `Value` string by character ranges.
2. Resolve `face` and `font-lock-face` text properties into `RenderFaceRef`.
3. Parse `display` text properties into typed `DisplayProperty`.
4. Emit replacement strings by pushing a new source frame, not by flattening to plain text.
5. Keep the existing `DisplayRowSpec` path alive as a compatibility consumer.
6. Run:

```bash
cargo nextest run -p neomacs-layout-engine display_source display_row
```

7. Commit:

```bash
git add neomacs-layout-engine/src/display_source.rs neomacs-layout-engine/src/display_source_test.rs neomacs-layout-engine/src/display_row.rs
git commit -m "refactor(layout): stream display items from lisp strings"
```

## Phase 3: Build The Shared Display Row Builder

**Files:**

- Create: `neomacs-layout-engine/src/display_row_builder.rs`
- Modify: `neomacs-layout-engine/src/matrix_builder.rs`
- Modify: `neomacs-layout-engine/src/composition.rs`
- Test: `neomacs-layout-engine/src/display_row_builder_test.rs`

**Steps:**

1. Implement `DisplayRowBuilder::push_item`.
2. Move or wrap the following behavior from `GlyphMatrixBuilder`:
   - `last_text_cluster_tail`
   - `push_cluster_continuation`
   - `push_run_member`
   - `push_wide_char_with_pixel_width`
   - row bidi reorder
3. Keep `GlyphMatrixBuilder` methods as delegating compatibility shims during migration.
4. Add tests for:
   - ASCII text
   - CJK wide char with padding
   - emoji/ZWJ cluster
   - combining mark cluster
   - Arabic/Indic complex run
   - RTL reordering
   - stretch item with pixel width
5. Run:

```bash
cargo nextest run -p neomacs-layout-engine display_row_builder matrix_builder
```

6. Commit:

```bash
git add neomacs-layout-engine/src/display_row_builder.rs neomacs-layout-engine/src/display_row_builder_test.rs neomacs-layout-engine/src/matrix_builder.rs neomacs-layout-engine/src/composition.rs
git commit -m "refactor(layout): build glyph rows from display items"
```

## Phase 4: Route Chrome Rows Through Display Items

**Files:**

- Modify: `neomacs-layout-engine/src/display_row.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: `neomacs-layout-engine/src/display_row_test.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Steps:**

1. Add `LayoutEngine::render_display_source_row`.
2. Convert mode-line/header-line/tab-line callers from `DisplayRowSpec` to `LispStringSourceCursor -> DisplayRowBuilder`.
3. Convert tab-bar and echo-area callers.
4. Keep `DisplayRowSpec` only as an assertion/compatibility path until test parity is proven.
5. Run:

```bash
cargo nextest run -p neomacs-layout-engine \
  display_row \
  display_status_line \
  layout_frame_rust_renders_tab_bar_text_from_lisp_tab_bar_keymap \
  layout_frame_rust_advances_live_output_through_tab_line_rows \
  layout_frame_rust_advances_live_output_through_mode_line_rows \
  layout_frame_rust_preserves_propertized_echo_message_faces
```

6. Commit:

```bash
git add neomacs-layout-engine/src/display_row.rs neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/display_row_test.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "refactor(layout): render chrome rows from display items"
```

## Phase 5: Add Buffer Text Source Cursor In Parallel

**Files:**

- Modify: `neomacs-layout-engine/src/display_source.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: `neomacs-layout-engine/src/display_source_test.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Steps:**

1. Extract the main buffer walker source reads into `BufferTextSourceCursor`.
2. Start with normal text, newline, tab, control chars, glyphless chars, and face changes.
3. Emit metadata items for cursor anchor, display point anchor, hit-test anchor, and window-end candidate.
4. Do not replace the main buffer renderer yet. Add a shadow path test that compares item-source output with existing main-buffer glyph output for simple rows.
5. Run:

```bash
cargo nextest run -p neomacs-layout-engine display_source layout_frame_rust_emits_buffer_tab_as_stretch_glyph layout_frame_rust_cursor_width_uses_current_glyph_advance_not_next_glyph
```

6. Commit:

```bash
git add neomacs-layout-engine/src/display_source.rs neomacs-layout-engine/src/display_source_test.rs neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "refactor(layout): stream simple buffer text items"
```

## Phase 6: Move Display Properties To Shared Items

**Files:**

- Modify: `neomacs-layout-engine/src/display_item.rs`
- Modify: `neomacs-layout-engine/src/display_source.rs`
- Modify: `neomacs-layout-engine/src/display_space.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: `neomacs-layout-engine/src/display_source_test.rs`
- Test: `neomacs-layout-engine/src/display_row_builder_test.rs`

**Steps:**

1. Extract one parser for `display` property values.
2. Convert `(space ...)`, `:align-to`, and width/height/ascent handling to `DisplayItemKind::Stretch`.
3. Convert replacement strings to source-stack frames.
4. Convert image/video/xwidget specs to typed display items.
5. Preserve GNU-like source position behavior for replacement strings.
6. Run:

```bash
cargo nextest run -p neomacs-layout-engine display_source display_row_builder layout_frame_rust_emits_display_string_replacement_glyphs
```

7. Commit:

```bash
git add neomacs-layout-engine/src/display_item.rs neomacs-layout-engine/src/display_source.rs neomacs-layout-engine/src/display_space.rs neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/display_source_test.rs neomacs-layout-engine/src/display_row_builder_test.rs
git commit -m "refactor(layout): share display property item parsing"
```

## Phase 7: Route Buffer Text Rows Through The Shared Builder

**Files:**

- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/window_output.rs`
- Modify: `neomacs-layout-engine/src/matrix_builder.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Steps:**

1. Replace direct `GlyphMatrixBuilder::push_*` calls in the main buffer walker with `DisplayRowBuilder::push_item`.
2. Keep existing source walking order and metadata updates until the cursor is fully extracted.
3. Install completed `GlyphRow`s through a new `WindowMatrixInstaller`.
4. Verify cursor, window-end, hit-test, line wrapping, truncation, and bidi tests.
5. Run:

```bash
cargo nextest run -p neomacs-layout-engine engine matrix_builder display_row_builder
```

6. Commit:

```bash
git add neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/window_output.rs neomacs-layout-engine/src/matrix_builder.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "refactor(layout): route buffer rows through display row builder"
```

## Phase 8: Remove Compatibility Spec And Semantic Builder Shims

**Files:**

- Modify: `neomacs-layout-engine/src/display_row.rs`
- Modify: `neomacs-layout-engine/src/matrix_builder.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: affected layout tests

**Steps:**

1. Delete `DisplayRowSpec` if no callers remain.
2. Delete `DisplayPropRecord`, `OverlayFaceRun`, and `OverlayAlignEntry` if they only existed for `DisplayRowSpec`.
3. Rename `StatusLineFace` to a generic render-facing face type or merge it into `RenderFaceRef`.
4. Remove `GlyphMatrixBuilder` semantic push methods that are no longer used outside tests.
5. Keep matrix installation APIs:
   - begin/end window matrix
   - install text row
   - install chrome row
   - publish images/videos/xwidgets
   - publish faces
6. Run:

```bash
cargo nextest run -p neomacs-layout-engine display_row_builder display_source engine matrix_builder
```

7. Commit:

```bash
git add neomacs-layout-engine/src/display_row.rs neomacs-layout-engine/src/matrix_builder.rs neomacs-layout-engine/src/engine.rs
git commit -m "refactor(layout): remove legacy display row spec path"
```

## Phase 9: Full Verification With Personal Config

**Files:**

- No code changes unless failures reveal bugs.

**Steps:**

1. Format:

```bash
cargo fmt --all
```

2. Run targeted nextest:

```bash
cargo nextest run -p neomacs-layout-engine display_source display_row_builder display_row display_status_line engine matrix_builder
```

3. Fresh release build:

```bash
cargo xtask fresh-build --release
```

4. Run Neomacs with the personal config:

```bash
./target/release/neomacs --debug-init
```

5. If any `ERROR` appears, stop and investigate root cause before committing further.
6. Commit only after verification is clean:

```bash
git status --short
git diff --check
git commit -m "refactor(layout): complete display item stream unification"
```

## Cleanup Targets

These names should disappear or shrink substantially by the end:

- `DisplayRowSpec`
- `DisplayPropRecord`
- `OverlayFaceRun`
- `OverlayAlignEntry`
- `render_display_row_spec_to_glyph_row`
- `render_display_row_spec_via_backend`
- `render_overlay_string` duplicate glyph logic
- direct semantic `GlyphMatrixBuilder::push_*` use from source walkers
- status-line-specific names for generic row/face data

These names should remain, but with narrower jobs:

- `GlyphMatrixBuilder`: window/frame matrix ownership and row installation.
- `FrameDisplayState`: immutable render state publication.
- renderer crates: rasterization only, no source semantics.

## Success Criteria

1. Chrome and buffer rows pass the same semantic tests for faces, display strings, display spaces, wide chars, emoji clusters, combining marks, RTL text, glyphless chars, and image/stretch/xwidget items where GNU supports them.
2. Main-buffer glyph rows still carry real GUI pixel widths and correct TTY cell widths.
3. Mode-line, header-line, tab-line, tab-bar, minibuffer, and echo-area rows no longer have a weaker renderer than main buffer text.
4. The source/VM boundary is explicit: source cursors may touch Lisp/buffers; row builder and renderer consume typed Rust data.
5. No renderer special cases are added for tab-line, tab-bar, or mode-line quality issues.
