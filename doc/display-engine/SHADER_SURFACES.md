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
            :fps 30           ; optional: cap animation to 30 Hz (battery)
            :width 320 :height 120)))
```

`:fps N` caps a surface's animation rate: it re-renders at most N times/sec
(with `iTime` still advancing in real wall-time, so motion plays at correct
speed — just fewer frames), and when shader surfaces are the only compositor
demand, the frame loop itself idles down to the highest active cap instead of
running at display refresh. Omit it (or pass a non-positive value) for the
full display-rate behavior. Works on both forms (`neomacs-surface-create
:fps 30` too).

No create call, no id: the resolver memoizes the spec content into a host
surface id exactly like `(video :file …)` (`DisplayHost::request_surface`,
`resolved_surfaces` memo in `main.rs`). Trade-offs versus the imperative API:

| | imperative (`:id`) | declarative (`:shader`) |
|---|---|---|
| WGSL errors | synchronous Lisp `error` | logged once, spec renders nothing |
| `set-uniform` | yes (same id, no recompile) | no — new uniform values = new spec = new surface |
| lifecycle | GC-managed handle (a dropped handle frees the surface at the next GC); explicit `destroy` frees now; buffer-tied via `neomacs-surface-attach` | memoized FIFO-capped at 64 entries; eviction frees the GPU objects, and an evicted-but-visible spec re-resolves on the next redisplay walk |
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
    leaves. zw = click state (Shadertoy semantics): while a mouse button is
    held after pressing over the surface, zw = the press position (same
    mapping as xy) with positive values; after release, zw keeps that
    position negated ("not pressed; last click was here"); zw = 0 until the
    first click ever. Deliberate difference from Shadertoy: Shadertoy
    freezes xy while no button is pressed (xy only tracks drags), whereas
    NeoMacs keeps updating xy on hover — drag detection (`iMouse.z > 0`)
    and click positions (`abs(iMouse.zw)`) port unchanged, but shaders that
    relied on xy standing still between clicks will see it follow the
    pointer.
  - 8 user `vec4<f32>` slots
- One accessor function per user uniform, generated from the `:uniforms`
  alist: `(speed . 2.0)` ⇒ `fn u_speed() -> f32`, `(tint . [r g b])` ⇒
  `fn u_tint() -> vec3<f32>`. Order of declaration = slot order.
- `iChannel0: texture_2d<f32>` + `iChannel0Sampler: sampler` (bindings 1–2),
  bound via `:channel0` (imperative or declarative) accepting:
  - a **surface id** — pixel surfaces for image processing, shader surfaces
    for multipass chains (a chain sees the source's *previous* frame,
    Shadertoy buffer semantics); self-reference is rejected;
  - an **`(image …)` spec** — resolved through the async image catalog
    (samples black until decoded, then picks up automatically);
  - a **`(video …)` spec** — the video's current frame, zero extra copies
    (the texture is the same one playback uses, DMA-BUF zero-copy included);
    `:autoplay` defaults to t in channel position (a never-playing channel
    samples black forever). Consumers that should follow the video need
    `:animate t`.
  Channels resolve per pass, so late creation / decode completion /
  per-frame video uploads are picked up. Unbound or missing channels sample
  transparent black.
- A fullscreen-triangle `@vertex` entry and an `@fragment` entry that calls
  `mainImage` with **Shadertoy fragCoord convention** (y-up, origin
  bottom-left): `mainImage(vec2(pos.x, u.iResolution.y - pos.y))`.

Color: `mainImage` returns **linear** RGBA; the target texture is the
sRGB swapchain format, so hardware encodes on store and the image pipeline
samples/blends it exactly like a decoded PNG. Alpha < 1 composites over the
buffer background (image pipeline alpha blend). Shadertoy ports that output
display-space color need `pow(c, vec3(2.2))` — documented, not hidden.

### GLSL (Shadertoy dialect)

`:glsl SOURCE` (imperative and declarative) accepts Shadertoy-style GLSL —
`void mainImage(out vec4 fragColor, in vec2 fragCoord)` reading bare
`iTime`, `iResolution` (vec3), `iMouse`, `iFrame` (int), `iTimeDelta`,
`texture(iChannel0, uv)` — most Shadertoy/Ghostty shaders paste unmodified.
The GLSL prelude (`compose_surface_glsl`) declares a std140 uniform block
byte-identical to the WGSL `NeoUniforms`, separate texture/sampler bindings
with `#define iChannel0 sampler2D(iChannel0Tex, iChannel0Sampler)` (naga's
glsl-in supports the Vulkan combined constructor), per-uniform accessor
functions, and a `main()` footer calling `mainImage` with y-up fragCoord.
The pipeline pairs the GLSL fragment module with a minimal WGSL vertex
module (`build_surface_pipeline`). Deliberately **no** `mainImage`
prototype in the prelude: with one, naga validates a call to a
never-defined function (silently blank surface); without it, a missing
`mainImage` is a synchronous Lisp error. The playground selects GLSL with a
`// language: glsl` header line.

## Full-frame post shader

```elisp
(neomacs-frame-shader "fn mainImage(...) { ... }")          ; WGSL
(neomacs-frame-shader shadertoy-source 'glsl)               ; Shadertoy GLSL
(neomacs-frame-shader src 'glsl '((curvature . 0.10)))      ; custom uniforms
(neomacs-frame-shader-set-uniform 'curvature 0.25)          ; live; no recompile
(neomacs-frame-shader nil)                                  ; remove
```

The Ghostty / Windows Terminal `custom-shader` feature: the shader runs
over the whole composited frame before present, with the frame bound as
`iChannel0` and `iTime`/`iResolution`/`iMouse` live (mouse in physical px,
y-up). UNIFORMS is the same alist as `:uniforms` — the same 8 `vec4` slots
and generated accessors (`(curvature . 0.10)` ⇒ `u_curvature()`), and
`neomacs-frame-shader-set-uniform` writes a slot on the installed shader
without recompiling (an error when none is installed). Example: `M-x
neomacs-shaders-crt` declares a `curvature` uniform and
`M-x neomacs-shaders-crt-curvature` retunes the barrel distortion live.
Implementation: an installed shader forces the offscreen render path
and `FramePost` replaces the scene→swapchain blit (both the transitions
path and the retained-static cursor-only path in `render_pass.rs`);
validation/composition happen on the Lisp thread (errors signal
synchronously); demand stays continuous while installed. v1 scope: the
frame texture is top-left origin while fragCoord is y-up — the pixel under
fragCoord is `vec2(fragCoord.x, iResolution.y - fragCoord.y) /
iResolution.xy`; overlays/transitions/cursor draw over the shaded frame
unprocessed.

The active render-quality policy can suppress frame shaders on a software
adapter. A new installation or live-uniform update is then rejected and
reported to Lisp (normally as a synchronous error; a transition race uses the
failure hook below). An already-requested shader is retained across a
hardware→software recovery and is restored if a later recovery selects hardware
again. A rare pipeline rejection by the active wgpu device, after synchronous
validation succeeded, runs `neomacs-frame-shader-error-functions` with the
renderer error string. If the optional `neomacs-surface` library has not been
loaded (including direct primitive use under `-Q`), the same late failure is
shown directly in the echo area.

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
  Terminal. The session now survives it: device loss triggers a contained
  GPU rebuild (see Known gaps below for what does and does not survive).
  Capped by:
- Dimension clamp 1..=4096 (matches `ImageCache::MAX_TEXTURE_SIZE`), uniform
  slots capped at 8, texture at physical (scale-factor) resolution.
- Lifecycle: ids are host-allocated (`next_host_surface_id`), and
  `neomacs-surface-create` wraps the id in a **GC-managed handle** (a
  `SurfaceObj` pseudovector, like GNU's xwidget — but deliberately not
  registry-rooted): when Lisp drops the last reference, the sweep queues the
  id and the evaluator's post-collection drain issues a best-effort
  `destroy_shader_surface`, so an un-destroyed surface is reclaimed at the
  next GC instead of leaking until exit. Explicit `neomacs-surface-destroy`
  still frees the GPU objects immediately (the handle's later GC re-destroy
  of the missing id is a render-thread no-op — harmless). Consumers
  (`set-uniform`, `destroy`, `:channel0`, the `(surface :id …)` spec) accept
  the handle or a plain integer id. `neomacs-surface-attach` ties an id to a
  buffer's lifetime via a local `kill-buffer-hook` (and
  `neomacs-surface-create-and-insert` attaches automatically); the
  declarative memo is FIFO-capped at 64 entries with `SurfaceFree` on
  eviction; surface bytes are registered in `MediaBudget` (renderer,
  `media_budget.rs`, beside the caches it polices) on create and removed on
  free, and an over-budget create evicts *recreatable* (declarative-spec)
  surfaces — the same re-resolve-on-next-walk safety argument as the memo
  cap.

## Staging

1. **DONE (this change)** — static pixels from Lisp (`:pixels`).
2. **DONE (this change)** — animated WGSL fragment surfaces: prelude,
   compositor clock, named uniforms, sync errors, visibility-scoped demand.
3. **DONE** — `:channel0` inputs (surface-to-surface, `(image …)` and
   `(video …)` specs via cross-cache binding, multipass chains); `iMouse`
   routing (hover xy + click state zw from render-thread hit tests);
   GLSL-in via naga (verbatim Shadertoy/Ghostty sources — three Ghostty
   shaders ship in `neomacs-shaders.el`, validated byte-identical);
   full-frame post pass (`neomacs-frame-shader`, custom uniforms with live
   `neomacs-frame-shader-set-uniform`); org-babel `wgsl`/`glsl` blocks
   render live surfaces under `#+RESULTS` (`lisp/ob-wgsl.el`).
4. Optional 3D mesh API — raymarching already covers most demand inside
   `mainImage`; revisit only with real users.

## Known gaps (prototype)

- ~~No device-loss recovery~~ FIXED: a GPU hang (infinite user shader →
  driver reset/TDR) no longer bricks the renderer. The wgpu device-lost
  callback — or 30 consecutive swapchain `Lost` acquisitions, for backends
  that only report the reset there — latches a flag; the render thread then
  drops the renderer and GPU context, clears every per-window GPU-resident
  object (glyph atlases, retained-static scene, transition snapshots, the
  frame-post composition target), rebuilds instance/adapter/device/queue
  plus all window surfaces, and sends `InputEvent::DisplayReset` to the
  evaluator, which clears the declarative video/webkit/surface memos,
  re-uploads all images under their existing ids, re-sends the frame
  shader, and forces a full redisplay. Expect a brief flash: the committed
  CPU frame re-renders immediately but its media quads stay blank until
  the re-resolves land. Survival matrix — the **frame shader survives**
  (host re-sends the composed module); **declarative media self-heals**
  (surfaces/videos/webkits re-create on the next redisplay walk; images
  re-decode under the same ids); **imperative surface textures do NOT
  survive** (`neomacs-surface-create` handles keep their ids but the GPU
  texture is gone — the quad stays blank until Lisp re-creates the
  surface). The hidden `neomacs--debug-lose-device` builtin exercises the
  whole path against a healthy device (no real TDR needed).
- ~~No GC integration~~ FIXED: `neomacs-surface-create` returns a GC-managed
  handle; a dropped handle's GPU objects are destroyed at the next garbage
  collection (see Lifecycle above). The declarative memo no longer leaks
  either (FIFO cap 64, GPU objects freed on eviction, evicted-but-visible
  specs re-resolve on the next walk).
- ~~`MediaBudget` covers shader surfaces only~~ FIXED: the budget lives in
  the renderer beside the caches it polices (`media_budget.rs`); image,
  video, webkit, and surface textures all register physical bytes
  (`MediaAccounting` events drained per frame via
  `reconcile_media_budget`), draws touch their entry (true LRU, wired in
  `layer_media.rs`), and an over-budget create evicts least-recently-used
  *recreatable* surfaces. Eviction still targets declarative surfaces only:
  imperative handles held by Lisp are never evicted (`recreatable` flag on
  `AssetCommand::SurfaceCreate`), and image/video/webkit entries are
  accounted but not evicted — their caches keep their own lifecycle
  (catalog re-decode, video playback, live WPE views).
  `NEOMACS_MEDIA_BUDGET_MB` overrides the 256MB limit for testing.
- ~~DPI captured at create; no rescale on monitor change~~ FIXED:
  `WgpuRenderer::set_scale_factor` resamples every shader surface's render
  target to the new physical resolution on a scale change
  (`ShaderSurfaceCache::rescale`), preserving `iTime`/uniforms and
  re-accounting `MediaBudget`; physical size is recomputed from the retained
  logical size so repeated rescales never drift. Pixel (`:pixels`) surfaces
  are content-defined and left untouched.
- ~~Failed *render-thread* compiles (naga-accepts/wgpu-rejects edge) only
  logged~~ FIXED: a render-thread build failure past naga pre-validation now
  sends `InputEvent::SurfaceCreateFailed { id, error }` back to the evaluator
  (mirroring the DisplayReset path), which runs the
  `neomacs-surface-error-functions` abnormal hook with the surface id and
  error; the default member `message`s it. (Ordinary shader syntax errors are
  still signaled synchronously from `neomacs-surface-create` via naga.)
- ~~TTY backend ignores surfaces~~ FIXED: the TTY renderer fills a surface's
  reserved columns with a `[shader]` placeholder centered in a light-shade
  fill (`surface_tty_placeholder`, `tty_rif.rs`) instead of blank space —
  GPU-only content a terminal cannot draw, now discoverable. (Image / video /
  xwidget still render blank on TTY.)

## Two traps hit while wiring this (checklist for the next media kind)

1. **Three duplicated single-spec head lists** must all learn the new head or
   the display property is silently misclassified as a *list of specs* and
   goes inert (space reserved by nothing renders):
   `SINGLE_DISPLAY_SPEC_HEADS` (`display_spec.rs`),
   `display_spec_head_starts_list` and `display_single_spec_replacing_p`
   (`neovm-core xdisp.rs`). Symptom: the end-to-end layout test emits no
   glyph while the parse unit test passes.
2. **Media must survive incremental replays.** Solved structurally: media are
   typed GLYPHS inside the row's glyph array (`GlyphType::Surface`, exactly
   GNU's IMAGE_GLYPH model — layout geometry and drawable identity travel in
   the one primitive that reserves the space), and materialization emits the
   wire `FrameGlyph::Surface` from the row walk. Any fast path that reuses or
   `pixel_y`-shifts a retained row therefore carries its media with zero
   media-specific code. The historical bug this guards: media used to live in
   flat vecs BESIDE the rows in `FrameDisplayState`, the frame state was
   rebuilt each frame, and a cursor-only replay reinstalled rows + faces but
   not media — a surface went blank one redisplay after creation (then needed
   an explicit `RetainedWindowMedia` carry, since deleted with the whole side
   channel). A new media kind now needs: a `GlyphType` variant (+ kind /
   width / hash / fallback-width arms), the materialize arm in
   `glyph_matrix.rs`, a `DisplayMediaReplacementKind` variant consumed by
   `push_media` (`display_row_builder.rs`), the `FrameGlyph` wire variant,
   and the renderer draw that matches it.
