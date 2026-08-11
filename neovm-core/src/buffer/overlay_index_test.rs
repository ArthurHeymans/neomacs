use super::*;
use crate::buffer::BufferId;
use crate::heap_types::OverlayData;

fn overlay(start: usize, end: usize) -> Value {
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

fn range(start: usize, end: usize) -> EmacsByteRange {
    EmacsByteRange::from_usize(start, end)
}

#[test]
fn same_start_order_survives_avl_rotations_and_removal() {
    crate::test_utils::init_test_tracing();
    let mut index = OverlayIndex::new();
    let first = overlay(2, 8);
    let second = overlay(2, 8);
    let third = overlay(2, 8);
    assert!(index.attach(first, range(2, 8)));
    assert!(index.attach(second, range(2, 8)));
    assert!(index.attach(third, range(2, 8)));

    assert_eq!(
        index.overlays_at(EmacsBytePos::new(4)),
        vec![third, second, first]
    );
    assert_eq!(index.detach(second), Some(range(2, 8)));
    assert_eq!(index.overlays_at(EmacsBytePos::new(4)), vec![third, first]);
}

#[test]
fn detach_two_child_node_preserves_interval_augmentation() {
    crate::test_utils::init_test_tracing();
    let mut index = OverlayIndex::new();
    let middle = overlay(20, 21);
    let left = overlay(10, 100);
    let right = overlay(30, 31);
    let far_right = overlay(40, 41);
    for (overlay, range) in [
        (middle, range(20, 21)),
        (left, range(10, 100)),
        (right, range(30, 31)),
        (far_right, range(40, 41)),
    ] {
        assert!(index.attach(overlay, range));
    }

    assert_eq!(index.detach(middle), Some(range(20, 21)));
    assert_eq!(index.overlays_at(EmacsBytePos::new(35)), vec![left]);
    assert_eq!(
        index.overlays_at(EmacsBytePos::new(40)),
        vec![left, far_right]
    );
}
