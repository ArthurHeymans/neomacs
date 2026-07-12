# Presented Geometry Design

Date: 2026-07-12

## Decision

Neomacs will converge on one immutable, presentation-scoped geometry module.
Redisplay publishes authoritative geometry once.  GNU compatibility, pointer
input, scrolling, child frames, and rendering query that same presentation;
they do not reconstruct coordinate transforms from character metrics, frame
parameters, or unrelated scalar fields.

The module is named `PresentedGeometry` in this design.  Its implementation may
initially extend the existing display snapshot rather than begin as a new
crate.  The interface and invariants matter more than its first physical home.

## Problem

Geometry is currently distributed across VM window state, redisplay snapshots,
Lisp compatibility functions, layout output, display protocol values, pointer
maps, and the child-frame compositor.  Most of these values use unqualified
`f32` rectangles.  Callers must remember whether a value is window-local,
text-body-local, frame-relative, parent-frame-relative, root-surface-relative,
logical, physical, pixels, or cells.

The Corfu/Treemacs failure demonstrates the consequence.  In GUI mode,
`window-pixel-left` divides an actual pixel origin by character width.  With no
left side window the origin is zero and the error is invisible.  With a
144-pixel Treemacs window, the selected window's translation is lost.

## GNU Emacs compatibility invariants

The internal Rust design may be safer than GNU's C implementation, but the
observable contracts must remain GNU-compatible.

1. Window pixel geometry and cell geometry are independent stored facts.
   `window-pixel-left` and `window-pixel-top` return stored pixels;
   `window-left-column` and `window-top-line` return stored cells.
2. Window body edges compose the actual pixel window origin with internal
   border, scrollbars, fringes, margins, header line, and tab line.
3. Text `posn` values are relative to the text body.  Mouse input arrives in
   frame space and is transformed through the same geometry.
4. Child-frame `left` and `top` are relative to the immediate parent's native
   rectangle.
5. Logical-to-device scaling is explicit and occurs at the native backend
   seam.

GNU source references:

- `src/window.c`: `window-pixel-left`, `window-pixel-top`, and
  `window_resize_apply`.
- `src/window.h`: `WINDOW_TO_FRAME_PIXEL_*`, `FRAME_TO_WINDOW_PIXEL_*`, and
  window box helpers.
- `lisp/window.el`: `window-edges` and `window-body-pixel-edges`.
- `src/keyboard.c`: `make_lispy_position` and `posn-at-x-y`.
- `src/xfns.c`: child-frame parent-relative positioning.
- `src/frame.c`: `frame-scale-factor`.

## Coordinate and unit types

Plain numeric coordinates do not cross the module interface.  Values name
both their unit and coordinate space:

```rust
struct LogicalPx(f32);
struct DevicePx(i32);
struct Column(i64);
struct Line(i64);

struct Point<Space> { x: LogicalPx, y: LogicalPx, _space: PhantomData<Space> }
struct Rect<Space> { origin: Point<Space>, size: Size<LogicalPx> }

enum WindowLocalSpace {}
enum WindowBodySpace {}
enum FrameLogicalSpace {}
enum ParentFrameSpace {}
enum RootSurfaceSpace {}
```

Runtime ownership is also carried explicitly.  A body-local point includes its
`WindowId`; a frame point includes its `FrameId`.  This prevents combining
coordinates from different windows merely because their static space matches.

Rectangles are finite, nonnegative, and half-open:
`[left, right) × [top, bottom)`.

## Authoritative presentation

Each completed redisplay produces one immutable presentation:

```rust
struct PresentedGeometry {
    presentation: PresentationId,
    frames: FrameGeometryIndex,
    windows: WindowGeometryIndex,
    positions: PresentedPositionIndex,
    interactions: InteractionIndex,
}
```

Window geometry stores pixel and cell facts separately:

```rust
struct PresentedWindow {
    window: WindowId,
    frame: FrameId,

    outer: Rect<FrameLogicalSpace>,
    regions: WindowRegions,

    left_column: Column,
    top_line: Line,
    total_columns: Columns,
    total_lines: Lines,

    rows: Vec<PresentedRow>,
    cursor: Option<Rect<FrameLogicalSpace>>,
}
```

Pixel queries never divide by character width or height.  Cell queries never
silently return pixels.

## Explicit window regions

Fringes, margins, scrollbars, dividers, and chrome are first-class rendered
regions:

```rust
struct WindowRegions {
    text_body: Rect<FrameLogicalSpace>,

    tab_line: Option<Rect<FrameLogicalSpace>>,
    header_line: Option<Rect<FrameLogicalSpace>>,
    mode_line: Option<Rect<FrameLogicalSpace>>,

    left_margin: Option<MarginGeometry>,
    right_margin: Option<MarginGeometry>,
    left_fringe: Option<Rect<FrameLogicalSpace>>,
    right_fringe: Option<Rect<FrameLogicalSpace>>,

    left_scrollbar: Option<Rect<FrameLogicalSpace>>,
    right_scrollbar: Option<Rect<FrameLogicalSpace>>,
    horizontal_scrollbar: Option<Rect<FrameLogicalSpace>>,

    right_divider: Option<Rect<FrameLogicalSpace>>,
    bottom_divider: Option<Rect<FrameLogicalSpace>>,
}

struct MarginGeometry {
    columns: Columns,
    bounds: Rect<FrameLogicalSpace>,
}
```

The layout producer supplies these rectangles because it knows the actual
fringe/margin ordering, scrollbar side, chrome height, text scale, fractional
metrics, and divider widths.  Consumers do not rebuild them as generic insets.

This supports fringes inside or outside margins and scrollbars on either side.
It also gives pointer hit testing semantic targets instead of inferred bands.

## Deep module interface

The external interface has two operations:

```rust
impl GeometryStore {
    pub fn publish(
        &mut self,
        facts: LayoutFacts,
    ) -> Result<PresentationId, GeometryError>;

    pub fn resolve<Q: GeometryQuery>(
        &self,
        presentation: PresentationId,
        query: Q,
    ) -> Result<Q::Output, GeometryError>;
}
```

`GeometryQuery` is sealed.  Callers request semantic results, not arbitrary
transform chains.  Initial queries include:

```rust
WindowGeometry { window }
WindowRegionBounds { window, region }
PositionGeometry { window, buffer_position }
HitTest { frame, point }
PlaceChild { child, parent, anchor, policy }
SurfaceGeometry { frame }
```

The module hides window layout, region composition, buffer-position lookup,
frame ancestry, clipping, hit testing, popup constraints, scaling, rounding,
and presentation lifetime.

## GNU geometry adapter

`neovm-core` owns a thin compatibility adapter.  It converts semantic queries
to GNU Lisp values and applies GNU rounding and error conventions:

```text
window-pixel-left          -> outer.left
window-pixel-top           -> outer.top
window-body-pixel-edges    -> text_body
window-left-column         -> stored left_column
window-top-line            -> stored top_line
window-fringes             -> stored fringe regions
window-margins             -> stored margin columns
window-scroll-bars         -> stored scrollbar regions
posn-at-point              -> glyph rect in WindowBodySpace
```

`PositionNotVisible` maps to GNU `nil`.  A stale presentation or invalid
transform remains a structured invariant error.

Real GNU Lisp such as `window.el` must be used by integration tests.  Tests
must not bypass this adapter by calling a convenient Rust helper directly.

## Pointer, scrolling, and rendering

Pointer input converts device coordinates to root logical coordinates exactly
once, then calls `HitTest`.  Rendering and hit testing share the same regions,
clips, z-order, and presentation.

Visual rows store frame/body bounds, buffer spans, continuation state, and
clipping.  Scroll and recenter planning query those rows and return a pure
viewport plan; the evaluator applies state mutation.

The renderer consumes resolved surface geometry.  It does not repair package-
specific offsets or infer window regions.

## Child frames and popups

GNU-compatible child placement remains immediate-parent-relative:

```rust
struct ChildPlacement {
    child: FrameId,
    parent: FrameId,
    rect_in_parent: Rect<ParentFrameSpace>,
}
```

Parent ancestry is accumulated exactly once when compositing or hit-testing.
Root-relative coordinates are derived, not stored as a second authoritative
position.  Fields named `parent_x`/`parent_y` must not contain root coordinates.

Neomacs-native popups may use a pure placement policy:

```rust
struct PopupRequest {
    anchor: Anchor,
    size: Size<LogicalPx>,
    preferred_side: Side,
    alignment: Alignment,
    gap: LogicalPx,
    constraints: PopupConstraints,
}
```

The resolver owns flip, slide, clamp, and resize decisions.  GNU packages such
as Corfu retain their Lisp placement policy; Neomacs supplies composable GNU
geometry and executes their final parent-relative rectangle unchanged.

## Scaling and rounding

Layout and protocol geometry use logical pixels.  Logical-to-device conversion
happens once in the backend adapter.  Quantize rectangle edges, then derive
size:

```text
device_left  = round(logical_left  * scale)
device_right = round(logical_right * scale)
device_width = device_right - device_left
```

This prevents independent origin/size rounding from creating gaps or overlap.
Wayland and X11 may use different backend adapters, but neither owns editor
layout or popup policy.

## Diagnostics

Every geometry query can produce a structured explanation:

```text
presentation=481
window_outer=(144,24,1831,1172) [stored/frame]
text_body=(168.8,41,...)         [stored/frame]
glyph=(475.2,323,7.2,17)        [stored/body]

transform body->frame=(+168.8,+41)
anchor_frame=(644,364,7.2,17)

child_requested=(493,382,289,342) [parent]
child_composited=(493,382,289,342) [root]
policy_offset=(-151,+1)
```

Each number identifies its presentation, owner, space, and provenance: stored
fact, transform, policy adjustment, or rounding result.

## Invariants

- A query uses exactly one `PresentationId`.
- Pixel and cell facts are different types and stored independently.
- Every coordinate identifies its owner and space.
- Window regions are explicit and reflect rendered ordering.
- Body-local position plus body-in-frame origin equals the rendered glyph.
- Rendering and hit testing use inverse paths through the same transforms.
- Child position is immediate-parent-relative.
- Frame ancestry and scale are applied exactly once.
- Missing authoritative GUI geometry never silently becomes a grid estimate.
- The compositor performs no GNU-package-specific geometry repair.

## Verification

Required regression families:

1. Add a left side window of width `d`: window, cursor, and child frame shift
   by `d`; body-local `posn` is unchanged.
2. Add a window above: equivalent Y translation properties hold.
3. Exercise fringe/margin ordering, scrollbars on both sides, header/tab/mode
   lines, and dividers.
4. Assert `body_origin + posn == rendered_glyph_origin`.
5. Assert render-to-hit-test round trips for every region.
6. Test nested child parent-to-root composition.
7. Test fractional Wayland scale edge rounding and X11 scale behavior.
8. Run loaded GNU `window.el` geometry functions.
9. Run a Corfu integration scenario with and without Treemacs.

## Migration

1. Correct GUI `window-pixel-left/top` and add GNU Lisp side-window tests.
2. Introduce typed pixel/cell/space values around existing snapshots.
3. Publish frame, window, explicit region, row, cursor, and position geometry
   under one presentation identity.
4. Route GNU geometry through the compatibility adapter.
5. Route pointer hit testing through the same presentation.
6. Route child-frame transforms and nested hit testing through it.
7. Route scroll/visibility planning through presented rows.
8. Delete duplicated geometry arithmetic, hit maps, and misleading absolute
   child fields.
9. Add a retained spatial graph internally only if query requirements justify
   its memory and indexing cost.

The migration must replace old geometry paths rather than permanently layering
another representation on top of them.
