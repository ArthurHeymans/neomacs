# GUI Observability and Agent Drivability Design

**Date:** 2026-07-02
**Status:** Proposal (supersedes and replaces
`2026-06-25-accessibility-gui-testing-design.md`)
**Scope:** How an AI agent (or any automation) observes what the NeoMacs GUI
actually displays and drives it interactively. Accessibility (AccessKit) is a
deferred consumer of the same data, not part of this design's deliverables.

**Goal, in one sentence:** agents work in plain text — they cannot usefully
read screenshots — so every GUI fact an agent needs (displayed text, resolved
colors, element geometry, cursor, chrome) must be obtainable as text, on
demand, from a live or test instance.

## Problem

Agents developing NeoMacs can test the TUI (text grid via tmux, batch oracle
suite) but the GUI is a black box:

- The PNG readback proves wgpu produced pixels; it cannot answer "which text,
  which faces, where is the cursor, what does the mode-line say".
- The existing `gui-state.json` artifact is written *by the fixture's Lisp
  code* from Lisp-visible state (`buffer-substring (window-start) (window-end)`
  in `neomacs-gui-tests/fixtures/startup-smoke.el`). It reports what Lisp
  intended, not what redisplay produced — no faces, no truncation/invisible
  handling, no cursor geometry, no mode-line/chrome content.

Two primitives are missing, and neither requires new display machinery:

1. **Observe** — "what did redisplay actually produce for this frame?"
2. **Drive** — "inject input into the live GUI event loop, then observe again."

## GNU Emacs Grounding

GNU's display truth is the glyph matrix, and GNU's own debug mechanism for
exactly this need is to dump it:

- `/home/exec/Projects/github.com/emacs-mirror/emacs/src/xdisp.c`:
  `dump-glyph-matrix`, `dump-frame-glyph-matrix`, `dump-glyph-row`,
  `trace-redisplay` — defined under `GLYPH_DEBUG` (`--enable-checking`
  builds), printing the current matrices to stderr.

NeoMacs already mirrors the model in plain data types, produced by the layout
engine *before* any renderer runs:

- `neomacs-display-protocol/src/glyph_matrix.rs`: `FrameDisplayState`,
  `WindowMatrixEntry`, `GlyphRow`, `Glyph`
- `neomacs-layout-engine/src/engine.rs:178`:
  `last_frame_display_state: Option<FrameDisplayState>` — the last completed
  frame is already retained on the VM/layout side
- The same `FrameDisplayState` feeds both the wgpu backend and the TTY
  rasterizer, so one snapshot mechanism covers GUI and TTY frames.

So the design is a port of GNU's mechanism — expose the real glyph state —
not an invention of a semantic interpretation layer.

**Parity note:** GNU's dump subrs exist only in `GLYPH_DEBUG` builds; the
oracle reference binary (release 31.0.50) does not define them. Defining
`dump-glyph-matrix` unconditionally would create an `fboundp` divergence.
The snapshot function therefore lives in the internal `neomacs--` namespace.
A `GLYPH_DEBUG`-faithful `dump-glyph-matrix` behind a cargo feature can come
later if ever needed.

## Design

### Phase 1 — Observe: the frame snapshot subr

New subr in `neovm-core`:

```elisp
(neomacs--frame-snapshot &optional FRAME FORMAT)   ; => string
(neomacs--write-frame-snapshot PATH &optional FRAME FORMAT) ; => t
```

- Forces a complete redisplay of FRAME (default: selected frame), then
  serializes the resulting `FrameDisplayState`. Synchronous on the VM thread;
  the render thread is never involved (respects the render-thread-must-not-
  touch-VM-values rule). Synchronous request/response also makes frame
  identity trivial — no artifact/frame synchronization machinery needed.
- FORMAT is `text` (default), `text-faces` (text plus per-row face-run
  annotations), or `json`. All renderings come from the same
  `FrameDisplayState`; there is no separate schema to drift.

**JSON = the real protocol types.** Derive `serde::Serialize` on
`FrameDisplayState` and its constituents (workspace already has
`serde = { version = "1", features = ["derive"] }`). The JSON *is* the
struct. Full fidelity by construction, because `FrameDisplayState` is
self-contained: glyphs with `face_id`/charpos/pixel metrics, the
`faces: HashMap<u32, Face>` table with **resolved** `foreground`/`background`
colors per face, frame/char pixel dimensions, cursors (`phys_cursor`,
`CursorItem`), window boxes (`WindowInfo`), images/videos/xwidgets/scroll
bars, and menu/tool/tab bars. Whatever the renderer sees, the agent sees —
text, color, and geometry included.

**Text = a greppable logical grid.** Pure function
`FrameDisplayState::render_text()` in `neomacs-display-protocol`: per window
a header line (window id, buffer, bounds, window-start/end), then one text
line per glyph row with the cursor position marked, plus labeled sections for
mode-line / header-line / tab-line / minibuffer / menu-bar / tab-bar.
Agents read this the way they read a tmux capture. The `text-faces` variant
additionally lists each row's face runs with resolved hex colors
(`cols 0-5 face=font-lock-keyword-face fg=#51afef bg=#282c34`), so "what
color is this element" is answerable by grep, without JSON parsing.
One protocol gap: `Face` today carries `id` + resolved colors/attributes but
not the Lisp face name (only the fixed basic faces have canonical names).
GNU's realized `struct face` keeps its `lface` vector, so the name is known
VM-side at realization time — add `lisp_name: Option<String>` to the
protocol `Face` so snapshots and tests can assert faces by name. With
proportional fonts the text grid is logical (row/column), not pixel-aligned;
pixel geometry is authoritative in the JSON.

### Phase 1b — GUI test harness integration

- Fixtures call `neomacs--write-frame-snapshot` directly — no
  environment-variable plumbing in the runtime, no render-thread dump hook.
- `GuiArtifactSet` gains `<scenario>.frame-snapshot.json` and
  `<scenario>.frame-snapshot.txt` beside the existing png/log/gui-state
  entries; the manifest records their presence and sizes.
- `real_gui_smoke` asserts visible text, cursor location, and mode-line
  content from the snapshot instead of from fixture-written Lisp state.
  `gui-state.json` may remain during a transition, but the snapshot is the
  display oracle.

### Phase 2 — Drive: the live agent loop

The eval channel already exists: `lisp/server.el` ships, and
`make-network-process` supports Unix local sockets. The work is verification
and documentation, plus fixing whatever real bugs surface:

1. Verify `server-start` + `emacsclient --eval` against a *GUI* NeoMacs under
   Xvfb / Weston headless (release build). Server traffic exercises
   network-process wakeups inside the GUI event loop — the same class as the
   accept-process-output starvation bug — so failures here are real
   event-loop bugs to fix at the root, not blockers to route around.
2. Document the loop (docs/gui-agent-testing.md):

```text
Xvfb :99 &  (or weston --backend=headless)
DISPLAY=:99 target/release/neomacs -Q --eval "(server-start)" &
emacsclient -s /run/user/…/emacs/server --eval \
  '(progn (execute-kbd-macro (kbd "C-x b foo RET"))
          (neomacs--frame-snapshot nil (quote text)))'
```

Input goes through elisp (`execute-kbd-macro`, `unread-command-events`,
synthesized mouse events via posn), which drives the real command loop.
Native injection (xdotool/XTEST, wtype) stays a documented escape hatch for
bug classes the elisp channel cannot reach (raw modifier handling, IME,
focus) — not built until needed.

### Phase 3 — Deferred: AccessKit as a product feature

Real screen-reader accessibility is worth having eventually (GNU Emacs has no
native accessibility tree), and the pipeline is:

```text
FrameDisplayState → semantic projection → accesskit::TreeUpdate
                                        → accesskit_winit::Adapter (winit 0.30)
```

It is explicitly out of scope here because it adds nothing for testing:

- The tree would be *built from* the snapshot, so tests already have strictly
  more information reading the snapshot directly.
- AccessKit's schema is closed and semantic: roles, bounds, text runs, coarse
  text styling. No faces by name, no overlay/property provenance, no buffer
  positions or window-start/end, no fringe/animation/render internals, no
  custom properties, no frame identity.
- Consuming it externally means AT-SPI/UIA: async, cached, environment-
  dependent, lossy — the wrong oracle for deterministic tests.

## What this replaces (vs. the 2026-06-25 doc)

- No parallel `AccessibilitySnapshot` schema, node-ID grammar, role
  vocabulary, style-run lowering, validation subsystem, or density profiles —
  serialize the real `FrameDisplayState` instead. An interpretation layer can
  drift from the rendered UI (the old doc's own top risk); the real struct
  cannot.
- No env-var-triggered dump in `frame_ingest.rs` — that runs on the render
  thread and yields one-shot artifacts. A synchronous subr on the VM side is
  simpler, thread-safe by construction, and usable interactively.
- Adds the missing half: the interactive drive loop. A one-shot artifact
  lets agents run pre-written fixtures; it does not let them debug.
- Test artifact is full-fidelity by default. The old doc's coarse "normal
  profile" would have hidden exactly the things that are hard to verify today
  (faces, precise layout).

## Testing Plan

Use `cargo nextest`, output redirected to a file under `./tmp/`.

- `neomacs-display-protocol`: serde round-trip of a synthetic
  `FrameDisplayState`; golden tests for `render_text()` (cursor marker,
  chrome sections, window headers).
- `neovm-core`: subr tests on TTY frames (the snapshot works for TTY too,
  since the TTY rasterizer consumes the same struct) — no oracle comparison,
  GNU lacks the subr by design.
- `neomacs-gui-tests`: harness-contract tests for the new artifact fields;
  `real_gui_smoke` (release binary, Xvfb) asserting snapshot content.
- Phase 2: a smoke script driving a live GUI instance via emacsclient and
  asserting on successive snapshots.

## Risks

- **Pre-render truth.** The snapshot captures display state before wgpu
  runs; font shaping/fallback/antialiasing bugs are invisible to it. PNG
  readback and renderer unit tests remain the pixel oracle — the two are
  complementary, as today.
- **serde on protocol types** adds derive weight to a hot crate. Accept it;
  feature-gate only if compile time measurably regresses.
- **server.el under the GUI loop** may surface real event-loop bugs.
  Expected and desirable: fix at the root per project policy.
- **Snapshot size.** A full frame JSON is large. Fine for artifacts and
  interactive use; if it ever matters, filter windows/rows at the caller, do
  not add density profiles to the schema.

## Implementation Steps

1. `serde::Serialize` for `glyph_matrix.rs` / `frame_glyphs.rs` types +
   round-trip test.
2. `lisp_name` on protocol `Face`, populated at face realization.
3. `FrameDisplayState::render_text()` + golden tests.
4. `neomacs--frame-snapshot` / `neomacs--write-frame-snapshot` subrs: force
   redisplay, read `engine.last_frame_display_state`, serialize.
5. Harness: new `GuiArtifactSet` entries, manifest fields, fixture switch,
   smoke assertions from the snapshot.
6. Verify/fix `server-start` + `emacsclient` on the GUI build; write
   `docs/gui-agent-testing.md` with the canonical agent loop.
7. (Separate, later) AccessKit adapter fed by the same snapshot.
