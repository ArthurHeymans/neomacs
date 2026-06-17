# Display Pipeline Refactor Handoff

Date: 2026-06-13 (refreshed 2026-06-17)
Branch: `main`
HEAD at refresh: `e58ae8082` (started from `77c140125`; this session landed 3
path-deletion slices + an exhaustive remaining-gap map — see "Landed" and
"Prioritized Execution Plan" below)

> Refresh note (2026-06-17): the original 2026-06-13 handoff stopped at
> `798f6b312 refactor: group buffer advance context`. **476 commits** landed
> between then and `77c140125`, so the per-slice bullet list below was rewritten
> to describe where the pipeline actually stands now rather than re-listing every
> wrapper commit. The "Why", "Desired Pipeline", "Important Invariants", GNU
> reference, manual-repro, and verification sections were still accurate and are
> unchanged.

## Goal

Continue the long-term display pipeline refactor until Neomacs has one typed source-to-row pipeline for main buffer text, overlay/display strings, mode/header/tab-line, tab-bar, minibuffer, echo area, and detached child-frame/posframe rows.

The goal is not complete. The recent work landed several safe slices, but the pipeline is still split between older buffer-walker code and newer request/source-based row rendering.

### Strategic guidance (agreed 2026-06-17)

The destination above is correct and matches GNU's real structure: source-specific
"get next element" via `it->method` dispatch (`GET_FROM_BUFFER` / `GET_FROM_STRING`
/ `GET_FROM_STRETCH` / `GET_FROM_IMAGE` / …) feeding a shared `produce_glyphs`
keyed on `it->what`. That is exactly `DisplayItemSource` → one row builder. Keep
the direction. Three refinements to *how* it's pursued:

- **Relax the "pure typed boundary" ideal.** "Below `DisplayItemSource` consume
  only typed display items, no peeking at source semantics" is stricter than GNU,
  which shares mutable `it` state (bidi level, cursor pos, hpos/x, wrap/continuation)
  across both layers — because wrap, cursor, and bidi inherently need to see across
  the boundary. Plan for the boundary to carry a small *typed* shared-progress
  channel (x/col, bidi level, wrap state), not zero shared state. The current
  `DisplayItem -> BufferTextDecodedSourceEvent` round-trip and the unremovable
  resolved-advance-for-wrap are that channel asking to be made explicit — not
  failures to design away.
- **Judge progress by paths deleted, not types added.** Value is realized when an
  old path is *removed* and a bug class becomes structurally impossible, not when
  another typed wrapper lands. After the 476-commit batch, no production main-buffer
  path is deleted yet; `BufferTextSourceCursor` is still dead-code behind a
  test-only toggle, and `display_row_append.rs` has grown to ~10k lines. Close one
  loop end-to-end before widening, and give that module a complexity budget so
  "unified" means smaller, not relocated.
- **Slice 5 (one row lifecycle) is the real prize.** The listed bugs are *lifecycle*
  bugs (inconsistent bidi/finalization timing). Shared production + one finalization
  boundary fixes them. Source unification (Slice 4) is necessary plumbing but fixes
  no listed bug on its own. Starting with Slice 2 is recommended: it is the cheapest
  way to confront the cross-boundary-state question before the big cursor swap.

## Current Status

Most recent display-pipeline commits on `main` (newest first):

- `77c140125 refactor: typed buffer source adapter and parity tests`
- `a39aca1d9 refactor: request display replacement append plan items`
- `f906fd14f refactor: move replacement cursor policy source`
- `d82458511 refactor: resolve display replacements from request`
- `3225f4b4e refactor: build display replacement source items`
- `03095e21e refactor: move display property replacement source item`

Baseline at refresh (`77c140125`):

```bash
cargo nextest run -p neomacs-layout-engine
```

passed with **1263 tests** (up from 1061 at the original handoff). Do not use
`cargo test`; the project owner explicitly requested `cargo nextest`.

### What landed in the 476 commits since the original handoff

Grouped by theme instead of per-commit, because almost every commit is one
containment/typing step in a long strangler-fig sequence:

- **The giant buffer redisplay loop left `engine.rs`.** `engine.rs` is now
  ~1790 lines; the per-window buffer walk is a typed state machine in the new
  `display_buffer_text_walk.rs` (~3396 lines) built from dozens of
  `BufferTextWindow*` request/state structs. `engine.rs::layout_window_rust`
  now delegates into that module.
- **A real typed buffer source exists but is not yet the production path.**
  `display_buffer_text_source.rs` adds `BufferTextSourceCursor`, a
  `DisplayItemSource` that walks plain buffer text + face/display-property
  boundaries and emits `DisplayItem`s. It is wired behind
  `LayoutEngine::use_typed_buffer_source` (default `false`, `#[cfg(test)]`
  setter only). When enabled, `display_buffer_text_walk.rs::consume_source_event`
  pulls one `DisplayItem` from the cursor and **lowers it back into the legacy
  `BufferTextDecodedSourceEvent`** the old loop consumes. So today it is a
  *parity shim* proving the cursor yields equivalent events — not a replacement
  of the downstream walker. Parity is asserted in `display_source_test.rs`.
- **Display replacements moved to the source layer.** Display-property
  replacement strings, media, stretches/spaces, mapped text, and placeholders
  are now built as typed source items and resolved from request/source context
  (`display_source.rs`, `display_source_resolver.rs`), instead of `engine.rs`
  wiring raw replacement append requests.
- **Chrome, line numbers, right borders, echo markers, and mock rows render
  through item sources / row requests.** Legacy direct row-glyph installers and
  the legacy line-number row writer were removed; status/chrome rows install as
  prebuilt rows through the row lifecycle.
- **Matrix builder boundary narrowed.** `matrix_builder.rs` API was internalized
  and row-writing pushed out; row glyph surgery now lives in the row builder.
- **Width/advance measurement centralized (but not unified).** ASCII buffer
  text and display items are measured through the row renderer; buffer advances
  are resolved from typed requests. The walker still computes a resolved advance
  for wrap/cursor decisions, so the width path is *not* fully unified yet (this
  is still the open "Slice 2" work below).

Out of scope but on the same branch in this range (do not let these confuse a
`git log` of the display work): Cranelift JIT (Milestone D + Tier-2 phases),
perf work (mimalloc global allocator, FxHash maps), `fix(#131)` multibyte buffer
decode, and `fix(#132)` subprocess isolation / wait-strategy refactor.

## Landed (2026-06-17 session): exhaustive map + first 3 path-deletions

A 12-agent mapping workflow enumerated **45 gaps across 10 subsystems** and
deduped them into a prioritized plan of 10 path-deletion slices, with an
adversarial completeness critic (see **Prioritized Execution Plan** below). The
map corrected two wrong assumptions *before* any code was written: complex runs
are *already* single-shaped (the double-shape was the **Natural** path), and the
`FaceColumns` advance is the TTY/no-font-metrics fallback, not removable
duplication.

Three slices then landed (HEAD `e58ae8082`; net **−1194 lines**; full
layout-engine suite green at 1217), each adversarially reviewed (verdict: SHIP,
zero blockers):

- **`4c0df03fd` — delete dead `bidi_layout.rs`** (the second bidi-reorder, over
  the retired `FrameGlyphBuffer`/`FrameGlyph` representation). Zero production
  callers; the only live row reorder is `glyph_row_writer::reorder_row_bidi` over
  `GlyphRow`. −1299 lines; one production bidi reorder remains. The deleted
  per-glyph `row_max_ascent` baseline logic was dead code on a defunct
  representation — the live `GlyphRow` uses GNU's single-per-row `ascent` model,
  so no capability was lost. (`bidi_layout.rs` no longer exists; ignore older
  plan/audit docs that still cite it by path.)
- **`685a288ff` — delete vestigial `DisplayOrigin::Posframe`** (`#[allow(dead_code)]`,
  never constructed; production posframes go through display-runtime's
  `ChildFrameManager`). NOTE: the mapped slice also proposed deleting the mock
  child-frame loop in `engine.rs::layout_mock_frame`, but that was **kept** —
  `mock-display.rs:690` exercises it, so it is live for the dev tool, not dead.
- **`4e95cf933` + `e58ae8082` — shaped-run cache in `FontMetricsService`**. Kills
  the Natural-path measure/render double-shape: every non-ASCII simple char and
  natural run was shaped twice (`measure_natural_width_to_text_row` +
  `NaturalDisplayRowAppendRenderPolicy`, both via `display_row.rs::text_run_advances_px
  → shape_run`, with no shaped-run cache). Now cached by integer-centipx face
  identity + run text, cleared by `clear_caches` (same font-change posture as the
  existing advance caches), bounded at 8192. Tests assert dedup, per-face
  distinctness across all four key fields, `clear_caches` invalidation, and the
  overflow branch.

## Why This Refactor Exists

The bugs that triggered this work were not isolated rendering accidents. They all came from the same architectural split:

- main buffer rendering had a rich ad hoc walker in `engine.rs`;
- chrome/string rows had separate simpler paths;
- posframe/minibuffer/tab-line/tab-bar rows reached glyph output with weaker face, width, overlay, display-property, and row lifecycle semantics;
- several paths carried already-derived glyph data instead of typed display intent.

That split caused bugs such as:

- tab-bar/tab-line/posframe text spacing and overlap problems;
- background face leakage from minibuffer prompt into completion candidates;
- posframe width changing while moving selection;
- overlay before/after string behavior diverging from GNU Emacs;
- row finalization and bidi normalization happening at inconsistent times.

The long-term design is to preserve typed display origin and display items until a single row builder handles shaping, width, faces, display properties, clipping, bidi, and row finalization.

## Desired Pipeline

Target shape:

```text
Source-specific side
--------------------
BufferTextSourceCursor
LispStringSourceCursor
Overlay/display string sources
Chrome row sources
Detached/posframe row sources
        |
        v
DisplayItemSource / DisplayItem stream
        |
        v
DisplayRowSourceRenderRequest
        |
        v
DisplayRowRenderer / DisplayRowBuilder
        |
        v
RenderedDisplayRow + metadata
        |
        v
Window or frame row installer
        |
        v
FrameDisplayState -> renderer
```

Important boundary:

- Code above `DisplayItemSource` may know about buffers, overlays, Lisp `Value`, text properties, window state, and GNU-specific source rules.
- Code below `DisplayItemSource` should consume typed Rust display items and row requests. It should not evaluate Lisp or infer source semantics from glyphs.

## Key Modules

- `neomacs-layout-engine/src/display_item.rs`
  - Typed display item language: `DisplayItem`, `DisplayItemKind`, source spans, text runs, media/stretch items, face refs.

- `neomacs-layout-engine/src/display_source.rs`
  - Source cursor abstraction and shared source types: `DisplayItemSource`, `DisplaySourceContext`, `LispStringSourceCursor`, `BufferTextSourceChar`, the buffer advance request/classification types, and display-replacement source resolution.

- `neomacs-layout-engine/src/display_buffer_text_source.rs` (new since handoff)
  - `BufferTextSourceCursor`: the typed `DisplayItemSource` for plain buffer text + face/display-property boundaries — the strangler-fig replacement for the old raw decoder. Also `BufferTextWindowSource*` (window-start resolution + text read) and `BufferTextSourceEventCursor` (the raw decoder still used in production). The cursor is gated behind `use_typed_buffer_source` and is currently a parity shim only.

- `neomacs-layout-engine/src/display_buffer_text_walk.rs` (new since handoff)
  - The per-window buffer redisplay walk as a typed state machine (`BufferTextWindow*` request/state structs). This is where the giant `engine.rs` loop now lives, and where `consume_source_event` chooses between the raw decoder and the typed cursor.

- `neomacs-layout-engine/src/display_row.rs`
  - Current row rendering center: `DisplayRowSourceRenderRequest`, private `LoweredDisplayRowSpec`, `DisplayRowRenderContext`, `DisplayRowRenderer`, fragment rendering, row finalization helpers.

- `neomacs-layout-engine/src/display_row_source_render.rs` (new since handoff)
  - Source → row render-request execution (`render_*_with_source_state` helpers) used by the walk module.

- `neomacs-layout-engine/src/display_row_walk_state.rs` (new since handoff)
  - Per-row overflow/wrap decision state (`BufferTextRowOverflowDecision`, etc.).

- `neomacs-layout-engine/src/display_row_append.rs`
  - Bridge for appending rendered fragments into current text rows and emitting output. **Now the largest module (~10k lines)** and still the biggest builder-coupling debt area after `engine.rs` shrank.

- `neomacs-layout-engine/src/display_origin.rs`
  - Typed origins for face/source policy decisions.

- `neomacs-layout-engine/src/display_face_policy.rs`
  - Base face policy. This should grow into the only place that decides how each origin gets its base face.

- `neomacs-layout-engine/src/display_source_resolver.rs`
  - Resolves display-source items and faces. Keep semantic source resolution here rather than in row glyph output.

- `neomacs-layout-engine/src/display_row_builder.rs`
  - Row geometry/progress/measurement concepts. Long-term, this should own more of the shared row lifecycle.

- `neomacs-layout-engine/src/matrix_builder.rs`
  - Matrix installation/bookkeeping. Long-term, this should not encode display semantics.

- `neomacs-layout-engine/src/engine.rs`
  - Frame-level orchestration (`layout_frame_rust`, `layout_window_rust`), `LayoutEngine` state, font-metrics setup, and the `use_typed_buffer_source` toggle. The large per-window buffer walk it used to host now lives in `display_buffer_text_walk.rs`; the remaining debt is `display_row_append.rs` and finishing the typed buffer source migration.

## Current Split To Remove

The split is no longer "two whole rendering paths" — chrome/string rows and the
main buffer row append now share `DisplayRowSourceRenderRequest` /
`display_row_append.rs`. What remains split is **how main-buffer text is
sourced**:

```text
Production main-buffer path (use_typed_buffer_source = false)
------------------------------------------------------------
display_buffer_text_walk.rs::consume_source_event
  -> BufferTextSourceEventCursor (raw UTF-8 decoder)
  -> BufferTextDecodedSourceEvent
  -> shared append/render (display_row_append.rs -> GlyphRow)

Typed parity path (use_typed_buffer_source = true, tests only)
--------------------------------------------------------------
display_buffer_text_walk.rs::consume_source_event
  -> BufferTextSourceCursor (DisplayItemSource)
  -> DisplayItem
  -> lowered BACK to BufferTextDecodedSourceEvent   <-- shim, not yet a replacement
  -> shared append/render (display_row_append.rs -> GlyphRow)
```

The goal is to delete the raw-decoder path and let `BufferTextSourceCursor`'s
`DisplayItem`s flow straight into the shared renderer without the
`DisplayItem -> BufferTextDecodedSourceEvent` round-trip. That round-trip is the
load-bearing piece of the next slice.

The old caller-facing `DisplayRowSpec` name is gone from source. The lowered row request is now the private `LoweredDisplayRowSpec` inside `display_row.rs`; new caller coverage should use `DisplayRowSourceRenderRequest`, `DisplayRowSourceAppendRequest`, or a higher-level row request.

## Important Invariants

Preserve these while refactoring:

- Use `cargo nextest`, never `cargo test`.
- Keep commits as cohesive architectural batches. Avoid tiny wrapper-only commits when related pipeline containment work can be verified together.
- Do not hide layout bugs with frontend workarounds.
- Do not let renderer code infer row source semantics from pixels or glyphs.
- Keep row ownership explicit:
  - tab-bar is frame chrome;
  - tab-line/header-line/mode-line are window-local chrome;
  - minibuffer/echo are normal window rows unless explicitly detached;
  - posframe/child-frame content belongs to that frame/window, not the parent minibuffer prompt.
- Use enums for semantic choices instead of bool flags.
- Keep `Value` and Lisp-specific parsing out of pure row building wherever possible.
- Row finalization, bidi normalization, clipping, and face insertion must happen at one predictable boundary.

## Suggested Next Slices

### Slice 1: Contain `DisplayRowSpec` (done)

Goal: make `DisplayRowSpec` an internal lowering type for `DisplayRowRenderer`.

Status: complete in source. The lowered type is now private `LoweredDisplayRowSpec`; keep it that way.

Steps:

1. Search:

   ```bash
   rg -n "DisplayRowSpec|display_row_spec\\(|display_row_spec_" neomacs-layout-engine/src
   ```

2. For every test/caller that can use `DisplayRowSourceRenderRequest`, migrate it.

3. Keep direct lowered-spec use only in renderer internals.

4. Focused verification:

   ```bash
   cargo nextest run -p neomacs-layout-engine display_row display_row_append
   cargo nextest run -p neomacs-layout-engine
   ```

### Slice 2: Retire Old Width Paths

Goal: make all row text use one measurement/advance path.

Look for:

```bash
rg -n "font_char_width|pixel_width|from_char_advances|text_char_advance_px_at_position_with_measurement" neomacs-layout-engine/src
```

Current issue: some paths still derive placement from face-width assumptions or precomputed per-character advances before row rendering. The row renderer should own measured text advance for all row kinds.

Do not remove a path until tests cover:

- main buffer ASCII;
- wide chars;
- grapheme clusters;
- tab-line/tab-bar;
- mode-line;
- minibuffer/echo;
- posframe/child-frame completion rows.

### Slice 3: Normalize Face Source Policy

Goal: prevent face leakage across prompt/candidate, overlay, display-string, and chrome boundaries.

Useful tests:

- minibuffer prompt background does not leak into completion candidates;
- overlay `before-string` and `after-string` use GNU-compatible anchor face policy;
- tab-line/tab-bar propertized strings preserve their own faces;
- detached/child-frame rows resolve faces independently of parent window rows.

Keep policy in:

- `display_origin.rs`
- `display_face_policy.rs`
- `display_source_resolver.rs`

Avoid putting source-specific face rules in `DisplayRowRenderer`.

### Slice 4: Move More Buffer Walker Output To `DisplayItemSource` (in progress)

Goal: shrink the buffer walk and let it consume typed `DisplayItem`s.

Status: the buffer loop already moved out of `engine.rs` into
`display_buffer_text_walk.rs`, and step 1 below is done — `BufferTextSourceCursor`
exists with parity tests. What remains is to stop lowering its items back to the
legacy `BufferTextDecodedSourceEvent`.

Approach:

1. ~~Extract a narrow source cursor for plain buffer text with resolved face refs.~~ Done: `BufferTextSourceCursor` in `display_buffer_text_source.rs`.
2. **Next: remove the parity round-trip.** Have `consume_source_event` (or its caller) consume `DisplayItem`s directly instead of translating `DisplayItem -> BufferTextDecodedSourceEvent`. Do this for plain text first; keep overlays/display-properties/cursor metadata on the legacy path until each is covered.
3. Keep matrix installation unchanged.
4. Add regression tests around cursor position, row end, clipping, and bidi as each segment type moves over.
5. Once a segment type renders identically from the typed source, flip and eventually delete the raw-decoder branch for it.

Do not try to migrate overlays, display properties, and cursor metadata all in one commit. The `use_typed_buffer_source` toggle is the seam: expand what it covers, keep both paths green via parity tests, then retire the legacy path.

### Slice 5: Unified Row Lifecycle

Goal: one lifecycle for row construction:

```text
begin row
consume display items
measure/shape/append slots
clip or exhaust source
finalize bidi
normalize external row
install row
emit output
```

Today these steps are still spread across `DisplayRowRenderer`, `GlyphMatrixBuilder`, `display_row_append.rs`, and `engine.rs`.

## GNU Emacs Reference Points

Use `/home/exec/Projects/github.com/emacs-mirror/emacs` as the local GNU source reference.

Useful files/functions:

- `src/xdisp.c`
  - redisplay iterator model;
  - display string handling;
  - glyph production;
  - overlay string loading;
  - mode-line/tab-line style string display.

GNU's C code is not a Rust abstraction to copy directly. The useful lesson is the conceptual split:

- source iteration can be source-specific;
- glyph production should be shared;
- display strings should not use a weaker display language than buffer text.

## Manual Repro Context

The user often validates with personal config and `/tmp/debug.txt`, sometimes with:

```bash
NEOMACS_DUMP_FRAME_GLYPHS=1
```

Recent visual areas to keep checking:

- `M-x`, type one char, press `TAB`: minibuffer should not spam duplicate visible "Making completion list..." messages.
- Vertico posframe:
  - no text overlap;
  - candidate spacing stable;
  - candidate row backgrounds not polluted by prompt face;
  - posframe width does not shrink while moving selection.
- Tab-bar/tab-line:
  - height should not grow on repeated `j`/`k`;
  - text spacing should match main row measurements.
- Menu-bar:
  - button click opens correct menu content;
  - popup anchored below clicked button;
  - hover opens submenus without selecting commands.

## Recommended Verification

For a narrow slice:

```bash
cargo nextest run -p neomacs-layout-engine <focused-test-filter>
cargo fmt --check
git diff --check
cargo check -p neomacs-layout-engine
```

Before pushing a display pipeline slice:

```bash
cargo nextest run -p neomacs-layout-engine
```

If touching cross-crate display protocol/runtime code, also run the relevant package:

```bash
cargo nextest run -p neomacs-display-protocol
cargo nextest run -p neomacs-display-runtime
```

Expect some unrelated warnings from `neovm-core` during `cargo check`; do not mix warning cleanup into display refactor commits unless directly required.

## Prioritized Execution Plan (mapped 2026-06-17)

This supersedes the conceptual "Suggested Next Slices" above as the authoritative
ordered worklist. It is the dedup+prioritized output of the 10-subsystem gap map,
ordered by (deletion value) × (inverse risk) × (dependency readiness): free
dead/mock deletions first, then the local correctness dedup, then the lifecycle
unification, then the typed-source keystone, then the convergences that depend on
it. **Lead with the free wins; do NOT relocate `display_row_append.rs` or merge
overlay/echo before the lifecycle is unified — relocating first only moves
duplication.**

Status legend: ✅ done · ⬜ remaining.

1. ✅ **Delete dead `bidi_layout.rs`** (`4c0df03fd`). Risk low. Independent.
2. ✅ **Delete `DisplayOrigin::Posframe`** (`685a288ff`). Risk low. Independent.
   (Mock child-frame loop deliberately kept — see Landed note.)
3. ✅ **Shaped-run cache** (`4e95cf933`, `e58ae8082`). Risk medium. Independent.
4. ✅ **Add characterization/parity tests** (`e184af092`). `rtl_text_and_chrome_rows_reorder_identically` (matrix_builder_test) + `child_frame_resolves_faces_and_width_independently_from_parent` (engine_test); both pass on HEAD.
   Deps: 1. The two intended tests: (a) child-frame face/width independence;
   (b) **cross-row-kind RTL bidi parity** — identical RTL text as a buffer `Text`
   row vs a `ModeLine`/`Minibuffer` chrome row. CAVEAT from the critique: (b) is a
   **forward-spec, not a characterization** — buffer rows reorder at
   `EndIncremental` (cursor remapped) while chrome reorders earlier with
   `phys_cursor=None`, so it may *not* pass on HEAD. Scope (b) to glyph order /
   `reversed_p` (identical today); the cursor-column axis is what slice 5 fixes.
   The mixed-ascent baseline test the critique wanted is **not needed** — the
   review confirmed the live `GlyphRow` single-per-row ascent already covers it.
5. ✅ **Collapse the `EndPrebuilt`/`EndIncremental` bidi+install fork into ONE
   row-end boundary** (core landed `6d5aadf9d`). `reorder_current_row_bidi` is now
   the single bidi finalizer for all row kinds; the bidi/finalization-timing bug
   class is structurally fixed. `normalize_external_row` no longer reorders (render
   produces logical order); `PrebuiltDisplayRowInstall` uses `EndIncremental`;
   the frame tab-bar reorders its side-vector clone; `EndPrebuilt` deleted.
   Validated: 1219 layout-engine + 1431/1435 cross-crate green (4 pre-existing tty
   failures unrelated). The scout's verdict simplified it: threading `phys_cursor`
   through chrome is NOT needed (point never lives in chrome; the active minibuffer
   already goes through the buffer walk), and nearly all pre-install geometry
   readers are vertical/count (reorder-independent). **One GNU-correct behavior
   delta** remains untested: the RTL echo continuation-marker side (markers now
   participate in paragraph direction) — recommend an RTL-echo characterization
   test + GUI check. **Optional remaining containment** (not needed for the bug-class
   fix): delete `PrebuiltDisplayRowInstall`/`PrebuiltCurrent` by having chrome
   append directly into the current row instead of copy-prebuilt-glyphs-then-reorder.
   _Original framing kept for reference:_ *The real "one row lifecycle" prize* and
   the structural fix for the finalization/bidi-timing bug class. Critique
   prerequisites (now resolved by the scout): enumerate every pre-install reader of
   chrome row geometry; resolve the tab-bar measure-then-relayout vs
   append-into-live-matrix conflict (height reader is order-independent, so safe).
   **PREREQUISITE for slice 6 (new, from the critique — schedule BEFORE 6):**
   **Express walk-state concerns in the typed source / `DisplayItemKind`.** The
   typed `BufferTextSourceCursor` cannot yet model hscroll-skip, invisible-text +
   selective-display ellipsis, line-number margins, tab classification,
   nobreak/NBSP policy, or word-wrap break candidates — none of which slice 5
   addresses. Each needs a typed-vs-legacy parity test (matrix_rows AND faces AND
   cursor col) behind `use_typed_buffer_source=false`. Without this, slice 6's
   `None`-fallthrough resync never reaches zero and the legacy path is undeletable.
6. ⬜ **Make `BufferTextSourceCursor` the production path; delete the legacy
   decoder + lowering shim.** Risk **high**, keystone. Deps: 5, 3, and the
   prerequisite above. Extend the existing typed-vs-raw parity harness
   (`engine_test.rs:1954/2000`) to compare **faces** (it only compares positions
   today — that gap is the overlay-face-regression guard) and to cover tabs, NBSP,
   glyphless+cluster-tails, multibyte, selective display, hscroll, invisible text,
   word-wrap, and display-property replacements. Only flip the default and delete
   `BufferTextDecodedSourceEvent` / `BufferTextSourceEventCursor` / the shim after
   all parity cases pass.
7. ⬜ **Overlay before/after-strings as `DisplayItem`s on the source stack;
   delete the side-channel render loop + third lisp-string lifecycle.** Risk high.
   Deps: 5, 6. Also replace the dual before/after sort with one interleaved sort
   mirroring GNU `compare_overlay_entries`.
8. ⬜ **Echo / inactive-minibuffer through the buffer walk over the echo-area
   buffer; delete the parallel echo loop, marker duplication, and resize
   estimator.** Risk high. Deps: 5, 6. Mirrors GNU `display_echo_area_1`.
   Critique prerequisite: make the first layout pass measure **unclamped** height
   (GNU `move_it_to` semantics) so deleting the pre-layout newline estimator
   doesn't deadlock the `visible_max_rows` clamp against the height it determines.
9. ⬜ **Relocate non-append clusters out of `display_row_append.rs` (~10k lines).**
   Risk medium. Deps: 5, 6 (so relocated clusters call the unified lifecycle, not
   just move duplication). The right-edge-marker / right-border move is
   independent + low-risk and **can be pulled forward anytime**.
10. ⬜ **Collapse the two-layer append-operation indirection + unify
    decoration-face resolution** (merge-over-`DEFAULT_FACE_ID`, GNU-style;
    retire `FaceResolver::next_dynamic_id`). Risk medium. Deps: 3, 6. Lowest
    payoff; do last. The `#[cfg(test)]`-only `for_buffer_text_range` constructor
    is deletable independently anytime.

**Risk note for slices 5–10:** these touch live redisplay in exactly the areas
validated visually (posframe, tab-bar/tab-line, bidi, echo/minibuffer). Do them
incrementally with per-segment parity tests and GUI validation
(`NEOMACS_DUMP_FRAME_GLYPHS=1`, the screenshot+`xdotool` loop) — not a one-shot
rush. The keystone (6) is gated on the unscheduled prerequisite above; sequence
it explicitly or the legacy path can never be deleted.
