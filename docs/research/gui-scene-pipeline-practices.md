# GUI scene, input, and presentation pipeline practices

Research date: 2026-07-13

This note compares primary documentation and source for Chromium/Viz,
Firefox/WebRender, Flutter, Qt Quick, GTK/GSK, Apple Core Animation, and
Wayland.  The question is how a multi-threaded GUI should keep logical state,
render data, coordinate transforms, hit testing, popups, and physical
presentation coherent.

## Executive finding

There is no single universal representation or pipeline.  Mature systems use
different structures optimized for logical state, layout/paint, compositor
work, and presentation.  The shared invariant is more important than the
structure:

> Render content, transforms, clipping, source mapping, and interaction data
> for a frame are produced coherently and activated atomically; physical
> presentation is a later outcome.

For Neomacs, the closest precedents are Chromium/Viz and Firefox/WebRender.
Both explicitly address asynchronous rendering and compositor-side hit
testing.  GTK and Wayland provide especially useful builder/commit and popup
placement patterns.

## Chromium: pending and active compositor state

Chromium maintains main-thread layer state plus compositor-thread pending,
active, and recycle trees.  Commit copies state to the compositor side;
activation replaces the active tree only when the pending tree is ready.  The
old active tree remains drawable while asynchronous raster work completes.

Source: [How cc Works](https://chromium.googlesource.com/chromium/src/+/master/docs/how_cc_works.md).

Frames are generated from active compositor state.  Viz hit-test data is
submitted with a compositor frame, and `HitTestManager` retrieves the hit-test
list whose frame index matches the active compositor frame.

Sources:

- [Chromium frame lifecycle](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/docs/life_of_a_frame.md)
- [Viz `HitTestManager`](https://chromium.googlesource.com/chromium/src/+/HEAD/components/viz/service/hit_test/hit_test_manager.h)
- [Viz `HitTestQuery`](https://chromium.googlesource.com/chromium/src/+/main/components/viz/common/hit_test/hit_test_query.h)

Chromium also distinguishes processing/activation from eventual presentation
feedback.  A submitted frame may be dropped, and presentation feedback can be
delayed or platform-dependent.

Design lesson: use `logical -> pending -> active -> presented/discarded`, and
activate frame-matched hit-test data with render data.  Do not treat submission
as physical presentation.

## Firefox/WebRender: shared transform and clip compilation

Gecko builds a display list.  WebRender turns the serialized display list into
a retained scene, then creates viewport-specific frames and GPU work.  Scene
building produces picture, spatial, and clip trees rather than one monolithic
tree.

Source: [Firefox rendering overview](https://firefox-source-docs.mozilla.org/gfx/RenderingOverview.html#webrender).

Hit-test items are emitted during the same display-list traversal as visual
content.  Although a hit-test item may paint no pixels, it travels through the
same clip chains, reference frames, stacking-context transforms, and current
asynchronous scrolling transforms.

Sources:

- [Firefox APZ and WebRender hit testing](https://firefox-source-docs.mozilla.org/gfx/AsyncPanZoom.html#hit-testing)
- [`nsDisplayCompositorHitTestInfo`](https://searchfox.org/mozilla-central/source/layout/painting/nsDisplayList.h)

Design lesson: interaction data does not need to be a visual primitive or use
the renderer's storage layout.  It must be compiled through the same transform
and clipping machinery and belong to the same frame revision.

## Apple Core Animation: model, presentation, and render trees

Core Animation explicitly documents three trees:

- the model tree stores target values changed by application code;
- the presentation tree exposes in-flight values approximating what is
  currently onscreen;
- the private render tree performs rendering and animation.

The presentation tree is read-only.  Calling `hitTest:` on a presentation
layer queries the presentation tree rather than the model tree.

Sources:

- [Core Animation basics](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/CoreAnimationBasics/CoreAnimationBasics.html)
- [`CALayer.presentation()`](https://developer.apple.com/documentation/quartzcore/calayer/presentation%28%29)
- [`CATransaction`](https://developer.apple.com/documentation/quartzcore/catransaction)

Design lesson: logical and visible values legitimately differ.  Callers must
select them by intent, and presentation state must not be mutated as if it were
the logical model.

## Qt Quick: explicit synchronization to a render-thread scene graph

Qt Quick maintains GUI-thread `QQuickItem` state and a separate rendering scene
graph.  In the threaded render loop, the GUI thread is briefly blocked while
changed item state is synchronized through `updatePaintNode`; it is then
released while the render thread draws independently.

Source: [Qt Quick Scene Graph](https://doc.qt.io/qt-6/qtquick-visualcanvas-scenegraph.html).

Qt's scene graph is sufficient for rendering and contains no references back
to QML items.  Input is generally picked using `QQuickItem` geometry, so Qt is
good evidence for synchronized render-thread projections but weaker evidence
for frame-correlated input.

Sources:

- [Qt Quick default renderer](https://doc.qt.io/qt-6/qtquick-visualcanvas-scenegraph-renderer.html)
- [`QQuickItem` coordinate mapping and containment](https://doc.qt.io/qt-6/qquickitem.html)

Design lesson: thread ownership and a defined synchronization phase matter.
Qt also demonstrates that current logical-tree picking can be acceptable for a
tightly synchronized toolkit, but it is less suitable when input crosses back
to an independently advancing evaluator.

## Flutter: multiple retained trees and shared paint transforms

Flutter uses immutable widget descriptions, persistent element and render
object trees, then composited layer and engine scenes.  Render objects own
layout, paint, hit testing, and accessibility behavior.

Source: [Flutter architectural overview](https://docs.flutter.dev/resources/architectural-overview).

Flutter's pipeline explicitly flushes layout before compositing bits and paint.
Hit testing applies the inverse of the same transforms used to paint children.
For anchored overlays, `CompositedTransformTarget` and
`CompositedTransformFollower` link leader and follower layers during
composition rather than asking clients to reconstruct global offsets.

Sources:

- [`PipelineOwner`](https://api.flutter.dev/flutter/rendering/PipelineOwner-class.html)
- [`RenderObject.applyPaintTransform`](https://api.flutter.dev/flutter/rendering/RenderObject/applyPaintTransform.html)
- [`CompositedTransformFollower`](https://api.flutter.dev/flutter/widgets/CompositedTransformFollower-class.html)

Design lesson: use the same transform semantics for paint, hit testing, and
anchored overlays.  Keep retained GPU/resource state as a cache rather than
semantic truth.

## GTK/GSK: mutable snapshot builder, immutable render output

GTK 4 walks the widget hierarchy into a `GtkSnapshot`, which maintains a stack
of render nodes and transforms.  Converting a snapshot to a `GskRenderNode`
seals it: no more nodes can be added afterward.  GTK caches widget render nodes
and invalidates them on demand.

Sources:

- [GTK drawing model](https://docs.gtk.org/gtk4/drawing-model.html)
- [`GtkSnapshot`](https://docs.gtk.org/gtk4/class.Snapshot.html)
- [`GtkSnapshot.to_node`](https://docs.gtk.org/gtk4/method.Snapshot.to_node.html)

GTK event delivery picks widgets through the widget hierarchy, with explicit
widget-local coordinate systems and transformation functions.

Sources:

- [`GtkWidget.pick`](https://docs.gtk.org/gtk4/method.Widget.pick.html)
- [GTK coordinate systems](https://docs.gtk.org/gtk4/coordinates.html)

Design lesson: a mutable builder followed by immutable output is an effective
publication interface.  Rendering can be retained and event-driven rather
than rebuilding continuously.

## Wayland: atomic surface commit, terminal feedback, semantic popups

Wayland surface state is double-buffered.  Buffer content, damage, scale,
transform, opaque region, and input region remain pending until
`wl_surface.commit`, which applies them atomically.

Source: [Wayland protocol specification](https://wayland.freedesktop.org/docs/html/apa.html#protocol-spec-wl_surface).

The presentation-time protocol associates feedback with one committed content
update and reports one terminal result: `presented` with timing information or
`discarded` because the update was superseded or destroyed.

Source: [presentation-time protocol](https://wayland.app/protocols/presentation-time).

The `xdg_positioner` popup protocol accepts popup size, parent anchor rectangle,
anchor, gravity, offset, and flip/slide/resize constraints.  Placement is
negotiated and acknowledged instead of being represented as an unchecked
global `(x, y)` guess.

Source: [xdg-shell protocol](https://wayland.app/protocols/xdg-shell).

Design lesson: commit input and visible surface state together; model physical
presentation separately; represent popup placement semantically.

## Frame scheduling and damage

Mature systems are normally event- and damage-driven:

- GTK requests layout or paint phases only after invalidation and caches render
  nodes between frames.
- Qt schedules a new frame with `update()` rather than requiring an unconditional
  application-side loop.
- Chromium carries damage with compositor frames and uses partial swaps where
  possible to reduce work and power use.

Sources:

- [GTK drawing model](https://docs.gtk.org/gtk4/drawing-model.html)
- [`QQuickWindow`](https://doc.qt.io/qt-6/qquickwindow.html)
- [Chromium damage tracking](https://chromium.googlesource.com/chromium/src/+/master/docs/how_cc_works.md#damage)

Design lesson: an animated cursor may request regular frames, but unchanged
editor content should remain cached.  Damage should identify the affected
region; correctness must not depend on partial rendering.

## Recommended Neomacs synthesis

No project supplies a complete design to copy.  Neomacs should combine the
following practices:

1. Preserve GNU-compatible logical `Frame`/`Window` state on the evaluator.
2. Model `Prepared`, `Active`, and `Presented | Discarded` as distinct states.
3. Build each prepared presentation atomically from one logical revision.
4. Permit separate render, hit-test, source-map, and accessibility projections,
   but give them one presentation identity and shared transform/clip semantics.
5. Activate all projections together.  Never combine a new render projection
   with an old hit-test projection.
6. Hit-test the renderer's active presentation and carry its identity back to
   the evaluator.
7. Retain scene metadata until ordered input delivery guarantees no event can
   still reference it.
8. Resolve popups from semantic anchors and placement constraints using the
   active presentation's transform tree.
9. Keep glyph atlases, textures, raster caches, and recycling structures out of
   semantic scene truth.
10. Use invalidation and damage for efficiency after the correctness model is
    established.

The ideal interface is therefore one **presentation transaction**, not
necessarily one physical scene structure.  Multiple projections are healthy;
independent identities, transforms, publication, or lifecycle are not.

## Neomacs implementation result

The 2026-07-13 refactor applies this synthesis as follows:

- `FramePresentationState` owns monotonic prepared and active
  `PresentationGeometry`; preparing never changes active geometry.
- The render thread emits ordered activation, discard, and retirement events.
  Rejected submissions are discarded on the evaluator as well.
- `PresentationSpatialPlan` seals window regions, frame placement, transforms,
  clips, source positions, and hit-test metadata from one plan. Protocol and
  renderer validation reject divergent projections.
- Logical GNU window queries read `Frame`/`Window` state or the latest private
  redisplay cache. Exact visual queries require the active presentation and
  use presentation-qualified semantic queries.
- Pointer observations carry the presentation used for renderer hit testing;
  stale observations are ignored instead of falling back to mutable live
  arithmetic.
- Native popup commands preserve anchor rectangle, preferred side, offset, and
  flip/shift policy through to popup layout, which resolves the final origin
  only after measuring content against the owning viewport.
- Legacy independent snapshot-publication and implicit geometry fallback APIs
  have been removed. Test-only cache fixtures are excluded from production.

The remaining lifecycle extension is platform presentation feedback with
scanout timing. It should attach to the existing presentation identity; it
does not require another scene, coordinate model, or fallback query path.
