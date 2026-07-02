# Display Audit 05 — Modernization Roadmap: to an Ultra-Fast, Modern GPU Pipeline

**Date**: 2026-07-02 · Part of the [display stack audit](2026-07-02-display-audit-00-overview.md).
**Goal**: keystroke → photon in well under a frame, sustained; idle at true zero cost; media zero-copy; text color-correct — without giving up full Emacs semantics (arbitrary Lisp may invalidate anything at any time).

**End-state target, stated up front**: a keystroke costs — signature check → re-layout of the affected row(s) only → row-hash diff → a few-KB instance-buffer update → one draw → present. Sub-millisecond CPU, trivial GPU. That is Zed/Ghostty-class latency with GNU-Emacs semantics, and nothing in the current chassis (two threads, moved grid frames, atlas + batched quads) prevents it.

The roadmap is ordered so that every phase is independently shippable, independently measurable, and none requires a rewrite. Phases 1–2 are where the order-of-magnitude lives.

---

## Phase 0 — Instrument first (≈ 1–2 days)

You cannot hold gains you cannot see. The stack currently has **zero benchmarks and zero GPU timing** (overview Finding 15).

1. **Keystroke-to-photon tracing**: a `tracing` span opened at input-event receipt (`window_events.rs`) carrying a monotonically increasing input id, propagated through the input channel → evaluator → `publish_gui_frame` (stamp the id into `FrameDisplayState`) → materialize → `present()`. Emit one summary line per frame under an env flag. This is *the* editor latency metric; everything below should be judged by it.
2. **GPU timestamp queries** around the main pass and each overlay/transition pass (`wgpu::Features::TIMESTAMP_QUERY`, guard on adapter support). Report via the existing `NEOMACS_RENDER_STATS` channel.
3. **CPU phase timers** (cheap `Instant` pairs, same reporting): layout per window, status-line eval, grid build, materialize, clone(s), vertex build, submit.
4. **Criterion benches** (`benches/` in the relevant crates): (a) `layout_window_rust` over a 10k-line rope with mixed faces; (b) `FrameDisplayState::materialize` at 120×40 and 200×60; (c) glyph vertex build for 10k glyphs; (d) `char_width` hot loop. Wire into CI as a tracked (not gating) job.
5. **Size regression tests** in the protocol crate: `assert!(size_of::<FrameGlyph>() <= 112)`, same for `Glyph`, `Face`, `EffectsConfig` — so growth is a conscious decision, not an accident.

Exit criterion: a one-page dashboard (even a text table) showing per-phase cost for: cold full frame, keystroke frame, scroll frame, idle blink frame.

## Phase 1 — Stop paying the dumb taxes (≈ days; no architecture changes)

Every item here is a deletion, a guard, or a reuse. Together they remove the majority of avoidable per-frame CPU. Individual expected effects are relative to the Phase-0 baseline; measure each.

1. **Delete the dead clone** (`render_pass.rs:398-406`): clone #2 of the frame plus its `apply_extra_spacing` pass is overwritten unread at `:452` (verified by hand). Pure deletion.
2. **Stop cloning to read hints** (`render_pass.rs:395`): `current_frame_clone()` deep-clones the whole buffer to read `effect_hints` and a bool. Read the fields through a borrow, or copy out just the hints.
3. **Make "take" take** (`frame_windows.rs:622-628`): `take_current_frame_for_render` clones because blink re-renders need the frame to persist. Replace with `Arc<FrameGlyphBuffer>` (render never mutates the buffer; `apply_extra_spacing` can become a draw-time transform or copy-on-write only when spacing ≠ 0), or re-materialize on blink (blink frames are rare). Either removes a ~MB memcpy per frame.
4. **Guard `resize()`** (`renderer/mod.rs:1612`): early-return when `(width, height, scale_factor)` are unchanged; keep the explicit resize path for real size changes. Also remove the resize-back dance in `render_pass.rs:610-611` by passing the target size into the render call instead of mutating global renderer state. Removes 2× surface reconfigure + 2× full-screen Stencil8 allocation per frame.
5. **Drain, then materialize** (`frame_ingest.rs:260-273`): drain the frame channel keeping only the newest `FrameDisplayState` per frame-id, then materialize once. Removes N−1 full materializes under redisplay bursts.
6. **Cache the face map** (`frame_state.rs:32-94`): rebuild `faces` only when a frame actually delivers new faces (key on frame identity/face epoch), not on every render; stop `Face::clone()`-ing 248 B per face per frame.
7. **Skip the subpixel build when subpixel is off** (`glyphs.rs:2379-2384,2511-2522`): branch once per frame on the render mode; stop constructing 6 × 48 B per glyph that is discarded. Similarly, collapse the vertex → tuple → flat_map → write chain (`:2616-2619`) to build directly into the arena's mapped range (one copy instead of three).
8. **Move `EffectsConfig` off the frame payload** (`glyph_matrix.rs:1140`; `glyphs.rs:2037-2040`): it is configuration (3,576 B × windows × frames today). Send it once on the command channel when it changes; keep only a small per-window "active effect ids" hint in the frame. Also fixes the renderer-side per-frame deep clone.
9. **Un-block the input path** (`thread_comm.rs:1023`): never block inside the winit callback. Options: grow to unbounded with a high-water warning; or try_send + render-thread-local overflow queue flushed on the next pump tick. Key/button events must be lossless *and* the UI thread must never stall on the evaluator.
10. **Clamp animation ticks to presentation** (`lifecycle.rs:306-323`): when the only reason to wake is animation, wake at the display's refresh cadence (track last-present time; wgpu Fifo already paces presents) instead of a fixed 4 ms. Fix the idle-dim hardcoded 16 ms step (`frame_windows.rs:712-713`) to use real dt.
11. **Font-metrics key allocation** (`font_metrics.rs:242,688`): intern `(family, weight, italic, size)` per resolved face once (a `FaceMetricsId`), key all metrics caches on the id — removes a `String` allocation per measured character. While there: return `&[ShapedGlyph]` (or `Arc<[ShapedGlyph]>`) from the shape-run cache instead of `clone()`, and replace the clear-all overflow (`:491-494`) with LRU.
12. **Delete debug cruft from hot paths**: the per-glyph `format!` band scan (`glyphs.rs:1109-1150`), the per-iteration `crossterm::size()` ioctl in TTY input (`tty_input.rs:304` — cache and refresh on SIGWINCH).

## Phase 2 — Make frames incremental (the order-of-magnitude phase)

The principle: **arbitrary Lisp means you cannot predict invalidation, but you can detect it cheaply after the fact.** GNU solved this in 1990 with current/desired matrices; Neomacs already computes the modern equivalent (row content hashes) and throws it away on the GUI path. Wire it through, at three levels:

1. **GUI row-hash damage diff (render side first — no layout changes needed).**
   Keep the previous frame's per-row hashes per window (they are already in `FrameDisplayState`, `glyph_matrix.rs:252`). On a new frame: diff row hashes; produce a damage set (changed rows ∪ moved-row ranges ∪ cursor rows ∪ side-item deltas).
   - Materialize **only damaged rows** into the flat form (or better: materialize per-row lazily and keep per-row `FrameGlyph` ranges).
   - Rebuild vertices **only for damaged rows**; keep per-row instance/vertex ranges in the arena from the previous frame and patch in place (`queue.write_buffer` at row offsets).
   - Draw: either keep the full-clear draw (cheap once geometry is retained — Zed proves full repaint is fine when per-frame CPU is tiny) or scissor to damage rects; decide by measurement, not ideology.
   - Scroll becomes a row-range move: same hashes at new y → adjust y in-place (or a per-row y-offset uniform), no re-rasterization, no re-materialize.
2. **Mode/header/tab-line cache (layout side; biggest per-redisplay Lisp win).**
   Key: (format value identity, buffer modification tick, point line/column when the format uses them, window width, selected-ness, face epoch). Value: the rendered chrome row (glyph run). Invalidate on `force-mode-line-update` (wire the existing Lisp entry point to bump a per-window chrome epoch). Expected effect: the ~4.3 ms Doom mode-line eval (`display_status_line.rs:56`) drops out of the steady-state keystroke path entirely. GNU does a version of this; parity requires care with `:eval` forms that read volatile state — the buffer-tick + point keys cover the overwhelmingly common cases, and the cache can be bypassed for formats detected to contain `:eval` reading un-keyed state (start conservative: cache only when the eval count for the window is stable across two identical keys).
3. **Per-window layout row cache (layout side; makes layout itself incremental).**
   Key: (buffer id, buffer tick, row start byte, window text width, face epoch, hscroll, relevant display options). Value: the built row (grid `GlyphRow` + walk-state deltas needed to resume). On redisplay: rows whose keys match are reused without walking; the walk runs only from the first damaged position (GNU's `try_window_id` insight, done with hashes instead of matrix bookkeeping). Single-char edits then re-lay-out ~1 row (+ continuation rows); scrolling re-lays-out ~the newly exposed rows.
   With (3) in place, "layout blocks Lisp" mostly stops mattering because layout of an unchanged window is a hash check — cheaper than moving layout off-thread, and semantically safe (still synchronous with the Lisp state).
4. **Windows-level gating**: skip whole windows whose (buffer tick, window-start, point-row, geometry, face epoch) tuple is unchanged — a per-window `RedisplaySignature`. Today the signature is frame-global; per-window granularity means a minibuffer echo doesn't re-lay-out the code window above it.

Ordering within Phase 2: (1) is self-contained on the render thread and pays immediately; (2) is self-contained in the layout crate; (3) is the deepest change; (4) falls out of (3)'s keys. After Phase 2, steady-state typing does O(changed rows) work end-to-end.

## Phase 3 — Modernize the GPU submission

1. **Instanced glyph rendering.** One static unit quad (4 verts / index buffer) + a per-glyph instance record `{pos: [f32;2], size: [f32;2], uv_origin: [u16;2], uv_size: [u16;2], color: u32, flags: u32}` ≈ 24–32 B — versus 6 × 32 B (grayscale) or 6 × 48 B (subpixel) unique vertices today. ~6× less vertex data, ~6× less build work, and — the real prize — **per-row instance ranges become the retained unit** for Phase 2's damage patching. Rect/border/cursor quads collapse into the same instanced path with a different flag. This is the single biggest renderer refactor and it simplifies code rather than complicating it (deletes the tuple/flat_map chain).
2. **Arena everything.** Extend `FrameVertexArena` (or the new instance arena) to overlays, effects, images, cursors, backgrounds — delete every per-frame `create_buffer_init` (renderer report §3). Fold all overlay encoders into the main encoder: **one `queue.submit` per frame** (except transitions).
3. **Fix text color math.**
   - Grayscale: move to premultiplied alpha (`src = One`), blend in linear space (sample sRGB texture as UNORM-sRGB view so hardware linearizes; or do the pow once, not twice), delete the double alpha application (`glyph.wgsl:46-50`). Validate perceptually against the current hand-tuned output before switching defaults.
   - Subpixel: request `wgpu::Features::DUAL_SOURCE_BLENDING` where available and emit per-channel coverage as the second source (`@blend_src(1)`) — correct subpixel text over *any* background, no more per-glyph assumed-bg rectangles (`glyph_subpixel.wgsl:46-55`). Fallback where the feature is missing: current behavior over solid backgrounds, grayscale over non-solid (what browsers do).
   - Add gamma-aware blending to the subpixel path (the current linear-in-sRGB interpolation is the classic fringing bug).
4. **Present-mode option**: keep Fifo default; expose Mailbox (`neomacs-render-present-mode`) for latency-sensitive users; keep `desired_maximum_frame_latency = 2` (or 1 under Mailbox).
5. **Media zero-copy by default.** Turn the implemented Vulkan DMA-BUF import (`vulkan_dmabuf.rs`) on for video and WebKit with a driver denylist for the known RADV fd-leak (instead of a global off switch); keep the CPU path as fallback. Add PTS-based video pacing (schedule texture flips against the presentation clock rather than "latest frame at tick"). Fix image-cache eviction to true LRU with on-screen pinning (`image_cache.rs:1117-1133`); bound the video/WebKit caches; wire or delete `media_budget.rs`.
6. **Small correctness/perf sundries**: replace the per-glyph page-pin linear scan (`pages.rs:313-326`) with a page-generation check; add a tiny per-row run cache in front of the atlas HashMap (text is ~identical frame-to-frame; after Phase 2 this is mostly moot but still helps cold rows); `PipelineCache` where supported for faster startup.

## Phase 4 — Architectural endgame (only with profiles in hand)

1. **Split status-line eval from the row walk, then parallelize window layout.** The row walk is pure data (interval trees, metrics caches — `&mut Context` is needed today mostly for mode-line eval and fontification triggers). Hoist per-window Lisp (chrome eval, `ensure_fontified` targets) into a pre-pass on the Lisp thread, then lay out window bodies in parallel with rayon over immutable views. Only worth it for many-window frames; measure first.
2. **Off-thread layout via snapshots — only if still needed.** With Phase 2's row caches, layout cost is O(damage) and synchronous layout stops being the bottleneck; the remaining motivation would be very large single-row damage (huge lines). The design (buffer rope snapshot + face/overlay epoch, layout on a worker, Lisp continues) is a big semantic step (GNU redisplay is synchronous with Lisp state) — treat as a research phase, not a commitment.
3. **Delete the dead weight** (can happen any time; listed here so it isn't forgotten): protocol `scene.rs` retained graph (choose the row-hash path and delete it — don't finish two competing designs), `DisplayBackend`/`BackendType` + runtime `TtyBackend` (the live TTY is `TtyRif`), runtime `core/{animation,cursor_animation,buffer_transition,animation_config}.rs`, `core/profiler.rs`, `media_budget.rs`, `gradient.wgsl` + `texture.wgsl`, `va_dmabuf_export.rs`, layout `display_iterator.rs` (per the 2026-06-21 deletion plan), the dead crossfade `_vertex_buffer` (`transitions.rs:72`), stale C-era comments (`frame_glyphs.rs:1-5,767-768`; `transition_policy.rs:32`), and the `unicode-script` dead dependency. Rename `tty_layout.rs` → `frame_layout.rs` (it is the shared GUI+TTY entry point).
4. **Typed identity**: replace `emacs_frame_id: 0`-means-primary and bare-`i64` window ids with newtypes carrying an explicit `Primary` variant.
5. **Latin ligature shaping (quality, not speed)**: route same-face Latin runs through `shape_run` behind a `neomacs-enable-ligatures` option (per-char advances are also a correctness choice for column math — Emacs semantics assume per-char cells; ligatures need a design decision about column mapping, which is why this is endgame, not Phase 1).
6. **Fontconfig fallback cache**: key entries by (fontset generation, …) as today but keep per-generation maps so a fontset change doesn't cold-start everything; prune old generations; call the (currently dead) `clear_caches` from the actual font-change/text-scale events, or delete it and rely on keyed caches with real LRU bounds.

## What NOT to do (considered and rejected)

- **Compute-shader / vello-style glyph rasterization.** Editor workloads are tens of thousands of small quads from a warm atlas; CPU rasterization (swash) into an atlas is what Zed, Ghostty, Alacritty, and every browser do. Compute rasterization buys generality Neomacs doesn't need and costs portability (wgpu limits/features variance) and debuggability. The bottleneck was never rasterization.
- **WebRender adoption.** emacs-ng proved it works and also proved the dependency weight. The bespoke renderer is already at 1–3 draws per screen; retained display lists would duplicate Phase 2 at higher integration cost.
- **A separate display-server process.** In-process moved values are currently a zero-cost boundary; a process split adds serialization (today: none) for a multi-client feature nobody has designed. If remote display ever becomes a goal, the grid protocol + row hashes are the right wire format — design versioning then.
- **Dropping Fifo/vsync by default.** Tearing/latency tradeoffs belong to users; Mailbox as an option (Phase 3.4) is enough.
- **Finishing `scene.rs`.** Two retained designs is one too many; the grid already is the retained model. Delete, don't complete.

## Sequencing and expected outcomes

| After | Steady-state keystroke path | Expected order |
|---|---|---|
| Phase 0 | unchanged, but measured | baseline numbers exist |
| Phase 1 | full relayout + **one** materialize + **zero** dead clones + guarded resize + lean vertex build | ~2–4× less render-thread CPU/frame; mode-line still dominates Lisp side |
| Phase 2 | signature → O(changed rows) layout → row diff → patch upload → draw | ~10×+ on typing/scroll; mode-line eval out of the loop; idle truly idle |
| Phase 3 | same, but instanced + single submit + correct text blending + zero-copy media | lower per-frame floor, correct subpixel everywhere, 4K video without CPU copies |
| Phase 4 | parallel/off-thread layout only if profiles demand; codebase −several dead subsystems | headroom + maintainability |

The honest summary: **Phases 0–2 are the product.** Phase 3 makes it beautiful engineering; Phase 4 is optionality. A working editor that lays out one row per keystroke and patches a few KB of instances will feel instant on any GPU made in the last decade — which is the point of "GPU-fast": not doing more work on the GPU, but doing almost no work anywhere.
