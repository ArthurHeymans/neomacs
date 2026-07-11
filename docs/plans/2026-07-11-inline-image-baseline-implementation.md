# Inline Image Baseline Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Preserve GNU-compatible image ascent semantics through row layout so tab-bar icons are baseline-aligned.

**Architecture:** Parse image ascent into a typed policy, resolve it once into concrete media ascent, and use that same ascent for both the stretch glyph and final media rectangle. Resolve absolute media `y` only after the row's final baseline is known; downstream output and rendering interfaces remain unchanged.

**Tech Stack:** Rust, Neomacs layout engine, `cargo nextest`

---

### Task 1: Lock down the regression

**Files:**
- Modify: `neomacs-layout-engine/src/display_row_test.rs`

1. Add a configurable-size recording image host.
2. Add a test rendering a 16x16 image with `:ascent center` in an 18-pixel row.
3. Assert the stretch ascent and final image `y` are derived from the shared row baseline.
4. Run `cargo nextest run -p neomacs-layout-engine <test-filter>` and confirm the assertion fails because the image is top-aligned.

### Task 2: Preserve and resolve image ascent

**Files:**
- Modify: `neomacs-layout-engine/src/display_spec.rs`
- Modify: `neomacs-layout-engine/src/display_item.rs`
- Modify: `neomacs-layout-engine/src/display_source_resolver.rs`
- Modify affected test constructors under `neomacs-layout-engine/src/`

1. Add `DisplayImageAscent::{Percent, Center}` with GNU's default of 50 percent.
2. Parse valid numeric and `center` values from the image plist.
3. Resolve the policy with image height and face row metrics.
4. Carry concrete ascent through `DisplayImageItem` and `DisplayMediaReplacement`.
5. Use that ascent in `replacement_stretch`.

### Task 3: Resolve media against the final row baseline

**Files:**
- Modify: `neomacs-layout-engine/src/display_row_render_item.rs`
- Modify: `neomacs-layout-engine/src/display_row_render_state.rs`
- Modify: `neomacs-layout-engine/src/display_row.rs`
- Modify affected tests under `neomacs-layout-engine/src/`

1. Collect pending media with concrete ascent instead of assigning row-top `y` immediately.
2. After row construction, place each medium at `row.pixel_y + row.ascent_px - ascent`.
3. Keep `RenderedDisplayRowMedia` as an absolute rectangle for downstream modules.
4. Run the focused regression and confirm it passes.

### Task 4: Verify and review

**Files:**
- Review all changed files.

1. Run `cargo fmt --all -- --check`.
2. Run `cargo nextest run -p neomacs-layout-engine`.
3. Run broader affected-crate `cargo nextest` suites if dependency changes require them.
4. Run the repository code-review workflow against the pre-change commit.
5. Address actionable findings, rerun verification, and commit the completed change.
