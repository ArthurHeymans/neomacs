# Accessibility and GUI Testability Design

**Date:** 2026-06-25
**Status:** Proposal
**Scope:** How NeoMacs should expose semantic GUI state for accessibility and
AI-readable visual tests across winit/wgpu, X11, Wayland, macOS, and Windows.

## Summary

NeoMacs should add an internal semantic GUI snapshot produced from the display
state, then later expose an accessibility view of that same snapshot through
AccessKit.

This gives two benefits from one source of truth:

- AI-readable GUI tests can assert text, roles, bounds, cursor position, and
  window state from JSON without OCR or manual image inspection.
- Native accessibility can be layered on later through AccessKit adapters for
  AT-SPI, NSAccessibility, and UI Automation.

The internal object should be treated as the canonical display oracle, not only
as an accessibility API model. Accessibility roles are one consumer; deterministic
GUI tests are another. The snapshot should therefore include both a role tree and
test-friendly summary fields such as visible text, selected window, minibuffer
text, active cursor, validation warnings, and frame identity.

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

The existing fixture-written GUI state artifact is useful, but it is not a
display oracle. It reports what Lisp test code intended or observed through Lisp
state. The proposed semantic snapshot should report what NeoMacs actually
published to the display pipeline for a specific frame.

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

Add an internal semantic display/accessibility snapshot to
`neomacs-display-protocol`.

Suggested module:

```text
neomacs-display-protocol/src/accessibility.rs
```

Suggested top-level types:

```rust
pub struct AccessibilitySnapshot {
    pub schema_version: u32,
    pub frame_id: DisplayFrameId,
    pub frame_sequence: Option<u64>,
    pub frame_bounds: AccessibilityRect,
    pub capture_stage: AccessibilityCaptureStage,
    pub visible_text: String,
    pub selected_window: Option<AccessibilityNodeId>,
    pub minibuffer_text: Option<String>,
    pub active_cursor: Option<AccessibilityCursor>,
    pub validation: AccessibilityValidation,
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

The exact type names can change, but the separation should remain:

- summary fields for common test assertions
- a coarse semantic node tree for accessibility and detailed inspection
- validation output for catching malformed display state early
- frame identity fields so the JSON can be matched to a PNG/readback capture

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

Use two snapshot density profiles:

- normal: frame, windows, visible lines, cursor, minibuffer, chrome, controls,
  media nodes, and coarse style/action semantics
- debug: optional glyph runs, face ids/full style summaries, bidi levels, clip
  rects, property/overlay provenance, and other data needed to diagnose
  rendering/layout issues

The default GUI test artifact should use the normal profile.

## Stable Node IDs

Use deterministic structured node ids from the start. A practical first scheme:

```text
root
frame:<frame_id>
window:<window_id>
row:<window_id>:<role>:<row_index>
run:<window_id>:<role>:<row_index>:<run_index>
cursor:<window_id>
scrollbar:<window_id>:<orientation>
image:<window_id>:<image_id>
video:<window_id>:<video_id>
xwidget:<window_id>:<xwidget_id>
```

This is stable enough for GUI tests and initial AccessKit mapping. If
screen-reader focus tracking later needs stronger stability across insertions or
scrolls, row/run ids can be refined to include buffer positions when available.

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

Implementation nuance: the runtime ingest path materializes a `FrameDisplayState`
into `FrameGlyphBuffer` before rendering. The cleanest dump point is just before
that materialization, while row/window structure is still rich. A best-effort
`FrameGlyphBuffer` snapshot can be added later for legacy/test-only paths, but it
should not become the canonical source if `FrameDisplayState` is available.

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

## Resolved Display Semantics

Faces, text properties, and overlays should appear in the snapshot as resolved
display semantics, not as raw Lisp implementation dumps.

Faces should usually be style runs on text nodes, not standalone accessibility
nodes. A line or text-run node can carry coarse style information:

```json
{
  "id": "row:42:text:1",
  "role": "text",
  "text": "let value = 1;",
  "style_runs": [
    { "range": [0, 3], "faces": ["font-lock-keyword-face"] },
    { "range": [4, 9], "faces": ["font-lock-variable-name-face"] }
  ]
}
```

The normal profile should include only style data that is useful for user-visible
behavior and stable GUI tests: face names when available, broad attributes such
as bold/italic/underline, and semantic highlights such as region, link, match,
error, warning, success, mode-line, cursor, or minibuffer-prompt. Full face ids,
resolved colors, font metrics, underline colors, overlay priorities, and exact
property sources belong in the debug profile.

Text properties and overlays should be lowered to the behavior they create:

- `invisible`: omitted/collapsed ranges, with optional debug provenance
- `display`: replacement text, stretch, image, video, or xwidget nodes
- `button`, `keymap`, `local-map`, `action`: role/action metadata such as
  `link`, `button`, and `activate`
- `help-echo`: accessible description or tooltip text
- `field`: input/minibuffer field boundaries
- `read-only`: read-only state
- `composition`: displayed grapheme/composed text
- `mouse-face`: hover style/debug metadata

For example:

```json
{
  "id": "run:42:text:3:0",
  "role": "link",
  "text": "README.md",
  "source_range": [120, 129],
  "bounds": { "x": 32, "y": 84, "width": 72, "height": 18 },
  "actions": ["activate"],
  "description": "Open README.md",
  "style": { "faces": ["link"] }
}
```

The key rule is that the default snapshot exposes what the user can perceive or
act on. Debug snapshots may include raw-ish provenance such as property names,
overlay ids, overlay priorities, face ids, and resolved colors/fonts when needed
to debug redisplay.

## Snapshot Validation

Each snapshot should run a cheap structural validation pass and include the
result in JSON. Validation should not panic in production/debug artifact mode;
it should report errors and warnings that tests can assert.

Initial checks:

- exactly one root node
- every child id exists and every non-root parent exists
- node bounds are finite and non-negative
- window and row bounds fit the frame unless explicitly clipped
- selected window exists and is unique
- active cursor belongs to the selected window when present
- cursor bounds are non-empty and inside the selected window/text area when
  available
- text ranges are ordered and fit known `window_start`/`window_end` metadata
- media and scroll bar nodes have nonzero bounds

Protocol unit tests should cover both valid snapshots and representative invalid
states so the validation schema stays useful.

## GUI Test Artifact

Add a new artifact beside PNG/readback/log files:

```text
target/neomacs-gui-tests/<backend>/<scenario>.accessibility.json
```

Recommended environment variable:

```text
NEOMACS_DEBUG_ACCESSIBILITY_TREE_JSON=/path/to/artifact.accessibility.json
```

The GUI harness should add this path to `GuiArtifactSet` immediately, even before
the runtime writer lands, so the harness contract is concrete:

```text
GuiArtifactSet {
  json,
  png,
  stderr,
  gui_state,
  accessibility,
}
```

The runtime should write the JSON atomically: serialize to a temporary file in
the same directory, flush it, then rename it over the destination. GUI smoke
tests often terminate the process after capture; atomic writes avoid partial JSON
artifacts.

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
    "frame_id": 123,
    "frame_sequence": 7,
    "visible_text": "...",
    "selected_window": "...",
    "minibuffer_text": null,
    "validation": {
      "errors": [],
      "warnings": []
    },
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

They should also be frame-synchronized. The PNG/readback manifest and the
accessibility artifact should carry the same frame id or frame sequence whenever
possible. If exact synchronization is unavailable at first, the manifest should
say so explicitly rather than implying that two independently captured artifacts
describe the same frame.

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

1. Extend `neomacs-gui-tests` with an `.accessibility.json` artifact path and
   planned/result manifest fields.
2. Add pure semantic/accessibility snapshot types to
   `neomacs-display-protocol`, including summary fields and validation output.
3. Add deterministic structured node ids.
4. Add unit tests for snapshot construction from synthetic `FrameDisplayState`.
5. Implement `FrameDisplayState::accessibility_snapshot()`.
6. Include text window, mode-line, minibuffer, cursor, scroll bar, and media
   nodes.
7. Lower display-affecting faces, text properties, and overlays into style runs,
   roles, actions, descriptions, and replacement/media nodes.
8. Add a debug profile for face ids, full resolved style, and property/overlay
   provenance.
9. Add snapshot validation tests.
10. Add `NEOMACS_DEBUG_ACCESSIBILITY_TREE_JSON` in frame ingest, writing
   atomically from the pre-materialized `FrameDisplayState` when available.
11. Update real GUI smoke tests to assert visible text from the accessibility
   artifact, not from fixture-written Lisp side state.
12. Add `buffer_name` to `WindowInfo` and populate it from layout/NeoVM.
13. Add AccessKit conversion behind a feature after the JSON snapshot format is
   stable.
14. Add small platform-specific AccessKit smoke tests later.

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

Artifact desynchronization:

If PNG readback and semantic JSON are captured from different frames, tests can
pass or fail for the wrong reason. Include frame identity in both artifacts and
prefer capturing the semantic snapshot on the same frame-readback path.

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

Privacy and artifact size:

The semantic snapshot may contain visible buffer text and file names. Keep it
behind explicit debug/test environment variables, keep the default profile
coarse, and reserve glyph/style dumps for opt-in debug profiles.

## Open Questions

- Is there already a stable frame sequence number that can be shared by
  readback PNG and semantic JSON, or should one be added to the display
  protocol?
- Should `FrameGlyphBuffer` also be able to produce a best-effort snapshot for
  legacy or test-only paths, or is `FrameDisplayState` coverage complete enough?
- Should line nodes include face/style summaries, or should that remain a debug
  extension?
- Which resolved text properties should be promoted into the normal profile
  first: links/buttons, help text, invisible/display, field/read-only, or
  composition?
- How much Lisp-visible metadata should be carried through `WindowInfo`
  instead of queried separately?
- What artifact retention policy should CI use, given that semantic snapshots
  can contain visible buffer text and file paths?

## Recommendation

Implement the internal semantic display snapshot first. Use it immediately in
`neomacs-gui-tests` as an AI-readable artifact and display oracle. Treat
AccessKit as the platform adapter layer after the internal tree and JSON schema
are proven.

This keeps NeoMacs aligned with GNU Emacs' redisplay model while adding the
semantic surface that custom winit/wgpu rendering needs for accessibility and
reliable GUI automation.
