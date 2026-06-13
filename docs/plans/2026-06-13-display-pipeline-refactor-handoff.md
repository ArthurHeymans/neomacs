# Display Pipeline Refactor Handoff

Date: 2026-06-13
Branch: `main`
Latest relevant pushed commit before this batch: `e86a1db80 refactor: measure synthetic text in source append`

## Goal

Continue the long-term display pipeline refactor until Neomacs has one typed source-to-row pipeline for main buffer text, overlay/display strings, mode/header/tab-line, tab-bar, minibuffer, echo area, and detached child-frame/posframe rows.

The goal is not complete. The recent work landed several safe slices, but the pipeline is still split between older buffer-walker code and newer request/source-based row rendering.

## Current Status

Recent commits on `main`:

- `e7f537b67 refactor: render source fragments from requests`
- `5b22c4f94 refactor: group append request render parts`
- `1591c69aa refactor: render display rows from source requests`
- `e501cc48d refactor: use source render requests in row tests`
- `92f0016f7 refactor: drop legacy display row spec test constructor`
- `ab77dba68 refactor: create source render requests from base faces`
- `12a4fe846 refactor: render minibuffer echo from source request`
- `691f063ce refactor: route overlay strings through append request`
- `576e02b6a refactor: render display sources from append request`
- `25313d877 refactor: route lisp string append through source request`

Verification for `e7f537b67`:

```bash
cargo nextest run -p neomacs-layout-engine
cargo fmt --check
git diff --check
cargo check -p neomacs-layout-engine
```

`cargo nextest run -p neomacs-layout-engine` passed with 1053 tests. Do not use `cargo test`; the project owner explicitly requested `cargo nextest`.

Continuation work after this handoff:

- `DisplayRowSpec` has been contained and renamed to the private `LoweredDisplayRowSpec` in `display_row.rs`; external tests/callers now use `DisplayRowSourceRenderRequest` or request-native helpers.
- Main buffer ordinary text and several special buffer item paths now append through shared source append requests; complex shaping keeps a premeasured policy only where the buffer walker still needs it for wrap/cursor decisions.
- Replacement-string display sources now use the renderer-owned font metrics slot through render policy measurement callbacks.
- The dormant `neomacs-layout-engine::display_backend` / `display_row_sink` scaffolding has been deleted. It represented an older backend-oriented width path and was no longer wired into production layout.
- Synthetic ellipses and truncation markers now let the source-row append renderer own text measurement instead of passing caller-precomputed `DisplayTextRunMeasurement`.
- Test-only direct display-item append stream helpers have been removed. Lisp string, synthetic text, and media replacement append coverage now exercises the same source-append request helpers used by runtime paths.
- `.config/nextest.toml` caps the default nextest worker pool at 24 threads while keeping the memory-limit wrapper on every test.
- Main buffer complex text append no longer constructs `DisplayTextRunMeasurement` in `engine.rs`; the append layer now lowers the already-resolved fragment advance into its render policy. The buffer walker still computes that advance for wrap and cursor decisions, so full width-path unification remains open.
- Buffer text fragment append now uses one typed append helper with `BufferTextFragmentAppendMeasurement::{Natural, ResolvedAdvance}` instead of separate natural/resolved append entry points. This keeps measurement policy at the source-append boundary while the main buffer walker still decides when a resolved advance is required.
- Append placement is private to `display_row_append.rs`, and append tests now build frames through `DisplayRowAppendSurface` instead of constructing lowered placement/frame internals.
- The generic `DisplayTextRunMeasurementPlan::from_char_advances` helper has been removed. Fallback per-character text-run measurement now lives in `DisplayRowGlyphMeasurementFace`, so caller-provided char-advance construction is no longer exposed as a shared measurement-plan API.
- Buffer text fragment append advance selection now uses typed `BufferTextFragmentAdvancePath` variants for natural face-column, natural rendered-fragment, and resolved complex-run paths instead of inline conditional policy.
- Latest local verification:

  ```bash
  cargo fmt --check
  git diff --check
  cargo check -p neomacs-layout-engine
  cargo nextest run -p neomacs-layout-engine display_row display_row_append
  cargo nextest run -p neomacs-layout-engine
  ```

  Full layout-engine nextest passed with 1031 tests after deleting the dormant backend tests, adding append-measurement coverage, and removing the test-only direct append stream.

## Why This Refactor Exists

The bugs that triggered this work were not isolated rendering accidents. They all came from the same architectural split:

- main buffer rendering had a rich ad hoc walker in `engine.rs`;
- chrome/string rows had separate simpler paths;
- posframe/minibuffer/tab-line/tab-bar rows reached glyph output with weaker face, width, overlay, display-property, and row lifecycle semantics;
- several paths carried already-derived glyph data instead of typed display intent.

That split caused bugs such as:

- tab-bar/tab-line/posframe text spacing and overlap problems;
- background face leakage from minibuffer prompt into completion candidates;
- posframe width changing while moving selection;
- overlay before/after string behavior diverging from GNU Emacs;
- row finalization and bidi normalization happening at inconsistent times.

The long-term design is to preserve typed display origin and display items until a single row builder handles shaping, width, faces, display properties, clipping, bidi, and row finalization.

## Desired Pipeline

Target shape:

```text
Source-specific side
--------------------
BufferTextSourceCursor
LispStringSourceCursor
Overlay/display string sources
Chrome row sources
Detached/posframe row sources
        |
        v
DisplayItemSource / DisplayItem stream
        |
        v
DisplayRowSourceRenderRequest
        |
        v
DisplayRowRenderer / DisplayRowBuilder
        |
        v
RenderedDisplayRow + metadata
        |
        v
Window or frame row installer
        |
        v
FrameDisplayState -> renderer
```

Important boundary:

- Code above `DisplayItemSource` may know about buffers, overlays, Lisp `Value`, text properties, window state, and GNU-specific source rules.
- Code below `DisplayItemSource` should consume typed Rust display items and row requests. It should not evaluate Lisp or infer source semantics from glyphs.

## Key Modules

- `neomacs-layout-engine/src/display_item.rs`
  - Typed display item language: `DisplayItem`, `DisplayItemKind`, source spans, text runs, media/stretch items, face refs.

- `neomacs-layout-engine/src/display_source.rs`
  - Source cursor abstraction: `DisplayItemSource`, `LispStringSourceCursor`, buffer/string source-facing cursor logic.

- `neomacs-layout-engine/src/display_row.rs`
  - Current row rendering center: `DisplayRowSourceRenderRequest`, private `LoweredDisplayRowSpec`, `DisplayRowRenderContext`, `DisplayRowRenderer`, fragment rendering, row finalization helpers.

- `neomacs-layout-engine/src/display_row_append.rs`
  - Bridge for appending rendered fragments into current text rows and emitting output. Still has significant builder coupling.

- `neomacs-layout-engine/src/display_origin.rs`
  - Typed origins for face/source policy decisions.

- `neomacs-layout-engine/src/display_face_policy.rs`
  - Base face policy. This should grow into the only place that decides how each origin gets its base face.

- `neomacs-layout-engine/src/display_source_resolver.rs`
  - Resolves display-source items and faces. Keep semantic source resolution here rather than in row glyph output.

- `neomacs-layout-engine/src/display_row_builder.rs`
  - Row geometry/progress/measurement concepts. Long-term, this should own more of the shared row lifecycle.

- `neomacs-layout-engine/src/matrix_builder.rs`
  - Matrix installation/bookkeeping. Long-term, this should not encode display semantics.

- `neomacs-layout-engine/src/engine.rs`
  - Still contains the large buffer redisplay walker. This is the main debt area.

## Current Split To Remove

There are still two broad rendering paths:

```text
Main buffer path
----------------
engine.rs buffer loop
  -> direct GlyphMatrixBuilder / display_row_append helpers
  -> GlyphRow

String/chrome path
------------------
DisplayRowSourceRenderRequest
  -> LoweredDisplayRowSpec
  -> DisplayItemSource
  -> DisplayRowRenderer
  -> GlyphRow
```

The old caller-facing `DisplayRowSpec` name is gone from source. The lowered row request is now the private `LoweredDisplayRowSpec` inside `display_row.rs`; new caller coverage should use `DisplayRowSourceRenderRequest`, `DisplayRowSourceAppendRequest`, or a higher-level row request.

## Important Invariants

Preserve these while refactoring:

- Use `cargo nextest`, never `cargo test`.
- One architectural slice per commit.
- Do not hide layout bugs with frontend workarounds.
- Do not let renderer code infer row source semantics from pixels or glyphs.
- Keep row ownership explicit:
  - tab-bar is frame chrome;
  - tab-line/header-line/mode-line are window-local chrome;
  - minibuffer/echo are normal window rows unless explicitly detached;
  - posframe/child-frame content belongs to that frame/window, not the parent minibuffer prompt.
- Use enums for semantic choices instead of bool flags.
- Keep `Value` and Lisp-specific parsing out of pure row building wherever possible.
- Row finalization, bidi normalization, clipping, and face insertion must happen at one predictable boundary.

## Suggested Next Slices

### Slice 1: Contain `DisplayRowSpec` (done)

Goal: make `DisplayRowSpec` an internal lowering type for `DisplayRowRenderer`.

Status: complete in source. The lowered type is now private `LoweredDisplayRowSpec`; keep it that way.

Steps:

1. Search:

   ```bash
   rg -n "DisplayRowSpec|display_row_spec\\(|display_row_spec_" neomacs-layout-engine/src
   ```

2. For every test/caller that can use `DisplayRowSourceRenderRequest`, migrate it.

3. Keep direct lowered-spec use only in renderer internals.

4. Focused verification:

   ```bash
   cargo nextest run -p neomacs-layout-engine display_row display_row_append
   cargo nextest run -p neomacs-layout-engine
   ```

### Slice 2: Retire Old Width Paths

Goal: make all row text use one measurement/advance path.

Look for:

```bash
rg -n "font_char_width|pixel_width|from_char_advances|text_char_advance_px_at_position_with_measurement" neomacs-layout-engine/src
```

Current issue: some paths still derive placement from face-width assumptions or precomputed per-character advances before row rendering. The row renderer should own measured text advance for all row kinds.

Do not remove a path until tests cover:

- main buffer ASCII;
- wide chars;
- grapheme clusters;
- tab-line/tab-bar;
- mode-line;
- minibuffer/echo;
- posframe/child-frame completion rows.

### Slice 3: Normalize Face Source Policy

Goal: prevent face leakage across prompt/candidate, overlay, display-string, and chrome boundaries.

Useful tests:

- minibuffer prompt background does not leak into completion candidates;
- overlay `before-string` and `after-string` use GNU-compatible anchor face policy;
- tab-line/tab-bar propertized strings preserve their own faces;
- detached/child-frame rows resolve faces independently of parent window rows.

Keep policy in:

- `display_origin.rs`
- `display_face_policy.rs`
- `display_source_resolver.rs`

Avoid putting source-specific face rules in `DisplayRowRenderer`.

### Slice 4: Move More Buffer Walker Output To `DisplayItemSource`

Goal: shrink the giant buffer loop in `engine.rs`.

Approach:

1. Extract a narrow source cursor for one simple segment type first, probably plain buffer text with resolved face refs.
2. Route that through `DisplayRowRenderer`.
3. Keep matrix installation unchanged.
4. Add regression tests around cursor position, row end, clipping, and bidi.

Do not try to migrate overlays, display properties, and cursor metadata all in one commit.

### Slice 5: Unified Row Lifecycle

Goal: one lifecycle for row construction:

```text
begin row
consume display items
measure/shape/append slots
clip or exhaust source
finalize bidi
normalize external row
install row
emit output
```

Today these steps are still spread across `DisplayRowRenderer`, `GlyphMatrixBuilder`, `display_row_append.rs`, and `engine.rs`.

## GNU Emacs Reference Points

Use `/home/exec/Projects/github.com/emacs-mirror/emacs` as the local GNU source reference.

Useful files/functions:

- `src/xdisp.c`
  - redisplay iterator model;
  - display string handling;
  - glyph production;
  - overlay string loading;
  - mode-line/tab-line style string display.

GNU's C code is not a Rust abstraction to copy directly. The useful lesson is the conceptual split:

- source iteration can be source-specific;
- glyph production should be shared;
- display strings should not use a weaker display language than buffer text.

## Manual Repro Context

The user often validates with personal config and `/tmp/debug.txt`, sometimes with:

```bash
NEOMACS_DUMP_FRAME_GLYPHS=1
```

Recent visual areas to keep checking:

- `M-x`, type one char, press `TAB`: minibuffer should not spam duplicate visible "Making completion list..." messages.
- Vertico posframe:
  - no text overlap;
  - candidate spacing stable;
  - candidate row backgrounds not polluted by prompt face;
  - posframe width does not shrink while moving selection.
- Tab-bar/tab-line:
  - height should not grow on repeated `j`/`k`;
  - text spacing should match main row measurements.
- Menu-bar:
  - button click opens correct menu content;
  - popup anchored below clicked button;
  - hover opens submenus without selecting commands.

## Recommended Verification

For a narrow slice:

```bash
cargo nextest run -p neomacs-layout-engine <focused-test-filter>
cargo fmt --check
git diff --check
cargo check -p neomacs-layout-engine
```

Before pushing a display pipeline slice:

```bash
cargo nextest run -p neomacs-layout-engine
```

If touching cross-crate display protocol/runtime code, also run the relevant package:

```bash
cargo nextest run -p neomacs-display-protocol
cargo nextest run -p neomacs-display-runtime
```

Expect some unrelated warnings from `neovm-core` during `cargo check`; do not mix warning cleanup into display refactor commits unless directly required.

## Good First Task For The Next Developer

Start with Slice 2: retire old width paths.

Slice 1 is complete in source: external tests and non-renderer callers construct `DisplayRowSourceRenderRequest` or a higher-level request, and lowered row construction is local to renderer lowering. The next concrete success criteria are:

- reduce duplicated width/advance decisions outside row rendering;
- keep main-buffer tests covering ASCII, wide characters, grapheme clusters, and display replacements green;
- full `neomacs-layout-engine` nextest remains green.
