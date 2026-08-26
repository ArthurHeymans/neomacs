# Cross-Platform Frame Scheduling and Animation Architecture

**Date:** 2026-07-11

**Status:** Proposed

**Scope:** GUI render scheduling, animation ownership, retained rendering, and
cross-platform presentation pacing

**Primary crates:** `neomacs-display-runtime`, `neomacs-renderer-wgpu`,
`neomacs-display-protocol`

## Executive summary

Neomacs currently has one coarse answer to several different questions: mark a
frame dirty and redraw it again soon. This works functionally, but it makes an
idle editor expensive whenever a continuous visual effect is enabled. A cursor
color cycle, for example, can keep the render loop awake even though no buffer,
window, font, glyph, child frame, or Lisp-visible display state has changed.

The observed result was approximately 8% process CPU while an org-journal
buffer was open and otherwise idle. Sampling showed that nearly all current
work was on the render/main thread rather than the evaluator thread. The
remaining activity was traced to cursor color cycling:

1. Cursor drawing computes a time-dependent color.
2. Drawing sets `needs_continuous_redraw = true`.
3. The runtime treats that boolean as frame dirtiness.
4. The event loop wakes again after a hard-coded 4 ms interval.
5. The next frame re-enters the root glyph and overlay rendering pipeline.

The immediate symptom is unnecessary CPU use. The architectural problem is
larger: Neomacs intends to support many render-side animations and effects, but
does not yet have a model that separates editor redisplay, scene repaint,
compositor-only animation, frame pacing, and presentation feedback.

This proposal introduces one deep module, the **frame coordinator**, which owns
the policy connecting visual demand to presentation. Internally it combines:

- explicit frame demand and invalidation classes;
- one-shot, per-window presentation clocks;
- native platform pacing when available;
- a bounded synthetic-clock fallback;
- an animation registry sampled using absolute presentation time;
- retained static layers and compositor-only effect rendering;
- visibility, focus, occlusion, and power policy;
- timing and work counters suitable for automated GUI tests.

The evaluator remains authoritative for GNU Emacs display semantics. The Rust
frontend becomes responsible for efficient presentation of the latest
committed scene. A compositor-only animation must never cause Lisp redisplay or
text layout, and it must not rebuild static glyph geometry.

The first implementation does not require a new framework or rendering crate.
`winit` remains the cross-platform window/event adapter and `wgpu` remains the
GPU adapter. An interpolation crate may be adopted later if several migrated
effects demonstrate enough duplicated timeline mathematics to justify it.

## Relationship to existing documents

This proposal builds on, and in one place corrects, earlier design work:

- [Two-Thread Architecture Design](2026-02-04-two-thread-architecture-design.md)
  correctly assigns editor state to the evaluator and animation/GPU ownership
  to the render thread. Its statement that `RedrawRequested` is itself native
  vsync is too strong: `winit` deliberately gives different timing guarantees
  on different platforms.
- [GUI Main Thread / Evaluator Worker Design](2026-04-26-gui-main-thread-evaluator-worker-design.md)
  establishes the actual thread ownership this proposal assumes.
- [Cursor Architecture Design](2026-06-08-cursor-architecture-design.md)
  defines cursor placement as a pure derivation from the committed display
  snapshot. This proposal preserves that invariant and addresses how the
  derived cursor is animated and scheduled.
- [Display Audit 02: GPU Renderer](../audit/2026-07-02-display-audit-02-gpu-renderer.md)
  documents the full-frame rebuild and per-frame CPU costs that make continuous
  effects expensive.
- [Render Thread Architecture Modernization Plan](../render-thread-architecture-plan.md)
  proposes `FrameCompositor` as the owner of renderable frame state. That struct
  now exists (`frame_windows.rs`) as the per-window owner of the current
  `FrameGlyphBuffer`, paired row damage, child frames, glyph atlas, and effect
  state. The frame coordinator proposed here sits above that compositor and
  decides when and at what scope it should run.

This document is the architectural "why" and target shape. It is not a promise
to replace the renderer in one change.

## Background: two independent pipelines

Neomacs has two conceptually independent pipelines that happen to meet at the
screen.

The **editor redisplay pipeline** turns Lisp-visible editor state into a display
snapshot. It includes buffer and overlay inspection, window layout, face
resolution, text shaping, glyph-row construction, cursor placement, mode-line
evaluation, and child-frame publication. GNU Emacs semantics constrain this
pipeline. It runs on the evaluator side and publishes immutable display state
to the GUI side.

The **presentation pipeline** turns the latest committed display snapshot plus
render-local visual state into pixels. It includes glyph atlas use, GPU buffer
updates, animations, effects, composition, surface acquisition, and present.
It runs on the GUI/render thread and is intentionally allowed to use a modern
Rust/GPU architecture.

These pipelines have different reasons to run:

| Trigger | Editor redisplay | Static scene repaint | Composite/present |
|---|---:|---:|---:|
| Buffer text changed | yes | yes | yes |
| Face/font changed | yes | yes | yes |
| Window geometry changed | yes | yes | yes |
| Cursor moved by an Emacs command | usually yes | maybe | yes |
| Cursor color-cycle phase changed | no | no | yes |
| Cursor glow opacity changed | no | no | yes |
| Child frame contents changed | yes | child layer only | yes |
| Window exposed after occlusion | no new semantics | maybe | yes |
| Nothing changed | no | no | no |

The current dirty-bit model loses these distinctions. Once a render-side effect
requests another frame, the presentation pipeline follows paths designed for a
new editor snapshot.

## The concrete failure

The cursor color-cycle path illustrates the failure without requiring a
synthetic benchmark.

`WgpuRenderer::emit_cursor_visual` computes the current hue from elapsed time
and sets `needs_continuous_redraw`. `RendererFrameEffects::needs_redraw()` then
folds that field together with many unrelated effects and transition queues.
The display runtime marks top-level visuals dirty, requests a redraw, and sets
`ControlFlow::WaitUntil(now + 4ms)` while any such work remains active.

At the next redraw, `render_frame_window_contents` calls
`render_frame_root_glyphs`, child-frame and content overlays, and chrome
overlays. Existing row reuse avoids some tessellation, but the runtime still
clones/materializes frame state, updates render context, enters glyph rendering,
walks overlay paths, acquires a surface, records GPU work, and presents.

This design has four feedback problems:

1. **Demand is discovered during drawing.** The scheduler cannot know whether
   another frame is needed until it has already rendered one.
2. **Demand is a boolean.** It carries no deadline, cadence, affected layer,
   damage region, completion condition, or reason.
3. **Dirty means several things.** New editor content and a changing shader
   parameter are represented by overlapping state.
4. **Pacing is a polling interval.** Four milliseconds means up to 250 wakeups
   per second, independent of display refresh and platform feedback.

Changing 4 ms to 16 ms would reduce the cost but preserve all four problems.
Disabling cursor effects by default would hide one trigger while leaving every
future continuous effect exposed to the same model.

## GNU Emacs as semantic reference, not compositor blueprint

GNU Emacs avoids this exact workload because its conventional cursor blink is
a low-frequency timer. `blink-cursor-interval` defaults to 0.5 seconds, blinking
starts after an idle delay, and focus/command hooks suspend and restart the
timer. A cursor blink changes cursor visibility; it does not continuously
animate a color at display cadence.

Neomacs should preserve GNU behavior for cursor state, point, selected-window
semantics, focus, frame visibility, and Lisp-visible redisplay. GNU's graphics
loop is not, however, a complete blueprint for a frontend whose product goal
includes shader effects, particles, smooth transitions, animated child frames,
video, and WebKit content.

The correct dividing line is:

- copy or faithfully port GNU's editor/display semantics on the evaluator side;
- use a modern retained compositor and presentation clock on the Rust side;
- make the boundary explicit enough that render-local animation cannot mutate
  or accidentally recompute GNU-owned state.

## What mature cross-platform projects do

The useful comparison is not which native callback each project invokes. It is
where each project puts the seam between frame demand and platform timing.

### Chromium

Chromium expresses display timing through `BeginFrameSource`. A compositor can
observe an external source tied to the platform display pipeline or a synthetic
timer source. Sources can be multiplexed while preserving monotonic frame
arguments. Observation is demand-driven: the scheduler can stop requesting
begin frames when no work remains.

On each begin frame, compositor-side scrolling and animations can update
without invoking the main-thread style/layout pipeline. A `BeginMainFrame` is
sent only when main-thread damage requires it. This is the closest mature
analogue to the distinction Neomacs needs between a committed editor scene and
render-local cursor effects.

References:

- [Chromium: Life of a frame](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/docs/life_of_a_frame.md)
- [Chromium: How cc works](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/docs/how_cc_works.md)
- [Chromium BeginFrame source interface](https://chromium.googlesource.com/chromium/src/+/50.0.2661.77/cc/scheduler/begin_frame_source.h)

### Flutter

Flutter uses a small abstract `VsyncWaiter`. `Animator::RequestFrame()` requests
one future frame. The platform implementation arms its native mechanism and
later fires one callback containing frame start and target presentation times.
Android uses Choreographer; Apple and embedder targets provide corresponding
adapters.

The important property is one-shot demand. A running animation asks for another
frame after processing the current one. A dormant application does not own a
permanent repeating timer merely because it is capable of animation.

References:

- [Flutter VsyncWaiter](https://chromium.googlesource.com/external/github.com/flutter/engine/+/refs/tags/3.22.0-21.0.pre/shell/common/vsync_waiter.h)
- [Life of a Flutter frame](https://flutter.googlesource.com/mirrors/flutter.git/+show/refs/heads/flutter-3.24-candidate.0/docs/engine/Life-of-a-Flutter-Frame.md)

### Qt Quick

Qt Quick separates GUI-thread animations from scene-graph animations that can
advance on the render thread. Its threaded render loop normally advances
animations in synchronization with presentation. It also documents the cases
where presentation cannot be trusted as a clock: hidden or non-renderable
windows, multiple windows, disabled vsync, broken drivers, and some virtual
machines. In those cases it falls back to elapsed-time/system-timer driving.

Qt therefore treats native vsync as a preferred adapter, not a universal
invariant. It also samples animation from time rather than incrementing logical
state by an assumed 16.67 ms on every loop iteration.

References:

- [Qt Quick scene graph and render loops](https://doc.qt.io/QT-6/qtquick-visualcanvas-scenegraph.html)
- [Qt render-thread Animator types](https://doc.qt.io/qt-6/qml-qtquick-animator.html)

### Firefox

Firefox routes platform sources through a common `VsyncSource` and
`CompositorVsyncScheduler`. On GTK, for example, the platform selects a hardware
or GLX source when supported and falls back to software vsync when it cannot
establish a reliable source. Compositor scheduling does not expose those choices
to page or animation code.

References:

- [Firefox compositor vsync scheduler](https://searchfox.org/mozilla-central/source/gfx/layers/ipc/CompositorVsyncScheduler.cpp)
- [Firefox GTK vsync source selection](https://searchfox.org/mozilla-central/source/gfx/thebes/gfxPlatformGtk.cpp)

### What `winit` does and does not provide

`winit::Window::request_redraw()` is the correct cross-platform way to request
a redraw event, but its timing contract intentionally varies:

- Windows maps requests into the `WM_PAINT` mechanism.
- Wayland can align redraw delivery with compositor frame callbacks when
  `pre_present_notify()` is called.
- Web aligns with `requestAnimationFrame`.
- Other platforms use their native window-system event mechanisms.

`pre_present_notify()` is currently meaningful on Wayland and unsupported on
X11, Windows, and macOS. It is safe and useful as an adapter hint; it cannot be
the architecture. `wgpu::PresentMode::Fifo` provides cross-platform queued
presentation/vsync behavior, but it does not tell application policy why a
frame is needed or whether layout can be skipped.

Reference:

- [`winit::Window` redraw and pre-present contract](https://docs.rs/winit/latest/winit/window/struct.Window.html)

## Goals

The target architecture must provide these properties:

1. Zero recurring render work when there is no editor change, media frame, or
   active visual effect.
2. A compositor-only effect does not invoke Lisp, layout, shaping, glyph-row
   construction, or static glyph rendering.
3. Frames are paced by the best available presentation clock and never by an
   unconstrained busy/poll loop.
4. Correct animation speed depends on monotonic time, not delivered frame count.
5. Wayland, X11, macOS, and Windows share scheduling policy while retaining
   platform-specific pacing optimizations.
6. Each native top-level window may have an independent clock and visibility
   state. Child frames composed into it share that clock.
7. Slow or dropped frames skip visual samples rather than slowing logical time.
8. Broken or unavailable native pacing falls back to a bounded synthetic clock.
9. Effects can declare their render cost and invalidation scope.
10. The design remains testable without a native window, GPU, or wall clock.
11. Timing, scheduling decisions, and render work are observable in GUI tests.
12. Existing effects can migrate incrementally.

## Non-goals

This proposal does not:

- change GNU-compatible redisplay semantics;
- move Lisp evaluation onto the render thread;
- require Vello, Bevy, Qt, Flutter, or another GUI framework;
- require direct Wayland, X11, Cocoa, or Win32 integration;
- promise zero GPU power use while a visible continuous effect is animating;
- assume swapchain images preserve previous frame contents;
- require partial-swap support from `wgpu`;
- migrate every existing visual effect in one commit;
- define a public plugin interface for third-party Rust effects yet.

## Vocabulary

The following terms are part of the proposed interface and should be used
consistently.

**Editor scene commit**: an immutable display snapshot published by the
evaluator. It represents GNU-owned editor/display state at one generation.

**Frame demand**: a declaration that pixels need to change now or in the future,
including the reason, cadence, and invalidation scope.

**Invalidation**: the least expensive category of work capable of producing the
correct pixels.

**Presentation clock**: a source of one-shot frame ticks. A tick includes a
monotonic frame time and, when known, a target presentation time and interval.

**Frame tick**: one opportunity to produce a frame. A tick is timing input, not
an instruction to rebuild editor state.

**Frame plan**: the scheduler's pure decision describing what work to perform
for one tick.

**Retained scene**: GPU-side or render-side resources representing unchanged
editor content across presents.

**Static layer**: retained content that changes only after an editor scene
commit or a render-resource invalidation such as resize/font-atlas replacement.

**Dynamic layer**: content sampled from animation/media state for a particular
frame tick.

**Native top-level window**: a window with its own OS/window-system surface and
presentation lifecycle.

**Child frame**: an Emacs frame composited into a parent native surface. It is a
scene layer, not an independent presentation clock unless it later receives a
native surface of its own.

## Design principles

### One-shot scheduling

The coordinator requests one frame, receives one tick/redraw, presents at most
one frame for that request, and requests another only if demand remains.
Duplicate requests are coalesced per native window.

### Absolute-time animation

Every animation is a function of monotonic time:

```text
visual_state = sample(animation, target_presentation_time)
```

It is never advanced using `state += speed_per_frame`. If three frames are
dropped, the next sample jumps to the correct point on the timeline.

### Minimum necessary invalidation

Demand must say whether it requires scene reconstruction, layer repaint, or
composition only. The scheduler must not infer this from a generic dirty bit.

### Demand exists before drawing

Drawing consumes a frame plan. It does not discover or latch the fact that
another frame will be necessary. Starting or updating an effect registers its
timeline and demand before the next render.

### Platform capability stays below policy

Animation and compositor code never ask whether the application is on Wayland,
X11, Cocoa, or Win32. A presentation-clock adapter uses `winit` and `wgpu`
facilities and reports timing/feedback through common types.

### Native timing preferred, synthetic timing always bounded

Native redraw/presentation behavior is used when it produces credible pacing.
A fallback clock enforces a minimum interval when timing is unavailable,
stalled, or observably too fast. The fallback is not a 4 ms polling loop.

### Retain expensive static work

Text shaping, atlas lookup, glyph geometry, child-frame text, and chrome that
did not change must survive compositor-only frames. Animation frames should
mostly update small uniforms/instance buffers, re-encode retained draw lists,
and composite retained content.

### Visibility is scheduling input

Hidden, minimized, or occluded windows cannot rely on presentation as a clock
and should not consume full-rate animation work. Policy decides whether to
pause, reduce cadence, or advance without presenting.

### Latency is scheduled, not incidental

Pacing is a latency problem as much as a power problem. The current 4 ms poll
accidentally bounds evaluator-commit pickup at ~4 ms; the replacement must
bound it deliberately. An editor commit submits demand through the event-loop
proxy and wakes the loop immediately — it never waits for the next timer or
animation tick. When a target presentation time and an estimated render cost
are known, frame work is aimed at the deadline (`target - estimated_cost -
margin`) rather than started at the earliest opportunity. Commit-to-present
latency is a first-class metric with a regression budget, so a scheduling
change that keeps every throughput counter green but adds presentation delay
is still visible.

## Target architecture

```text
Evaluator thread
  buffer/window/Lisp changes
             |
             | immutable editor scene commit
             v
+---------------------------- Render thread -----------------------------+
|                                                                        |
|  FrameCoordinator                                                     |
|    +-------------------+        +----------------------+               |
|    | AnimationRegistry |        | PresentationClock    |               |
|    | timelines/demand  |        | native + fallback    |               |
|    +---------+---------+        +----------+-----------+               |
|              |                             | FrameTick                  |
|              +-------------+---------------+                            |
|                            v                                            |
|                    +---------------+                                    |
|                    | FrameScheduler| -- pure decision --> FramePlan     |
|                    +-------+-------+                                    |
|                            |                                            |
|                            v                                            |
|                    FrameCompositor                                      |
|             +--------------+----------------+                           |
|             | retained static scene         |                           |
|             | dynamic effect/media layers   |                           |
|             +--------------+----------------+                           |
|                            | wgpu command buffers                        |
|                            v                                            |
|                       Surface present                                   |
|                            | timing/result                               |
|                            +--------------------> FrameCoordinator       |
|                                                                        |
+------------------------------------------------------------------------+
```

The external seam is `FrameCoordinator`. `RenderApp` should not independently
inspect animations, dirty state, media state, transitions, cursor movement,
and timers to choose `ControlFlow`. Those decisions belong behind the
coordinator's interface.

The internal modules are allowed to be composed for ownership and testing, but
their details should not spread back into `lifecycle.rs`.

## Core data model

The exact Rust representation may evolve during implementation, but the model
must preserve the distinctions below.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Invalidation {
    None,
    CompositeOnly { layers: LayerMask },
    RepaintLayers { layers: LayerMask, damage: Damage },
    RebuildScene,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cadence {
    OnDemand,
    NextPresentation,
    MaxRate(NonZeroU16),
    At(Instant),
}

pub struct FrameDemand {
    pub invalidation: Invalidation,
    pub cadence: Cadence,
    pub reason: DemandReason,
}

pub struct FrameTick {
    pub frame_time: Instant,
    pub target_presentation_time: Instant,
    pub estimated_interval: Duration,
    pub source: ClockSource,
}

pub enum RenderWork {
    None,
    CompositeOnly { layers: LayerMask },
    RepaintLayers { layers: LayerMask, damage: Damage },
    RebuildScene,
}

pub struct FramePlan {
    pub tick: FrameTick,
    pub work: RenderWork,
    pub should_present: bool,
}

pub enum PacingAction {
    Sleep,
    RequestRedraw,
    WakeAt(Instant),
}
```

`DemandReason` is diagnostic, not policy encoded as strings. Representative
variants include `EditorCommit`, `CursorAnimation`, `FiniteEffect`,
`Transition`, `Video`, `WebKit`, `Expose`, and `DebugCapture`.

`LayerMask` initially needs only broad retained groups, not one bit per current
effect:

- root editor content;
- child-frame content;
- cursor and cursor effects;
- transient overlays;
- native chrome;
- media/WebKit;
- transition composition.

`Damage` can begin as `FullLayer` versus a list of rectangles. The architecture
must carry damage even if the first retained implementation redraws a complete
layer. That avoids making full-layer work a permanent interface invariant.

## FrameCoordinator interface

A small interface should hide clock selection, request coalescing, fallback
timers, animation aggregation, and visibility policy. One possible shape is:

```rust
impl FrameCoordinator {
    pub fn submit_demand(
        &mut self,
        window: NativeWindowId,
        demand: FrameDemand,
    ) -> PacingAction;

    pub fn begin_frame(
        &mut self,
        window: NativeWindowId,
        tick: FrameTick,
    ) -> FramePlan;

    pub fn finish_frame(
        &mut self,
        window: NativeWindowId,
        result: PresentResult,
    ) -> PacingAction;

    pub fn update_window_state(
        &mut self,
        window: NativeWindowId,
        state: WindowPresentationState,
    ) -> PacingAction;
}
```

Callers do not set `ControlFlow` directly based on individual effect fields.
They execute the returned action through a narrow `winit` adapter.

The coordinator maintains an explicit `request_pending` invariant per window.
Submitting ten demands before the next redraw updates aggregate demand but
requests only one redraw.

## Presentation clocks and cross-platform behavior

The presentation-clock seam has two real adapters from the beginning, so it is
not hypothetical abstraction:

1. **Winit/native clock adapter**: requests redraws and consumes
   `RedrawRequested`; calls `pre_present_notify()` before present so Wayland can
   arm its frame callback; observes surface acquisition/present timing.
2. **Synthetic clock adapter**: supplies bounded monotonic deadlines when native
   pacing is unavailable or not credible.

There should not initially be separate public `WaylandClock`, `X11Clock`,
`WindowsClock`, and `MacClock` types. `winit` already owns that platform seam.
Platform-specific adapters should be added only if measurements prove that a
capability cannot be expressed through `winit`/`wgpu`.

### Wayland

For a continuing animation, request one redraw. After rendering, call
`Window::pre_present_notify()` immediately before `SurfaceTexture::present()`.
When supported, `winit` uses the Wayland surface frame callback to throttle the
next `RedrawRequested`. The synthetic clock remains available for startup,
occlusion, callback stalls, and tests; it should not race a healthy native
callback.

### X11

`pre_present_notify()` has no pacing implementation. FIFO surface acquisition
and presentation may provide blocking/backpressure, but application scheduling
must not assume that every driver/compositor combination behaves identically.
The coordinator uses measured intervals and a bounded wake deadline to prevent
runaway requests.

### Windows

`request_redraw()` integrates with `WM_PAINT`, while `wgpu` presentation maps to
the graphics backend's swap-chain behavior. `WM_PAINT` alone is not a universal
vsync clock. The same measured/fallback policy applies, with FIFO presentation
providing the normal backpressure.

### macOS

`winit` integrates redraw delivery with the application/window lifecycle and
`wgpu` presents through Metal. The initial implementation should use that
portable path. A display-link adapter is justified only if profiling shows that
portable redraw plus FIFO cannot provide stable pacing or power behavior.

### Multiple monitors and refresh-rate changes

The monitor-reported refresh interval is an initial estimate, not truth. Moving
a window between monitors updates the estimate. Observed credible frame timing
should refine it. Animation samples continue to use monotonic absolute time, so
a 60 Hz to 144 Hz move changes smoothness and cadence without changing speed.

### Broken pacing detection

The coordinator should detect at least these conditions:

- redraw/present intervals repeatedly much shorter than the selected maximum
  cadence;
- a requested native frame that does not arrive before a conservative timeout;
- surface occlusion, timeout, loss, or repeated acquisition failure;
- a window becoming hidden/minimized;
- impossible or zero monitor refresh information.

The response is to use a synthetic deadline or suspend presentation, not to
enter `ControlFlow::Poll`. The two responses must be disambiguated: on
Wayland, frame callbacks stopping usually *means* the surface is hidden or
occluded, and the correct response is to suspend presentation. The synthetic
fallback clock applies only to windows believed visible; driving renders into
a surface the compositor is not presenting defeats the power goal. When
visibility is unknown, one bounded recovery request may probe the surface,
but a probe that also goes unanswered is treated as occlusion, not as a
license to free-run.

## Per-window ownership

Each native top-level window owns:

- presentation-clock state;
- one outstanding-request flag;
- recent presentation timing;
- visibility, focus, minimization, and occlusion state;
- aggregate frame demand;
- retained static and dynamic layer generations;
- frame-local animation timelines/effect queues.

Child Emacs frames currently render into their parent surface. They therefore
share the parent's frame tick and presentation. A child-frame animation marks a
child layer dirty in the parent coordinator; it does not create a second clock.

For multiple native windows, the event loop's `WaitUntil` deadline is the
earliest synthetic deadline across windows. Native redraw requests remain
window-specific. A slow or occluded window must not force unrelated windows to
render.

## Non-frame service wakes: WPE and GLib

One standing obligation is not frame demand and must not be modeled as frame
demand: WPE WebKit requires its dedicated `GMainContext` to be pumped on the
render thread for IPC, networking, and internal tasks, even when no WebKit
frame is available and no pixels will change. Today `pump_glib()` runs on
every `about_to_wait` pass, which couples WebKit servicing to render-loop
wakeups.

The policy is explicit:

- **No WPE views alive:** no GLib pumping and no service wakes. The event
  loop may sit in indefinite `Wait`; the "completely idle" lifecycle example
  holds unconditionally.
- **WPE views alive:** the coordinator maintains a bounded service cadence
  (independent of any frame demand) that wakes the loop to pump the
  `GMainContext`. This wake performs GLib dispatch only; it renders nothing
  unless separate frame demand exists. The cadence is a service parameter,
  not a pacing parameter, and it appears in diagnostics as its own wake
  reason.
- A later refinement may integrate the GLib context's file descriptors with
  the event loop so servicing becomes event-driven rather than polled; that
  removes the standing cadence but not the distinction.

This is the sole standing exception to "no demand means no recurring wakeup,"
it exists only while WebKit views are alive, and it is bounded and observable.

## Animation registry

The animation registry owns active render-local timelines. Effects register
demand when they start or when configuration changes. Rendering does not set a
"continue" latch as a side effect.

A representative internal interface is:

```rust
pub struct AnimationSample {
    pub invalidation: Invalidation,
    pub status: AnimationStatus,
    pub next: AnimationNext,
}

pub enum AnimationStatus {
    Active,
    Finished,
}

pub enum AnimationNext {
    Dormant,
    NextPresentation,
    At(Instant),
}
```

Finite effects remove themselves when sampled finished. Infinite ambient
effects, such as cursor color cycling, return `NextPresentation` or a configured
maximum rate while the window is eligible to animate.

The registry aggregates all samples using the strongest invalidation and
earliest deadline. It also records reasons so diagnostics can answer "why is
this window still rendering?" without enabling per-effect logging.

## Retained scene and render work

Animation scheduling alone can cap the current workload to display cadence, but
it cannot make each frame cheap. The renderer must also distinguish static
editor content from dynamic visual layers.

Retention has two tiers, and the cheaper tier comes first.

**Tier 1 — retained geometry (the default).** The expensive CPU work in a
frame is materialization, glyph traversal, shaping-adjacent state, atlas
walking, and vertex building — not the draw calls. The text arena already
renders in a handful of draws. So the primary retained representation is
persistent vertex/instance buffers and atlas state keyed by scene generation:
when the generation is unchanged, a frame re-encodes the cached draw lists and
never re-tessellates. This requires no offscreen render targets, no
composition pass, no texture memory budget, and no layer-lifetime management.
The target composition under Tier 1 is:

```text
re-encoded retained static draw lists (root, child frames, chrome)
  + current media/WebKit textures
  + dynamic cursor/effect geometry (small buffers/uniforms)
  + transient overlays
  + transition composition
  -> swapchain surface
```

**Tier 2 — retained textures (adopted per layer, on measurement).** Offscreen
retained textures earn their keep only where composition must sample static
content as an image: transitions and crossfades (which already retain
textures today), and potentially child frames or expensive chrome if
re-encoding their draws is measured to matter. Tier 2 is a follow-up decision
per layer, not the Stage 4 deliverable. This ordering deletes the largest
risk cluster in this proposal — texture memory budgets, stale-layer
corruption, and generation-tracked target lifetimes — from the critical path.

Under either tier: an editor scene commit invalidates affected static groups.
A cursor color-cycle sample invalidates only the cursor/effects layer. A
child-frame content update rebuilds that child's retained geometry (or
repaints its retained texture under Tier 2). A resize invalidates retained
buffers and textures and usually requires complete rebuild.

Swapchain images cannot generally be treated as preserved. Even a cursor-only
frame may need to write the full output surface. The economical path is to
re-encode retained static draw lists (or draw retained textures where they
exist) and then draw the dynamic effect layer. This still uses GPU bandwidth,
but avoids CPU glyph traversal, shaping-related state, atlas lookup, large
vertex rebuilding, and many allocations.

The existing transition offscreen textures demonstrate that Neomacs can retain
GPU content across frames. The general retained scene should not be implemented
by forcing every ordinary frame through transition policy; it needs explicit
ownership and generation tracking in `FrameCompositor`.

### Required cursor-only invariant

After the static scene is warm, a cursor color-cycle frame must not:

- request or wait for a new evaluator frame;
- clone/materialize `FrameGlyphBuffer` merely to redraw unchanged text;
- run the glyph build path (today's `render_frame_root_glyphs`): no
  re-tessellation, re-materialization, or vertex rebuilding;
- reset current-frame fonts or walk every glyph for atlas entries;
- rebuild unchanged child-frame text geometry;
- rebuild mode-line, tab-line, tab-bar, or chrome text;
- change cursor geometry in the committed scene.

It may:

- acquire the target surface;
- sample render-local animation time;
- update a small uniform or instance buffer;
- re-encode retained static draw lists;
- composite retained textures where those exist;
- draw the cursor/effect layer;
- present and record feedback.

## GPU ownership

GPU offload is appropriate when visual state can be expressed as a function of
small parameters and time.

| Effect/property | Preferred execution |
|---|---|
| Color cycle, opacity, glow, gradient | fragment shader |
| Translation, scale, cursor slide | vertex shader or compositor transform |
| Ripple, wave, simple particles | shader/instancing; compute if justified |
| Crossfade | retained textures plus shader |
| Text content, bidi, line wrapping | evaluator/layout CPU |
| Glyph raster cache miss | renderer CPU plus GPU upload |
| Buffer/overlay semantics | evaluator/Lisp pipeline |

For cursor color cycling, the CPU should not calculate HSL and rebuild cursor
vertices every frame. A frame uniform can carry presentation time, animation
epoch, speed, saturation, and lightness. The shader derives color from those
values. Geometry changes only when the cursor shape or target changes.

GPU offload does not eliminate the need to request and present frames. A GPU
cannot independently decide to replace a desktop window's presented image
through ordinary `wgpu`. The CPU must still receive a frame opportunity, encode
or submit work, and present. Offload reduces CPU-side scene work; cadence and
visibility policy reduce total power.

## Visibility and power policy

Animation capability should not imply maximum-rate rendering in every state.
The policy should support:

- **Visible and focused:** normal configured cadence.
- **Visible and unfocused:** normal interaction effects; ambient effects may be
  reduced or paused according to configuration.
- **Occluded/minimized/hidden:** no presentation; finite timelines may advance
  by absolute time and complete without producing intermediate frames.
- **Power-saving/reduced-motion mode:** disable or reduce ambient continuous
  effects while preserving editor correctness.
- **Remote rendering on a capable GPU:** cap expensive effects using measured
  frame cost and configured maximum cadence.
- **Software-adapter compatibility:** suppress the current GPU/offscreen effect
  families and their standing frame demand. Preserve the requested
  configuration verbatim so a later hardware recovery restores it. If a
  bounded software implementation of an effect is added, its cadence must be
  derived from measured cost and user configuration rather than an invented
  fixed frame-rate cap.

These are product policies above effect implementation. Individual shaders
should not inspect focus or platform state. Backend classification, requested
configuration, and effective execution policy therefore meet at one typed
render-quality boundary; render and scheduler call sites consume its decisions
instead of branching on adapter type independently.

## Lifecycle examples

### Completely idle editor

1. No evaluator commit and no animation/media demand exist.
2. Coordinator returns `PacingAction::Sleep`.
3. Event loop uses `ControlFlow::Wait`.
4. No redraw, surface acquisition, GPU submission, or present occurs.

### Idle editor with cursor color cycle

1. Cursor effect is registered as compositor-only demand.
2. Coordinator requests one frame from the preferred clock.
3. Tick arrives with target presentation time.
4. Scheduler produces `CompositeOnly(CursorEffects)`.
5. Renderer composites retained scene and shader-driven cursor.
6. Present completes; active animation requests one subsequent tick.
7. Evaluator remains asleep throughout.

### Buffer edit while cursor effect is active

1. Evaluator publishes a new editor scene generation.
2. Existing cursor demand and new scene demand are coalesced.
3. The next plan is `RebuildScene`, the stronger invalidation.
4. Static retained layers are replaced for the new generation.
5. Cursor is sampled at the same target presentation time and composed.
6. Following frames return to compositor-only work if no further commits occur.

### Dropped or delayed frame

1. A tick arrives later than expected.
2. Animations sample the actual target/current monotonic time.
3. No loop attempts to render missing historical frames.
4. Finite effects may complete immediately if their end time passed.

### Occluded window

1. Window state becomes occluded.
2. Coordinator clears/preserves demands according to effect policy but stops
   requesting presents.
3. On exposure, static scene is reused if valid or repainted if surface/resource
   invalidation requires it.
4. Animations sample current time rather than replaying hidden frames.

## Error and fallback behavior

Surface outcomes are scheduling input:

- `Success` and `Suboptimal`: present and record timing; schedule subsequent
  demand normally.
- `Lost` or `Outdated`: reconfigure resources, invalidate retained targets, and
  request a repaint when the surface is ready.
- `Timeout`: avoid immediate spinning; schedule a bounded retry.
- `Occluded`: suspend presentation until visibility/exposure changes.
- validation/device failure: report and stop repeated demand that cannot
  produce a frame.

If native callbacks stall, one fallback deadline may request recovery. The
coordinator must prevent native and fallback sources from producing duplicate
frames for the same pending request.

## Observability

The system should expose counters per native window and process-wide totals:

- event-loop wakeups;
- native versus synthetic ticks;
- redraw requests issued and coalesced;
- redraw events received;
- editor scene commits;
- frame plans by `RenderWork` class;
- root/child static layer repaints;
- compositor-only frames;
- surface acquisitions, presents, and failures;
- dropped/skipped frame opportunities;
- observed presentation interval and jitter;
- active demand reasons;
- CPU frame preparation time and GPU submission count;
- commit-to-present latency: evaluator scene commit to the present that first
  shows it (and input-to-present where attributable);
- GLib service wakes, counted separately from frame ticks.

Tracing should describe state transitions, not print one `INFO` line per frame
by default. Per-frame detail belongs at `trace` or in bounded diagnostic dumps.
A concise diagnostic snapshot should be available to GUI tests and future
AccessKit/test tooling:

```text
window=0x100000000
clock=native estimated_hz=59.94 pending=true
scene_generation=812 static_generation=812
active=[cursor-color-cycle]
last_plan=composite-only(cursor-effects)
root_repaints=0/last_300_frames
```

## Testing strategy

### Pure scheduler tests

Use a fake monotonic clock and deterministic frame ticks. These tests require no
window or GPU and run with `cargo nextest`.

Required cases include:

- no demand returns `Sleep`;
- duplicate demand creates one outstanding request;
- editor commit dominates compositor-only demand;
- earliest deadline wins;
- finite animation becomes dormant at its end time;
- a late tick samples current time rather than frame count;
- hidden/occluded windows do not request presentation;
- exposure requests exactly one recovery frame;
- native callback timeout selects fallback without duplicate request;
- implausibly fast native feedback is capped;
- independent native windows retain independent demand and timing;
- child-frame demand maps to its parent native window;
- a coalesced one-shot demand does not re-anchor a `MaxRate` cadence phase
  (no drift or beat frequency after an editor commit interleaves with an
  ambient effect);
- an editor commit wakes the loop immediately rather than waiting for the
  next animation deadline.

### Render-plan tests

Use a recording/fake compositor adapter to prove which operations a plan calls.
The cursor-only regression must assert zero root glyph and child static-layer
repaints after warm-up.

### Renderer tests

Test shader parameters and retained-generation invalidation without relying on
wall-clock sleeps. Where headless `wgpu` is available, compare a static frame
plus two cursor-time samples and confirm that only the cursor pixels change.

### Cross-platform integration

- Wayland: headless Weston verifies frame-callback pacing and
  `pre_present_notify()` integration.
- X11: Xvfb plus a real compositor/driver job where available verifies bounded
  fallback behavior.
- Windows and macOS CI: verify one-shot redraw, occlusion/minimize behavior, and
  no runaway loop under FIFO presentation.
- Software/virtual GPU: verify broken-vsync detection and maximum-rate cap.

### Manual performance acceptance

For a warmed org-journal or Rust buffer with no input:

- evaluator/Lisp thread activity attributable to cursor animation is zero;
- static root repaint count remains zero across cursor-only frames;
- event-loop wake/present rate does not exceed selected display/effect cadence;
- disabling the last continuous effect returns the loop to indefinite wait;
- CPU and GPU measurements are recorded before and after each migration stage.

No universal CPU percentage is specified because hardware, compositor, driver,
resolution, and effect configuration vary. The structural counters are the
portable acceptance criteria; CPU reduction is the measured consequence.

## Dependency strategy

The initial architecture uses existing dependencies:

- `winit`: event delivery, window lifecycle, redraw requests, Wayland
  pre-present hint;
- `wgpu`: FIFO surfaces, retained textures, uniforms, shaders, command
  submission;
- `std::time::Instant`: monotonic scheduler and animation time.

`glissade` is a candidate for pure interpolation, keyframes, and inertial
values. It should be evaluated only after multiple migrated effects reveal a
stable common need. It does not provide presentation clocks, invalidation,
retained scenes, or frame scheduling.

Vello is a possible future renderer for vector-heavy effect layers, but it is
not required and should not be introduced as part of scheduling. Replacing the
existing text renderer would combine unrelated risks and obscure whether the
new pacing model works.

`calloop`, direct Smithay integration, Bevy, egui, iced, and other full-loop or
framework dependencies are not recommended for this work. They either duplicate
the `winit` event-loop owner, specialize the application to one platform, or
bring an incompatible application model.

## Migration plan

The migration should proceed as tracer bullets with tests and measurements,
not as a renderer rewrite.

### Stage 0: establish evidence

Add bounded counters for wakeups, redraw requests/events, scene commits, render
work classes, root glyph passes, child static passes, presents, and active
demand reasons. Record the idle org-journal baseline with cursor color cycling
enabled and disabled.

This stage changes no scheduling policy.

### Stage 1: introduce typed demand and pure scheduling

Create scheduler types and deterministic tests. Adapt existing dirty/effect
state into `FrameDemand` while preserving current behavior. Centralize the
decision currently spread through `handle_about_to_wait` behind the coordinator.

The old 4 ms cadence may remain as a compatibility adapter during this stage,
but only one location may select it.

`handle_about_to_wait` also performs per-iteration service work that is not
scheduling: `process_commands`, window creates/destroys, `poll_frame`,
monitor refresh, and `pump_glib`. These must become event-driven inputs
(command channel wakes, commit wakes through the event-loop proxy, explicit
service cadence for GLib) or explicit coordinator inputs by the end of the
migration. Otherwise `about_to_wait` survives as a second, implicit
scheduler and Stage 8 cannot complete. The `poll_when_idle` flag's 16 ms
branch must likewise end up owned by the coordinator or deleted.

### Stage 2: one-shot cross-platform pacing

Implement per-native-window request coalescing. Drive rendering from
`RedrawRequested`. Call `pre_present_notify()` immediately before present.
Use native pacing where credible and synthetic deadlines where necessary.
Remove the unconditional "active means wake in 4 ms" policy.

Verify Wayland, X11, Windows, macOS, and synthetic-clock behavior independently.

### Stage 3: cursor color-cycle tracer bullet

Register cursor color cycle as explicit infinite compositor-only demand. Delete
its render-time `needs_continuous_redraw` latch. Sample it using frame tick time.
At this stage the cursor may still travel through more renderer work than the
target, but scheduling and completion ownership must be correct.

### Stage 4: retain static editor geometry

Add generation-keyed retained geometry to `FrameCompositor`: persistent
vertex/instance buffers and atlas state that survive compositor-only frames.
When the scene generation is unchanged, a frame re-encodes cached draw lists
and performs no materialization, glyph traversal, or tessellation. Split
static rebuild from dynamic composition. Make the cursor-only structural test
pass: zero glyph-build passes after warm-up.

Retained offscreen textures are explicitly not this stage's deliverable.
They are adopted later, per layer, where composition must sample static
content as an image (transitions already do this) and where re-encode cost
is measured to matter.

### Stage 5: shader-driven cursor effects

Move color cycling and other parameter-only cursor effects to uniforms and
shaders. Retain cursor geometry until its slot/style/target changes. Combine
effect passes and use persistent buffers where practical.

### Stage 6: migrate finite effects and transitions

Move fades, cursor trails, ripples, scroll effects, and transitions into the
animation registry one family at a time. Every migration deletes a branch from
`RendererFrameEffects::needs_redraw()` and gains explicit completion tests.

### Stage 7: media, WebKit, and adaptive policy

Represent video/WebKit frame availability as frame demand rather than generic
active content. Add focus, occlusion, reduced-motion, power-saving, and maximum
cadence policy. Ensure media cadence and effect cadence aggregate correctly.

### Stage 8: delete legacy scheduling state

Delete `needs_continuous_redraw`, effect-driven generic dirty latches, and the
hard-coded active 4 ms wake. Make coordinator metrics the single explanation
for why a native window will render again.

Each stage should be a small series of test-first commits and use `cargo nextest`
for Rust tests. Release verification should use the repository's fresh release
build workflow and record process/thread CPU under the same scenario.

## Alternatives considered

### Change 4 ms to 16 ms

This caps wakeups near 60 Hz but assumes one refresh rate, remains timer-driven,
still rebuilds static content, and provides no invalidation model. Rejected as a
target; acceptable only as a temporary emergency mitigation.

### Disable cursor color cycling by default

This removes the current trigger but not the architectural defect. Future
ambient effects reproduce it. Product defaults can still be reconsidered for
power policy, but they are not the fix.

### Rely only on FIFO present

FIFO often supplies backpressure, but surface acquisition/present behavior does
not replace one-shot demand, visibility handling, damage classification, or a
fallback for broken/absent pacing. Rejected.

### Rely only on `pre_present_notify()`

Its pacing behavior is Wayland-specific. X11, Windows, and macOS would retain an
undefined application policy. It remains a required adapter hint, not the seam.

### Write four platform-specific render loops

This would duplicate scheduling semantics and make effect behavior platform
dependent. `winit` already centralizes window-system adaptation. Rejected until
a concrete missing capability justifies a narrow platform adapter.

### Adopt a game-engine loop

Game engines generally assume a continuously ticking world and optimize the
work done per tick. Neomacs must spend no recurring work when idle and preserve
an evaluator-owned editor scene. Rejected.

### Replace the renderer with Vello or another GUI framework

That may improve particular rendering tasks but does not inherently solve frame
demand or platform clocks. It would combine scheduler validation with a large
text/render migration. Rejected for this proposal.

## Risks and mitigations

**Risk: retained textures increase GPU memory.**

Tier 1 retained geometry allocates no offscreen targets, so this risk applies
only to layers promoted to Tier 2 textures. For those: track allocation per
window, recreate only on size/format changes, release for long-hidden windows
if needed, and expose memory counters.

**Risk: stale static layers create visual corruption.**

Use explicit scene/resource generations and conservative full-layer fallback.
Never infer validity solely from a dirty boolean.

**Risk: native callback and fallback timer produce duplicate frames.**

Maintain one request token/generation per window. Whichever source satisfies it
first consumes it; later callbacks are ignored or become input to the next
explicit request.

**Risk: multi-window pacing creates global coupling.**

Keep clock/demand state per native window and aggregate only the earliest event
loop wake deadline.

**Risk: animations differ across refresh rates.**

Sample absolute monotonic time. Refresh affects sample density, not duration.

**Risk: compositor-only rendering accidentally reads mutable evaluator state.**

Render only committed immutable scene snapshots and frame-local animation
state. Preserve the existing thread ownership rule.

**Risk: optimization changes visual output.**

Add screenshot/pixel-difference tests for static versus retained paths and
effect-specific samples before deleting the old path.

**Risk: too many public abstractions.**

Expose the frame coordinator as the primary interface. Keep clock selection,
fallback heuristics, animation aggregation, and platform hints private until a
real second caller requires them.

## Architectural invariants

The completed design must enforce these invariants:

1. No demand means no recurring wakeup. (Sole standing exception: bounded,
   observable GLib service wakes while WPE WebKit views are alive; see the
   service-wake section.)
2. At most one redraw request is outstanding per native window.
3. Rendering consumes demand; it does not create continuation demand as a side
   effect.
4. Every active timeline has an explicit next-frame/completion policy.
5. Animation state is sampled from monotonic absolute time.
6. Compositor-only demand cannot invoke evaluator redisplay.
7. Compositor-only demand cannot rebuild valid static glyph geometry or
   repaint valid retained layers.
8. Child frames without native surfaces share the parent presentation clock.
9. Platform-specific behavior is confined to presentation adapters.
10. Native pacing failure cannot produce an unbounded loop.
11. Surface failure cannot produce an immediate retry storm.
12. Every scheduled frame has at least one inspectable demand reason.
13. Static scene validity is generation-based.
14. Disabling or finishing the last active effect returns the event loop to
    indefinite wait.

## Decisions

This proposal makes the following decisions:

- Adopt a one-shot, demand-driven frame coordinator.
- Keep one presentation clock per native top-level window.
- Treat child frames as parent-composited layers.
- Prefer native/window-system pacing but always provide synthetic fallback.
- Use `winit`/`wgpu` as adapters instead of creating platform loops now.
- Call `pre_present_notify()` before every supported present, while recognizing
  that its current pacing benefit is Wayland-specific.
- Separate editor scene rebuild, layer repaint, and compositor-only work.
- Retain static editor content across animation frames: geometry first;
  offscreen textures per layer only where composition requires sampling
  static content as an image.
- Move parameter-only visual effects to GPU shaders when practical.
- Treat commit-to-present latency as a first-class scheduled quantity and
  metric, and wake the loop immediately on evaluator commits.
- Model WPE GLib servicing as bounded service wakes distinct from frame
  demand; true indefinite wait requires no live WebKit views.
- Preserve GNU Emacs semantics above the evaluator/render boundary.
- Introduce no new rendering framework as part of this migration.

## Open questions

These questions should be resolved by measurement during implementation rather
than guessed in the initial interface:

1. Which native platforms provide sufficiently stable pacing through current
   `winit`/`wgpu` behavior without an additional adapter?
2. What timeout identifies a stalled native frame request without causing
   duplicate frames on slow/remote displays?
3. Should ambient cursor color cycling default to display cadence, 30 Hz, or a
   configurable maximum rate?
4. Which chrome elements should be retained with root content versus isolated
   into independently invalidated layers?
5. What retained texture budget is appropriate per frame and globally?
6. Can several current effect passes share one dynamic instance buffer and one
   command encoder without complicating clipping/order?
7. When should a long-occluded window release retained GPU targets?
8. Is presentation feedback exposed by future `winit`/`wgpu` versions sufficient
   to replace interval estimation on more platforms?
9. After several migrations, does shared interpolation logic justify adopting
   `glissade`, or is a smaller internal timeline module deeper and more stable?

None of these questions blocks introducing typed demand, one-shot scheduling,
absolute-time sampling, or the cursor color-cycle tracer bullet.

## Expected outcome

With the architecture complete, an idle org-journal buffer with cursor color
cycling enabled still presents changing cursor pixels, but the evaluator sleeps
and static editor rendering remains untouched. The render thread wakes at a
bounded display/effect cadence, updates a small amount of dynamic state,
composites retained layers, and presents. When the effect is disabled or the
window becomes ineligible to animate, the process returns to event-driven idle.

More importantly, future effects gain a common model. Their authors declare a
timeline, invalidation scope, and cadence instead of manually setting dirty
flags or adding another timer to the render loop. Platform behavior remains
localized, tests can explain every frame, and performance work can optimize
specific render classes without changing editor semantics.
