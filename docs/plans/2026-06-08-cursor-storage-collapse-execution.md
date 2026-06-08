# Cursor Storage Collapse — Execution Plan

**Date:** 2026-06-08
**Companion:** `docs/plans/2026-06-08-cursor-architecture-design.md` (the "why").
This doc is the "what/how" for migration **Step 3–4**: collapse the two cursor
stores (`phys_cursor` + `window_cursors`) into one per-window list, then delete
the stored geometry so `cursor_draw_rect` is the sole geometry source.
**Status:** proposed. Not started.
**Ground rule:** every stage compiles, passes `cargo nextest`, is independently
committable, and is behavior-preserving unless explicitly noted. TTY + GUI
verification at each stage that touches a draw path.

---

## 1. The verified producer/consumer graph

### Types
- `PhysCursor` — `neomacs-display-protocol/src/frame_glyphs.rs`. Fields:
  `window_id, charpos, row, col, slot_id, x, y, width, height, ascent, style,
  color, cursor_fg`.
- `WindowCursorVisual` — same file. Fields: `window_id, slot_id, x, y, width,
  height, style, color` (no `charpos`/`ascent`/`cursor_fg`).
- `CursorItem` — layout-side twin in `neomacs-layout-engine/src/matrix_builder.rs`
  (`GlyphMatrix.cursors`), same shape as `WindowCursorVisual`.
- `CursorStyle` — `FilledBox | Hollow | Bar(f32) | Hbar(f32)` (`frame_glyphs.rs`
  / `cursor.rs`).
- `DisplaySlotId` — `{ window_id, row, col }`.
- `FrameGlyphBuffer` carries `phys_cursor: Option<PhysCursor>` and
  `window_cursors: Vec<WindowCursorVisual>`.

### Producers (writes)
| Site | What |
|---|---|
| `matrix_builder.rs::set_phys_cursor` | sets `phys_cursor`; resolves the column via `resolve_cursor_visual_col`; **syncs** the matching `CursorItem`'s slot (the dedup-enabler). |
| `matrix_builder.rs::push_cursor` | pushes a `CursorItem` to `self.cursors`. |
| `engine.rs:6372` | point-cursor `push_cursor` (ALL windows) + `set_phys_cursor` (selected only, `:6398`), under one bounds guard at `:6369`. |
| `engine.rs:6484` | a second `push_cursor` (EOB / secondary path) — must be handled the same as `:6372`. |
| `matrix_builder.rs:1280` | `state.cursors = self.cursors` → `FrameDisplayState` → `FrameGlyphBuffer.window_cursors` (conversion `CursorItem → WindowCursorVisual`). |
| `frame_glyphs.rs::add_cursor` | protocol-side push to `window_cursors` (tests + conversion). |

### Consumers (reads)
| Site | Uses `phys_cursor` | Uses `window_cursors`/`cursors` | What |
|---|---|---|---|
| `tty/mod.rs:604-617` | yes (`:615`) | yes (`:604`, dedup) | draw: non-selected from list, selected from `phys_cursor` |
| `renderer/content.rs:296` | yes (dedup ref) | yes (loop, dedup) | TTY-ish draw, same pattern |
| `renderer/glyphs.rs:1929 + 1955` | yes (`:1955`) | yes (`:1929`, dedup) | GPU draw, same pattern |
| `frame_windows.rs::cursor_target_for_frame` | yes | — | animation target (selected) |
| `frame_windows.rs:779` | — | yes (mut) | `apply_visual_cursor_animations`: overwrite per-window cursor x/y/w/h |
| `frame_state.rs:225-237` | yes (mut) | yes (mut) | `apply_extra_spacing`: remap x/y from `slot_positions` (both stores) |
| `render_pass.rs:401/460 + 410` | yes (mut) | yes (mut) | passes both to `apply_extra_spacing`; overrides `phys_cursor` x/y/w/h with the animated rect |
| `bidi_layout.rs:168-180, 270-279` | yes (mut) | yes (mut) | remap cursor x (reorder) and y (ascent) — already updates BOTH consistently |
| `engine.rs:2119` `find_window_cursor_y_in_builder` | — (reads `builder.cursors()`) | yes | finds a non-hollow cursor's y inside a window's bounds, for line-animation hints |
| effects (`phys_cursor_effects`, `find_cursor_pos`, `window_cursor_effects`) | yes | yes | glow/spotlight/trail keyed off the active cursor |

### The decisive finding
All three drawing backends use the identical shape:

```rust
for c in &frame.window_cursors {
    if frame.phys_cursor.is_some_and(|p| window_cursor_visual_matches_phys(c, p)) { continue; }
    draw(c);                       // non-selected windows
}
if let Some(p) = &frame.phys_cursor { draw(p); }   // selected window
```

So the **selected window's `window_cursors` entry is always deduped away** in
every backend; the selected cursor is sourced from `phys_cursor` everywhere. The
selected entry exists only to be (a) synced by `set_phys_cursor`, (b) skipped by
the dedup, (c) scanned by `find_window_cursor_y_in_builder` and the x/y remappers
(which never draw it). That redundancy is bug C's root.

---

## 2. Target model

```rust
// display-protocol. One entry per window. No phys/visual split.
struct WindowCursor {
    window_id: i64,
    at: SlotId,            // the reference; geometry derived via cursor_draw_rect
    kind: CursorKind,      // Filled | Hollow | Bar(f32) | Hbar(f32) | NoCursor
    color: Color,
    cursor_fg: Color,      // box cursor inverts the glyph under it
    ascent: f32,           // non-geometry metric some draws need
    active: bool,          // the selected window's cursor (effects/animation target)
    // NB: no x/y/width/height once Stage C lands.
}

// FrameGlyphBuffer: replace `phys_cursor` + `window_cursors` with:
cursors: Vec<WindowCursor>,
```

The active cursor (for animation + effects) is `cursors.iter().find(|c| c.active)`.
Geometry is `cursor_draw_rect(matrix, c.at, c.kind)` for every cursor, selected or
not. `window_cursor_visual_matches_phys` and the dedup disappear (one entry per
window — nothing to dedup).

---

## 3. Staged execution

### Stage A — Delete the redundant selected-window cursor (behavior-preserving)
Removes the dual storage *for the selected window* without introducing the new
type yet. Highest value-to-risk: it kills the sync loop and makes bug C
impossible, in ~3 files.

- `engine.rs`: gate both point-cursor `push_cursor` sites (`:6372`, `:6484`) on
  `!params.selected`, so the selected window is represented solely by
  `phys_cursor`. (Safe: every backend already deduped that entry.)
- `matrix_builder.rs::set_phys_cursor`: delete the sync loop (no `CursorItem` to
  sync now).
- `engine.rs::find_window_cursor_y_in_builder` (`:2119`): also consult
  `builder.phys_cursor()` so the selected window's cursor-y for line animations
  is preserved (it used to come from the now-absent `CursorItem`).
- Leave the dedup in the three backends as a defensive no-op (removed in Stage D).
- Tests: replace `set_phys_cursor_syncs_the_redundant_window_cursor_slot` with
  `selected_window_pushes_no_redundant_window_cursor`; keep all resolver tests.
- **Verify:** TTY oracle + GUI — selected cursor still drawn; a split with a
  non-selected window still shows that window's hollow cursor; `SPC h d h → gg →
  j` still single + flush.
- Risk: low. One subtlety to check: any effect that read the selected window's
  `window_cursors` entry (none found — effects key off `phys_cursor`).

### Stage B — Unify into one `Vec<WindowCursor>` (keep geometry fields)
Introduce the new type and the single list; delete `phys_cursor` +
`window_cursors`. **Keep** `x/y/width/height` on `WindowCursor` for now so
`apply_extra_spacing` / `apply_visual_cursor_animations` / `bidi_layout` keep
working unchanged. Behavior-preserving: draws still resolve through
`cursor_draw_rect(at)`; the geometry fields remain the fallback.

- display-protocol: add `WindowCursor`; change `FrameGlyphBuffer` to
  `cursors: Vec<WindowCursor>`; add `active_cursor()` / `active_cursor_mut()`.
- layout: build one `WindowCursor` per window into `cursors`; the selected one
  has `active = true` and `kind = FilledBox` (or buffer/window cursor type), the
  rest `active = false`, `kind = Hollow` (per `cursor_in_non_selected_windows`).
  Fold `resolve_cursor_visual_col` + the box/bar/stretch resolution into building
  this single entry.
- Update every consumer to the one list:
  - draws (TTY `mod.rs`, `content.rs`, GPU `glyphs.rs`): `for c in &frame.cursors
    { draw(cursor_draw_rect(frame, c.at, c.kind), c) }`; drop the dedup branch.
  - `cursor_target_for_frame`, effects: use `frame.active_cursor()`.
  - `apply_extra_spacing`, `apply_visual_cursor_animations`, `bidi_layout`:
    iterate the one list (still adjusting the retained geometry fields).
  - `render_pass.rs:410` animated-rect override: apply to the active cursor.
- Tests: migrate every `PhysCursor { … }` / `add_cursor(…)` /
  `WindowCursorVisual` construction (see §4) to `WindowCursor`.
- **Verify:** TTY + GUI, including a multi-window split (selected filled +
  non-selected hollow), the heading repro, and the blank-line repro.
- Risk: medium. Large but mechanical; the behavior-preservation lever is keeping
  the geometry fields.

### Stage C — Derive geometry; delete the stored geometry fields
Remove `x/y/width/height` from `WindowCursor`. `cursor_draw_rect(matrix, at,
kind)` becomes the sole source. This is design-doc Step 4 and the real payoff.

- Delete the geometry fields from `WindowCursor`.
- `apply_extra_spacing` / `apply_visual_cursor_animations` / `bidi_layout`
  cursor-remap blocks: **delete** the cursor branches. The glyphs they key off
  already carry the spacing/bidi-adjusted positions, and `cursor_draw_rect`
  resolves the cursor from those glyphs at draw time — so re-deriving per frame is
  automatically correct. (Keep the glyph-adjustment code; only the cursor-copy
  adjustment goes away.)
- Animation: the slide target is `cursor_draw_rect(matrix, active.at, kind)`;
  `render_pass.rs:410` interpolates render-local state toward it (no stored
  cursor geometry to override).
- `find_window_cursor_y_in_builder`: derive y from the matrix row of `at`.
- **Verify:** TTY + GUI with line-spacing/letter-spacing set, an RTL line, and
  the cursor slide animation (the bug-D scenario) — confirm no second box, no
  drift under spacing or bidi.
- Risk: medium-high (touches spacing/bidi/animation). Mitigate by verifying each
  sub-path (spacing, bidi, animation) separately.

### Stage D — Remove the dedup and finalize
- Delete `window_cursor_visual_matches_phys` and the dedup branch in all three
  backends (one entry per window — nothing to dedup).
- Delete `WindowCursorVisual`, `CursorItem`, `PhysCursor` and any now-dead
  conversion code.
- Update `drafts/cursor-audit.md` parity rows that referenced the old split.
- **Verify:** full `cargo nextest`; TTY oracle suite; GUI smoke (heading,
  blank line, split window, animation).

---

## 4. Tests to migrate (constructors of the old types)
- `neomacs-display-protocol/src/frame_glyphs_test.rs`: `set_phys_cursor_*`,
  `add_cursor_*`, `cursor_draw_rect_*`, `cursor_cell_rect_*`, `full_frame_simulation`
  (constructs `PhysCursor` + `add_cursor`).
- `neomacs-layout-engine/src/matrix_builder_test.rs`: `builder_preserves_phys_cursor`,
  `builder_remaps_phys_cursor_to_visual_bidi_column`,
  `phys_cursor_slot_col_accounts_for_line_number_gutter`,
  `phys_cursor_on_hidden_prefix_resolves_to_first_visible_glyph`,
  `set_phys_cursor_syncs_the_redundant_window_cursor_slot` (deleted in Stage A),
  `resolve_cursor_visual_col_*`.
- `neomacs-layout-engine/src/bidi_layout_test.rs`: `make_phys_cursor`,
  `test_active_phys_cursor_moves_in_mixed_text`.
- `neomacs-renderer-wgpu/src/renderer/glyphs_test.rs`: `window_cursor_visual_match_uses_slot_identity`
  (deleted with the dedup in Stage D), RTL cursor tests.
- `neomacs-display-runtime/src/render_thread/...`: cursor animation/target tests.

Keep each test's *intent*; only the constructor and field names change. The
resolver tests (`resolve_cursor_visual_col_*`) are unaffected.

---

## 5. Verification strategy
- **Unit:** `cargo nextest` per touched crate after every stage; the resolver and
  `cursor_draw_rect` suites are the regression backbone.
- **TTY oracle:** the TTY comparison tests (the cursor is drawn into the
  `TtyGrid` deterministically) — the cheapest end-to-end cursor check; run after
  Stages A, B, C, D.
- **GUI:** the standing repros, each rebuilt with the pdump regen
  (`xtask fresh-build --release --skip-build --no-byte-compile`, see
  `reference_incremental_build_pdump`):
  1. `SPC h d h → gg → j → j` — single cursor, flush past the gutter on the blank
     line, on the glyph on text lines (bugs A, B, E).
  2. A heading line with `l` repeated — single box that tracks (bugs C, D).
  3. A horizontal split (`C-x 3`) — selected window filled box + other window
     hollow box, both correctly placed (the multi-window path Stage B/C rework).
  4. Line-spacing / letter-spacing set, and an RTL line — cursor stays on its
     glyph (Stage C).

## 6. Risk register
- **R1 multi-window draw regression (Stage B/C).** The collapse changes how
  non-selected cursors are produced. Mitigate: verify a split window in TTY +
  GUI; non-selected = hollow, selected = filled.
- **R2 spacing/bidi cursor drift (Stage C).** Deleting the cursor-copy adjustment
  relies on `cursor_draw_rect` re-deriving from already-adjusted glyphs. Mitigate:
  verify with line-spacing and an RTL line explicitly.
- **R3 animation double-box regression (Stage C).** The slide must target
  `cursor_draw_rect(active.at)`, not stored geometry. Mitigate: the bug-D heading
  repro.
- **R4 pdump churn.** Every GUI check needs a binary rebuild + pdump regen; budget
  for it, do not skip (the fix is found in the GUI, per session history).

## 7. Recommended cut line
Stages **A + B** deliver the structural win — one store, no redundancy, bug C
impossible — at low/medium risk, and are a sensible first PR. Stages **C + D**
(derive-only geometry, delete the old types) are the ideal end-state and a
natural second PR once A+B are confirmed live.

## 8. Status (2026-06-08)

**Landed + GUI-verified:**
- **Stage A** (`37a0c5485`): selected-window `push_cursor` guarded on
  `!params.selected`; `set_phys_cursor` sync loop deleted;
  `find_window_cursor_y_in_builder` consults the phys cursor. Behavior-preserving.
- **Stage B** (`bbee33b15`): `FrameGlyphBuffer` now holds one
  `window_cursors: Vec<WindowCursor>`, one entry per window, the selected one
  `active`. `active_cursor()` replaces `phys_cursor`; every backend draws one
  merged loop (no dedup, no separate phys draw). GUI-verified: single window
  (single cursor, flush on the blank line) and a `C-x 3` split
  (`window_cursors=1 active_cursors=1`, selected cursor drawn, no duplicate).
- **Stage D substance** (folded into B): `window_cursor_visual_matches_phys` and
  the dedup branches are gone; `WindowCursorVisual` is replaced by `WindowCursor`.
  `PhysCursor` and `CursorItem` are NOT deleted — they remain the layout-internal
  types (they carry `charpos`/`row`/`col` for column resolution and the
  CursorItem decorative path).

The two-representations-of-one-cursor bug class is now structurally impossible:
there is one cursor per window, and `cursor_draw_rect` (slot-keyed) is the primary
draw resolver for every backend.

**Stage C — deferred (entangled, low marginal value).** Implementation revealed
the stored geometry on `WindowCursor` is not vestigial; it carries two things the
glyph matrix does not:
1. **Cursor positioning/sizing policy.** `cursor_draw_rect` takes `y`/`height`
   from the fallback (stored geometry), not the glyph. That geometry comes from
   `resolve_cursor_geometry`, which encodes Hbar bottom-offset `y`, x-stretch
   width clamping, EOB sizing, and row-height policy. The raw slot glyph does not
   carry these, so deriving `y`/`height` from it would mis-place Hbar and stretch
   cursors.
2. **The slide-animation interpolation buffer.** `render_pass.rs:410` overwrites
   the active cursor's geometry with `render.cursor`'s interpolated rect each
   frame, and `emit_cursor_visual` draws the static box at that `static_rect`. So
   the geometry fields ARE the slide's per-frame state.

Deleting the fields therefore means re-deriving the cursor sizing policy from the
matrix AND rewiring the slide so the static box reads render-local animation
state — a GUI-visible-risk re-architecture, not a clean field deletion. Given A+B
already removed the bug class and made `cursor_draw_rect` the primary path, the
marginal value is low. Recommended as its own focused effort with dedicated
animation (slide), Hbar, and stretch-cursor GUI verification. Until then,
`cursor_draw_rect` deriving x/width from the glyph (done) already keeps the
drawn cursor on its glyph; only the y/height fallback and the slide retain the
stored geometry.
