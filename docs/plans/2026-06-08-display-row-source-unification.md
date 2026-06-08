# Display Row Source Unification Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Move buffer-adjacent chrome text sources toward one row-rendering abstraction so mode-line, header-line, tab-line, tab-bar, minibuffer/echo, and eventually buffer text share glyph production semantics.

**Architecture:** Keep the correct ownership split: window-local chrome remains in each window matrix, and frame-level tab-bar remains a `FrameChromeRow`. Unify the text production below those ownership decisions by introducing explicit row source/request types and routing each source through one display-row renderer with backend-owned measurement and neutral glyph accumulation.

**Tech Stack:** Rust, `neomacs-layout-engine`, `neomacs-display-protocol`, `cargo nextest`, existing `DisplayBackend`, `GlyphMatrixBuilder`, `FrameDisplayState`.

---

## Baseline

Current state as of `6c6389f66`:

- `mode-line`, `header-line`, and `tab-line` use `render_rust_status_line_value_via_backend`.
- `tab-bar` uses `render_frame_tab_bar_rust`, emits frame-level chrome, and now gets measured pixel widths through `display_text_plain_via_backend`.
- `DisplayBackend` has `produce_glyph_with_pixel_width`.
- `GuiDisplayBackend` and `TtyDisplayBackend` share neutral `GlyphRowSink`.
- Final rasterization is shared through `FrameDisplayState -> FrameGlyphBuffer -> renderer`.

Remaining architectural problems:

- `StatusLineKind` duplicates `GlyphRowRole`.
- `StatusLineSpec` is status-line-specific but structurally close to a generic row spec.
- `render_status_line_spec_via_backend` still hardcodes status-line concepts and always uses `TtyDisplayBackend` internally.
- `tab-bar` still converts keymap items to a plain `String`, so it cannot share propertized-string handling.
- Main buffer text still has a large separate walker; full buffer unification is a later phase.

## Non-Negotiable Invariants

1. Preserve GNU-visible row placement: tab-bar is frame chrome; tab-line/header-line/mode-line are window-local rows.
2. Preserve existing TTY behavior and pty parity while changing internals.
3. Preserve GUI pixel-width geometry: no new `pixel_width == 0.0` glyphs from GUI/chrome text paths unless the glyph is intentionally widthless.
4. Do not hide bugs with special cases in the renderer; fix row-source geometry before `FrameDisplayState::materialize`.
5. Use `cargo nextest`, not `cargo test`.
6. One architectural slice per commit.

---

### Task 1: Rename The Backend Accumulator Boundary

**Files:**
- Modify: `neomacs-layout-engine/src/display_backend.rs`
- Test: `neomacs-layout-engine/src/display_backend_test.rs`

**Goal:** Make the neutral sink an explicit concept, not an implementation detail hidden in `display_backend.rs`.

**Step 1: Write failing tests**

Add tests that assert both `GuiDisplayBackend` and `TtyDisplayBackend` preserve a caller-supplied measured width through `produce_glyph_with_pixel_width`.

Expected new tests:

```rust
#[test]
fn tty_produce_glyph_with_pixel_width_preserves_cell_advance() {
    let mut be = TtyDisplayBackend::new();
    let f = default_face();
    be.produce_glyph_with_pixel_width(GlyphKind::Char('x'), &f, 0, 1.0);
    assert_eq!(be.pending_glyphs()[0].pixel_width, 1.0);
}

#[test]
fn gui_produce_glyph_with_pixel_width_preserves_measured_advance() {
    let mut svc = FontMetricsService::new();
    let mut be = GuiDisplayBackend::new(&mut svc);
    let f = gui_face();
    let width = be.char_advance(&f, 'x');
    be.produce_glyph_with_pixel_width(GlyphKind::Char('x'), &f, 0, width);
    assert!((be.pending_glyphs()[0].pixel_width - width).abs() < 0.001);
}
```

**Step 2: Run red/green verification**

Run:

```bash
cargo nextest run -p neomacs-layout-engine tty_produce_glyph_with_pixel_width_preserves_cell_advance gui_produce_glyph_with_pixel_width_preserves_measured_advance
```

Expected after implementation: both pass.

**Step 3: Refactor**

If useful, move `GlyphRowSink` into a small sibling module:

```text
neomacs-layout-engine/src/display_row_sink.rs
```

Keep it crate-private. Do not add new rendering behavior.

**Step 4: Verify**

Run:

```bash
cargo nextest run -p neomacs-layout-engine display_backend
```

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/display_backend.rs neomacs-layout-engine/src/display_backend_test.rs neomacs-layout-engine/src/display_row_sink.rs neomacs-layout-engine/src/lib.rs
git commit -m "refactor(layout): name shared display row sink"
```

---

### Task 2: Introduce Generic Display Row Types

**Files:**
- Create: `neomacs-layout-engine/src/display_row.rs`
- Modify: `neomacs-layout-engine/src/lib.rs`
- Test: `neomacs-layout-engine/src/display_row_test.rs`

**Goal:** Add generic row request/spec/source types without changing call sites.

**Step 1: Write failing tests**

Add construction tests for:

```rust
pub(crate) enum DisplaySource {
    PropertizedString(Value),
    PlainString(String),
}

pub(crate) struct DisplayRowRequest {
    pub role: GlyphRowRole,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub window_id: i64,
    pub matrix_row: Option<usize>,
    pub base_face: ResolvedFace,
    pub source: DisplaySource,
}
```

Initial tests should only verify role/source construction and that `GlyphRowRole::ModeLine`, `HeaderLine`, `TabLine`, `TabBar`, and `Minibuffer` are accepted.

**Step 2: Run test to verify failure**

Run:

```bash
cargo nextest run -p neomacs-layout-engine display_row
```

Expected: compile failure until module/types exist.

**Step 3: Implement minimal types**

Keep data-only types. No renderer yet.

**Step 4: Verify**

Run:

```bash
cargo nextest run -p neomacs-layout-engine display_row
```

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/display_row.rs neomacs-layout-engine/src/display_row_test.rs neomacs-layout-engine/src/lib.rs
git commit -m "refactor(layout): add display row request types"
```

---

### Task 3: Collapse `StatusLineKind` Into `GlyphRowRole`

**Files:**
- Modify: `neomacs-layout-engine/src/display_status_line.rs`
- Modify: `neomacs-layout-engine/src/display_status_line_test.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`

**Goal:** Remove the duplicate enum and use `GlyphRowRole` as the single row role classifier.

**Step 1: Write failing test**

Change or add a test asserting that status-line spec construction stores `GlyphRowRole::TabLine` directly.

Expected assertion:

```rust
assert_eq!(spec.role, GlyphRowRole::TabLine);
```

**Step 2: Run red**

Run:

```bash
cargo nextest run -p neomacs-layout-engine display_status_line::tests::status_line_spec_uses_glyph_row_role
```

Expected: compile failure or missing field until refactor.

**Step 3: Implement**

- Replace `StatusLineKind` field with `GlyphRowRole`.
- Delete `StatusLineKind::row_role`.
- Update callers in `engine.rs` to pass `GlyphRowRole::{ModeLine, HeaderLine, TabLine}`.
- Keep function names unchanged for this task.

**Step 4: Verify**

Run:

```bash
cargo nextest run -p neomacs-layout-engine display_status_line layout_frame_rust_advances_live_output_through_tab_line_rows layout_frame_rust_advances_live_output_through_mode_line_rows
```

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/display_status_line.rs neomacs-layout-engine/src/display_status_line_test.rs neomacs-layout-engine/src/engine.rs
git commit -m "refactor(layout): use glyph row role for status rows"
```

---

### Task 4: Rename `StatusLineSpec` To A Generic Row Spec

**Files:**
- Modify: `neomacs-layout-engine/src/display_status_line.rs`
- Modify: `neomacs-layout-engine/src/display_status_line_test.rs`

**Goal:** Make the existing spec generic before moving it out of the status-line module.

**Step 1: Write failing test**

Rename one focused test to assert generic naming:

```rust
fn build_display_row_spec_preserves_display_space_align_entries()
```

It should call a new method name:

```rust
engine.build_propertized_display_row_spec(...)
```

**Step 2: Run red**

Run:

```bash
cargo nextest run -p neomacs-layout-engine build_display_row_spec_preserves_display_space_align_entries
```

Expected: missing method/type.

**Step 3: Implement**

- Rename `StatusLineSpec` to `DisplayRowSpec`.
- Rename `build_rust_status_line_spec` to `build_propertized_display_row_spec`.
- Rename `render_status_line_spec_via_backend` to `render_display_row_spec_via_backend`.
- Keep compatibility wrapper methods for one commit if call-site churn is too large, but delete wrappers before the task commit.

**Step 4: Verify**

Run:

```bash
cargo nextest run -p neomacs-layout-engine display_status_line
```

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/display_status_line.rs neomacs-layout-engine/src/display_status_line_test.rs
git commit -m "refactor(layout): generalize status row spec naming"
```

---

### Task 5: Move Generic Row Rendering Out Of `display_status_line.rs`

**Files:**
- Create or expand: `neomacs-layout-engine/src/display_row.rs`
- Modify: `neomacs-layout-engine/src/display_status_line.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: move relevant tests or keep module tests wired through `display_row_test.rs`

**Goal:** Make status-line module a thin compatibility/orchestration layer, with actual row rendering in `display_row.rs`.

**Step 1: Write failing test**

Add a test in `display_row_test.rs`:

```rust
fn render_propertized_display_row_preserves_pixel_widths()
```

It should build a propertized row from `Value::string("AB")`, render it, and assert nonzero `pixel_width`.

**Step 2: Run red**

Run:

```bash
cargo nextest run -p neomacs-layout-engine render_propertized_display_row_preserves_pixel_widths
```

**Step 3: Implement**

- Move `DisplayRowSpec`, display-property harvest, face-run logic, and render loop into `display_row.rs`.
- Leave `display_status_line.rs` only with helpers that are genuinely status-line-specific, such as format-value evaluation if it remains there.
- Ensure mode/header/tab-line callers use the generic row renderer.

**Step 4: Verify**

Run:

```bash
cargo nextest run -p neomacs-layout-engine display_row display_status_line layout_frame_rust_advances_live_output_through_tab_line_rows layout_frame_rust_advances_live_output_through_mode_line_rows
```

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/display_row.rs neomacs-layout-engine/src/display_row_test.rs neomacs-layout-engine/src/display_status_line.rs neomacs-layout-engine/src/engine.rs
git commit -m "refactor(layout): move row rendering to display row module"
```

---

### Task 6: Convert Tab-Bar To A Display Source

**Files:**
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/display_row.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs` or existing tab-bar test

**Goal:** Keep tab-bar as frame chrome, but make its text production go through the same display-row request path.

**Step 1: Write failing test**

Extend `layout_frame_rust_renders_tab_bar_text_from_lisp_tab_bar_keymap` or add a narrower unit test that verifies tab-bar glyphs have nonzero `pixel_width`.

Expected assertion:

```rust
assert!(tab_bar_glyphs.iter().all(|glyph| glyph.pixel_width > 0.0));
```

**Step 2: Run red**

Run:

```bash
cargo nextest run -p neomacs-layout-engine layout_frame_rust_renders_tab_bar_text_from_lisp_tab_bar_keymap
```

If the current implementation already passes due `6c6389f66`, adjust the test to assert use of `DisplaySource::PlainString` through the new row renderer by testing the row renderer directly.

**Step 3: Implement**

- Change `render_frame_tab_bar_rust` to build `DisplayRowRequest { role: GlyphRowRole::TabBar, source: DisplaySource::PlainString(tab_bar.text), ... }`.
- Use the generic row renderer to get a `GlyphRow`.
- Keep `pending_tab_bar` metadata and `FrameChromeRow` placement unchanged.

**Step 4: Verify**

Run:

```bash
cargo nextest run -p neomacs-layout-engine layout_frame_rust_renders_tab_bar_text_from_lisp_tab_bar_keymap display_row
```

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/display_row.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "refactor(layout): render tab bar through display row source"
```

---

### Task 7: Convert Minibuffer Echo To A Display Source

**Files:**
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/display_row.rs`
- Test: existing minibuffer echo tests in `engine.rs`

**Goal:** Replace the custom `render_minibuffer_echo_via_backend` text loop with one or more `DisplayRowRequest`s.

**Step 1: Write failing/locking tests**

Use existing tests as lock tests:

```bash
cargo nextest run -p neomacs-layout-engine layout_frame_rust_keeps_echo_message_in_minibuffer_window_for_tty layout_frame_rust_keeps_echo_message_in_minibuffer_window_for_gui layout_frame_rust_resizes_multiline_echo_rows_for_tty layout_frame_rust_resizes_multiline_echo_rows_for_gui
```

Before refactor they should pass. If needed, add a glyph `pixel_width` assertion for GUI echo rows.

**Step 2: Implement**

- Add wrapping/truncation policy to `DisplayRowRequest` if not already present.
- Render each echo visual row with `DisplaySource::PlainString`.
- Preserve the right-special-column `$` / `\` behavior.

**Step 3: Verify**

Run:

```bash
cargo nextest run -p neomacs-layout-engine layout_frame_rust_keeps_echo_message_in_minibuffer_window_for_tty layout_frame_rust_keeps_echo_message_in_minibuffer_window_for_gui layout_frame_rust_resizes_multiline_echo_rows_for_tty layout_frame_rust_resizes_multiline_echo_rows_for_gui display_row
```

**Step 4: Commit**

```bash
git add neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/display_row.rs
git commit -m "refactor(layout): render echo rows through display row source"
```

---

### Task 8: Document Buffer-Text Full Unification As A Separate Phase

**Files:**
- Modify: `docs/plans/2026-06-08-display-row-source-unification.md`

**Goal:** Stop the chrome-row refactor from silently becoming a rewrite of the main buffer walker.

**Step 1: Add a future phase section**

Document that full buffer text unification requires:

- a `BufferTextSource` adapter,
- typed source positions,
- display-property stack parity,
- bidi/cluster preservation,
- `GlyphMatrixBuilder` API cleanup,
- and a much larger test matrix.

**Step 2: Commit**

```bash
git add docs/plans/2026-06-08-display-row-source-unification.md
git commit -m "docs(layout): scope buffer text row unification phase"
```

---

## Verification Matrix

Run after every implementation task:

```bash
cargo fmt --all
cargo nextest run -p neomacs-layout-engine display_backend display_row display_status_line
```

Run after any task touching `engine.rs`:

```bash
cargo nextest run -p neomacs-layout-engine \
  layout_frame_rust_renders_tab_bar_text_from_lisp_tab_bar_keymap \
  layout_frame_rust_advances_live_output_through_tab_line_rows \
  layout_frame_rust_advances_live_output_through_mode_line_rows \
  layout_frame_rust_keeps_echo_message_in_minibuffer_window_for_tty \
  layout_frame_rust_keeps_echo_message_in_minibuffer_window_for_gui
```

Known caveat:

```text
cargo nextest run -p neomacs-layout-engine
```

currently fails in this environment at `font_metrics::tests::char_width_jetbrains_mono`
because the installed JetBrains Mono metrics report non-monospace `A`/`W` widths. Do not use that unrelated failure to block scoped row-rendering commits; do record it whenever reporting full-package verification.

---

## Stop Conditions

Stop and re-plan if any task requires:

- changing renderer glyph atlas behavior,
- changing `FrameDisplayState::materialize` fallback semantics,
- special-casing tab-line/tab-bar in the wgpu renderer,
- rewriting the main buffer walker,
- or touching GNU-visible Lisp semantics outside layout.

Those are signs the task has escaped the row-source unification slice.
