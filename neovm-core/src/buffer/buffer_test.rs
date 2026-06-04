use super::*;
use crate::buffer::text::ImplementedBufferTextBackendKind;
use crate::emacs_core::value::ValueKind;
use crate::heap_types::{LispString, OverlayData};

// -----------------------------------------------------------------------
// Helper: create a buffer with some text and correct zv.
// -----------------------------------------------------------------------
fn buf_with_text(text: &str) -> Buffer {
    buf_with_text_backend(text, BufferTextBackendKind::GapBuffer)
}

fn buf_with_text_backend(text: &str, kind: BufferTextBackendKind) -> Buffer {
    let implemented_kind = require_implemented_kind(kind);
    let mut buf =
        Buffer::new_with_text_backend_kind(BufferId(1), Value::string("test"), implemented_kind);
    buf.text = BufferText::from_str_with_backend_kind(text, implemented_kind);
    buf.widen();
    buf
}

fn require_implemented_kind(kind: BufferTextBackendKind) -> ImplementedBufferTextBackendKind {
    kind.implemented()
        .expect("test backend should be implemented")
}

fn implemented_text_backends() -> [BufferTextBackendKind; 3] {
    [
        BufferTextBackendKind::GapBuffer,
        BufferTextBackendKind::PieceTree,
        BufferTextBackendKind::Rope,
    ]
}

fn manager_with_text_backend(kind: BufferTextBackendKind) -> BufferManager {
    let implemented_kind = require_implemented_kind(kind);
    let mut mgr = BufferManager::new();
    mgr.set_default_text_backend_kind(implemented_kind);
    if let Some(id) = mgr.current_buffer_id() {
        mgr.get_mut(id)
            .expect("scratch buffer")
            .text
            .convert_backend_kind(implemented_kind);
    }
    mgr
}

fn buffer_text_property_snapshot(buf: &Buffer) -> Vec<(usize, usize, Vec<(Value, Value)>)> {
    buf.text
        .text_props_intervals_snapshot()
        .into_iter()
        .map(|interval| {
            let properties = interval
                .key_order
                .iter()
                .copied()
                .map(|key| (key, interval.properties[&key]))
                .collect();
            (interval.start, interval.end, properties)
        })
        .collect()
}

/// Test helper: allocate a scratch `MarkerObj` via the tagged heap and
/// register it on `buf` at `pos`. Keeps the old `buf.register_marker(id, pos, ty)`
/// call shape used by the pre-chain tests.
fn register_marker_for_test(
    buf: &mut Buffer,
    marker_id: u64,
    pos: usize,
    insertion_type: InsertionType,
) -> Value {
    let marker_value = Value::make_marker(crate::heap_types::LispMarker {
        buffer: Some(buf.id),
        insertion_type: insertion_type == InsertionType::After,
        marker_id: Some(marker_id),
        bytepos: 0,
        charpos: 0,
        last_position_valid: true,
        next_marker: std::ptr::null_mut(),
    });
    let marker_ptr = marker_value
        .as_veclike_ptr()
        .expect("freshly allocated marker should have a veclike ptr")
        as *mut crate::tagged::header::MarkerObj;
    buf.register_marker(marker_ptr, marker_id, pos, insertion_type);
    marker_value
}

// -----------------------------------------------------------------------
// Buffer creation & naming
// -----------------------------------------------------------------------

#[test]
fn new_buffer_is_empty() {
    crate::test_utils::init_test_tracing();
    let buf = Buffer::new(BufferId(1), Value::string("*scratch*"));
    assert_eq!(buf.name_value(), Value::string("*scratch*"));
    assert_eq!(buf.point(), 0);
    assert_eq!(buf.point_min(), 0);
    assert_eq!(buf.point_max(), 0);
    assert_eq!(buf.buffer_size(), 0);
    assert!(!buf.is_modified());
    assert!(!buf.get_read_only());
    assert!(buf.get_multibyte());
    assert!(buf.file_name_value().is_nil());
    assert!(buf.mark().is_none());
}

#[test]
fn buffer_manager_gc_traces_buffer_and_dead_buffer_names() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let live_id = mgr.create_buffer("live");
    let dead_id = mgr.create_buffer("dead");
    assert!(mgr.kill_buffer(dead_id));

    let live_name = mgr.get(live_id).expect("live buffer").name_value();
    let dead_name = mgr
        .dead_buffer_last_name_value(dead_id)
        .expect("dead buffer name");

    let mut roots = Vec::new();
    mgr.trace_roots(&mut roots);

    assert!(roots.contains(&live_name));
    assert!(roots.contains(&dead_name));
}

#[test]
fn buffer_id_equality() {
    crate::test_utils::init_test_tracing();
    let a = BufferId(1);
    let b = BufferId(1);
    let c = BufferId(2);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn create_indirect_buffer_shares_root_text_and_updates_siblings() {
    crate::test_utils::init_test_tracing();
    for kind in implemented_text_backends() {
        let mut mgr = manager_with_text_backend(kind);
        let base_id = mgr.current_buffer_id().expect("scratch buffer");

        let _ = mgr.insert_into_buffer(base_id, "abcd");
        let indirect_id = mgr
            .create_indirect_buffer(base_id, "*indirect*", false)
            .expect("indirect buffer");

        let base = mgr.get(base_id).expect("base buffer");
        let indirect = mgr.get(indirect_id).expect("indirect buffer");
        assert_eq!(base.text.backend_kind(), kind);
        assert_eq!(indirect.text.backend_kind(), kind);
        assert_eq!(indirect.base_buffer, Some(base_id));
        assert!(base.text.shares_storage_with(&indirect.text));
        assert_eq!(indirect.buffer_string(), "abcd");

        let _ = mgr.goto_buffer_byte(base_id, 0);
        let _ = mgr.insert_into_buffer(base_id, "zz");
        assert_eq!(mgr.get(base_id).unwrap().buffer_string(), "zzabcd");
        assert_eq!(mgr.get(indirect_id).unwrap().buffer_string(), "zzabcd");

        let _ = mgr.delete_buffer_region(indirect_id, 2, 4);
        assert_eq!(mgr.get(base_id).unwrap().buffer_string(), "zzcd");
        assert_eq!(mgr.get(indirect_id).unwrap().buffer_string(), "zzcd");
    }
}

#[test]
fn create_indirect_buffer_flattens_double_indirection() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let base_id = mgr.current_buffer_id().expect("scratch buffer");
    let first_id = mgr
        .create_indirect_buffer(base_id, "*indirect-one*", false)
        .expect("first indirect");
    let second_id = mgr
        .create_indirect_buffer(first_id, "*indirect-two*", false)
        .expect("second indirect");

    assert_eq!(mgr.get(first_id).unwrap().base_buffer, Some(base_id));
    assert_eq!(mgr.get(second_id).unwrap().base_buffer, Some(base_id));
    assert!(
        mgr.get(base_id)
            .unwrap()
            .text
            .shares_storage_with(&mgr.get(second_id).unwrap().text)
    );
}

#[test]
fn indirect_buffers_keep_undo_state_in_sync() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let base_id = mgr.current_buffer_id().expect("scratch buffer");
    let indirect_id = mgr
        .create_indirect_buffer(base_id, "*indirect-undo*", false)
        .expect("indirect buffer");

    let _ = mgr.insert_into_buffer(base_id, "abc");
    {
        let undo_val = mgr
            .get(indirect_id)
            .and_then(|buf| buf.buffer_local_value("buffer-undo-list"));
        assert!(
            undo_val.is_some() && !undo_val.unwrap().is_nil(),
            "indirect buffer should observe the base buffer's undo history"
        );
    }

    let result = mgr.undo_buffer(indirect_id, 1).expect("undo result");
    assert!(result.applied_any);
    assert_eq!(mgr.get(base_id).unwrap().buffer_string(), "");
    assert_eq!(mgr.get(indirect_id).unwrap().buffer_string(), "");
}

#[test]
fn from_dump_restores_indirect_buffer_shared_text_state() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let base_id = mgr.current_buffer_id().expect("scratch buffer");
    let _ = mgr.insert_into_buffer(base_id, "abcdef");
    let indirect_id = mgr
        .create_indirect_buffer(base_id, "*indirect-restored*", false)
        .expect("indirect buffer");
    let _ =
        mgr.put_buffer_text_property(base_id, 1, 4, Value::symbol("face"), Value::symbol("bold"));
    let _ = mgr.insert_into_buffer(base_id, "z");

    let mut dumped = mgr.dump_buffers().clone();
    let independent_indirect = dumped.get(&indirect_id).expect("indirect buffer").clone();
    let indirect = dumped.get_mut(&indirect_id).expect("indirect buffer");
    indirect.text = BufferText::from_dump(
        independent_indirect.text.dump_text(),
        independent_indirect.get_multibyte(),
    );
    indirect
        .text
        .text_props_replace(independent_indirect.text.text_props_snapshot());
    indirect.undo_state =
        SharedUndoState::from_parts(independent_indirect.get_undo_list(), false, false);

    let restored = BufferManager::from_dump(
        dumped,
        mgr.dump_current(),
        mgr.dump_next_id(),
        mgr.dump_next_marker_id(),
        None,
        None,
        require_implemented_kind(BufferTextBackendKind::GapBuffer),
    );

    let base = restored.get(base_id).expect("base buffer");
    let indirect = restored.get(indirect_id).expect("indirect buffer");
    assert!(base.text.shares_storage_with(&indirect.text));
    assert!(base.undo_state.shares_with(&indirect.undo_state));
    assert_eq!(
        indirect
            .text
            .text_props_get_property(1, Value::symbol("face")),
        Some(Value::symbol("bold"))
    );
}

#[test]
fn from_dump_preserves_dumped_buffer_order() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let scratch = mgr.current_buffer_id().expect("scratch buffer");
    let one = mgr.create_buffer("one");
    let two = mgr.create_buffer("two");
    let three = mgr.create_buffer("three");

    let restored = BufferManager::from_dump(
        mgr.dump_buffers().clone(),
        Some(three),
        mgr.dump_next_id(),
        mgr.dump_next_marker_id(),
        Some(&[two, scratch, three, one]),
        None,
        require_implemented_kind(BufferTextBackendKind::GapBuffer),
    );

    assert_eq!(restored.buffer_list(), vec![two, scratch, three, one]);
}

#[test]
fn indirect_buffers_preserve_narrowing_across_shared_edits() {
    crate::test_utils::init_test_tracing();
    for kind in implemented_text_backends() {
        let mut mgr = manager_with_text_backend(kind);
        let base_id = mgr.current_buffer_id().expect("scratch buffer");
        let _ = mgr.insert_into_buffer(base_id, "abcdef");
        let indirect_id = mgr
            .create_indirect_buffer(base_id, "*indirect-narrow*", false)
            .expect("indirect buffer");

        let _ = mgr.narrow_buffer_to_region(indirect_id, 2, 6);
        let _ = mgr.goto_buffer_byte(indirect_id, 4);

        let _ = mgr.goto_buffer_byte(base_id, 0);
        let _ = mgr.insert_into_buffer(base_id, "zz");

        let indirect = mgr.get(indirect_id).expect("indirect buffer");
        assert_eq!(indirect.text.backend_kind(), kind);
        assert_eq!(indirect.point_min(), 4);
        assert_eq!(indirect.point_max(), 8);
        assert_eq!(indirect.point(), 6);
        assert_eq!(indirect.buffer_string(), "cdef");

        let _ = mgr.delete_buffer_region(base_id, 0, 2);

        let indirect = mgr.get(indirect_id).expect("indirect buffer");
        assert_eq!(indirect.text.backend_kind(), kind);
        assert_eq!(indirect.point_min(), 2);
        assert_eq!(indirect.point_max(), 6);
        assert_eq!(indirect.point(), 4);
        assert_eq!(indirect.buffer_string(), "cdef");
    }
}

#[test]
fn cloned_indirect_buffers_do_not_share_base_state_markers() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let base_id = mgr.current_buffer_id().expect("scratch buffer");
    let _ = mgr.insert_into_buffer(base_id, "aaa\nbbb\nccc\n");

    // The first indirect forces the base buffer to allocate the hidden
    // PT/BEGV/ZV markers GNU uses to preserve per-buffer state.
    let _first = mgr
        .create_indirect_buffer(base_id, "*indirect-state-first*", true)
        .expect("first indirect buffer");
    let second = mgr
        .create_indirect_buffer(base_id, "*indirect-state-second*", true)
        .expect("second indirect buffer");

    mgr.set_current(second);
    let _ = mgr.narrow_buffer_to_region(second, 4, 7);
    mgr.set_current(base_id);

    let base = mgr.get(base_id).expect("base buffer");
    assert_eq!(base.point_min(), 0);
    assert_eq!(base.point_max(), base.total_bytes());

    let second = mgr.get(second).expect("second indirect buffer");
    assert_eq!(second.point_min(), 4);
    assert_eq!(second.point_max(), 7);
}

#[test]
fn cloned_indirect_buffers_do_not_share_base_mark_marker() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let base_id = mgr.current_buffer_id().expect("scratch buffer");
    let _ = mgr.insert_into_buffer(base_id, "abcdef");
    mgr.get_mut(base_id).expect("base buffer").set_mark_byte(2);

    let indirect = mgr
        .create_indirect_buffer(base_id, "*indirect-mark-clone*", true)
        .expect("indirect buffer");
    mgr.get_mut(indirect)
        .expect("indirect buffer")
        .set_mark_byte(5);

    assert_eq!(mgr.get(base_id).expect("base buffer").mark_byte(), Some(2));
    assert_eq!(
        mgr.get(indirect).expect("indirect buffer").mark_byte(),
        Some(5)
    );
}

#[test]
fn indirect_buffer_overlays_track_shared_edits() {
    crate::test_utils::init_test_tracing();
    for kind in implemented_text_backends() {
        let mut mgr = manager_with_text_backend(kind);
        let base_id = mgr.current_buffer_id().expect("scratch buffer");
        let _ = mgr.insert_into_buffer(base_id, "abcdef");
        let indirect_id = mgr
            .create_indirect_buffer(base_id, "*indirect-overlays*", false)
            .expect("indirect buffer");

        let overlay = Value::make_overlay(OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(indirect_id),
            start: 2,
            end: 4,
            front_advance: false,
            rear_advance: false,
        });
        mgr.get_mut(indirect_id)
            .expect("indirect buffer")
            .overlays
            .insert_overlay(overlay);

        let _ = mgr.goto_buffer_byte(base_id, 0);
        let _ = mgr.insert_into_buffer(base_id, "zz");
        let indirect = mgr.get(indirect_id).expect("indirect buffer");
        assert_eq!(indirect.text.backend_kind(), kind);
        assert_eq!(indirect.overlays.overlay_start(overlay), Some(4));
        assert_eq!(indirect.overlays.overlay_end(overlay), Some(6));

        let _ = mgr.goto_buffer_byte(base_id, 4);
        let _ = mgr.insert_into_buffer_before_markers(base_id, "yy");
        let indirect = mgr.get(indirect_id).expect("indirect buffer");
        assert_eq!(indirect.text.backend_kind(), kind);
        assert_eq!(indirect.overlays.overlay_start(overlay), Some(6));
        assert_eq!(indirect.overlays.overlay_end(overlay), Some(8));

        let _ = mgr.delete_buffer_region(base_id, 0, 2);
        let indirect = mgr.get(indirect_id).expect("indirect buffer");
        assert_eq!(indirect.text.backend_kind(), kind);
        assert_eq!(indirect.overlays.overlay_start(overlay), Some(4));
        assert_eq!(indirect.overlays.overlay_end(overlay), Some(6));
    }
}

#[derive(Debug, PartialEq, Eq)]
struct BufferSwapPayloadSnapshot {
    backend_kind: BufferTextBackendKind,
    buffer_string: String,
    point_byte: usize,
    point_min_byte: usize,
    point_max_byte: usize,
    mark_byte: Option<usize>,
    undo_list: Value,
    text_properties: Vec<(usize, usize, Vec<(Value, Value)>)>,
}

fn buffer_swap_payload_snapshot(buf: &Buffer) -> BufferSwapPayloadSnapshot {
    BufferSwapPayloadSnapshot {
        backend_kind: buf.text.backend_kind(),
        buffer_string: buf.buffer_string(),
        point_byte: buf.point_byte(),
        point_min_byte: buf.point_min_byte(),
        point_max_byte: buf.point_max_byte(),
        mark_byte: buf.mark_byte(),
        undo_list: buf.get_undo_list(),
        text_properties: buffer_text_property_snapshot(buf),
    }
}

fn buffer_byte_pos_for_char(mgr: &BufferManager, id: BufferId, char_pos: usize) -> usize {
    mgr.get(id)
        .expect("buffer")
        .text
        .buf_charpos_to_bytepos(char_pos)
}

#[test]
fn implemented_text_backends_match_buffer_swap_text_side_effects() {
    crate::test_utils::init_test_tracing();
    let face = Value::symbol("face");
    let bold = Value::symbol("bold");
    let italic = Value::symbol("italic");

    for left_kind in implemented_text_backends() {
        for right_kind in implemented_text_backends() {
            let mut mgr = BufferManager::new();
            let left_id = mgr.current_buffer_id().expect("scratch buffer");
            let right_id = mgr.create_buffer("*swap-right*");
            mgr.get(left_id)
                .expect("left buffer")
                .text
                .try_convert_backend_kind(left_kind)
                .expect("left backend should convert");
            mgr.get(right_id)
                .expect("right buffer")
                .text
                .try_convert_backend_kind(right_kind)
                .expect("right backend should convert");

            mgr.insert_into_buffer(left_id, "aébc日本")
                .expect("left insert");
            mgr.insert_into_buffer(right_id, "uvwxyzλ")
                .expect("right insert");

            let left_prop_start = buffer_byte_pos_for_char(&mgr, left_id, 1);
            let left_prop_end = buffer_byte_pos_for_char(&mgr, left_id, 5);
            mgr.put_buffer_text_property(left_id, left_prop_start, left_prop_end, face, bold)
                .expect("left property");
            let right_prop_start = buffer_byte_pos_for_char(&mgr, right_id, 2);
            let right_prop_end = buffer_byte_pos_for_char(&mgr, right_id, 7);
            mgr.put_buffer_text_property(right_id, right_prop_start, right_prop_end, face, italic)
                .expect("right property");

            let left_marker_pos = buffer_byte_pos_for_char(&mgr, left_id, 3);
            let left_marker = register_marker_for_test(
                mgr.get_mut(left_id).expect("left buffer"),
                501,
                left_marker_pos,
                InsertionType::After,
            );
            let right_marker_pos = buffer_byte_pos_for_char(&mgr, right_id, 4);
            let right_marker = register_marker_for_test(
                mgr.get_mut(right_id).expect("right buffer"),
                502,
                right_marker_pos,
                InsertionType::Before,
            );

            let left_overlay_start = buffer_byte_pos_for_char(&mgr, left_id, 1);
            let left_overlay_end = buffer_byte_pos_for_char(&mgr, left_id, 4);
            let left_overlay = Value::make_overlay(OverlayData {
                serial: 0,
                plist: Value::NIL,
                buffer: Some(left_id),
                start: left_overlay_start,
                end: left_overlay_end,
                front_advance: false,
                rear_advance: true,
            });
            mgr.get_mut(left_id)
                .expect("left buffer")
                .overlays
                .insert_overlay(left_overlay);

            let right_overlay_start = buffer_byte_pos_for_char(&mgr, right_id, 2);
            let right_overlay_end = buffer_byte_pos_for_char(&mgr, right_id, 6);
            let right_overlay = Value::make_overlay(OverlayData {
                serial: 0,
                plist: Value::NIL,
                buffer: Some(right_id),
                start: right_overlay_start,
                end: right_overlay_end,
                front_advance: true,
                rear_advance: false,
            });
            mgr.get_mut(right_id)
                .expect("right buffer")
                .overlays
                .insert_overlay(right_overlay);

            let left_point = buffer_byte_pos_for_char(&mgr, left_id, 3);
            let left_mark = buffer_byte_pos_for_char(&mgr, left_id, 4);
            let left_narrow_start = buffer_byte_pos_for_char(&mgr, left_id, 1);
            let left_narrow_end = buffer_byte_pos_for_char(&mgr, left_id, 5);
            mgr.narrow_buffer_to_region(left_id, left_narrow_start, left_narrow_end)
                .expect("left narrow");
            mgr.goto_buffer_byte(left_id, left_point)
                .expect("left point");
            mgr.set_buffer_mark(left_id, left_mark).expect("left mark");

            let right_point = buffer_byte_pos_for_char(&mgr, right_id, 5);
            let right_mark = buffer_byte_pos_for_char(&mgr, right_id, 1);
            let right_narrow_start = buffer_byte_pos_for_char(&mgr, right_id, 2);
            let right_narrow_end = buffer_byte_pos_for_char(&mgr, right_id, 7);
            mgr.narrow_buffer_to_region(right_id, right_narrow_start, right_narrow_end)
                .expect("right narrow");
            mgr.goto_buffer_byte(right_id, right_point)
                .expect("right point");
            mgr.set_buffer_mark(right_id, right_mark)
                .expect("right mark");

            mgr.get_mut(left_id)
                .expect("left buffer")
                .set_undo_list(Value::symbol("left-undo"));
            mgr.get_mut(right_id)
                .expect("right buffer")
                .set_undo_list(Value::symbol("right-undo"));

            let left_before = buffer_swap_payload_snapshot(mgr.get(left_id).expect("left buffer"));
            let right_before =
                buffer_swap_payload_snapshot(mgr.get(right_id).expect("right buffer"));

            mgr.swap_buffer_text(left_id, right_id)
                .expect("swap should succeed");

            let left_after = mgr.get(left_id).expect("left buffer");
            let right_after = mgr.get(right_id).expect("right buffer");
            assert_eq!(
                buffer_swap_payload_snapshot(left_after),
                right_before,
                "left buffer after swap should have right payload for {left_kind:?}->{right_kind:?}"
            );
            assert_eq!(
                buffer_swap_payload_snapshot(right_after),
                left_before,
                "right buffer after swap should have left payload for {left_kind:?}->{right_kind:?}"
            );

            assert_eq!(left_after.overlays.len(), 1);
            assert_eq!(right_after.overlays.len(), 1);
            assert_eq!(
                right_overlay.as_overlay_data().unwrap().buffer,
                Some(left_id)
            );
            assert_eq!(
                left_overlay.as_overlay_data().unwrap().buffer,
                Some(right_id)
            );
            assert_eq!(
                left_after.overlays.overlay_start(right_overlay),
                Some(right_overlay_start)
            );
            assert_eq!(
                right_after.overlays.overlay_start(left_overlay),
                Some(left_overlay_start)
            );

            assert_eq!(right_marker.as_marker_data().unwrap().buffer, Some(left_id));
            assert_eq!(left_marker.as_marker_data().unwrap().buffer, Some(right_id));
            assert_eq!(
                left_after
                    .text
                    .marker_chain_lookup(502)
                    .map(|(b, c, _)| (b, c)),
                Some((
                    right_marker_pos,
                    left_after.text.buf_bytepos_to_charpos(right_marker_pos)
                ))
            );
            assert_eq!(
                right_after
                    .text
                    .marker_chain_lookup(501)
                    .map(|(b, c, _)| (b, c)),
                Some((
                    left_marker_pos,
                    right_after.text.buf_bytepos_to_charpos(left_marker_pos)
                ))
            );
        }
    }
}

// -----------------------------------------------------------------------
// Point movement
// -----------------------------------------------------------------------

#[test]
fn goto_char_clamps_to_accessible_region() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("hello");
    buf.goto_char(3);
    assert_eq!(buf.point(), 3);

    // Past end — clamped to zv.
    buf.goto_char(999);
    assert_eq!(buf.point(), buf.point_max());

    // Before start — clamped to begv.
    buf.goto_char(0);
    buf.narrow_to_byte_region(2, buf.point_max_byte());
    buf.goto_char(0);
    assert_eq!(buf.point(), 2);
}

#[test]
fn point_char_converts_byte_to_char_pos() {
    crate::test_utils::init_test_tracing();
    // "cafe\u{0301}" — 'e' + combining acute = 5 bytes, 5 chars in UTF-8
    let mut buf = buf_with_text("hello");
    buf.goto_char(3);
    assert_eq!(buf.point_char(), 3);
}

#[test]
fn gnu_style_buffer_fields_track_char_and_byte_positions() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("éz");
    assert_eq!(buf.begv, 0);
    assert_eq!(buf.begv_byte, 0);
    assert_eq!(buf.zv, 2);
    assert_eq!(buf.zv_byte, 3);

    buf.goto_byte('é'.len_utf8());
    assert_eq!(buf.pt, 1);
    assert_eq!(buf.pt_byte, 2);

    buf.set_mark_byte(3);
    assert_eq!(buf.mark(), Some(3));
    assert_eq!(buf.mark_byte(), Some(3));
    assert_eq!(buf.mark_char(), Some(2));
}

#[test]
fn byte_position_aliases_match_legacy_buffer_apis() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("hello world");
    buf.narrow_to_byte_region(2, 9);
    buf.goto_byte(7);
    buf.set_mark_byte(4);

    assert_eq!(buf.point_byte(), buf.point());
    assert_eq!(buf.point_min_byte(), buf.point_min());
    assert_eq!(buf.point_max_byte(), buf.point_max());
    assert_eq!(buf.mark_byte(), buf.mark());
}

#[test]
fn accessible_region_snapshot_restores_saved_bounds() {
    crate::test_utils::init_test_tracing();
    for kind in implemented_text_backends() {
        let mut buf = buf_with_text_backend("aébc", kind);
        buf.narrow_to_byte_region(1, 4);
        buf.goto_byte(3);

        let saved = buf.accessible_region_snapshot();
        buf.widen();
        buf.goto_byte(buf.total_bytes());
        buf.insert("Z");
        buf.restore_accessible_region(saved);

        assert_eq!(buf.point_min_byte(), 1);
        assert_eq!(buf.point_max_byte(), 4);
        assert_eq!(buf.point_min_char(), 1);
        assert_eq!(buf.point_max_char(), 3);
        assert_eq!(buf.point_byte(), 4);
    }
}

#[test]
fn accessible_region_snapshot_can_restore_end_to_current_full_buffer() {
    crate::test_utils::init_test_tracing();
    for kind in implemented_text_backends() {
        let mut buf = buf_with_text_backend("aébc", kind);
        buf.narrow_to_byte_region(1, buf.total_bytes());

        let saved = buf.accessible_region_snapshot();
        buf.widen();
        buf.goto_byte(buf.total_bytes());
        buf.insert("Z");
        buf.restore_accessible_region_with_current_full_end(saved);

        assert_eq!(buf.point_min_byte(), 1);
        assert_eq!(buf.point_max_byte(), buf.total_bytes());
        assert_eq!(buf.point_min_char(), 1);
        assert_eq!(buf.point_max_char(), buf.total_chars());
        assert_eq!(buf.point_byte(), buf.total_bytes());
    }
}

#[test]
fn cached_char_positions_track_multibyte_edits_and_narrowing() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("ééz");
    assert_eq!(buf.point_max_char(), 3);

    buf.goto_byte('é'.len_utf8());
    assert_eq!(buf.point_char(), 1);

    buf.insert("ß");
    assert_eq!(buf.point_byte(), 4);
    assert_eq!(buf.point_char(), 2);
    assert_eq!(buf.point_max_char(), 4);

    buf.narrow_to_byte_region('é'.len_utf8(), buf.point_max_byte());
    assert_eq!(buf.point_min_char(), 1);
    assert_eq!(buf.point_max_char(), 4);

    buf.delete_region(2, 4);
    assert_eq!(buf.point_byte(), 2);
    assert_eq!(buf.point_char(), 1);
    assert_eq!(buf.point_max_char(), 3);
    assert_eq!(buf.buffer_string(), "éz");
}

#[test]
fn char_position_conversions_clamp_to_buffer_and_accessible_bounds() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("ééz");
    assert_eq!(buf.total_chars(), 3);
    assert_eq!(
        buf.char_pos_to_emacs_byte_pos_clamped(CharPos0::new(99))
            .get(),
        "ééz".len()
    );
    assert_eq!(buf.lisp_pos_to_emacs_byte_pos(99).get(), "ééz".len());

    buf.narrow_to_byte_region('é'.len_utf8(), "ééz".len());
    assert_eq!(buf.point_min_char(), 1);
    assert_eq!(buf.point_max_char(), 3);
    assert_eq!(
        buf.lisp_pos_to_accessible_emacs_byte_pos(1).get(),
        'é'.len_utf8()
    );
    assert_eq!(
        buf.lisp_pos_to_accessible_emacs_byte_pos(99).get(),
        "ééz".len()
    );
}

// -----------------------------------------------------------------------
// Insertion
// -----------------------------------------------------------------------

#[test]
fn insert_at_point_advances_point() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("test"));
    // zv starts at 0 for an empty buffer; insert should extend it.
    buf.insert("hello");
    assert_eq!(buf.point(), 5);
    assert_eq!(buf.buffer_string(), "hello");
    assert_eq!(buf.buffer_size(), 5);
    assert!(buf.is_modified());
}

#[test]
fn insert_in_middle() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("helo");
    buf.goto_char(3);
    buf.insert("l");
    assert_eq!(buf.buffer_string(), "hello");
    assert_eq!(buf.point(), 4);
}

#[test]
fn insert_adjusts_mark() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("ab");
    buf.set_mark(1);
    buf.goto_char(0);
    buf.insert("X");
    // Mark was at 1, insert at 0 pushes it to 2.
    assert_eq!(buf.mark(), Some(2));
    assert_eq!(buf.mark_char(), Some(2));
}

#[test]
fn insert_empty_string_is_noop() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("hello");
    buf.goto_char(2);
    buf.insert("");
    assert_eq!(buf.buffer_string(), "hello");
    assert!(!buf.is_modified()); // still unmodified from initial state
}

// -----------------------------------------------------------------------
// Deletion
// -----------------------------------------------------------------------

#[test]
fn delete_region_basic() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("hello world");
    buf.goto_char(11); // at end
    buf.delete_region(5, 11);
    assert_eq!(buf.buffer_string(), "hello");
    assert_eq!(buf.point(), 5); // was past deleted range
}

#[test]
fn delete_region_adjusts_point_inside() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("abcdef");
    buf.goto_char(3); // in middle of deleted range
    buf.delete_region(1, 5);
    assert_eq!(buf.point(), 1); // collapsed to start of deletion
    assert_eq!(buf.buffer_string(), "af");
}

#[test]
fn delete_region_adjusts_point_at_end_boundary() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("abcdef");
    buf.goto_char(5);
    buf.delete_region(1, 5);
    assert_eq!(buf.point(), 1);
    assert_eq!(buf.point_char(), 1);
}

#[test]
fn delete_region_adjusts_mark() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("abcdef");
    buf.set_mark(4);
    buf.delete_region(1, 3);
    // mark was at 4, past deleted range end (3), so shifts by 2
    assert_eq!(buf.mark(), Some(2));
    assert_eq!(buf.mark_char(), Some(2));
}

#[test]
fn delete_region_moves_marker_at_end_to_start() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("0123456789ABCDEF");
    register_marker_for_test(&mut buf, 1, 12, InsertionType::Before);
    buf.delete_region(5, 12);
    let (byte_pos, char_pos, _ins) = buf.text.marker_chain_lookup(1).expect("marker");
    assert_eq!(byte_pos, 5);
    assert_eq!(char_pos, 5);
}

#[test]
fn mark_char_tracks_multibyte_edits() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("ééz");
    buf.set_mark_byte('é'.len_utf8());
    buf.goto_byte('é'.len_utf8());
    buf.insert("ß");
    assert_eq!(buf.mark(), Some(2));
    assert_eq!(buf.mark_char(), Some(1));

    buf.delete_region(0, 2);
    assert_eq!(buf.mark(), Some(0));
    assert_eq!(buf.mark_char(), Some(0));
}

#[test]
fn delete_region_adjusts_zv() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("abcdef");
    assert_eq!(buf.zv, 6);
    buf.delete_region(2, 4);
    assert_eq!(buf.zv, 4);
}

#[test]
fn delete_empty_range_is_noop() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("hello");
    buf.delete_region(2, 2);
    assert_eq!(buf.buffer_string(), "hello");
}

// -----------------------------------------------------------------------
// Substring / buffer_string
// -----------------------------------------------------------------------

#[test]
fn buffer_substring_range() {
    crate::test_utils::init_test_tracing();
    let buf = buf_with_text("hello world");
    assert_eq!(
        buf.buffer_substring_range(EmacsByteRange::from_usize(6, 11)),
        "world"
    );
}

#[test]
fn buffer_string_returns_accessible() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("hello world");
    buf.narrow_to_region(6, 11);
    assert_eq!(buf.buffer_string(), "world");
}

// -----------------------------------------------------------------------
// char_after / char_before
// -----------------------------------------------------------------------

#[test]
fn char_after_basic() {
    crate::test_utils::init_test_tracing();
    let buf = buf_with_text("hello");
    assert_eq!(buf.char_after(0), Some('h'));
    assert_eq!(buf.char_after(4), Some('o'));
    assert_eq!(buf.char_after(5), None);
}

#[test]
fn char_before_basic() {
    crate::test_utils::init_test_tracing();
    let buf = buf_with_text("hello");
    assert_eq!(buf.char_before(0), None);
    assert_eq!(buf.char_before(1), Some('h'));
    assert_eq!(buf.char_before(5), Some('o'));
}

#[test]
fn char_after_multibyte() {
    crate::test_utils::init_test_tracing();
    // Each Chinese character is 3 bytes in UTF-8.
    let buf = buf_with_text("\u{4f60}\u{597d}"); // "nihao" in Chinese
    assert_eq!(buf.char_after(0), Some('\u{4f60}'));
    assert_eq!(buf.char_after(3), Some('\u{597d}'));
}

#[test]
fn char_before_multibyte() {
    crate::test_utils::init_test_tracing();
    let buf = buf_with_text("\u{4f60}\u{597d}");
    assert_eq!(buf.char_before(3), Some('\u{4f60}'));
    assert_eq!(buf.char_before(6), Some('\u{597d}'));
}

#[test]
fn char_width_helpers_use_typed_emacs_and_storage_positions() {
    crate::test_utils::init_test_tracing();
    for kind in implemented_text_backends() {
        let buf = buf_with_text_backend("a\u{4f60}b", kind);

        assert_eq!(buf.char_after_emacs_len(0), Some(1));
        assert_eq!(buf.char_after_emacs_len(1), Some('\u{4f60}'.len_utf8()));
        assert_eq!(buf.char_after_emacs_len(4), Some(1));
        assert_eq!(buf.char_after_emacs_len(5), None);

        assert_eq!(buf.char_before_emacs_len(1), Some(1));
        assert_eq!(buf.char_before_emacs_len(4), Some('\u{4f60}'.len_utf8()));
        assert_eq!(buf.char_before_emacs_len(5), Some(1));
        assert_eq!(buf.char_before_emacs_len(0), None);

        assert_eq!(buf.char_after_storage_len(0), Some(1));
        assert_eq!(buf.char_after_storage_len(1), Some('\u{4f60}'.len_utf8()));
        assert_eq!(buf.char_before_storage_len(4), Some('\u{4f60}'.len_utf8()));
        assert_eq!(buf.char_before_storage_len(5), Some(1));
    }
}

// -----------------------------------------------------------------------
// Narrowing
// -----------------------------------------------------------------------

#[test]
fn narrow_and_widen() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("hello world");
    buf.goto_char(8);
    buf.narrow_to_region(6, 11);
    assert_eq!(buf.point_min(), 6);
    assert_eq!(buf.point_max(), 11);
    assert_eq!(buf.buffer_size(), 5);
    assert_eq!(buf.buffer_string(), "world");
    // Point was 8 — still within [6, 11].
    assert_eq!(buf.point(), 8);

    buf.widen();
    assert_eq!(buf.point_min(), 0);
    assert_eq!(buf.point_max(), 11);
}

#[test]
fn narrow_clamps_point() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("hello world");
    buf.goto_char(2);
    buf.narrow_to_region(5, 11);
    // Point 2 < begv 5 => clamped to 5.
    assert_eq!(buf.point(), 5);
}

// -----------------------------------------------------------------------
// Markers
// -----------------------------------------------------------------------

#[test]
fn marker_tracks_insertion_after() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("ab");
    register_marker_for_test(&mut buf, 1, 1, InsertionType::After);
    buf.goto_char(1);
    buf.insert("XY");
    // Marker was at 1 with After => advances to 3.
    let (byte_pos, char_pos, _ins) = buf.text.marker_chain_lookup(1).expect("marker");
    assert_eq!(byte_pos, 3);
    assert_eq!(char_pos, 3);
}

#[test]
fn marker_stays_on_insertion_before() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("ab");
    register_marker_for_test(&mut buf, 1, 1, InsertionType::Before);
    buf.goto_char(1);
    buf.insert("XY");
    // Marker was at 1 with Before => stays at 1.
    let (byte_pos, char_pos, _ins) = buf.text.marker_chain_lookup(1).expect("marker");
    assert_eq!(byte_pos, 1);
    assert_eq!(char_pos, 1);
}

#[test]
fn marker_adjusts_on_deletion() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("abcdef");
    register_marker_for_test(&mut buf, 1, 4, InsertionType::After);
    buf.delete_region(1, 3);
    // Marker was at 4 (past deleted range [1,3)), shifts by 2 => 2.
    let (byte_pos, char_pos, _ins) = buf.text.marker_chain_lookup(1).expect("marker");
    assert_eq!(byte_pos, 2);
    assert_eq!(char_pos, 2);
}

#[test]
fn marker_inside_deleted_range_collapses() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("abcdef");
    register_marker_for_test(&mut buf, 1, 2, InsertionType::After);
    buf.delete_region(1, 5);
    // Marker at 2 inside [1,5) => collapses to 1.
    let (byte_pos, char_pos, _ins) = buf.text.marker_chain_lookup(1).expect("marker");
    assert_eq!(byte_pos, 1);
    assert_eq!(char_pos, 1);
}

#[test]
fn marker_char_pos_tracks_multibyte_edits() {
    crate::test_utils::init_test_tracing();
    let mut buf = buf_with_text("ééz");
    register_marker_for_test(&mut buf, 1, 'é'.len_utf8(), InsertionType::After);
    buf.goto_byte('é'.len_utf8());
    buf.insert("ß");
    let (byte_pos, char_pos, _ins) = buf.text.marker_chain_lookup(1).expect("marker");
    assert_eq!(byte_pos, 4);
    assert_eq!(char_pos, 2);

    buf.delete_region(2, 4);
    let (byte_pos, char_pos, _ins) = buf.text.marker_chain_lookup(1).expect("marker");
    assert_eq!(byte_pos, 2);
    assert_eq!(char_pos, 1);
}

#[derive(Debug, PartialEq, Eq)]
struct BackendEditSnapshot {
    buffer_string: String,
    point_byte: usize,
    point_char: usize,
    mark_byte: Option<usize>,
    mark_char: Option<usize>,
    marker_position: Option<(usize, usize)>,
    text_properties: Vec<(usize, usize, Vec<(Value, Value)>)>,
}

fn run_backend_edit_script(kind: BufferTextBackendKind) -> BackendEditSnapshot {
    let mut buf = buf_with_text_backend("éaßbc", kind);
    assert_eq!(buf.text.backend_kind(), kind);

    let face = Value::symbol("face");
    let bold = Value::symbol("bold");
    assert!(buf.text.text_props_put_property(2, 6, face, bold));
    register_marker_for_test(&mut buf, 42, 3, InsertionType::After);
    buf.set_mark_byte(2);

    buf.goto_byte(3);
    buf.insert("日本");

    let delete_start = buf.text.buf_charpos_to_bytepos(2);
    let delete_end = buf.text.buf_charpos_to_bytepos(4);
    buf.delete_region(delete_start, delete_end);

    let replace_start = buf.text.buf_charpos_to_bytepos(1);
    let replace_end = buf.text.buf_charpos_to_bytepos(3);
    let replacement = LispString::from_utf8("Ωx");
    buf.replace_region_lisp_string(replace_start, replace_end, &replacement);

    let marker_position = buf
        .text
        .marker_chain_lookup(42)
        .map(|(byte_pos, char_pos, _)| (byte_pos, char_pos));

    BackendEditSnapshot {
        buffer_string: buf.buffer_string(),
        point_byte: buf.point_byte(),
        point_char: buf.point_char(),
        mark_byte: buf.mark(),
        mark_char: buf.mark_char(),
        marker_position,
        text_properties: buffer_text_property_snapshot(&buf),
    }
}

#[test]
fn implemented_text_backends_match_edit_marker_and_property_side_effects() {
    crate::test_utils::init_test_tracing();
    let baseline = run_backend_edit_script(BufferTextBackendKind::GapBuffer);
    for kind in implemented_text_backends() {
        assert_eq!(
            run_backend_edit_script(kind),
            baseline,
            "{kind:?} edit side effects diverged from gap buffer"
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum UndoEntrySnapshot {
    Boundary,
    Point(i64),
    Insert {
        beg1: i64,
        end1: i64,
    },
    Delete {
        text: String,
        pos1: i64,
    },
    FirstChange(i64),
    PropertyChange {
        prop: String,
        old_value: String,
        beg1: i64,
        end1: i64,
    },
    MarkerAdjustment {
        marker_id: Option<u64>,
        adjustment: i64,
    },
    Other(String),
    ImproperTail(String),
}

fn undo_value_label(value: Value) -> String {
    if value.is_nil() {
        return "nil".to_owned();
    }
    if value.is_t() {
        return "t".to_owned();
    }
    if let Some(name) = value.as_symbol_name() {
        return format!("symbol:{name}");
    }
    if let Some(n) = value.as_fixnum() {
        return format!("fixnum:{n}");
    }
    if value.is_string() {
        return format!(
            "string:{}",
            value
                .as_runtime_string_owned()
                .expect("string value should have string payload")
        );
    }
    if value.is_marker() {
        return format!(
            "marker:{:?}",
            value
                .as_marker_data()
                .expect("marker value should have marker payload")
                .marker_id
        );
    }
    format!("{:?}", value.kind())
}

fn undo_entry_snapshot(entry: Value) -> UndoEntrySnapshot {
    match entry.kind() {
        ValueKind::Nil => UndoEntrySnapshot::Boundary,
        ValueKind::Fixnum(point) => UndoEntrySnapshot::Point(point),
        ValueKind::Cons => {
            let car = entry.cons_car();
            let cdr = entry.cons_cdr();
            match (car.kind(), cdr.kind()) {
                (ValueKind::Fixnum(beg1), ValueKind::Fixnum(end1)) => {
                    UndoEntrySnapshot::Insert { beg1, end1 }
                }
                (ValueKind::String, ValueKind::Fixnum(pos1)) => UndoEntrySnapshot::Delete {
                    text: car
                        .as_runtime_string_owned()
                        .expect("delete undo text should be a string"),
                    pos1,
                },
                (ValueKind::T, ValueKind::Fixnum(modtime)) => {
                    UndoEntrySnapshot::FirstChange(modtime)
                }
                (_, ValueKind::Fixnum(adjustment)) if car.is_marker() => {
                    UndoEntrySnapshot::MarkerAdjustment {
                        marker_id: car
                            .as_marker_data()
                            .expect("marker value should have marker payload")
                            .marker_id,
                        adjustment,
                    }
                }
                (ValueKind::Nil, _) if cdr.is_cons() => {
                    let prop = cdr.cons_car();
                    let rest1 = cdr.cons_cdr();
                    if rest1.is_cons() {
                        let old_value = rest1.cons_car();
                        let rest2 = rest1.cons_cdr();
                        if rest2.is_cons()
                            && let (Some(beg1), Some(end1)) =
                                (rest2.cons_car().as_fixnum(), rest2.cons_cdr().as_fixnum())
                        {
                            return UndoEntrySnapshot::PropertyChange {
                                prop: undo_value_label(prop),
                                old_value: undo_value_label(old_value),
                                beg1,
                                end1,
                            };
                        }
                    }
                    UndoEntrySnapshot::Other("malformed-property-change".to_owned())
                }
                _ => UndoEntrySnapshot::Other(format!(
                    "cons:{}:{}",
                    undo_value_label(car),
                    undo_value_label(cdr)
                )),
            }
        }
        _ => UndoEntrySnapshot::Other(undo_value_label(entry)),
    }
}

fn undo_list_snapshot(mut list: Value) -> Vec<UndoEntrySnapshot> {
    let mut entries = Vec::new();
    while list.is_cons() {
        entries.push(undo_entry_snapshot(list.cons_car()));
        list = list.cons_cdr();
    }
    if !list.is_nil() {
        entries.push(UndoEntrySnapshot::ImproperTail(undo_value_label(list)));
    }
    entries
}

#[derive(Debug, PartialEq, Eq)]
struct BackendUndoSnapshot {
    undo_before: Vec<UndoEntrySnapshot>,
    undo_result: UndoExecutionResult,
    undo_after: Vec<UndoEntrySnapshot>,
    buffer_string: String,
    point_byte: usize,
    point_char: usize,
    mark_byte: Option<usize>,
    mark_char: Option<usize>,
    marker_position: Option<(usize, usize)>,
    text_properties: Vec<(usize, usize, Vec<(Value, Value)>)>,
}

fn run_backend_undo_script(kind: BufferTextBackendKind) -> BackendUndoSnapshot {
    let mut mgr = manager_with_text_backend(kind);
    let id = mgr.current_buffer_id().expect("scratch buffer");
    let implemented_kind = require_implemented_kind(kind);
    let face = Value::symbol("face");
    let bold = Value::symbol("bold");
    let italic = Value::symbol("italic");

    {
        let buf = mgr.get_mut(id).expect("scratch buffer");
        buf.text = BufferText::from_str_with_backend_kind("aébc日本z", implemented_kind);
        buf.widen();
        buf.goto_byte(0);
        buf.set_mark_byte(buf.text.buf_charpos_to_bytepos(2));
        let prop_start = buf.text.buf_charpos_to_bytepos(1);
        let prop_end = buf.text.buf_charpos_to_bytepos(5);
        assert!(
            buf.text
                .text_props_put_property(prop_start, prop_end, face, bold)
        );
        let marker_pos = buf.text.buf_charpos_to_bytepos(4);
        register_marker_for_test(buf, 88, marker_pos, InsertionType::After);
        buf.set_undo_list(Value::NIL);
    }

    mgr.record_undo_point_before_command(id)
        .expect("record point before command");
    let insert_pos = mgr
        .get(id)
        .expect("scratch buffer")
        .text
        .buf_charpos_to_bytepos(2);
    mgr.goto_buffer_byte(id, insert_pos).expect("goto insert");
    mgr.insert_into_buffer(id, "Ω").expect("insert text");

    let prop_start = mgr
        .get(id)
        .expect("scratch buffer")
        .text
        .buf_charpos_to_bytepos(1);
    let prop_end = mgr
        .get(id)
        .expect("scratch buffer")
        .text
        .buf_charpos_to_bytepos(6);
    mgr.put_buffer_text_property(id, prop_start, prop_end, face, italic)
        .expect("put text property");

    let delete_start = mgr
        .get(id)
        .expect("scratch buffer")
        .text
        .buf_charpos_to_bytepos(4);
    let delete_end = mgr
        .get(id)
        .expect("scratch buffer")
        .text
        .buf_charpos_to_bytepos(6);
    mgr.delete_buffer_region(id, delete_start, delete_end)
        .expect("delete region");

    let replace_start = mgr
        .get(id)
        .expect("scratch buffer")
        .text
        .buf_charpos_to_bytepos(1);
    let replace_end = mgr
        .get(id)
        .expect("scratch buffer")
        .text
        .buf_charpos_to_bytepos(3);
    mgr.replace_buffer_region_lisp_string(
        id,
        replace_start,
        replace_end,
        &LispString::from_utf8("qλ"),
    )
    .expect("replace region");

    let undo_before = undo_list_snapshot(mgr.get(id).expect("scratch buffer").get_undo_list());
    let undo_result = mgr.undo_buffer(id, 1).expect("undo result");
    let buf = mgr.get(id).expect("scratch buffer");
    let marker_position = buf
        .text
        .marker_chain_lookup(88)
        .map(|(byte_pos, char_pos, _)| (byte_pos, char_pos));
    assert_eq!(buf.text.backend_kind(), kind);

    BackendUndoSnapshot {
        undo_before,
        undo_result,
        undo_after: undo_list_snapshot(buf.get_undo_list()),
        buffer_string: buf.buffer_string(),
        point_byte: buf.point_byte(),
        point_char: buf.point_char(),
        mark_byte: buf.mark(),
        mark_char: buf.mark_char(),
        marker_position,
        text_properties: buffer_text_property_snapshot(buf),
    }
}

#[test]
fn implemented_text_backends_match_undo_recording_and_execution() {
    crate::test_utils::init_test_tracing();
    let baseline = run_backend_undo_script(BufferTextBackendKind::GapBuffer);
    for kind in implemented_text_backends() {
        assert_eq!(
            run_backend_undo_script(kind),
            baseline,
            "{kind:?} undo semantics diverged from gap buffer"
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
struct BackendMigrationSnapshot {
    backend_kind: BufferTextBackendKind,
    buffer_string: String,
    char_to_byte_4: usize,
    byte_to_char_at_char_4: usize,
    marker_position: Option<(usize, usize)>,
    overlay_range: (Option<usize>, Option<usize>),
    text_properties: Vec<(usize, usize, Vec<(Value, Value)>)>,
}

fn run_backend_migration_script(
    initial_kind: BufferTextBackendKind,
    target_kind: BufferTextBackendKind,
    convert: bool,
) -> BackendMigrationSnapshot {
    let mut buf = buf_with_text_backend("aébc日本z", initial_kind);

    let face = Value::symbol("face");
    let bold = Value::symbol("bold");
    let prop_start = buf.text.buf_charpos_to_bytepos(1);
    let prop_end = buf.text.buf_charpos_to_bytepos(5);
    assert!(
        buf.text
            .text_props_put_property(prop_start, prop_end, face, bold)
    );
    register_marker_for_test(&mut buf, 77, 4, InsertionType::After);
    let overlay = Value::make_overlay(OverlayData {
        serial: 0,
        plist: Value::NIL,
        buffer: Some(buf.id),
        start: 3,
        end: 8,
        front_advance: false,
        rear_advance: true,
    });
    buf.overlays.insert_overlay(overlay);

    let cached_byte = buf.text.buf_charpos_to_bytepos(4);
    assert_eq!(buf.text.buf_bytepos_to_charpos(cached_byte), 4);

    if convert {
        buf.text
            .convert_backend_kind(require_implemented_kind(target_kind));
    }
    assert_eq!(buf.text.backend_kind(), target_kind);
    assert_eq!(buf.text.buf_charpos_to_bytepos(4), cached_byte);
    assert_eq!(buf.text.buf_bytepos_to_charpos(cached_byte), 4);

    let insert_pos = buf.text.buf_charpos_to_bytepos(2);
    buf.goto_byte(insert_pos);
    buf.insert("Ω");

    let delete_start = buf.text.buf_charpos_to_bytepos(5);
    let delete_end = buf.text.buf_charpos_to_bytepos(6);
    buf.delete_region(delete_start, delete_end);

    let char_to_byte_4 = buf.text.buf_charpos_to_bytepos(4);
    BackendMigrationSnapshot {
        backend_kind: buf.text.backend_kind(),
        buffer_string: buf.buffer_string(),
        char_to_byte_4,
        byte_to_char_at_char_4: buf.text.buf_bytepos_to_charpos(char_to_byte_4),
        marker_position: buf
            .text
            .marker_chain_lookup(77)
            .map(|(byte_pos, char_pos, _)| (byte_pos, char_pos)),
        overlay_range: (
            buf.overlays.overlay_start(overlay),
            buf.overlays.overlay_end(overlay),
        ),
        text_properties: buffer_text_property_snapshot(&buf),
    }
}

#[test]
fn buffer_text_backend_migration_preserves_side_data_and_post_edit_semantics() {
    crate::test_utils::init_test_tracing();
    for target in implemented_text_backends() {
        let baseline = run_backend_migration_script(target, target, false);
        for source in implemented_text_backends() {
            assert_eq!(
                run_backend_migration_script(source, target, true),
                baseline,
                "{source:?} -> {target:?} backend migration diverged from direct target backend"
            );
        }
    }
}

// -----------------------------------------------------------------------
// Buffer-local variables
// -----------------------------------------------------------------------

#[test]
fn buffer_local_get_set() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("test"));
    assert!(buf.get_buffer_local("tab-width").is_none());

    buf.set_buffer_local("tab-width", Value::fixnum(4));
    let val = buf.get_buffer_local("tab-width").unwrap();
    assert!(val.is_fixnum());

    buf.set_buffer_local("tab-width", Value::fixnum(8));
    let val = buf.get_buffer_local("tab-width").unwrap();
    assert!(val.is_fixnum());
}

#[test]
fn buffer_local_multiple_vars() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("test"));
    buf.set_buffer_local("fill-column", Value::fixnum(80));
    buf.set_buffer_local("major-mode", Value::symbol("text-mode"));

    assert!(buf.get_buffer_local("fill-column").is_some());
    assert!(buf.get_buffer_local("major-mode").is_some());
    assert!(buf.get_buffer_local("nonexistent").is_none());
}

#[test]
fn buffer_local_defaults_include_builtin_per_buffer_vars() {
    crate::test_utils::init_test_tracing();
    let buf = Buffer::new(BufferId(1), Value::string("test"));

    assert_eq!(
        buf.buffer_local_value("major-mode"),
        Some(Value::symbol("fundamental-mode"))
    );
    assert_eq!(
        buf.buffer_local_value("mode-name"),
        Some(Value::string("Fundamental"))
    );
    assert_eq!(buf.buffer_local_value("buffer-file-name"), Some(Value::NIL));
    assert_eq!(
        buf.buffer_local_value("buffer-auto-save-file-name"),
        Some(Value::NIL)
    );
    assert_eq!(
        buf.buffer_local_value("buffer-display-count"),
        Some(Value::fixnum(0))
    );
    assert_eq!(
        buf.buffer_local_value("buffer-display-time"),
        Some(Value::NIL)
    );
    assert_eq!(
        buf.buffer_local_value("buffer-invisibility-spec"),
        Some(Value::T)
    );
    assert_eq!(buf.buffer_local_value("buffer-undo-list"), Some(Value::NIL));
}

#[test]
fn ordered_buffer_local_bindings_use_symbol_ids() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("test"));
    buf.set_buffer_local("fill-column", Value::fixnum(80));
    buf.set_buffer_local("major-mode", Value::symbol("text-mode"));

    let ordered = buf.ordered_buffer_local_bindings();
    assert!(
        ordered
            .iter()
            .any(|(sym_id, _)| *sym_id == crate::emacs_core::intern::intern("fill-column"))
    );
    assert!(
        ordered
            .iter()
            .any(|(sym_id, _)| *sym_id == crate::emacs_core::intern::intern("major-mode"))
    );
    assert!(
        ordered
            .iter()
            .any(|(sym_id, _)| *sym_id == crate::emacs_core::intern::intern("buffer-undo-list"))
    );
}

#[test]
fn buffer_file_name_variable_tracks_slot_backed_state() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("test"));
    assert_eq!(buf.buffer_local_value("buffer-file-name"), Some(Value::NIL));

    buf.set_buffer_local("buffer-file-name", Value::string("/tmp/demo.txt"));
    assert_eq!(
        buf.file_name_runtime_string_owned().as_deref(),
        Some("/tmp/demo.txt")
    );
    assert_eq!(buf.file_name_value(), Value::string("/tmp/demo.txt"));
    assert_eq!(
        buf.buffer_local_value("buffer-file-name"),
        Some(Value::string("/tmp/demo.txt"))
    );

    buf.set_buffer_local("buffer-file-name", Value::NIL);
    assert!(buf.file_name_value().is_nil());
    assert_eq!(buf.buffer_local_value("buffer-file-name"), Some(Value::NIL));
}

#[test]
fn buffer_auto_save_file_name_variable_tracks_slot_backed_state() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("test"));
    assert_eq!(
        buf.buffer_local_value("buffer-auto-save-file-name"),
        Some(Value::NIL)
    );

    buf.set_buffer_local(
        "buffer-auto-save-file-name",
        Value::string("/tmp/#demo.txt#"),
    );
    assert_eq!(
        buf.auto_save_file_name_runtime_string_owned().as_deref(),
        Some("/tmp/#demo.txt#")
    );
    assert_eq!(
        buf.auto_save_file_name_value(),
        Value::string("/tmp/#demo.txt#")
    );
    assert_eq!(
        buf.buffer_local_value("buffer-auto-save-file-name"),
        Some(Value::string("/tmp/#demo.txt#"))
    );

    buf.set_buffer_local("buffer-auto-save-file-name", Value::NIL);
    assert!(buf.auto_save_file_name_value().is_nil());
    assert_eq!(
        buf.buffer_local_value("buffer-auto-save-file-name"),
        Some(Value::NIL)
    );
}

// -----------------------------------------------------------------------
// Modified flag
// -----------------------------------------------------------------------

#[test]
fn modified_flag() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("test"));
    assert!(!buf.is_modified());
    buf.insert("x");
    assert!(buf.is_modified());
    buf.set_modified(false);
    assert!(!buf.is_modified());
}

#[test]
fn modified_state_tracks_autosaved_semantics() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("test"));
    assert_eq!(buf.modified_state_value(), Value::NIL);
    assert!(!buf.recent_auto_save_p());
    assert_eq!(buf.modified_tick(), 1);
    assert_eq!(buf.chars_modified_tick(), 1);

    assert_eq!(buf.restore_modified_state(Value::T), Value::T);
    assert_eq!(buf.modified_state_value(), Value::T);
    assert_eq!(buf.modified_tick(), 2);
    assert_eq!(buf.chars_modified_tick(), 1);
    assert!(!buf.recent_auto_save_p());

    assert_eq!(
        buf.restore_modified_state(Value::symbol("autosaved")),
        Value::symbol("autosaved")
    );
    assert_eq!(buf.modified_state_value(), Value::symbol("autosaved"));
    assert_eq!(buf.modified_tick(), 2);
    assert_eq!(buf.chars_modified_tick(), 1);
    assert!(buf.recent_auto_save_p());

    assert_eq!(buf.restore_modified_state(Value::NIL), Value::NIL);
    assert_eq!(buf.modified_state_value(), Value::NIL);
    assert_eq!(buf.modified_tick(), 2);
    assert_eq!(buf.chars_modified_tick(), 1);
    assert!(!buf.recent_auto_save_p());
}

#[test]
fn modification_ticks_track_content_changes() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("test"));
    assert_eq!(buf.modified_tick(), 1);
    assert_eq!(buf.chars_modified_tick(), 1);

    buf.insert("abcdef");
    assert_eq!(buf.modified_tick(), 4);
    assert_eq!(buf.chars_modified_tick(), 4);

    buf.set_modified(false);
    assert_eq!(buf.modified_tick(), 4);
    assert_eq!(buf.chars_modified_tick(), 4);
    assert_eq!(buf.modified_state_value(), Value::NIL);

    buf.delete_region(0, 6);
    assert_eq!(buf.modified_tick(), 7);
    assert_eq!(buf.chars_modified_tick(), 7);
    assert_eq!(buf.modified_state_value(), Value::T);
}

#[test]
fn chars_modified_tick_rejoins_modiff_after_non_char_modification() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("test"));
    assert_eq!(buf.restore_modified_state(Value::T), Value::T);
    assert_eq!(buf.modified_tick(), 2);
    assert_eq!(buf.chars_modified_tick(), 1);

    buf.insert("x");
    assert_eq!(buf.modified_tick(), 3);
    assert_eq!(buf.chars_modified_tick(), 3);
    assert_eq!(buf.modified_state_value(), Value::T);
}

// -----------------------------------------------------------------------
// BufferManager — creation, lookup, kill
// -----------------------------------------------------------------------

#[test]
fn manager_starts_with_scratch() {
    crate::test_utils::init_test_tracing();
    let mgr = BufferManager::new();
    let scratch = mgr.find_buffer_by_name("*scratch*");
    assert!(scratch.is_some());
    assert!(mgr.current_buffer().is_some());
    assert_eq!(
        mgr.current_buffer().unwrap().name_value(),
        Value::string("*scratch*")
    );
}

#[test]
fn manager_create_and_lookup() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let id = mgr.create_buffer("foo.el");
    assert!(mgr.get(id).is_some());
    assert_eq!(mgr.get(id).unwrap().name_value(), Value::string("foo.el"));
    assert_eq!(mgr.find_buffer_by_name("foo.el"), Some(id));
    assert_eq!(mgr.find_buffer_by_name("bar.el"), None);
}

#[test]
fn manager_set_current() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let a = mgr.create_buffer("a");
    let b = mgr.create_buffer("b");
    mgr.set_current(a);
    assert_eq!(
        mgr.current_buffer().unwrap().name_value(),
        Value::string("a")
    );
    mgr.set_current(b);
    assert_eq!(
        mgr.current_buffer().unwrap().name_value(),
        Value::string("b")
    );
}

#[test]
fn indirect_buffer_reads_undo_list_from_shared_state() {
    // Phase 10F: `buffer-undo-list` now reads directly from
    // `SharedUndoState` via `Buffer::get_undo_list`, so both
    // base and indirect buffers observe the same value without
    // any per-buffer cache. The previous version of this test
    // verified the cache-refresh behavior that the old
    // `BufferLocals::lisp_bindings` mirror needed — that
    // mirror is gone, and so is the refresh dance.
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let base_id = mgr.current_buffer_id().expect("scratch buffer");
    let indirect_id = mgr
        .create_indirect_buffer(base_id, "*switch-current-indirect*", false)
        .expect("indirect buffer");
    let _ = mgr.insert_into_buffer(base_id, "abc");

    let shared = mgr.get(base_id).expect("base buffer").get_undo_list();
    assert_eq!(
        mgr.get(indirect_id)
            .expect("indirect buffer")
            .get_buffer_local("buffer-undo-list"),
        Some(shared)
    );
}

#[test]
fn manager_kill_buffer() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let id = mgr.create_buffer("doomed");
    assert!(mgr.kill_buffer(id));
    assert!(mgr.get(id).is_none());
    assert!(!mgr.kill_buffer(id)); // already dead
}

#[test]
fn manager_kill_current_clears_current() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let scratch = mgr.find_buffer_by_name("*scratch*").unwrap();
    mgr.set_current(scratch);
    mgr.kill_buffer(scratch);
    assert!(mgr.current_buffer().is_none());
}

#[test]
fn manager_buffer_list() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let scratch = mgr.find_buffer_by_name("*scratch*").expect("scratch");
    let a = mgr.create_buffer("a");
    let b = mgr.create_buffer("b");
    assert_eq!(mgr.buffer_list(), vec![scratch, a, b]);
}

#[test]
fn manager_recorded_switch_records_even_when_buffer_is_already_current() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let scratch = mgr.find_buffer_by_name("*scratch*").expect("scratch");
    let a = mgr.create_buffer("a");
    let b = mgr.create_buffer("b");

    mgr.switch_current(a);
    mgr.switch_current(b);
    assert_eq!(mgr.buffer_list(), vec![b, a, scratch]);

    mgr.switch_current_unrecorded(a);
    assert_eq!(mgr.current_buffer_id(), Some(a));
    assert_eq!(mgr.buffer_list(), vec![b, a, scratch]);

    mgr.switch_current(a);
    assert_eq!(mgr.buffer_list(), vec![a, b, scratch]);
}

#[test]
fn manager_order_after_relinks_without_selecting_or_recording_head() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let scratch = mgr.find_buffer_by_name("*scratch*").expect("scratch");
    let messages = mgr.create_buffer("*Messages*");
    let minibuf = mgr.create_buffer(" *Minibuf-0*");

    assert_eq!(mgr.current_buffer_id(), Some(scratch));
    assert!(mgr.note_buffer_order_after(messages, minibuf));
    assert_eq!(mgr.current_buffer_id(), Some(scratch));
    assert_eq!(mgr.buffer_list(), vec![scratch, minibuf, messages]);
}

#[test]
fn manager_generate_new_buffer_name_unique() {
    crate::test_utils::init_test_tracing();
    let mgr = BufferManager::new();
    // "*scratch*" is taken, "foo" is not.
    assert_eq!(mgr.generate_new_buffer_name("foo"), "foo");
    assert_eq!(mgr.generate_new_buffer_name("*scratch*"), "*scratch*<2>");
}

#[test]
fn manager_generate_new_buffer_name_increments() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    mgr.create_buffer("buf");
    assert_eq!(mgr.generate_new_buffer_name("buf"), "buf<2>");
    mgr.create_buffer("buf<2>");
    assert_eq!(mgr.generate_new_buffer_name("buf"), "buf<3>");
}

#[test]
fn manager_generate_new_buffer_name_honors_ignore_candidate() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    mgr.create_buffer("buf");
    mgr.create_buffer("buf<2>");
    assert_eq!(
        mgr.generate_new_buffer_name_ignoring("buf", Some("buf<2>")),
        "buf<2>"
    );
    assert_eq!(
        mgr.generate_new_buffer_name_ignoring("buf", Some("buf<3>")),
        "buf<3>"
    );
}

#[test]
fn manager_generate_new_buffer_name_hidden_buffer_uses_random_suffix() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    mgr.create_buffer(" hidden");
    assert_eq!(
        mgr.generate_new_buffer_name_ignoring_with_random(" hidden", None, || 123_456),
        " hidden-123456"
    );
}

#[test]
fn manager_generate_new_buffer_name_hidden_buffer_falls_back_after_random_collision() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    mgr.create_buffer(" hidden");
    mgr.create_buffer(" hidden-123456");
    assert_eq!(
        mgr.generate_new_buffer_name_ignoring_with_random(" hidden", None, || 123_456),
        " hidden-123456<2>"
    );
}

// -----------------------------------------------------------------------
// BufferManager — markers
// -----------------------------------------------------------------------

#[test]
fn manager_create_and_query_marker() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let id = mgr.create_buffer("m");
    // Insert some text so there is room for a marker.
    mgr.get_mut(id).unwrap().text = BufferText::from_str("abcdef");
    mgr.get_mut(id).unwrap().widen();

    let (mid, _) = mgr.create_marker(id, 3, InsertionType::After);
    assert_eq!(mgr.marker_position(id, mid), Some(3));
    assert_eq!(mgr.marker_char_position(id, mid), Some(3));
}

#[test]
fn manager_marker_clamped_to_buffer_len() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let id = mgr.create_buffer("m");
    // Buffer is empty (len = 0), marker at 100 should be clamped.
    let (mid, _) = mgr.create_marker(id, 100, InsertionType::Before);
    assert_eq!(mgr.marker_position(id, mid), Some(0));
    assert_eq!(mgr.marker_char_position(id, mid), Some(0));
}

#[test]
fn manager_marker_nonexistent_buffer() {
    crate::test_utils::init_test_tracing();
    let mgr = BufferManager::new();
    let pos = mgr.marker_position(BufferId(9999), 1);
    assert_eq!(pos, None);
}

#[test]
fn manager_labeled_widen_uses_innermost_and_without_restriction_reaches_full_buffer() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let id = mgr.create_buffer("labeled");
    mgr.set_current(id);
    mgr.get_mut(id).unwrap().insert("abcdef");

    let _ = mgr.internal_labeled_narrow_to_region(id, 1, 4, Value::symbol("tag"));
    let buf = mgr.get(id).unwrap();
    assert_eq!(buf.point_min(), 1);
    assert_eq!(buf.point_max(), 4);

    let _ = mgr.widen_buffer(id);
    let buf = mgr.get(id).unwrap();
    assert_eq!(buf.point_min(), 1);
    assert_eq!(buf.point_max(), 4);

    let _ = mgr.internal_labeled_widen(id, &Value::symbol("tag"));
    let buf = mgr.get(id).unwrap();
    assert_eq!(buf.point_min(), 0);
    assert_eq!(buf.point_max(), 6);
}

#[test]
fn manager_save_restriction_state_restores_labeled_stack() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let id = mgr.create_buffer("saved-labeled");
    mgr.set_current(id);
    mgr.get_mut(id).unwrap().insert("abcdefgh");
    let _ = mgr.internal_labeled_narrow_to_region(id, 1, 5, Value::symbol("tag"));

    let saved = mgr
        .save_current_restriction_state()
        .expect("restriction state should save");
    let _ = mgr.internal_labeled_widen(id, &Value::symbol("tag"));
    let _ = mgr.narrow_buffer_to_region(id, 2, 3);
    mgr.restore_saved_restriction_state(saved);

    let buf = mgr.get(id).unwrap();
    assert_eq!(buf.point_min(), 1);
    assert_eq!(buf.point_max(), 5);

    let _ = mgr.widen_buffer(id);
    let buf = mgr.get(id).unwrap();
    assert_eq!(buf.point_min(), 1);
    assert_eq!(buf.point_max(), 5);
}

#[test]
fn manager_reset_outermost_restrictions_restores_current_innermost_after_mutation() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let id = mgr.create_buffer("redisplay-labeled");
    mgr.set_current(id);
    mgr.get_mut(id).unwrap().insert("abcdef");

    let _ = mgr.internal_labeled_narrow_to_region(id, 1, 5, Value::symbol("outer"));
    let _ = mgr.internal_labeled_narrow_to_region(id, 2, 4, Value::symbol("inner"));

    let buf = mgr.get(id).unwrap();
    assert_eq!(buf.point_min(), 2);
    assert_eq!(buf.point_max(), 4);

    let saved = mgr.reset_outermost_restrictions();
    let buf = mgr.get(id).unwrap();
    assert_eq!(buf.point_min(), 0);
    assert_eq!(buf.point_max(), 6);

    let _ = mgr.internal_labeled_widen(id, &Value::symbol("inner"));
    let buf = mgr.get(id).unwrap();
    assert_eq!(buf.point_min(), 1);
    assert_eq!(buf.point_max(), 5);

    mgr.restore_outermost_restrictions(saved);
    let buf = mgr.get(id).unwrap();
    assert_eq!(buf.point_min(), 1);
    assert_eq!(buf.point_max(), 5);
}

// -----------------------------------------------------------------------
// BufferManager — current_buffer_mut
// -----------------------------------------------------------------------

#[test]
fn manager_current_buffer_mut_insert() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let current = mgr.current_buffer_id().unwrap();
    mgr.insert_into_buffer(current, "hello");
    assert_eq!(mgr.current_buffer().unwrap().buffer_string(), "hello");
}

#[test]
fn manager_replace_buffer_contents_resets_narrowing_and_point() {
    crate::test_utils::init_test_tracing();
    let mut mgr = BufferManager::new();
    let current = mgr.current_buffer_id().unwrap();
    let buf = mgr.get_mut(current).unwrap();
    buf.insert("abcdefgh");
    buf.narrow_to_region(2, 6);
    buf.goto_char(4);

    mgr.replace_buffer_contents(current, "xy");

    let buf = mgr.get(current).unwrap();
    assert_eq!(buf.buffer_string(), "xy");
    assert_eq!(buf.point(), 0);
    assert_eq!(buf.point_min(), 0);
    assert_eq!(buf.point_max(), 2);
}

// -----------------------------------------------------------------------
// Integration: multiple operations
// -----------------------------------------------------------------------

#[test]
fn integration_edit_narrow_widen() {
    crate::test_utils::init_test_tracing();
    let mut buf = Buffer::new(BufferId(1), Value::string("work"));
    buf.insert("abcdefghij");
    assert_eq!(buf.buffer_string(), "abcdefghij");

    buf.narrow_to_region(2, 8);
    assert_eq!(buf.buffer_string(), "cdefgh");

    buf.goto_char(5);
    buf.insert("XX");
    assert_eq!(buf.buffer_string(), "cdeXXfgh");

    buf.widen();
    assert_eq!(buf.buffer_string(), "abcdeXXfghij");
}

// -----------------------------------------------------------------------
// T8 C-1 regression: state markers must survive GC without Lisp refs
// -----------------------------------------------------------------------

#[test]
fn state_markers_survive_gc_without_lisp_references() {
    // Post-T8 invariant: BufferStateMarkers.pt_marker_ptr / begv_marker_ptr /
    // zv_marker_ptr must survive GC even when no Lisp value holds them. If the
    // chain is the only structural reference AND the chain isn't rooted, an
    // unmarked marker would be spliced out by unchain_dead_markers and freed,
    // leaving the state_markers struct pointing at freed memory.
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();

    // Create a base buffer with some text, then make an indirect buffer.
    // `create_indirect_buffer` is the code path that calls
    // `ensure_buffer_state_markers` on both the root and the indirect,
    // which is what materialises the pt/begv/zv state markers.
    let base_id = eval.buffers.current_buffer_id().expect("scratch buffer");
    let _ = eval.buffers.insert_into_buffer(base_id, "hello world");
    let indirect_id = eval
        .buffers
        .create_indirect_buffer(base_id, "*gc-state-marker-indirect*", false)
        .expect("indirect buffer");

    // Snapshot the raw pointers from state_markers before GC. Use the
    // indirect buffer because that is where the noncurrent-state markers
    // conceptually live (the root also gets one for the same reason).
    let (pt_ptr, begv_ptr, zv_ptr, expected_buf) = {
        let buffer = eval
            .buffers
            .get(indirect_id)
            .expect("indirect buffer present");
        let sm = buffer
            .state_markers
            .as_ref()
            .expect("state markers populated by create_indirect_buffer");
        (
            sm.pt_marker_ptr,
            sm.begv_marker_ptr,
            sm.zv_marker_ptr,
            buffer.id,
        )
    };

    // Sanity: all three pointers are non-null and distinct.
    assert!(!pt_ptr.is_null(), "pt_marker_ptr populated");
    assert!(!begv_ptr.is_null(), "begv_marker_ptr populated");
    assert!(!zv_ptr.is_null(), "zv_marker_ptr populated");

    // Walk the indirect buffer's marker chain BEFORE GC: we expect pt,
    // begv, zv to all be present (the chain-head slot ultimately points
    // at one of them, and each one's `next_marker` eventually reaches
    // the others). This is our positive baseline — if the pre-GC chain
    // does not contain these three, the test setup is wrong.
    let chain_contains_before = unsafe {
        let buffer = eval
            .buffers
            .get(indirect_id)
            .expect("indirect buffer present");
        let head_slot: *const *mut crate::tagged::header::MarkerObj =
            buffer.text.markers_head_slot_raw() as *const _;
        let mut contains = [false; 3];
        let mut curr = *head_slot;
        while !curr.is_null() {
            if curr == pt_ptr {
                contains[0] = true;
            }
            if curr == begv_ptr {
                contains[1] = true;
            }
            if curr == zv_ptr {
                contains[2] = true;
            }
            curr = (*curr).data.next_marker;
        }
        contains
    };
    assert!(
        chain_contains_before.iter().all(|&b| b),
        "pre-GC baseline: chain must contain all three state markers, got {chain_contains_before:?}"
    );

    // Force a full GC. No Lisp value references these three markers; the
    // only structural references are (a) the intrusive marker chain and
    // (b) the `BufferStateMarkers` raw pointers. If neither is treated as
    // a GC root, `unchain_dead_markers` will splice them out and
    // `sweep_objects` will free them.
    eval.gc_collect_exact();

    // After GC, the pointers must still point at LIVE markers whose
    // header has the expected tag and whose data still reflects the
    // buffer binding. Reading a freed allocation is UB; we can't make
    // this test segfault-proof without ASAN, but if the allocation was
    // reused for something else, `data.buffer` will almost certainly no
    // longer match `expected_buf`.
    unsafe {
        let pt_buffer = (*pt_ptr).data.buffer;
        let begv_buffer = (*begv_ptr).data.buffer;
        let zv_buffer = (*zv_ptr).data.buffer;

        assert_eq!(pt_buffer, Some(expected_buf), "pt_marker survived GC");
        assert_eq!(begv_buffer, Some(expected_buf), "begv_marker survived GC");
        assert_eq!(zv_buffer, Some(expected_buf), "zv_marker survived GC");

        assert!(
            (*pt_ptr).data.marker_id.is_some(),
            "pt_marker retains its marker_id"
        );
        assert!(
            (*begv_ptr).data.marker_id.is_some(),
            "begv_marker retains its marker_id"
        );
        assert!(
            (*zv_ptr).data.marker_id.is_some(),
            "zv_marker retains its marker_id"
        );
    }

    // The chain must STILL contain all three state markers after GC;
    // `unchain_dead_markers` splices out anything with `header.gc.marked`
    // false, so a post-GC chain containing them proves they were marked
    // (i.e. they were treated as reachable by the mark phase).
    let chain_contains_after = unsafe {
        let buffer = eval
            .buffers
            .get(indirect_id)
            .expect("indirect buffer present");
        let head_slot: *const *mut crate::tagged::header::MarkerObj =
            buffer.text.markers_head_slot_raw() as *const _;
        let mut contains = [false; 3];
        let mut curr = *head_slot;
        let mut guard = 0usize;
        while !curr.is_null() && guard < 4096 {
            if curr == pt_ptr {
                contains[0] = true;
            }
            if curr == begv_ptr {
                contains[1] = true;
            }
            if curr == zv_ptr {
                contains[2] = true;
            }
            curr = (*curr).data.next_marker;
            guard += 1;
        }
        contains
    };
    assert!(
        chain_contains_after.iter().all(|&b| b),
        "post-GC: all three state markers must remain on the chain; got {chain_contains_after:?}. \
         A `false` here proves C-1: an unmarked state marker was spliced out and its allocation \
         freed, leaving BufferStateMarkers with a dangling pointer."
    );
}

#[test]
fn buffer_mark_marker_survives_gc_without_lisp_reference() {
    // GNU stores the mark as `BVAR (buffer, mark)`, so `mark_buffer`
    // roots it with the rest of the buffer object.  Neomacs mirrors that
    // with Buffer::mark_marker_ptr; the raw pointer must be traced even
    // when no Lisp variable currently holds `(mark-marker)`.
    crate::test_utils::init_test_tracing();
    let mut eval = crate::emacs_core::eval::Context::new();
    let buffer_id = eval.buffers.current_buffer_id().expect("scratch buffer");
    let _ = eval.buffers.insert_into_buffer(buffer_id, "alpha\nbeta\n");
    let _ = eval.buffers.set_buffer_mark(buffer_id, 1);

    let mark_ptr = eval
        .buffers
        .get(buffer_id)
        .expect("buffer present")
        .mark_marker_ptr;
    assert!(!mark_ptr.is_null(), "mark marker should be materialized");

    eval.gc_collect_exact();

    let mark = crate::emacs_core::marker::builtin_mark_marker(&mut eval, vec![])
        .expect("mark-marker should return a live marker after GC");
    assert!(mark.is_marker(), "mark-marker remains a marker after GC");
    let position =
        crate::emacs_core::marker::marker_position_as_int_with_buffers(&eval.buffers, &mark)
            .expect("marker-position should accept the buffer mark after GC");
    assert_eq!(position, 2);

    unsafe {
        assert_eq!(
            (*mark_ptr).data.buffer,
            Some(buffer_id),
            "raw buffer mark pointer still refers to its buffer"
        );
    }
}
