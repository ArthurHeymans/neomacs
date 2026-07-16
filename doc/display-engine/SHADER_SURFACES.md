# Shader Surfaces: GPU textures & user shaders from Elisp

Status: **experimental prototype** (stages 1–2 of 4). The Elisp API surface is
`neomacs-surface-*` and may change; gate uses on `(featurep 'neomacs-surface)`.

## Motivation

NeoMacs already composites three kinds of external GPU textures inline in
buffer text — images, video frames, WPE WebKit views — each as a textured
quad in z-order step 7 of `render_frame_glyphs`. A *shader surface* is the
fourth kind: a texture **rendered by NeoMacs itself** from a user-supplied
WGSL fragment shader (or uploaded raw pixels), owned by an Elisp-visible id,
placed in buffer flow with a `display` text property, and animated by the
compositor clock at zero Lisp cost.

No other editor exposes this: terminals ship config-file post shaders
(Ghostty `custom-shader`, Windows Terminal `experimental.pixelShaderPath`,
Rio librashader), app frameworks ship the API shape (Qt `ShaderEffect`,
Flutter `FragmentProgram`, Blender's Python `gpu` module), but a
runtime-scriptable shader object in a text editor's native display pipeline
is unoccupied territory.

## Elisp API (stage 1–2)

```elisp
;; Animated fragment-shader surface (stage 2)
(setq id (neomacs-surface-create
          :width 240 :height 120
          :shader "fn mainImage(fragCoord: vec2<f32>) -> vec4<f32> {
                     let uv = fragCoord / u.iResolution.xy;
                     return vec4<f32>(0.5 + 0.5*cos(u.iTime + uv.xyx + vec3<f32>(0.0,2.0,4.0)), 1.0);
                   }"
          :uniforms '((speed . 2.0) (tint . [1.0 0.5 0.2]))
          :animate t))

;; Static pixel texture from Lisp data (stage 1): RGBA8, row-major
(setq id2 (neomacs-surface-create :width 2 :height 2
                                  :pixels (unibyte-string 255 0 0 255  0 255 0 255
                                                          0 0 255 255  255 255 255 255)))

;; Show it inline — a standard display property, like video/webkit
(insert (propertize " " 'display (list 'surface :id id :width 240 :height 120)))
;; or the convenience wrapper:
(neomacs-surface-insert id 240 120)

(neomacs-surface-set-uniform id 'speed 3.5)   ; cheap; no recompile
(neomacs-surface-destroy id)
```

Bad WGSL signals a Lisp `error` **synchronously from
`neomacs-surface-create`** with naga's rendered diagnostics — the shader
playground loop (edit WGSL buffer, `C-c C-c`, see error or live surface) works
with ordinary `condition-case`.

### Declarative form (spec-is-the-identity, like image/video)

```elisp
(insert (propertize " " 'display
  '(surface :shader "fn mainImage(fragCoord: vec2<f32>) -> vec4<f32> { ... }"
            :uniforms ((speed . 2.0))
            :animate t
            :width 320 :height 120)))
```

No create call, no id: the resolver memoizes the spec content into a host
surface id exactly like `(video :file …)` (`DisplayHost::request_surface`,
`resolved_surfaces` memo in `main.rs`). Trade-offs versus the imperative API:

| | imperative (`:id`) | declarative (`:shader`) |
|---|---|---|
| WGSL errors | synchronous Lisp `error` | logged once, spec renders nothing |
| `set-uniform` | yes (same id, no recompile) | no — new uniform values = new spec = new surface |
| lifecycle | explicit `destroy` (or buffer-tied via `neomacs-surface-attach`) | memoized FIFO-capped at 64 entries; eviction frees the GPU objects, and an evicted-but-visible spec re-resolves on the next redisplay walk |
| use for | playgrounds, interactive uniforms | fire-and-forget decorations, modes, dashboards |

## Shader contract

User source defines:

```wgsl
fn mainImage(fragCoord: vec2<f32>) -> vec4<f32>
```

The renderer prepends a generated prelude and compiles the concatenation
(WGSL module-scope declarations are order-independent):

- `u: NeoUniforms` at `@group(0) @binding(0)`, Shadertoy-compatible names:
  - `u.iResolution: vec4<f32>` — surface size in *physical* pixels (xy; z = scale factor)
  - `u.iTime: f32` — seconds since creation, advancing **only while the
    surface is actually rendered** (pauses offscreen — free battery win)
  - `u.iTimeDelta: f32`, `u.iFrame: f32`
  - `u.iMouse: vec4<f32>` — xy = hover position in *physical* pixels
    (origin bottom-left, y-up, matching `fragCoord`) while the pointer is
    over the surface; persists at the last hover position when the pointer
    leaves. zw reserved for click state, currently 0.
  - 8 user `vec4<f32>` slots
- One accessor function per user uniform, generated from the `:uniforms`
  alist: `(speed . 2.0)` ⇒ `fn u_speed() -> f32`, `(tint . [r g b])` ⇒
  `fn u_tint() -> vec3<f32>`. Order of declaration = slot order.
- `iChannel0: texture_2d<f32>` + `iChannel0Sampler: sampler` (bindings 1–2):
  another surface bound via `:channel0 ID` (imperative or declarative) —
  pixel surfaces for image processing, shader surfaces for multipass chains
  (a chain sees the source's *previous* frame, Shadertoy buffer semantics).
  Unbound channels sample transparent black; self-reference is rejected.
- A fullscreen-triangle `@vertex` entry and an `@fragment` entry that calls
  `mainImage` with **Shadertoy fragCoord convention** (y-up, origin
  bottom-left): `mainImage(vec2(pos.x, u.iResolution.y - pos.y))`.

Color: `mainImage` returns **linear** RGBA; the target texture is the
sRGB swapchain format, so hardware encodes on store and the image pipeline
samples/blends it exactly like a decoded PNG. Alpha < 1 composites over the
buffer background (image pipeline alpha blend). Shadertoy ports that output
display-space color need `pow(c, vec3(2.2))` — documented, not hidden.

## Architecture

Mirror of the video path at every seam; the only new machinery is the
offscreen pass and runtime WGSL compilation.

| Seam | Video (template) | Surface (this feature) |
|---|---|---|
| protocol id | `VideoId` (`types.rs`) | `SurfaceId` |
| wire glyph | `FrameGlyph::Video` | `FrameGlyph::Surface` |
| matrix item | `VideoItem`, `state.videos` | `SurfaceItem`, `state.surfaces` |
| chrome | `ChromeMedia::Video` | `ChromeMedia::Surface` |
| spec head | `(video :file …)` | `(surface :id …)` (`display_spec.rs`) |
| layout item | `DisplayVideoItem` / `…Kind::Video` | `DisplaySurfaceItem` / `…Kind::Surface` |
| resolver | `resolve_video_display_property` → host request | `resolve_surface_display_property` — **no host round-trip**; the id in the spec was allocated by `neomacs-surface-create` |
| host trait | `DisplayHost::request_video` | `DisplayHost::create_shader_surface` / `set_shader_surface_uniform` / `destroy_shader_surface` (default no-ops keep TUI/tests inert) |
| command | `AssetCommand::VideoCreate/…` | `AssetCommand::SurfaceCreate/SurfaceSetUniform/SurfaceFree` |
| GPU cache | `VideoCache` (upload per frame) | `ShaderSurfaceCache` (**render** per frame: offscreen pass into own texture) |
| draw | `draw_inline_videos` (image pipeline) | `draw_inline_surfaces` (same pipeline; the image phase intentionally leaves pipeline+group0 bound for follow-on media phases) |
| pacing | `DemandReason::Video` ← `has_playing_videos` | `DemandReason::ShaderSurface` ← `has_active_shader_surfaces` |

### Per-frame flow

1. `process_surface_frames` (beside `process_video_frames`, before the main
   pass): for each surface with `animate && recently_drawn`, or `dirty`
   (created / uniform changed), write the uniform buffer and encode one
   render pass (fullscreen triangle, user pipeline) into the surface's
   texture. Own encoder, submitted before the main pass samples it.
2. `render_frame_glyphs` step 7 composites `FrameGlyph::Surface` quads with
   the image pipeline + per-surface bind group (existing
   `create_texture_bind_group` layout: texture + sampler).
3. `declare_frame_demands` submits `DemandReason::ShaderSurface` at
   `Cadence::MaxRate` while any animated surface was drawn within the last
   ~500 ms (`active_until` stamped in the draw phase). Scroll it offscreen →
   demand retracts, `iTime` freezes; scroll back → the redisplay frame marks
   it drawn and demand resumes. This is deliberately *stricter* than video's
   process-wide demand — the battery lesson from the survey.

### Validation & safety

- `neomacs-surface-create` validates synchronously on the main thread with
  naga (`wgsl-in` front end + full validator) — same crate version wgpu
  itself embeds, no device needed. Errors become Lisp errors with naga's
  span-annotated message. The render thread compiles the *same* composed
  source (shared `compose_surface_shader`), wrapped in a wgpu error scope as
  a second line of defense; a late failure marks the cache entry failed and
  the quad simply doesn't draw.
- naga guarantees memory safety, not termination: an infinite fragment loop
  can still TDR the GPU — the same accepted risk as Ghostty/Windows
  Terminal. Device-loss recovery is future work; documented, capped by:
- Dimension clamp 1..=4096 (matches `ImageCache::MAX_TEXTURE_SIZE`), uniform
  slots capped at 8, texture at physical (scale-factor) resolution.
- Lifecycle: ids are host-allocated (`next_host_surface_id`); explicit
  `neomacs-surface-destroy` frees the GPU objects (RAII drop).
  `neomacs-surface-attach` ties an id to a buffer's lifetime via a local
  `kill-buffer-hook` (and `neomacs-surface-create-and-insert` attaches
  automatically); the declarative memo is FIFO-capped at 64 entries with
  `SurfaceFree` on eviction; surface bytes are registered in `MediaBudget`
  (render thread, `asset_commands.rs`) on create and removed on free. Still
  not tied to GC, and nothing drives budget eviction yet.

## Staging

1. **DONE (this change)** — static pixels from Lisp (`:pixels`).
2. **DONE (this change)** — animated WGSL fragment surfaces: prelude,
   compositor clock, named uniforms, sync errors, visibility-scoped demand.
3. **PARTIALLY DONE** — `:channel0` surface-to-surface inputs landed (pixel
   sources + multipass chains); hover-only `iMouse` routing landed.
   Remaining: image/video/webkit ids as channel
   sources (cross-cache binding), click-state `iMouse.zw`, GLSL-in via naga
   (imports the Ghostty/Shadertoy shader corpus), full-frame post pass
   (librashader runs on wgpu — RetroArch preset ecosystem nearly free).
4. Optional 3D mesh API — raymarching already covers most demand inside
   `mainImage`; revisit only with real users.

## Known gaps (prototype)

- No GC integration: a bare `neomacs-surface-create` id that is neither
  destroyed nor attached (`neomacs-surface-attach`, automatic in
  `neomacs-surface-create-and-insert`) still lives until exit. The
  declarative memo no longer leaks (FIFO cap 64, GPU objects freed on
  eviction, evicted-but-visible specs re-resolve on the next walk).
- `MediaBudget` is accounting-only: the render thread registers shader
  surfaces (logical w*h*4 at create, removed at free) but no eviction
  driver consumes `get_eviction_candidates`, image/video/webkit caches
  never register at all, and shader bytes are really physical
  (scale-factor squared) — full parity is future work.
- DPI captured at create; no rescale on monitor change.
- `iMouse` is hover-only: zw (click/drag state) not yet routed.
- Failed *render-thread* compiles (naga-accepts/wgpu-rejects edge) log and
  blank the quad instead of surfacing to Lisp.
- TTY backend ignores surfaces (like image/video/xwidget).
- Media windows forfeit the scroll/edit incremental fast paths (full rebuild
  instead — see below); cursor-only replay carries media and stays fast.

## Two traps hit while wiring this (checklist for the next media kind)

1. **Three duplicated single-spec head lists** must all learn the new head or
   the display property is silently misclassified as a *list of specs* and
   goes inert (space reserved by nothing renders):
   `SINGLE_DISPLAY_SPEC_HEADS` (`display_spec.rs`),
   `display_spec_head_starts_list` and `display_single_spec_replacing_p`
   (`neovm-core xdisp.rs`). Symptom: the end-to-end layout test emits no
   glyph while the parse unit test passes.
2. **Media must survive incremental replays.** Media items live beside the
   rows in `FrameDisplayState` (not inside them, unlike GNU's in-matrix media
   glyphs), and the frame state is rebuilt from scratch each frame. Fast-path
   replays reinstall retained rows + faces; `RetainedWindowMedia` now carries
   the window's media the same way (re-installed verbatim on cursor-only
   replay; scroll/edit escalate to a full rebuild for media windows until
   media coordinate shifting lands). Without this, a surface went blank one
   redisplay after creation. Long-term fix: attach media to rows (GNU model).
