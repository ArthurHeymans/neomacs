# Display Pipeline Refactor Handoff

Date: 2026-06-13
Last refreshed: 2026-06-22
Branch: `main`

> **STATUS (2026-06-22): SPINE COMPLETE — this handoff is historical.**
> The refactor spine is finished and verified (`cargo nextest -p neomacs-layout-engine`
> = 1250/1250). The "Remaining Phases" below are closed (see the per-phase status
> in that section). The authoritative current docs are:
> - `docs/plans/2026-06-21-display-pipeline-deletion-plan.md` — the deletion-targeted
>   re-framing that supersedes the older "extract more modules" framing here.
> - `docs/plans/2026-06-22-pipeline-completion-proof.md` — the per-path proof that
>   measurement/advance and face/source policy are owned at the shared boundary.
> Do **not** act on the old "extract types out of display_row.rs" framing — keeping the
> request/renderer/context types in the row center is intentional; the wrapper tower
> was collapsed onto one canonical `DisplayRowSourceRenderRequest` instead.

## Final Goal

Finish the full display pipeline refactor: every display source should become a
typed display item stream, then flow through one shared row-rendering lifecycle.

Target shape:

```text
buffer / Lisp string / overlay string / display property / mode-line /
tab-line / tab-bar / minibuffer / echo area / child-frame or posframe
        -> DisplayItemSource / DisplayItem
        -> DisplayRowSourceRenderRequest or append request
        -> DisplayRowRenderer / DisplayRowBuilder
        -> RenderedDisplayRow
        -> row finalizer
        -> window/frame installer
        -> renderer
```

Source-specific code may understand buffers, overlays, Lisp values, text
properties, window state, and GNU display rules. Once it emits typed items and row
requests, the shared row path should own face realization, text measurement,
display-property rendering, clipping/wrapping, bidi finalization, row metadata,
and installation.

The goal is not complete yet. The main buffer source is unified, echo/minibuffer
now use the buffer walk, `DisplayRowSpec` is gone, and much of the old matrix and
append sprawl has been deleted or split. The remaining work is mostly
containment: shrink the row center, delete duplicated measurement/policy paths,
and reduce the matrix/builder boundary to row installation and bookkeeping.

## Current Verified State

Local branch status at refresh:

- Branch is far ahead of `origin/main`; do not assume remote has this state.
- Do not push unless explicitly asked.
- `docs/plans/2026-06-13-display-pipeline-refactor-handoff.md` is a handoff file
  and may be intentionally left uncommitted.
- Use repo-local temp only: `TMPDIR=/home/exec/Projects/github.com/eval-exec/neomacs-main/tmp`.

Recent local display-pipeline commits at this refresh:

```text
a87d25355 refactor: extract display row render item policy
b7c42316f refactor: move display row geometry
41b2e721a refactor: extract display row face state
7efb7530b refactor: extract measured display rows
b9c5b336e refactor: extract display row source state
44f8181c4 refactor: extract display row render state
1b11decdb refactor: extract display row metrics
4cf3e7d2d refactor: split output frame artifact state
cff4d331a refactor: route output row cursor lifecycle
96d007074 refactor: type output current row mutations
95c1b65a0 refactor: type output window row mutations
6f197eda3 refactor: type special glyph row mutations
3de403248 refactor: type current row output mutations
```

Latest local code commit at this refresh is `a87d25355`. It extracts row render
policy and render-item lowering from `display_row.rs` into:

- `neomacs-layout-engine/src/display_row_render_policy.rs`
- `neomacs-layout-engine/src/display_row_render_item.rs`

Verification run for that commit:

```bash
env TMPDIR=/home/exec/Projects/github.com/eval-exec/neomacs-main/tmp cargo check -p neomacs-layout-engine --tests
env TMPDIR=/home/exec/Projects/github.com/eval-exec/neomacs-main/tmp cargo nextest run -p neomacs-layout-engine --status-level fail --no-fail-fast
```

The layout-engine nextest suite was run twice around the extraction. Both runs
passed: 1250 tests, 0 failures.

Known unrelated warnings during these commands come from `neovm-core`:

- unused assignment to `p`;
- unused assignment to `cycle_completed`;
- `unsafe_op_in_unsafe_fn` in `seed_all_context_roots`.

Do not mix those warning cleanups into display-pipeline commits unless a slice
actually touches that code.

## What Is Done

The main completed phases:

- `DisplayRowSpec` containment is finished. The old caller-facing spec name is no
  longer present in `neomacs-layout-engine/src`.
- Main-buffer source unification is finished. `BufferTextSourceCursor` is the
  production source path; the legacy raw decoder, typed-source fallback flag, and
  parity shim were deleted in earlier local commits.
- Echo and inactive minibuffer rendering now flow through the normal buffer-text
  walk by using the echo area buffer, instead of a parallel synthetic row path.
- The old giant window loop was moved out of `engine.rs`; `engine.rs` is now
  mostly frame/window orchestration.
- Buffer source rendering has been split across focused modules:
  `display_buffer_source_walk.rs`, `display_buffer_source_loop_render.rs`,
  `display_buffer_source_body_render.rs`, `display_buffer_source_item_render.rs`,
  `display_buffer_source_row_lifecycle.rs`, `display_buffer_source_render_plan.rs`,
  and related helpers.
- Row finalization is centralized enough that prebuilt/incremental bidi split was
  removed; row-end finalization is no longer duplicated in the old form.
- Row render policy and render-item lowering now live outside `display_row.rs` in
  `display_row_render_policy.rs` and `display_row_render_item.rs`.
- Right-edge/right-border special glyphs, line-number margin rendering, overlay
  string rendering, row metrics, row geometry, measured rows, row face state, row
  source state, row render state, output row cursor lifecycle, and current/window
  output mutations have been extracted from older large modules.
- `display_row_append.rs` no longer exists. Its remaining responsibilities are in
  `display_row_append_context.rs` and the newer buffer/row modules.
- `matrix_builder.rs` no longer exists under that name. Matrix-facing behavior has
  been split into output/current-row/window-row/special-glyph paths and row
  installers.
- Overlay string ordering has been corrected where it was safe. Do not collapse
  before-string and after-string injection into a single stop point: prior review
  found that would merge distinct GNU positions and break cursor behavior.
- Shaped-run caching was added so natural text rendering does not shape the same
  run twice in the normal measure/render path.

## Current Pipeline

Main buffer and echo/minibuffer text now broadly follow this shape:

```text
engine.rs
  -> display_buffer_window_render.rs / display_buffer_window_source.rs
  -> display_buffer_source_walk.rs
  -> BufferTextSourceCursor
  -> DisplayItem
  -> display_buffer_source_* render/append modules
  -> DisplayRowSourceRenderRequest / DisplayRowSourceAppendRequest
  -> DisplayRowRenderer / DisplayRowBuilder
  -> RenderedDisplayRow
  -> row finalizer / output installer
```

Chrome/string rows broadly follow:

```text
LispStringSourceCursor or source-specific request
  -> DisplayItemSource / DisplayItem
  -> DisplayRowSourceRenderRequest
  -> DisplayRowRenderer / DisplayRowBuilder
  -> RenderedDisplayRow
  -> installer
```

Important current modules:

- `display_item.rs`: typed display item language.
- `display_source.rs`: source cursor abstraction and shared display source types.
- `display_buffer_text_source.rs`: `BufferTextSourceCursor`.
- `display_buffer_source_*`: production main-buffer source walking, item
  rendering, replacement/display-property rendering, overflow, row lifecycle, and
  render-plan logic.
- `display_row.rs`: still the row center. It now holds row source request/session
  types, row render context, and `DisplayRowRenderer`.
- `display_row_render_policy.rs`: row render clipping/exhaustion policy.
- `display_row_render_item.rs`: source-item to row-item lowering and media
  remainder handling.
- `display_row_builder.rs`: row geometry/progress/measurement and row item
  building.
- `display_row_finalizer.rs`: row finalization boundary.
- `display_row_source_render.rs` and `display_row_source_append.rs`: source to
  row render/append execution.
- `display_row_append_context.rs`: append surfaces/frames/kinds. This is still a
  coupling point, but it is no longer a 10k-line append module.
- `display_rendered_row_output_install.rs`, `display_current_row_output.rs`,
  `display_output_window_row*.rs`: output installation and matrix-facing behavior.

## Remaining Phases

> **ALL CLOSED as of 2026-06-22.** Per-phase resolution:
> - **Phase 1 (extract types out of `display_row.rs`) — REJECTED / superseded.** Doing
>   this would repeat the file-count/wrapper-growth problem. Instead the wrapper tower
>   was collapsed onto one canonical `DisplayRowSourceRenderRequest`. `display_row.rs`
>   is the row center and intentionally owns the renderer/context/request types.
> - **Phase 2 (normalize measurement/width) — DONE + PROVEN.** One advance authority
>   (`DisplayRowWriter` + measured-face metrics + the single `calc_pixel_width_or_height`
>   `(space)` evaluator). See `2026-06-22-pipeline-completion-proof.md`.
> - **Phase 3 (normalize face/source policy) — DONE + PROVEN.** One per-origin
>   `BaseFacePolicy` + `FaceResolver` + a single `resolve_and_install_measured_face`
>   realizer (2 call sites). See the completion proof.
> - **Phase 4 (overlay/display-property typed convergence) — CAPPED (correct-as-is).**
>   The candidate folds are redesigns, not behavior-preserving folds; the current design
>   is GNU-faithful. See the deletion plan's "intentionally NOT recommended" section.
> - **Phase 5 (shrink matrix/output) — DONE.** Forwarding hops + row-grid collapsed;
>   the post-install edge-marker placement is GNU-correct, not a defect.
> - **Phase 6 (broaden regression coverage) — DONE/ongoing.** Added end-to-end face-merge,
>   buffer display-prop replacement, RTL-chrome, and chrome `:align-to` tests.
>
> The detail below is retained as historical context for each phase.

### 1. Extract row request/session/plan types out of `display_row.rs`

`display_row.rs` is still the next large containment target. The likely next
types to move are:

- `DisplayRowItemSourceRenderRequest`
- `DisplayRowSourceFragmentRenderRequest`
- `DisplayRowSourceFragmentFrame`
- `DisplayRowLispStringSourceSessionRequest`
- `DisplayRowLispStringSourceRenderRequest`
- `DisplayRowLispStringSourceSession`
- `DisplayRowSourceRequestPolicy`
- `DisplayRowSourceRenderRequest`
- `DisplayRowRenderContext`

Keep public call sites stable. Move types by responsibility, not by alphabetical
chunks. Good candidate modules:

- `display_row_source_request.rs`
- `display_row_lisp_string_request.rs`
- `display_row_render_context.rs`

Success condition: `display_row.rs` becomes mostly orchestration and
`DisplayRowRenderer`, not a registry for every request/session type.

### 2. Normalize measurement and width ownership

Search starting points:

```bash
rg -n "font_char_width|char_advance|pixel_width|natural.*width|advance_px|width_px" neomacs-layout-engine/src
```

Goal: row rendering/measurement owns text advance. Source walking may need typed
progress for wrap/cursor decisions, but it should not duplicate independent pixel
width rules.

Tests to keep green or add:

- ASCII buffer text;
- CJK wide chars;
- emoji / ZWJ grapheme clusters;
- combining marks;
- tabs;
- mode-line/tab-line/tab-bar strings;
- minibuffer/echo;
- child-frame/posframe rows.

### 3. Normalize face/source policy

Policy should live in source/face policy modules, not in row glyph emission.

Useful files:

- `display_origin.rs`
- `display_face_policy.rs`
- `display_source_resolver.rs`
- `display_buffer_source_face_resolution.rs`
- `display_row_face_state.rs`

Regression areas:

- minibuffer prompt face must not leak into completion candidates;
- overlay before/after strings need GNU-compatible anchor policy;
- display-property strings and replacement text must keep their own faces;
- tab-line/tab-bar propertized strings should preserve their faces;
- child-frame/posframe rows should resolve independently from parent rows.

### 4. Finish overlay/display-property typed item convergence

Do not force overlay before/after strings into one injection point. The before and
after positions are distinct. The safe remaining work is containment and typed
item convergence:

- remove any leftover overlay scaffolding that only wraps the same shared row
  path;
- keep cursor capture and row-break behavior intact;
- ensure display-property replacements, stretches, media, and mapped text stay
  typed until row rendering;
- add focused tests before deleting a special case.

### 5. Shrink matrix/output responsibility further

The old `MatrixBuilder` target is already mostly gone, but the final goal is the
same: matrix-facing code should install completed rows and manage frame/window
state, not encode display semantics.

Look for remaining semantic work in output/install paths:

```bash
rg -n "face|width|advance|bidi|overlay|display property|replacement|cursor" neomacs-layout-engine/src/display_*output* neomacs-layout-engine/src/display_rendered_row_output_install.rs
```

Move source semantics back above the row-render boundary when found.

### 6. Broaden regression coverage

Before calling the full refactor done, coverage should include:

- main buffer plain text, tabs, truncation, wrapping, hscroll, invisible text;
- overlays and display properties;
- replacement strings, stretches, images/media placeholders;
- bidi in buffer and chrome rows;
- wide chars and grapheme clusters;
- mode-line/header-line/tab-line/tab-bar;
- echo area and active/inactive minibuffer;
- child-frame/posframe rendering;
- row finalization and cursor placement around clipped rows.

## Important Invariants

- Use `cargo nextest`, not `cargo test`, for suite verification.
- Use repo-local temp: `TMPDIR=/home/exec/Projects/github.com/eval-exec/neomacs-main/tmp`.
- Keep display semantics above the row-render boundary; row installers should not
  rediscover source meaning from glyphs or pixels.
- Preserve explicit row ownership:
  - tab-bar is frame chrome;
  - tab-line/header-line/mode-line are window chrome;
  - active minibuffer is a buffer walk;
  - inactive echo renders through the echo area buffer;
  - child-frame/posframe content belongs to that frame/window.
- Use enums for semantic choices instead of bool flags.
- Keep Lisp-specific parsing and `Value` handling out of pure row building where
  possible.
- Row finalization, bidi normalization, clipping, and face insertion should happen
  at predictable shared boundaries.

## GNU Reference

Use the local GNU source at:

```text
/home/exec/Projects/github.com/emacs-mirror/emacs
```

Most useful file:

```text
src/xdisp.c
```

Useful conceptual anchors:

- source iteration is source-specific;
- glyph production is shared;
- display strings should not use a weaker display language than buffer text;
- minibuffer/echo display uses echo area buffers and the normal display walk;
- overlay before/after positions are distinct display stops.

## Recommended Verification

For a focused layout-engine slice:

```bash
cargo fmt -p neomacs-layout-engine
env TMPDIR=/home/exec/Projects/github.com/eval-exec/neomacs-main/tmp cargo check -p neomacs-layout-engine --tests
env TMPDIR=/home/exec/Projects/github.com/eval-exec/neomacs-main/tmp cargo nextest run -p neomacs-layout-engine <focused-filter>
git diff --check
```

Before handing off a large display-pipeline batch:

```bash
env TMPDIR=/home/exec/Projects/github.com/eval-exec/neomacs-main/tmp cargo nextest run -p neomacs-layout-engine --status-level fail --no-fail-fast
```

If touching display protocol/runtime:

```bash
env TMPDIR=/home/exec/Projects/github.com/eval-exec/neomacs-main/tmp cargo nextest run -p neomacs-display-protocol
env TMPDIR=/home/exec/Projects/github.com/eval-exec/neomacs-main/tmp cargo nextest run -p neomacs-display-runtime
```

GUI smoke areas that have caught regressions before:

- `M-x`, type one char, press `TAB`: no duplicate visible "Making completion list..." rows.
- Vertico/posframe: stable width, no text overlap, prompt face does not leak into candidates.
- Tab-bar/tab-line: stable height and spacing across repeated navigation.
- Menu-bar: correct popup content, anchoring, hover behavior, and submenu behavior.
