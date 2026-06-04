# Buffer Text Backend Refactor Plan

Date: 2026-06-03
Status: in progress
Scope: `neovm-core` buffer text storage, edit semantics, text properties,
markers, backend selection, and backend parity tests

## Goal

Make Neomacs buffer text storage support multiple physical backends without
letting backend details leak into Lisp-visible buffer semantics.

The long-term architecture should preserve GNU Emacs observable behavior while
allowing Neomacs to use better physical storage for different workloads:

- `gap-buffer` for default GNU-like behavior and compatibility.
- `piece-tree` for large files and edit-heavy workloads.
- `rope` later, only after the backend contract is clean.

The target design is:

```text
Buffer / Lisp-visible editor semantics
  -> BufferText semantic layer
    -> private TextBackend enum
      -> GapBuffer / PieceTree / future Rope
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

The main weakness is that some names and APIs are still gap-shaped:

- `BufferTextLayout = GapTextLayout`.
- `gap_layout()` is public on `BufferText`.
- `TextBackend::primary_anchor()` exposes a `GPT`/`GPT_BYTE`-like concept.
- `BufferTextBackendLayout` mixes generic metrics with gap-specific layout.
- Some tests assert gap layout where they should assert backend-neutral metrics.

These leaks are manageable now, but they should be cleaned before adding a real
rope backend or depending on piece-tree in wider editor paths.

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
pub struct StorageBytePos(pub usize);

#[repr(transparent)]
pub struct DisplayColumn(pub usize);
```

Rules:

- Lisp boundary APIs use `LispCharPos1`.
- Internal character positions use `CharPos0`.
- Text backend byte APIs use `EmacsBytePos` unless explicitly dealing with
  storage bytes.
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

Gap internals should move behind explicit names:

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
    // Add Rope(TextMetrics) only when a real Rope backend lands.
}
```

Guideline:

- Production editor code uses `TextMetrics`.
- Backend tests may use backend debug layout.
- GNU compatibility functions may use gap compatibility layout.
- Display/search/edit code must not depend on `GapDebugLayout`.

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

For `piece-tree` and future `rope`, Neomacs needs explicit compatibility
semantics because GNU has no non-gap backend. Recommended policy:

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

## Phase 9: Rope Later

Do not add a serious rope backend until:

- typed positions are in place,
- layout is backend-neutral,
- edit transactions are centralized,
- backend matrix tests exist,
- piece-tree passes broad semantic coverage.

When added, rope should be another `TextBackend` variant, not a second semantic
path.

## Phase 10: Test Strategy

Add backend matrix tests for both unit tests and oracle-style tests:

```elisp
(gap-buffer piece-tree)
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
10. Only after these are stable, add a real rope backend.

## Progress

Completed so far:

- Replaced gap-shaped public layout aliases with `TextMetrics`,
  `GapDebugLayout`, and `TextBackendDebugLayout`.
- Moved layout types into `buffer/text/layout.rs`.
- Added typed position, range, length, extent, insertion, and edit range
  wrappers.
- Tightened private backend boundaries for byte ranges, position conversion,
  edit mutation, byte access, storage/Emacs byte conversion, and conversion
  anchors.
- Centralized Emacs-byte character counting for buffer text storage.

Next work:

- Continue migrating semantic layers above `TextBackend` to typed positions.
- Add backend matrix tests for markers, text properties, undo, narrowing, and
  indirect buffers.
- Move insert/delete/replace semantic ordering into an explicit transaction
  module once the boundary is fully typed.

## Success Criteria

The refactor is successful when:

- normal editor code cannot observe the concrete backend except through
  Neomacs-only introspection APIs;
- display/search/edit/undo paths do not call gap-specific layout APIs;
- backend implementations contain no marker, overlay, undo, hook, or buffer
  local logic;
- gap-buffer and piece-tree pass the same semantic matrix;
- GNU oracle tests pass with the default gap backend;
- piece-tree can be enabled for targeted tests without changing Lisp-visible
  behavior.
