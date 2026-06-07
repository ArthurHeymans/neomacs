# Buffer Text Backend Refactor Plan

Date: 2026-06-03
Status: in progress, updated 2026-06-06
Scope: `neovm-core` buffer text storage, edit semantics, text properties,
markers, backend selection, and backend parity tests

## Goal

Make Neomacs buffer text storage support multiple physical backends without
letting backend details leak into Lisp-visible buffer semantics.

The long-term architecture should preserve GNU Emacs observable behavior while
allowing Neomacs to use better physical storage for different workloads:

- `gap-buffer` for default GNU-like behavior and compatibility.
- `piece-tree` for large files and edit-heavy workloads.
- `rope` as another physical backend behind the same semantic layer.

The target design is:

```text
Buffer / Lisp-visible editor semantics
  -> BufferText semantic layer
    -> private TextBackend enum
      -> GapBuffer / PieceTree / Rope
```

Neomacs should copy GNU Emacs semantic behavior and semantic ordering. It should
not copy GNU Emacs' physical gap-buffer coupling as a general architecture.

## GNU Model

GNU Emacs uses a gap buffer as the central buffer text representation. Many
internal modules directly reason about the physical gap:

- `src/insdel.c` moves and resizes the gap during insertion, deletion, and
  replacement.
- `src/marker.c` uses `GPT`, `GPT_BYTE`, and marker caches to convert between
  character and byte positions.
- `src/search.c` scans before and after the gap explicitly.
- `src/editfns.c` exposes `(gap-position)` and `(gap-size)` and copies text by
  splitting around the gap.
- `src/xdisp.c` uses gap-adjacent unchanged ranges and assumes some redisplay
  paths do not cross the gap.

That coupling is part of GNU Emacs' implementation history. Neomacs needs GNU
Emacs behavior, but it should not require every subsystem to know whether the
physical text storage is a gap buffer, piece tree, or rope.

## Current Neomacs Shape

Neomacs already has the right broad direction:

- `Buffer` owns buffer identity and editor state.
- `BufferText` owns shared text state.
- `BufferTextStorage` owns metrics, concrete backend, text properties, marker
  chain, modification ticks, and position caches.
- `TextBackend` is private to the buffer module and dispatches to concrete
  backend implementations.
- Indirect buffers share text through `BufferText::shared_clone`.
- Backend kind is selectable through Neomacs-only APIs.

The broad shape is now in place:

- `piece-tree` and `rope` are implemented backend variants.
- `TextBackendDebugLayout` carries explicit backend-specific debug layout.
- Production text-position conversion uses backend indexes for indexed
  backends and GNU-style anchor scans for the gap backend.
- Text-property interval storage is character-indexed like GNU intervals;
  `BufferText` owns conversion from Emacs byte ranges at the boundary.
- Edit transactions, search replacement bookkeeping, syntax motion helpers,
  navigation property-boundary helpers, and several buffer builtins now carry
  typed char/byte positions through their semantic boundaries.

The main remaining weakness is that some manager, display, and string-only
helpers still accept raw `usize` positions near semantic boundaries. They
should be migrated toward `EmacsBytePos`, `EmacsByteRange`, `CharPos0`, and
`CharRange` so the compiler catches coordinate-system mistakes before they
reach backend code.

## Non-Negotiable Invariants

1. `Buffer` owns buffer identity and editor state: point, mark, narrowing,
   overlays, undo state, buffer-local variables, hooks, and buffer lifecycle.
2. `BufferText` owns GNU text semantics: text properties, marker chain,
   char/byte conversion, modification ticks, multibyte state, and shared text
   identity for indirect buffers.
3. `TextBackend` owns physical byte storage only.
4. Concrete backends must not adjust markers, overlays, undo, hooks, narrowing,
   display state, or Lisp-visible buffer state.
5. Search, display, undo, markers, overlays, and Lisp APIs must not know which
   backend is active.
6. GNU-visible behavior must be the same regardless of backend, except for
   explicit Neomacs-only introspection APIs.

## Target Module Structure

Refactor toward this shape:

```text
neovm-core/src/buffer/
  buffer.rs
  buffer_text.rs
  edit_transaction.rs
  position.rs
  text/
    mod.rs
    kind.rs
    metrics.rs
    layout.rs
    backend/
      mod.rs
      gap.rs
      piece_tree.rs
      rope.rs
```

Responsibilities:

- `buffer.rs`: buffer identity, buffer-local state, narrowing, point/mark,
  buffer manager, indirect buffers.
- `buffer_text.rs`: semantic text owner and public buffer text operations.
- `edit_transaction.rs`: GNU-shaped insert/delete/replace transaction order.
- `position.rs`: typed coordinate spaces.
- `text/kind.rs`: backend kind and Lisp symbol mapping.
- `text/metrics.rs`: backend-neutral text metrics.
- `text/layout.rs`: debug and compatibility layout types.
- `text/backend/*`: physical storage implementations only.

## Phase 1: Document And Enforce The Boundary

Add architecture comments near `BufferTextStorage` and `TextBackend` explaining
the ownership split:

- `BufferText` is the semantic layer.
- `TextBackend` is the storage layer.
- Concrete backends must not implement editor semantics.

Add small tests or compile-time structure checks where practical. The important
guard is social and architectural: new backend code should not import marker,
overlay, undo, or buffer-local APIs.

## Phase 2: Strong Position Types

Neomacs has repeated risk around mixed coordinate systems. Add and gradually
migrate to typed positions:

```rust
#[repr(transparent)]
pub struct CharPos0(pub usize);

#[repr(transparent)]
pub struct LispCharPos1(pub i64);

#[repr(transparent)]
pub struct EmacsBytePos(pub usize);

#[repr(transparent)]
pub struct DisplayColumn(pub usize);
```

Rules:

- Lisp boundary APIs use `LispCharPos1`.
- Internal character positions use `CharPos0`.
- Text backend byte APIs use `EmacsBytePos` unless explicitly dealing with
  storage bytes.
- Physical storage coordinates are backend-private implementation details and
  must not appear in the generic `BufferText`/`TextBackend` contract.
- Rendering/display code uses display-specific coordinates.
- Raw `usize` is allowed only inside tight local algorithms after conversion at
  the boundary.

Migration order:

1. Add the types.
2. Use them in new APIs.
3. Convert `BufferText` char/byte conversion APIs.
4. Convert edit, search, marker, and display boundaries.
5. Remove broad raw-position APIs once call sites are clean.

This should catch many position-class bugs at compile time.

Progress:

- `CharPos0`, `CharRange`, `EmacsBytePos`, `EmacsByteRange`, `CharLen`,
  `EmacsByteLen`, and display/narrowing wrappers exist.
- Text-property slice/append/merge APIs at the `BufferText` boundary use typed
  Emacs byte positions/ranges.
- Search replacement update and undo paths carry typed ranges/char lengths.
- `BufferText` forward multibyte scanning stores typed scan state; raw offsets
  remain only for chunk-local slice movement.
- Indexed backends build edit ranges from typed insertion positions and
  measured extents, and whole-buffer debug/dump ranges use backend metrics
  endpoints rather than raw byte lengths.
- Syntax motion helpers convert through typed Emacs byte positions internally
  and unwrap only at legacy public byte-offset returns.
- Navigation intangible/property-boundary helpers carry `EmacsBytePos` through
  text-property and overlay boundary scans.
- Pdump buffer conversion now carries typed Emacs byte positions until the
  final raw compatibility fields are populated.
- `insert-file-contents` post-format handling preserves the accessible start as
  `EmacsBytePos`, matching GNU's separation of buffer byte positions from file
  byte offsets.
- Undo reinsert handling carries typed byte positions through the manager, and
  `primitive-undo` now keeps Lisp-visible undo positions as `LispCharPos1`
  until converting to `CharPos0`/`EmacsBytePos` at the edit boundary, matching
  GNU's Lisp-level `goto-char`/`delete-region` shape.
- Layout bridge copy/count/overlay scan paths isolate the signed display API at
  the bridge and construct typed `EmacsByteRange` values before touching buffer
  snapshots.
- Display output row/point emission now stores temporary row positions as
  `LispCharPos1` and unwraps only when filling the legacy protocol snapshot.
- Rope and piece-tree split helpers distinguish backend-local byte offsets from
  byte lengths and character lengths, so backend-local physical mutation is
  harder to confuse with global Emacs byte positions.
- Text-property validation now crosses explicit `LispCharPos1`/`CharPos0`
  boundaries before converting to Emacs bytes, and production string
  text-property slicing, mode-line property rendering, composition ranges,
  undo yank deletion, pdump interval load, and set-buffer-multibyte overlay
  remapping now construct typed `CharRange`/`EmacsByteRange` values at their
  semantic boundaries.
- Line motion, syntax scanning, marker Lisp-position reporting,
  `window-text-pixel-size`, `position-bytes`, tree-sitter visible-base
  position reporting, and tree-sitter edit invalidation now cross typed
  `LispCharPos1`, `LispBytePos1`, `CharPos0`, and `EmacsBytePos` boundaries
  instead of open-coded `+ 1` conversions at Lisp-facing call sites.
- Accessible narrowed character ranges expose typed Lisp bounds, so undo
  visibility, narrowing normalization, tree-sitter position checks, and
  syntax cache lower bounds no longer recompute `BEGV`/`ZV` with raw `usize`
  arithmetic at their semantic boundaries.
- Text-property interval internals use typed `CharRange` endpoints at their
  semantic split/merge boundaries, and backend storage call sites no longer
  depend on broad `*_usize` endpoint helpers.
- Lisp marker raw storage is converted through one buffer-local
  `marker_data` helper module, so marker chain adjustment, mark marker storage,
  and marker-position reads share one typed `TextPositionAnchor` boundary.
- Marker construction, registration, copying, save-excursion state, search
  match-data reuse, register storage, syntax marker bounds, window history
  point restoration, and pdump marker hashing now keep `LispCharPos1` or
  `EmacsBytePos` until the final Lisp value or dump compatibility boundary.
- Point, narrowing, and edit-state paired writes now route through typed
  anchor setters in `Buffer`. Raw `PT`/`PT_BYTE`, `BEGV`/`BEGV_BYTE`, and
  `ZV`/`ZV_BYTE` assignment is isolated to those setters, while full-buffer
  marker anchors are derived from current text metrics to avoid stale endpoint
  shortcuts after representation changes such as `set-buffer-multibyte`.
- Buffer Lisp-position conversion APIs now require `LispCharPos1`, so callers
  such as buffer builtins, search, coding, window commands, process insertion,
  tree-sitter, XML/zlib helpers, display queries, and navigation helpers must
  explicitly cross the Lisp-position boundary before converting to
  `EmacsBytePos`.
- Raw `CharRange::from_usize` and `EmacsByteRange::from_usize` constructors
  are test-only, so production range construction now has to name typed
  endpoints or lengths at the semantic boundary.
- The unused `CharPos0::from_usize` alias was removed; production code now uses
  `CharPos0::new` as the single explicit constructor for internal 0-based
  character positions.
- `TextInsertion` no longer exposes raw byte/character position accessors;
  callers must use the typed `EmacsBytePos` and `CharPos0` accessors.
- Buffer point restoration and full-buffer char/byte clamp helpers now compare
  and clamp typed `CharPos0`/`EmacsBytePos` values before entering the
  `BufferText` conversion boundary.
- Buffer byte-range clamping now clamps typed `EmacsBytePos` endpoints and
  constructs the final `EmacsByteRange` without round-tripping through raw
  endpoint integers.
- Narrowing, marker anchor construction, accessible point clamping, and
  Lisp-to-accessible byte conversion now clamp typed `CharPos0` or
  `EmacsBytePos` values until the final `BufferText` conversion.
- Layout buffer snapshots now clamp typed character and Emacs-byte positions
  directly before querying the text snapshot.
- Layout-engine tests now use local typed `EmacsByteRange` helpers instead of
  depending on core's test-only raw range constructor across crate boundaries.
- Buffer text newline scans now clamp typed `EmacsBytePos` endpoints and build
  typed ranges before entering backend chunk iteration; raw offsets are limited
  to chunk-local byte arithmetic.
- Text-property object-interval partitioning now carries `CharLen` and
  `CharPos0` through length clamping instead of unwrapping interval endpoints
  and rebuilding typed positions.
- Saved restriction restoration now clamps marker-derived `EmacsBytePos`
  anchors against a typed buffer end position before rebuilding the narrowing
  range.
- Tree-sitter buffer edit tracking now accepts an ordered `EmacsByteRange` and
  unwraps to raw byte offsets only at the tree-sitter API boundary.
- Buffer builtins and display sizing now clamp Lisp-derived character and byte
  positions against typed buffer endpoints before calling conversion or copy
  helpers.
- Buffer-backed Lisp reader decode and slice helpers now convert raw reader
  offsets into typed `EmacsBytePos` + `EmacsByteLen` ranges at the buffer access
  boundary.
- String and display formatting text-property helpers now build character
  ranges as typed `CharPos0` + `CharLen` pairs instead of constructing two raw
  endpoints independently.
- Remaining edit and backend conformance test helpers now keep raw `usize`
  inputs at the test-data boundary, assert ordered endpoints, and construct
  typed ranges from start positions plus typed lengths.
- Shared `BufferText` aliases now have coverage for backend conversion,
  semantic edits, and text-property visibility, while deep clones are checked
  to remain independent.
- Text-property and overlay byte conversion helpers now accept typed
  `LispCharPos1` internally after GNU-style range validation, while preserving
  original Lisp argument values in error payloads.
- Buffer substring, insertion, comparison, narrowing, labeled narrowing,
  edit-function region validation, `call-process-region`,
  `process-send-region`, XML region parsing, zlib decompression, and charset
  region queries now validate like GNU's `validate_region` path and carry
  `LispCharPos1` into the byte-conversion helper instead of passing raw `i64`
  positions through shared semantic helpers.
- Production `emacs_core` call sites no longer use fully-qualified raw
  `LispCharPos1::new` wrappers at byte-conversion sites; imports now make the
  typed boundary explicit in buffer, display, window, process, font,
  tree-sitter, hook, and misc-eval paths.
- Test-only raw `Buffer` point/mark getters and marker registration wrappers
  were removed. Tests now assert through `point_emacs_byte_pos`,
  `point_char_pos`, `mark_emacs_byte_pos`, `mark_char_pos`, and
  `register_marker_at_emacs_byte_pos`, so future test code exercises the same
  typed APIs as production.
- Test-only raw `BufferManager` narrowing, text-property, restriction, labeled
  narrowing, marker creation, and inserted-property clearing shims were removed.
  Tests now construct `EmacsByteRange` or `EmacsBytePos` explicitly and call the
  production typed APIs directly.
- Display and search boundary helpers now use named range/metric types instead
  of anonymous `(usize, usize)` tuples where the pair has semantic meaning:
  mode-line/window text sizing uses typed line/column and text metrics, and
  replacement/isearch byte spans use `MatchGroup`.
- Remaining work is to push these types through the rest of `BufferManager`,
  display engine internals, and string-only helper boundaries so raw `usize` is
  confined to local algorithms after explicit conversion.

## Phase 3: Replace Gap-Shaped Layout With Metrics

Remove the general alias:

```rust
pub type BufferTextLayout = GapTextLayout;
```

Normal code should see only backend-neutral metrics:

```rust
pub struct TextMetrics {
    pub chars: usize,
    pub emacs_bytes: usize,
}
```

Gap internals should stay behind explicit names:

```rust
pub struct GapDebugLayout {
    pub gpt: CharPos0,
    pub gpt_byte: EmacsBytePos,
    pub z: CharPos0,
    pub z_byte: EmacsBytePos,
    pub gap_size: usize,
}

pub enum TextBackendDebugLayout {
    Gap(GapDebugLayout),
    PieceTree(TextMetrics),
    Rope(TextMetrics),
}
```

Guideline:

- Production editor code uses `TextMetrics`.
- Backend tests may use backend debug layout.
- GNU compatibility functions may use gap compatibility layout.
- Display/search/edit code must not depend on `GapDebugLayout`.

Progress:

- Generic backend debug layout is an enum: gap exposes `GapDebugLayout`, while
  piece-tree and rope expose backend-neutral `TextMetrics`.
- `BufferText` exposes gap-compatible layout only for tests/debugging, not as
  the normal production layout API.

## Phase 4: Formal Backend Contract

Keep `TextBackend` as a private enum rather than a public trait object.

Reasoning:

- Enum dispatch gives exhaustiveness when adding `Rope`.
- It avoids dynamic dispatch in hot text operations.
- It keeps pdump/backend-kind conversion explicit.
- It avoids a public trait that leaks backend implementation details.

The backend contract should be exactly:

- `kind`
- `metrics`
- `is_multibyte`
- `set_multibyte`
- `byte_at`
- `emacs_byte_at`
- `char_code_at`
- `char_to_byte`
- `byte_to_char`
- `copy_emacs_bytes_to`
- `for_each_emacs_byte_chunk`
- `has_contiguous_emacs_bytes`
- `with_contiguous_emacs_bytes`
- `insert_emacs_bytes`
- `delete_range`
- `replace_same_len_emacs_bytes`
- `dump_text`
- `from_dump`

The backend contract should not include:

- markers
- text properties
- overlays
- undo
- hooks
- narrowing
- point
- redisplay invalidation
- Lisp values, except for explicitly serialized text bytes

## Phase 5: Central Edit Transaction

Create a single GNU-shaped edit transaction pipeline for insert/delete/replace.

Target order:

1. Normalize and validate positions.
2. Run before-change semantics.
3. Record undo.
4. Adjust text properties.
5. Adjust markers and marker insertion type.
6. Mutate the physical backend.
7. Update metrics, modified ticks, and char-modified ticks.
8. Invalidate position caches, redisplay caches, search caches, and parser
   caches.
9. Run after-change semantics.

Concrete backend mutation should be one step inside that transaction. Backend
code should not know why the edit happened or which semantic side effects are
required.

## Phase 6: Marker, Text Property, And Overlay Ownership

Keep all semantic interval and marker logic above the backend.

Important GNU parity cases:

- marker insertion type `nil` vs `t`
- marker relocation during insert/delete/replace
- marker behavior in indirect buffers
- text property interval split and merge behavior
- sticky/nonsticky property behavior
- overlay movement and evaporating overlays
- undo records for text and text properties
- `buffer-swap-text`

Backend conversion must preserve all of this state.

## Phase 7: GNU Gap Compatibility APIs

GNU exposes physical gap state:

- `(gap-position)` returns `GPT`.
- `(gap-size)` returns `GAP_SIZE`.

For `gap-buffer`, Neomacs should return the real gap state.

For `piece-tree` and `rope`, Neomacs needs explicit compatibility semantics
because GNU has no non-gap backend. Recommended policy:

- Maintain virtual gap compatibility state in `BufferText`, not in the backend.
- Update the virtual gap anchor after edits.
- Return virtual values for `(gap-position)` and `(gap-size)`.
- Do not allow internal editor code to depend on the virtual gap.

This keeps compatibility APIs meaningful without reintroducing gap coupling.

## Phase 8: Piece-Tree Hardening

Before piece-tree becomes a serious option beyond experiments:

- Ensure tree balancing cannot degrade after many edits.
- Benchmark repeated insert/delete near the same position and random positions.
- Ensure chunk iteration is allocation-free for common scans.
- Ensure char/byte conversion uses cached metrics and anchors efficiently.
- Ensure multibyte/unibyte conversion matches GNU behavior.
- Ensure pdump preserves backend kind and enough text state.
- Ensure backend conversion preserves text, markers, text properties, overlays,
  and indirect-buffer sharing.

Piece-tree must pass the same semantic test matrix as gap-buffer.

## Phase 9: Rope Backend

Rope should stay behind the same private `TextBackend` enum contract:

- no marker, overlay, undo, hook, narrowing, or point logic in rope code;
- no Lisp-visible behavior differences from gap-buffer;
- no separate semantic path for rope-specific editing;
- the same backend matrix tests must cover gap-buffer, piece-tree, and rope.

## Phase 10: Test Strategy

Add backend matrix tests for both unit tests and oracle-style tests:

```elisp
(gap-buffer piece-tree rope)
```

Coverage areas:

- insert/delete/replace
- `insert-before-markers`
- marker insertion type
- text properties
- overlays
- undo
- narrowing
- indirect buffers
- `buffer-swap-text`
- search and regex search
- display cursor position
- pdump save/load
- `(gap-position)` and `(gap-size)`

Neomacs-only backend selection APIs can be used to run the same test body over
multiple storage backends. GNU oracle tests still define the expected
Lisp-visible behavior.

## Execution Order

Recommended implementation order:

1. Remove `BufferTextLayout = GapTextLayout`.
2. Add backend-neutral `TextMetrics` and explicit gap debug layout.
3. Rename `primary_anchor()` into a backend-neutral conversion anchor API.
4. Add typed position wrappers and migrate new code first.
5. Convert `BufferText` char/byte conversion APIs to typed boundaries.
6. Move insert/delete/replace semantic ordering into `edit_transaction.rs`.
7. Add backend matrix tests for markers, text properties, undo, narrowing, and
   indirect buffers.
8. Harden piece-tree conversion and chunk iteration.
9. Add virtual gap compatibility state for non-gap backends.
10. Keep rope on the same backend contract and broaden matrix tests whenever
    new semantic behavior is touched.

## Progress

Completed so far:

- Replaced gap-shaped public layout aliases with `TextMetrics`,
  `GapDebugLayout`, and `TextBackendDebugLayout`.
- Moved layout types into `buffer/text/layout.rs`.
- Added typed position, range, length, extent, insertion, and edit range
  wrappers.
- Moved insert/delete/replace measurement toward GNU's paired char/byte edit
  shape: `BufferText` now constructs backend-neutral edit ranges and insertion
  descriptors from Emacs byte positions, while `insdel.rs` carries measured
  `TextExtent` values instead of recomputing raw lengths at each mutation
  site.
- Introduced a typed `TextTransposition` descriptor for GNU
  `transpose-regions` so the Lisp-facing character ranges and computed Emacs
  byte ranges move together through the builtin, current-buffer edit path, and
  shared-buffer sibling updates.
- Tightened private backend boundaries for byte ranges, position conversion,
  edit mutation, byte access, storage/Emacs byte conversion, and conversion
  anchors.
- Centralized Emacs-byte character counting for buffer text storage.
- Added `piece-tree` and `rope` as implemented backend kinds behind the private
  `TextBackend` enum.
- Added backend matrix coverage for markers, text properties, overlays, undo,
  narrowing, indirect buffers, `buffer-swap-text`, search/regex search,
  display-facing mode-line position specs, `window-text-pixel-size`,
  layout-buffer snapshot bytes/positions/properties, full layout-engine
  glyph rows/display points/physical cursor output, display-property
  replacement, invisible text, multiline overlay strings, bidi rows, wrapped
  redisplay retry scrolling, point-line tail retry output, hscroll cursor
  output, hit-test rows, same-engine edit redisplay cache invalidation,
  selective display, glyphless display geometry, composite glyph output,
  redisplay fontification after edits, pdump backend preservation, and
  `(gap-position)` / `(gap-size)` compatibility.
- Added virtual gap compatibility state for non-gap backends.
- Switched position conversion caches to an explicit `BufferText` content epoch
  and invalidated them after every backend content mutation, including same-size
  replacements where `(chars, bytes)` does not change.
- Split the GNU gap compatibility surface from backend conversion anchors:
  `BufferText` now stores virtual gap state as a typed `GapCompatState`, while
  char/byte conversion uses typed `TextPositionBounds` to keep GNU
  `marker.c`-style below/above anchor pairs together.
- Added a private `PhysicalTextBackend` contract so gap-buffer, piece-tree, and
  rope must implement the same storage-only surface while `TextBackend` remains
  an enum with static, exhaustive dispatch.
- Moved `BufferEditState` to typed `TextPositionAnchor` fields for point,
  `BEGV`, and `ZV`, and added internal `TextExtentDelta` helpers so insert,
  delete, and replace state transitions update paired char/byte coordinates
  together, matching GNU's `insdel.c` invariant that semantic positions carry
  both coordinate spaces.
- Added focused edit-transaction tests for equality-boundary behavior around
  insertion positions, deletion starts, replacement starts/ends, `BEGV`, and
  `ZV`.
- Migrated same-length substitution and transposition side-effect paths toward
  typed ranges and anchors: `subst-char-in-region` now derives its character
  span from `TextEditRange`, and `transpose-regions` moves point, sibling
  point state, and marker coordinates through `TextTransposition::transpose_anchor`.
- Added a typed character-range measurement boundary above storage:
  `BufferText` and `Buffer` now construct `TextEditRange` and
  `TextTransposition` from `CharRange`, so Lisp-facing edit code no longer
  manually pairs character positions with byte positions for
  `transpose-regions` and `subst-char-in-region`.
- Threaded the measured `TextEditRange` through same-length substitution and
  shared-buffer sibling updates so undo, hooks, and indirect-buffer side
  effects use the same GNU-style `(char, byte)` descriptor as the text edit.
- Added measured delete/replace entry points on `Buffer` and `BufferManager`,
  then migrated delete/replace hook accounting in delete-region,
  delete-and-extract-region, base64 region replacement, zlib replacement,
  replace-buffer-contents, replace-region-contents, replace-match/search
  replacement, indentation cleanup, line filtering, and rectangle deletion so
  `signal_after_change` and the actual mutation consume the same
  `TextEditRange`.
- Added measured byte-range and character-range delete entry points on
  `BufferManager`, then migrated undo replay, `call-process-region` deletion,
  file replacement, auto-coding work-buffer cleanup, and whole-buffer
  replacement paths so callers choose the coordinate system at the boundary and
  the edit core receives one paired GNU-style `TextEditRange`.
- Added backend matrix coverage proving the measured manager edit entry points
  preserve raw-wrapper behavior for text, point/mark, marker relocation, text
  properties, and undo records.
- Added typed insert marker placement and a measured `MeasuredInsertEdit`
  descriptor so GNU's insert point, character length, byte length, marker
  placement, and replacement marker adjustment move together through current
  and indirect-buffer side effects.  `InsertSideEffectPolicy` now describes
  only the buffer-state scope, not the edit itself.
- Unified Lisp-string insertion paths behind one conversion/measurement helper
  and added backend matrix coverage for shared insert policy side effects,
  including normal insert, insert-before-markers, replacement insert,
  indirect-buffer point/mark state, overlays, markers, and text properties.
- Replaced the raw optional backend conversion anchor with a typed
  `TextPositionHint` and centralized `BufferText` conversion-bound collection
  so gap-buffer contributes GPT as one storage hint while generic char/byte
  conversion keeps the GNU `marker.c` bracketing model above all physical
  backends.
- Added typed point, mark, and narrowing entry points at the `Buffer` and
  `BufferManager` boundary (`EmacsBytePos` / `EmacsByteRange`) and moved
  representative marker, shift-selection, narrowing, indirect-buffer clone,
  and whole-buffer replacement paths onto those typed APIs.
- Added typed marker-chain registration, lookup, movement, and deletion
  adjustment boundaries using `TextPositionAnchor` / `TextEditRange`, while
  keeping raw wrappers only at Lisp/test compatibility edges.
- Collapsed marker remapping for `set-buffer-multibyte` and
  `transpose-regions` to a single `TextPositionAnchor -> TextPositionAnchor`
  boundary, so marker byte and character caches move as one typed value.
- Gated raw text-property convenience wrappers to tests so production callers
  use typed `EmacsBytePos` / `EmacsByteRange` boundaries while the interval
  table stays character-indexed like GNU intervals.
- Moved labeled-restriction marker cloning to typed `TextPositionAnchor`
  lookup/creation so production no longer unwraps raw marker `(byte, char)`
  tuples across the buffer-text boundary.
- Gated raw point and mark mutators to tests so production movement code uses
  typed `EmacsBytePos` / anchor entry points.
- Gated the raw buffer substring `text_range` helper to tests so production
  string extraction uses `EmacsByteRange`.
- Added typed buffer character query helpers for `EmacsBytePos` /
  `EmacsByteLen` and moved `editfns` character deletion/query paths onto
  those entry points.
- Moved charset, insert-hook text property, and selected buffer/symbol
  builtins onto typed buffer character query entry points.
- Moved indentation column scanning and horizontal-space deletion onto typed
  buffer character query entry points.
- Moved navigation line predicates, line-count adjustment, forward-line
  adjustment, and skip-chars loops onto typed buffer character query entry
  points.
- Moved syntax scanning and comment/prefix motion onto typed syntax-character
  units that carry Emacs byte start/end positions, eliminating direct
  production use of raw buffer character query helpers in `emacs_core`.
- Moved dependent layout tests to typed character query APIs and gated raw
  buffer `usize` character query wrappers to `neovm-core` tests.
- Moved dependent layout, binary, and backend benchmark point movement callers
  off raw `goto_byte` and onto typed `EmacsBytePos` entry points.
- Moved `position-bytes` and selected text-property/string interval paths onto
  explicit Lisp-position to `CharPos0`/`EmacsBytePos` conversion, keeping
  GNU's character-indexed interval model visible at the semantic boundary.
- Replaced remaining production `CharRange::from_usize` /
  `EmacsByteRange::from_usize` construction in composition, pdump interval
  load, `set-buffer-multibyte` overlay remapping, mode-line property slicing,
  primitive-undo yank deletion, and string property slicing with typed range
  constructors.
- Typed backend conformance and edit-range test expectations so the backend
  matrix now validates byte/character range boundaries through
  `EmacsBytePos`/`CharPos0` even in test helpers.
- Aligned pdump backend tags with the runtime `BufferTextBackendKind`
  `num_enum` tags and added a round-trip test so serialized backend kind drift
  fails at test time instead of becoming a silent image compatibility bug.
- Moved line motion, syntax scan entry points, `window-text-pixel-size`
  range handling, and `byte-to-position` onto explicit
  `LispCharPos1` / `LispBytePos1` boundary conversions.  The display fix also
  adds multibyte range coverage so character positions are not accidentally
  treated as byte offsets when measuring a window region.
- Centralized the indexed-backend treap priority/serial generator behind a
  shared typed helper, so piece-tree and rope no longer carry duplicated
  balancing seed logic while remaining separate physical storage
  implementations behind the same `TextBackend` contract.
- Collapsed shared-buffer edit metadata propagation onto a single typed edit
  enum for insert/delete/replace/same-length edits, reducing manager-level
  dispatch duplication while preserving the existing GNU-shaped edit ordering.
- Routed shared transposition propagation through that same typed edit metadata,
  including sibling point remapping, so this same-length edit case no longer
  leaves transaction-specific state adjustment inline in the manager loop.
- Replaced closure-based shared edit sibling dispatch with typed
  `SharedTextEditMetadata` dispatch.  The metadata now states whether an edit
  can update point/narrowing fields, preserving same-length substitution's
  no-refresh behavior while routing all shared edits through one manager path.
- Moved the shared edit sibling scope type next to the transaction metadata, so
  `insdel.rs` no longer owns the shape of an indirect-buffer edit transaction.
- Replaced the shared-buffer state-update boolean with a typed
  `SharedBufferStateUpdate` enum, making the distinction between direct field
  updates and state-marker refresh explicit in transaction policy code.
- Moved shared edit side-effect application from the manager loop onto
  `Buffer`, so `BufferManager` now selects affected siblings while `Buffer`
  owns the edit-policy-to-side-effect mapping.
- Replaced nullable shared edit state policy plumbing with a typed
  `SharedTextEditStatePolicy`, so same-length edits explicitly declare that
  they do not participate in point/narrowing state updates.
- Moved shared sibling state-policy derivation onto `SharedTextEditMetadata`,
  removing the remaining manager-level boolean that decided whether an edit
  could touch point/narrowing fields.
- Replaced raw side-effect policy booleans with named transaction enums for
  buffer state fields, accessible-start movement, point-at-insertion behavior,
  and shared text side-data adjustment.
- Replaced the same-length edit modified-state boolean with a named policy, so
  same-length substitutions and transpositions explicitly choose whether they
  record a change or preserve a clean modified state.
- Added a transaction-owned insertion storage plan that keeps converted Emacs
  bytes, measured insert metadata, marker policy, and source text properties
  together before current-buffer insertion mutates storage.
- Added a transaction-owned replacement storage plan that keeps converted
  replacement bytes, measured replacement metadata, and replacement text
  properties together before current-buffer replacement mutates storage.
- Added a transaction-owned deletion storage plan that keeps measured delete
  metadata, deleted text, and marker-adjustment undo entries together before
  current-buffer deletion mutates storage.
- Split current-buffer delete and replace execution into plan-consuming
  transaction executors, matching the existing insert executor shape and
  keeping GNU undo ordering, storage mutation, and side-effect application at
  one explicit boundary for each edit kind.
- Moved the current-buffer insert/delete/replace executors, shared edit
  side-effect dispatch, same-length side effects, and first-change undo
  preparation into `edit_transaction.rs`; `insdel.rs` now keeps the public edit
  entry points while transaction ordering lives with the typed policy layer.
- Moved same-length substitution execution and transpose-region undo recording
  into `edit_transaction.rs`, keeping GNU's per-character delete+insert undo
  for substitution and span-vs-region undo choices for transposition with the
  rest of the typed transaction policy.
- Moved current-buffer transposition execution into `edit_transaction.rs` so
  storage replacement, text-property movement, marker remapping, point update,
  and same-length side effects all consume one `TranspositionStoragePlan`.
- Moved shared-buffer sibling edit propagation into `edit_transaction.rs` so
  sibling state-policy derivation, side-effect application, and state-marker
  refresh are owned by the transaction policy layer instead of `insdel.rs`.
- Added a typed shared-edit executor and `SharedTextEditOutcome`, so public
  manager edit entry points describe the current-buffer mutation result while
  `edit_transaction.rs` owns shared scope capture and sibling propagation.
- Routed minibuffer prompt and initial text installation through the
  `BufferManager` edit transaction path, keeping GNU minibuffer erase/insert
  ordering while centralizing shared-buffer side effects.
- Routed tree-sitter parse-string temp-buffer insertion and `*Messages*`
  logging insertion through `BufferManager`, preserving GNU's hidden-buffer
  and message state-restoration flow while avoiding direct storage mutation in
  those production paths.
- Gated the legacy direct `replace_match_buffer` helper to tests after
  confirming production `replace-match` already computes GNU replacement text
  and applies it through the manager-backed buffer replacement path.
- Moved process-output, erase-buffer, and auto-coding probe point resets onto
  `BufferManager::goto_buffer_emacs_byte_pos`, keeping direct `Buffer` borrows
  out of those production point-movement boundaries after manager edits.
- Moved isolated case-conversion and window-buffer-switch point restoration
  onto `BufferManager::goto_buffer_emacs_byte_pos`; syntax scanner point churn
  remains reserved for a dedicated scanner-level refactor because GNU's
  `syntax.c` also advances point inside scanner algorithms.
- Added a manager-owned point-anchor restore API and routed `fileio.rs`
  insert-file/format-decode point preservation through it, matching GNU's
  point preservation while keeping state-marker updates outside direct
  `Buffer` mutation.
- Routed coding-system buffer-destination point preservation through the same
  manager-owned point-anchor API, leaving direct point-anchor mutation confined
  to the buffer/manager layer.
- Added manager-owned full-range and accessible-region snapshot restoration
  APIs, then routed erase-buffer, minibuffer setup, and `*Messages*` logging
  widen/restore behavior through them so narrowing state-marker updates stay
  above concrete text storage.
- Split buffer regex search matching from point movement: regex helpers now
  return the target byte position, while search builtins apply successful
  motion through `BufferManager::goto_buffer_emacs_byte_pos` after each step.
- Routed `set-buffer-multibyte` shared-buffer state remapping for point,
  narrowing, mark, overlays, last-window-start, and the multibyte flag through
  `BufferManager` methods while preserving the existing byte-boundary
  conversion algorithm.
- Split `forward-comment` scanning from buffer point mutation by introducing a
  local syntax scanner cursor; the scanner records its temporary point in the
  cursor and the builtin applies the final point through `BufferManager`.
- Routed `backward-prefix-chars` and `parse-partial-sexp` final point movement
  through `BufferManager`, matching GNU's primitive-boundary point updates while
  keeping syntax scanners from mutating concrete buffer storage directly.
- Typed the newline scan helpers on `Buffer`/`BufferText` to accept and return
  `EmacsBytePos`, so line navigation callers cross the semantic text boundary
  with explicit byte-position types instead of raw `usize` coordinates.
- Removed raw `usize` accessible-range membership/clamp helpers and replaced
  their production callers with typed `EmacsBytePos` / `CharPos0` checks, so
  narrowing-boundary tests cannot accidentally mix byte and character spaces.
- Removed the raw `CharRange` start/end accessors from production use; string
  comparison and text-property plist validation now order and construct
  character ranges through typed `CharPos0` boundaries.
- Removed the unused raw `EmacsByteRange` start/end accessors so byte ranges
  expose typed endpoints only; callers that need storage indices must unwrap
  `EmacsBytePos` explicitly at the local indexing site.
- Reworked the test-only regex replacement helper to use the buffer replace
  transaction path instead of spelling replacement as direct point movement,
  delete, then insert; backend parity tests now exercise the same replace
  machinery used by runtime callers.
- Added `BufferText` backend matrix coverage for typed newline scanning after
  edits, comparing `next_newline_emacs_byte`, `prev_newline_emacs_byte`, and
  `count_newlines_emacs_byte` against the gap backend for every implemented
  storage backend.
- Replaced production gap-buffer `TextPositionAnchor::from_usize` construction
  with typed `CharPos0`/`EmacsBytePos` anchors and made the raw constructor
  test-only, keeping char/byte anchor pairs explicit in conversion code.

Next work:

- Continue moving full insert/delete/replace ordering into
  `edit_transaction.rs`, including cache invalidation and before/after-change
  hook sequencing.
- Continue migrating semantic layers above `TextBackend` to typed positions in
  larger behavior-preserving chunks.
- Keep expanding backend matrix coverage whenever a newly migrated semantic
  layer starts using `BufferText`.

## Success Criteria

The refactor is successful when:

- normal editor code cannot observe the concrete backend except through
  Neomacs-only introspection APIs;
- display/search/edit/undo paths do not call gap-specific layout APIs;
- backend implementations contain no marker, overlay, undo, hook, or buffer
  local logic;
- gap-buffer, piece-tree, and rope pass the same semantic matrix;
- GNU oracle tests pass with the default gap backend;
- non-gap backends can be enabled for targeted tests without changing
  Lisp-visible behavior.
