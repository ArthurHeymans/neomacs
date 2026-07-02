# Display Audit 01 — Runtime & Threading (`neomacs-display-runtime`)

**Date**: 2026-07-02 · Part of the [display stack audit](2026-07-02-display-audit-00-overview.md).
**Scope**: `neomacs-display-runtime/` — thread architecture, channel protocol, redraw scheduling, animation, backend abstraction, input path, hot-path hygiene, instrumentation, media flow. Cross-referenced into `neomacs-display-protocol` (frame payload), `neomacs-renderer-wgpu` (GPU submission), and `neomacs-bin` (engine-side frame production).

---

## 1. Thread & data-flow architecture

```
EMACS / ENGINE THREAD (Lisp eval + layout)      RENDER THREAD (winit event loop)
                                                 owns: winit EventLoop, wgpu Device/Queue,
redisplay_fn -> publish_gui_frame                      WPE/WebKit + dedicated GMainContext
  -> layout_frame_display_state()  (layout
     runs HERE, builds FrameDisplayState)        about_to_wait() = THE PUMP:
                                                   process_commands (drain cmd_rx)
  frame_tx.try_send(FrameDisplayState) --MOVE-->   poll_frame(): drain frame_rx + materialize()
    unbounded, never blocks          frame chan    pump_glib (g_main_context_iteration)
                                                   tick blink/anim/size/idle-dim -> mark dirty
  cmd_tx.try_send(RenderCommand) --MOVE-->         request_redraw() for dirty windows
    bounded(64)                     cmd chan       compute next wake (4ms / blink / 16ms / Wait)
                                                 window_event(RedrawRequested)
  input_rx.recv() <----MOVE------------------      -> render_frame_window() -> output.present()
    drained in wait loop            input chan   window_event(Mouse/Key) -> hit-test ->
  <-- wakeup pipe (1 byte libc::write) bnd(4096)     comms.send_input(InputEvent) (+wakeup)
                                                 helper threads: per-video decode + puller,
                                                   image decode pool (1/core) -> bounded mpsc
```

Two primary threads. The render thread is single-threaded for winit, wgpu, WebKit, GLib, and all animation. The engine thread runs layout itself and talks only through three crossbeam channels plus a wakeup pipe. Media decoding fans out to helper threads that feed bounded mpsc channels back to the render thread.

### Details

- The render thread owns everything display-related (`render_thread/mod.rs:1-3`); `RenderApp` is the winit `ApplicationHandler` (`app_handler.rs:7-34`). The wgpu Device/Queue are created in `init_wgpu` (`bootstrap.rs:65-82`) and held as `Arc` in `RenderGpuContext` (`state.rs:301-306`).
- Two spawn modes exist: the product runs the event loop **on the process main thread** (`run_render_loop_current_thread`, `bootstrap.rs:446-470`, `poll_when_idle = false`); a legacy path spawns a dedicated thread with `with_any_thread(true)` (`thread_handle.rs:123-157`).
- WPE/WebKit runs **on the render thread** with a dedicated `GMainContext` created, acquired, and pushed thread-default *before* any WebKit API call (`bootstrap.rs:391-408`). `pump_glib` (`media.rs:102-112`) drains it and explicitly refuses to pump the default context (avoids racing the engine's own GLib usage).
- The engine connects via `evaluator.redisplay_fn` → `publish_gui_frame` (`neomacs-bin/src/main.rs:3459-3489`), which runs layout **on the engine thread**.
- On exit the wgpu adapter is deliberately **leaked** (`std::mem::forget`, `lifecycle.rs:353-368`) to dodge an `eglTerminate` SEGV during Wayland teardown. Intentional; documented here so nobody "fixes" it blind.

## 2. Channel protocol (`thread_comm.rs:785-819`)

| Channel | Type | Direction | Payload | Discipline |
|---|---|---|---|---|
| frame | crossbeam **unbounded** | Engine → Render | `FrameDisplayState` | `try_send`, never blocks, never drops (`main.rs:3482`; rationale comment `thread_comm.rs:773`) |
| cmd | bounded(64) | Engine → Render | `RenderCommand` (~70 variants) | `try_send` |
| input | bounded(4096) | Render → Engine | `InputEvent` | split discipline, below |
| wakeup | OS pipe | Render → Engine | 1 byte | one `libc::write` per event (`:649-653`) |

- The frame payload is **moved, not cloned or serialized**. `FrameDisplayState` (`glyph_matrix.rs:656-725`) is the grid form: `Vec<WindowMatrixEntry>` (each a `GlyphMatrix` of `GlyphRow`s of 56-byte `Glyph` cells) plus a `faces: HashMap<u32, Face>` and side vectors (backgrounds, borders, cursors, images, videos, xwidgets, scroll bars, stipples, fringe) and GUI bar state. Size scales with on-screen glyph count.
- **Input send is split by durability** (`send_input`, `thread_comm.rs:996-1037`): lossy events (MouseMove, cancelled MenuSelection, WebKitProgressChanged — classified at `:938-952`) use `try_send` and are **dropped when the queue is full**; everything else (keys, buttons, scroll, resize, close, focus) uses a **blocking `send`** (`:1023`). See Finding 12 in the overview: a stalled engine + full queue blocks the render thread *inside the winit callback* — a UI freeze.
- Each input event performs one wakeup-pipe write syscall, unbatched (`:1005/:1028`).

## 3. Redraw scheduling

**Dirty-driven, not continuous** — this part of the design is right.

- `handle_about_to_wait` (`lifecycle.rs:200-324`) is the pump. It never renders; it only marks dirty and calls `request_redraw()` (`frame_windows.rs:1870-1876`). All GPU work happens in `WindowEvent::RedrawRequested` → `render_frame_window` (`window_events.rs:392-399`).
- Wake pacing (`lifecycle.rs:306-323`): if anything is active (dirty window, cursor animating, effects, transitions) or live content exists (WebKit/video) → `WaitUntil(now + 4ms)` (~250 Hz); else the blink deadline; else (legacy poll mode) 16 ms; else `ControlFlow::Wait` — true event-driven idle.
- **Present is Fifo/vsync** with `desired_maximum_frame_latency = 2` (`bootstrap.rs:145-148`). Consequence: the 4 ms active tick over-wakes ~4× per presented frame on a 60 Hz surface. Every tick re-runs animation state and re-marks dirty; the extra wakes buy nothing visible.
- **Frame coalescing is partial**: `poll_frame` (`frame_ingest.rs:260-581`) drains all queued frames and `set_current_frame` overwrites so only the newest per frame-id is rendered (`frame_windows.rs:545-550`) — but **`materialize()` runs on every queued frame first** (`frame_ingest.rs:273`), so the O(glyphs) grid→flat conversion is paid N times when the engine outpaces the renderer.
- **No damage/dirty-region tracking**: dirtiness is a single `compositor.dirty: bool` per window. Any change — one character, cursor blink — repaints the entire window.

## 4. Animation system

- The live animation code is `render_thread/cursor.rs` (motion/blink/size springs), `render_thread/transitions.rs` + renderer-side transition state (scroll slide, crossfade), and `RendererFrameEffects`. All ticked on the render thread from `about_to_wait` (`lifecycle.rs:237-252`); no timer thread.
- Math is time-delta based (`Instant`), framerate-independent (exponential + critically-damped spring, `cursor.rs:379-461`) — **except** idle-dim, which hardcodes a 16 ms step (`frame_windows.rs:712-713`) and therefore animates at the wrong speed on non-60 Hz displays.
- Every state-changing tick sets `compositor.dirty = true` → full-window redraw.
- `core/animation.rs`, `core/cursor_animation.rs`, `core/buffer_transition.rs`, `core/animation_config.rs` are **dead** (test-only); the glob re-exports in `core/mod.rs` mask this.

## 5. Backend abstraction — vestigial

- The `DisplayBackend` trait (`backend/mod.rs:17-26`) and `BackendType` enum (`:29-43`) look like the central architecture but **drive nothing**: `BackendType` is never matched; only `TtyBackend` implements the trait (`tty/mod.rs:817`) and it is never instantiated outside tests; there is no `WgpuBackend` and no impl for `WgpuRenderer`.
- The runtime hardcodes `renderer: Option<WgpuRenderer>` (`state.rs:312`). There is **no `Box<dyn>` and no dynamic dispatch on the hot path** — present is a direct `output.present()` (`render_pass.rs:989`). Backend selection is compile-time cfg only. (Good for performance; the dead trait is a readability problem, not a speed one.)
- The (dead-at-runtime) TTY backend in this crate rasterizes `FrameGlyphBuffer` to a `TtyGrid` and diffs grids, writing hand-rolled ANSI to stdout (`tty/mod.rs:447-448,935-941`). Note: the *live* TTY path is a different mechanism — `TtyRif` in the protocol crate, driven from `neomacs-bin` (see [report 04](2026-07-02-display-audit-04-protocol-integration.md)).
- The WPE→GPU default is **CPU pixel upload, not zero-copy**; DMA-BUF import is opt-in via `NEOMACS_WEBKIT_IMPORT=dmabuf-first` (`state.rs:120-132`), disabled by default due to a wgpu layout-transition issue on some drivers (`media.rs:247-254`).

## 6. Input path

- Fully synchronous on the render thread: winit callback → hit-test → `send_input` (`window_events.rs:120-563`). No locks on the path.
- Per mouse-move: scale divide + band-gated chrome hit tests + O(child frames) + O(webkits) rect scans (`pointer_events.rs:1679-1877`). No coalescing beyond bounded-channel drop.
- **O(glyphs) per scroll event** in wpe-webkit builds: `handle_mouse_wheel` → `webkit_glyph_hit_test` scans the entire glyph buffer for an Xwidget under the pointer (`pointer_events.rs:19-36,2127-2128`) on every wheel event. MouseButton does the scan only on press; MouseMove not at all. (The `wpe-webkit` feature is off by default — `neomacs-bin` default features are `["video","neo-term"]` — so stock builds don't pay this.)
- Per printable keypress: `translate_committed_text` allocates a `Vec<u32>` (`input.rs:180-199`) — minor.
- The blocking-send hazard for non-lossy events is described in §2.
- TTY input (`tty_input.rs`) is a separate reader thread + raw thread + unbounded intermediate channel; the poll+read loop calls `crossterm::size()` (an ioctl) every iteration (`:304`); no mouse/bracketed-paste on the unix path.

## 7. Hot-path hygiene (the key findings)

1. **Three full `FrameGlyphBuffer` clones per rendered frame** in `render_frame_window_contents_to_acquired_surface` (`render_pass.rs:373-613`):
   - **Clone A** — `render.current_frame_clone()` at `:395` deep-clones the entire buffer; it is used only to read `effect_hints` (`:415-421`) and drive the FPS counter. A full deep clone to read a small field.
   - **Clone B** — `frame_for_decision.clone()` at `:398`, plus a conditional `apply_extra_spacing` pass over **all** glyphs (`:399-406`). This value is **overwritten at `:452`** (`frame = drained_frame`) before any read — clone B and its spacing pass are **dead work** (verified by hand; the spacing pass is then redone on the drained frame at `:453-460`).
   - **Clone C** — `take_current_frame_for_render()` at `:449` → `current_frame.clone()` (`frame_windows.rs:622-628`). The name says "take"; the implementation **clones** (deliberate — blink re-renders need the frame to persist — but the name misleads about cost, and an `Arc` or blink-time re-materialize would remove the copy).
   - The buffer being cloned: `Vec<FrameGlyph>` (112 B per glyph, ~1.1 MB at 10k glyphs) + `faces: HashMap<u32, Face>` (248 B per face) + window infos + cursors.
2. **Face map rebuilt with clones every render**: `refresh_faces_from_frames` (`frame_state.rs:32-94`, called from `prepare_frame_state_for_render` on each render) builds a fresh HashMap and `Face::clone()`s every face from every window and child frame, every frame, whether or not anything changed.
3. **`materialize()` on coalesced-away frames** (§3).
4. **`EffectsConfig` cloned per render** in transition detection (`render_pass.rs:537,600`) on top of the renderer-side clone (see [report 02](2026-07-02-display-audit-02-gpu-renderer.md)).
5. **Leftover debug scan on the glyph hot path**: an unconditional per-glyph `format!` near a y-band in `neomacs-renderer-wgpu/src/renderer/glyphs.rs:1109-1150`; plus an env-gated `NEOMACS_DUMP_FRAME_GLYPHS` block (`frame_ingest.rs:277-442`).
6. **The good part**: the main glyph GPU path reuses a persistent `FrameVertexArena` (`dynamic_buffer.rs`) — `begin_frame` resets a cursor, growth doubles, uploads go `write_buffer` into a persistent buffer. No per-frame VBO allocation for text. Per-frame `create_buffer_init` churn is confined to child-frame/media/transition/overlay paths (which Phase 3 of the roadmap extends the arena to).

## 8. Instrumentation

- `core/profiler.rs` is **dead scaffolding**: a hash-based *sampling* profiler (an Emacs `profiler.c` reimplementation — CallStack → HashMap hit counts). It measures Lisp-style call stacks, not frame/phase/GPU timings, and has **zero call sites** (only `pub mod profiler;` in `core/mod.rs:8`).
- Real instrumentation today: `GlyphRenderStats` behind env `NEOMACS_RENDER_STATS` (renderer crate) and a wall-clock `FpsCounter` (`frame_state.rs:19-30`).
- **No GPU timestamp queries anywhere** — every render pass has `timestamp_writes: None`.

## 9. Media flow

- **Video**: per-video GStreamer decode thread + puller; **two CPU copies per displayed frame** (puller `map.as_slice().to_vec()` → bounded `sync_channel(4)` → render-thread `write_texture`); latest-frame coalescing drops stale frames; **no PTS pacing** (frames display on the render tick, not on their timestamps). The DMA-BUF zero-copy branch exists but is hardcoded off (`dmabuf_info = None` — RADV fd-leak workaround), so the default is a by-design GPU→CPU→GPU round trip for VA-API content.
- **Images**: thread-pool decode (one thread per core), moved through mpsc, single upload copy; a 64 MiB LRU-ish budget whose eviction is actually FIFO-by-lowest-id (can evict a *visible* image — see renderer report §8); no DMA-BUF.
- `media_budget.rs` (a 256 MB cross-media budget type) is **unused**.
- Floating video/WebKit vertex buffers are `create_buffer_init`'d per frame (`renderer/media.rs:314-320,491-508`).

## 10. Summary of runtime-layer debt

1. 3× frame clone (1 dead) per render — highest-leverage single fix in the codebase.
2. Whole-window repaint on any change (single dirty bool) + 250 Hz active wake against 60 Hz Fifo.
3. Per-frame face-map rebuild with clones.
4. `materialize()` on frames that will be discarded.
5. Blocking input send inside the winit callback.
6. O(glyphs) wheel-event scan (wpe builds).
7. Dead subsystems compiled and maintained: `DisplayBackend`/`BackendType`/`TtyBackend`(runtime), `core/animation*`, `profiler.rs`, `media_budget.rs`.
8. Zero-copy paths implemented but switched off (video DMA-BUF, WebKit dmabuf-first).
9. Debug cruft on hot paths (`format!` scan; per-iteration `crossterm::size()` ioctl in TTY input).
10. Framerate-dependent idle-dim step.
11. One wakeup syscall per input event, unbatched.

### Uncertainty flags

- All cost claims derive from reading clone/allocation patterns — no profiles exist to confirm magnitudes (see overview Finding 15).
- `neomacs-renderer-wgpu/src/renderer/mod.rs` (~96 KB) was not exhaustively read by this sub-audit; the renderer report covers it.
- Default features are `["video","neo-term"]`; `wpe-webkit` is **off** by default, so the wheel-scan and WebKit CPU-copy findings apply only to wpe builds.
