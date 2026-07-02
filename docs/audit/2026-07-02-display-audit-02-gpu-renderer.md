# Display Audit 02 — GPU Renderer (`neomacs-renderer-wgpu`)

**Date**: 2026-07-02 · Part of the [display stack audit](2026-07-02-display-audit-00-overview.md).
**Scope**: `neomacs-renderer-wgpu/` plus the driving orchestration in `neomacs-display-runtime/src/render_thread/render_pass.rs`.
**Stack**: wgpu **29.0.3**; swash 0.2.7 (glyph rasterization); cosmic-text 0.18.2 (eval-exec fork — shaping); fontdb 0.23; resvg 0.47; ash 0.38 (Vulkan DMA-BUF); image 0.25.

---

## 1. Frame lifecycle: frame received → present

Two paths, selected by `need_offscreen` (`render_pass.rs:415-421`) — **false in the common case** (true only for an active transition policy or a `ThemeTransition` hint).

**Common path (no transition):**

1. `surface.get_current_texture()` → `output`; create `surface_view` (`render_pass.rs:426-464`).
2. **`renderer.resize(native.w, native.h)` unconditionally** (`:468-469`) → reconfigures the surface, recreates the full-screen Stencil8 texture, rewrites the uniform buffer (`mod.rs:1612-1643` — **no change-guard**).
3. Drain the materialized `FrameGlyphBuffer` (`:449-452`) — after two prior deep clones (see runtime report §7).
4. `render_frame_window_contents(...)` → `render_frame_root_glyphs` → **`renderer.render_frame_glyphs()`** (`render_pass.rs:580-595,:802`): the whole scene in **one encoder, one render pass with `LoadOp::Clear`, one `queue.submit()`** (`glyphs.rs:1999-2003,2009,3768`).
5. `detect_frame_transitions()` (`:596-606`).
6. Overlays: `render_frame_content_overlays` (scroll indicators, breadcrumbs, tooltips, IME, watermark, FPS) then `render_frame_chrome_overlays` (menu/compact bar, toolbar) (`:657-684`). **Each overlay = its own encoder + its own `queue.submit()`** (`ui_overlays.rs:217,487,…`).
7. **`renderer.resize(old_w, old_h)`** — resize *back* (`render_pass.rs:610-611`): a second full resize per frame.
8. `output.present()` (`render_pass.rs:989`).

**Transition path:** toggle `current_is_a` (offscreen A/B double buffer), `ensure_frame_offscreen_textures` (reused; allocated only when `None` — `transitions.rs:353-372`), render into offscreen A/B, `blit_texture_to_view` → surface (`render_pass.rs:549`, `mod.rs:2336`), then `render_frame_transitions` composites previous+current as extra surface passes — one encoder+submit each (`transitions.rs:80-202`) — then overlays, then present. A/B textures are transition-only, not a per-frame cost.

**Z-order inside the single main pass** (`glyphs.rs:1096-1107`): non-overlay backgrounds → pre-content background effects → pre-content effects → cursor-background/trail → `for overlay_pass in 0..2 { text build + draw }` → inline images/videos/webkit → front cursors/borders.

## 2. Pipeline inventory

**14 `wgpu::RenderPipeline`s, all built once** in `create_renderer_internal` (`mod.rs:628-1390`), never per-frame:

| Pipeline | Shader | Blend | Purpose |
|---|---|---|---|
| `rect` | rect.wgsl | ALPHA_BLENDING | solid quads (backgrounds, effects, borders) |
| `rounded_rect` | rounded_rect.wgsl | ALPHA | rounded corners, animated border styles |
| `corner_mask` | rounded_rect.wgsl | dst *= src_alpha | child-frame corner masking |
| `glyph` | glyph.wgsl | ALPHA | grayscale/color text |
| `subpixel_glyph` | glyph_subpixel.wgsl | **None (opaque)** | LCD subpixel text |
| `image` | image.wgsl | ALPHA | images/video/webkit quads |
| `opaque_image` | image.wgsl `fs_main_opaque` | ALPHA | opaque media |
| `stencil_write` | (color write mask empty) | — | child-frame clip write |
| 6 × `stencil_*` read variants | — | — | child-frame rounded-corner clipping |

- MSAA off everywhere (`sample_count: 1`). `cache: None` on all pipelines (no `PipelineCache`).
- Transitions reuse `image_pipeline`.
- Passes per common frame: **1 main + 1 per visible overlay** (each a separate submit). Transition frames add 1 blit + N transition passes + snapshot copies.

## 3. Draw-call structure

- **All geometry is CPU-built into `Vec`s each frame. No instancing. No index buffers.** Everything is `draw(0..n, 0..1)` triangle lists — 6 unique vertices per quad.
- **Main text is well batched**: per glyph, 6 `GlyphVertex` are built (`glyphs.rs:2436-2467`), bucketed into mask/subpixel/color (`:2542-2557`), then per bucket `flat_map().collect()` → one `arena.upload()` → **one draw per consecutive atlas-page run** (`:2668-2699`), with a bind-group change only on page switch (`:2695`). A full screen of text ≈ **1–3 draw calls**.
- **`FrameVertexArena` is the good pattern** (`dynamic_buffer.rs:55-113`): persistent, bump-allocated, ×2-growable GPU buffer written via `queue.write_buffer`; `begin_frame()` resets the cursor. Used by the main text path (`glyphs.rs:1044-1045,2668-2674`) and the child-frame content path (`content.rs:1123-1149`).
- **Everything else bypasses the arena** and calls `create_buffer_init` per draw per frame: backgrounds / overlay-rects / decorations / borders / cursor in the main pass (`glyphs.rs:2080,2124,3148,3306,3321,3716,3746,4042,4078,4358`), **every inline image** (`glyphs.rs:3433`, `content.rs:1342`), and **every UI overlay** (15+ sites: `ui_overlays.rs:183,424,1256,2214,2736,…`). A declared `image_vertex_arena` (`mod.rs:166`) exists but is used only by toolbar icons.
- Vertex formats (`vertex.rs`): Rect 24 B, Texture 16 B, Glyph 32 B, SubpixelGlyph 48 B, RoundedRect 72 B; Uniforms 16 B.

## 4. Glyph atlas

- **Rasterizer = swash** via a held `ScaleContext` (`glyph_atlas/mod.rs:181,244`; render at `:708-723`), reached through the cosmic-text fork. Grayscale (`Format::Alpha`) and LCD subpixel (`Format::Subpixel`) both supported; the mode is a per-request decision gated on fontconfig (`:956-962,702-706`).
- **Three atlas families**, 2048² pages, ≤8 pages each (≤24 textures total): `AlphaMask = R8Unorm`, `SubpixelMask = Rgba8Unorm`, `ColorRgba = Rgba8UnormSrgb` (`types.rs:40-53,524-526`; textures created in `pages.rs:67-80`).
- **Packing**: shelf allocator (row-based, no intra-page free list; `allocator.rs:29-114`).
- **Subpixel positioning is real**: the cache key includes quarter-pixel `x_bin`/`y_bin` (up to 4×4 rasters per glyph per size), computed as `SubpixelBin::new(phys * scale)` (`glyphs.rs:2202-2203`) and fed to swash as a render offset (`mod.rs:693-700`); vertex positions stay integer. The key also includes `face_id`, `font_size_bits`, `font_identity: u64`, and render mode. Color is **not** in the key — masks are tinted per-vertex (correct design).
- **Eviction exists and is bounded**: page-level LRU of unpinned pages (`mod.rs:1051-1190`, `pages.rs:333-340`) with generation-stamped invalidation and per-frame pinning; the composed/emoji cache is capped at 1024 entries with 60-frame recency (`mod.rs:938-949`). The single-glyph entry map has no count cap of its own but is bounded via the page budget.
- **Color/emoji**: COLR/CPAL and embedded color bitmaps (`Source::ColorOutline` / `ColorBitmap`, `mod.rs:708-712`) rendered into the sRGB RGBA atlas. ZWJ clusters go through a `ComposedGlyphKey` path (`mod.rs:499-649`). **OT-SVG font glyphs are not handled** (no SVG source).
- **Upload**: `queue.write_texture` per glyph, on cache miss only, no staging belt (`mod.rs:994-1026`) — fine, misses are rare in steady state.

## 5. Text-quality path — two inconsistent color models

- **Grayscale (`glyph.wgsl:46-50`)**: samples `.r` coverage, applies manual `pow(2.2)` / `pow(1/2.2)` gamma around `fg * alpha`, returns `(result_srgb, alpha)`. The target is an sRGB surface (`mod.rs:1564-1571`) with `ALPHA_BLENDING` (`src = SrcAlpha`). The fragment pre-multiplies rgb by coverage **and** the blend multiplies by SrcAlpha again → **alpha is effectively applied twice**, on top of manual gamma over a hardware-sRGB target. The shader comment says it is hand-tuned; the result may look acceptable, but the math is non-standard. A premultiplied pipeline (`src = One`) with linear-space blending would be canonical. Perceptual impact unmeasured — flagged for verification.
- **Subpixel (`glyph_subpixel.wgsl:46-55`)**: per-channel RGB coverage, composites against a **per-vertex `bg_color`**, returns opaque `(rgb, 1.0)` with **blending disabled** (pipeline blend `None`, `mod.rs:929`). Two consequences: (i) **no gamma correction at all** — linear interpolation of sRGB values produces the classic subpixel darkening/fringing; (ii) the result is correct **only where the actual pixel behind the glyph equals the passed `bg_color`** — subpixel text over images, gradients, or effects paints a rectangular background patch. The background is sampled per glyph (`glyphs.rs:2345-2354,2384`). The industry answer is dual-source blending (see roadmap Phase 3).
- Which path dominates at runtime is a fontconfig decision (`subpixel_enabled()`), not a compile-time default.

## 6. Surface configuration

`mod.rs:1393-1408`: usage `RENDER_ATTACHMENT`; first available **sRGB** surface format (`:1564-1571`); **`present_mode = Fifo`** (hard vsync — no Mailbox/Immediate option, not configurable); `alpha_mode = Auto`; `desired_maximum_frame_latency = 2`; no MSAA. Device requested with **no features and default limits** (`:1551-1552`) — relevant later because dual-source blending is an optional feature. Power preference from `NEOMACS_GPU`, default `HighPerformance` (`lib.rs:49-55`).

**Resize** (`mod.rs:1612-1643`): no `(width, height)` change guard; every call reconfigures the surface, recreates the full-screen Stencil8 texture, and rewrites the uniform. Called **twice per rendered frame** (`render_pass.rs:469,611`). Whether `surface.configure` with identical parameters is cheap in wgpu 29 is driver-dependent; the stencil-texture recreation is unconditionally real work. Verified by hand; highest-confidence easy fix in the renderer.

## 7. Full-frame vs partial

**Full clear + full geometry rebuild every frame.** The main pass clears with the (premultiplied) background (`glyphs.rs:2015-2021`); the code comments state the contract: *"rebuild the entire frame … no incremental updates"* (`:2005-2006`). **No scissor rects, no damage regions, no dirty tracking, no row/tile caching.** The child-frame path uses `LoadOp::Load` to composite (`content.rs:1014,1034`) but still rebuilds all its geometry. Redraw *scheduling* is gated upstream (dirty flags), but each actual redraw regenerates everything from the glyph buffer.

## 8. Effects, transitions, media

- **Effects create no pipelines or textures**: the ~130 cursor/window/pattern effects are pure `Vec<RectVertex>` builders drawn through the shared `rect_pipeline` as extra draws inside the main pass (`glyphs.rs:2057-2059,3757`; macros `:33-88`). Idle cost is near-zero for the emit functions themselves (they early-return empty Vecs) — but a **full `EffectsConfig` deep clone (3,576 B, owns Vecs) happens every frame regardless of activity** (`glyphs.rs:2037-2040`).
- **Active effects**: each builds a fresh Vec + a fresh `create_buffer_init` per effect per frame (`glyphs.rs:37-43`) and latches `needs_continuous_redraw` → sustained full-frame redraws at the 4 ms tick. Several are O(windows × glyphs) scans rebuilt each frame (minimap `window_effects.rs:1652,1674`; header-shadow `:1751,1759`). Pattern effects run full-grid `sin/cos/sqrt` per cell on the CPU (`pattern_effects.rs:539-548`). Animated rounded-border styles run heavy per-fragment math (3-octave fbm, `exp`, `atan2`, multiple SDF evaluations — `rounded_rect.wgsl:107-385`).
- **Transitions**: correct reuse of offscreen A/B; one dead per-frame buffer (`transitions.rs:72` `_vertex_buffer`).
- **Images**: async pool decode → single `write_texture`; texture+view+bind group cached once per image (`image_cache.rs:73-81,1047-1070`). Budget 64 MB but **eviction is FIFO-by-lowest-id, not LRU** (`:50,1117-1133`) — it can evict an on-screen image while keeping an off-screen newer one. Per frame: a fresh vertex buffer per image (`glyphs.rs:3433`, `content.rs:1342`) and an O(n) eviction scan even when idle (`:1022-1027`).
- **Video**: GStreamer; bounded `sync_channel(4)`; coalesced to one frame per video per tick (`video_cache.rs:404-425`); texture+bind group reused; steady state is one `write_texture` per frame (`:624`) **plus a full-frame `to_vec()` CPU copy per frame** (`:984`). The pipeline is a **by-design GPU→CPU→GPU round trip**: VA-API decodes and converts on the GPU, the frame is downloaded, then re-uploaded (`:856-869`, RADV sync-fd workaround). **Video cache unbounded.**
- **DMA-BUF zero-copy is fully implemented** (ash external-memory-fd import + `create_texture_from_hal`, `vulkan_dmabuf.rs:335-573,607-611`; packed-RGB only, NV12/YUV rejected `:133-145`) but **dead in the video path** (`dmabuf_info = None`, `video_cache.rs:963-968`; `va_dmabuf_export.rs` has no live callers). The only live consumer is WebKit under the `wpe-webkit` feature, itself defaulting to CPU pixels. **WebKit cache unbounded**; its `last_updated` field is written but never read.

## 9. Hot-path hygiene

- **Per-glyph atlas HashMap lookup every frame**: `get_or_create_atlas`/`_composed` once per glyph (`glyphs.rs:2258`, `content.rs:482`), hashing a ~28 B key; on hit, `pin_entry_page` does a **linear scan over pages** (`pages.rs:313-326`). Fully redundant frame-to-frame for unchanged text — nothing caches at the run or row level.
- **Subpixel vertices and fg/bg colors are built for every glyph unconditionally** (`glyphs.rs:2379-2384,2511-2522`) even when the subpixel path is disabled and the data is discarded — 6 × 48 B per glyph of wasted build.
- **Redundant vertex copies**: vertices → `mask_data` tuples → a second `flat_map().collect()` Vec (`:2616-2619`) → `write_buffer` staging ≈ **three copies of the full glyph set per frame**, ×2 via the overlay-pass loop.
- **~15–20 full scans of `frame_glyphs.glyphs` per frame**: box spans, backgrounds, rows/scrollbars, overlay rects, cursor background, stats, the 2× main build passes, image/video/webkit/decoration scans.
- **Leftover debug work**: an unconditional per-glyph `format!` scan near a y-band (`glyphs.rs:1109-1150`).
- **Overlays**: each rewrites the uniform, re-emits its glyphs, `create_buffer_init`s, and submits a separate command buffer — a dozen+ submits per frame with chrome visible.

## 10. What the renderer does NOT do (modern-GPU gaps)

- No instancing (a per-instance quad + storage/instance buffer would cut vertex bytes ~6× and make row-level caching of instance ranges natural).
- No index buffers; no compute shaders; no indirect draws; no cross-pipeline batching.
- No damage/scissor/dirty-rect path; no glyph-run/row/tile caching.
- No Mailbox/Immediate present option; no `PipelineCache`.
- Two dead shaders compiled by nobody: `gradient.wgsl` (contains invalid nested-fn WGSL) and `texture.wgsl`.

## 11. CPU cost estimate — 200×60 screen (~10k Char glyphs)

GPU submission is cheap: text batches to ~1–3 draws; ~10–30 draws total; 1 main submit + a few overlay submits. The CPU side dominates and is fully regenerated each redraw:

- Glyph-vector scans: ~15–20 × 10k ≈ **150k–200k glyph visits/frame** (enum match + float ops each).
- Main build loop: ~10k × { face-cache probe, 2 × `SubpixelBin`, atlas HashMap lookup + ≤8-page pin scan, build 6 GlyphVertex **and** 6 SubpixelGlyphVertex, compute subpixel fg/bg } → ~10k atlas lookups, **~120k vertices built** (half discarded when grayscale).
- Vertex bytes moved: mask ≈ 10k × 6 × 32 B ≈ 1.9 MB, + flat_map copy 1.9 MB, + write_buffer staging 1.9 MB ≈ **~5.7 MB copies/frame**, plus ~2.9 MB subpixel built-and-dropped when grayscale.
- Heap allocations/frame: dozens of Vecs (box spans, backgrounds, overlay rects, cursor, mask/subpixel/color data ×2 passes, two flat_map collects, rendered_char_bounds, EffectsConfig clone), several in the 1–2 MB range.
- Plus ~15+ `create_buffer_init`, 2 × `resize()` (2 stencil allocs + 2 surface configures).
- With any continuous effect active, all of this repeats **every vsync**.

## 12. Renderer smells, ranked

1. `resize()` ×2/frame, unguarded (`render_pass.rs:469,611`; `mod.rs:1612-1643`).
2. `create_buffer_init` per draw per frame outside main text (overlays/images/effects/bg/borders/cursor).
3. Redundant vertex copies + unconditional subpixel build (`glyphs.rs:2511-2522,2616-2619`).
4. ~15–20 full glyph scans/frame.
5. Per-glyph atlas lookup + linear page-pin scan every frame; no run/row caching.
6. Full clear + full rebuild; no damage path (`glyphs.rs:2005-2021`).
7. One encoder+submit per overlay (`ui_overlays.rs:217,487,…`).
8. Video GPU→CPU→GPU + per-frame `to_vec()`; DMA-BUF dead (`video_cache.rs:856-869,963-968,984`).
9. Cache-policy hazards: image FIFO-by-id eviction; video/WebKit caches unbounded.
10. Glyph gamma/premultiply double-apply (grayscale); no gamma + assumed-bg compositing (subpixel).
11. Per-frame `EffectsConfig` clone even when idle (`glyphs.rs:2037-2040`).
12. Dead code: `gradient.wgsl`, `texture.wgsl`, `va_dmabuf_export.rs`, `transitions.rs:72`.

### Uncertainty flags

- Subpixel-vs-grayscale dominance is a runtime fontconfig decision; which cost profile applies depends on user config.
- The grayscale gamma/alpha math is hand-tuned and unmeasured — it may be a deliberate perceptual choice; verify with side-by-side rendering before changing.
- Whether wgpu 29's `surface.configure` no-ops identical configs is driver-dependent; the stencil recreation is unconditional either way.
- Effect/transition trigger frequency is governed by the runtime crate.
