# Replacing librsvg/Cairo with resvg in Neomacs

Research date: 2026-07-18

This note evaluates the current `resvg`/`usvg`/`tiny-skia` stack as a
replacement for Neomacs's librsvg/Cairo SVG path. It is based on upstream
documentation, source, releases, and first-party issue reports. Statements
labelled **Inference** describe consequences for Neomacs rather than upstream
guarantees.

## Executive summary

Neomacs should replace librsvg/Cairo with `resvg` 0.47 and make a hard cutover
after compatibility tests pass. The replacement removes the SVG path's native
Cairo, GLib, GIO, Pango, and librsvg requirements, which resolves the conflict
between Ubuntu 22.04's Cairo 1.16 and current librsvg's Cairo 1.18 requirement.
It also gives Windows, macOS, Linux, and ARM the same Rust rendering stack.

The integration itself is small: parse bytes with `usvg::Tree::from_data`,
allocate a `tiny_skia::Pixmap`, render with a root transform, and return
demultiplied RGBA pixels. Four policies must not be delegated to defaults:

1. Preserve Neomacs/GNU Emacs natural-size behavior rather than treating
   `usvg::Tree::size()` as equivalent in every edge case.
2. Load and cache fonts intentionally. Compiling system-font support does not
   populate the font database, and system font sets vary by host.
3. Replace the default external-image resolver, which permits local file
   reads, with an explicit Neomacs resource policy.
4. Convert tiny-skia's premultiplied RGBA buffer to straight RGBA before wgpu
   upload.

`resvg` is not a browser. It intentionally supports static SVG, has minimal
CSS support, and has open compatibility bugs. Those limitations are acceptable
for an editor image backend if the migration is guarded by the tests described
below.

## Upstream baseline

The latest upstream release at the research date is
[`resvg` 0.47.0](https://github.com/linebender/resvg/releases/tag/v0.47.0),
released on 2026-02-10. It uses Rust edition 2024 and declares Rust 1.87 as its
minimum version. Neomacs's Rust 1.96.1 toolchain satisfies that requirement.

The default `resvg` features are `svgz`, `text`, `system-fonts`,
`memmap-fonts`, and `raster-images`. Raster-image support covers the decoders
used for JPEG, PNG, GIF, and WebP content embedded in SVGs. The crate's feature
definitions are in the
[`resvg` 0.47 manifest](https://github.com/linebender/resvg/blob/v0.47.0/crates/resvg/Cargo.toml).

The public path needed by Neomacs is:

```text
SVG bytes
    -> usvg::Tree::from_data(data, &options)
    -> tree size plus Neomacs natural-size policy
    -> tiny_skia::Pixmap
    -> resvg::render(tree, root_transform, pixmap)
    -> Pixmap::take_demultiplied()
    -> straight sRGB RGBA texture bytes
```

Primary API references:

- [`usvg::Tree`](https://docs.rs/usvg/0.47.0/usvg/struct.Tree.html)
- [`usvg::Options`](https://docs.rs/usvg/0.47.0/usvg/struct.Options.html)
- [`resvg::render`](https://docs.rs/resvg/0.47.0/resvg/fn.render.html)
- [`resvg::render_node`](https://docs.rs/resvg/0.47.0/resvg/fn.render_node.html)

`usvg::Tree` is `Clone`, `Send`, and `Sync`. Parsing and rendering are separate
stages, so parsed trees can be cached independently of output scale. The
renderer documents its output as sRGB. The project describes itself as a
portable, static SVG renderer with no native system-library dependency and a
large SVG-to-PNG regression suite
([upstream README](https://github.com/linebender/resvg)).

## Natural size, units, and bounding boxes

`usvg::Options` defaults to 96 DPI and a 100 by 100 fallback viewport. Its
unit conversion implements the expected formulas:

```text
1in = dpi
1cm = dpi / 2.54
1mm = dpi / 25.4
1pt = dpi / 72
1pc = dpi / 6
```

Source: [`units.rs`](https://github.com/linebender/resvg/blob/v0.47.0/crates/usvg/src/parser/units.rs).

At the root, missing `width` or `height` values default to `100%`. With a
`viewBox`, percentages are evaluated against the corresponding viewBox
dimension. With no `viewBox` and relative or missing root dimensions, `usvg`
starts from `Options::default_size`, lays out the document, and then replaces
the tree size using the content bounding box's right and bottom extents.

Source: [`converter.rs`](https://github.com/linebender/resvg/blob/v0.47.0/crates/usvg/src/parser/converter.rs).

`usvg` exposes several different boxes:

- object bounding box, excluding stroke;
- stroke bounding box;
- layer bounding box, which accounts for stroke, filters, clipping, and group
  children.

The distinction is documented by
[`usvg::Group`](https://docs.rs/usvg/0.47.0/usvg/struct.Group.html), and
`resvg::render_node` directs callers to `abs_layer_bounding_box()` for the
expected output extent.

### Neomacs-specific sizing risks

**Inference:** `Tree::size()` cannot replace the existing
`natural_dimensions()` policy without compatibility tests. For example:

```xml
<svg width="100" viewBox="0 0 20 40">...</svg>
```

The current Neomacs policy derives the missing height from the viewBox aspect
ratio and obtains 100 by 200. `usvg` treats the missing height as `100%` of the
viewBox height and produces a tree size of 100 by 40. This is an observable
layout change even though both results can be internally consistent render
viewports.

**Inference:** dimensionless SVGs need special attention. `usvg`'s automatic
root-size restoration uses the root absolute object bounding box, while
Neomacs currently asks librsvg for ink geometry. Strokes, markers, filters,
positive origin offsets, and negative coordinates can therefore produce
different natural sizes. Neomacs should retain its current explicit dimension
policy and define a documented fallback based on the most suitable `usvg`
box, probably layer or stroke geometry depending on GNU compatibility tests.

`usvg` flattens the root viewBox into transforms during preprocessing. If the
original width, height, and viewBox attributes are required to preserve the
existing policy, Neomacs should inspect only the root XML attributes before or
alongside `Tree::from_data`; it should not create a second SVG renderer.

## Text and fonts

The `text` feature uses `rustybuzz`, `ttf-parser`, and `fontdb`. It converts
laid-out glyphs into paths rather than delegating to native text rendering.
This removes Pango and native font rasterization from the SVG backend, but it
also means text pixels and metrics can differ from librsvg/Pango.

Important upstream behavior:

- `Options::default()` contains an empty font database.
- The caller must use `fontdb_mut().load_system_fonts()` or load explicit font
  files/data.
- `FontResolver` permits custom font-family and fallback selection.
- `fontdb` warns that system font discovery is complicated and may omit fonts
  outside the locations it scans.
- Embedded CSS `@font-face` remains unsupported because WOFF/WOFF2 support is
  missing.

Sources:

- [`usvg::FontResolver`](https://docs.rs/usvg/0.47.0/usvg/struct.FontResolver.html)
- [`fontdb::Database`](https://docs.rs/fontdb/0.23.0/fontdb/struct.Database.html)
- [Open `@font-face` issue](https://github.com/linebender/resvg/issues/541)

**Inference:** load the font database once, outside individual image decodes,
and share it with each set of parsing options. Repeated system scans would add
I/O and latency to redisplay.

**Inference:** the upstream claim of reproducible cross-platform pixels
assumes equivalent inputs. If NeoMacs loads each host's system fonts, font
selection and glyph outlines will still vary across systems. That is normal
editor behavior, but tests that require identical pixels should load a fixed
test font set.

## CSS, images, and external resources

`usvg` resolves presentation attributes, inline styles, and its supported CSS
into a simplified tree. The official documentation describes CSS support as
minimal and says unsupported SVG features are ignored. `Options::style_sheet`
can inject an additional stylesheet.

Source: [`usvg` documentation](https://docs.rs/usvg/0.47.0/usvg/).

The default image resolver supports embedded data URLs and local external
JPEG, PNG, GIF, WebP, SVG, and SVGZ files. It deliberately performs no network
requests. However, its string resolver treats an arbitrary non-data href as a
path, joins it to `resources_dir` when supplied, and reads an existing file.
Custom resolver closures can replace both data and string handling.

Source: [`image.rs`](https://github.com/linebender/resvg/blob/v0.47.0/crates/usvg/src/parser/image.rs).

**Inference:** Neomacs should not inherit the local-file default accidentally.
SVG can originate in buffers, packages, email, or downloaded content. The
initial migration should allow embedded data images and reject string hrefs.
If GNU compatibility later requires relative image files, Neomacs should pass
a controlled base directory and an explicit allow-listing resolver rather than
the process working directory.

## Pixels, alpha, and color

`tiny-skia::Pixmap` stores four-byte, premultiplied RGBA pixels. Its ordinary
`data()` and `take()` methods preserve that representation. The
`take_demultiplied()` method returns straight RGBA bytes. `resvg::render`
documents the produced color space as sRGB.

Source: [`tiny-skia` pixmap implementation](https://docs.rs/tiny-skia/0.12.0/src/tiny_skia/pixmap.rs.html).

**Inference:** Neomacs must use `take_demultiplied()` because its current
`DecodedSvg.rgba` contract is straight RGBA. Uploading `Pixmap::take()` would
darken translucent edges and produce incorrect blending.

Wide-gamut CSS colors are not a safe compatibility assumption. A current
first-party report shows an unsupported `color(display-p3 ...)` declaration
overriding an otherwise valid fallback stroke with no stroke
([issue 914](https://github.com/linebender/resvg/issues/914)). Test SVGs should
therefore include fallback declarations emitted by tools such as Figma.

## Security and resource limits

The static subset is a favorable default for an editor image backend:
`resvg` does not implement scripting, events, or animation. `usvg` detects and
removes recursive references and rejects SVGs with more than 1,000,000
elements. `resvg` also constrains some intermediate layer allocation relative
to the output canvas.

Sources:

- [Project scope and safety notes](https://github.com/linebender/resvg)
- [`usvg` parser source](https://github.com/linebender/resvg/blob/v0.47.0/crates/usvg/src/parser/mod.rs)
- [`resvg` renderer source](https://github.com/linebender/resvg/blob/v0.47.0/crates/resvg/src/lib.rs)

These checks are not a complete resource policy:

- upstream has an open request for configurable memory limits
  ([issue 815](https://github.com/linebender/resvg/issues/815));
- current SVGZ parsing expands the stream with `read_to_end` and has no
  decompressed-size argument;
- embedded data images can be much larger after decoding;
- expensive filters can consume substantial CPU and intermediate memory; and
- the default href resolver can read local files.

**Inference:** retain Neomacs's GPU dimension cap, add a maximum SVG input
size, bound output pixel count with checked arithmetic, reject uncontrolled
external resources, and decide whether SVGZ is needed. If not, disable the
`svgz` feature. A timeout or work budget is harder to enforce in-process, so
the practical first gate is strict dimensions, input sizes, and image cache
limits.

## Current compatibility reports

The following are first-party tracker reports, not claims that every SVG using
these features is broken:

- Root `<svg transform>` combined with a viewBox differs from browser output:
  [issue 899](https://github.com/linebender/resvg/issues/899).
- A transform on a nested `<svg>` is reported as applied twice in 0.47:
  [issue 1066](https://github.com/linebender/resvg/issues/1066).
- RTL multiline text using per-`tspan` `dy` offsets can collapse onto one
  baseline: [issue 1093](https://github.com/linebender/resvg/issues/1093).
- Percent-encoded non-ASCII local image paths are not resolved as decoded
  paths: [issue 1073](https://github.com/linebender/resvg/issues/1073).
- A blur-filter performance regression has been reported:
  [issue 790](https://github.com/linebender/resvg/issues/790).

These reports argue for targeted fixtures and version pinning, not for keeping
librsvg as a permanent second backend.

## Migration design

Keep the current Neomacs-facing module boundary:

```text
query_dimensions(data) -> logical natural size
decode(data, limits, realization) -> DecodedSvg
```

Inside that boundary:

1. Construct controlled `usvg::Options` with 96 DPI, a shared font database,
   and explicit image resolvers.
2. Parse the SVG once for each cache entry.
3. Determine natural dimensions using the preserved NeoMacs policy plus the
   selected `usvg` geometry fallback.
4. Apply `constrain_dimensions` and the current logical/device realization
   rules.
5. Allocate the pixmap with checked nonzero dimensions.
6. Pass a root scale transform to `resvg::render` so the complete natural
   coordinate space maps into the raster extent.
7. return `take_demultiplied()` bytes.

**Inference:** a parsed-tree cache is worthwhile because `Tree` is `Send` and
`Sync`, and the same image may be rendered at different device scales. Cache
keys must include SVG bytes and all parse-affecting policy: DPI, font database
revision, language, injected CSS, and external-resource base or resolver mode.

Do not maintain librsvg and resvg as permanent runtime-selectable backends.
That would double the semantics, dependencies, test matrix, and debugging
surface. A short-lived comparison harness or golden-output generator is useful
during migration, after which librsvg/Cairo should leave the SVG dependency
path.

## Test gates

The cutover should not merge until these gates pass.

### Natural dimensions

- explicit width and height in px;
- `in`, `cm`, `mm`, `pt`, and `pc` at 96 DPI;
- viewBox with no explicit dimensions;
- width-only and height-only viewBox documents;
- percentage dimensions with and without a viewBox;
- dimensionless paths at the origin and positive offsets;
- negative-origin content;
- strokes, markers, filters, and clipping extending geometry;
- empty documents and zero/NaN/overflowing dimensions.

Expected logical sizes should be compared with the existing Neomacs tests and
the relevant GNU Emacs behavior, not selected solely from resvg output.

### Rasterization and alpha

- solid opaque color;
- subpixel antialiased edges over transparent pixels;
- partial element opacity and masks;
- gradients and filters;
- HiDPI and fractional device scales;
- maximum-size constraint without clipping absolute coordinates;
- pixel checks proving straight RGBA rather than premultiplied RGBA.

### Text

- explicit family and generic family fallback;
- missing font fallback;
- Latin, CJK, combining marks, Arabic/RTL, and emoji/color fonts;
- text-only dimensionless SVG;
- `tspan`, `dx`/`dy`, text-on-path, and vertical text cases used by Neomacs;
- deterministic golden tests with repository-controlled fonts;
- host-font smoke tests without pixel-exact assertions.

### Resources and CSS

- embedded PNG, JPEG, GIF, WebP, SVG, and optional SVGZ;
- external local href rejected by the default Neomacs policy;
- nested SVG unable to escape the same resource policy;
- large data URL and decompression-limit rejection;
- inline CSS cascade, presentation attributes, `currentColor`, and injected
  stylesheet behavior;
- a Figma-style valid fallback plus unsupported Display-P3 declaration.

### Robustness and performance

- malformed XML, invalid UTF-8, recursive references, and excessive elements;
- checked failure for huge requested pixmaps;
- bounded cache behavior for many unique SVGs;
- representative icons and SVG text measured for parse and render latency;
- blur/filter stress cases run under CI memory and time ceilings;
- concurrent parsing/rendering tests if decode work uses worker threads.

### Build and packaging

- Ubuntu 22.04 x86_64 and ARM builds with no librsvg/Cairo requirement from the
  SVG module;
- Windows and macOS builds using the same Rust backend;
- an audit showing whether Cairo/GLib/Pango packages remain required by other
  subsystems before deleting them from CI or packaging;
- artifact smoke tests and the existing GLIBC 2.35 ceiling.

## Recommendation

Adopt `resvg` 0.47 with a hard-cutover plan. Preserve the existing
`query_dimensions`/`decode` interface, keep natural-size behavior as an
explicit Neomacs policy, use a shared font database, disable uncontrolled file
hrefs, enforce resource caps, and return demultiplied sRGB RGBA.

Do not downgrade librsvg to fit Cairo 1.16, and do not bundle a private
Cairo/librsvg stack as the permanent architecture. Both choices retain native
dependency and security-update costs without improving cross-platform
consistency. Do not keep two production SVG backends after the compatibility
fixtures pass.

The remaining risk is behavioral compatibility, especially natural sizing,
fonts, and unusual CSS. It is finite and testable. The native dependency
conflict is structural; choosing `resvg` removes it.
