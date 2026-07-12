# GNU-Shaped Face Ownership Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development to implement this plan task-by-task.

**Goal:** Make frame-local Lisp face specifications the sole mutable authority and move conversion to render-facing faces out of Lisp setters and into redisplay-owned derived caches.

**Architecture:** Preserve GNU Emacs semantics: face setters mutate frame-local Lisp vectors, mark face state changed, and return; redisplay derives concrete face attributes when needed. First remove eager setter-side realization and duplicate per-frame mirroring, then introduce a frame-scoped generation-keyed cache so unchanged redisplays do no face-table rebuild. Keep the existing font-realization seam at `LayoutEngine::finish_frame_output` intact.

**Tech Stack:** Rust, neovm-core Lisp evaluator, neomacs-layout-engine, cargo-nextest.

---

### Task 1: Make Lisp face mutation specification-only

**Files:**
- Modify: `neovm-core/src/emacs_core/font.rs`
- Test: `neovm-core/src/emacs_core/font_test.rs`

1. Write a regression test proving `internal-set-lisp-face-attribute` changes the frame-local Lisp vector and advances `face_change_count`, but does not update the render-facing `FaceTable` or `Frame::realized_faces` before redisplay synchronization.
2. Run the focused test with `cargo nextest run` and confirm it fails against eager mirroring.
3. Remove setter-side `FaceTable` mutation and realized-face mirroring while retaining GNU-visible font/default-frame side effects.
4. Run the focused core face tests and commit the vertical slice.

### Task 2: Keep redisplay behavior correct through one derived conversion

**Files:**
- Modify: `neovm-core/src/emacs_core/font.rs`
- Modify: `neomacs-layout-engine/src/engine.rs`
- Test: `neomacs-layout-engine/src/engine_test.rs`

1. Add or strengthen a rendered-output test proving a frame-local face mutation is visible on the next layout.
2. Confirm it fails if redisplay conversion is disabled.
3. Make the redisplay entry point the only owner of Lisp-specification-to-runtime-face conversion.
4. Remove redundant mirroring into `Frame::realized_faces` and run focused nextest suites.

### Task 3: Avoid unchanged full-table rebuilding

**Files:**
- Modify: `neovm-core/src/window/mod.rs`
- Modify: `neovm-core/src/emacs_core/font.rs`
- Modify: `neovm-core/src/emacs_core/eval.rs`
- Test: `neovm-core/src/emacs_core/font_test.rs`

1. Add a public-seam test proving two redisplay preparations at the same face generation reuse the derived face state.
2. Confirm the test fails because the current implementation rebuilds unconditionally.
3. Add frame-scoped source/materialized generation tracking and skip conversion while generations match.
4. Prove a later mutation invalidates the cache and commit.

### Task 4: Separate authoritative specifications from disposable realized state

**Files:**
- Modify: `neovm-core/src/window/mod.rs`
- Modify: `neovm-core/src/emacs_core/xfaces/mod.rs`
- Test: `neovm-core/src/emacs_core/xfaces/xfaces_test.rs`

1. Add a test proving clearing derived faces cannot delete the frame's Lisp face specifications.
2. Confirm the existing `clear_realized_faces` behavior fails it.
3. Separate cache clearing from specification clearing and remove the unused per-frame runtime-face mirror if no production reader remains.
4. Run core and layout nextest suites and commit.

### Task 5: Verify the Corfu child-frame hot path

**Files:**
- Modify only if evidence finds another eager path.

1. Run focused nextest suites for `neovm-core` and `neomacs-layout-engine`.
2. Run the full workspace with `cargo nextest run`.
3. Re-run the existing child-frame timing reproduction and compare setter/creation duration with the recorded multi-second baseline.
4. Inspect `git diff --check` and the final diff before reporting completion.
