# Display Pipeline Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor Neomacs redisplay into a typed display item pipeline where display origin survives until face resolution, shaping, row layout, cursor geometry, and rendering decisions are made.

**Architecture:** Introduce a typed display-origin and display-fragment layer first, then migrate one source at a time. Keep renderer output unchanged after each task. Preserve GNU redisplay semantics, especially different base-face policies for buffer text, overlay strings, display-property strings, prefixes, and chrome rows.

**Tech Stack:** Rust, neomacs-layout-engine, neovm-core buffer/text-property/overlay APIs, neomacs-display-protocol glyph matrices, `cargo nextest`.

---

### Task 1: Land Overlay String Origin Slice

**Files:**
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/neovm_bridge.rs`
- Modify: `neomacs-layout-engine/src/display_source_resolver.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`
- Test: `neomacs-layout-engine/src/neovm_bridge_test.rs`

**Step 1: Write failing tests**

Add tests proving:
- EOB overlay `before-string` candidates do not inherit minibuffer prompt face.
- `face_for_overlay_string` uses anchor text `face` property but ignores overlay `face`.

**Step 2: Run tests to verify failure**

Run:

```bash
cargo nextest run -p neomacs-layout-engine layout_frame_rust_overlay_before_string_uses_overlay_string_base_face face_for_overlay_string_uses_text_property_but_ignores_overlay_face
```

Expected: FAIL before implementation.

**Step 3: Implement minimal abstraction**

Add:

```rust
enum OverlayStringKind {
    Before,
    After,
}

enum DisplayStringOrigin {
    OverlayString {
        anchor_charpos: usize,
        kind: OverlayStringKind,
    },
}
```

Add `FaceResolver::face_for_overlay_string`.

Change `render_overlay_string` to take `DisplayStringOrigin` and ask `FaceResolver` for the base face.

**Step 4: Run tests**

Run:

```bash
cargo nextest run -p neomacs-layout-engine overlay display_replacement display_height face_for_overlay_string
```

Expected: PASS.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/neovm_bridge.rs neomacs-layout-engine/src/display_source_resolver.rs neomacs-layout-engine/src/engine_test.rs neomacs-layout-engine/src/neovm_bridge_test.rs
git commit -m "Refactor overlay string face resolution"
```

### Task 2: Extract Display Origin Types

**Files:**
- Create: `neomacs-layout-engine/src/display_origin.rs`
- Modify: `neomacs-layout-engine/src/lib.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: `neomacs-layout-engine/src/display_origin_test.rs` or inline module tests

**Step 1: Write failing compile-level tests**

Add tests that construct every planned origin:

```rust
let _ = DisplayOrigin::BufferText { charpos: CharPos0::new(0) };
let _ = DisplayOrigin::OverlayString {
    overlay_id: Value::fixnum(1),
    anchor_charpos: CharPos0::new(0),
    kind: OverlayStringKind::Before,
};
let _ = DisplayOrigin::DisplayPropertyString {
    anchor_charpos: CharPos0::new(0),
    source: DisplayPropertySource::TextProperty,
};
let _ = DisplayOrigin::ModeLine;
let _ = DisplayOrigin::HeaderLine;
let _ = DisplayOrigin::TabLine;
let _ = DisplayOrigin::EchoArea;
```

**Step 2: Run tests to verify failure**

Run:

```bash
cargo nextest run -p neomacs-layout-engine display_origin
```

Expected: FAIL because the module/types do not exist.

**Step 3: Create `display_origin.rs`**

Move `OverlayStringKind` and `DisplayStringOrigin` out of `engine.rs` and replace with:

```rust
pub(crate) enum DisplayOrigin {
    BufferText { charpos: CharPos0 },
    OverlayString {
        overlay_id: Value,
        anchor_charpos: CharPos0,
        kind: OverlayStringKind,
    },
    DisplayPropertyString {
        anchor_charpos: CharPos0,
        source: DisplayPropertySource,
    },
    LinePrefix { anchor_charpos: CharPos0 },
    WrapPrefix { anchor_charpos: CharPos0 },
    ModeLine,
    HeaderLine,
    TabLine,
    EchoArea,
}
```

**Step 4: Wire existing overlay string path**

Use `DisplayOrigin::OverlayString` in `engine.rs`.

**Step 5: Run tests**

Run:

```bash
cargo nextest run -p neomacs-layout-engine overlay display_origin
```

Expected: PASS.

**Step 6: Commit**

```bash
git add neomacs-layout-engine/src/display_origin.rs neomacs-layout-engine/src/lib.rs neomacs-layout-engine/src/engine.rs
git commit -m "Introduce typed display origins"
```

### Task 3: Centralize Base Face Policy

**Files:**
- Create: `neomacs-layout-engine/src/display_face_policy.rs`
- Modify: `neomacs-layout-engine/src/neovm_bridge.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: `neomacs-layout-engine/src/neovm_bridge_test.rs`

**Step 1: Write failing policy tests**

Add tests for:
- `BufferText` includes text properties and overlay faces.
- `OverlayString` uses anchor text property and ignores overlay faces.
- `DisplayPropertyString` uses underlying buffer face.
- `ModeLine`, `HeaderLine`, `TabLine`, `EchoArea` resolve fixed base faces.

**Step 2: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine face_policy
```

Expected: FAIL.

**Step 3: Implement policy enum**

```rust
pub(crate) enum BaseFacePolicy {
    BufferFaceIncludingOverlays,
    OverlayStringAtAnchor,
    DisplayPropertyUnderlyingFace,
    DefaultFace,
    FixedBasicFace(BasicFaceId),
}
```

Add:

```rust
impl FaceResolver {
    pub(crate) fn base_face_for_origin<B: LayoutBufferView>(
        &self,
        buffer: Option<&B>,
        origin: &DisplayOrigin,
        policy: BaseFacePolicy,
        next_check: &mut usize,
    ) -> ResolvedFace;
}
```

**Step 4: Migrate overlay strings**

Replace direct `face_for_overlay_string` calls in `engine.rs` with `base_face_for_origin`.

**Step 5: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine overlay face_policy
```

Expected: PASS.

**Step 6: Commit**

```bash
git add neomacs-layout-engine/src/display_face_policy.rs neomacs-layout-engine/src/neovm_bridge.rs neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/neovm_bridge_test.rs
git commit -m "Centralize display origin face policy"
```

### Task 4: Introduce Display Text Fragment

**Files:**
- Create: `neomacs-layout-engine/src/display_text.rs`
- Modify: `neomacs-layout-engine/src/display_source.rs`
- Modify: `neomacs-layout-engine/src/display_row_append.rs`
- Test: `neomacs-layout-engine/src/display_source_resolver.rs`
- Test: `neomacs-layout-engine/src/display_row_append.rs`

**Step 1: Write failing tests**

Add tests that convert:
- a buffer span
- a Lisp string
- an overlay string
- a display-property string

into `DisplayTextFragment` without rendering.

**Step 2: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine display_text
```

Expected: FAIL.

**Step 3: Implement fragment type**

```rust
pub(crate) enum DisplayTextStorage {
    BufferSpan { start: CharPos0, end: CharPos0 },
    LispString(Value),
    Static(&'static str),
}

pub(crate) struct DisplayTextFragment {
    pub(crate) storage: DisplayTextStorage,
    pub(crate) origin: DisplayOrigin,
    pub(crate) base_face_policy: BaseFacePolicy,
}
```

**Step 4: Add conversion adapters**

Keep existing renderers. Add adapters that wrap current inputs into `DisplayTextFragment`.

**Step 5: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine display_text display_source display_row_append
```

Expected: PASS.

**Step 6: Commit**

```bash
git add neomacs-layout-engine/src/display_text.rs neomacs-layout-engine/src/display_source.rs neomacs-layout-engine/src/display_row_append.rs
git commit -m "Introduce display text fragments"
```

### Task 5: Migrate Overlay Strings To Display Text Fragment

**Files:**
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/display_row_append.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Step 1: Write failing adapter test**

Assert overlay `before-string` rendering uses a `DisplayTextFragment` with `DisplayOrigin::OverlayString`.

**Step 2: Run test**

```bash
cargo nextest run -p neomacs-layout-engine overlay_string_fragment
```

Expected: FAIL.

**Step 3: Replace `render_overlay_string` inputs**

Change `render_overlay_string` to accept `DisplayTextFragment` instead of raw `Value` plus origin.

**Step 4: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine overlay display_replacement display_height
```

Expected: PASS.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/display_row_append.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "Render overlay strings through display text fragments"
```

### Task 6: Migrate Display Property Strings

**Files:**
- Modify: `neomacs-layout-engine/src/display_property.rs`
- Modify: `neomacs-layout-engine/src/display_source.rs`
- Modify: `neomacs-layout-engine/src/display_row_append.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Step 1: Write failing GNU semantics test**

Create a buffer char with a display string and an overlay face. Assert the display string uses the underlying buffer face policy, not overlay-string policy.

**Step 2: Run test**

```bash
cargo nextest run -p neomacs-layout-engine display_property_string_base_face
```

Expected: FAIL.

**Step 3: Migrate display replacement strings**

Use `DisplayTextFragment {
    origin: DisplayOrigin::DisplayPropertyString { ... },
    base_face_policy: BaseFacePolicy::DisplayPropertyUnderlyingFace,
}`

**Step 4: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine display_replacement display_property_string_base_face
```

Expected: PASS.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/display_property.rs neomacs-layout-engine/src/display_source.rs neomacs-layout-engine/src/display_row_append.rs neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "Render display property strings through display text fragments"
```

### Task 7: Migrate Prefix Strings

**Files:**
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/display_row_append.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Step 1: Write failing tests**

Cover `line-prefix` and `wrap-prefix` base face behavior and cursor geometry.

**Step 2: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine prefix
```

Expected: FAIL where behavior is not yet modeled by `DisplayTextFragment`.

**Step 3: Migrate prefix rendering**

Represent prefixes as:

```rust
DisplayOrigin::LinePrefix { anchor_charpos }
DisplayOrigin::WrapPrefix { anchor_charpos }
```

Use `BaseFacePolicy::DefaultFace` unless GNU parity requires underlying face for a specific case.

**Step 4: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine prefix overlay display_replacement
```

Expected: PASS.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/display_row_append.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "Render prefix strings through display text fragments"
```

### Task 8: Migrate Chrome Rows

**Files:**
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/display_row.rs`
- Modify: `neomacs-layout-engine/src/display_row_append.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Step 1: Write failing tests**

Add tests proving mode-line, header-line, tab-line, and tab-bar text use area base faces but share glyph advance/display item behavior with buffer text.

**Step 2: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine mode_line header_line tab_line tab_bar
```

Expected: current tests pass, new fragment-specific tests fail.

**Step 3: Migrate chrome text**

Emit chrome text as `DisplayTextFragment` with:

```rust
DisplayOrigin::ModeLine
DisplayOrigin::HeaderLine
DisplayOrigin::TabLine
```

Use `BaseFacePolicy::FixedBasicFace`.

**Step 4: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine mode_line header_line tab_line tab_bar
```

Expected: PASS.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/display_row.rs neomacs-layout-engine/src/display_row_append.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "Render chrome rows through display text fragments"
```

### Task 9: Migrate Buffer Text Last

**Files:**
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/display_source.rs`
- Modify: `neomacs-layout-engine/src/display_row_append.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Step 1: Write failing adapter tests**

Add tests that normal buffer text, tabs, CJK, glyphless chars, display spaces, and cursor slots are emitted through `DisplayTextFragment`.

**Step 2: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine buffer_text_fragment
```

Expected: FAIL.

**Step 3: Migrate buffer text**

Replace direct per-char face plumbing with:

```rust
DisplayTextFragment {
    storage: DisplayTextStorage::BufferSpan { start, end },
    origin: DisplayOrigin::BufferText { charpos },
    base_face_policy: BaseFacePolicy::BufferFaceIncludingOverlays,
}
```

Keep row loop semantics unchanged. Do not change wrapping/truncation in this task.

**Step 4: Run tests**

```bash
cargo nextest run -p neomacs-layout-engine layout_frame_rust implemented_text_backends_match
```

Expected: PASS except known environment-specific font metric tests if still present.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src/engine.rs neomacs-layout-engine/src/display_source.rs neomacs-layout-engine/src/display_row_append.rs neomacs-layout-engine/src/engine_test.rs
git commit -m "Render buffer text through display text fragments"
```

### Task 10: Remove Deprecated Face/Glyph Paths

**Files:**
- Modify: `neomacs-layout-engine/src/engine.rs`
- Modify: `neomacs-layout-engine/src/display_row_append.rs`
- Modify: `neomacs-layout-engine/src/glyph_advance.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

**Step 1: Search deprecated paths**

Run:

```bash
rg -n "current_resolved_face|current_text_face_id|char_advance\\(|append_buffer_text_char_to_text_row" neomacs-layout-engine/src
```

**Step 2: Delete or narrow old APIs**

Remove paths no longer used by any display source. Keep only internal implementation helpers under the unified fragment path.

**Step 3: Run full focused suite**

```bash
cargo nextest run -p neomacs-layout-engine overlay display_replacement prefix mode_line header_line tab_line layout_frame_rust
```

Expected: PASS.

**Step 4: Run package suite**

```bash
cargo nextest run -p neomacs-layout-engine --no-fail-fast
```

Expected: PASS or only known environment-specific font metric failures documented with exact names.

**Step 5: Commit**

```bash
git add neomacs-layout-engine/src
git commit -m "Remove deprecated display face and glyph paths"
```

### Task 11: Manual Visual Verification

**Files:**
- No required code changes
- Optional: update `docs/testing`

**Step 1: Build**

Run:

```bash
cargo build --release
```

**Step 2: User visual checks**

Ask user to test:
- `M-x` with Vertico posframe and custom yellow prompt face
- Vertico candidate movement with `C-j` / `C-k`
- tab-bar text
- tab-line text
- mode-line text
- echo-area messages
- minibuffer prompt
- overlay-heavy buffers, including org folds

**Step 3: Debug log if needed**

Use:

```bash
NEOMACS_DUMP_FRAME_GLYPHS=1
```

Expected: glyphs from all sources carry expected face IDs and stable geometry.

**Step 4: Final commit if docs/tests updated**

```bash
git add docs/testing
git commit -m "Document display pipeline visual checks"
```

---

## Execution Rules

- Use TDD for every behavioral change.
- Never use `cargo test`; use `cargo nextest`.
- Commit after each green task.
- Do not rewrite `engine.rs` in one patch.
- Do not change renderer semantics while refactoring layout semantics.
- Preserve GNU behavior even when it looks strange.
- Keep display-property string semantics separate from overlay-string semantics.
- Treat a full-suite font-metrics environment failure separately from display-pipeline regressions.
