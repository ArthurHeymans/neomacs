# Font Realization and Render Boundary Design

Date: 2026-07-05
Status: Draft

## 1. Problem Statement

Neomacs currently has two semantic font-selection paths for GUI text:

```text
Evaluator / layout thread
  face attributes, font-at, find-font, metrics, row layout
  -> FontMetricsService / fontconfig / fontset-like policy
  -> Lisp-visible and layout-visible selected font information

Render thread
  FrameGlyph + Face family/weight/slant/size
  -> glyph_atlas::face_to_attrs_for_text
  -> fontconfig fallback + cosmic-text/fontdb family selection
  -> rasterized glyphs
```

This makes the display pipeline capable of answering one font to Lisp/layout code
and drawing another font on screen. The immediate symptom is that Treemacs text can
look too wide or otherwise unlike GNU Emacs even when Lisp-level font oracle checks
appear plausible.

The current display protocol already has a partial bridge:

- `Face::font_file_path: Option<String>`
- renderer glyph-cache identity hashes `font_file_path`
- glyph atlas can prime a font file when the path is present

But pure-Rust layout normally drops that identity:

- `DisplayRowFace::from_resolved` sets `font_file_path: None`
- `FrameGlyphBuffer::synthesize_face` sets `font_file_path: None`

So `None` is the common GUI text path, and the render thread performs a fresh
semantic font decision from family/weight/slant.

## 2. Design Principle

Semantic text realization must happen before the render thread.

The render thread may ask:

```text
How do I rasterize this exact resolved font/glyph?
```

The render thread must not ask:

```text
Which font should this face or character use?
```

This is stricter than "pass a font file to the renderer." Font selection and
text shaping are coupled. Emoji variation selectors, CJK fallback, Arabic/Indic
shaping, ligatures, and fontset fallback can all change glyph IDs, cluster
boundaries, positions, and advances. If layout selects one font but the renderer
reshapes from family/weight/style, the bug class remains.

## 3. Target Architecture

Long term, the pipeline should be:

```text
Face attrs + text + frame/fontset context
        |
        v
Shared Emacs-compatible font policy
        |
        v
Platform font backend
  Linux: fontconfig
  macOS: CoreText
  Windows: DirectWrite
        |
        v
ResolvedFontIdentity + ResolvedFont metrics
        |
        v
Exact-font shaper
  HarfBuzz / rustybuzz / platform shaper behind one trait
        |
        v
Resolved shaped glyph stream
        |
        v
Render thread
  open/cache exact font handles
  rasterize exact glyph IDs
  manage atlas/GPU resources
```

The shared policy layer owns GNU/Emacs compatibility. Platform backends only
enumerate/open candidate fonts and provide exact font metadata.

## 4. Ownership Split

Evaluator/layout owns:

- face inheritance, remapping, and face IDs
- fontset lookup and fallback order
- generic family alias handling
- alternative font family and registry alists
- fontconfig/CoreText/DirectWrite candidate enumeration through backend traits
- GNU-compatible candidate scoring
- character coverage decisions
- weight/slant/width normalization
- exact font identity creation
- shaping into glyph IDs, cluster ranges, offsets, and advances
- Lisp-visible APIs: `font-at`, `find-font`, `face-font`, `font-info`
- row metrics and cursor/layout geometry

Render thread owns:

- exact font-handle cache
- glyph rasterization for `(font identity, glyph id, size, variation coords)`
- glyph atlas allocation and eviction
- subpixel bins and mask format decisions
- color glyph upload
- GPU buffers, draw ordering, clipping, effects, and compositing

Render thread does not own:

- fontconfig/CoreText/DirectWrite fallback decisions
- family alias resolution
- fontset lookup
- weight/slant/width substitute selection
- emoji/CJK fallback choice
- shaping by family/weight/style request

## 5. Core Types

### 5.1 FontRequest

`FontRequest` is an input to the resolver. It should not cross the render boundary
as drawable truth.

```rust
pub struct FontRequest {
    pub frame_id: DisplayFrameId,
    pub face_id: u32,
    pub family: Option<String>,
    pub foundry: Option<String>,
    pub registry: Option<String>,
    pub weight: FontWeight,
    pub slant: FontSlant,
    pub width: FontWidth,
    pub pixel_size: f32,
    pub dpi: f32,
    pub character: Option<char>,
    pub script: Option<Script>,
}
```

### 5.2 ResolvedFontIdentity

`ResolvedFontIdentity` is an exact, platform-openable identity. Do not define this
as "file path only"; macOS and Windows may need native descriptors or stable
backend keys.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ResolvedFontIdentity {
    pub backend: FontBackendKind,
    pub stable_key: String,
    pub file_path: Option<PathBuf>,
    pub face_index: u32,
    pub postscript_name: Option<String>,
    pub collection_index: Option<u32>,
    pub variation_coords: Vec<FontVariationCoord>,
}
```

Linux can usually populate `file_path + face_index`. macOS can use CoreText font
URLs/descriptors. Windows can use DirectWrite face identity, with file paths only
when reliably available.

### 5.3 ResolvedFont

`ResolvedFont` is the resolver's canonical answer for a concrete font instance.

```rust
pub struct ResolvedFont {
    pub id: ResolvedFontId,
    pub identity: ResolvedFontIdentity,
    pub family: String,
    pub full_name: Option<String>,
    pub postscript_name: Option<String>,
    pub weight: FontWeight,
    pub slant: FontSlant,
    pub width: FontWidth,
    pub pixel_size: f32,
    pub metrics: FontMetrics,
    pub source: FontResolutionSource,
}
```

`source` should distinguish primary face font, fontset fallback, emoji fallback,
platform fallback, and emergency fallback. This is important for traces and oracle
debugging.

### 5.4 ResolvedGlyph

`ResolvedGlyph` is the renderable unit. It is already past semantic selection and
shaping.

```rust
pub struct ResolvedGlyph {
    pub resolved_font_id: ResolvedFontId,
    pub glyph_id: u32,
    pub cluster_start: usize,
    pub cluster_end: usize,
    pub x_offset: f32,
    pub y_offset: f32,
    pub x_advance: f32,
    pub y_advance: f32,
}
```

### 5.5 ShapedTextRun

`ShapedTextRun` is the strongest display contract between layout and render.

```rust
pub struct ShapedTextRun {
    pub text: Box<str>,
    pub text_range: Range<usize>,
    pub face_id: u32,
    pub glyphs: Vec<ResolvedGlyph>,
    pub direction: TextDirection,
}
```

For simple ASCII this is still one glyph per character. For complex clusters it
can contain multiple glyphs and cluster mappings.

## 6. Display Protocol Shape

Frame state should carry a resolved font table:

```rust
pub struct FrameDisplayState {
    pub fonts: HashMap<ResolvedFontId, ResolvedFont>,
    pub faces: HashMap<u32, Face>,
    pub text_runs: Vec<DisplayTextRun>,
    // existing backgrounds, cursors, images, videos, borders, etc.
}
```

`Face` remains the visual face record: colors, decorations, default font id,
metrics, Lisp name. It should stop being treated as a render-time font request.

```rust
pub struct Face {
    pub id: u32,
    pub foreground: Color,
    pub background: Color,
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub default_resolved_font_id: Option<ResolvedFontId>,
    // existing decoration fields
}
```

Final renderable text should reference shaped glyphs:

```rust
pub struct DisplayTextRun {
    pub window_id: DisplayWindowId,
    pub row_role: GlyphRowRole,
    pub clip_rect: Option<Rect>,
    pub slot_range: Range<DisplaySlotId>,
    pub face_id: u32,
    pub x: f32,
    pub y: f32,
    pub baseline: f32,
    pub glyphs: Vec<ResolvedGlyph>,
}
```

During migration, existing `FrameGlyph::Char` can remain. Add resolved font
identity first at the face table, then introduce run/glyph-level shaped output.

## 7. Platform Backend Abstraction

The resolver should be shared. Candidate enumeration and opening should be
platform-specific.

```rust
pub trait FontBackend {
    fn list_candidates(&mut self, request: &FontRequest) -> Vec<FontCandidate>;
    fn open_font(&mut self, identity: &ResolvedFontIdentity) -> Result<FontHandle, FontError>;
    fn metrics(&mut self, handle: &FontHandle, size: f32) -> Result<FontMetrics, FontError>;
    fn supports_char(&mut self, handle: &FontHandle, ch: char) -> bool;
    fn font_bytes(&mut self, handle: &FontHandle) -> Result<FontDataRef<'_>, FontError>;
}
```

Backend implementations:

- `LinuxFontBackend`: fontconfig candidate list, file/index identities.
- `MacFontBackend`: CoreText descriptors, font URLs, collection indexes, native
  descriptors where paths are insufficient.
- `WindowsFontBackend`: DirectWrite families/faces, axis metadata, native face
  identity where paths are insufficient.

The shared resolver performs Emacs-compatible ordering and scoring over
`FontCandidate`.

## 8. Shaper Abstraction

Do not expose `cosmic-text` as the architectural contract. Hide shaping behind a
trait that consumes an exact `ResolvedFont`.

```rust
pub trait TextShaper {
    fn shape_run(
        &mut self,
        font_store: &mut FontStore,
        font: &ResolvedFont,
        text: &str,
        options: ShapeOptions,
    ) -> Result<Vec<ResolvedGlyph>, ShapeError>;
}
```

The initial implementation may use current `cosmic-text` machinery if it can be
constrained to exact font identity. The long-term implementation should prefer
HarfBuzz/rustybuzz over exact font bytes/face index so layout and renderer never
re-run high-level font selection.

## 9. Renderer API

Renderer glyph atlas APIs should move away from `Face`.

Current shape:

```rust
get_or_create_atlas(key: &GlyphKey, face: Option<&Face>, ...)
```

Target shape:

```rust
get_or_create_glyph(
    font: &ResolvedFont,
    glyph: &ResolvedGlyph,
    pixel_size: f32,
    subpixel: SubpixelRequest,
) -> Option<GlyphAtlasHandle>
```

Atlas cache key should be:

```rust
pub struct GlyphAtlasKey {
    pub resolved_font_id: ResolvedFontId,
    pub glyph_id: u32,
    pub pixel_size_bits: u32,
    pub variation_key: VariationKey,
    pub x_bin: SubpixelBin,
    pub y_bin: SubpixelBin,
    pub render_mode: GlyphRenderMode,
}
```

It should not contain family/weight/slant as selection inputs. Those are already
resolved into `ResolvedFontId`.

## 10. Handling Unresolved Fonts

Unresolved GUI text should become abnormal.

Migration policy:

1. Initially warn when GUI text reaches renderer without a resolved font identity.
2. Add a counter and trace target for unresolved/emergency fallback usage.
3. Gate tests on zero unresolved normal text in representative GUI snapshots.
4. Eventually reject unresolved text in debug builds.

Emergency fallback should be explicit:

```rust
fn emergency_unresolved_font_fallback(face: &Face, text: &str) -> ResolvedFont {
    tracing::error!(
        face_id = face.id,
        family = %face.font_family,
        text = %text,
        "unresolved GUI text reached render thread; using emergency font fallback"
    );
    // keep UI alive, but mark result as FontResolutionSource::EmergencyFallback
}
```

The normal renderer path must not call fontconfig/CoreText/DirectWrite to answer
"which font should this use?"

## 11. Compatibility With GNU Emacs

GNU Emacs has multiple callers, but they converge on realized face/font objects:

```text
face attributes / font spec
  -> font_find_for_lface
  -> font_open_for_lface
  -> realized face->font

font-at / display / fallback
  -> same realization machinery
```

Neomacs should mirror that property, even though it has a render thread. The
render thread may own GPU resources and thread-local font handles, but it should
consume the same realized font result that Lisp-visible APIs report.

The desired invariant:

```text
(font-at ...)
(find-font ...)
layout advances
actual rendered glyphs
```

must agree on resolved font identity for normal text.

## 12. Migration Plan

### Phase 0: Instrument Current Divergence

- Add trace logs when layout emits GUI `Face` records with `font_file_path == None`.
- Add trace logs when `glyph_atlas::face_to_attrs_for_text` invokes fallback or
  `match_font_for_char`.
- Capture requested face fields and actual selected font file/postscript from the
  renderer for Treemacs repro frames.

### Phase 1: Carry Face-Level Resolved Font Identity

- Add `ResolvedFontId`, `ResolvedFontIdentity`, and `ResolvedFont` to
  `neomacs-display-protocol`.
- Add `FrameDisplayState::fonts`.
- Add `Face::default_resolved_font_id`.
- Populate default face font identity from the existing layout/binary
  `FontMetricsService` resolution path.
- Preserve `font_file_path` as a temporary Linux bridge, but do not treat it as
  the final abstraction.

### Phase 2: Make Renderer Font Selection Emergency-Only

- Change renderer code to prefer `ResolvedFontId` over family/weight/slant.
- Move current `glyph_atlas::face_to_attrs_for_text` semantic fallback behind
  `emergency_unresolved_font_fallback`.
- Warn or error when normal GUI text lacks resolved identity.
- Update glyph cache identity to use `ResolvedFontId` and font generation.

### Phase 3: Introduce Exact Shaped Runs

- Add `ShapedTextRun` / `ResolvedGlyph` display protocol types.
- Teach layout text-run measurement to retain glyph IDs, cluster ranges, offsets,
  advances, and `ResolvedFontId`.
- Emit shaped runs for GUI text while retaining existing char glyphs for fallback
  and incremental migration.
- Add tests proving the shaped run used for layout is the run passed to render.

### Phase 4: Rasterize Exact Glyph IDs

- Change glyph atlas APIs to consume `ResolvedFont + ResolvedGlyph`.
- Open/cache exact platform font handles by `ResolvedFontIdentity`.
- Rasterize exact glyph IDs.
- Remove normal renderer calls to `fontconfig`, `font_match`, and family-based
  `cosmic-text` selection.

### Phase 5: Cross-Platform Font Backends

- Extract current Linux fontconfig code into `LinuxFontBackend`.
- Add backend trait and shared `FontResolver`.
- Add `MacFontBackend` using CoreText descriptors.
- Add `WindowsFontBackend` using DirectWrite identities.
- Keep scoring/policy shared and platform-neutral.

### Phase 6: Replace High-Level Cosmic Contract

- Hide any remaining `cosmic-text` usage behind `TextShaper`.
- Prefer HarfBuzz/rustybuzz shaping over exact font bytes/face index.
- Delete duplicated layout/render `FontSystem` semantic selection.

## 13. Testing Strategy

Unit tests:

- `ResolvedFontIdentity` equality and hash stability.
- Font candidate scoring independent of backend.
- `Face` with resolved default font survives frame protocol round trip.
- Glyph atlas key changes when `ResolvedFontId`, glyph id, variation coords, or
  subpixel bins change.

Integration tests:

- `font-at` result file/postscript equals frame text run `ResolvedFont`.
- `find-font` result agrees with resolver candidate chosen for matching face.
- Treemacs-like bold/regular Noto/monospace rows emit non-`None`
  `ResolvedFontId`.
- Emoji variation-selector clusters use emoji resolved font identity.
- CJK fallback emits glyph-level fallback `ResolvedFontId`, not face default.

Oracle tests:

- Compare GNU Emacs and Neomacs `font-at` / `find-font` at Lisp level.
- Verify renderer trace for the same text uses the same resolved identity.
- Treat renderer emergency fallback count as a failure for normal GUI oracle runs.

Visual/regression tests:

- Treemacs sample frame screenshot.
- Mixed ASCII/CJK/emoji frame screenshot.
- Text-scale and face-remap sample.
- Variable font weight sample, if available.

## 14. Non-Goals

- Do not make the render thread own GNU-compatible font policy.
- Do not make `file_path` the only durable font identity.
- Do not require every platform to expose a stable filesystem path.
- Do not remove `cosmic-text` before the resolver/shaper abstraction exists.
- Do not mix unrelated display row or render-thread refactors into this migration.

## 15. Open Questions

- Should the initial shaped-run protocol coexist with `FrameGlyph::Char`, or
  should it replace char glyphs for GUI text immediately?
- Should `ResolvedFontId` be frame-local, process-global, or generation-scoped?
  Frame-local is simplest for protocol snapshots; process-global may improve
  renderer cache reuse.
- Should shaping live in `neomacs-layout-engine` or a new `neomacs-font` crate?
  A new crate may be cleaner once macOS/Windows backends arrive.
- How much GNU font scoring should live in `neovm-core` versus layout/font crate?
  Lisp-visible APIs need access, but render/layout should not depend on evaluator
  internals unnecessarily.
- Can `cosmic-text` be constrained to exact font identity well enough for Phase 3,
  or should the first shaped-run implementation use HarfBuzz/rustybuzz directly?

## 16. Success Criteria

- Renderer normal text path has no semantic font selection.
- GUI text frame snapshots carry resolved font identities.
- `font-at`, `find-font`, layout metrics, and rendered glyphs agree on font
  identity for normal text.
- Renderer emergency fallback usage is zero in standard GUI oracle runs.
- The design supports Linux/fontconfig, macOS/CoreText, and Windows/DirectWrite
  without making Linux file paths the protocol contract.
