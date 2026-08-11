use super::*;
use crate::buffer::BufferId;

fn alloc_overlay(start: usize, end: usize) -> Value {
    Value::make_overlay(OverlayData {
        serial: 0,
        plist: Value::NIL,
        buffer: Some(BufferId(1)),
        start,
        end,
        front_advance: false,
        rear_advance: false,
    })
}

fn emacs_byte_pos(byte: usize) -> EmacsBytePos {
    EmacsBytePos::new(byte)
}

fn emacs_byte_range(start: usize, end: usize) -> EmacsByteRange {
    EmacsByteRange::from_usize(start, end)
}

fn emacs_byte_len(len: usize) -> EmacsByteLen {
    EmacsByteLen::new(len)
}

fn overlay_start(list: &OverlayList, overlay: Value) -> Option<usize> {
    list.overlay_start_emacs_byte_pos(overlay)
        .map(EmacsBytePos::get)
}

fn overlay_end(list: &OverlayList, overlay: Value) -> Option<usize> {
    list.overlay_end_emacs_byte_pos(overlay)
        .map(EmacsBytePos::get)
}

fn overlays_at(list: &OverlayList, pos: usize) -> Vec<Value> {
    list.overlays_at_emacs_byte_pos(emacs_byte_pos(pos))
}

fn overlays_in_region(
    list: &OverlayList,
    start: usize,
    end: usize,
    accessible_end: usize,
) -> Vec<Value> {
    list.overlays_in_accessible_emacs_byte_range(
        emacs_byte_range(start, end),
        emacs_byte_pos(accessible_end),
    )
}

#[test]
fn insert_and_delete_overlay_preserves_object_identity() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let overlay = alloc_overlay(2, 5);
    list.insert_overlay(overlay);
    assert_eq!(overlays_at(&list, 3), vec![overlay]);
    assert!(list.delete_overlay(overlay));
    assert!(overlays_at(&list, 3).is_empty());
    assert!(overlay_live_buffer(overlay).is_none());
}

#[test]
fn same_range_overlays_remain_distinct_objects() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let first = alloc_overlay(2, 5);
    let second = alloc_overlay(2, 5);
    list.insert_overlay(first);
    list.insert_overlay(second);

    let overlays = overlays_at(&list, 3);
    assert_eq!(overlays.len(), 2);
    assert!(overlays.iter().any(|overlay| eq_value(overlay, &first)));
    assert!(overlays.iter().any(|overlay| eq_value(overlay, &second)));

    assert!(list.delete_overlay(first));
    let overlays = overlays_at(&list, 3);
    assert_eq!(overlays.len(), 1);
    assert!(eq_value(&overlays[0], &second));
}

#[test]
fn raw_overlays_at_matches_gnu_same_start_itree_order() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let first = alloc_overlay(2, 5);
    let second = alloc_overlay(2, 5);
    let third = alloc_overlay(2, 5);
    list.insert_overlay(first);
    list.insert_overlay(second);
    list.insert_overlay(third);

    assert_eq!(overlays_at(&list, 3), vec![third, second, first]);

    assert!(list.delete_overlay(second));
    assert_eq!(overlays_at(&list, 3), vec![third, first]);
}

#[test]
fn text_edit_relocation_preserves_same_start_itree_order() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let first = alloc_overlay(2, 5);
    let second = alloc_overlay(2, 5);
    let third = alloc_overlay(2, 5);
    list.insert_overlay(first);
    list.insert_overlay(second);
    list.insert_overlay(third);

    list.adjust_for_insert_at_emacs_byte_pos(emacs_byte_pos(1), emacs_byte_len(3), true);

    assert_eq!(overlays_at(&list, 6), vec![third, second, first]);
}

#[test]
fn overlays_at_prunes_right_subtree_when_all_starts_are_after_position() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    for index in 0..2_000 {
        let start = 10 + index * 2;
        list.insert_overlay(alloc_overlay(start, start + 1));
    }

    reset_overlays_at_node_visit_count();
    assert!(overlays_at(&list, 0).is_empty());

    let visits = overlays_at_node_visit_count();
    assert!(
        visits < 8,
        "overlays_at should prune right subtrees that start after the queried position; visited {visits} nodes"
    );
}

#[test]
fn monotonic_overlay_insertion_keeps_interval_index_logarithmic() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let overlay_count = 2_000usize;
    for index in 0..overlay_count {
        let start = 10 + index * 2;
        list.insert_overlay(alloc_overlay(start, start + 1));
    }

    // GNU's red-black interval tree has height <= 2 * log2(n + 1).  This is
    // an interface performance guarantee, not a test of a particular balancing
    // algorithm: AVL, red-black, and a high-fanout tree all satisfy it.
    let logarithmic_height_bound = 2 * (usize::BITS - overlay_count.leading_zeros()) as usize;
    assert!(
        list.interval_index_height() <= logarithmic_height_bound,
        "sorted insertion produced interval-index height {}; expected at most {} for {} overlays",
        list.interval_index_height(),
        logarithmic_height_bound,
        overlay_count
    );
}

#[test]
fn inserted_position_property_lookup_inspects_only_nearby_overlays() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let property = Value::symbol("face");
    let target = alloc_overlay(2, 8);
    list.insert_overlay(target);
    list.overlay_put(target, property, Value::symbol("bold"))
        .unwrap();
    for index in 0..2_000 {
        let start = 100 + index * 2;
        list.insert_overlay(alloc_overlay(start, start + 1));
    }

    reset_best_overlay_candidate_inspection_count();
    assert_eq!(
        list.highest_priority_overlay_for_inserted_emacs_byte_pos(emacs_byte_pos(4), &property,),
        Some(target)
    );
    let inspections = best_overlay_candidate_inspection_count();
    assert!(
        inspections < 32,
        "an inserted-position lookup should inspect only interval-tree candidates; inspected {inspections} overlays"
    );
}

#[test]
fn tail_insertion_inspects_only_overlays_with_affected_endpoints() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    for index in 0..2_000 {
        let start = index * 2;
        list.insert_overlay(alloc_overlay(start, start + 1));
    }
    let affected = alloc_overlay(5_000, 5_010);
    list.insert_overlay(affected);

    reset_overlay_edit_candidate_inspection_count();
    list.adjust_for_insert_at_emacs_byte_pos(emacs_byte_pos(5_000), emacs_byte_len(3), true);

    assert_eq!(overlay_start(&list, affected), Some(5_003));
    assert_eq!(overlay_end(&list, affected), Some(5_013));
    let inspections = overlay_edit_candidate_inspection_count();
    assert!(
        inspections < 32,
        "a tail insertion should inspect only overlays with affected endpoints; inspected {inspections} overlays"
    );
}

#[test]
fn tail_deletion_inspects_only_overlays_with_affected_endpoints() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    for index in 0..2_000 {
        let start = index * 2;
        list.insert_overlay(alloc_overlay(start, start + 1));
    }
    let affected = alloc_overlay(5_000, 5_010);
    list.insert_overlay(affected);

    reset_overlay_edit_candidate_inspection_count();
    list.adjust_for_delete_emacs_byte_range(emacs_byte_range(4_997, 5_000));

    assert_eq!(overlay_start(&list, affected), Some(4_997));
    assert_eq!(overlay_end(&list, affected), Some(5_007));
    let inspections = overlay_edit_candidate_inspection_count();
    assert!(
        inspections < 32,
        "a tail deletion should inspect only overlays with affected endpoints; inspected {inspections} overlays"
    );
}

#[test]
fn raw_overlays_in_matches_gnu_same_start_itree_order() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let start = alloc_overlay(1, 5);
    let end = alloc_overlay(48, 52);
    let full = alloc_overlay(1, 52);
    list.insert_overlay(start);
    list.insert_overlay(end);
    list.insert_overlay(full);

    assert_eq!(overlays_in_region(&list, 1, 52, 52), vec![full, start, end]);
}

#[test]
fn sorted_overlay_precedence_matches_gnu_same_range_identity_order() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let first = alloc_overlay(2, 5);
    let second = alloc_overlay(2, 5);
    list.insert_overlay(first);
    list.insert_overlay(second);

    let mut overlays = overlays_at(&list, 3);
    list.sort_overlay_ids_by_priority_desc(&mut overlays);
    assert_eq!(overlays, vec![second, first]);
}

#[test]
fn delete_overlay_removes_non_root_interval_entry() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let root = alloc_overlay(20, 30);
    let earlier = alloc_overlay(2, 10);
    list.insert_overlay(root);
    list.insert_overlay(earlier);

    assert_eq!(overlays_at(&list, 5), vec![earlier]);
    assert!(list.delete_overlay(earlier));
    assert!(overlays_at(&list, 5).is_empty());
    assert_eq!(overlays_at(&list, 25), vec![root]);
    assert!(overlay_live_buffer(earlier).is_none());
}

#[test]
fn overlay_put_prepends_new_properties_and_updates_in_place() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let overlay = alloc_overlay(0, 1);
    list.insert_overlay(overlay);
    let face = Value::symbol("face");
    let help = Value::symbol("help-echo");
    list.overlay_put(overlay, face, Value::symbol("bold"))
        .unwrap();
    list.overlay_put(overlay, help, Value::string("tip"))
        .unwrap();
    list.overlay_put(overlay, face, Value::symbol("italic"))
        .unwrap();
    let plist = list.overlay_plist(overlay).unwrap();
    assert_eq!(
        crate::emacs_core::print::print_value(&plist),
        "(help-echo \"tip\" face italic)"
    );
}

#[test]
fn move_overlay_updates_boundaries() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let overlay = alloc_overlay(0, 2);
    list.insert_overlay(overlay);
    list.move_overlay_to_emacs_byte_range(overlay, emacs_byte_range(4, 7));
    assert_eq!(overlay_start(&list, overlay), Some(4));
    assert_eq!(overlay_end(&list, overlay), Some(7));
    assert_eq!(overlays_at(&list, 5), vec![overlay]);
}

#[test]
fn move_overlay_removes_old_non_root_interval_entry() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let root = alloc_overlay(20, 30);
    let earlier = alloc_overlay(2, 10);
    list.insert_overlay(root);
    list.insert_overlay(earlier);

    list.move_overlay_to_emacs_byte_range(earlier, emacs_byte_range(40, 45));
    assert!(overlays_at(&list, 5).is_empty());
    assert_eq!(overlays_at(&list, 25), vec![root]);
    assert_eq!(overlays_at(&list, 42), vec![earlier]);
    assert_eq!(overlay_start(&list, earlier), Some(40));
    assert_eq!(overlay_end(&list, earlier), Some(45));
}

#[test]
fn move_overlay_evaporates_zero_width_overlay() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let overlay = alloc_overlay(2, 5);
    list.insert_overlay(overlay);
    list.overlay_put(overlay, Value::symbol("evaporate"), Value::T)
        .unwrap();
    list.move_overlay_to_emacs_byte_range(overlay, emacs_byte_range(4, 4));
    assert!(list.is_empty());
    assert!(overlay_live_buffer(overlay).is_none());
}

#[test]
fn insert_adjusts_front_and_rear_advance() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let overlay = alloc_overlay(5, 10);
    list.insert_overlay(overlay);
    list.set_front_advance(overlay, true);
    list.set_rear_advance(overlay, true);
    list.adjust_for_insert_at_emacs_byte_pos(emacs_byte_pos(5), emacs_byte_len(2), false);
    assert_eq!(overlay_start(&list, overlay), Some(7));
    assert_eq!(overlay_end(&list, overlay), Some(12));
}

#[test]
fn empty_front_advance_overlay_does_not_invert_on_insert() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let overlay = alloc_overlay(5, 5);
    list.insert_overlay(overlay);
    list.set_front_advance(overlay, true);
    list.set_rear_advance(overlay, false);
    list.adjust_for_insert_at_emacs_byte_pos(emacs_byte_pos(5), emacs_byte_len(2), false);
    assert_eq!(overlay_start(&list, overlay), Some(5));
    assert_eq!(overlay_end(&list, overlay), Some(5));
}

#[test]
fn before_markers_insert_moves_overlay_boundaries_at_point() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let starts_here = alloc_overlay(5, 10);
    let ends_here = alloc_overlay(2, 5);
    let empty = alloc_overlay(5, 5);
    list.insert_overlay(starts_here);
    list.insert_overlay(ends_here);
    list.insert_overlay(empty);
    list.adjust_for_insert_at_emacs_byte_pos(emacs_byte_pos(5), emacs_byte_len(2), true);
    assert_eq!(overlay_start(&list, starts_here), Some(7));
    assert_eq!(overlay_end(&list, starts_here), Some(12));
    assert_eq!(overlay_start(&list, ends_here), Some(2));
    assert_eq!(overlay_end(&list, ends_here), Some(7));
    assert_eq!(overlay_start(&list, empty), Some(7));
    assert_eq!(overlay_end(&list, empty), Some(7));
}

#[test]
fn replace_preserves_overlay_spanning_replaced_text() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let overlay = alloc_overlay(25, 32);
    list.insert_overlay(overlay);

    list.adjust_for_replace_at_emacs_byte_pos(
        emacs_byte_pos(26),
        emacs_byte_len(5),
        emacs_byte_len(5),
    );
    assert_eq!(overlay_start(&list, overlay), Some(25));
    assert_eq!(overlay_end(&list, overlay), Some(32));

    list.adjust_for_insert_at_emacs_byte_pos(emacs_byte_pos(10), emacs_byte_len(15), false);
    assert_eq!(overlay_start(&list, overlay), Some(40));
    assert_eq!(overlay_end(&list, overlay), Some(47));
}

#[test]
fn delete_evaporates_zero_width_overlay() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let overlay = alloc_overlay(5, 10);
    list.insert_overlay(overlay);
    list.overlay_put(overlay, Value::symbol("evaporate"), Value::T)
        .unwrap();
    list.adjust_for_delete_emacs_byte_range(emacs_byte_range(5, 10));
    assert!(list.is_empty());
    assert!(overlay_live_buffer(overlay).is_none());
}

#[test]
fn priority_sort_uses_gnu_precedence_rules() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let low = alloc_overlay(2, 7);
    let high = alloc_overlay(4, 7);
    list.insert_overlay(low);
    list.insert_overlay(high);
    list.overlay_put(low, Value::symbol("face"), Value::symbol("bold"))
        .unwrap();
    list.overlay_put(low, Value::symbol("priority"), Value::fixnum(1))
        .unwrap();
    list.overlay_put(high, Value::symbol("face"), Value::symbol("italic"))
        .unwrap();
    list.overlay_put(
        high,
        Value::symbol("priority"),
        Value::cons(Value::fixnum(1), Value::fixnum(2)),
    )
    .unwrap();
    let mut ids = overlays_at(&list, 4);
    list.sort_overlay_ids_by_priority_desc(&mut ids);
    assert_eq!(ids, vec![high, low]);
}

#[test]
fn highest_priority_property_value_wins_outright_over_lower_precedence_carriers() {
    // GNU `get_char_property_and_overlay`: the highest-precedence overlay carrying
    // the property wins OUTRIGHT. No lower-precedence overlay gets a say, even
    // when the winner's value means "inactive" downstream -- e.g. an `invisible`
    // value absent from `buffer-invisibility-spec` keeps the text VISIBLE, and a
    // lower-priority `invisible` must not hide it anyway. Scanning on past the
    // winner is what hid text GNU shows.
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let property = Value::symbol("invisible");
    let high = alloc_overlay(2, 10);
    let low = alloc_overlay(2, 10);
    list.insert_overlay(high);
    list.insert_overlay(low);
    list.overlay_put(high, property, Value::symbol("not-in-spec"))
        .unwrap();
    list.overlay_put(high, Value::symbol("priority"), Value::fixnum(10))
        .unwrap();
    list.overlay_put(low, property, Value::T).unwrap();
    list.overlay_put(low, Value::symbol("priority"), Value::fixnum(1))
        .unwrap();

    let winner = list.highest_priority_overlay_property_value_at_emacs_byte_pos(
        emacs_byte_pos(4),
        property,
        None,
    );
    assert_eq!(winner, Some(Value::symbol("not-in-spec")));
}

#[test]
fn highest_priority_property_value_is_none_when_no_overlay_carries_it() {
    // `None` is the caller's signal to fall back to the TEXT property. An overlay
    // that does carry the property shadows the text property instead, so the
    // fallback must be keyed on this and nothing else -- consulting the text
    // property first is what hid text a covering overlay declared visible.
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let property = Value::symbol("invisible");
    let unrelated = alloc_overlay(2, 10);
    list.insert_overlay(unrelated);
    list.overlay_put(unrelated, Value::symbol("face"), Value::symbol("bold"))
        .unwrap();
    // An explicit nil value does not count as carrying the property (GNU skips
    // `NILP (tem)` candidates).
    let nil_valued = alloc_overlay(2, 10);
    list.insert_overlay(nil_valued);
    list.overlay_put(nil_valued, property, Value::NIL).unwrap();

    assert_eq!(
        list.highest_priority_overlay_property_value_at_emacs_byte_pos(
            emacs_byte_pos(4),
            property,
            None
        ),
        None
    );
}

#[test]
fn ascending_property_values_order_by_gnu_precedence_including_cons_priority() {
    // The merge policy (`face`) needs GNU `sort_overlays` order, not a bare
    // `priority` integer compare -- which reads a `(PRIMARY . SECONDARY)` priority
    // as 0 and drops the containment rule.
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let property = Value::symbol("face");
    let low = alloc_overlay(2, 7);
    let high = alloc_overlay(4, 7);
    list.insert_overlay(low);
    list.insert_overlay(high);
    list.overlay_put(low, property, Value::symbol("bold"))
        .unwrap();
    list.overlay_put(low, Value::symbol("priority"), Value::fixnum(1))
        .unwrap();
    list.overlay_put(high, property, Value::symbol("italic"))
        .unwrap();
    list.overlay_put(
        high,
        Value::symbol("priority"),
        Value::cons(Value::fixnum(1), Value::fixnum(2)),
    )
    .unwrap();

    // Ascending precedence: the winner merges LAST. Same ordering as
    // `sort_overlay_ids_by_priority_desc`, reversed.
    assert_eq!(
        list.overlay_property_values_ascending_at_emacs_byte_pos(emacs_byte_pos(4), property, None),
        vec![Value::symbol("bold"), Value::symbol("italic")]
    );
}

#[test]
fn property_resolvers_filter_window_specific_overlays() {
    // Both policies honor the overlay `window` property (GNU
    // `overlay_matches_window`), so a per-window highlight cannot leak into
    // another window through either path.
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let property = Value::symbol("face");
    let windowed = alloc_overlay(2, 10);
    list.insert_overlay(windowed);
    list.overlay_put(windowed, property, Value::symbol("hl-line"))
        .unwrap();
    list.overlay_put(windowed, Value::symbol("window"), Value::make_window(7))
        .unwrap();

    for window_id in [None, Some(7)] {
        assert_eq!(
            list.overlay_property_values_ascending_at_emacs_byte_pos(
                emacs_byte_pos(4),
                property,
                window_id
            ),
            vec![Value::symbol("hl-line")],
            "window_id={window_id:?} should see its own overlay"
        );
    }
    assert!(
        list.overlay_property_values_ascending_at_emacs_byte_pos(
            emacs_byte_pos(4),
            property,
            Some(9)
        )
        .is_empty(),
        "another window must not see a window-specific overlay"
    );
    assert_eq!(
        list.highest_priority_overlay_property_value_at_emacs_byte_pos(
            emacs_byte_pos(4),
            property,
            Some(9)
        ),
        None
    );
}

#[test]
fn property_extent_uses_gnu_winner_across_irrelevant_boundaries() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let property = Value::symbol("mouse-face");
    let outer = alloc_overlay(2, 18);
    let nested_low = alloc_overlay(5, 8);
    let nested_high = alloc_overlay(10, 14);
    let nil_overlay = alloc_overlay(6, 12);
    for overlay in [outer, nested_low, nested_high, nil_overlay] {
        list.insert_overlay(overlay);
    }
    list.overlay_put(outer, property, Value::symbol("outer"))
        .unwrap();
    list.overlay_put(
        outer,
        Value::symbol("priority"),
        Value::cons(Value::fixnum(4), Value::fixnum(1)),
    )
    .unwrap();
    list.overlay_put(nested_low, property, Value::symbol("low"))
        .unwrap();
    list.overlay_put(nested_low, Value::symbol("priority"), Value::fixnum(3))
        .unwrap();
    list.overlay_put(nested_high, property, Value::symbol("high"))
        .unwrap();
    list.overlay_put(
        nested_high,
        Value::symbol("priority"),
        Value::cons(Value::fixnum(4), Value::fixnum(2)),
    )
    .unwrap();
    list.overlay_put(nil_overlay, property, Value::NIL).unwrap();
    list.overlay_put(nil_overlay, Value::symbol("priority"), Value::fixnum(99))
        .unwrap();

    let bounds = emacs_byte_range(0, 20);
    let outer_extent = list
        .highest_priority_overlay_property_extent_at_emacs_byte_pos(
            emacs_byte_pos(7),
            property,
            bounds,
            None,
        )
        .unwrap();
    assert_eq!(outer_extent.overlay(), Some(outer));
    assert_eq!(outer_extent.value(), Value::symbol("outer"));
    assert_eq!(outer_extent.range(), emacs_byte_range(2, 10));

    let high_extent = list
        .highest_priority_overlay_property_extent_at_emacs_byte_pos(
            emacs_byte_pos(11),
            property,
            bounds,
            None,
        )
        .unwrap();
    assert_eq!(high_extent.overlay(), Some(nested_high));
    assert_eq!(high_extent.value(), Value::symbol("high"));
    assert_eq!(high_extent.range(), emacs_byte_range(10, 14));
}

#[test]
fn absent_property_extent_stops_at_first_non_nil_overlay() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let property = Value::symbol("mouse-face");
    let nil_overlay = alloc_overlay(2, 6);
    let value_overlay = alloc_overlay(8, 12);
    list.insert_overlay(nil_overlay);
    list.insert_overlay(value_overlay);
    list.overlay_put(nil_overlay, property, Value::NIL).unwrap();
    list.overlay_put(value_overlay, property, Value::symbol("highlight"))
        .unwrap();

    let extent = list
        .highest_priority_overlay_property_extent_at_emacs_byte_pos(
            emacs_byte_pos(4),
            property,
            emacs_byte_range(0, 20),
            None,
        )
        .unwrap();
    assert_eq!(extent.overlay(), None);
    assert_eq!(extent.value(), Value::NIL);
    assert_eq!(extent.range(), emacs_byte_range(0, 8));
}

#[test]
fn windowed_overlay_property_extent_is_restricted_to_its_window() {
    // GNU restricts an overlay carrying a `window` property (e.g. hl-line with a
    // non-sticky flag) to that window: its `mouse-face` must not win in another
    // window. Same rule as the overlay's face / display / invisible.
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let mouse_face = Value::symbol("mouse-face");
    let ov = alloc_overlay(0, 10);
    list.insert_overlay(ov);
    list.overlay_put(ov, mouse_face, Value::symbol("highlight"))
        .unwrap();
    list.overlay_put(ov, Value::symbol("window"), Value::make_window(7))
        .unwrap();

    let bounds = emacs_byte_range(0, 20);
    let at = emacs_byte_pos(5);
    let winner = |window_id| {
        list.highest_priority_overlay_property_extent_at_emacs_byte_pos(
            at, mouse_face, bounds, window_id,
        )
        .unwrap()
        .overlay()
    };
    // The overlay's own window (or no window context) -> it wins.
    assert_eq!(winner(Some(7)), Some(ov));
    assert_eq!(winner(None), Some(ov));
    // A different window -> filtered out, so there is no winning overlay.
    assert_eq!(winner(Some(8)), None);
}

#[test]
fn property_extent_inspects_each_unrelated_overlay_only_once_per_sweep() {
    crate::test_utils::init_test_tracing();
    let mut list = OverlayList::new();
    let mouse_face = Value::symbol("mouse-face");
    let winner = alloc_overlay(0, 4_100);
    list.insert_overlay(winner);
    list.overlay_put(winner, mouse_face, Value::symbol("highlight"))
        .unwrap();
    list.overlay_put(winner, Value::symbol("priority"), Value::fixnum(10))
        .unwrap();

    for index in 0..2_000 {
        let start = 2 + index * 2;
        let unrelated = alloc_overlay(start, start + 1);
        list.insert_overlay(unrelated);
        list.overlay_put(unrelated, Value::symbol("face"), Value::symbol("bold"))
            .unwrap();
    }

    reset_overlay_property_extent_inspection_count();
    let extent = list
        .highest_priority_overlay_property_extent_at_emacs_byte_pos(
            emacs_byte_pos(2_001),
            mouse_face,
            emacs_byte_range(0, 4_100),
            None,
        )
        .unwrap();
    assert_eq!(extent.overlay(), Some(winner));
    assert_eq!(extent.range(), emacs_byte_range(0, 4_100));
    let inspections = overlay_property_extent_inspection_count();
    assert!(
        inspections <= 2_001,
        "one extent query should inspect each candidate at most once; inspected {inspections} overlays"
    );
}
