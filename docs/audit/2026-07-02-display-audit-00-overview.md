# Neomacs Rust Display Stack — Performance & Architecture Audit (Overview)

**Date**: 2026-07-02
**Scope**: the five display crates — `neomacs-display-protocol`, `neomacs-layout-engine`, `neomacs-display-runtime`, `neomacs-renderer-wgpu`, `neomacs-bin` — plus the redisplay integration points in `neovm-core`.
**Method**: four parallel read-only sub-audits (runtime/threading, GPU renderer, layout engine, protocol/integration), cross-checked against each other, with the two headline findings re-verified by hand against source. Struct sizes marked *measured* were obtained by compiling a probe crate against `neomacs-display-protocol` on x86-64 (default features). No benchmarks were run — the repository contains none (itself Finding 15) — so all cost statements are derived from code structure, allocation patterns, and measured type sizes, not from profiles. Where a claim could not be fully verified it is flagged inline.

This is the index and executive summary. The detailed subsystem reports:

- [01 — Runtime & Threading](2026-07-02-display-audit-01-runtime-threading.md)
- [02 — GPU Renderer (wgpu)](2026-07-02-display-audit-02-gpu-renderer.md)
- [03 — Layout Engine](2026-07-02-display-audit-03-layout-engine.md)
- [04 — Protocol & Integration](2026-07-02-display-audit-04-protocol-integration.md)
- [05 — Modernization Roadmap](2026-07-02-display-audit-05-modernization-roadmap.md)

---

## 1. Verdict

The skeleton of the Neomacs frontend is genuinely good: the right thread split, the right crate boundaries, the right libraries, and a GPU submission path whose *draw-call* structure is already excellent (a full screen of text batches to 1–3 draw calls through a persistent vertex arena). The GPU is nearly idle.

The problem is the execution model above the GPU: **every layer of the pipeline regenerates its entire output on every frame, and several layers do that work more than once per frame.** Layout re-walks every visible window from scratch on the Lisp thread (blocking Lisp), the protocol layer re-materializes the entire glyph buffer (including for frames that are then thrown away by coalescing), the render thread deep-clones the full frame three times (one of the clones is provably dead code), the renderer reconfigures the surface and recreates a full-screen stencil texture twice per frame, rebuilds ~5.7 MB of vertex data per full screen, and re-runs the mode-line's arbitrary Lisp on every redisplay with no cache.

The codebase is, in the words of its own design document (`docs/plans/2026-06-08-cursor-architecture-design.md`), *"the worst of both"* — an immediate-mode renderer paying retained-mode taxes. The irony is that the data structures needed for incrementality already exist: `GlyphRow` carries an FNV-1a content hash and a `row_equal()` comparator, and the TTY backend uses them to do proper damage-diffed output. The GUI — the performance-critical path — ignores them and rebuilds the world.

None of this requires a rewrite to fix. The high-order bits are deletions (dead clone, dead spacing pass), guards (resize), reuse (drain-then-materialize, face-map cache, mode-line cache), and finally wiring the already-computed row hashes into a damage path. The chassis — two threads, moved (never serialized) frame values, grid protocol, atlas + batched quads — is the same shape Zed, Ghostty, and Alacritty converged on, and it is the right shape.

## 2. The pipeline as it actually runs today

```
ENGINE / LISP THREAD ("neomacs-evaluator")            RENDER THREAD (OS main thread)
────────────────────────────────────────              ─────────────────────────────────
command loop → before read_char blocks:               winit event loop (ApplicationHandler)
  redisplay_for_input_wait()                          owns wgpu Device/Queue, WPE/WebKit,
    RedisplaySignature gate                           dedicated GMainContext
    (skip if nothing visible changed)
    └─ redisplay_fn → publish_gui_frame               about_to_wait() pump:
         for each frame (bottom→top):                   drain cmd channel
           layout_frame_display_state                   poll_frame(): drain frame channel,
             LayoutEngine::layout_frame_rust              materialize() EVERY queued frame,
             — every window re-laid-out                   keep only the newest
               from scratch, serially,                  pump GLib (WPE)
               ON this thread                           tick blink/cursor/transitions
             — mode/header/tab-line:                    mark dirty → request_redraw()
               arbitrary Lisp eval, uncached            compute next wake (4ms active /
         → FrameDisplayState (character grid,             blink deadline / Wait idle)
           row hashes computed but unused by GUI)
         frame_tx.try_send  ──── unbounded ────→      RedrawRequested:
         (moved value, zero serialization)              clone frame ×3 (1 dead)
                                                        rebuild face map (clones)
  input_rx ←──── bounded(4096) ──────────────           resize() ×2 (no guard)
  (keys: blocking send from render thread!)             rebuild ALL vertices (~5.7 MB)
  wakeup pipe (1 byte per event)                        1 main pass LoadOp::Clear
                                                        + 1 encoder+submit per overlay
                                                        present() (Fifo/vsync)
```

TTY mode branches off after `FrameDisplayState`: the same layout engine output goes through `TtyRif`, which keeps current/desired grids, **diffs row hashes**, and emits only changed cells as ANSI. The GUI path has no equivalent.

## 3. What is already right (keep these)

1. **Thread architecture**: Lisp+layout on one thread; winit+wgpu+WPE on the render thread; three crossbeam channels + a wakeup pipe; frame payloads are **moved, never serialized or copied across the channel**. This is the modern shape.
2. **Dirty-driven redraw**: the render thread renders only on `RedrawRequested`; idle is `ControlFlow::Wait`. `RedisplaySignature` (engine side) suppresses frames when nothing visible changed.
3. **Renderer core**: all 14 render pipelines created once at startup; main text batched through a persistent, bump-allocated, ×2-growing `FrameVertexArena` (`write_buffer` into a reused buffer — no per-frame VBO allocation for text); ~1–3 draw calls per screenful of text with bind-group changes only on atlas-page switches.
4. **Glyph atlas**: swash rasterization; three atlas families (R8 grayscale, RGBA8 subpixel, RGBA8-sRGB color/emoji); 2048² shelf-packed pages (≤8 per family); page-level LRU eviction with generation stamps; quarter-pixel subpixel positioning bins; COLR/CPAL and bitmap emoji; ZWJ cluster composition.
5. **Layout feature architecture**: faces, invisible text, display properties, and overlays resolve from in-process interval trees with per-run `next_check` boundary checkpoints — **no per-character VM crossings anywhere**, and only one per-character *Rust* probe (overlay strings) in the whole feature set. Bidi has a true LTR fast path (full UAX#9 only for rows containing ≥ U+0590).
6. **Real shaping exists**: cosmic-text (eval-exec fork) with harfrust (the HarfBuzz Rust port) + skrifa underneath; complex scripts (Arabic, Indic, Thai, …) are run-shaped.
7. **Media groundwork**: GStreamer video decode off-thread with latest-frame coalescing; image decode on a thread pool; a complete Vulkan DMA-BUF zero-copy import implementation exists (currently switched off — see Finding 10).

## 4. Ranked findings

Severity-ordered. Each is elaborated in the subsystem reports; file:line references are to the state of the tree on 2026-07-02.

| # | Finding | Evidence | Status |
|---|---------|----------|--------|
| 1 | **3× full `FrameGlyphBuffer` deep clone per rendered frame; clone #2 (plus its `apply_extra_spacing` pass over all glyphs) is dead — overwritten before any read.** | `neomacs-display-runtime/src/render_thread/render_pass.rs:395` (clone 1, used only to read `effect_hints` + fps), `:398-406` (clone 2, dead — reassigned at `:452`), `:449` via `take_current_frame_for_render` which *clones*, not takes (`frame_windows.rs:622-628`) | verified by hand |
| 2 | **`renderer.resize()` called twice per rendered frame with no same-size guard** → 2× `surface.configure` + 2× full-screen `Stencil8` texture recreation + uniform write, every frame. | `render_pass.rs:469` and `:611` (restore-back); `neomacs-renderer-wgpu/src/renderer/mod.rs:1612-1643` (no dimension guard) | verified by hand |
| 3 | **Mode-line / header-line / tab-line formats are re-evaluated as arbitrary Lisp on every redisplay for every visible window, with zero cross-frame caching.** An in-code comment measures ~4.3 ms for a Doom-config mode-line — most of a 60 Hz frame budget, paid per redisplay, on the Lisp thread. | `neomacs-layout-engine/src/display_status_line.rs:56-57` (cost comment), `:1023-1070` → `neovm-core .../xdisp.rs:996-1041` (`:eval` = arbitrary Lisp); no cache: zero hits for cache/memo/dirty in the module | verified by sub-audit |
| 4 | **No damage tracking anywhere on the GUI path** — while the protocol already computes per-row FNV-1a hashes and `row_equal()`, and the TTY backend diffs them. GUI re-materializes and repaints everything; window dirtiness is a single `bool`. | `neomacs-display-protocol/src/glyph_matrix.rs:252,337,383` (hashes), `tty_rif.rs:599` (TTY diff); `frame_glyphs.rs:766-768` ("cleared and rebuilt from scratch each frame"); `glyphs.rs:2005` | verified |
| 5 | **No layout row cache**: `LayoutEngine`'s only cross-frame state is `prev_window_infos`/`prev_selected_window_id`/`prev_background` (transition hints). Every redisplay re-lays-out every visible window from scratch, serially, on the Lisp thread. | `neomacs-layout-engine/src/engine.rs:150-200` (struct), `:366-919` (`layout_frame_rust`), `:928-1048` (`layout_window_rust`); no rayon/threads in the crate | verified by hand |
| 6 | **`materialize()` runs on frames that are then coalesced away**: `poll_frame` materializes every queued `FrameDisplayState` inside the drain loop, then keeps only the newest per frame-id. Under redisplay bursts the full O(glyphs) rebuild is paid N times for one render. | `neomacs-display-runtime/src/render_thread/frame_ingest.rs:260-273`; `frame_windows.rs:545-550` | verified |
| 7 | **`create_buffer_init` per draw per frame for everything except main text** — every UI overlay (15+ sites, each with its own encoder and `queue.submit`), every inline image, every active effect, backgrounds/borders/cursor. The good arena pattern exists but is applied only to the main text path. | `ui_overlays.rs:183,217,424,487,1256,2214,2736,…`; `glyphs.rs:37-43,2080,2124,3148,3306,3321,3433,3716,3746,4042,4078,4358`; `content.rs:1342` | verified |
| 8 | **Font metrics hot path allocates a `String` per `char_width` call** (cache key built before the probe, even on ASCII-array hits); all metrics caches unbounded; `clear_caches` has **zero call sites** (documented invalidation never fires); shape-run cache does clear-all at 8192 entries instead of LRU; a fontset change invalidates the entire fontconfig fallback cache at once (FcFontList FFI re-storm). **Latin/CJK text is measured per-character — programming ligatures are never run-shaped.** | `font_metrics.rs:242,265-269,688,709-714,491-494,930`; `fontconfig.rs:42-43,98-106`; `display_source_append_plan.rs:101-108` | verified by sub-audit |
| 9 | **`EffectsConfig` is 3,576 bytes / 149 effect fields (measured), cloned per window into every frame payload and deep-cloned again per frame in the renderer** — configuration data riding the per-frame hot path. | `effect_config.rs:1576`; `glyph_matrix.rs:1092,1140`; `glyphs.rs:2037-2040` | measured |
| 10 | **Zero-copy is implemented and switched off**: video takes a by-design GPU→CPU→GPU round trip with a full-frame `to_vec()` per displayed frame (VA-API decodes on GPU, downloads, re-uploads); the complete Vulkan DMA-BUF import path is dead (`dmabuf_info = None`, RADV fd-leak workaround); WebKit defaults to CPU pixel upload (`NEOMACS_WEBKIT_IMPORT=dmabuf-first` opt-in). Image-cache eviction is FIFO-by-lowest-id (can evict a visible image); video and WebKit caches are unbounded. | `video_cache.rs:856-869,963-968,984`; `vulkan_dmabuf.rs:335-573`; `image_cache.rs:50,1117-1133`; runtime `state.rs:120-132` | verified |
| 11 | **Text rendering color math is inconsistent and partly wrong**: the grayscale shader pre-multiplies coverage in the fragment *and* blends with `SrcAlpha` (alpha applied twice) plus manual pow-2.2 gamma over an sRGB target; the subpixel shader has **no** gamma handling and composites against a per-glyph *assumed* background with blending disabled — wrong wherever the real background isn't that color (images, gradients, effects). | `glyph.wgsl:46-50`; `glyph_subpixel.wgsl:46-55`; pipeline blends `mod.rs:876,914-931` | verified; perceptual impact unmeasured |
| 12 | **Input-path hazards**: non-lossy input (keys, buttons, scroll) uses a *blocking* channel send from inside the winit event callback — a stalled engine with a full 4096-entry queue freezes the UI thread. Every wheel event in wpe-webkit builds does an O(glyphs) scan for xwidget hit-testing. One wakeup-pipe syscall per event, unbatched. | `thread_comm.rs:1023,996-1037`; `pointer_events.rs:19-36,2127-2128` | verified; webkit scan is wpe-feature-gated (off by default) |
| 13 | **Frame pacing mismatch**: when anything is animating, the pump wakes every 4 ms (~250 Hz) while presentation is hard-locked to Fifo/vsync (~60 Hz) — ~4× over-ticking, each tick re-marking windows dirty. Idle-dim animation hardcodes a 16 ms step (framerate-dependent). No Mailbox/Immediate option exists. | `lifecycle.rs:306-323`; `frame_windows.rs:712-713`; `mod.rs:1393-1408` | verified |
| 14 | **Large dead subsystems ship in-tree and mislead readers**: the retained `Scene` graph with dirty-region tracking (abandoned mid-build — `// TODO: Build text nodes from glyph rows`), the `DisplayBackend` trait + `BackendType` enum (never dispatched; the runtime is hardwired to the concrete `WgpuRenderer`), `core/{animation,cursor_animation,buffer_transition,animation_config,profiler}.rs` (test-only), `media_budget.rs` (unused), two orphan shaders (`gradient.wgsl` — also invalid WGSL — and `texture.wgsl`), `va_dmabuf_export.rs` (no live callers), layout `display_iterator.rs` (flagged dead by `docs/plans/2026-06-21-display-pipeline-deletion-plan.md`), plus a debug per-glyph `format!` scan near a y-band on the glyph hot path. | `scene.rs:504`; `backend/mod.rs:17-43`; `core/mod.rs:8`; `glyphs.rs:1109-1150`; `transitions.rs:72` | verified |
| 15 | **Zero performance instrumentation**: no criterion benches or `benches/` anywhere in the display stack; no GPU timestamp queries (`timestamp_writes: None` on every pass); the only tools are an env-gated stats counter (`NEOMACS_RENDER_STATS`) and a wall-clock FPS counter. For a project whose stated goal is "blazing fast", there is currently no way to detect a regression. | repo-wide search; renderer passes; `frame_state.rs:19-30` | verified |

## 5. Cost model (200×60 ≈ 10k visible glyphs, from code structure)

Per **rendered** frame on the render thread, today:

- 3 deep clones of a `FrameGlyphBuffer` whose glyph vector alone is ~1.1 MB (10k × 112 B) plus faces map (248 B/face) and side vectors.
- ~15–20 full scans of the glyph vector (box spans, backgrounds, rows/scrollbars, overlay rects, cursor background, stats, 2× main build passes, image/video/webkit/decoration scans) ≈ 150k–200k glyph visits.
- Main text build: per glyph — 1-entry face cache probe, 2× `SubpixelBin` computations, 1 atlas HashMap lookup + linear page-pin scan, construction of 6 grayscale vertices *and* 6 subpixel vertices (one set discarded) → ~120k vertices built, ~5.7 MB of vertex bytes moved (build → flat_map copy → write_buffer), plus ~2.9 MB built-and-dropped when subpixel is off.
- 2 × `resize()` (2 surface configures + 2 full-screen Stencil8 creations + uniform writes).
- ~15+ `create_buffer_init` calls (backgrounds, borders, cursor, overlays, images), one encoder + submit per visible overlay.
- 1 `EffectsConfig` deep clone (3.5 KB struct with owned Vecs) even when idle.

Per **redisplay** on the Lisp thread: full window re-layout for every visible window + one arbitrary-Lisp mode-line eval per window (+ header/tab lines) + full grid build + (on the render thread) full materialize — repeated for every queued frame even if coalesced away.

With any continuous cursor effect enabled, all of the above repeats every vsync.

GPU-side, by contrast: ~10–30 draw calls, one main submit, a few overlay submits. **The frame is CPU-bound; the GPU pipeline is not the bottleneck.**

## 6. Comparison with the state of the art

- **Zed (GPUI)**: full-frame repaint every frame — but per-frame CPU is tiny because layout is retained and painting is instanced quads from cached data. Lesson: full repaint is *fine*; expensive regeneration is not. Neomacs's `LoadOp::Clear` is not the sin; the 5.7 MB rebuild feeding it is.
- **Ghostty / Alacritty**: cell grids, damage tracking, instanced per-cell rendering, sub-ms CPU frames. Neomacs already *has* the cell grid and the row hashes — it just doesn't use them for the GUI.
- **Neovide**: grid-diff protocol from the editor core (Neovim's UI protocol is inherently damage-based) + GPU compositing with animation layered on top. Closest in spirit to what `FrameDisplayState` + row hashes could become.
- **emacs-ng (WebRender)**: retained display lists; proved GPU Emacs is viable but carried a heavy dependency. Neomacs's bespoke wgpu renderer is leaner and already competitive at the draw-call level.
- **GNU Emacs**: the original incremental-redisplay design (current/desired matrices, row-level diff) exists precisely because arbitrary Lisp can invalidate anything. Neomacs replaced C xdisp with a clean Rust engine but dropped the incremental half. The roadmap's Phase 2 restores it with better tools (content hashes instead of pointer identity).

## 7. Where this goes

The [Modernization Roadmap](2026-07-02-display-audit-05-modernization-roadmap.md) lays out five phases:

- **Phase 0 — instrument** (GPU timestamps, keystroke-to-photon tracing, criterion benches) so gains are provable and regressions are caught.
- **Phase 1 — stop paying the dumb taxes** (dead clone, resize guard, drain-before-materialize, face-map reuse, skip-subpixel-when-off, key-alloc fix, EffectsConfig off the hot path, non-blocking input, tick clamping). Days of work; removes the majority of avoidable per-frame CPU with zero architectural change.
- **Phase 2 — make frames incremental** (GUI row-hash damage diff, mode-line cache, per-window layout row cache). This is the real win: a keystroke becomes "re-layout one row, diff, upload a few KB, draw."
- **Phase 3 — modernize GPU submission** (instanced glyphs, arena everywhere, single encoder, premultiplied + dual-source-blended text, DMA-BUF on by default with denylist, optional Mailbox).
- **Phase 4 — endgame** (parallel per-window layout after splitting mode-line eval out; snapshot-based off-thread layout only if profiles still demand it) — plus deleting the dead subsystems inventoried in Finding 14.

Explicitly rejected: compute-shader/vello-style glyph rasterization, WebRender adoption, and a separate display-server process. At editor workloads, atlas + instanced quads is the converged industry answer; the bottleneck was never rasterization.

**End-state target**: keystroke → signature check → one-row relayout → row-hash diff → few-KB GPU upload → one draw → present. Sub-millisecond CPU, trivial GPU — Zed/Ghostty-class latency with full Emacs semantics.
