# Cursor Architecture: Why It Drifts, and the Single-Source Design

**Date:** 2026-06-08
**Scope:** The text cursor's position pipeline — how point becomes a drawn
caret, across the layout pass, the layout→render thread boundary, the GPU
renderer, the TTY backend, the bidi reorderer, and the slide animation.
**Status:** Stages 1–5 of the unification, plus blank-line bug E and the
empty-slot snap, are **landed on `main`**. The structural end-state (collapsing
the dual cursor storage) and the broader generalization are **proposed** here.
**References:**
- Prior audit: `drafts/cursor-audit.md` (2026-04-09) — neomacs-vs-GNU cursor
  surface inventory (17 findings). This doc is the architecture follow-up: it
  explains *why* the subsystem produced a cluster of bugs and *what* the
  single-source end-state is.
- GNU Emacs: `/home/exec/Projects/github.com/emacs-mirror/emacs` —
  `src/xdisp.c` (`set_cursor_from_row`, `draw_phys_cursor_glyph`,
  `display_and_set_cursor`, `get_window_cursor_type`), `src/dispextern.h`
  (`struct glyph`, `struct glyph_row`, `struct glyph_matrix`, `w->phys_cursor`,
  `struct cursor_pos`).
- neomacs source: `neomacs-display-protocol/src/frame_glyphs.rs`,
  `neomacs-layout-engine/src/{matrix_builder,engine,bidi_layout}.rs`,
  `neomacs-renderer-wgpu/src/renderer/glyphs.rs`,
  `neomacs-display-runtime/src/render_thread/{frame_windows,frame_ingest}.rs`,
  `neomacs-display-runtime/src/backend/tty/mod.rs`.

**Ground rule (standing, from the user):** neomacs keeps 100% identical
*observable* behavior to GNU Emacs (semantics, logical display). Below the
observable contract it is free to be better, but the cursor's *model* should
follow GNU's, because GNU's model is what makes the cursor correct by
construction.

---

## Executive summary

In one interactive bug-hunt driving Doom Emacs, the text cursor produced **five
distinct bugs**. They were not five unrelated defects — they were one design
flaw surfacing five times:

> The cursor's position is **stored as geometry in ~six independent places**,
> each recomputed from different inputs. When any two disagree, the cursor is
> drawn in the wrong place, or twice.

The bugs stayed hidden for normal monospace ASCII text because the cheapest
representation — a grid estimate `x = column × average_char_width` — happens to
equal the true glyph position there. A proportional or scaled font (an org
heading), a line-number gutter, hidden text, or a blank line each break that
coincidence, and a different pair of representations diverges.

GNU Emacs never has this class of bug because its cursor is **an index into a
single glyph matrix**, resolved once, with geometry read from the indexed glyph
at draw time. There is no second copy to drift.

The fix landed so far funnels every cursor *draw* through one matrix-indexed
function (`cursor_draw_rect`) and makes the column *resolver* total. The ideal
end-state deletes the stored geometry entirely: the cursor becomes a `SlotId`
reference plus a `kind`, and all geometry is a pure function of the frame
matrix. The same principle retires the echo-area and glyph-atlas drift bugs.

---

## Part I — The problem

### I.1 The six representations of one cursor

| # | Representation | Where | What it stores |
|---|---|---|---|
| 1 | `CapturedCursorInfo` | layout emit (`engine.rs`) | column = **Text-area index**, x = `content_x` |
| 2 | `resolved_cursor` | `resolve_cursor_geometry` (`engine.rs` ~6362) | a rect derived from #1 |
| 3 | `PhysCursor` | `matrix_builder.rs` / `frame_glyphs.rs` | `col` **and** `slot_id.col` **and** `x,y,w,h` — three position fields that can disagree |
| 4 | `WindowCursorVisual` / `CursorItem` | `push_cursor` (`engine.rs` ~6372) | a *separate* per-window cursor, seeded from #2 |
| 5 | animation `CursorTarget` | `cursor_target_for_frame` (`frame_windows.rs`) | rebuilt from `PhysCursor.x` |
| 6 | draw-time rect | renderer (`glyphs.rs`) | re-derived again when drawing |

Six computations of "where is the cursor." Each is reachable independently; none
is canonically derived from the others.

### I.2 The five bugs = five disagreements

| Bug | Commit | Disagreement |
|---|---|---|
| **A** line-number gutter | `bb58b8d3e` | #1's `col` is a Text-area index; #6 reads it as a *materialize* index (gutter + text) → cursor lands `lnum_cols` cells left, in the gutter |
| **B** hidden `#+title:` prefix | `b57a29168` | point sits on invisible text → no glyph carries its charpos → the resolver fell back to the captured gutter column → a stray cursor in the gutter |
| **C** two cursors on a heading | `609c6c980` | #3 (`PhysCursor`) resolved past the gutter but #4 (`window_cursors`) kept the pre-resolution slot → the renderer's dedup (which requires byte-equal `slot_id`) failed → both drawn |
| **D** heading shows two boxes | `97eb4cd5b` | #6 drew the static box at the glyph x (238); #5 built the animation target from `PhysCursor.x` (grid 192) → static box + animated box = two |
| **E** blank line after title | `2f73639a0` + `32eafdc44` | the row has only gutter glyphs and an empty text area → the resolver returned `None` → the cursor kept captured `col=0` → gutter; and the empty-slot fallback x was the grid estimate, ~7px short of the text column |

Bugs A and B were the same resolver missing two cases (gutter accounting, then
hidden text). C and D were two *different stored copies* of the cursor each
holding a stale value. E was the resolver being non-total **and** the
grid-estimate fallback leaking through.

### I.3 Root causes

1. **Geometry is copied, not referenced.** A cursor is a copy of a glyph's
   position. The copy goes stale the instant the glyph moves — scaled font,
   bidi reorder, hidden text, gutter — because nothing keeps it tied to the
   glyph it came from.
2. **An approximation hides the flaw.** `x = col × avg_char_width` coincides
   with the true glyph x for monospace, so every bug was invisible until a
   proportional/heading font separated them. Approximations that are *usually*
   right defer the bug to the least-tested path.
3. **Coordinate-space proliferation.** Text-index column, materialize column,
   grid-x, glyph-x, `slot_id`, animation-target — with scattered, lossy
   conversions. The gutter is the canonical trap: `col` means a Text-area index
   on one side of a boundary and a materialize index on the other.
4. **Illegal states are representable.** `PhysCursor { col, slot_id.col, x, … }`
   lets the three position fields disagree. Nothing in the type system forbids
   it, so they diverge.
5. **The forcing function: the thread/serialization boundary.** GNU shares one
   glyph matrix by pointer; the cursor is an index into it. neomacs splits
   layout and render across threads and a GPU, so the matrix cannot be shared
   by pointer — and the expedient choice was to *copy geometry* across the
   boundary instead of *copying the reference*. That copy is where every drift
   enters. The GPU/animation layer then added still more copies (#5, #6) that
   GNU does not have.

---

## Part II — How GNU Emacs avoids it

GNU's redisplay has exactly one cursor authority, and the cursor is placed *by
reference to a glyph*, never by recomputation.

- **The glyph matrix is the single source of truth.** `struct glyph_matrix` →
  `struct glyph_row[]` → `struct glyph[]` (`dispextern.h`). One matrix per
  window. A glyph's pixel-x is not stored redundantly; it is the running sum of
  `pixel_width` along the row — the same walk the drawer performs.

- **`set_cursor_from_row` (`xdisp.c`) is the only place cursor position is
  decided, and it walks the very row that will be drawn.** It finds the glyph
  whose `charpos == PT` and records `w->phys_cursor` as `{hpos, vpos}` — an
  *index* into the matrix — plus the glyph's x and the row's y. Because it walks
  the actual produced glyphs:
  - Line-number glyphs (`NILP (glyph->object) && glyph->charpos < 0`) are walked
    past inherently — no separate gutter accounting to desync (this is bug A).
  - Hidden text: PT's glyph is simply absent; the "use the following glyph" rule
    lives right here, computed once (bug B).
  - End of line / blank row: the function still lands the cursor — at the row's
    `x` plus its used pixel widths. **It is total** (bug E).

- **One cursor per window: `w->phys_cursor` + `phys_cursor_type`.** There is no
  "visual cursor" separate from the "phys cursor." Selected vs non-selected only
  changes what `get_window_cursor_type` returns at *draw* time.
  `draw_phys_cursor_glyph` / `display_and_set_cursor` look up the glyph at
  `(vpos, hpos)` in the matrix and draw the cursor over it. The cursor
  *references* the matrix glyph; it never carries a copy (bugs C and D).

GNU's invariants, stated plainly:

> The cursor is an **index** into the finalized glyph matrix. It is resolved
> **once**, after the matrix is final. Geometry is **read from the indexed
> glyph at draw time**, never stored. There is **one cursor per window**; type
> is a draw-time decision. Placement walks **the same glyph sequence that
> renders**, so every per-glyph quirk (line numbers, hidden text, wide chars,
> bidi) is handled in one place, by construction.

---

## Part III — What is implemented (landed on `main`)

The fix funnels all cursor *geometry derivation* through one matrix-indexed
function and makes the column *resolver* total. It does not yet delete the
stored geometry or the dual storage (that is Part VI).

### III.1 Progress log

| Stage | Commit | Outcome |
|---|---|---|
| 1 | `7ae11107b` | `FrameGlyph::cell_rect` and `FrameGlyphBuffer::cursor_cell_rect` — resolve a slot to the occupying glyph's cell rect (`frame_glyphs.rs`). |
| 2 + 3 | `76ce58a24` | `FrameGlyphBuffer::cursor_draw_rect(slot_id, style, fallback)` becomes THE single placement function. The renderer's `cursor_glyph_slot_rect` (`glyphs.rs`) and the slide-animation target `cursor_target_for_frame` (`frame_windows.rs`) both delegate to it; the child-frame fallback (`frame_ingest.rs`) too. Supersedes the partial `cell_x` animation fix. |
| 4 | `20ca5f9e1` | The `window_cursors` draw loop (`glyphs.rs`) routes through `cursor_draw_rect` as well — non-selected windows' cursors slot-snap exactly like the selected one. Both draw paths now converge on one function. |
| 5 | `139d2b90b` | The gutter-walk + hidden-prefix logic is lifted out of `set_phys_cursor` into `resolve_cursor_visual_col` (`matrix_builder.rs`) — one named authority for "which column is point on," applied to both the phys cursor and the redundant window cursor. |
| E | `2f73639a0` | `resolve_cursor_visual_col` is made **total** over a matching window+row: when no glyph carries point's charpos (blank line, end of line) it returns `col_acc` (the first column past the gutter and text) instead of `None`. Fixes the blank-line-after-`#+title:` cursor landing in the gutter. |
| snap | `32eafdc44` | For an empty slot (no glyph), `cursor_draw_rect` snaps x to the right edge of the nearest preceding glyph in the row (`row_pen_x_before`) rather than the grid-estimate fallback — GNU's end-of-line rule. Verified live: blank-line cursor x 70 → 77, flush with the text cell. |

Predecessor single-representation fixes (the same bug-hunt): bug A `bb58b8d3e`,
bug B `b57a29168`, bug C `609c6c980`, bug D `97eb4cd5b`; plus the echo-area
grow-only fix `755cf3e2f` and the glyph-atlas allocator overlap fix `48e1254c8`
(both instances of the same "stored derived state drifts" disease, Part V).

### III.2 Current code map

| Concern | Function | File |
|---|---|---|
| slot → cell rect | `FrameGlyph::cell_rect`, `FrameGlyphBuffer::slot_glyph` | `neomacs-display-protocol/src/frame_glyphs.rs` |
| **slot → cursor rect (single source)** | `FrameGlyphBuffer::cursor_draw_rect` | same |
| empty-slot x | `FrameGlyphBuffer::row_pen_x_before` | same |
| point → column (single resolver, total) | `GlyphMatrixBuilder::resolve_cursor_visual_col` | `neomacs-layout-engine/src/matrix_builder.rs` |
| stores resolved cursor + syncs the redundant copy | `GlyphMatrixBuilder::set_phys_cursor` | same |
| static draw delegates here | `cursor_glyph_slot_rect`, `cursor_render_rect` | `neomacs-renderer-wgpu/src/renderer/glyphs.rs` |
| per-window cursor draw loop | (in `render` method, ~`glyphs.rs:1929`) | same |
| animation target delegates here | `cursor_target_for_frame` | `neomacs-display-runtime/src/render_thread/frame_windows.rs` |
| bidi remap (phys + window, consistent) | Steps C / 7 | `neomacs-layout-engine/src/bidi_layout.rs` |

After these changes, every place that turns a cursor into pixels calls
`cursor_draw_rect`, and every place that turns point into a column calls
`resolve_cursor_visual_col`. Two funnels. The remaining smell is that the cursor
*still carries geometry* across the boundary (`PhysCursor` fields, the separate
`window_cursors` Vec) — only used as a fallback now, but still able, in
principle, to disagree. Part VI deletes it.

---

## Part IV — The ideal long-term design

### IV.1 North star (one rule)

> **Derived state is a pure function of a single source, evaluated on demand.
> Nothing downstream stores geometry; it stores references into the one source
> and derives pixels at the point of use.**

GNU gets this for free because its source — the glyph matrix — is shared by
pointer. neomacs must reproduce the invariant *across a serialization boundary*.
The move that makes it work: **send the reference, not the copy.**

### IV.2 The ideal data model

One immutable snapshot per frame; everything else is a reference into it.

```rust
// THE single source of truth. Immutable once layout finishes; crosses the
// thread boundary as an Arc. Glyphs carry their own pixel geometry.
struct FrameMatrix {
    windows: Vec<WindowMatrix>,        // one per window
    // faces, stipples, frame identity, background, …
}

struct WindowMatrix {
    rows: Vec<GlyphRow>,               // glyphs with attached x / y / width / …
    cursor: Option<WindowCursor>,      // ← a reference, resolved at draw time
    // selection: Option<SlotRange>, overlays: …  (same pattern)
}

// The cursor is a reference + intent. No geometry fields, so the
// three-fields-that-can-disagree bug class is unrepresentable.
struct WindowCursor { at: SlotId, kind: CursorKind }  // Filled | Hollow | Bar | Hbar
```

What is **gone** relative to today: `PhysCursor`'s `col` / `x` / `y` / `width` /
`height`; the separate `WindowCursorVisual` / `window_cursors` Vec; the
`phys_cursor`-vs-per-window split; the animation `CursorTarget` as stored state.
One cursor per window; selected-vs-not is just which `kind` layout chose. That
deletion retires bugs C and D *by construction* — there is nothing left to
desync.

### IV.3 The ideal pipeline — two total pure functions, one snapshot

```
model (buffers / windows)
   │  point_to_slot(row, point) -> SlotId          ← layout side, TOTAL
   ▼
FrameMatrix   (immutable; the only thing crossing the thread boundary)
   │
   ▼
   slot_to_rect(matrix, slot, kind) -> Rect         ← render side, TOTAL, pure
   ▼
GPU / TTY draw commands
```

- **Layout owns "point → slot."** Computed once while building the row, exactly
  like GNU `set_cursor_from_row`. This is today's `resolve_cursor_visual_col`,
  made total. Its result is stored as `cursor.at`. The render side never
  re-decides which slot.
- **Render owns "slot → pixels."** This is today's `cursor_draw_rect`. A pure
  function of the matrix. Total — empty slots resolve from row geometry
  (`row_pen_x_before`), never a grid guess.
- **The cursor between them is just a `SlotId`.** That one value is the entire
  interface.

Animation, effects, and the glyph atlas become **pure derivations the render
thread owns and never sends back**:

```rust
// Animation is render-local visual state, not a source of truth. Both endpoints
// come from the matrix; "where the cursor is" is never stored here.
let target = slot_to_rect(matrix, win.cursor.at, win.cursor.kind);
self.cursor_anim = interpolate(self.cursor_anim, target, dt);
```

### IV.4 This is GNU's invariants, re-cast for a threaded/GPU engine

| GNU (shared matrix, one thread) | neomacs ideal (snapshot + thread split) |
|---|---|
| `w->phys_cursor` = `(hpos, vpos)` index | `WindowCursor.at` = `SlotId` |
| geometry read from matrix at draw | `slot_to_rect(matrix, slot)` at draw |
| one cursor/window, type at draw | one `WindowCursor`, `kind` at layout |
| `set_cursor_from_row` walks the drawn row, total | `point_to_slot`, total |

Same guarantees. The only addition is "the matrix is an immutable value passed
by `Arc`" to survive the boundary.

### IV.5 Retained vs immediate — pick one coherently

Today neomacs has the *worst of both*: it rebuilds the GPU frame each time
(immediate) **and** keeps retained derived caches (`window_cursors`, animation
targets, the atlas) that need manual reconciliation. The ideal picks one:

- **Immediate-from-snapshot (recommended).** The `FrameMatrix` is the only
  retained value; everything else is re-derived each frame from it. Simplest;
  kills the reconciliation bug class outright. Cheap for terminal-sized frames
  on a modern GPU.
- **Retained-with-diff (only if a profiler demands it).** Make `FrameMatrix` a
  persistent/immutable structure with cheap structural diff, so the GPU updates
  incrementally by diffing matrices — *not* by hand-maintained caches. This is
  GNU's `current` vs `desired` matrix diff (`update_window`), done with an
  immutable value.

Either way the matrix is the one source; the difference is only whether the
renderer recomputes or diffs. Do not mix the two.

---

## Part V — The same disease elsewhere

The north-star rule dissolves the other drift bugs from the same bug-hunt:

- **Echo-area** (`755cf3e2f`). The minibuffer height was stored in `Frame::new`,
  recomputed in `window_text_area_bounds`, and reconciled by a grow-only resize
  that never shrank an over-allocated idle echo area. Ideal: height is a pure
  function of the minibuffer content + frame metrics, computed once per layout —
  no stored height to fall out of sync.
- **Glyph atlas** (`48e1254c8`). A glyph slot was allocated in one place and the
  shelf cursor advanced in another; on a wrapped shelf the two disagreed and two
  glyphs overlapped in the texture, rendering as each other. Ideal: the atlas is
  a *pure cache* of `glyph_key → texture_region` with **one writer**, keyed by
  and invalidated from the glyph identity, never read as a source of truth.

The unifying statement:

> **Derived state that is stored rather than computed will drift from its
> source.** If you must cache it for performance, there is exactly one writer,
> the cache is keyed by and invalidated from the source, and it is never read as
> if it were the source.

---

## Part VI — Migration path

Ordered, lowest-risk first. Steps 1–2 are landed and shipping.

1. **[done]** Funnel all geometry derivation through one `cursor_draw_rect`
   (`76ce58a24`, `20ca5f9e1`).
2. **[done]** Make `point_to_slot` / `resolve_cursor_visual_col` total
   (`2f73639a0`); snap empty-slot x from row geometry (`32eafdc44`).
3. **[next — highest leverage]** Collapse `phys_cursor` + `window_cursors` into
   one `Vec<WindowCursor> { at, kind, color }`, with `kind` (filled / hollow)
   chosen by selected-ness at layout time. Update the three consumers to iterate
   it:
   - GPU: `neomacs-renderer-wgpu/src/renderer/glyphs.rs` (the two cursor draw
     loops).
   - TTY: `neomacs-display-runtime/src/backend/tty/mod.rs:604`.
   - bidi: `neomacs-layout-engine/src/bidi_layout.rs` (already remaps phys and
     window cursors consistently — Steps C / 7 — so this collapses to one list).
   This is the one cross-backend refactor. It **deletes the sync loop and the
   dedup** (`window_cursor_visual_matches_phys`), and makes "two representations
   of one cursor" *impossible*, not merely currently-synced.
4. **Delete the geometry fields** from the cursor type (leave `at` + `kind`).
   `cursor_draw_rect` becomes the sole geometry source; the fallback rect
   disappears once every cursor has a resolvable slot and empty slots resolve via
   `row_pen_x_before` / a proper empty-cell position from the matrix.
5. **Generalize the reference pattern** to selection / region highlight,
   overlays, and mouse highlight — anything that currently carries geometry
   across the boundary.
6. **[long term]** Make `FrameMatrix` the single immutable snapshot; move
   animation and the atlas to pure derivations the renderer owns; add structural
   diffing only where profiling demands it (Part IV.5).

After step 3 the cursor is structurally identical to GNU's `w->phys_cursor`, and
the same template (steps 4–6) retires the rest of the display layer's drift bugs.

---

## Part VII — Invariants to enforce going forward

A checklist for any future cursor / display change:

1. **`slot_id` is the only cursor state that crosses layout→render.** If a patch
   adds an `x` / `y` / `col` next to a `slot_id`, it is re-introducing drift.
2. **Geometry is a pure function of the matrix.** `cursor_draw_rect(matrix,
   slot)` is the only place a cursor becomes pixels. No second derivation.
3. **The resolver is total.** `point_to_slot` over a matching window+row always
   returns a column; `None` means only wrong-window / row-out-of-range. "No
   cursor here" is not a legal answer for a row containing point.
4. **Never compute a cursor pixel from `col × average_width`.** Positions come
   from the glyph; the empty-cell fallback is the row's actual pen-x at the slot,
   not a grid estimate.
5. **One cursor per window.** Selected-vs-not is a `kind` chosen at layout time,
   not a second stored cursor.
6. **One column space.** The Text-area index must not escape the emitter; emit
   the materialize column (or the `SlotId`) directly, so `col` means the same
   thing on both sides of every boundary.
