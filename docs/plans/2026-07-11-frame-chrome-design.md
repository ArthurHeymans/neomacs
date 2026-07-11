# Frame Chrome Design

**Date:** 2026-07-11

## Problem

Frame-level chrome is currently split across several unrelated protocol fields and rendering paths:

- `frame_chrome_rows` carries the tab-bar display row.
- `tab_bar` separately carries tab hit-testing metadata.
- `gui_menu_bar`, `gui_tool_bar`, and `gui_compact_bar` carry renderer-specific overlay models.
- Layout and render code independently accumulate heights and reconstruct vertical positions.

The split permits contradictory geometry. In the observed tab-bar failure, `FrameChromeRow.pixel_bounds.y` held the absolute band origin while its `GlyphRow.pixel_y` held the same absolute offset. Protocol materialization treated the latter as local and added both values, placing tab text at `104px` inside a clip band spanning `52..70px`. Separately installed images remained at `52px`.

This is an interface failure rather than an isolated arithmetic error. The model exposes two positions without making their coordinate spaces or ownership explicit.

## Goals

- Give menu, tool, compact, and tab bars one owner for stacking and frame geometry.
- Make coordinate spaces explicit and prevent double translation by construction.
- Keep feature-specific semantic content specialized.
- Derive visuals and hit regions from the same immutable layout result.
- Support GUI and TTY through concrete adapters at one real seam.
- Replace ambiguous protocol fields rather than preserving compatibility with them.

## Non-goals

- Converting every chrome feature into the same low-level content representation.
- Generalizing frame chrome to arbitrary scene graphs.
- Supporting multiline chrome before a concrete feature requires it.
- Preserving the current serialized display protocol.

## Considered designs

### Shared geometry helper

Keep the existing protocol fields but calculate their rectangles through a common helper. This is low risk, but shallow: coordinate conventions, hit testing, clipping, and publication remain distributed across callers.

### Generic render-node tree

Convert every bar into generic glyph, image, rectangle, and hit-region nodes. This unifies output but forces Lisp display rows, menus, and tool icons through a lowest-common-denominator interface. It exposes rendering mechanics instead of hiding them.

### Shared band shell with typed content

Use one frame-chrome module for ownership, ordering, measurement, placement, clipping, and interaction geometry. Retain specialized content variants for the different features.

This is the selected design. It creates a deep module: callers provide semantic requests, while the module hides the mechanics that must remain consistent.

## Module and interface

The layout engine owns a `FrameChrome` value:

```rust
pub struct FrameChrome {
    bands: Vec<FrameChromeBand>,
}

pub struct FrameChromeBand {
    id: ChromeBandId,
    kind: FrameChromeKind,
    bounds: FrameRect,
    content: FrameChromeContent,
    hit_regions: Vec<ChromeHitRegion>,
}

pub enum FrameChromeContent {
    DisplayRow(BandDisplayRow),
    Menu(MenuBarModel),
    ToolBar(ToolBarModel),
    CompactBar(CompactBarModel),
}
```

`FrameChrome` is the only module allowed to stack frame-level bars vertically. Callers do not calculate `y`, row indexes, accumulated preceding heights, clip rectangles, or frame-space hit regions.

The display protocol publishes one value:

```rust
pub frame_chrome: FrameChrome
```

It replaces `frame_chrome_rows`, `tab_bar`, `gui_menu_bar`, `gui_tool_bar`, and `gui_compact_bar`.

Deleting this module would force stacking, translation, clipping, and hit-region construction back into several callers. It therefore passes the deletion test and earns its seam.

## Coordinate spaces

Coordinate spaces are represented by distinct types:

```rust
pub struct FrameRect(Rect);
pub struct BandRect(Rect);
pub struct BandPoint(Point);
```

A frame rectangle cannot accidentally be added to another frame rectangle. Translation is directional and explicit:

```rust
impl FrameRect {
    pub fn place(&self, local: BandRect) -> FrameRect;
}
```

A band owns exactly one absolute `FrameRect`. Everything inside the band is band-local.

`BandDisplayRow` does not expose `pixel_y`. A frame-chrome band currently contains one display row, so the row begins at local `y = 0`. If a real multiline feature appears, it can introduce a private band-relative row offset then; speculative positioning is not part of the interface.

The GUI adapter performs the sole local-to-frame translation. For content at local `(0, 0)`, the materialized frame position is exactly the band origin.

## Data flow

### 1. Build semantic models

Feature-specific producers interpret Lisp and return values:

```rust
fn build_tab_bar(ctx: &mut Context) -> Option<TabBarModel>;
fn build_menu_bar(ctx: &Context) -> Option<MenuBarModel>;
fn build_tool_bar(ctx: &mut Context) -> Option<ToolBarModel>;
```

These producers own Lisp semantics, labels, enabled state, commands, faces, and display properties. They know nothing about frame coordinates or preceding bars.

### 2. Layout frame chrome

`FrameChrome::layout` measures and stacks all requested bands:

```rust
pub fn layout(
    frame: FrameSize,
    requests: impl IntoIterator<Item = ChromeBandRequest>,
    renderer: &mut ChromeContentRenderer,
) -> Result<FrameChrome, ChromeLayoutError>;
```

Each request contains its kind, visibility policy, preferred height, and semantic content. The module measures content, resolves the final height, and places each visible band after the preceding visible band.

Compact mode is a presentation policy: it replaces separate menu and tool bands rather than independently positioning another overlay.

### 3. Materialize through adapters

GUI and TTY are two concrete adapters at the frame-chrome seam:

```rust
pub trait FrameChromeAdapter {
    type Output;

    fn materialize(&mut self, chrome: &FrameChrome) -> Self::Output;
}
```

The GUI adapter produces frame-positioned glyphs, images, fills, and hit regions. The TTY adapter maps bands to terminal rows and cell-oriented interaction metadata. Both consume authoritative band geometry; neither stacks bands or reconstructs placement from heights.

## Interaction model

Hit regions are band-local and live beside their content:

```rust
pub struct ChromeHitRegion {
    local_bounds: BandRect,
    action: ChromeAction,
}

pub enum ChromeAction {
    SelectTab { tab: TabId },
    OpenMenu { menu: MenuId },
    InvokeToolBarItem { item: ToolBarItemId },
}
```

Materialization translates visual content and hit regions through the same band origin. The render thread receives semantic actions and does not infer them from glyph columns or duplicate item geometry.

Stable semantic identifiers replace positional indexes where identity must survive filtering or layout changes.

## Invariants and errors

Construction enforces these invariants:

- Bands are ordered and non-overlapping.
- Every band lies within the frame.
- A band owns exactly one absolute rectangle.
- All band content and hit regions are local to that band.
- Every hit region is clipped to its owning band.
- Compact presentation and separate menu/tool presentation are mutually exclusive.
- Semantic item identity does not depend on glyph column numbers.
- Visuals and hit regions undergo the same coordinate translation.

Disabled or empty bands are omitted and are not errors. Invalid geometry returns a typed error:

```rust
pub enum ChromeLayoutError {
    InvalidFrameSize,
    InvalidMeasuredHeight { kind: FrameChromeKind },
    ContentExceedsBand { kind: FrameChromeKind },
}
```

Non-finite and negative dimensions fail at the layout seam instead of being independently clamped by renderers. A failed chrome layout prevents publication of the new snapshot; the runtime retains the last valid snapshot and logs the offending band. Partially positioned chrome is never published.

## Testing strategy

`FrameChrome::layout` is the primary test surface. Tests assert observable results rather than private measurement or stacking steps:

- Bands appear in policy order.
- Hidden bands consume no space.
- Each visible band begins at the preceding band's bottom.
- Every band lies within the frame.
- Band-local `(0, 0)` materializes at the band origin.
- Visual and hit-region translations produce identical frame origins.
- Compact mode replaces menu and tool bands.
- Measured tab-bar height moves following content correctly.

The regression test for the reported failure uses nonzero preceding chrome:

```text
menu + tool height = 52
tab local y        = 0
tab frame y        = 52
materialized y     = 52
```

It asserts that tab text, background, image, clip rectangle, and hit region all intersect the same band. This prevents partial fixes that move only one output kind.

Property tests generate valid frame sizes and band heights. Every successful layout must satisfy:

```text
band[i].bottom <= band[i + 1].top
0 <= band.top <= band.bottom <= frame.height
materialize(local_point) = band.origin + local_point
```

Adapter tests cover conversion only: GUI coordinates remain pixels, while TTY bands occupy the expected terminal rows. Old tests for `absolute_output_row`, separate tab metadata, and renderer-owned placement are deleted once equivalent behavior is covered through the new interface.

## Migration

1. Introduce semantic chrome models, coordinate-space newtypes, and `FrameChrome` without switching publication.
2. Move the tab bar to `FrameChrome`, including its display row, media, and semantic hit regions.
3. Move menu, tool, and compact bars behind the same layout interface while retaining their typed content.
4. Switch render-thread interaction to materialized `ChromeAction` hit regions.
5. Implement the TTY adapter over the same band layout.
6. Replace the old protocol fields with `frame_chrome` and bump the protocol version.
7. Delete renderer-owned stacking, duplicate height accumulation, `FrameTabBarState`, and ambiguous absolute-row construction.
8. Replace shallow tests with interface and adapter tests.

Each migration step must keep one authoritative geometry source. Temporary compatibility adapters may read the new `FrameChrome` value to populate old outputs, but old geometry must never feed back into the new module.

## Expected result

Frame chrome becomes one deep module with a small semantic interface. Placement knowledge is local, coordinate translation occurs once, visual and interactive geometry cannot diverge, and feature-specific content remains appropriately specialized.
