# Accessibility and GUI Testability Design

**Date:** 2026-06-25
**Status:** Proposal
**Scope:** How NeoMacs should expose semantic GUI state for accessibility and
AI-readable visual tests across winit/wgpu, X11, Wayland, macOS, and Windows.

## Summary

NeoMacs should add an internal accessibility snapshot produced from the display
state, then later expose that same snapshot through AccessKit.

This gives two benefits from one source of truth:

- AI-readable GUI tests can assert text, roles, bounds, cursor position, and
  window state from JSON without OCR or manual image inspection.
- Native accessibility can be layered on later through AccessKit adapters for
  AT-SPI, NSAccessibility, and UI Automation.

The important design choice is to build the semantic tree before the wgpu
renderer. winit knows windows and input events; wgpu knows pixels and buffers.
Neither knows what text is displayed. NeoMacs does know that in its redisplay
and layout artifacts, especially `FrameDisplayState`, `GlyphRow`, and
`WindowInfo`.

## Background

GUI window tests are hard for an AI agent because screenshots are opaque. A PNG
readback can prove that pixels were produced, but it cannot reliably answer:

- Which buffer is visible?
- Which text is on screen?
- Where is the cursor?
- Which window is selected?
- Is the minibuffer showing a prompt or message?
- Are mode-line, tab-line, menu-bar, toolbar, images, videos, xwidgets, and
  scroll bars present with sane geometry?

The current `neomacs-gui-tests` direction is therefore correct: keep GUI tests
text-first and write stable artifacts under `target/neomacs-gui-tests`. The next
step should be a semantic artifact, not OCR.

## GNU Emacs Grounding

GNU Emacs' core GUI display model is redisplay/glyph-matrix based, not
accessibility-tree based.

Relevant GNU source:

- `/home/exec/Projects/github.com/emacs-mirror/emacs/src/dispextern.h`
  - `struct glyph`
  - `struct glyph_row`
  - `struct glyph_matrix`
- `/home/exec/Projects/github.com/emacs-mirror/emacs/src/xdisp.c`
  - row construction
  - cursor placement
  - window-start/window-end redisplay semantics

NeoMacs already mirrors this model in Rust:

- `neomacs-display-protocol/src/glyph_matrix.rs`
  - `FrameDisplayState`
  - `WindowMatrixEntry`
  - `GlyphRow`
  - `Glyph`
- `neomacs-display-protocol/src/frame_glyphs.rs`
  - `FrameGlyphBuffer`
  - `FrameGlyph`
  - `WindowInfo`
  - `DisplaySlotId`
- `neomacs-layout-engine/src/window_output.rs`
  - window display range and row output
- `neomacs-display-runtime/src/render_thread/frame_ingest.rs`
  - materializes `FrameDisplayState` before rendering

That means NeoMacs should not try to infer accessibility from pixels. The
semantic information is already present before rendering.

## Recommended Architecture

Add an internal accessibility snapshot to `neomacs-display-protocol`.

Suggested module:

```text
neomacs-display-protocol/src/accessibility.rs
```

Suggested top-level types:

```rust
pub struct AccessibilitySnapshot {
    pub schema_version: u32,
    pub frame_id: DisplayFrameId,
    pub frame_bounds: AccessibilityRect,
    pub nodes: Vec<AccessibilityNode>,
    pub root: AccessibilityNodeId,
}

pub struct AccessibilityNode {
    pub id: AccessibilityNodeId,
    pub parent: Option<AccessibilityNodeId>,
    pub role: AccessibilityRole,
    pub name: Option<String>,
    pub value: Option<String>,
    pub bounds: AccessibilityRect,
    pub window_id: Option<DisplayWindowId>,
    pub text_range: Option<AccessibilityTextRange>,
    pub state: AccessibilityState,
    pub children: Vec<AccessibilityNodeId>,
}
```

The tree should be coarse-grained:

```text
application/window
  frame
    menu_bar
    tool_bar
    tab_bar
    document window
      visible line
        text run
      cursor metadata
      scroll bar
    minibuffer
    popup menu / tooltip
```

Do not make every glyph an accessibility node. That would be noisy, expensive,
and unpleasant for screen readers. Glyph-level geometry can exist in debug
snapshots, but the normal accessibility tree should be line/text-run/control
oriented.

## Where To Build It

Build the snapshot from `FrameDisplayState`, before or during frame ingest.

Good insertion points:

- Pure conversion in `neomacs-display-protocol`:
  - `FrameDisplayState::accessibility_snapshot()`
  - best for unit testing and keeping renderer/runtime out of the core model
- Runtime dump in `neomacs-display-runtime/src/render_thread/frame_ingest.rs`:
  - if `NEOMACS_DEBUG_ACCESSIBILITY_TREE_JSON` is set, write the snapshot JSON
  - pairs naturally with existing surface readback debug hooks

Avoid building this in `neomacs-renderer-wgpu`. The renderer sees a flattened
`FrameGlyphBuffer`; it is too late in the pipeline and too tied to visual draw
details.

## Node Content

The first useful snapshot should include:

- frame id and pixel size
- top-level window nodes with:
  - `window_id`
  - selected state
  - minibuffer state
  - bounds
  - text bounds
  - `window_start`
  - `window_end`
  - buffer id
  - buffer name, if available
  - buffer file name
  - modified flag
- visible line nodes with:
  - role: text, mode-line, header-line, tab-line, tab-bar, minibuffer
  - row index
  - text
  - bounds
  - start/end char positions where available
  - bidi/reversed row state where useful
- cursor node or cursor metadata with:
  - active selected cursor
  - `DisplaySlotId`
  - char position
  - bounds
  - cursor style
- media/control nodes:
  - image
  - video
  - xwidget
  - scroll bar
  - menu item
  - toolbar item
  - popup/tooltip

One small protocol gap: `WindowInfo` currently has file name and modified flag,
but not buffer name. Accessibility and GUI tests should know the buffer name
without having to infer it from mode-line text, so add `buffer_name` to
`WindowInfo` when convenient.

## GUI Test Artifact

Add a new artifact beside PNG/readback/log files:

```text
target/neomacs-gui-tests/<backend>/<scenario>.accessibility.json
```

Recommended environment variable:

```text
NEOMACS_DEBUG_ACCESSIBILITY_TREE_JSON=/path/to/artifact.accessibility.json
```

The GUI harness should then record:

```json
{
  "status": "passed",
  "observed_artifacts": {
    "png_exists": true,
    "accessibility_exists": true,
    "accessibility_bytes": 12345
  },
  "accessibility": {
    "schema_version": 1,
    "visible_text": "...",
    "selected_window": "...",
    "nodes": []
  }
}
```

This lets an AI agent check GUI behavior with plain text:

- assert visible text contains expected lines
- assert selected window and minibuffer state
- assert cursor is inside the selected text window
- assert mode-line/tab-line nodes exist
- assert scroll bars and media nodes have nonzero bounds
- still keep PNG readback to prove pixels rendered

PNG and accessibility JSON should be treated as complementary:

- PNG: "did wgpu produce visible pixels?"
- accessibility JSON: "what UI state did NeoMacs think it displayed?"

## AccessKit Layer

AccessKit is the best Rust project to learn from and target.

Useful references:

- AccessKit overview: https://accesskit.dev/how-it-works/
- AccessKit repository: https://github.com/AccessKit/accesskit
- `accesskit_winit`: https://docs.rs/accesskit_winit/latest/accesskit_winit/
- egui accessibility notes: https://github.com/emilk/egui/blob/main/docs/accessibility.md
- Slint AccessKit announcement: https://slint.dev/blog/slint-1.1-released

AccessKit expects custom-rendered applications to provide an accessibility tree
with stable node ids, roles, names, bounds, values, children, and actions. That
matches the proposed internal snapshot well.

Do not start by wiring every platform API directly. Build the internal tree
first, then map it to AccessKit:

```text
FrameDisplayState
  -> AccessibilitySnapshot
  -> accesskit::TreeUpdate
  -> accesskit_winit::Adapter
```

This keeps tests deterministic and avoids making the first implementation
depend on AT-SPI/macOS/Windows runtime behavior.

## Platform Strategy

Linux X11:

- run under Xvfb for CI/headless tests
- emit PNG + accessibility JSON
- later validate AccessKit/AT-SPI in a smaller platform-specific smoke suite

Linux Wayland:

- run under Weston headless for CI/headless tests
- emit PNG + accessibility JSON
- later validate AccessKit/AT-SPI separately

macOS:

- prefer self-hosted or real desktop sessions for end-to-end window tests
- emit internal JSON from NeoMacs rather than relying on screenshots
- later validate NSAccessibility through AccessKit when the tree is stable

Windows:

- prefer an interactive desktop session or self-hosted runner for real GUI tests
- emit internal JSON from NeoMacs
- later validate UI Automation through AccessKit when the tree is stable

The cross-platform baseline should be internal snapshot tests plus optional
headless display harnesses. OS accessibility API tests should be smoke tests,
not the primary correctness oracle.

## Implementation Plan

1. Add pure accessibility snapshot types to `neomacs-display-protocol`.
2. Add unit tests for snapshot construction from synthetic `FrameDisplayState`.
3. Implement `FrameDisplayState::accessibility_snapshot()`.
4. Include text window, mode-line, minibuffer, cursor, scroll bar, and media
   nodes.
5. Add `NEOMACS_DEBUG_ACCESSIBILITY_TREE_JSON` in frame ingest.
6. Extend `neomacs-gui-tests` to expect and parse `.accessibility.json`.
7. Update real GUI smoke tests to assert visible text from the accessibility
   artifact, not from fixture-written Lisp side state.
8. Add `buffer_name` to `WindowInfo` and populate it from layout/NeoVM.
9. Add AccessKit conversion behind a feature after the JSON snapshot format is
   stable.
10. Add small platform-specific AccessKit smoke tests later.

## Testing Plan

Use `cargo nextest`, not `cargo test`.

Protocol/layout tests:

```text
cargo nextest run -p neomacs-display-protocol accessibility
cargo nextest run -p neomacs-layout-engine accessibility
```

Runtime/harness tests:

```text
cargo nextest run -p neomacs-display-runtime accessibility
cargo nextest run -p neomacs-gui-tests
```

Real GUI smoke tests should continue to use:

```text
target/release/neomacs
```

They should not use `target/debug/neomacs`.

## Risks

Accessibility tree drift:

If the accessibility tree is built from separate state, it will drift from the
rendered UI. Build it from `FrameDisplayState` so the same redisplay snapshot
feeds rendering, testing, and accessibility.

Too many nodes:

One node per glyph will be noisy and slow. Use line/text-run nodes by default.

Platform API instability:

AT-SPI, NSAccessibility, and UI Automation behavior depends on environment and
timing. Keep the primary test oracle as internal JSON, then add small OS-level
smoke tests.

Missing semantics:

Some content may not have ideal roles on day one. It is acceptable to start with
text, window, cursor, minibuffer, mode-line, scroll bar, image, video, and
xwidget nodes, then refine roles/actions later.

## Open Questions

- Should the accessibility snapshot live only in `FrameDisplayState`, or should
  `FrameGlyphBuffer` also be able to produce a best-effort snapshot for legacy
  or test-only paths?
- Should line nodes include face/style summaries, or should that remain a debug
  extension?
- How much Lisp-visible metadata should be carried through `WindowInfo`
  instead of queried separately?
- What stable node id scheme should be used for lines that change every
  redisplay? A practical first version can derive ids from frame id, window id,
  row role, and row index, then refine if screen-reader focus tracking needs
  better stability.

## Recommendation

Implement the internal accessibility snapshot first. Use it immediately in
`neomacs-gui-tests` as an AI-readable artifact. Treat AccessKit as the platform
adapter layer after the internal tree is proven.

This keeps NeoMacs aligned with GNU Emacs' redisplay model while adding the
semantic surface that custom winit/wgpu rendering needs for accessibility and
reliable GUI automation.
