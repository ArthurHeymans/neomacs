# WGPU Glyph Atlas Refactor Plan

Date: 2026-05-31
Status: implementation plan
Scope: `neomacs-renderer-wgpu`, with limited call-site changes in `neomacs-display-runtime`

## Goal

Replace the current per-glyph texture/bind-group rendering model with a real
GPU glyph atlas so text draw submission is proportional to material/page runs,
not glyph count or unique character count.

The refactor must preserve visual correctness:

- Text is rendered in visual order.
- Subpixel glyph rasterization remains aligned to the same subpixel bins.
- Composed glyphs, emoji, combining marks, overstrike, clipping, cursor inverse
  video, child frames, overlays, and HiDPI scale changes keep working.
- The TTY renderer is not part of this problem and should not be changed.

The implementation should use Rust's type system aggressively enough that
material mismatches, invalid atlas rectangles, stale handles, and boolean-state
confusion are caught early. Do not build a large pile of runtime `bool` checks
when an enum, newtype, sealed constructor, or typestate marker can represent the
invariant.

## Current Problem

`WgpuGlyphAtlas` is named like an atlas, but it currently caches each glyph as
its own `wgpu::Texture`, `TextureView`, and `BindGroup`.

That creates these bottlenecks:

- A glyph texture bind is required whenever the rendered glyph key changes.
- Simple glyphs are currently sorted by partial key to reduce bind-group churn.
  That can reduce draw calls, but it also globally reorders glyph draws.
- Worst-case text is still one draw per glyph because every glyph can have a
  different key.
- Composed glyphs still create a small vertex buffer and issue one draw per
  composed glyph.
- Child-frame content repeats the same simple/composed split.
- Overlay icon paths have similar per-item buffer/draw patterns, but they are
  lower priority than buffer text.

The core problem is not just draw calls. It is the resource model: one GPU
texture per glyph makes batching inherently limited.

## Target Shape

The target renderer has these properties:

- Glyph rasterization still happens on cache miss in `WgpuGlyphAtlas`.
- Cache misses upload pixels into atlas pages instead of creating per-glyph
  textures.
- A cached glyph returns atlas metadata: material, page id, UV rectangle,
  pixel size, bearings, and advance.
- Renderer text collection builds vertices in visual order using atlas UVs.
- Draw submission batches contiguous runs with the same material and atlas page.
- In the common case, a full ASCII/subpixel text pass draws with one atlas page
  and one draw call.
- Composed glyphs use the same atlas path as single-codepoint glyphs.
- Per-frame glyph vertex data is uploaded through reusable dynamic buffers
  instead of creating transient buffers every frame.

The first implementation should prefer simple page creation over complex
compaction. Correctness and debuggability matter more than squeezing every last
atlas pixel in the first pass.

## Non-Goals

Do not bundle these into the first implementation:

- Rewriting text shaping or font measurement.
- Moving font work between eval/render threads.
- Rewriting renderer visual effects.
- Generalizing image/video/WebKit texture caches.
- Atlas compaction/defragmentation.
- A full renderer abstraction rewrite.

Toolbar/compact-bar icon batching can be handled after text because it is the
same class of problem but has lower impact.

## Design Principle: Let Types Carry Invariants

The implementation should avoid the current style where a `CachedGlyph` has
runtime booleans like `is_color` and `is_subpixel`, and callers must remember
which pipeline matches those booleans.

Prefer these patterns:

1. Use enums for mutually exclusive states.
2. Use newtypes for ids and coordinate spaces.
3. Keep struct fields private when invalid values are possible.
4. Use smart constructors for rectangle/size validation.
5. Use `NonZeroU32` or similar where `0` is not a valid id.
6. Use phantom marker types where material-specific ids/entries should not mix.
7. Return copyable metadata from atlas lookup instead of borrowing atlas entries
   through later renderer mutation.
8. Match exhaustively on material variants at pipeline boundaries.

### Suggested Type Model

Use marker types for material-specific atlas entries:

```rust
pub enum AlphaMask {}
pub enum SubpixelMask {}
pub enum ColorRgba {}

pub trait GlyphMaterial: private::Sealed {
    const KIND: GlyphMaterialKind;
    const TEXTURE_FORMAT: wgpu::TextureFormat;
    const BYTES_PER_PIXEL: u32;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlyphMaterialKind {
    AlphaMask,
    SubpixelMask,
    ColorRgba,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageId<M: GlyphMaterial> {
    raw: NonZeroU32,
    _marker: PhantomData<M>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasEntry<M: GlyphMaterial> {
    page: PageId<M>,
    rect: AtlasContentRect,
    uv: UvRect,
    metrics: GlyphMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnyAtlasEntry {
    Alpha(AtlasEntry<AlphaMask>),
    Subpixel(AtlasEntry<SubpixelMask>),
    Color(AtlasEntry<ColorRgba>),
}
```

The renderer must match on `AnyAtlasEntry` before choosing a pipeline. That
makes it difficult to accidentally render a subpixel mask with the alpha shader
or a color glyph with the mask shader.

Represent rasterized pixels as an enum:

```rust
pub enum RasterizedGlyphPixels {
    Alpha {
        size: PixelSize,
        bytes: Vec<u8>,
    },
    Subpixel {
        size: PixelSize,
        rgba: Vec<u8>,
    },
    Color {
        size: PixelSize,
        rgba_srgb: Vec<u8>,
    },
}
```

`RasterizedGlyphPixels::material()` should be the only place that maps
rasterization output to atlas material. It should also validate expected byte
length:

```rust
impl RasterizedGlyphPixels {
    pub fn validated(self) -> Result<Self, GlyphUploadError>;
    pub fn material(&self) -> GlyphMaterialKind;
    pub fn size(&self) -> PixelSize;
    pub fn bytes(&self) -> &[u8];
}
```

Avoid carrying `is_color` and `is_subpixel` as independent booleans. They encode
a three-way state and allow impossible combinations.

### Coordinate Newtypes

Use at least these small value types:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelSize {
    width: NonZeroU32,
    height: NonZeroU32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasAllocationRect {
    x: u32,
    y: u32,
    width: NonZeroU32,
    height: NonZeroU32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasContentRect {
    x: u32,
    y: u32,
    width: NonZeroU32,
    height: NonZeroU32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UvRect {
    min: [f32; 2],
    max: [f32; 2],
}
```

The allocator can return a padded allocation rect plus an inner content rect.
The renderer should only receive the content rect/UV. This makes it harder to
sample padding by mistake.

If the team wants to go further, add lightweight `LogicalPx` and `PhysicalPx`
wrappers. That would prevent repeated mistakes around `scale_factor`, but it is
not required for the first atlas landing.

### Replace Boolean Render Mode Arguments

Current APIs pass `enable_subpixel: bool`. Prefer:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubpixelRequest {
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlyphRasterMode {
    Alpha,
    Subpixel,
}
```

Then:

```rust
fn raster_mode(&self, request: SubpixelRequest) -> GlyphRasterMode {
    match (request, self.subpixel_order.allows_horizontal_subpixel()) {
        (SubpixelRequest::Enabled, true) => GlyphRasterMode::Subpixel,
        _ => GlyphRasterMode::Alpha,
    }
}
```

This makes call sites self-documenting and avoids boolean inversion bugs.

## Atlas Data Structures

### Page Storage

An atlas page owns one GPU texture and one bind group. Pages are material
specific:

```rust
struct AtlasPage<M: GlyphMaterial> {
    id: PageId<M>,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    allocator: ShelfAllocator,
    last_accessed_generation: u64,
}
```

The atlas can store pages as separate typed vectors:

```rust
struct GlyphAtlasPages {
    alpha: Vec<AtlasPage<AlphaMask>>,
    subpixel: Vec<AtlasPage<SubpixelMask>>,
    color: Vec<AtlasPage<ColorRgba>>,
}
```

This is more verbose than `HashMap<GlyphMaterialKind, Vec<Page>>`, but it gives
compile-time protection inside the atlas implementation. If that becomes too
heavy, keep typed APIs at the boundary and store an internal enum:

```rust
enum AtlasPageAny {
    Alpha(AtlasPage<AlphaMask>),
    Subpixel(AtlasPage<SubpixelMask>),
    Color(AtlasPage<ColorRgba>),
}
```

Either is acceptable. Do not store a page with `material: GlyphMaterialKind` and
then manually assert the texture format at every use site unless the typed form
becomes impractical.

### Allocation

Use a simple shelf allocator first:

```rust
struct ShelfAllocator {
    page_size: u32,
    padding: u32,
    cursor_x: u32,
    cursor_y: u32,
    shelf_height: u32,
}
```

Allocation algorithm:

1. Reject glyphs larger than `page_size - 2 * padding`.
2. Requested allocation size is glyph width/height plus padding on all sides.
3. If it does not fit on the current shelf, advance to a new shelf.
4. If it does not fit on a new shelf, return `None`.
5. Return both allocation rect and content rect.

Use 1 or 2 pixels of padding. Because the sampler is linear, padding must be
filled so atlas neighbors do not bleed into glyph edges. The safest first
implementation is:

- upload glyph pixels into the content rect;
- duplicate edge pixels into the padding border;
- compute UVs from the content rect, not from the padded allocation rect.

Do this for all formats: R8 alpha, RGBA subpixel, and RGBA color.

If edge duplication is too much for the first patch, use transparent padding and
UVs inset to texel centers. That is less ideal and should be treated as a
temporary step.

### Page Size

Use a conservative default:

- `2048 x 2048` for broad compatibility.
- Consider `4096 x 4096` after checking adapter limits.

The page size should be a small config struct, not a magic constant scattered
through the atlas:

```rust
pub struct GlyphAtlasConfig {
    pub page_size: u32,
    pub padding: u32,
    pub max_pages_per_material: usize,
}
```

Construct it through `GlyphAtlasConfig::default_for_device(&wgpu::Device)` if
adapter limits are available at construction time. Otherwise use a hard default
and keep the type ready for future tuning.

## Cache Entries

Keep separate key types for single and composed glyphs, but unify the cached
value:

```rust
struct CachedAtlasGlyph {
    entry: AnyAtlasEntry,
    advance_width: f32,
    last_accessed_generation: u64,
}
```

Prefer returning an owned/copyable handle:

```rust
#[derive(Debug, Clone, Copy)]
pub struct GlyphAtlasHandle {
    entry: AnyAtlasEntry,
    advance_width: f32,
}
```

Do not return `&CachedAtlasGlyph` to renderer code that will continue calling
into `glyph_atlas`. A copied handle avoids borrow checker pressure and prevents
long-lived immutable borrows from blocking later cache misses.

Whitespace behavior can remain `None`. If it becomes confusing, make it
explicit:

```rust
pub enum GlyphLookup {
    Resident(GlyphAtlasHandle),
    Empty,
}
```

That is clearer than `Option` at call sites, but it is not mandatory for the
first version.

## Renderer Draw Planning

### Preserve Visual Order

The current simple-glyph batching sorts glyphs by key. The atlas version should
not do that.

Instead:

1. Iterate `FrameGlyph::Char` in the same order used today.
2. Resolve/cache the glyph in the atlas.
3. Build vertices with atlas UVs.
4. Append a prepared glyph item in visual order.
5. Build draw runs by splitting only when material or page changes.

This keeps overlapping glyphs, overhangs, and mixed-color runs correct.

### Draw Item Types

Use an enum to connect material to vertex type:

```rust
enum PreparedGlyph {
    Alpha {
        page: PageId<AlphaMask>,
        vertices: [GlyphVertex; 6],
    },
    Subpixel {
        page: PageId<SubpixelMask>,
        vertices: [SubpixelGlyphVertex; 6],
    },
    Color {
        page: PageId<ColorRgba>,
        vertices: [GlyphVertex; 6],
    },
}
```

Then build material-specific buffers/runs:

```rust
struct DrawRun<M: GlyphMaterial> {
    page: PageId<M>,
    vertex_range: Range<u32>,
}

struct TextDrawPlan {
    alpha_vertices: Vec<GlyphVertex>,
    alpha_runs: Vec<DrawRun<AlphaMask>>,
    subpixel_vertices: Vec<SubpixelGlyphVertex>,
    subpixel_runs: Vec<DrawRun<SubpixelMask>>,
    color_vertices: Vec<GlyphVertex>,
    color_runs: Vec<DrawRun<ColorRgba>>,
}
```

Important: if separating vertices by material reorders alpha/subpixel/color
relative to each other, only do that where the existing rendering order already
separates those materials. The current renderer already draws mask, then
subpixel, then color categories. This plan preserves that category order unless
a later correctness audit says mixed material order must be exact.

Within each material, do not sort. Split runs in the encountered order:

```rust
fn push_alpha(plan: &mut TextDrawPlan, page: PageId<AlphaMask>, verts: [GlyphVertex; 6]) {
    let start = plan.alpha_vertices.len() as u32;
    plan.alpha_vertices.extend_from_slice(&verts);
    let end = plan.alpha_vertices.len() as u32;

    match plan.alpha_runs.last_mut() {
        Some(run) if run.page == page && run.vertex_range.end == start => {
            run.vertex_range.end = end;
        }
        _ => plan.alpha_runs.push(DrawRun {
            page,
            vertex_range: start..end,
        }),
    }
}
```

This gets one draw for a one-page material pass and remains correct when pages
alternate.

### Bind Groups

The renderer should not store bind groups inside glyph entries. It should ask
the atlas for the page bind group immediately before drawing:

```rust
let bind_group = glyph_atlas.bind_group(run.page);
render_pass.set_bind_group(1, bind_group, &[]);
render_pass.draw(run.vertex_range.clone(), 0..1);
```

Typed `PageId<M>` should select the typed accessor:

```rust
glyph_atlas.alpha_bind_group(page_id);
glyph_atlas.subpixel_bind_group(page_id);
glyph_atlas.color_bind_group(page_id);
```

If the public API needs one method, use exhaustive matching on a runtime enum at
the boundary only.

## Dynamic Vertex Buffers

After atlas rendering works, replace recurring `create_buffer_init` calls for
glyph vertices with reusable buffers:

```rust
struct DynamicVertexBuffer<T: bytemuck::Pod> {
    buffer: wgpu::Buffer,
    capacity: usize,
    label: &'static str,
    _marker: PhantomData<T>,
}

impl<T: bytemuck::Pod> DynamicVertexBuffer<T> {
    fn upload<'a>(
        &'a mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[T],
    ) -> wgpu::BufferSlice<'a>;
}
```

The buffer should grow geometrically and reuse capacity across frames. Use
`queue.write_buffer` for uploads. If a frame exceeds capacity, create a larger
buffer. Do not shrink every frame.

This may require changing some renderer methods from `&self` to `&mut self`.
That is acceptable for core rendering paths. Do it in a dedicated patch after
the atlas path is correct.

## Refactor Sequence

### Step 1: Add Renderer Statistics

Add low-overhead per-frame counters before changing behavior.

Track:

- total frame glyphs;
- text glyphs;
- composed glyphs;
- unique single glyph keys;
- unique composed glyph keys;
- glyph texture uploads;
- glyph atlas page uploads after the refactor;
- glyph draw calls;
- glyph bind-group changes;
- glyph vertex buffer creations;
- composed glyph draw calls;
- atlas pages by material;
- cache hit/miss counts.

Add counters around:

- main frame glyph draw path;
- child-frame content draw path;
- overlay text helper;
- glyph atlas cache miss/upload path.

Make the counters easy to log with an environment variable such as
`NEOMACS_RENDER_STATS=1`. Keep default overhead minimal.

Acceptance:

- Existing behavior unchanged.
- Stats show current draw-call baseline on a normal text frame.
- Stats can distinguish simple glyph draws from composed glyph draws.

### Step 2: Add Pure Atlas Types and Allocator

Add the type model and allocator without switching renderer behavior yet.

Files likely touched:

- `neomacs-renderer-wgpu/src/glyph_atlas.rs`, or split into:
  - `glyph_atlas/mod.rs`
  - `glyph_atlas/types.rs`
  - `glyph_atlas/allocator.rs`
  - `glyph_atlas/pages.rs`

Keep the first split modest. If `glyph_atlas.rs` is already hard to navigate,
split now. If that causes too much churn, add private modules later.

Acceptance:

- Allocator tests pass.
- Type constructors reject invalid zero sizes and oversized allocations.
- No renderer behavior changes yet.

### Step 3: Implement Atlas Pages Behind New APIs

Add page-backed lookup APIs next to the existing APIs. Do not delete the legacy
per-glyph texture path until the renderer has moved.

Possible temporary API:

```rust
pub fn get_or_create_atlas(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    key: &GlyphKey,
    face: Option<&Face>,
    subpixel: SubpixelRequest,
) -> GlyphLookup;
```

And:

```rust
pub fn get_or_create_composed_atlas(...) -> GlyphLookup;
```

Do not expose page internals. Return handles.

Acceptance:

- Existing tests still pass.
- New tests prove single and composed glyphs can be rasterized, allocated,
  uploaded, cached, and looked up without creating per-glyph bind groups.
- Cache hit does not upload again.
- Scale factor and metric changes clear page-backed cache correctly.

### Step 4: Convert Main Frame Text Rendering

Convert the text section in the main renderer.

Rules:

- Remove global key sorting from the atlas path.
- Use atlas UVs in vertices.
- Build draw plans in visual order.
- Split draw runs only on material/page change.
- Preserve the existing layer order: backgrounds, cursor background/trail,
  non-overlay text, overlay backgrounds, overlay text, decorations, media,
  front cursors/borders.
- Preserve current clipping behavior by adjusting both vertex positions and
  atlas UVs.

Do main-frame simple and composed glyphs together. Composed glyphs should not
remain on a separate per-glyph draw path.

Acceptance:

- A plain ASCII frame renders correctly.
- Composed glyphs render correctly.
- Stats show main-frame composed glyph draw calls are no longer one per
  composed glyph.
- Stats show a common one-page text pass draws once per material/page run.

### Step 5: Convert Child-Frame Content Rendering

Apply the same draw planner to `render_frame_content`.

This should share code with the main text path. Avoid copying the full batching
logic into `content.rs`.

Good shared unit boundary:

```rust
fn collect_text_draw_plan(
    renderer_state: TextRenderState<'_>,
    frame: &FrameGlyphBuffer,
    glyph_atlas: &mut WgpuGlyphAtlas,
    faces: &HashMap<u32, Face>,
    selection: TextLayerSelection,
) -> TextDrawPlan;
```

`TextLayerSelection` should be an enum:

```rust
enum TextLayerSelection {
    All,
    NonOverlay,
    Overlay,
}
```

Avoid a `want_overlay: bool` argument in new shared code.

Acceptance:

- Child frames with plain text render correctly.
- Child frames with composed glyphs render correctly.
- Rounded-corner stencil path still works.
- Stats include child-frame text draw calls.

### Step 6: Convert Overlay Text Helpers

Update overlay text helpers that draw window watermarks, breadcrumbs, FPS, menu
text, popup menu text, tooltip text, and IME preedit text.

They should use the same atlas handle type and visual-order run builder where
possible. Some helper paths may have simpler positioning, but they should not
reintroduce per-glyph texture resources.

Acceptance:

- UI overlays render correctly.
- Overlay text stats do not show per-glyph bind groups.

### Step 7: Add Dynamic Vertex Buffers

Replace hot-path glyph `create_buffer_init` calls with persistent vertex
buffers.

Do this after correctness is stable so performance changes are isolated.

Acceptance:

- Renderer stats show glyph vertex buffer creations are not proportional to
  frame count in steady state.
- Buffer growth handles large files without panics.
- No stale-buffer visual artifacts after resize or scale factor changes.

### Step 8: Remove Legacy Per-Glyph Texture Path

After main, child, and overlay text use atlas pages:

- Delete per-glyph `texture`, `view`, and `bind_group` fields from cached glyphs.
- Delete old lookup APIs or make them wrappers around atlas handles if needed.
- Update comments so `WgpuGlyphAtlas` accurately describes page-backed atlas
  storage.

Acceptance:

- No text path creates per-glyph textures.
- `rg "Glyph Texture|Composed Glyph Texture|Glyph Bind Group"` no longer finds
  live per-glyph allocation labels except in tests or migration notes.

### Step 9: Optional Icon Batching

Toolbar and compact-bar icons currently have per-item buffer/draw behavior.
After text is fixed, batch icons by texture page or at least by contiguous
texture runs. This is lower priority and can be a follow-up.

## Eviction Strategy

Use a conservative first version:

1. Prefer creating a new page when a material page is full.
2. Keep `max_pages_per_material`.
3. Do not evict pages used in the current frame generation.
4. If over the cap, evict the least-recently-used page for that material after
   the frame or before the next allocation.
5. Remove every cache entry that points to the evicted page.

Do not implement individual glyph eviction inside a page in the first version.
That creates fragmentation and stale-rect risk. Page-level eviction is blunt but
much easier to reason about.

Represent eviction results explicitly:

```rust
pub enum PageAllocationResult<M: GlyphMaterial> {
    Allocated {
        page: PageId<M>,
        rect: AtlasContentRect,
    },
    NeedNewPage,
    GlyphTooLarge(PixelSize),
}
```

Avoid returning bare `Option` where the caller needs to know whether to create a
new page or reject an oversized glyph.

## Error Handling

Atlas upload failures should not silently render corrupt text.

Use a small error enum:

```rust
#[derive(Debug, thiserror::Error)]
pub enum GlyphUploadError {
    #[error("glyph has zero size")]
    ZeroSize,
    #[error("glyph {glyph:?} is larger than atlas page {page_size}")]
    GlyphTooLarge { glyph: PixelSize, page_size: u32 },
    #[error("pixel buffer length mismatch for {material:?}: expected {expected}, got {actual}")]
    PixelDataLength {
        material: GlyphMaterialKind,
        expected: usize,
        actual: usize,
    },
}
```

Do not panic on ordinary glyph failures. Return `GlyphLookup::Empty` or log a
warning as the current code does. Panic only for internal invariant violations
that indicate a bug in the atlas implementation.

## Shader Changes

Existing glyph shaders sample `texture_2d<f32>` using vertex UVs. Atlas pages
can keep the same bind group layout:

- binding 0: atlas page texture;
- binding 1: sampler.

The main shader change is not conceptual; UVs now point to an atlas subrect
instead of `[0, 0]..[1, 1]` for a per-glyph texture.

Keep separate shaders/pipelines:

- alpha mask glyph pipeline;
- subpixel glyph pipeline;
- color/image pipeline.

Do not merge these in the first refactor. The material enum should choose the
existing pipeline.

## Testing Plan

### Pure Unit Tests

Add allocator tests:

- first allocation starts at expected padded rect;
- adjacent allocations do not overlap;
- shelf wraps when width is exceeded;
- allocator rejects oversized glyphs;
- allocator fills multiple shelves;
- content rect is inside allocation rect;
- padding is applied exactly once;
- UV rect maps to content rect, not padding rect.

Add type/constructor tests:

- `PixelSize::new(0, h)` fails;
- `PixelSize::new(w, 0)` fails;
- `AtlasContentRect::new` rejects zero dimensions;
- invalid page ids cannot be constructed without `NonZeroU32`;
- material byte-size validation catches short/long pixel buffers.

Add cache-key tests:

- different `font_identity` values produce different entries;
- different `font_size_bits` values produce different entries;
- different `x_bin`/`y_bin` values produce different entries;
- alpha and subpixel render modes do not collide;
- single and composed glyph caches do not collide.

Add draw-plan tests as pure logic:

- one page, one material, many glyphs -> one run;
- page A, page A, page B -> two runs;
- page A, page B, page A -> three runs, preserving order;
- alpha and subpixel items go to separate vertex collections;
- composed glyphs enter the same plan as single glyphs;
- overstrike produces two glyph items or twelve vertices as expected.

### WGPU/Atlas Tests

Where the existing test harness can create a device:

- create atlas page for each material;
- upload a small alpha glyph;
- upload a small subpixel glyph;
- upload a small color glyph;
- cache hit returns same page and UV;
- page overflow creates a second page;
- scale-factor change clears pages and cache;
- metric change clears pages and cache if the current behavior requires it.

Do not require pixel-perfect screenshot tests for these low-level tests. The
goal is resource correctness.

### Renderer Regression Tests

Add or extend renderer tests around the pure draw planner. Full GPU rendering
tests may be difficult in CI, so keep most checks pure:

- non-overlay text selection excludes chrome rows;
- overlay text selection includes mode/header/tab rows;
- clipping adjusts UVs and positions consistently;
- inverse-video cursor foreground/background override still changes vertex
  color;
- subpixel vertices carry background and foreground colors;
- color glyph vertices use white tint with alpha fade.

### Manual Visual Test Matrix

Run these manually before merging the renderer conversion:

- Plain ASCII file, large enough to fill the window.
- Rust source file with many punctuation characters.
- Mixed face colors in one line.
- Bold/overstrike fallback.
- Italic/overhang-heavy text such as `ffff`, `www`, `///`, `VA`.
- Combining marks.
- Emoji and emoji ZWJ sequences.
- RTL text.
- Mixed LTR/RTL text.
- Mode line, header line, tab line, tab bar.
- Minibuffer echo area.
- Child frames, including rounded child frames.
- Cursor filled box, bar, hbar, hollow.
- Cursor inverse-video on the glyph under point.
- HiDPI scale factor if available.
- Scale factor change while running.
- Theme/font change causing face remapping.
- Subpixel enabled host.
- Subpixel disabled/fallback host.

### Performance Checks

Before and after, collect stats for the same scenarios:

- empty startup frame;
- large ASCII file;
- large Rust file;
- mixed Unicode file;
- dashboard/mode-line heavy frame;
- child-frame popup/menu case.

Record:

- total frame glyphs;
- text glyphs;
- unique glyph keys;
- atlas pages by material;
- glyph draw calls;
- glyph bind-group changes;
- glyph texture uploads/page uploads;
- glyph vertex buffer creations;
- frame render time if available.

Expected result:

- simple one-page text pass: draw calls near one per material/page run;
- composed glyphs: no longer one draw per composed glyph;
- vertex buffer creations: no longer proportional to glyph count after dynamic
  buffers land;
- no statistically significant regression in first-frame glyph upload beyond
  atlas page allocation overhead.

## Suggested Commands

Narrow tests first:

```sh
cargo test -p neomacs-renderer-wgpu glyph_atlas
cargo test -p neomacs-renderer-wgpu renderer::glyphs
cargo test -p neomacs-renderer-wgpu renderer::content
```

Then broader checks:

```sh
cargo test -p neomacs-renderer-wgpu
cargo test -p neomacs-display-protocol
cargo test -p neomacs-display-runtime
```

If GUI/manual testing is available, run with stats enabled:

```sh
NEOMACS_RENDER_STATS=1 cargo run -p neomacs-bin -- --gui
```

Adjust the exact binary flags to the current project entry point.

## Acceptance Criteria

The refactor is complete when:

- `WgpuGlyphAtlas` stores glyphs in atlas pages, not per-glyph textures.
- Single and composed glyphs share the same atlas-backed draw path.
- Main frame text no longer globally sorts glyphs by key to batch.
- Main frame text draw calls are proportional to material/page runs.
- Child-frame content uses the same atlas-backed path.
- Overlay text uses atlas-backed rendering or has a documented follow-up.
- Renderer stats prove draw calls dropped on representative frames.
- Unit tests cover allocator, key separation, page overflow, cache clear,
  draw-run planning, and composed glyph handling.
- Manual visual checks pass for ASCII, Unicode, composed, RTL, cursor, overlay,
  and child-frame cases.
- Legacy per-glyph texture fields and bind-group creation labels are removed
  from live text rendering code.

## Main Risks

### Texture Bleeding

Linear sampling can bleed neighboring glyph pixels. Mitigate with padding and
edge duplication. Test with high-contrast glyphs and small punctuation.

### Visual Reordering

Sorting glyphs by key is unsafe for overlapping glyphs. The atlas path should
preserve visual order and only split contiguous runs.

### Borrow Checker Pressure

Returning references from atlas lookup while continuing to mutate the atlas will
make renderer code awkward. Return copyable handles instead.

### Page Eviction During Current Frame

Do not evict a page that is referenced by the draw plan being built. Use frame
generations and avoid eviction of pages touched in the current generation.

### Mixed Material Ordering

The current renderer already separates mask/subpixel/color paths. Preserve that
initially. If a later visual bug shows material ordering matters within a row,
the draw planner can become a single ordered list of material runs instead of
separate material vectors.

### API Churn

Main and child text paths currently duplicate logic. Use the refactor to create
shared draw-planning helpers, but avoid a broad renderer architecture rewrite.

## Review Checklist For Each Patch

- Does this patch compile with exhaustive matches instead of adding boolean
  branches?
- Are invalid sizes/rectangles impossible to construct directly?
- Does any new API return a bare `Option` where callers need a reason?
- Does renderer code preserve visual order?
- Does this introduce a per-glyph GPU texture, bind group, buffer, or draw?
- Does a cache hit avoid upload?
- Are scale factor and metric invalidation still correct?
- Are tests added at the level where the invariant lives?
- Are stats updated so performance claims are measurable?

## Recommended Patch Breakdown

1. Stats only.
2. Atlas type model and allocator tests.
3. Page-backed atlas upload/cache APIs.
4. Main frame text conversion, including composed glyphs.
5. Child-frame text conversion.
6. Overlay text conversion.
7. Dynamic glyph vertex buffers.
8. Legacy path deletion and comments cleanup.
9. Optional toolbar/compact-bar icon batching.

Each patch should compile and preserve behavior except where it intentionally
switches a specific renderer path to atlas pages.
