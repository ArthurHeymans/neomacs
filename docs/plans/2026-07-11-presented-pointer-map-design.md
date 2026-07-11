# Presented Pointer Map Design

## Status

Accepted on 2026-07-11; implementation refinements recorded on 2026-07-12.

## Context

Neomacs now preserves GNU-compatible tab-bar click semantics through
snapshot-scoped `PresentationId` and `InteractionId` values. Pointer motion
also hit-tests tab-bar interaction regions and records a hovered interaction,
but that state is not consumed when the tab bar is drawn. Close and plus icons
therefore do not change visually on hover.

GNU Emacs implements two related tab-bar feedback paths:

- A formatted tab caption, including its close icon, carries
  `mouse-face = tab-bar-tab-highlight`. Native display code resolves that
  property at the glyph under the pointer, finds the complete property range,
  and redraws the existing glyph range with the realized mouse face.
- The add-tab item has no `mouse-face`. Native display code instead redraws its
  image glyph in raised state on hover and sunken state while pressed.

GNU stores this transient state in `Mouse_HLInfo` and redraws the current glyph
matrix with `DRAW_MOUSE_FACE`, `DRAW_IMAGE_RAISED`, or
`DRAW_IMAGE_SUNKEN`. It does not rebuild the tab bar on pointer motion.

Neomacs has an additional constraint: evaluator/layout and WGPU runtime live
on different threads and communicate through immutable frame snapshots. Lisp
values and face lookup cannot cross into the renderer.

## Decision

Introduce a general, snapshot-scoped `PresentedPointerMap`. It describes
pointer hit regions, click targets, and transient paint overrides for visible
frame primitives.

The evaluator/layout side resolves Lisp properties and realizes faces while
building the immutable presentation. The runtime performs hit-testing and
selects transient paint overrides without evaluating Lisp, mutating the frame,
or requesting layout. The renderer redraws existing primitives at their
published geometry with the selected override.

The first two adapters are:

1. GNU-compatible tab-bar hover and pressed feedback.
2. Main-buffer `mouse-face`, including wrapped property ranges.

This is deliberately not a tab-bar-only hover flag and not a general scene
graph.

## Invariants

1. Pointer meaning is interpreted against the exact presentation that produced
   the visible pixels.
2. Click meaning and pointer appearance are independent references.
3. All Lisp property lookup and face realization happen before publication.
4. Pointer motion performs no evaluator call, allocation, or layout.
5. Transient paint overrides preserve published geometry.
6. Replacing a presentation clears appearances from the retired presentation.
7. Ordinary non-interactive text publishes no pointer records.

## Interface

The protocol owns renderer-safe presentation types. Adapters first publish
source-addressed records; one installation pass resolves them against the final
canonical primitive table:

```rust
#[serde(transparent)]
pub struct PointerAppearanceId(u32);

pub struct PresentedPointerMap {
    regions: Vec<PresentedPointerRegion>,
    appearances: Vec<PresentedPointerAppearance>,
}

pub struct PresentedPointerRegion {
    bounds: FrameRect,
    interaction: Option<InteractionId>,
    appearance: Option<PointerAppearanceId>,
}

pub struct PresentedPointerSourceMap {
    regions: Vec<PresentedPointerRegion>,
    appearances: Vec<PresentedPointerSourceAppearance>,
}

pub struct PresentedPointerAppearance {
    paint_spans: Vec<PresentedPaintSpan>,
    hover: PointerDrawMode,
    pressed: PointerDrawMode,
}

pub struct PresentedSourcePaintSpan {
    kind: PresentedPrimitiveKind,
    row_role: GlyphRowRole,
    slot: DisplaySlotId,
    clip: FrameRect,
}

pub enum PointerDrawMode {
    Face(FaceId),
    ImageRelief(PointerImageRelief),
}
```

`PresentedPointerSourceMap::append` lets independent buffer and chrome adapters
compose without knowing final primitive indices. `FrameGlyphBuffer` performs
the only source-slot-to-primitive materialization, discards appearances whose
sources did not survive final display, remaps their region IDs, and validates
the resulting `PresentedPointerMap` atomically.

### Why interaction and appearance are separate

GNU's semantic and visual ranges are not identical. The body and close icon of
a tab have different click meanings, but the final tab-name formatter applies
one `mouse-face` range across the whole formatted caption. Consequently, the
body and close hit regions may use different `InteractionId` values while
sharing one `PointerAppearanceId`.

The plus item has an add-tab interaction and a separate appearance whose hover
and pressed modes are raised and sunken image drawing.

## Layout-side deep module

Construction complexity is hidden behind layout-owned adapters and the single
protocol materialization boundary. The pipeline:

1. Observes final rendered rows whose source positions are already preserved.
2. Carries a small row-local `GlyphPointerAppearanceId` on each interactive
   glyph. The ID indexes a row side table containing the full source identity
   and realized face, avoiding repeated source records on every glyph while
   preserving them through rollback, bidi reorder, and row reuse.
3. Resolves effective `mouse-face`, including overlays, display strings, and
   property boundaries.
4. Realizes the effective hover face into the frame face table.
5. Emits source-addressed paint spans and resolves them to immutable primitive
   spans only after all frame glyphs have been materialized.
6. Registers evaluator-owned interactions independently from appearances.
7. Coalesces adjacent equivalent hit regions and deduplicates appearances.
8. Validates all references before publishing the map.

Source identity includes the source kind and object, effective property range,
property owner, and occurrence. Occurrence distinguishes ordinary source text,
overlay before/after strings, and buffer display replacements, preventing
unrelated visible instances of the same source range from coalescing.

## Runtime state and input routing

Runtime state is qualified by presentation:

```rust
pub struct ActivePointerAppearance {
    presentation: PresentationId,
    appearance: PointerAppearanceId,
    phase: PointerAppearancePhase,
}

pub enum PointerAppearancePhase {
    Hover,
    Pressed,
}
```

Pointer motion:

1. Hit-tests the displayed `PresentedPointerMap`.
2. Updates the active appearance only if the presentation and appearance
   changed.
3. Marks the union of the old and new appearance damage rectangles dirty.
4. Keeps visual hover under the actual pointer.

Pointer press separately captures the `InputTarget`. The captured input target
remains stable until release even if visual hover moves elsewhere. This keeps
drag and release semantics independent from hover feedback.

Unknown or stale presentation references produce no input and no appearance.
Replacing the frame clears non-current hover and press state.

## Renderer behavior

Final paint spans use presentation-local indices into the immutable canonical
glyph/image table. A span may cover text glyphs or replacement images. One
appearance may contain multiple spans, for example when a main-buffer
`mouse-face` range wraps across rows.

While traversing normal frame primitives, the renderer selects drawing mode in
this order:

```text
captured and pressed -> pressed override
hovered              -> hover override
otherwise            -> normal primitive data
```

`Face(FaceId)` uses the alternate realized face while preserving the
primitive's original x/y position, advance, clipping, and source geometry. A
bold hover face must not move subsequent glyphs or trigger layout.

`ImageRelief` contains the already-resolved light/dark colors, thickness,
margins, active edges, and rounded-corner erase metadata. It is a generic
primitive-renderer operation, not tab-bar policy. Keeping these facts in the
immutable appearance prevents the render thread from consulting Lisp, theme,
or image-decoding state during pointer motion.

## Adapters

### Tab bar

For an enabled tab-bar item:

```text
mouse-face present -> Face(realized mouse-face)
mouse-face absent  -> resolved raised ImageRelief on hover
pressed            -> resolved sunken ImageRelief
```

The close icon and tab body retain distinct evaluator interactions. Appearance
ranges follow the actual `mouse-face` property boundaries, so they may share a
whole-tab appearance.

### Main buffer

For buffer text, overlays, display strings, and replacement images:

```text
mouse-face present -> Face(realized mouse-face)
mouse-face absent  -> no pointer appearance
```

Overlay priority and effective property resolution must match the existing
display-property pipeline. Wrapped ranges publish multiple paint spans under
one appearance.

### Future adapters

Scroll bars, mode/header/tab lines, fringes, margins, toolbars, and native
controls can publish through the same seam. Adding native controls requires a
typed native interaction target beside the currently evaluator-owned
`InteractionId`; it is not part of this implementation. Embedded WebKit or
video surfaces remain adapters because they own internal hit-testing.

The first implementation does not add gestures, drag-and-drop, help-echo
function evaluation, or embedded-surface internal hit-testing.

## Performance

Pointer regions are indexed by row and x coordinate. The runtime hot path is a
row lookup followed by an x-range lookup and an active-ID comparison. It
performs no Lisp lookup and no layout.

Layout publishes records only for interactive property runs or controls.
Adjacent identical regions are coalesced, and equivalent appearances are
deduplicated. Memory therefore scales with interactive runs rather than total
glyph count. Row-local appearance interning keeps ordinary glyph overhead to a
niche-sized optional token plus one side-table entry per distinct row
appearance.

Main-buffer `mouse-face` resolution caches the maximal contiguous extent for
which the effective property winner is unchanged. Overlay precedence is
resolved by sweeping indexed start/end boundaries from the active set, so the
display walk does not rescan every overlay at every character or boundary.

## Validation and failure behavior

Before publication:

- regions and clips must be finite and within the frame;
- appearance and primitive references must belong to the same presentation;
- paint spans must be non-empty and within the primitive table;
- face IDs must exist in the frame face table;
- empty or unresolved appearances are discarded.

Face realization follows GNU's fallback behavior through the evaluator/layout
face resolver. If no valid face can be realized, the region remains clickable
but has no visual override.

Image relief colors depend on facts derived from the final decoded RGBA pixels,
including GNU's four-corner background and partial-alpha mask heuristics.
Pending image metadata therefore cannot be guessed from the source encoding;
ready metadata invalidates layout and is captured in the next presentation.
Async decode requests carry an `(image id, load generation)` token. Freeing or
reloading an ID retires its active token, stale worker outcomes are rejected,
and accepted terminal outcomes retire their generation. Thus an old decode can
neither publish metadata for a reused ID nor resurrect stale relief geometry.

## Testing strategy

Tests exercise the module through its public seam:

- protocol validation and hit-testing;
- distinct interaction IDs sharing one appearance;
- tab close `mouse-face` and plus raised/sunken fallback;
- main-buffer property runs, overlay priority, display strings, images, and
  wrapped spans;
- hover enter, same-appearance movement, switching, leaving, and stale
  presentations;
- pointer capture independent from visual hover;
- renderer command selection for face, raised, and sunken overrides;
- geometry unchanged under a font-weight-changing hover face;
- integration tests with injected pointer motion and pixel/snapshot comparison.

Rust verification uses `cargo nextest`, never `cargo test`.

## Rejected alternatives

### Tab-bar-only brightness

Rejected because it ignores Lisp/theme `mouse-face`, cannot serve main-buffer
text, and embeds tab policy in the renderer.

### Evaluator round trip on every pointer move

Rejected because it introduces latency, races against newer frames, and makes
hover depend on redisplay.

### Prebuilt duplicate visual layers

Rejected after comparison with GNU. It can reproduce the pixels, but duplicates
glyph/image data and is less direct than redrawing immutable primitives with a
transient draw override.

### Resolve Lisp properties in the runtime

Rejected because Lisp values, GC roots, face lookup, and overlay precedence are
evaluator/layout responsibilities.

## Migration

1. Add protocol identities, pointer-map types, validation, and tests without
   changing rendering.
2. Add snapshot-qualified runtime hover/pressed state and generic hit-testing.
3. Add renderer face and raised/sunken primitive overrides.
4. Publish tab-bar appearances and delete tab-bar-specific hovered state.
5. Publish main-buffer `mouse-face` runs through the same builder.
6. Add integration coverage and remove superseded shallow tests and helpers.
