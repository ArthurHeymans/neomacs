# Display Audit 03 — Layout Engine (`neomacs-layout-engine`)

**Date**: 2026-07-02 · Part of the [display stack audit](2026-07-02-display-audit-00-overview.md).
**Scope**: `neomacs-layout-engine/` — pipeline shape, incrementality, the VM/bridge boundary, display features, status lines, fonts/metrics/shaping, allocation hygiene.
**Note**: this crate is the `xdisp.c` replacement. It is by far the largest display crate (~99k lines including tests) and, feature-wise, the most complete and disciplined part of the stack. Its two structural gaps — no incrementality and uncached status-line eval — dominate its cost profile.

---

## 1. Pipeline shape

```
neovm-core buffer/window/face state
   │  (in-process reads through LayoutBufferView / neovm_bridge.rs — no eval,
   │   interval trees + next_check boundary checkpoints)
   ▼
LayoutEngine::layout_frame_rust           engine.rs:366-919  (per frame; ON the Lisp thread)
   ├─ per window: layout_window_rust      engine.rs:928-1048
   │    └─ display source walk → DisplayItem stream → DisplayRowRenderer
   │       → RenderedDisplayRow → installer   (docs/plans/2026-06-13 handoff shape)
   ├─ chrome rows: tab-line / header-line / mode-line   display_status_line.rs
   │    └─ format_mode_line via neovm-core (ARBITRARY LISP, uncached)
   ├─ frame-level: tab-bar, menu/tool bar items, borders, cursors, fringes
   ▼
FrameDisplayState (character grid + side lists + faces map)
   │  glyph_matrix.rs:657 — GlyphRow FNV hashes computed here
   ▼
frame_tx  (GUI)                    TtyRif rasterize + row-hash diff (TTY)
```

- The engine is invoked **unconditionally per redisplay** via `neomacs-bin/src/tty_layout.rs:37-79` (misleading file name — it is the shared GUI+TTY entry point), for every frame in the render tree bottom-to-top (`main.rs:3471-3482`).
- Layout runs **on the Lisp/evaluator thread** and blocks Lisp until the whole frame is laid out. Only GPU work is off-thread.
- Windows are laid out **serially**; there is no rayon, no thread pool, no SIMD in the crate.

## 2. Incrementality — the central finding

**There is none.**

- `LayoutEngine`'s cross-frame state (struct at `engine.rs:150-200`) is: `prev_window_infos: HashMap<DisplayWindowId, WindowInfo>` (`:169`), `prev_selected_window_id` (`:171`), `prev_background` (`:173`) — all used to *detect transitions/effects hints* (`:244-282,910`), none used to reuse layout output.
- There is **no display-row cache**: no structure maps buffer positions or content hashes to previously-built rows. Searches for cross-frame caching (`cache|memo|dirty|unchanged|last_frame`) in the row-construction modules come back empty (the only caches are font metrics — §6).
- Deceptive file names, clarified: `display_row_replacement.rs` implements **`display`-property text replacement** (a display spec replacing buffer text), not frame-to-frame row replacement. `display_buffer_source_row_lifecycle.rs` implements the **within-one-pass** row lifecycle (hscroll skip, selective display, invisible text, line breaks, end-of-buffer tails), not cross-frame lifecycle.
- Upstream, the only gate is `RedisplaySignature` in `neovm-core` (`eval.rs:6836-6848`): buffer/overlay/text-property ticks + point + geometry. It suppresses *identical whole frames* — a genuinely good short-circuit — but once any tick changes, **everything visible is re-laid-out**: every window, every row, every chrome line.

Consequences, concretely:

- **Single-char self-insert in a 10k-line buffer**: signature differs → full relayout of *all* visible windows (not just the edited one), all mode/header/tab lines re-evaluated, full grid rebuilt, full materialize, full GPU rebuild.
- **Scroll one line**: same — no row reuse, despite ~97% of rows being identical content at shifted y.
- **Cursor blink**: does *not* re-run layout (blink is render-thread-owned — correct), but any redisplay for other reasons pays the full path.

The protocol layer computes per-row FNV-1a content hashes (`glyph_matrix.rs:252,337`) *after* layout — used today only by the TTY diff. These hashes are the natural foundation for both a GUI damage path (roadmap Phase 2) and a layout-side row cache.

## 3. The VM/bridge boundary (`neovm_bridge.rs`) — per-run, not per-char

This is the strongest part of the crate's design. The `LayoutBufferView` trait (`neovm_bridge.rs:92-121`) exposes buffer text, text properties, overlays, faces, point, and window config as **in-process data-structure reads** — no `eval_form`, no VM dispatch — and every feature uses **`next_check` boundary checkpoints** so work happens per-run:

| Feature | Granularity | VM eval? | Evidence |
|---|---|---|---|
| Faces (text props + overlays merged) | per-run (`next_check`) | No | `neovm_bridge.rs:3034-3118`; overlay ends folded into next_check `:3074-3077,3110-3115` |
| Invisible text | per-boundary bulk next-change | No | `neovm_bridge.rs:2007-2089`; scans only the `invisible` prop (`:2076-2079`) min overlay boundary (`:2082`); checkpointed in `display_buffer_source_row_lifecycle.rs:1153-1161` |
| Display properties | per-run; parses already-fetched Values into typed enums | No | `display_property.rs:109` (classify), `display_spec.rs`; `(space :align-to …)` → `DisplayLengthExpr` (`:238-283`) resolved against Rust metrics |
| Overlay faces | per-run | No | interval tree `Itree` in `neovm-core/src/buffer/overlay.rs:266-272`, O(log n + k) queries `:405-413` |
| Overlay before/after strings | **per-char probe within runs** (see below) | No | `display_row_overlay_string.rs:270-282` |
| Line numbers | 2 Rust line counts per window + incremental advance | No | `display_buffer_window_geometry.rs:251-287`; `display_row_walk_state.rs:485-500` |
| Bidi | per-row; **skipped entirely for LTR rows** | No | `glyph_row_writer.rs:454-476`: `all_ltr` when no char ≥ U+0590 → identity levels/reorder; full UAX#9 (`bidi/resolver.rs:13`) only otherwise |
| **Mode/header/tab-line** | **per window, per redisplay** | **YES — arbitrary Lisp** | §4 |
| Tab-bar build | per frame (when `tab-bar-make-keymap-1` bound) | YES (several `eval_form`s) | `display_status_line.rs:1127-1264` |

**The one per-character scan in the feature set**: `first_overlay_string_charpos_in_range` (`display_row_overlay_string.rs:270-282`) does `(start..end).find(|c| !overlay_strings_at(c).is_empty())` per text run whenever overlay handling is enabled — O(run_len × log n) interval-tree probes. Callers: `display_buffer_source_text_run.rs:76,236`. A "next overlay boundary ≥ pos" query on the interval tree would eliminate it. This is a Rust-side cost only; it never crosses into the VM.

## 4. Status lines — the expensive per-frame Lisp

- `eval_status_line_format_value` (`display_status_line.rs:1023`) reads the format (window-param → buffer-local → global, `:1035-1053`) and calls `neovm_core .../xdisp.rs:996 format_mode_line_for_display` → `format_mode_line_recursive` (`xdisp.rs:1041`), which evaluates `:eval` forms — **arbitrary Lisp**, with the target buffer made current and the window selected.
- Invoked once per visible chrome row per window inside `WindowChromeRowsRenderRequest::render`: tab-line `:604`, header-line `:643`, mode-line `:688-706` — **on every redisplay**, since `layout_frame_rust` has no per-window change gate.
- **No cache of any kind**: no key on buffer tick/point-line/format value; searches for cache/memo/dirty in the module return nothing. (`MODE_LINE_EVAL_COUNT` at `:66-86` only proves once-per-redisplay, i.e. no double eval — it is not a cross-frame cache.)
- The in-code comment (`:56-57`) records the stakes: **~4.3 ms for a Doom-config mode-line** — most of a 60 Hz frame budget, paid per redisplay, per window, on the Lisp thread.
- The *rendering* of the returned propertized string is cheap (it reuses the same Rust `next_check` text-property harvester, `render_lisp_string_source_request` `:166`); the cost is producing the string.

GNU Emacs has the same structural problem and mitigates with mode-line caching; the roadmap's mode-line cache (key: format value identity + buffer tick + point line + window geometry + selected-ness) is Phase 2's second-biggest win after the row diff.

## 5. Shaping — real, but routed per-character for Latin

- **cosmic-text 0.18.2 (eval-exec fork)** is the only shaper (`Cargo.toml:128,180`); underneath it: **harfrust** (HarfBuzz Rust port) + **skrifa** (font reading) + **swash** (scaling/raster). `rustybuzz` appears in Cargo.lock but is not cosmic-text's shaper. `allsorts` 0.16 is used **only** to decode WOFF/WOFF2 into SFNT (`font_loader.rs:117-198`). `unicode-script` is a declared dependency but **unused** — `complex_script` hand-rolls its own ranges (`composition.rs:132-155`). Dead dep.
- Every `Buffer::set_text` uses `Shaping::Advanced` (`font_metrics.rs:406,471,566,622,785,816,905`) → real shaping when a run is shaped together.
- **Routing is per-character** (`display_source_append_plan.rs:101-108`, `DisplaySourceAppendMeasurementKind::for_char`):
  - Complex scripts (Arabic, Syriac, Indic, Thai, Lao, Tibetan, Myanmar) → `ResolvedComplexRun` → run-shaped once via `FontMetricsService::shape_run` (`font_metrics.rs:434-496`; callers `display_row_face_state.rs:941-952`, `display_text_run_measurement.rs:118-144`).
  - **Everything else — Latin, CJK, emoji — is measured per character** via `char_width`. Consequence: **programming ligatures (`=>`, `!=`, `->`) never get a shaped (single-ligature) advance** on the measurement path; display/replacement strings do get run-shaped (`display_row_face_state.rs:720-730`, `display_row_replacement.rs:61`).

## 6. Font metrics & matching — good caches with three flaws

**Hot path** for the common case: `advance_for_char` → `glyph_advance_px` (`display_row_face_state.rs:445-448`) → `FontMetricsService::char_width` (`font_metrics.rs:680-730`):

- **ASCII fast path** (`:692-701`): `ascii_cache: HashMap<MetricsCacheKey, [f32;128]>` — hit = array index; miss = shape all 95 printable ASCII chars once per (face,size), each in its own cosmic-text `Buffer` (`:803-825`).
- **Non-ASCII** (`:703-729`): `char_cache: HashMap<(MetricsCacheKey, char), f32>`; miss shapes that single char.
- Frame cell width: `FrameColumnWidth::from_advances` (`:128-139`) — monospace families use max/fixed-pitch width, proportional use average; individual glyph advances still probe the cache (no "fixed advance, skip lookup" fast path).

**Flaw 1 — per-char String allocation**: `MetricsCacheKey::new` does `family.to_string()` (`:242`) and is constructed **before** the cache probe (`:688`), so *every measured character* — including ASCII-array hits — allocates a `String` to build the lookup key. Non-ASCII builds a second key (`:709-714`). `shape_run` likewise builds `(key, text.to_string())` per call and `clone()`s the cached `Vec<ShapedGlyph>` on hit (`:445-450`). An interned face-id → key map removes all of it.

**Flaw 2 — unbounded caches, dead invalidation**: `ascii_cache`, `char_cache`, `metrics_cache` have no cap and no eviction. `clear_caches` (`:930`) — documented as "call when fonts change (text-scale-adjust)" — has **zero call sites in the repository**. Because size is in the key (`font_size_centipx`, `:246`), text-scale changes are *correct* but every size ever used persists forever; and `prime_file` can load a font mid-session **without invalidation** (`:280-293`) — a font-file swap under a fixed (family,weight,size) key serves stale metrics. The `shaped_run_cache` is capped at 8192 but the overflow policy is **clear-the-entire-map** (`:491-494`), not LRU. `intern_family` `Box::leak`s a String per distinct family (`:340`).

**Flaw 3 — fallback cache mass-invalidation**: font matching uses real fontconfig FFI (`fontconfig.rs`, `yeslogic-fontconfig-sys`; the workhorse `fc_list_candidates` `:1214-1377` builds FcPattern/FcCharSet and runs `FcFontList`). Cold miss per uncovered character is expensive (a full system font list scan, repeated across alternative families and query langs, `:623-644,775-798`). Results are cached in `FC_CHAR_MATCH_CACHE` (`:42-43`) keyed on `(family, codepoint, prefer_monospace, weight, italic, fontset_generation)` (`:98-106`) — so **any fontset change bumps the generation and cold-starts the entire fallback cache**: the next redisplay re-runs FcFontList FFI for every non-ASCII character on screen. Old entries are never evicted, only shadowed. (Contrast: `FAMILY_WEIGHT_CACHE` in `font_match.rs:33,68-72` folds `db().len()` into its key — a survivable-growth pattern the other caches don't replicate.)

**Font loading is clean**: fontdb `Source::File` (mmap via memmap2) registered once per path, gated by `path_to_family` (`font_loader.rs:20-23,42,52`); failures cached as `None`; WOFF is the only read-into-Vec path; the `FontSystem` is created once and owned by the engine (`engine.rs:165,295,353`). No per-frame font work.

## 7. Feature costs worth knowing

- **Line numbers**: two Rust newline counts per window (`display_buffer_window_geometry.rs:251-287` via `access.count_lines`, `neovm_bridge.rs:1783`), then incremental `advance_line` per row (`display_row_walk_state.rs:485-500`); number text is a `format!` per row (`:80`). No VM calls.
- **Invisible text**: boundary hops, one probe per boundary; the only loop is collapsing consecutive hidden runs into one ellipsis (GNU parity), still boundary-bounded (`neovm_bridge.rs:2023`).
- **Bidi**: `all_ltr` scan per row (cheap; `units.iter().any(|u| u.ch >= '\u{0590}')`), full resolver only for rows that need it. The comment at `glyph_row_writer.rs:458-463` correctly identifies pure-LTR as "rank-1 redisplay-layout cost center" and fast-paths it.
- **Overlay strings**: the per-char probe (§3) — the one avoidable per-position cost in the feature set.

## 8. Summary of layout-layer debt

1. **No incrementality**: no row cache, no per-window change gate; every redisplay re-lays-out everything (§2). The single biggest architectural gap in the stack.
2. **Status lines eval arbitrary Lisp per window per redisplay, uncached** (§4). The single biggest per-redisplay time sink on the Lisp thread.
3. Layout **blocks the Lisp thread** (serial, per-window; `rust-display-engine.md` defers parallel layout to "Phase 8+").
4. **Latin ligatures never run-shaped**; shaping only for complex scripts and display strings (§5).
5. Per-char `String` allocation on the width hot path; unbounded metrics caches; `clear_caches` dead; clear-all shape-cache overflow; fallback-cache mass invalidation on fontset change (§6).
6. Per-char overlay-string probe (§3).
7. Dead scaffolding flagged by the project's own deletion plan (`docs/plans/2026-06-21-display-pipeline-deletion-plan.md`): `display_iterator.rs` (abandoned 2026-04-11 `struct It` port), and the pipeline's +2245-line/70→81-module growth during the refactor.
8. Dead dependency: `unicode-script`.

### What surprised the auditors

- How *clean* the VM boundary is — per-run checkpoints everywhere, interval trees, no eval in the feature path. The expensive Lisp is confined to exactly the places GNU also pays it (mode-line, tab-bar).
- That the grid + row hashes — the raw material of incremental redisplay — are built every frame and then used only by the TTY.
- That `tty_layout.rs` is the shared layout entry point for GUI and TTY (the name suggests otherwise).
