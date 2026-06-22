# Display Pipeline Deletion-Targeted Re-Plan

Date: 2026-06-21
Branch: `main`
Supersedes the "extract more" framing in
`docs/plans/2026-06-13-display-pipeline-refactor-handoff.md`.

## Why this plan exists

The handoff plan describes the remaining work as "containment" but measurement
shows the recent cadence did the opposite. Over the last ~90 commits the
non-test display pipeline **grew +2245 lines and went from 70 to 81 production
modules**, almost entirely through `extract X / move X / type X / hide X`
commits — net-zero relocations that add file boundaries and
request/context/policy wrapper layers without deleting any path. There are now
96 `display_*` files; 9 production modules are under 80 lines (one is 12).

This plan is the inverse: it ranks concrete **deletions and merges** by
value/risk. Every slice names what it *removes*. It was produced by an 11-cluster
audit (one auditor per module group) with each candidate adversarially verified
by an independent skeptic; refuted candidates were dropped. 41 candidates
survived verification.

**Honest net removal: ~1,200 production lines and ~7 files** across the landable
slices, the largest single win being pure low-risk dead code.

## The systemic disease (three patterns)

1. **Single-field / identical-field newtype towers** — the dominant pattern.
   `Request -> Context -> Item` chains where every layer copies the same fields
   forward and forwards a single method. Instances in *every* cluster:
   `DisplayRowSourceGeometry == DisplayRowGeometry`, the 3-level
   fragment/item/source render-request stack,
   `DisplayItemSourceAppendContext == SingleDisplayItemAppendContext`, the
   4-struct cursor-install chain, the `EndOfBufferTail/HscrollSkip/InvisibleText`
   `{context}` wrappers, `BufferWindowChromeHeights` bundle/unbundle,
   `DisplayRowRenderResult`, `TextOutputSpanContext`, `WindowChromeDisplayText`.
   This is the single biggest source of the +2245 growth.

2. **Duplicated finalization / geometry / face-resolution rules** — GNU keeps
   these single in `xdisp.c`; this code forked them. Post-row-transition
   position/hit-range sync implemented 3x, absolute-Y→relative-Y row-metric rule
   2x, twin `RenderState` structs, cursor vertical-clip predicate 2x. Each
   duplicate is also a drift-bug risk.

3. **Over-extracted tiny modules + test scaffolding in production files** — 9
   sub-80-line modules, a 387-line dead `display_iterator.rs`, single-consumer
   `display_media.rs`/`display_space.rs`, and several production modules carrying
   `#[cfg(test)]` walker/append harnesses that exist only to drive one test.

4. **(Process, not a slice) "append-concept diffusion"** — the dissolved
   `display_row_append.rs` left a 10,808-line catch-all test grafted via `#[path]`
   onto an unrelated 684-line host. This is a *symptom to stop reproducing*, not
   work to chase: re-partitioning it removes no code and grows file count. The fix
   is to stop the extraction process.

## Ranked slices

Land in rank order. Each slice is independently shippable and verified with
`env TMPDIR=<repo>/tmp cargo nextest run -p neomacs-layout-engine`.

### Rank 1 — Delete dormant `display_iterator.rs` + self-test  (−499 lines, −2 files, low risk)
Pure dead scaffolding from an abandoned 2026-04-11 unification plan (its own doc:
"Dormant at introduction... nothing uses it yet"). **Independently verified**: the
only references in the whole repo are `lib.rs:43` (`pub mod display_iterator;`) and
the file's own `#[path]` self-test; no use of `It`/`ItMethod`/`new_for_*` anywhere.
Precedent: sibling artifact `bidi_layout.rs` from the same plan already deleted as
dead (commit `0f9b5b51e`). Delete `display_iterator.rs`, `display_iterator_test.rs`,
and the `lib.rs:43` decl. Highest value-per-risk in the whole audit.

### Rank 2 — Delete dead `CursorAnchor`/`HitTestAnchor` item kinds + empty `stipple_patterns` shadow  (−58 lines, low risk)
Two independent confirmed-dead surfaces. The anchor variants are
`#[allow(dead_code)]`, constructed only in `display_item_test.rs`, and appear in
production solely in a no-op match arm shared with the live `RowBreak` (simplify to
`RowBreak(_) => {}`). The `OutputFrameBuildState.stipple_patterns` field is only ever
constructed empty, cleared, and copied into a `FrameDisplayState` that already
defaults it empty — never inserted into. Keep a `RowBreak`-only test for typed
coverage.

### Rank 3 — Merge two twin-struct pairs + dedup three identical post-transition sync helpers  (−84 lines, low risk)
(a) `BufferSourceBodyRenderState` is byte-identical to `BufferSourceWalkRenderState`
— one survivor serves both `render_body_and_tail` and `render_visible_steps`.
(b) `TextOutputSpanContext` is a 4-field clone of `TextRowOutput` — move its two
methods onto `TextRowOutput`. (c) Two of three byte-identical
`sync_*_after_row_transition` helpers collapse into one shared free fn; three
duplicate unit tests collapse into one. Preserve each call-site's `is_exhausted()`
guard; do **not** pull in the genuinely-divergent `overflow.rs:437/:566` variants.

### Rank 4 — Collapse the C1 row-center newtype tower  (−128 lines, medium risk)
(a) `DisplayRowSourceGeometry` (display_row.rs:492-572) is a field-for-field clone
of `DisplayRowGeometry`; hold the real type directly, the `source_request_*` methods
relocate onto the policy. (b) Fold the two intermediate request newtypes
`DisplayRowSourceFragmentRenderRequest` / `DisplayRowItemSourceRenderRequest` onto
`DisplayRowSourceRenderRequest` (the base already re-wraps itself, proving the layer
is a no-op). (c) Fold `DisplayRowRenderResult` onto `RenderedDisplayRow`. (d) Inline
the `finalize_glyph_row` one-line forwarder. Most bodies *relocate* not delete, so
net < gross spans. Two live render paths converge on the request wrappers — both must
keep working. Land geometry-clone (low) first, then the request fold, then the
render-result wrapper.

### Rank 5 — Collapse C6/C8 buffer-source Request→Context wrappers + transports  (−120 lines, low risk)
Three pure `struct X { context: XContext }` request newtypes
(`EndOfBufferTail`/`HscrollSkip`/`InvisibleText`) fold onto their Context.
`BufferWindowSourceReadRequest` 2-field wrapper → call
`from_window_params(...).read_into(...)` directly. `BufferWindowChromeHeights`
bundle/unbundle → pass the values already in scope. `BufferSourceOutputSetup::new`
15-arg pure forwarder → fold into `from_parts` (keep the name `new`). Each has one
live caller; tests already bypass several wrappers, proving they aren't the real seam.

### Rank 6 — Collapse C7 output/install forwarding hops + cursor-install middle layer  (−175 lines, −1 file, medium risk — land last)
Remove the ~13 window-output-only forwarder fns in `display_text_output_install.rs`
(double hop into `window_output`), the redundant middle
`DisplayOutputCursorArtifactInstallRequest`, the private `DisplayRowCurrentRowInstaller`
(fold into `DisplayRowCurrentRowOutput`), and merge `display_output_row_grid.rs` into
its sole consumer `display_output_window_state.rs`. **Load-bearing — rehome, do NOT
delete:** `install_output_resolved_face` (4 consumers), `install_display_row` /
`DisplayRowOutputInstall` (consumed by the *production* `display_mock_frame.rs`),
`TextWindowRowDecorationRequest` (imported by `display_row_source_render.rs:40`).
Highest-coupling slice — land last, in sub-steps (current-row installer → grid merge →
forwarder fusion), nextest after each.

### Rank 7 — Collapse C3 append-context duplicate-field layers  (−89 lines, medium risk)
`DisplayItemSourceAppendContext` is byte-identical to
`SingleDisplayItemAppendContext`; fold its 2 methods over, taking `face_id` as an
explicit arg. Collapse `LispStringSourceAppendContext` /
`LispStringSourceRowAppendContext` transient forwarders, fold
`BufferLinePrefixRenderContext` into its Request, drop the
`DisplayRowAppendSourceMeasureRequest` single-field wrapper. **Correctness items from
verification:** (1) method-name collision — the moved `render_with_policy<S,P>` must
land as `render_source_with_policy`; (2) `face_id` is intentionally re-derived per
item, so moved methods must accept it as an arg, not read `self.face_id`, or per-item
face/cursor resolution regresses. Face/cursor tests are the canary. Should land after
Rank 4 (it references the now-canonical `DisplayRowSourceRenderRequest`).

### Rank 8 — Rehome test-only scaffolding out of three production modules  (−90 lines, low risk)
Move the `DisplayRowSourceStep`/`DisplayRowSourceWalker` `#[cfg(test)]` harness out of
`display_row_source_state.rs` (and delete the test-only re-export pin at
`display_row.rs:36`); move 2 of the 4 `#[cfg(test)]` append/measure items out of
`display_buffer_source_item_append.rs`; consolidate the byte-identical
`text_row_source_measure_state` helper. **Scope discipline:** do NOT move
`append_source_text_request_to_text_row` (reads private fields → would force new
`pub(crate)` accessors). Do NOT collapse the intentionally-divergent mock fixtures
(`image_id 42 vs 77`, `window_id 1 vs 41`).

### Rank 9 — Merge two tiny spec modules + inline two pass-through helpers  (−56 lines, −2 files, low risk)
`display_space.rs` (75) folds into sibling `display_spec.rs`; `display_media.rs` (99,
single consumer) inlines into `display_source_resolver.rs`; the
`render_face_ref_with_fallback` one-line wrapper inlines into its single caller
(`render_face_ref_id` stays — 4 other callers); the `WindowChromeDisplayText` 1-field
newtype with an ignored `_selected_window` param inlines. **Keep** `display_face_id.rs`
and `BaseFacePolicy` — distinct shared concerns, not over-extraction.

## Explicitly NOT recommended (anti-goals)

- **Rehoming the 10,808-line `display_row_append_test.rs`** — high risk, ~0 net lines,
  likely *+N* files. It is a move, not a deletion, and contradicts the containment
  goal. Optional housekeeping only, after the real deletions land.
- **Merging the 3 face micro-modules into one `display_face.rs`** — yields a grab-bag
  with a 16-file blast radius for only −2 files. Not worth the risk.
- The absolute-Y row-metric DRY (~5 lines) and the cursor-clip predicate DRY (net +1
  line) — maintainability only, not a reduction.

## Sequencing

Ranks 1–3 are independent pure deletions/single-file merges — ship in any order.
Rank 4 before Rank 7 (Rank 7 references the request type Rank 4 makes canonical,
avoiding double-touching call sites). Rank 6 last among the indirection slices
(highest coupling). Rank 8 independent — but doing it after 4/7 lets it also drop
tests covering now-collapsed types.

## Acceptance / surface budget

"Done" must mean **fewer files and fewer lines than today**, not more. Set a ceiling:
the pipeline should end below ~80 production modules and below today's 37,210 non-test
lines. Any future slice that can't name a deleted path is churn, not progress.
