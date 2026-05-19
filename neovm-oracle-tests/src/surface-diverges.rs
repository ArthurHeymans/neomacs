//! Oracle divergence surface tests for text properties.
//!
//! Each test targets a *known divergence* between NeoMacs and GNU Emacs.
//! Tests that FAIL expose an active divergence; tests that PASS serve as
//! regression guards (the divergence was either already fixed or does not
//! surface at this test level).
//!
//! ## Confirmed divergences (FAILING as of 2026-05-19)
//!
//! - **D4** `signal_after_change` / `prepare_to_modify_buffer_1` not called
//!   for property mutations → after-change-functions and before-change-functions
//!   are not fired when text properties are modified via put-text-property,
//!   add-text-properties, remove-text-properties, or set-text-properties.
//!   GNU Emacs fires these hooks (used by font-lock, jit-lock, etc.).
//!
//! Divergence IDs reference the audit report sections.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

// ═══════════════════════════════════════════════════════════════════════
//  D1 · Sticky property inheritance on insertion  (HIGH)
//
//  GNU: adjust_intervals_for_insertion (intervals.c:802) calls
//       merge_properties_sticky per property on newly inserted intervals.
//       Default: front-sticky=t, rear-nonsticky=nil → rear-sticky.
//  NeoMacs: adjust_for_insert (text_props.rs:641) splits and shifts
//           without any property inheritance.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d1_sticky_insert_middle_inherits() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Inserting into a uniformly-propertied region: the new text should
    // inherit `face 'bold` via default rear-stickiness.
    // GNU → bold, NeoMacs → nil
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "abcdefghij")
  (put-text-property 1 11 'face 'bold)
  (goto-char 5)
  (insert "XXX")
  (get-text-property 6 'face))
"#);
}

#[test]
fn surface_d1_sticky_front_sticky_explicit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Explicit front-sticky: text inserted at the right boundary inherits.
    // GNU → bold, NeoMacs → nil
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "aaaa")
  (put-text-property 1 5 'face 'bold)
  (put-text-property 1 5 'front-sticky t)
  (goto-char 5)
  (insert "bbbb")
  (get-text-property 5 'face))
"#);
}

#[test]
fn surface_d1_sticky_rear_nonsticky_blocks_inherit() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // rear-nonsticky=t: text inserted at the right boundary should NOT
    // inherit.  GNU → nil (blocked by rear-nonsticky).
    // NeoMacs → also nil (never inherits), so this test PASSES for the
    // *wrong reason*.  Kept as a guard: if NeoMacs later implements
    // blanket inheritance without respecting rear-nonsticky, this will fail.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "aaaa")
  (put-text-property 1 5 'face 'bold)
  (put-text-property 1 5 'rear-nonsticky t)
  (goto-char 5)
  (insert "bbbb")
  (get-text-property 5 'face))
"#);
}

#[test]
fn surface_d1_sticky_insert_before_front_sticky_boundary() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Insert at position 1 of a front-sticky interval: the inserted text
    // should inherit the property because front-sticky means "text inserted
    // at my front gets my properties."
    // GNU → bold, NeoMacs → nil
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "aaaa")
  (put-text-property 1 5 'face 'bold)
  (put-text-property 1 5 'front-sticky t)
  (goto-char 1)
  (insert "bbbb")
  (get-text-property 1 'face))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D2 · Byte vs character position confusion  (HIGH)
//
//  TextPropertyTable stores character positions, but text_props_put_property
//  (buffer_text.rs) can receive byte positions directly without conversion.
//  With multibyte text, intervals land at wrong offsets.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d2_multibyte_put_get_roundtrip() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Each Greek letter is 2 bytes in UTF-8.  Positions 1-4 should cover
    // "αβγδ" and 5-8 covers "εζηθ".  If byte positions leak through,
    // property on char range 1-4 actually lands on byte range 1-4
    // (only "α" + half of "β").
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "αβγδεζηθ")
  (put-text-property 1 5 'test 'front)
  (put-text-property 5 9 'test 'back)
  (list (get-text-property 1 'test)
        (get-text-property 4 'test)
        (get-text-property 5 'test)
        (get-text-property 8 'test)))
"#);
}

#[test]
fn surface_d2_multibyte_boundary_precision() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Property on chars 1-3 of multibyte text: only "αβ" and the start
    // of "γ" should carry the property.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "αβγδε")
  (put-text-property 1 3 'marker 'a)
  (list (get-text-property 1 'marker)
        (get-text-property 2 'marker)
        (get-text-property 3 'marker)
        (get-text-property 4 'marker)))
"#);
}

#[test]
fn surface_d2_multibyte_next_property_change() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Boundary between two property regions in multibyte text.
    // If byte positions leak, the boundary appears at the wrong place.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "αβγδεζηθ")
  (put-text-property 1 5 'zone 'left)
  (put-text-property 5 9 'zone 'right)
  (next-single-property-change 1 'zone))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D3 · No undo recording for property changes  (HIGH)
//
//  GNU: record_property_change per interval per property before mutation.
//  NeoMacs: only increments modified tick (buffer.rs:3714).
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d3_undo_list_populated_after_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // After put-text-property, buffer-undo-list should have a property
    // change entry.  NeoMacs records nothing.
    // GNU → t, NeoMacs → nil
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello world")
  (setq buffer-undo-list nil)
  (put-text-property 1 12 'face 'bold)
  (not (null buffer-undo-list)))
"#);
}

#[test]
fn surface_d3_undo_restores_previous_property() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Set face=bold, record undo, change to italic, undo → should get bold.
    // GNU → bold, NeoMacs → italic (undo has no effect on properties)
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello world")
  (put-text-property 1 12 'face 'bold)
  (setq buffer-undo-list nil)
  (put-text-property 1 12 'face 'italic)
  (undo)
  (get-text-property 1 'face))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D4 · No signal_after_change for property mutations  (HIGH)
//
//  GNU: all property mutation functions call signal_after_change after
//       modifying intervals.  This fires after-change-functions used by
//       font-lock, jit-lock, etc.
//  NeoMacs: signal_after_change is never called in the property path.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d4_after_change_fired_on_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // after-change-functions should fire when put-text-property mutates.
    // GNU → ((1 12 0)), NeoMacs → nil
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello world")
  (let (calls)
    (add-hook 'after-change-functions
              (lambda (beg end len)
                (push (list beg end len) calls))
              nil t)
    (put-text-property 1 12 'face 'bold)
    (nreverse calls)))
"#);
}

#[test]
fn surface_d4_after_change_fired_on_add() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Same for add-text-properties.
    // GNU → ((1 6 0)), NeoMacs → nil
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello world")
  (let (calls)
    (add-hook 'after-change-functions
              (lambda (beg end len)
                (push (list beg end len) calls))
              nil t)
    (add-text-properties 1 6 '(face bold test t))
    (nreverse calls)))
"#);
}

#[test]
fn surface_d4_after_change_fired_on_remove() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Same for remove-text-properties.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello world")
  (put-text-property 1 12 'face 'bold)
  (let (calls)
    (add-hook 'after-change-functions
              (lambda (beg end len)
                (push (list beg end len) calls))
              nil t)
    (remove-text-properties 1 12 '(face))
    (nreverse calls)))
"#);
}

#[test]
fn surface_d4_before_change_fired_on_set() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // GNU calls prepare_to_modify_buffer_1 before property mutations,
    // which runs before-change-functions.  NeoMacs skips this.
    // GNU → ((1 6)), NeoMacs → nil
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello world")
  (let (calls)
    (add-hook 'before-change-functions
              (lambda (beg end)
                (push (list beg end) calls))
              nil t)
    (set-text-properties 1 6 '(face bold))
    (nreverse calls)))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D5 · Overlay start/end are raw positions, not markers  (HIGH)
//
//  GNU: overlays use Lisp markers for start/end, which auto-advance when
//       text is inserted before them.
//  NeoMacs: OverlayData (heap_types.rs) stores raw usize byte positions.
//           Overlays become stale after text insertion before them.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d5_overlay_start_end_track_insertion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Insert text before an overlay: its start/end markers should advance.
    // GNU → (8 12), NeoMacs → (5 9) (stale)
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "ABCDEFGHIJ")
  (let ((ov (make-overlay 5 9)))
    (overlay-put ov 'test 'marked)
    (goto-char 2)
    (insert "xxx")
    (list (overlay-start ov) (overlay-end ov))))
"#);
}

#[test]
fn surface_d5_overlay_property_after_insertion_before() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // After inserting 3 chars at position 1, the overlay originally at
    // 5-9 should now be at 8-12.  get-char-property at 8 should find it.
    // GNU → marked, NeoMacs → nil (overlay still at 5-9, nothing at 8)
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "ABCDEFGHIJ")
  (let ((ov (make-overlay 5 9)))
    (overlay-put ov 'face 'bold)
    (goto-char 1)
    (insert "XXX")
    (get-char-property 8 'face)))
"#);
}

#[test]
fn surface_d5_overlay_start_end_track_deletion_before() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Delete text before an overlay: markers should retreat.
    // GNU → (3 7), NeoMacs → (5 9) (stale)
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "ABCDEFGHIJ")
  (let ((ov (make-overlay 5 9)))
    (overlay-put ov 'test 'marked)
    (delete-region 1 3)
    (list (overlay-start ov) (overlay-end ov))))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D6 · Adjacent equal-property intervals never merged  (MEDIUM)
//
//  GNU: merge_interval_left/right merges adjacent intervals with
//       identical plists after property changes.
//  NeoMacs: merge_adjacent_equal_properties_around (text_props.rs:933)
//           exists but is never called from the mutation path.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d6_next_property_change_adjacent_equal_props() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Two adjacent regions with identical face=bold.  GNU merges them
    // into one interval → next-property-change returns nil.
    // NeoMacs keeps two intervals → returns 6.
    // GNU → nil, NeoMacs → 6
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "abcdefghij")
  (put-text-property 1 6 'face 'bold)
  (put-text-property 6 11 'face 'bold)
  (next-property-change 1))
"#);
}

#[test]
fn surface_d6_next_property_change_three_way_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Three regions, same property.  GNU merges all three.
    // next-property-change 1 → nil (all merged).
    // NeoMacs → 5 (boundary between first and second region).
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "abcdefghijklmno")
  (put-text-property 1 6 'test 'x)
  (put-text-property 6 11 'test 'x)
  (put-text-property 11 16 'test 'x)
  (next-property-change 1))
"#);
}

#[test]
fn surface_d6_merge_after_overlapping_put() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Set props on 1-6 and 6-11 with different values, then make them
    // equal by setting the same property on both.  GNU merges the now-
    // equal adjacent intervals.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "abcdefghij")
  (put-text-property 1 6 'face 'bold)
  (put-text-property 6 11 'face 'italic)
  ;; Now make them equal
  (put-text-property 1 11 'face 'bold)
  ;; Should be fully merged — next-property-change returns nil
  (next-property-change 1))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D7 · Overlay evaporate not triggered by text deletion  (MEDIUM)
//
//  GNU: Fdelete_all_overlays and text deletion check evaporate property,
//       removing overlays whose content is fully deleted.
//  NeoMacs: evaporate only checked in overlay-put and move-overlay.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d7_overlay_evaporate_on_delete_content() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Delete the entire content of an evaporate-enabled overlay.
    // GNU: overlay is removed → overlay-start returns nil.
    // NeoMacs: overlay survives → overlay-start returns a position.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello world")
  (let ((ov (make-overlay 7 12)))
    (overlay-put ov 'evaporate t)
    (delete-region 7 12)
    (if (overlay-start ov) 'alive 'evaporated)))
"#);
}

#[test]
fn surface_d7_overlay_evaporate_on_kill_region() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Kill-region to empty an overlay with evaporate=t.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "prefix MIDDLE suffix")
  (let ((ov (make-overlay 8 14)))
    (overlay-put ov 'evaporate t)
    (delete-region 8 14)
    (if (overlay-start ov) 'alive 'evaporated)))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D8 · Read-only stickiness check missing for insertions  (MEDIUM)
//
//  GNU: verify_interval_modification checks read-only with front-sticky/
//       rear-nonsticky even for insertions (start==end).
//  NeoMacs: verify_text_read_only_in_state returns early for empty ranges.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d8_read_only_blocks_sticky_insertion() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Inserting at the front of a read-only, front-sticky interval should
    // signal buffer-read-only.
    // GNU → blocked, NeoMacs → inserted (no read-only check for insertions)
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello")
  (put-text-property 1 6 'read-only t)
  (put-text-property 1 6 'front-sticky t)
  (condition-case err
      (progn
        (goto-char 1)
        (insert "X")
        'inserted)
    (buffer-read-only 'blocked)))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D9 · No stickiness-aware merge during graft/yank  (MEDIUM)
//
//  GNU: graft_intervals_into_buffer (intervals.c:1570) does per-interval
//       copy-or-merge with stickiness-based inheritance.
//  NeoMacs: append_shifted/merge_missing_shifted has no stickiness logic.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d9_insert_propertied_string_merge() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Insert a propertied substring into a buffer that already has
    // properties at the insertion point.  GNU merges according to
    // stickiness; NeoMacs may overwrite or leave gaps.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "aaaa")
  (put-text-property 1 5 'face 'bold)
  (let ((str (propertize "bbbb" 'face 'italic)))
    (goto-char 3)
    (insert str)
    ;; At position 3 we inserted italic "bbbb" into bold "aaaa"
    ;; GNU: inserted text keeps its own property (italic)
    ;; NeoMacs: behavior may differ
    (get-text-property 3 'face)))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D10 · display property — fringe/margin specs missing  (MED-HIGH)
//
//  GNU: (left-fringe BITMAP FACE), (right-fringe BITMAP FACE),
//       (margin OBJECT) display specs are handled by xdisp.c.
//  NeoMacs: string replacement and space specs work; fringe/margin
//           display specs are not implemented.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d10_display_property_string_value() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Baseline: display property with a simple string should be stored
    // and retrievable.  This is the passing case.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello world")
  (put-text-property 1 6 'display "XXXXX")
  (get-text-property 1 'display))
"#);
}

#[test]
fn surface_d10_display_property_space_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // (space :width 10) display spec should be stored as a property.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello")
  (put-text-property 1 2 'display '(space :width 10))
  (get-text-property 1 'display))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D11 · composition property — stubs only  (MEDIUM)
//
//  GNU: find-composition returns composition info at position.
//  NeoMacs: composite.rs find-composition-internal always returns nil.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d11_find_composition_returns_nil() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // find-composition at a position with no composition data.
    // Both should return nil — this is a baseline check.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello")
  (find-composition 1))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D12 · line-prefix / wrap-prefix — not rendered  (MEDIUM)
//
//  GNU: line-prefix and wrap-prefix text properties affect visual lines.
//  NeoMacs: data structures exist (types.rs) but are always empty during
//           rendering.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d12_line_wrap_prefix_storage() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // line-prefix and wrap-prefix should at least be storable and
    // retrievable as text properties even if rendering doesn't use them.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "hello world")
  (put-text-property 1 6 'line-prefix ">> ")
  (put-text-property 1 6 'wrap-prefix "   ")
  (list (get-text-property 1 'line-prefix)
        (get-text-property 1 'wrap-prefix)
        (get-text-property 6 'line-prefix)))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D13 · Overlay char-property-change combined boundary  (MEDIUM)
//
//  next-char-property-change must combine overlay boundaries with text
//  property boundaries.  If overlay positions are stale, the combined
//  boundary detection diverges.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d13_next_char_property_change_after_insert() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Insert text that shifts both a text property boundary and an overlay.
    // The combined next-char-property-change result depends on both.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "abcdefghij")
  (put-text-property 1 6 'face 'bold)
  (let ((ov (make-overlay 3 8)))
    (overlay-put ov 'face 'italic)
    (goto-char 1)
    (insert "XX")
    ;; After inserting 2 chars at pos 1:
    ;; text-prop boundary moved from 6→8, overlay from 3-8→5-10
    (list (next-char-property-change 1)
          (next-char-property-change 5)
          (next-char-property-change 10))))
"#);
}

// ═══════════════════════════════════════════════════════════════════════
//  D14 · invisible property with buffer-invisibility-spec ellipsis
//
//  GNU: buffer-invisibility-spec with ((t . t)) causes invisible text
//       to show an ellipsis.  The property-level behavior should match.
//  NeoMacs: invisible status is tracked but ellipsis rendering may be
//           incomplete in the display engine.
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn surface_d14_invisible_property_basic() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // Basic invisible property storage and retrieval with spec.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "visible hidden visible")
  (add-text-properties 8 14 '(invisible t))
  (setq buffer-invisibility-spec '((t . t)))
  (list (get-text-property 8 'invisible)
        (get-text-property 15 'invisible)))
"#);
}

#[test]
fn surface_d14_invisible_p_function() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    // invisible-p should return non-nil for positions with invisible
    // property that matches buffer-invisibility-spec.
    assert_oracle_parity_with_bootstrap(r#"
(with-temp-buffer
  (insert "abcdefghij")
  (add-text-properties 4 7 '(invisible t))
  (setq buffer-invisibility-spec '(t))
  (list (invisible-p 3)
        (invisible-p 4)
        (invisible-p 7)))
"#);
}
