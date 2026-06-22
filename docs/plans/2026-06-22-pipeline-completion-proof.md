# Display Pipeline Completion Proof — Measurement + Face/Source Policy

Date: 2026-06-22
Branch: `main` (HEAD synced with `origin/main`)
Verification: `cargo nextest run -p neomacs-layout-engine` → **1250 / 1250 pass** on current HEAD.

## Purpose

Close the two open completion-audit items agreed with review:

1. Exhaustive per-path proof that **text measurement/advance** is owned at the shared row boundary.
2. Exhaustive per-path proof that **face/source policy** is owned at the shared boundary.

Method: identify the single authority for each dimension, enumerate every source
path, show each path reaches the authority, then **falsify** — search for any path
that computes advance or realizes a face *outside* the authority. A "real divergent
path" would be a finding requiring code work; none was found.

## The two shared authorities

### A. Measurement / advance — one authority

- **Glyph emission + per-char advance:** `DisplayRowWriter` (`display_row_builder.rs`).
  `glyph_advance_px` resolves advance through a `dyn DisplayGlyphMeasurer`
  (the `FontMetricsService`); `resolve_advance_px` is measured-or-fallback;
  `from_glyphs` derives column metrics (incl. composed clusters via
  `composition::composed_cluster_cols`).
- **Per-face measured metrics:** `display_row_face_state.rs::advance_for_char(font_metrics, fallback_advance_px)`
  — the font-metric-backed width carried on the active face. Every advance in this
  module routes through `advance_for_char` (or the `DisplayRowCharWidthPolicy`
  fallback), never an independent rule.
- **`(space :width/:align-to …)`:** one evaluator, `calc_pixel_width_or_height`
  (`display_pixel_calc.rs`) — unified for buffer **and** chrome as of the
  2026-06-21 `(space)` slice (the inferior `length_expr_pixels` was deleted).
- **Documented exception (not a divergence):** the TTY / no-font-metrics
  `FaceColumns` advance, gated on `mode.uses_font_metrics() == false`. This is the
  GNU `term.c` `produce_glyphs` analog, not removable duplication.

### B. Face / source policy — one authority

- **Per-origin base face:** `BaseFacePolicy::from(DisplayOrigin)`
  (`display_face_policy.rs`) — enumerates **every** origin (verified exhaustive):
  `BufferText → BufferFaceIncludingOverlays`, `OverlayString → OverlayStringAtAnchor`,
  `DisplayPropertyString → DisplayPropertyUnderlyingFace`,
  `LinePrefix/WrapPrefix → (underlying)`, `ModeLine → FixedBasicFace(ModeLine{Active,Inactive})`,
  `HeaderLine → FixedBasicFace(HeaderLine{Active,Inactive})`, `TabLine → FixedBasicFace(TabLine)`,
  `TabBar → FixedBasicFace(TabBar)`.
- **Resolution:** `FaceResolver` — `face_at_pos` (default → text-prop → overlays-last,
  the GNU `face_at_buffer_position` merge order; independently verified by the
  2026-06-21 face-merge regression test), `face_for_overlay_string`, and fixed
  basic-face lookups.
- **Realization:** `resolve_and_install_measured_face` — **exactly 2 call sites**,
  both in the shared render path (`display_row_source_render.rs`,
  `display_buffer_source_face_resolution.rs`). No face is realized below the row
  boundary.

## Per-path proof

Every real source implements `DisplayItemSource` and flows through one shared row
center (`DisplayRowRenderer` / `DisplayRowWriter`; `DisplayRowRenderer` has 2 call
sites total). The base face for each is one `BaseFacePolicy` variant.

| Source path | `DisplayItemSource` | `DisplayOrigin` | Base-face policy | Advance route |
|---|---|---|---|---|
| Main buffer text | `BufferTextSourceCursor` | `BufferText` | `BufferFaceIncludingOverlays` (`face_at_pos`) | `DisplayRowWriter` |
| Overlay before/after strings | `LispStringSourceCursor` | `OverlayString` | `OverlayStringAtAnchor` | `DisplayRowWriter` |
| Display-property (string/stretch/media/mapped) | `BufferDisplayReplacementStringSource` / `LispStringSourceCursor` | `DisplayPropertyString` | `DisplayPropertyUnderlyingFace` | `DisplayRowWriter` (+ `calc_pixel_width_or_height` for stretch) |
| Mode-line / header-line / tab-line / tab-bar | `LispStringSourceCursor` | `ModeLine`/`HeaderLine`/`TabLine`/`TabBar` | `FixedBasicFace(…)` | `DisplayRowWriter` (+ `calc_pixel_width_or_height` for `(space)`) |
| Active minibuffer / inactive echo | `BufferTextSourceCursor` (active = its own buffer; inactive = ` *Echo Area 0*` via `ensure_echo_area_buffers`, `engine.rs:104`) | `BufferText` | `BufferFaceIncludingOverlays` | `DisplayRowWriter` |
| Child-frame / posframe | per child's own sources, via `layout_frame_rust` (`engine.rs:348`) | per-source | per-source | `DisplayRowWriter` |
| Special glyphs / line-number margin | `RightEdgeMarkerItemSource` / `LineNumberMarginItemSource` | special-glyph origins | fixed/derived | `DisplayRowWriter` |

Minibuffer/echo reduce to the buffer-text path (they *are* buffer walks).
Child-frames reduce to whatever sources they contain, through the same pipeline —
so neither introduces a new measurement or face path.

## Falsification result

Searched the whole crate for advance/width computation and face realization outside
the authorities:

- **Measurement:** the only out-of-authority hits are (a) the line-number *gutter*
  width (`display_buffer_window_geometry.rs` — window chrome geometry, not glyph
  advance), and (b) wrap/cursor decisions that consume the **already-resolved**
  advance (`display_row_walk_state.rs`, `display_source_item_append.rs`) — the
  legitimate shared-progress channel GNU also uses (shared `it` state). **No
  independent pixel-width rule.**
- **Face:** `resolve_and_install_measured_face` has exactly 2 callers (both shared);
  `BaseFacePolicy::from` covers every origin. **No below-boundary face realization.**

**No divergent path found.**

## Conclusion

Measurement/advance and face/source policy are **owned at the shared boundary for all
source paths**. The only per-path variation is (1) the *source-specific base-face
policy* and (2) the *source cursor* — both are GNU-faithful by design (GNU likewise
varies the base face per source: `face_at_buffer_position` vs `face_at_string_position`
vs the mode-line face), not divergence.

**The two open audit items are closed. The refactor spine is complete.**

Remaining work is **not** code refactoring:
- Doc cleanup (this commit supersedes the older "extract more modules" phases in the
  2026-06-13 handoff).
- The intentionally-capped items remain correct-as-is, **not** debt: the TTY
  `FaceColumns` fallback; and the three "purity" candidates (buffer display-prop
  feed, special-glyphs-into-lifecycle, overlay continuation) which were each found to
  be redesigns or already-correct GNU design, not behavior-preserving folds.
