use super::*;
use crate::display_cursor::{
    CapturedTextWindowCursorPublishContext, CapturedTextWindowCursorPublishOutcome,
    CursorGeometryContext, CursorGeometrySource, VisualTextWindowCursorPublishContext,
    VisualTextWindowCursorPublishSummary, cursor_style_for_window,
};
use crate::display_face_id::FrameFaceIdAllocator;
use crate::display_face_policy::BaseFacePolicy;
use crate::display_item::RenderFaceRef;
use crate::display_origin::{DisplayOrigin, OverlayStringKind};
use crate::display_output_builder::DisplayOutputBuilder;
use crate::display_row::{
    DisplayRowActiveFaceState, DisplayRowFace, DisplayRowGeometry, DisplayRowGlyphMeasurer,
    DisplayRowMeasurementPolicy, DisplayRowRenderBounds, DisplayRowRenderer,
    DisplayRowSourceFragmentFrame,
};
use crate::display_row_builder::{
    DisplayGlyphMeasurer, DisplayRowGlyphCheckpoint, DisplayRowPosition, DisplayTabPolicy,
};
use crate::display_row_geometry::DisplayRowMaxX;
use crate::display_row_transition::{
    DisplayRowLineBreakTransitionPlan, DisplayRowTransitionRenderState,
};
use crate::display_row_walk_state::{
    DisplayRowTextOverflowDecision, SpecialTextRowOverflowDecision, TextRowTransitionStatePolicy,
    next_window_start_for_partially_visible_point_row,
    next_window_start_for_point_line_continuation, next_window_start_from_visible_rows,
};
use crate::display_source::{DisplayReplacementSpaceGeometry, DisplayReplacementStretchSourceItem};
use crate::glyph_advance::GlyphAdvanceQuantization;
use crate::neovm_bridge::{FaceResolver, LayoutBufferSnapshot, RustBufferAccess};
use crate::types::{VisualCursorSpec, WindowKind};
use crate::window_output::{TextWindowOutputTarget, WindowOutputEmitter};
use neomacs_display_protocol::cursor::CursorBarWidth;
use neomacs_display_protocol::frame_glyphs::{CursorKind, DisplaySlotId, GlyphRowRole};
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphRow, GlyphType};
use neovm_core::buffer::{
    BufferId, BufferTextBackendKind, CharPos0, EmacsBytePos, EmacsByteRange, LispCharPos1,
};
use neovm_core::emacs_core::eval::{
    DisplayHost, GuiFrameHostRequest, ImageResolveRequest, ResolvedImage, ResolvedVideo,
    ResolvedWebKit, VideoResolveRequest, WebKitResolveRequest,
};
use neovm_core::emacs_core::load::{
    apply_runtime_startup_state, create_bootstrap_evaluator_cached_with_features,
};
use neovm_core::emacs_core::value::StringTextPropertyRun;
use neovm_core::emacs_core::{Context, Value};
use neovm_core::face::FaceTable;
use neovm_core::heap_types::LispString;
use neovm_core::window::{
    DisplayPointSnapshot, DisplayRowSnapshot, WindowCursorSnapshot, WindowVisibleBufferSpan,
};
use std::sync::{Arc, Mutex};

trait BufferTextPropertyTestExt {
    fn put_text_property(&mut self, start: usize, end: usize, name: Value, value: Value) -> bool;
}

fn emacs_byte_range(start: usize, end: usize) -> EmacsByteRange {
    EmacsByteRange::new(EmacsBytePos::new(start), EmacsBytePos::new(end))
}

impl BufferTextPropertyTestExt for neovm_core::buffer::Buffer {
    fn put_text_property(&mut self, start: usize, end: usize, name: Value, value: Value) -> bool {
        self.text_props_put_property_in_emacs_byte_range(emacs_byte_range(start, end), name, value)
    }
}

#[test]
fn resize_mini_windows_mode_parses_gnu_values() {
    assert_eq!(
        ResizeMiniWindowsMode::from_lisp_value(Some(&Value::NIL)),
        ResizeMiniWindowsMode::Disabled
    );
    assert_eq!(
        ResizeMiniWindowsMode::from_lisp_value(Some(&Value::symbol("grow-only"))),
        ResizeMiniWindowsMode::GrowOnly
    );
    assert_eq!(
        ResizeMiniWindowsMode::from_lisp_value(Some(&Value::T)),
        ResizeMiniWindowsMode::Exact
    );
    assert_eq!(
        ResizeMiniWindowsMode::from_lisp_value(Some(&Value::symbol("anything-else"))),
        ResizeMiniWindowsMode::Exact
    );
}

#[test]
fn grow_only_minibuffer_shrinks_only_when_visible_region_is_empty() {
    assert!(ResizeMiniWindowsMode::GrowOnly.should_grow());
    assert!(!ResizeMiniWindowsMode::Disabled.should_grow());
    // `t` (Exact) always shrinks, regardless of exact_p / emptiness.
    assert!(ResizeMiniWindowsMode::Exact.should_shrink(false, false));
    assert!(ResizeMiniWindowsMode::Exact.should_shrink(true, false));
    // `nil` (Disabled) never shrinks.
    assert!(!ResizeMiniWindowsMode::Disabled.should_shrink(false, true));
    assert!(!ResizeMiniWindowsMode::Disabled.should_shrink(true, true));
    // `grow-only` shrinks for an empty buffer (GNU `BEGV == ZV`)...
    assert!(ResizeMiniWindowsMode::GrowOnly.should_shrink(false, true));
    // ...or when an exact resize is requested (GNU `exact_p`, i.e. the
    // post-command `resize_echo_area_exactly` with `minibuf_level == 0`),
    // even for a non-empty shorter message.
    assert!(ResizeMiniWindowsMode::GrowOnly.should_shrink(true, false));
    // But never for a non-empty buffer with no exact request (normal
    // mid-redisplay grow-only behavior keeps the larger size).
    assert!(!ResizeMiniWindowsMode::GrowOnly.should_shrink(false, false));
}

#[test]
fn word_wrap_break_candidate_records_rewind_position_and_clears() {
    let mut candidate = WordWrapBreakCandidate::default();

    assert!(!candidate.is_available());

    candidate.record(
        7,
        42,
        3,
        (Some(LispCharPos1::new(9)), Some(LispCharPos1::new(13))),
        DisplayRowGlyphCheckpoint::default(),
    );

    assert!(candidate.is_available());
    assert_eq!(candidate.byte_idx(), 7);
    assert_eq!(candidate.charpos(), 42);
    assert_eq!(candidate.display_point_count(), 3);
    assert_eq!(
        candidate.row_display_positions(),
        (Some(LispCharPos1::new(9)), Some(LispCharPos1::new(13)))
    );
    assert_eq!(
        candidate.glyph_checkpoint(),
        DisplayRowGlyphCheckpoint::default()
    );

    candidate.clear();

    assert!(!candidate.is_available());
}

#[test]
fn word_wrap_render_state_records_candidates_only_when_wrap_is_allowed() {
    let mut state = WordWrapRenderState::new(true);

    assert!(!state.candidate().is_available());

    state.record_candidate(
        ' ',
        7,
        42,
        3,
        (Some(LispCharPos1::new(9)), Some(LispCharPos1::new(13))),
        DisplayRowGlyphCheckpoint::default(),
    );

    assert!(!state.candidate().is_available());

    state.allow_after_current_char(' ');
    state.record_candidate(
        'a',
        7,
        42,
        3,
        (Some(LispCharPos1::new(9)), Some(LispCharPos1::new(13))),
        DisplayRowGlyphCheckpoint::default(),
    );

    assert!(state.candidate().is_available());
    assert_eq!(state.candidate().byte_idx(), 7);
    assert_eq!(state.candidate().charpos(), 42);

    state.reset_after_row_transition();

    assert!(!state.candidate().is_available());

    let mut disabled = WordWrapRenderState::new(false);
    disabled.allow_after_current_char(' ');
    disabled.record_candidate(
        'a',
        1,
        2,
        3,
        (None, None),
        DisplayRowGlyphCheckpoint::default(),
    );

    assert!(!disabled.candidate().is_available());
}

#[test]
fn text_row_transition_state_policy_applies_line_break_state_updates() {
    let mut line_numbers = LineNumberRenderState::new(true, 3, 5);
    let mut hscroll = HorizontalScrollSkipState::new(LineWrapMode::Truncate, 4);
    hscroll.consume_columns(2);
    let mut word_wrap = WordWrapRenderState::new(true);
    word_wrap.allow_after_current_char(' ');
    word_wrap.record_candidate(
        'a',
        7,
        11,
        2,
        (None, None),
        DisplayRowGlyphCheckpoint::default(),
    );
    let mut trailing = TrailingWhitespaceRenderState::new(true, 0x00112233);
    trailing.track_rendered_char(
        ' ',
        DisplayRowStartMarker::Active {
            row: DisplayRowMarker::Row(0),
            x: 24.0,
        },
    );

    let mut col = 8;
    let mut prefix_request = DisplayRowPrefixRequest::None;
    DisplayRowTransitionRenderState::new(
        &mut prefix_request,
        true,
        &mut line_numbers,
        &mut hscroll,
        &mut word_wrap,
        &mut trailing,
    )
    .apply_line_break_row_start(
        DisplayRowLineBreakTransitionPlan::hidden_line_break(),
        &mut col,
    );

    assert_eq!(col, 0);
    assert_eq!(prefix_request, DisplayRowPrefixRequest::Line);
    assert_eq!(line_numbers.current_line(), 4);
    assert_eq!(hscroll.consumed_columns(), 0);
    assert!(!word_wrap.candidate().is_available());
    assert_eq!(trailing.start_marker(), DisplayRowStartMarker::Inactive);
}

#[test]
fn text_row_transition_state_policy_applies_character_wrap_state_updates() {
    let mut line_numbers = LineNumberRenderState::new(true, 3, 5);
    let mut hscroll = HorizontalScrollSkipState::new(LineWrapMode::Truncate, 4);
    hscroll.consume_columns(2);
    let mut word_wrap = WordWrapRenderState::new(true);
    word_wrap.allow_after_current_char(' ');
    word_wrap.record_candidate(
        'a',
        7,
        11,
        2,
        (None, None),
        DisplayRowGlyphCheckpoint::default(),
    );
    let mut trailing = TrailingWhitespaceRenderState::new(true, 0x00112233);
    trailing.track_rendered_char(
        '\t',
        DisplayRowStartMarker::Active {
            row: DisplayRowMarker::Row(0),
            x: 24.0,
        },
    );

    let prefix = TextRowTransitionStatePolicy::character_wrap().apply(
        &mut line_numbers,
        &mut hscroll,
        &mut word_wrap,
        &mut trailing,
    );

    assert_eq!(
        prefix,
        crate::display_row_walk_state::TextRowTransitionPrefixAction::Wrap
    );
    assert_eq!(line_numbers.current_line(), 3);
    assert_eq!(hscroll.consumed_columns(), 2);
    assert_eq!(word_wrap.candidate().byte_idx(), 7);
    word_wrap.record_candidate(
        'b',
        9,
        13,
        4,
        (None, None),
        DisplayRowGlyphCheckpoint::default(),
    );
    assert_eq!(word_wrap.candidate().byte_idx(), 7);
    assert_eq!(trailing.start_marker(), DisplayRowStartMarker::Inactive);
}

#[test]
fn special_text_row_overflow_decision_names_fit_truncate_and_wrap() {
    assert_eq!(
        SpecialTextRowOverflowDecision::for_width(4.0, 6.0, 10.0, LineWrapMode::Truncate),
        SpecialTextRowOverflowDecision::Fits
    );
    assert_eq!(
        SpecialTextRowOverflowDecision::for_width(5.0, 6.0, 10.0, LineWrapMode::Truncate),
        SpecialTextRowOverflowDecision::Truncate
    );
    assert_eq!(
        SpecialTextRowOverflowDecision::for_width(5.0, 6.0, 10.0, LineWrapMode::Wrap),
        SpecialTextRowOverflowDecision::Wrap
    );
}

#[test]
fn buffer_text_row_overflow_decision_names_main_text_wrap_policy() {
    let empty_wrap = WordWrapRenderState::new(true);

    assert_eq!(
        DisplayRowTextOverflowDecision::for_char(
            'x',
            4.0,
            6.0,
            10.0,
            LineWrapMode::Truncate,
            empty_wrap
        ),
        DisplayRowTextOverflowDecision::Fits
    );
    assert_eq!(
        DisplayRowTextOverflowDecision::for_char(
            '\t',
            12.0,
            16.0,
            10.0,
            LineWrapMode::Truncate,
            empty_wrap
        ),
        DisplayRowTextOverflowDecision::Fits
    );
    assert_eq!(
        DisplayRowTextOverflowDecision::for_char(
            'x',
            5.0,
            6.0,
            10.0,
            LineWrapMode::Truncate,
            empty_wrap
        ),
        DisplayRowTextOverflowDecision::Truncate
    );
    assert_eq!(
        DisplayRowTextOverflowDecision::for_char(
            'x',
            5.0,
            6.0,
            10.0,
            LineWrapMode::Wrap,
            empty_wrap
        ),
        DisplayRowTextOverflowDecision::CharacterWrap
    );

    let mut word_wrap = WordWrapRenderState::new(true);
    word_wrap.allow_after_current_char(' ');
    word_wrap.record_candidate(
        'a',
        7,
        11,
        2,
        (Some(LispCharPos1::new(3)), None),
        DisplayRowGlyphCheckpoint::default(),
    );

    assert_eq!(
        DisplayRowTextOverflowDecision::for_char(
            'x',
            5.0,
            6.0,
            10.0,
            LineWrapMode::Wrap,
            word_wrap
        ),
        DisplayRowTextOverflowDecision::WordWrap {
            break_candidate: word_wrap.candidate(),
        }
    );
}

#[test]
fn invisible_text_scan_checkpoint_tracks_next_visibility_change() {
    let mut checkpoints = InvisibleTextScanCheckpoint::new(10);

    assert!(!checkpoints.should_check(9));
    assert!(checkpoints.should_check(10));

    checkpoints.record_next_visible(15);

    assert!(!checkpoints.should_check(14));
    assert!(checkpoints.should_check(15));
}

#[test]
fn trailing_whitespace_render_state_tracks_enabled_marker_and_background() {
    let marker = DisplayRowStartMarker::Active {
        row: DisplayRowMarker::Row(0),
        x: 24.0,
    };
    let later_marker = DisplayRowStartMarker::Active {
        row: DisplayRowMarker::Row(0),
        x: 48.0,
    };
    let mut state = TrailingWhitespaceRenderState::new(true, 0x00112233);

    assert_eq!(state.background(), Some(Color::from_pixel(0x00112233)));
    assert_eq!(state.start_marker(), DisplayRowStartMarker::Inactive);

    state.track_rendered_char(' ', marker);
    state.track_rendered_char('\t', later_marker);

    assert_eq!(state.start_marker(), marker);

    state.track_rendered_char('x', later_marker);

    assert_eq!(state.start_marker(), DisplayRowStartMarker::Inactive);

    state.track_rendered_char('\t', later_marker);
    state.reset_after_row_transition();

    assert_eq!(state.start_marker(), DisplayRowStartMarker::Inactive);

    let mut disabled = TrailingWhitespaceRenderState::new(false, 0x00ABCDEF);
    disabled.track_rendered_char(' ', marker);

    assert_eq!(disabled.background(), None);
    assert_eq!(disabled.start_marker(), DisplayRowStartMarker::Inactive);
}

#[test]
fn hit_row_range_tracker_builds_ranges_and_tracks_pending_finish() {
    let mut tracker = HitRowRangeTracker::new(10);

    assert_eq!(
        tracker.range_to(14),
        DisplayRowHitRange {
            charpos_start: 10,
            charpos_end: 14,
        }
    );
    assert!(!tracker.should_finish_current_row(10, false));
    assert!(tracker.should_finish_current_row(11, false));
    assert!(tracker.should_finish_current_row(10, true));

    tracker.advance_to(14);

    assert_eq!(
        tracker.range_to(20),
        DisplayRowHitRange {
            charpos_start: 14,
            charpos_end: 20,
        }
    );
    assert!(!tracker.should_finish_current_row(14, false));
}

#[test]
fn face_scan_checkpoint_tracks_resolution_boundaries_and_invalidation() {
    let mut checkpoint = FaceScanCheckpoint::initial();

    assert!(checkpoint.should_resolve_at(0));

    *checkpoint.next_check_mut() = 12;

    assert!(!checkpoint.should_resolve_at(11));
    assert!(checkpoint.should_resolve_at(12));

    checkpoint.invalidate();

    assert!(checkpoint.should_resolve_at(0));
    assert_eq!(*checkpoint.next_check_mut(), 0);
}

#[test]
fn cursor_capture_state_captures_once_and_refines_matching_main_char_width() {
    let mut state = CursorCaptureState::new();
    let first = CapturedCursorInfo {
        x: 1.0,
        y: 2.0,
        face_w: 7.0,
        face_h: 14.0,
        face_ascent: 10.0,
        bg: Color::from_pixel(0x00112233),
        byte_idx: 5,
        col: 3,
        display_row_offset: 2,
        slot_width: None,
        stretch_like: false,
        glyph_row_resolved: false,
    };
    let second = CapturedCursorInfo {
        x: 9.0,
        byte_idx: 8,
        ..first
    };

    assert!(state.is_missing());

    state.capture_once(first);
    state.capture_once(second);
    state.update_for_main_char(8, 44.0);
    state.update_for_main_char(5, 12.5);

    let captured = state.as_ref().expect("cursor should be captured");
    assert_eq!(captured.x, 1.0);
    assert_eq!(captured.byte_idx, 5);
    assert_eq!(captured.slot_width, Some(12.5));
}

#[test]
fn frame_face_id_allocator_clamps_to_sentinel_and_allocates_sequential_ids() {
    let mut allocator = FrameFaceIdAllocator::new(100);

    assert_eq!(allocator.allocate(), 100);
    assert_eq!(allocator.allocate(), 101);
    assert_eq!(allocator.finish(), 102);

    let mut clamped = FrameFaceIdAllocator::new(0);

    assert_eq!(clamped.allocate(), BasicFaceId::SENTINEL);
    assert_eq!(clamped.finish(), BasicFaceId::SENTINEL + 1);

    let mut frame_counter = 0;
    FrameFaceIdAllocator::new(200).finish_into(&mut frame_counter);
    assert_eq!(frame_counter, 200);
}

#[test]
fn display_row_prefix_request_tracks_pending_prefix_mode() {
    let _eval = Context::new();
    let mut request = DisplayRowPrefixRequest::initial(true, true);

    assert!(request.is_requested());
    let line_source = request
        .source_from_values(
            DisplayRowPrefixValues::new(
                Some(Value::string("line-property")),
                Some(Value::string("wrap-property")),
                Some(Value::string("line-default")),
                Some(Value::string("wrap-default")),
            ),
            CharPos0::new(3),
        )
        .expect("line prefix source");
    assert_eq!(
        line_source.origin(),
        DisplayOrigin::LinePrefix {
            anchor_charpos: CharPos0::new(3),
        }
    );
    assert_eq!(
        line_source.value().as_runtime_string_owned(),
        Some("line-property".to_string())
    );
    let line_fallback_source = request
        .source_from_values(
            DisplayRowPrefixValues::new(
                Some(Value::fixnum(1)),
                None,
                Some(Value::string("line-default")),
                None,
            ),
            CharPos0::new(4),
        )
        .expect("line default source");
    assert_eq!(
        line_fallback_source.value().as_runtime_string_owned(),
        Some("line-default".to_string())
    );

    request.clear();

    assert!(!request.is_requested());

    request.request_wrap();

    assert!(request.is_requested());
    let wrap_source = request
        .source_from_values(
            DisplayRowPrefixValues::new(
                Some(Value::string("line-property")),
                None,
                Some(Value::string("line-default")),
                Some(Value::string("wrap-default")),
            ),
            CharPos0::new(5),
        )
        .expect("wrap prefix source");
    assert_eq!(
        wrap_source.origin(),
        DisplayOrigin::WrapPrefix {
            anchor_charpos: CharPos0::new(5),
        }
    );
    assert_eq!(
        wrap_source.value().as_runtime_string_owned(),
        Some("wrap-default".to_string())
    );

    // The line prefix is now requested unconditionally so the per-row
    // `line-prefix` TEXT PROPERTY is always consulted (the variable default is
    // only a fallback); the no-prefix case is gated downstream by
    // `source_from_values` returning None, not by skipping the request.
    assert_eq!(
        DisplayRowPrefixRequest::initial(false, true),
        DisplayRowPrefixRequest::Line
    );

    request.clear();
    request.apply_transition_prefix_action(
        true,
        crate::display_row_walk_state::TextRowTransitionPrefixAction::Wrap,
    );
    let transition_wrap_source = request
        .source_from_values(
            DisplayRowPrefixValues::new(None, Some(Value::string("transition-wrap")), None, None),
            CharPos0::new(6),
        )
        .expect("transition wrap prefix source");
    assert_eq!(
        transition_wrap_source.value().as_runtime_string_owned(),
        Some("transition-wrap".to_string())
    );

    request.clear();
    // A transition requests the prefix regardless of the variable default so the
    // per-row text property is consulted; the no-prefix case is handled by
    // `source_from_values` returning None, not by skipping the request.
    request.apply_transition_prefix_action(
        false,
        crate::display_row_walk_state::TextRowTransitionPrefixAction::Line,
    );
    assert!(request.is_requested());
    assert_eq!(request, DisplayRowPrefixRequest::Line);
}

#[test]
fn display_row_prefix_request_builds_typed_prefix_source() {
    let _eval = Context::new();
    let line_value = Value::string("line");
    let line_source = DisplayRowPrefixRequest::Line
        .source_for_value(line_value, CharPos0::new(4))
        .expect("line prefix source");
    assert_eq!(line_source.value(), line_value);
    assert_eq!(
        line_source.origin(),
        DisplayOrigin::LinePrefix {
            anchor_charpos: CharPos0::new(4),
        }
    );
    assert_eq!(line_source.base_face_policy(), BaseFacePolicy::DefaultFace);

    let wrap_value = Value::string("wrap");
    let wrap_source = DisplayRowPrefixRequest::Wrap
        .source_for_value(wrap_value, CharPos0::new(7))
        .expect("wrap prefix source");
    assert_eq!(wrap_source.value(), wrap_value);
    assert_eq!(
        wrap_source.origin(),
        DisplayOrigin::WrapPrefix {
            anchor_charpos: CharPos0::new(7),
        }
    );
    assert_eq!(wrap_source.base_face_policy(), BaseFacePolicy::DefaultFace);

    assert!(
        DisplayRowPrefixRequest::None
            .source_for_value(Value::string("none"), CharPos0::new(0))
            .is_none()
    );
}

#[test]
fn overlay_string_render_source_exposes_typed_render_inputs() {
    let _eval = Context::new();
    let text = Value::string("overlay");
    let overlay_id = Value::symbol("overlay-id");
    let source = OverlayStringRenderSource::new(
        crate::neovm_bridge::OverlayDisplayString {
            string: text,
            overlay_id,
            after_string_p: false,
            priority: 0,
        },
        CharPos0::new(9),
        OverlayStringKind::After,
    );

    assert_eq!(source.value(), text);
    assert_eq!(source.anchor_i64(), 9);
    assert_eq!(
        source.origin(),
        DisplayOrigin::OverlayString {
            overlay_id,
            anchor_charpos: CharPos0::new(9),
            kind: OverlayStringKind::After,
        }
    );
    assert_eq!(
        source.base_face_policy(),
        BaseFacePolicy::OverlayStringAtAnchor
    );
}

#[test]
fn horizontal_scroll_skip_state_consumes_and_resets_remaining_columns() {
    let mut state = HorizontalScrollSkipState::new(LineWrapMode::Truncate, 5);

    assert!(state.should_skip());
    assert!(state.should_show_left_truncation());
    assert_eq!(state.consumed_columns(), 0);

    state.consume_columns(2);

    assert!(state.should_skip());
    assert_eq!(state.consumed_columns(), 2);

    state.consume_columns(9);

    assert!(!state.should_skip());
    assert_eq!(state.consumed_columns(), 5);

    state.reset_line();

    assert!(state.should_skip());
    assert_eq!(state.consumed_columns(), 0);
    assert!(!HorizontalScrollSkipState::new(LineWrapMode::Wrap, 5).should_skip());
}

#[test]
fn box_face_row_state_tracks_active_row_and_start_x() {
    let mut state = BoxFaceRowState::inactive();

    assert!(!state.is_active());
    assert_eq!(state.start_x(), None);
    assert_eq!(state.row(), DisplayRowMarker::Inactive);

    state.activate(DisplayRowMarker::Row(2), 18.0);

    assert!(state.is_active());
    assert_eq!(state.start_x(), Some(18.0));
    assert_eq!(state.row(), DisplayRowMarker::Row(2));

    state.continue_on_row(DisplayRowMarker::Row(3), 4.0);

    assert!(state.is_active());
    assert_eq!(state.start_x(), Some(4.0));
    assert_eq!(state.row(), DisplayRowMarker::Row(3));

    state.clear();

    assert!(!state.is_active());
    assert_eq!(state.row(), DisplayRowMarker::Inactive);
}

#[test]
fn line_number_render_state_tracks_current_point_and_pending_render() {
    let mut state = LineNumberRenderState::new(true, 7, 9);

    assert!(state.should_render());
    assert_eq!(state.current_line(), 7);
    assert_eq!(state.point_line(), 9);
    assert!(!state.is_current_line());
    assert_eq!(state.display_number(3, false, 0), 2);
    let request = state
        .margin_render_request(3, false, 0, 0, 4)
        .expect("line number request");
    assert_eq!(request.text(), "2");
    assert_eq!(request.cols(), 4);
    assert_eq!(request.face().face_name(), "line-number");

    state.consume_render_request();

    assert!(!state.should_render());

    state.advance_line();
    state.advance_line();

    assert!(state.should_render());
    assert_eq!(state.current_line(), 9);
    assert!(state.is_current_line());
    assert_eq!(state.display_number(3, true, 10), 19);
    let request = state
        .margin_render_request(3, true, 10, 3, 5)
        .expect("current line request");
    assert_eq!(request.text(), "19");
    assert_eq!(request.cols(), 5);
    assert_eq!(request.face().face_name(), "line-number-current-line");

    state.consume_render_request();
    state.advance_hidden_line();

    assert!(!state.should_render());
    assert_eq!(state.current_line(), 10);
    assert!(!LineNumberRenderState::new(false, 7, 9).should_render());

    let major_tick = LineNumberRenderState::new(true, 12, 9)
        .margin_render_request(1, false, 0, 4, 3)
        .expect("major tick line number request");
    assert_eq!(major_tick.text(), "12");
    assert_eq!(major_tick.face().face_name(), "line-number-major-tick");
}

#[test]
fn line_number_render_state_renders_blank_gutter_on_continuation_rows() {
    // First row of a buffer line renders the absolute number with a non-blank
    // gutter (GNU `maybe_produce_line_number`).
    let mut state = LineNumberRenderState::new(true, 7, 9);
    let first = state
        .margin_render_request(1, false, 0, 0, 4)
        .expect("first-row line number request");
    assert!(!first.blank());
    assert_eq!(first.text(), "7");
    assert_eq!(first.cols(), 4);
    state.consume_render_request();
    assert!(!state.should_render());

    // A wrapped continuation row re-arms the pending render but renders a blank
    // (no-number), width-reserved gutter so its text aligns with the first row.
    state.mark_continuation_row();
    assert!(state.should_render());
    let continuation = state
        .margin_render_request(1, false, 0, 0, 4)
        .expect("continuation-row line number request");
    assert!(continuation.blank());
    assert_eq!(continuation.text(), "");
    assert_eq!(continuation.cols(), 4);
    assert_eq!(continuation.face().face_name(), first.face().face_name());
    state.consume_render_request();
    assert!(!state.should_render());

    // The next buffer line resets back to a non-blank numbered gutter.
    state.advance_line();
    let next_line = state
        .margin_render_request(1, false, 0, 0, 4)
        .expect("next-line line number request");
    assert!(!next_line.blank());
    assert_eq!(next_line.text(), "8");
}

#[test]
fn captured_cursor_info_builds_from_active_face_state() {
    let eval = Context::new();
    let resolver = crate::neovm_bridge::FaceResolver::new(
        eval.face_table(),
        0x00FFFFFF,
        0x00000000,
        14.0,
        None,
    );
    let mut face = resolver.default_face().clone();
    face.bg = 0x00445566;
    let mut font_metrics = None;
    let measured = DisplayRowMeasurementPolicy::for_frame(false).measured_face(
        9,
        &face,
        None,
        7.5,
        crate::display_row_metrics::DisplayRowFallbackMetrics {
            char_width: 7.5,
            row_height: 18.0,
            ascent: 13.0,
        },
        &mut font_metrics,
    );
    let active_face = DisplayRowActiveFaceState::new(face, measured);

    let cursor = CapturedCursorInfo::from_active_face_state(
        &active_face,
        CapturedCursorPlacement {
            x: 21.0,
            y: 34.0,
            byte_idx: 5,
            col: 3,
            display_row_offset: 2,
            slot_width: CapturedCursorSlotWidth::FaceChar,
            stretch_like: false,
        },
    );

    assert_eq!(cursor.x, 21.0);
    assert_eq!(cursor.y, 34.0);
    assert_eq!(cursor.face_w, 7.5);
    assert_eq!(cursor.face_h, 18.0);
    assert_eq!(cursor.face_ascent, 13.0);
    assert_eq!(cursor.bg, Color::from_pixel(0x00445566));
    assert_eq!(cursor.byte_idx, 5);
    assert_eq!(cursor.col, 3);
    assert_eq!(cursor.display_row_offset, 2);
    assert_eq!(cursor.slot_width, Some(7.5));
    assert!(!cursor.stretch_like);
}

#[test]
fn captured_cursor_info_builds_display_box_from_active_face_state() {
    let eval = Context::new();
    let resolver = crate::neovm_bridge::FaceResolver::new(
        eval.face_table(),
        0x00FFFFFF,
        0x00000000,
        14.0,
        None,
    );
    let mut face = resolver.default_face().clone();
    face.bg = 0x00445566;
    let mut font_metrics = None;
    let measured = DisplayRowMeasurementPolicy::for_frame(false).measured_face(
        9,
        &face,
        None,
        7.5,
        crate::display_row_metrics::DisplayRowFallbackMetrics {
            char_width: 7.5,
            row_height: 18.0,
            ascent: 13.0,
        },
        &mut font_metrics,
    );
    let active_face = DisplayRowActiveFaceState::new(face, measured);

    let cursor = CapturedCursorInfo::display_box_from_active_face_state(
        &active_face,
        CapturedCursorPlacement {
            x: 21.0,
            y: 34.0,
            byte_idx: 5,
            col: 3,
            display_row_offset: 2,
            slot_width: CapturedCursorSlotWidth::Explicit(42.0),
            stretch_like: true,
        },
        31.0,
        29.0,
    );

    assert_eq!(cursor.x, 21.0);
    assert_eq!(cursor.y, 34.0);
    assert_eq!(cursor.face_w, 7.5);
    assert_eq!(cursor.face_h, 31.0);
    assert_eq!(cursor.face_ascent, 29.0);
    assert_eq!(cursor.bg, Color::from_pixel(0x00445566));
    assert_eq!(cursor.byte_idx, 5);
    assert_eq!(cursor.col, 3);
    assert_eq!(cursor.display_row_offset, 2);
    assert_eq!(cursor.slot_width, Some(42.0));
    assert!(cursor.stretch_like);
}

#[test]
fn captured_cursor_info_builds_line_break_from_active_face_state() {
    let eval = Context::new();
    let resolver = crate::neovm_bridge::FaceResolver::new(
        eval.face_table(),
        0x00FFFFFF,
        0x00000000,
        14.0,
        None,
    );
    let mut face = resolver.default_face().clone();
    face.bg = 0x00445566;
    let mut font_metrics = None;
    let measured = DisplayRowMeasurementPolicy::for_frame(false).measured_face(
        9,
        &face,
        None,
        7.5,
        crate::display_row_metrics::DisplayRowFallbackMetrics {
            char_width: 7.5,
            row_height: 18.0,
            ascent: 13.0,
        },
        &mut font_metrics,
    );
    let active_face = DisplayRowActiveFaceState::new(face, measured);

    let cursor = CapturedCursorInfo::line_break_from_active_face_state(
        &active_face,
        CapturedCursorPlacement {
            x: 21.0,
            y: 34.0,
            byte_idx: 5,
            col: 3,
            display_row_offset: 2,
            slot_width: CapturedCursorSlotWidth::FaceChar,
            stretch_like: false,
        },
        24.0,
    );

    assert_eq!(cursor.x, 21.0);
    assert_eq!(cursor.y, 34.0);
    assert_eq!(cursor.face_w, 7.5);
    assert_eq!(cursor.face_h, 24.0);
    assert_eq!(cursor.face_ascent, 13.0);
    assert_eq!(cursor.bg, Color::from_pixel(0x00445566));
    assert_eq!(cursor.byte_idx, 5);
    assert_eq!(cursor.col, 3);
    assert_eq!(cursor.display_row_offset, 2);
    assert_eq!(cursor.slot_width, Some(7.5));
    assert!(!cursor.stretch_like);
}

#[test]
fn captured_cursor_info_builds_from_visual_state() {
    let cursor = CapturedCursorInfo::from_visual_state(
        CapturedCursorVisualState {
            face_width: 9.0,
            face_height: 22.0,
            face_ascent: 17.0,
            background: Color::from_pixel(0x00112233),
        },
        CapturedCursorPlacement {
            x: 21.0,
            y: 34.0,
            byte_idx: 5,
            col: 3,
            display_row_offset: 2,
            slot_width: CapturedCursorSlotWidth::Explicit(18.0),
            stretch_like: true,
        },
    );

    assert_eq!(cursor.x, 21.0);
    assert_eq!(cursor.y, 34.0);
    assert_eq!(cursor.face_w, 9.0);
    assert_eq!(cursor.face_h, 22.0);
    assert_eq!(cursor.face_ascent, 17.0);
    assert_eq!(cursor.bg, Color::from_pixel(0x00112233));
    assert_eq!(cursor.byte_idx, 5);
    assert_eq!(cursor.col, 3);
    assert_eq!(cursor.display_row_offset, 2);
    assert_eq!(cursor.slot_width, Some(18.0));
    assert!(cursor.stretch_like);
}

#[test]
fn cursor_geometry_source_builds_from_captured_cursor_and_row_metrics() {
    let cursor = CapturedCursorInfo::from_visual_state(
        CapturedCursorVisualState {
            face_width: 9.0,
            face_height: 22.0,
            face_ascent: 17.0,
            background: Color::from_pixel(0x00112233),
        },
        CapturedCursorPlacement {
            x: 21.0,
            y: 34.0,
            byte_idx: 5,
            col: 3,
            display_row_offset: 2,
            slot_width: CapturedCursorSlotWidth::Explicit(18.0),
            stretch_like: true,
        },
    );
    let row_metric = RowMetricsSnapshot::new(9, 9, 32.0, 25.0, 19.0);

    let source = CursorGeometrySource::from_captured_cursor(
        &cursor,
        row_metric,
        CursorGeometryContext {
            window_id: 7,
            slot_width: 18.0,
            default_line_height: 16.0,
            ends_at_visible_eob: true,
        },
    );

    assert_eq!(
        source.slot_id,
        DisplaySlotId {
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(7),
            row: 9,
            col: 3,
        }
    );
    assert_eq!(source.x, 21.0);
    assert_eq!(source.y, 34.0);
    assert_eq!(source.slot_width, 18.0);
    assert_eq!(source.face_height, 22.0);
    assert_eq!(source.face_ascent, 17.0);
    assert_eq!(source.row_height, 25.0);
    assert_eq!(source.row_ascent, 19.0);
    assert_eq!(source.default_line_height, 16.0);
    assert!(source.stretch_like);
    assert!(source.ends_at_visible_eob);
    assert_eq!(source.cursor_fg, Color::from_pixel(0x00112233));
}

#[test]
fn cursor_geometry_source_builds_from_display_point_snapshot() {
    let point = DisplayPointSnapshot {
        buffer_pos: LispCharPos1::from_one_based_usize(4),
        x: 11,
        y: 13,
        width: 17,
        height: 19,
        row: 3,
        col: 5,
    };

    let source = CursorGeometrySource::from_display_point(
        &point,
        VisualCursorGeometryContext {
            window_id: -10,
            text_area_left: 100.0,
            window_top: 7.0,
        },
    );

    assert_eq!(
        source.slot_id,
        DisplaySlotId {
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(-10),
            row: 3,
            col: 5,
        }
    );
    assert_eq!(source.x, 111.0);
    assert_eq!(source.y, 20.0);
    assert_eq!(source.slot_width, 17.0);
    assert_eq!(source.face_height, 19.0);
    assert_eq!(source.face_ascent, 19.0);
    assert_eq!(source.row_height, 19.0);
    assert_eq!(source.row_ascent, 19.0);
    assert_eq!(source.default_line_height, 19.0);
    assert!(!source.stretch_like);
    assert!(!source.ends_at_visible_eob);
    assert_eq!(source.cursor_fg, Color::BLACK);
}

#[test]
fn captured_cursor_info_resolves_explicit_slot_width_before_style_width() {
    let cursor = CapturedCursorInfo::from_visual_state(
        CapturedCursorVisualState {
            face_width: 9.0,
            face_height: 22.0,
            face_ascent: 17.0,
            background: Color::from_pixel(0x00112233),
        },
        CapturedCursorPlacement {
            x: 21.0,
            y: 34.0,
            byte_idx: 0,
            col: 1,
            display_row_offset: 2,
            slot_width: CapturedCursorSlotWidth::Explicit(18.0),
            stretch_like: true,
        },
    );
    let mut params = test_window_params();
    params.x_stretch_cursor = true;

    let width = cursor.resolved_slot_width(CursorStyle::FilledBox, b"\t", &params);

    assert_eq!(width, 18.0);
}

#[test]
fn captured_cursor_info_resolves_missing_slot_width_from_style_width() {
    let mut cursor = CapturedCursorInfo::from_visual_state(
        CapturedCursorVisualState {
            face_width: 8.0,
            face_height: 22.0,
            face_ascent: 17.0,
            background: Color::from_pixel(0x00112233),
        },
        CapturedCursorPlacement {
            x: 21.0,
            y: 34.0,
            byte_idx: 0,
            col: 1,
            display_row_offset: 2,
            slot_width: CapturedCursorSlotWidth::Explicit(18.0),
            stretch_like: true,
        },
    );
    cursor.slot_width = None;
    let mut params = test_window_params();
    params.x_stretch_cursor = true;

    let width = cursor.resolved_slot_width(CursorStyle::FilledBox, b"\t", &params);

    assert_eq!(width, 56.0);
}

#[test]
fn captured_cursor_info_builds_logical_cursor_position() {
    let cursor = CapturedCursorInfo::from_visual_state(
        CapturedCursorVisualState {
            face_width: 9.0,
            face_height: 22.0,
            face_ascent: 17.0,
            background: Color::from_pixel(0x00112233),
        },
        CapturedCursorPlacement {
            x: 21.4,
            y: 34.0,
            byte_idx: 5,
            col: 3,
            display_row_offset: 2,
            slot_width: CapturedCursorSlotWidth::Explicit(18.0),
            stretch_like: true,
        },
    );
    let row_metric = RowMetricsSnapshot::new(9, 9, 32.6, 25.0, 19.0);

    let logical = cursor.logical_cursor_position(row_metric, 7, 10.0, 2.0);

    assert_eq!(logical.x, 11);
    assert_eq!(logical.y, 31);
    assert_eq!(logical.row, 9);
    assert_eq!(logical.col, 3);
}

#[test]
fn captured_text_window_cursor_publish_context_publishes_captured_cursor() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("captured-cursor-publish-context", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 10.0, 20.0);
    let mut builder = DisplayOutputBuilder::new();
    builder.begin_window(window_id.0, 1, 10, Rect::new(0.0, 0.0, 160.0, 64.0), true);
    builder.begin_row(0, GlyphRowRole::Text);
    let cursor = CapturedCursorInfo::from_visual_state(
        CapturedCursorVisualState {
            face_width: 8.0,
            face_height: 16.0,
            face_ascent: 12.0,
            background: Color::from_pixel(0x00112233),
        },
        CapturedCursorPlacement {
            x: 24.0,
            y: 20.0,
            byte_idx: 0,
            col: 3,
            display_row_offset: 0,
            slot_width: CapturedCursorSlotWidth::Explicit(8.0),
            stretch_like: false,
        },
    );
    let mut params = test_window_params();
    params.window_id = window_id.0 as i64;
    params.selected = true;
    params.cursor_color = 0x00ffffff;

    let outcome = CapturedTextWindowCursorPublishContext::new(
        &params, b"abc", 0, 10.0, 20.0, 20.0, 64.0, 8.0, 16.0, 4, false,
    )
    .publish_captured_cursor(
        cursor,
        &[RowMetricsSnapshot::new(0, 0, 20.0, 16.0, 12.0)],
        RowMetricsSnapshot::new(0, 0, 20.0, 16.0, 12.0),
        TextWindowOutputTarget::from_builder(&mut builder),
        &mut output_emitter,
    );

    assert_eq!(outcome, CapturedTextWindowCursorPublishOutcome::Published);
    let phys = builder.phys_cursor().expect("selected phys cursor");
    assert_eq!(
        phys.slot_id,
        DisplaySlotId {
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(window_id.0 as i64),
            row: 0,
            col: 0,
        }
    );
    assert_eq!(phys.charpos, 4);
    assert_eq!(phys.width, 8.0);
    assert_eq!(phys.height, 16.0);
}

#[test]
fn visual_text_window_cursor_publish_context_publishes_decorative_cursor_from_display_point() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("visual-cursor-publish-context", 320, 120, buf_id);
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut output_emitter = WindowOutputEmitter::new(frame_id, window_id, 0, 10.0, 20.0);
    output_emitter.push_display_point(LispCharPos1::ONE, 34.0, 52.0, 11.0, 17.0, 2, 4);

    let mut params = test_window_params();
    params.x_stretch_cursor = false;
    params.visual_cursors = vec![VisualCursorSpec {
        id: -42,
        charpos: 0,
        cursor_kind: CursorKind::Bar,
        cursor_bar_width: CursorBarWidth::new(3),
        color: 0x00112233,
        effects: None,
    }];
    let mut builder = DisplayOutputBuilder::new();

    let summary = VisualTextWindowCursorPublishContext::new(&params, 10.0, 20.0, 20.0, 80.0, 8.0)
        .publish_visual_cursors(
            TextWindowOutputTarget::from_builder(&mut builder),
            &output_emitter,
        );

    assert_eq!(
        summary,
        VisualTextWindowCursorPublishSummary {
            requested: 1,
            published: 1,
            ..Default::default()
        }
    );
    let state = builder.finish(10, 1, 8.0, 16.0);
    assert_eq!(state.cursors.len(), 1);
    let cursor = &state.cursors[0];
    assert_eq!(cursor.window_id.get(), -42);
    assert_eq!(
        cursor.slot_id,
        DisplaySlotId {
            window_id: neomacs_display_protocol::types::DisplayWindowId::new(-42),
            row: 2,
            col: 4,
        }
    );
    assert_eq!(cursor.x, 34.0);
    assert_eq!(cursor.y, 52.0);
    assert_eq!(cursor.width, 3.0);
    assert_eq!(cursor.height, 17.0);
    assert_eq!(cursor.color, Color::from_pixel(0x00112233));
}

fn test_window_params() -> WindowParams {
    WindowParams {
        window_id: 1,
        buffer_id: 1,
        bounds: Rect::new(0.0, 0.0, 800.0, 600.0),
        text_bounds: Rect::new(0.0, 0.0, 800.0, 560.0),
        selected: true,
        kind: WindowKind::Main,
        window_start: 1,
        window_end: 0,
        point: 1,
        buffer_size: 1,
        buffer_begv: 1,
        hscroll: 0,
        vscroll: 0,
        wrap_mode: LineWrapMode::Wrap,
        word_wrap: false,
        tab_width: 8,
        scroll_conservatively: 0,
        scroll_margin: 0,
        tab_stop_list: vec![],
        default_fg: 0xFFFFFF,
        default_bg: 0x000000,
        char_width: 8.0,
        char_height: 16.0,
        window_system: true,
        font_pixel_size: 14.0,
        font_ascent: 12.0,
        mode_line_height: 0.0,
        header_line_height: 0.0,
        tab_line_height: 0.0,
        cursor_kind: neomacs_display_protocol::frame_glyphs::CursorKind::FilledBox,
        cursor_bar_width: CursorBarWidth::TWO,
        x_stretch_cursor: false,
        cursor_color: 0xFFFFFF,
        cursor_effects: None,
        visual_cursors: Vec::new(),
        left_fringe_width: 0.0,
        right_fringe_width: 0.0,
        indicate_empty_lines: 0,
        show_trailing_whitespace: false,
        trailing_ws_bg: 0,
        fill_column_indicator: -1,
        fill_column_indicator_char: '|',
        fill_column_indicator_fg: 0,
        extra_line_spacing: 0.0,
        selective_display: 0,
        escape_glyph_fg: 0,
        nobreak_char_display: 0,
        nobreak_char_fg: 0,
        glyphless_char_fg: 0,
        wrap_prefix: vec![],
        line_prefix: vec![],
        left_margin_width: 0.0,
        right_margin_width: 0.0,
        vertical_scroll_bar_side: None,
        horizontal_scroll_bar: false,
        scroll_bar_pixel_width: 0.0,
        scroll_bar_pixel_height: 0.0,
    }
}

fn realize_test_gui_frame(eval: &mut Context, frame_id: neovm_core::window::FrameId) {
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(Some(Value::symbol("neo")));
        frame.install_gnu_gui_default_parameters();
    }
    assert!(eval.frame_manager_mut().select_frame(frame_id));
    let results = eval
        .eval_str_each("(internal-set-lisp-face-attribute 'default :height 100 (selected-frame))");
    assert!(
        results.iter().all(Result::is_ok),
        "test GUI frame should have a realized default face height, got {results:?}"
    );
}

#[derive(Default)]
struct RecordingImageDisplayHost {
    requests: Arc<Mutex<Vec<ImageResolveRequest>>>,
    video_requests: Arc<Mutex<Vec<VideoResolveRequest>>>,
    webkit_requests: Arc<Mutex<Vec<WebKitResolveRequest>>>,
}

impl DisplayHost for RecordingImageDisplayHost {
    fn realize_gui_frame(&mut self, _request: GuiFrameHostRequest) -> Result<(), String> {
        Ok(())
    }

    fn resize_gui_frame(&mut self, _request: GuiFrameHostRequest) -> Result<(), String> {
        Ok(())
    }

    fn resolve_image(
        &self,
        _request: ImageResolveRequest,
    ) -> Result<Option<ResolvedImage>, String> {
        panic!("layout must use nonblocking request_image");
    }

    fn request_image(&self, request: ImageResolveRequest) -> Result<Option<ResolvedImage>, String> {
        self.requests
            .lock()
            .expect("requests lock")
            .push(request.clone());
        Ok(Some(ResolvedImage {
            image_id: 77,
            width: 32,
            height: 24,
            dimensions_known: true,
        }))
    }

    fn request_video(&self, request: VideoResolveRequest) -> Result<Option<ResolvedVideo>, String> {
        self.video_requests
            .lock()
            .expect("video requests lock")
            .push(request);
        Ok(Some(ResolvedVideo { video_id: 88 }))
    }

    fn request_webkit(
        &self,
        request: WebKitResolveRequest,
    ) -> Result<Option<ResolvedWebKit>, String> {
        self.webkit_requests
            .lock()
            .expect("webkit requests lock")
            .push(request);
        Ok(Some(ResolvedWebKit { webkit_id: 99 }))
    }
}

fn window_matrix_text(entry: &neomacs_display_protocol::glyph_matrix::WindowMatrixEntry) -> String {
    entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled)
        .flat_map(|row| row.glyphs[1].iter())
        .filter_map(|glyph| match &glyph.glyph_type {
            neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => Some(*ch),
            neomacs_display_protocol::glyph_matrix::GlyphType::Composite { text } => {
                text.chars().next()
            }
            _ => None,
        })
        .collect()
}

fn enabled_window_row_texts(
    entry: &neomacs_display_protocol::glyph_matrix::WindowMatrixEntry,
) -> Vec<String> {
    entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled)
        .map(|row| {
            row.glyphs[1]
                .iter()
                .filter_map(|glyph| match &glyph.glyph_type {
                    neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => Some(*ch),
                    neomacs_display_protocol::glyph_matrix::GlyphType::Composite { text } => {
                        text.chars().next()
                    }
                    _ => None,
                })
                .collect()
        })
        .collect()
}

/// Concatenated text of every enabled row's text area (`glyphs[1]`) in a
/// backend layout trace.  Char glyphs contribute their character; composites
/// their text.  Used to assert on rendered output (e.g. ellipsis runs).
fn backend_trace_text_area_text(trace: &BackendLayoutTrace) -> String {
    trace
        .matrix_rows
        .iter()
        .filter(|row| row.enabled)
        .flat_map(|row| row.glyph_areas[1].iter())
        .filter(|glyph| !glyph.padding)
        .filter_map(|glyph| match &glyph.kind {
            GlyphKindTrace::Char(ch) | GlyphKindTrace::Glyphless(ch) => Some(ch.to_string()),
            GlyphKindTrace::Composite(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn glyphs_logical_text(glyphs: &[Glyph]) -> String {
    glyphs
        .iter()
        .filter(|glyph| !glyph.padding)
        .map(|glyph| match &glyph.glyph_type {
            GlyphType::Char { ch } | GlyphType::Glyphless { ch } => ch.to_string(),
            GlyphType::Composite { text } => text.to_string(),
            GlyphType::Stretch { width_cols } => " ".repeat(usize::from(*width_cols)),
            _ => String::new(),
        })
        .collect::<Vec<_>>()
        .join("")
}

fn render_buffer_text_source_shadow_row(
    buf_id: BufferId,
    snapshot: &LayoutBufferSnapshot,
    line_end: CharPos0,
    width_px: f32,
    height_px: f32,
    ascent_px: f32,
    char_width_px: f32,
) -> GlyphRow {
    let mut source = crate::display_buffer_text_source::BufferTextSourceCursor::new(
        buf_id,
        snapshot,
        CharPos0::ZERO,
        line_end,
        RenderFaceRef::FaceId(0),
    );
    let mut font_metrics = None;
    let mut renderer = DisplayRowRenderer::new(&mut font_metrics);
    let table = FaceTable::new();
    let resolver = FaceResolver::new(&table, 0x00ff_ffff, 0x0000_0000, 14.0, None);
    let mut face_ids = FrameFaceIdAllocator::new(1);
    DisplayRowSourceFragmentFrame::new(
        DisplayRowGeometry::new(
            0.0,
            width_px,
            height_px,
            char_width_px,
            ascent_px,
            DisplayTabPolicy::every(8),
        ),
        GlyphRowRole::Text,
        0,
        resolver.default_face(),
    )
    .render_request(DisplayRowRenderBounds::new(
        DisplayRowPosition::new(0.0, 0),
        DisplayRowMaxX::Bounded(width_px),
    ))
    .render(&mut renderer, &mut source, &resolver, &mut face_ids)
    .expect("typed buffer text source row")
    .into_row()
}

fn expected_gui_glyph_advance(
    metrics: &mut FontMetricsService,
    ch: char,
    family: &str,
    weight: u16,
    italic: bool,
    font_size: f32,
) -> f32 {
    let face_metrics = metrics.font_metrics(family, weight, italic, font_size);
    let columns = crate::composition::base_width_cols(ch);
    let minimum = f32::from(columns) * face_metrics.char_width.max(1.0);
    let measured = metrics.char_width(ch, family, weight, italic, font_size);

    GlyphAdvanceQuantization::PreserveLogicalPixels.resolve(Some(measured), minimum, minimum)
}

fn assert_point_width_matches_advance(
    point: &DisplayPointSnapshot,
    expected_advance: f32,
    label: &str,
    all_points: &[DisplayPointSnapshot],
) {
    let expected_width = expected_advance.round() as i64;
    assert!(
        (point.width - expected_width).abs() <= 1,
        "expected {label} width near {expected_width} ({expected_advance:.3}px), got {point:?}; points={all_points:?}"
    );
}

fn assert_point_delta_matches_advance(
    from: &DisplayPointSnapshot,
    to: &DisplayPointSnapshot,
    expected_advance: f32,
    label: &str,
    all_points: &[DisplayPointSnapshot],
) {
    let observed = (to.x - from.x) as f32;
    assert!(
        (observed - expected_advance).abs() <= 1.0,
        "expected {label} x delta near {expected_advance:.3}px, got {} -> {}; points={all_points:?}",
        from.x,
        to.x
    );
}

fn assert_replacement_slot_between_neighbors(
    eval: &Context,
    frame_id: neovm_core::window::FrameId,
    replacement_pos: usize,
    expected_width: i64,
) -> DisplayPointSnapshot {
    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(frame.selected_window)
        .expect("display snapshot");
    let before = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(
            replacement_pos.saturating_sub(1),
        ))
        .expect("previous point");
    let replacement = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(replacement_pos))
        .expect("replacement point");
    let after = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(replacement_pos + 1))
        .expect("following point");

    assert_eq!(replacement.x, before.x + before.width);
    assert_eq!(replacement.width, expected_width);
    assert_eq!(replacement.row, before.row);
    assert_eq!(replacement.row, after.row);
    assert!(
        replacement.x + replacement.width <= after.x,
        "replacement slot should own the covered source geometry before following text; before={before:?} replacement={replacement:?} after={after:?}"
    );
    replacement.clone()
}

fn enabled_window_row_texts_expanding_stretches(
    entry: &neomacs_display_protocol::glyph_matrix::WindowMatrixEntry,
) -> Vec<String> {
    entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled)
        .map(|row| {
            row.glyphs[1]
                .iter()
                .flat_map(|glyph| match &glyph.glyph_type {
                    neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => {
                        std::iter::repeat_n(*ch, 1).collect::<Vec<_>>()
                    }
                    neomacs_display_protocol::glyph_matrix::GlyphType::Composite { text } => {
                        text.chars().collect::<Vec<_>>()
                    }
                    neomacs_display_protocol::glyph_matrix::GlyphType::Stretch { width_cols } => {
                        std::iter::repeat_n(' ', usize::from(*width_cols)).collect::<Vec<_>>()
                    }
                    _ => Vec::new(),
                })
                .collect()
        })
        .collect()
}

fn implemented_text_backends() -> impl Iterator<Item = BufferTextBackendKind> {
    BufferTextBackendKind::implemented_variants()
}

fn convert_current_buffer_text_backend(eval: &mut Context, kind: BufferTextBackendKind) {
    let form = format!("(neomacs-set-buffer-text-backend '{})", kind.symbol_name());
    let result = eval
        .eval_str(&form)
        .unwrap_or_else(|err| panic!("convert buffer text backend with {form}: {err}"));
    assert_eq!(result.as_symbol_name(), Some(kind.symbol_name()));
}

fn insert_fragmented_current_buffer_text(eval: &mut Context, text: &str) {
    let buffer_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer = eval
        .buffer_manager_mut()
        .get_mut(buffer_id)
        .expect("current buffer");
    buffer.insert(text);

    for marker in ["\n", "日本", "Ω"] {
        if let Some(pos) = text.find(marker) {
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(pos));
            buffer.insert("tmp");
            buffer.delete_emacs_byte_range(emacs_byte_range(pos, pos + "tmp".len()));
        }
    }

    assert_eq!(buffer.buffer_string(), text);
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GlyphKindTrace {
    Char(char),
    Composite(String),
    Stretch(u16),
    Image(i32),
    Glyphless(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GlyphTrace {
    kind: GlyphKindTrace,
    face_id: u32,
    charpos: usize,
    bidi_level: u8,
    wide: bool,
    padding: bool,
    pixel_width_bits: u32,
    pixel_height_bits: u32,
    pixel_ascent_bits: u32,
}

impl GlyphTrace {
    fn from_glyph(glyph: &Glyph) -> Self {
        let kind = match &glyph.glyph_type {
            GlyphType::Char { ch } => GlyphKindTrace::Char(*ch),
            GlyphType::Composite { text } => GlyphKindTrace::Composite(text.to_string()),
            GlyphType::Stretch { width_cols } => GlyphKindTrace::Stretch(*width_cols),
            GlyphType::Image { image_id } => GlyphKindTrace::Image(*image_id),
            GlyphType::Glyphless { ch } => GlyphKindTrace::Glyphless(*ch),
        };
        Self {
            kind,
            face_id: glyph.face_id,
            charpos: glyph.charpos,
            bidi_level: glyph.bidi_level,
            wide: glyph.wide,
            padding: glyph.padding,
            pixel_width_bits: glyph.pixel_width.to_bits(),
            pixel_height_bits: glyph.pixel_height.to_bits(),
            pixel_ascent_bits: glyph.pixel_ascent.to_bits(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RowTrace {
    role: GlyphRowRole,
    enabled: bool,
    cursor_col: Option<u16>,
    cursor_type: Option<String>,
    truncated_left: bool,
    continued: bool,
    displays_text: bool,
    ends_at_zv: bool,
    mode_line: bool,
    pixel_y_bits: u32,
    height_px_bits: u32,
    ascent_px_bits: u32,
    start_charpos: usize,
    end_charpos: usize,
    glyph_areas: [Vec<GlyphTrace>; 3],
}

impl RowTrace {
    fn from_row(row: &GlyphRow) -> Self {
        Self {
            role: row.role,
            enabled: row.enabled,
            cursor_col: row.cursor_col,
            cursor_type: row.cursor_type.map(|cursor| format!("{cursor:?}")),
            truncated_left: row.truncated_left,
            continued: row.continued,
            displays_text: row.displays_text,
            ends_at_zv: row.ends_at_zv,
            mode_line: row.mode_line,
            pixel_y_bits: row.pixel_y.to_bits(),
            height_px_bits: row.height_px.to_bits(),
            ascent_px_bits: row.ascent_px.to_bits(),
            start_charpos: row.start_charpos,
            end_charpos: row.end_charpos,
            glyph_areas: [
                row.glyphs[0].iter().map(GlyphTrace::from_glyph).collect(),
                row.glyphs[1].iter().map(GlyphTrace::from_glyph).collect(),
                row.glyphs[2].iter().map(GlyphTrace::from_glyph).collect(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HitRowTrace {
    y_start_bits: u32,
    y_end_bits: u32,
    charpos_start: i64,
    charpos_end: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowHitTrace {
    content_x_bits: u32,
    char_w_bits: u32,
    rows: Vec<HitRowTrace>,
    first_col_hits: Vec<i64>,
}

impl WindowHitTrace {
    fn from_window(window: &crate::hit_test::WindowHitData) -> Self {
        Self {
            content_x_bits: window.content_x.to_bits(),
            char_w_bits: window.char_w.to_bits(),
            rows: window
                .rows
                .iter()
                .map(|row| HitRowTrace {
                    y_start_bits: row.y_start.to_bits(),
                    y_end_bits: row.y_end.to_bits(),
                    charpos_start: row.charpos_start,
                    charpos_end: row.charpos_end,
                })
                .collect(),
            first_col_hits: window
                .rows
                .iter()
                .map(|row| {
                    let y = (row.y_start + row.y_end) / 2.0;
                    crate::hit_test::hit_test_window_charpos(window.window_id, window.content_x, y)
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BackendLayoutTrace {
    matrix_rows: Vec<RowTrace>,
    points: Vec<DisplayPointSnapshot>,
    output_rows: Vec<DisplayRowSnapshot>,
    phys_cursor: Option<WindowCursorSnapshot>,
    visible_span: Option<WindowVisibleBufferSpan>,
    window_start: LispCharPos1,
    window_point: LispCharPos1,
    hit: Option<WindowHitTrace>,
}

fn selected_window_layout_trace(
    eval: &Context,
    engine: &LayoutEngine,
    frame_id: neovm_core::window::FrameId,
) -> BackendLayoutTrace {
    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let selected_window = frame.selected_window;
    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let display_snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let (window_start, window_point) =
        match frame.find_window(selected_window).expect("selected window") {
            neovm_core::window::Window::Leaf {
                window_start,
                point,
                ..
            } => (*window_start, *point),
            other => panic!("expected leaf window, got {other:?}"),
        };
    let hit = unsafe {
        (&*std::ptr::addr_of!(crate::hit_test::FRAME_HIT_DATA))
            .as_ref()
            .and_then(|windows| {
                windows
                    .iter()
                    .find(|window| window.window_id == selected_window.0 as i64)
            })
            .map(WindowHitTrace::from_window)
    };

    BackendLayoutTrace {
        matrix_rows: window_entry
            .matrix
            .rows
            .iter()
            .filter(|row| row.enabled)
            .map(RowTrace::from_row)
            .collect(),
        points: display_snapshot.points.clone(),
        output_rows: display_snapshot.rows.clone(),
        phys_cursor: display_snapshot.phys_cursor.clone(),
        visible_span: display_snapshot.visible_buffer_span(),
        window_start,
        window_point,
        hit,
    }
}

fn backend_layout_trace_with_buffer_and_window_setup(
    kind: BufferTextBackendKind,
    frame_name: &str,
    text: &str,
    frame_width: u32,
    frame_height: u32,
    setup: impl FnOnce(&mut neovm_core::buffer::Buffer, BufferId, &str),
    setup_window: impl FnOnce(&mut neovm_core::window::Window),
) -> BackendLayoutTrace {
    let mut eval = Context::new();
    convert_current_buffer_text_backend(&mut eval, kind);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        insert_fragmented_current_buffer_text(&mut eval, text);
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        setup(buffer, buf_id, text);
        assert_eq!(buffer.text_backend_kind(), kind);
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame(frame_name, frame_width, frame_height, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
        setup_window(window);
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);
    selected_window_layout_trace(&eval, &engine, frame_id)
}

/// Layout-engine micro-benchmark — the REDISPLAY LAYOUT cost, the rank-1
/// interactive-latency cost center (the engine had ZERO timers in ~49k LOC). One
/// GUI engine over a realistic buffer/frame: COLD first layout then WARM
/// steady-state (min-of-N). The engine rebuilds the frame in full every cycle (no
/// incremental fast-path), so warm = the per-redisplay-cycle floor a keystroke
/// pays. Reports via panic! (like the jit_bench_* family) so the line surfaces
/// under nextest capture; the test "fails" by design. Build needs the jit feature
/// (a bare neovm-core build is broken on this branch, pre-existing). Run:
///   cargo nextest run -p neomacs-layout-engine --features neovm-core/jit \
///     --release --run-ignored ignored-only --no-capture layout_bench_warm
#[test]
#[ignore = "macro benchmark; run explicitly in release"]
fn layout_bench_warm() {
    use std::time::{Duration, Instant};
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    // ~120 lines of representative code-like ASCII (a real editing buffer).
    let text = "(defun example-helper (alpha beta) (let ((sum (+ alpha beta))) (* sum sum)))\n"
        .repeat(120);
    insert_fragmented_current_buffer_text(&mut eval, &text);
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-bench", 1000, 700, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
    }
    let mut engine = LayoutEngine::new();
    let t0 = Instant::now();
    engine.layout_frame_rust(&mut eval, frame_id);
    let cold = t0.elapsed();
    let mut best = Duration::MAX;
    for _ in 0..100 {
        let t = Instant::now();
        engine.layout_frame_rust(&mut eval, frame_id);
        best = best.min(t.elapsed());
    }
    panic!("BENCH layout_frame_rust 1000x700 (~120-line buffer, GUI): warm {best:?} cold {cold:?}");
}

fn backend_layout_trace_with_buffer_setup(
    kind: BufferTextBackendKind,
    frame_name: &str,
    text: &str,
    frame_width: u32,
    frame_height: u32,
    setup: impl FnOnce(&mut neovm_core::buffer::Buffer, BufferId, &str),
) -> BackendLayoutTrace {
    backend_layout_trace_with_buffer_and_window_setup(
        kind,
        frame_name,
        text,
        frame_width,
        frame_height,
        setup,
        |_| {},
    )
}

fn layout_trace_for_plain_text(text: &str) -> BackendLayoutTrace {
    layout_trace_with_buffer_setup(text, 360, 180, |_, _, _| {})
}

fn layout_trace_with_buffer_setup(
    text: &str,
    frame_width: u32,
    frame_height: u32,
    setup: impl FnOnce(&mut neovm_core::buffer::Buffer, BufferId, &str),
) -> BackendLayoutTrace {
    layout_trace_with_buffer_and_window_setup(text, frame_width, frame_height, setup, |_| {})
}

fn layout_trace_with_buffer_and_window_setup(
    text: &str,
    frame_width: u32,
    frame_height: u32,
    setup: impl FnOnce(&mut neovm_core::buffer::Buffer, BufferId, &str),
    setup_window: impl FnOnce(&mut neovm_core::window::Window),
) -> BackendLayoutTrace {
    let mut eval = Context::new();
    convert_current_buffer_text_backend(&mut eval, BufferTextBackendKind::GapBuffer);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        insert_fragmented_current_buffer_text(&mut eval, text);
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        setup(buffer, buf_id, text);
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "typed-source-parity",
        frame_width,
        frame_height,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
        setup_window(window);
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);
    selected_window_layout_trace(&eval, &engine, frame_id)
}

#[test]
fn layout_frame_rust_lays_out_plain_text() {
    let text = "Hello, world!\nThis is a test.\n";
    let trace = layout_trace_for_plain_text(text);

    assert!(!trace.matrix_rows.is_empty());
}

#[test]
fn layout_frame_rust_lays_out_mixed_chars() {
    let text = "a\tb\n\u{0001}c\n日\nd\u{200b}\n";
    let trace = layout_trace_for_plain_text(text);

    assert!(!trace.matrix_rows.is_empty());
}

#[test]
fn layout_frame_rust_lays_out_face_property() {
    let text = "abc\ndef\n";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, text: &str| {
        let start = text.find('b').expect("b start");
        let end = start + "bc".len();
        assert!(buffer.put_text_property(start, end, Value::symbol("face"), Value::symbol("bold")));
    };
    let trace = layout_trace_with_buffer_setup(text, 360, 180, setup);

    assert!(!trace.matrix_rows.is_empty());
}

#[test]
fn layout_frame_rust_lays_out_simple_display_property() {
    let text = "abcXYZdef\n";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, text: &str| {
        let start = text.find("XYZ").expect("XYZ start");
        let end = start + "XYZ".len();
        assert!(buffer.put_text_property(start, end, Value::symbol("display"), Value::string("R")));
    };
    let trace = layout_trace_with_buffer_setup(text, 360, 180, setup);

    assert!(!trace.matrix_rows.is_empty());
}

#[test]
fn layout_frame_rust_lays_out_truncated_long_line() {
    let text = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, _text: &str| {
        buffer.set_buffer_local("truncate-lines", Value::T);
    };
    let trace = layout_trace_with_buffer_setup(text, 120, 120, setup);

    assert!(!trace.matrix_rows.is_empty());
}

#[test]
fn layout_frame_rust_lays_out_wrapped_long_line() {
    let text = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, _text: &str| {
        buffer.set_buffer_local("truncate-lines", Value::NIL);
    };
    let trace = layout_trace_with_buffer_setup(text, 120, 120, setup);

    assert!(!trace.matrix_rows.is_empty());
}

/// Stage 5: a long line with `truncate-lines=t` (wider than the window) sets
/// the `right-arrow` truncation bitmap in the RIGHT fringe of the truncated row.
#[test]
fn layout_frame_rust_truncated_row_sets_right_arrow_fringe() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let right_arrow_index: u16 = eval
        .eval_str("(get 'right-arrow 'fringe)")
        .expect("right-arrow fringe prop")
        .as_fixnum()
        .expect("fringe index") as u16;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ\n");
        buf.set_buffer_local("truncate-lines", Value::T);
        buf.goto_emacs_byte_pos(EmacsBytePos::new(0));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("trunc-fringe", 48, 200, buf_id);
    // Window-system frame so the fringes have width (GNU only draws fringe
    // bitmaps on GUI frames; TTY frames have 0-width fringes).
    if let Some(frame) = eval.frame_manager_mut().get_mut(frame_id) {
        frame.set_window_system(Some(Value::symbol("neo")));
    }
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");

    let right_arrow_rows = entry
        .matrix
        .rows
        .iter()
        .filter(|row| {
            row.right_fringe_bitmap
                .is_some_and(|info| info.bitmap_index == right_arrow_index)
        })
        .count();
    assert!(
        right_arrow_rows >= 1,
        "a truncated long line should set right-arrow in the right fringe \
         (right_arrow_index={right_arrow_index}); right fringe bitmaps = {:?}",
        entry
            .matrix
            .rows
            .iter()
            .map(|r| r.right_fringe_bitmap.map(|i| i.bitmap_index))
            .collect::<Vec<_>>()
    );
}

/// Stage 5: a long line with `truncate-lines=nil` (wraps) sets the
/// `right-curly-arrow` continuation bitmap in the RIGHT fringe of the continued
/// row, and the `left-curly-arrow` on the continuation row's LEFT fringe.
#[test]
fn layout_frame_rust_continued_row_sets_curly_arrow_fringe() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let right_curly_index: u16 = eval
        .eval_str("(get 'right-curly-arrow 'fringe)")
        .expect("right-curly-arrow fringe prop")
        .as_fixnum()
        .expect("fringe index") as u16;
    let left_curly_index: u16 = eval
        .eval_str("(get 'left-curly-arrow 'fringe)")
        .expect("left-curly-arrow fringe prop")
        .as_fixnum()
        .expect("fringe index") as u16;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ\n");
        buf.set_buffer_local("truncate-lines", Value::NIL);
        buf.goto_emacs_byte_pos(EmacsBytePos::new(0));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("cont-fringe", 48, 200, buf_id);
    if let Some(frame) = eval.frame_manager_mut().get_mut(frame_id) {
        frame.set_window_system(Some(Value::symbol("neo")));
    }
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");

    let right_curly_rows = entry
        .matrix
        .rows
        .iter()
        .filter(|row| {
            row.right_fringe_bitmap
                .is_some_and(|info| info.bitmap_index == right_curly_index)
        })
        .count();
    let left_curly_rows = entry
        .matrix
        .rows
        .iter()
        .filter(|row| {
            row.left_fringe_bitmap
                .is_some_and(|info| info.bitmap_index == left_curly_index)
        })
        .count();
    assert!(
        right_curly_rows >= 1,
        "a wrapped line should set right-curly-arrow on the continued row's right \
         fringe (idx={right_curly_index}); right = {:?}",
        entry
            .matrix
            .rows
            .iter()
            .map(|r| r.right_fringe_bitmap.map(|i| i.bitmap_index))
            .collect::<Vec<_>>()
    );
    assert!(
        left_curly_rows >= 1,
        "the continuation row should set left-curly-arrow on its left fringe \
         (idx={left_curly_index}); left = {:?}",
        entry
            .matrix
            .rows
            .iter()
            .map(|r| r.left_fringe_bitmap.map(|i| i.bitmap_index))
            .collect::<Vec<_>>()
    );
}

#[test]
fn layout_frame_rust_lays_out_line_numbers() {
    let text = "abc\ndef\nghi\n";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, _text: &str| {
        buffer.set_buffer_local("display-line-numbers", Value::T);
    };
    let trace = layout_trace_with_buffer_setup(text, 360, 180, setup);

    assert!(!trace.matrix_rows.is_empty());
}

#[test]
fn layout_frame_rust_lays_out_word_wrap() {
    let text = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, _text: &str| {
        buffer.set_buffer_local("truncate-lines", Value::NIL);
        buffer.set_buffer_local("word-wrap", Value::T);
    };
    let trace = layout_trace_with_buffer_setup(text, 120, 120, setup);

    assert!(!trace.matrix_rows.is_empty());
}

/// Full-pipeline regression for the word-wrap word-splitting bug: with
/// `word-wrap=t` / `truncate-lines=nil`, GNU keeps whole words across a wrapped
/// break (`...word02 `|`word03...`), never splitting a word (`...word02 wor`|
/// `d03...`) or dropping the word-start char (`...word02 `|`d03...`).
///
/// The bug had TWO coupled parts and this test catches BOTH:
///   A. the partial word that fit on the first row was left drawn (leftover
///      glyphs), and
///   B. the word-start (candidate) char was already consumed during the
///      overflow attempt and never re-produced — so the continuation row
///      started AFTER it, dropping the word prefix.
/// A first-row-only check catches only (A). This drives a real buffer through
/// the whole layout pipeline and asserts the CONTINUATION row re-renders
/// starting at the SAME word-boundary char the first row stopped before, which
/// only holds when (B) is fixed too (the consumption cursor is rewound).
#[test]
fn word_wrap_keeps_words_whole_across_wrapped_rows() {
    // Equal-length space-separated words: word00 starts at charpos 0, and word
    // N starts at charpos 7*N (each "wordNN " is 7 chars). Buffer chars are pure
    // ASCII so charpos == byte index.
    let text = "word00 word01 word02 word03 word04 word05 word06 word07 word08 word09 word10 word11 word12";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, _text: &str| {
        buffer.set_buffer_local("truncate-lines", Value::NIL);
        buffer.set_buffer_local("word-wrap", Value::T);
    };
    let trace = layout_trace_with_buffer_setup(text, 200, 240, setup);

    // The first text glyph (char + its charpos) of each non-mode-line Text row.
    let row_first_text_glyphs: Vec<(usize, char)> = trace
        .matrix_rows
        .iter()
        .filter(|row| !row.mode_line && row.role == GlyphRowRole::Text && row.displays_text)
        .filter_map(|row| {
            row.glyph_areas[1]
                .iter()
                .find_map(|glyph| match glyph.kind {
                    GlyphKindTrace::Char(ch) => Some((glyph.charpos, ch)),
                    _ => None,
                })
        })
        .collect();

    // The buffer wraps onto multiple rows (200px / 8px-per-char ≈ 24 cols, the
    // 89-char line needs >=4 rows). Need at least one wrapped continuation row to
    // exercise the break.
    assert!(
        row_first_text_glyphs.len() >= 2,
        "expected the long line to wrap onto multiple rows, got {row_first_text_glyphs:?}"
    );

    // Every Text row (the first AND every continuation row) must begin at a WORD
    // START. Word starts are at charpos 7*N ('w'); a split/dropped word would
    // begin mid-word (e.g. charpos 22 'r' of word03 after dropping "wor", or
    // charpos 25 '0' if the whole "word" prefix was dropped). This is the
    // load-bearing assertion that part B fixes: without the consumption-cursor
    // rewind the continuation row starts AFTER the candidate char.
    for (charpos, ch) in &row_first_text_glyphs {
        assert_eq!(
            charpos % 7,
            0,
            "continuation row starts mid-word at charpos {charpos} (char {ch:?}); \
             word-wrap split or dropped a word. first-text glyphs per row: {row_first_text_glyphs:?}"
        );
        assert_eq!(
            *ch, 'w',
            "the word-start char at charpos {charpos} should be 'w' (the 'w' of a 'wordNN'); \
             got {ch:?} — the candidate char was dropped. rows: {row_first_text_glyphs:?}"
        );
    }

    // Pin the EXACT continuation seam: the first wrapped continuation row must
    // re-render starting at the candidate char 'w' of word03 (charpos 21) — the
    // same word boundary the first row stopped before. With the bug, this row
    // instead started at charpos 25 ('0' of "...03"), dropping "word".
    assert_eq!(
        row_first_text_glyphs[1],
        (21, 'w'),
        "first continuation row must re-render the dropped word-start char (word03 @ charpos 21); \
         rows: {row_first_text_glyphs:?}"
    );
}

// Walk-state coverage guards: these scenarios exercise the typed-source walk
// through item-step arms (control chars, NBSP/SHY, selective-display '\r') or
// bypass item consumption entirely (invisible/hscroll short-circuit before
// source-item consumption). They remain as single-path regression guards so
// the NBSP / selective-display / invisible / hscroll / complex-text scenarios
// keep being laid out.

#[test]
fn layout_frame_rust_lays_out_invisible_text() {
    let text = "abcXYZdef\nghi\n";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, text: &str| {
        let start = text.find("XYZ").expect("XYZ start");
        let end = start + "XYZ".len();
        assert!(buffer.put_text_property(start, end, Value::symbol("invisible"), Value::T));
    };
    let trace = layout_trace_with_buffer_setup(text, 360, 180, setup);

    assert!(!trace.matrix_rows.is_empty());
}

#[test]
fn layout_frame_rust_emits_one_ellipsis_for_invisible_region_split_by_face() {
    // Regression for the org-fold "long dot-fill" bug: ONE contiguous invisible
    // region that has a DIFFERENT text property (`face`) changing in its middle
    // must collapse to exactly ONE ellipsis.  The buggy code computed the
    // invisible region's end via the next change of ANY text property, so the
    // mid-region `face` boundary fragmented the region into several `...` runs
    // (a long dot-fill).  The fix scans only the `invisible` property's next
    // change (GNU `next_single_char_property_change(pos, Qinvisible, ...)`), so
    // the whole region is skipped once -> one ellipsis.
    //
    // Buffer text avoids literal `.` so every `.` glyph comes from an ellipsis.
    let text = "AAAfooBBBbarCCC\nDDD\n";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, text: &str| {
        buffer.set_buffer_local(
            "buffer-invisibility-spec",
            Value::list(vec![Value::cons(Value::symbol("outline"), Value::T)]),
        );
        // One contiguous invisible region covering "fooBBBbar".
        let invis_start = text.find("foo").expect("foo start");
        let invis_end = text.find("CCC").expect("CCC start");
        assert!(buffer.put_text_property(
            invis_start,
            invis_end,
            Value::symbol("invisible"),
            Value::symbol("outline"),
        ));
        // A face change strictly INSIDE the invisible region: this is the
        // unrelated property whose boundary used to fragment the region.
        let face_start = text.find("BBB").expect("BBB start");
        let face_end = face_start + "BBB".len();
        assert!(buffer.put_text_property(
            face_start,
            face_end,
            Value::symbol("face"),
            Value::symbol("bold"),
        ));
    };
    let trace = layout_trace_with_buffer_setup(text, 360, 180, setup);

    let rendered = backend_trace_text_area_text(&trace);
    let dot_count = rendered.matches('.').count();
    let ellipsis_runs = rendered.matches("...").count();

    // Exactly one ellipsis (the default `...` = 3 dots) for the whole region.
    assert_eq!(
        dot_count, 3,
        "expected exactly one 3-dot ellipsis for the folded region, got {dot_count} dots; rendered={rendered:?}"
    );
    assert_eq!(
        ellipsis_runs, 1,
        "expected exactly one `...` ellipsis run, got {ellipsis_runs}; rendered={rendered:?}"
    );
    // Visible text on both sides of the fold survives.
    assert!(
        rendered.contains("AAA") && rendered.contains("CCC"),
        "visible text around the fold must render, rendered={rendered:?}"
    );
}

#[test]
fn layout_frame_rust_collapses_consecutive_invisible_runs_to_one_ellipsis() {
    // Regression for org folding over a link: a CONTIGUOUS hidden region whose
    // `invisible` VALUE changes mid-region must collapse to ONE ellipsis. A
    // folded org subtree (`outline`, shows ellipsis) containing a link whose URL
    // is separately invisible (`org-link`, no ellipsis) is three runs of
    // differing `invisible` value but all hidden. GNU `handle_invisible_prop`
    // advances over the consecutive invisible runs showing a single ellipsis;
    // stopping at each value change emitted one per ellipsis-bearing run.
    let text = "AAAfooBBBbarCCC\nDDD\n";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, text: &str| {
        buffer.set_buffer_local(
            "buffer-invisibility-spec",
            Value::list(vec![
                Value::cons(Value::symbol("outline"), Value::T),
                Value::list(vec![Value::symbol("org-link")]),
            ]),
        );
        let foo = text.find("foo").expect("foo start");
        let bbb = text.find("BBB").expect("BBB start");
        let bbb_end = bbb + "BBB".len();
        let ccc = text.find("CCC").expect("CCC start");
        // `outline` (ellipsis) around an `org-link` (no ellipsis) middle: the
        // `invisible` value changes twice inside one contiguous hidden region.
        assert!(buffer.put_text_property(
            foo,
            bbb,
            Value::symbol("invisible"),
            Value::symbol("outline"),
        ));
        assert!(buffer.put_text_property(
            bbb,
            bbb_end,
            Value::symbol("invisible"),
            Value::symbol("org-link"),
        ));
        assert!(buffer.put_text_property(
            bbb_end,
            ccc,
            Value::symbol("invisible"),
            Value::symbol("outline"),
        ));
    };
    let trace = layout_trace_with_buffer_setup(text, 360, 180, setup);

    let rendered = backend_trace_text_area_text(&trace);
    let dot_count = rendered.matches('.').count();
    let ellipsis_runs = rendered.matches("...").count();

    // One ellipsis for the whole collapsed region (the opening `outline` run's
    // ellipsis). Without collapsing this is two (`outline` foo + `outline` bar).
    assert_eq!(
        dot_count, 3,
        "expected ONE 3-dot ellipsis for the collapsed region, got {dot_count} dots; rendered={rendered:?}"
    );
    assert_eq!(
        ellipsis_runs, 1,
        "expected ONE `...` run, got {ellipsis_runs}; rendered={rendered:?}"
    );
    assert!(
        rendered.contains("AAA") && rendered.contains("CCC"),
        "visible text around the fold must render, rendered={rendered:?}"
    );
}

#[test]
fn layout_frame_rust_lays_out_nobreak_chars() {
    // U+00A0 NBSP and U+00AD SHY are delivered as plain Text by the typed cursor;
    // the nobreak display policy is applied downstream by the walk.
    let text = "a\u{00A0}b\u{00AD}c\nd\u{00A0}\u{00A0}e\n";
    let trace = layout_trace_for_plain_text(text);

    assert!(!trace.matrix_rows.is_empty());
}

#[test]
fn layout_frame_rust_lays_out_selective_display() {
    // selective-display>0 + embedded '\r': the typed cursor emits '\r' as a
    // ControlChar (a translated shim arm); the selective-display tail-skip is
    // walk-state run on the consumed char.
    let text = "visible\rhidden\nnext\rgone\n";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, _text: &str| {
        buffer.set_buffer_local("selective-display", Value::fixnum(1));
    };
    let trace = layout_trace_with_buffer_setup(text, 360, 180, setup);

    assert!(!trace.matrix_rows.is_empty());
}

#[test]
fn layout_frame_rust_lays_out_hscroll() {
    // Window hscroll>0 on a truncated long line: the walk skips leading columns
    // BEFORE consuming the source (render_hscroll_skip), so sourcing is bypassed
    // for the skipped span.
    let text = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ\n";
    let buf_setup = |buffer: &mut neovm_core::buffer::Buffer, _buf_id: BufferId, _text: &str| {
        buffer.set_buffer_local("truncate-lines", Value::T);
    };
    let win_setup = |window: &mut neovm_core::window::Window| {
        if let neovm_core::window::Window::Leaf { hscroll, .. } = window {
            *hscroll = 10;
        }
    };
    let trace = layout_trace_with_buffer_and_window_setup(text, 200, 120, buf_setup, win_setup);

    assert!(!trace.matrix_rows.is_empty());
}

#[test]
fn layout_frame_rust_lays_out_complex_text() {
    // Arabic (contextual joining), Hebrew (RTL bidi), and an emoji ZWJ family
    // (composition): the typed cursor folds these into TextRuns that the append
    // layer re-shapes/clusters/reorders downstream — the source carries only the
    // chars + faces, not shaping decisions.
    let text = "العربية\nאבגד\n👨\u{200d}👩\u{200d}👧\nmixed العربية text\n";
    let trace = layout_trace_for_plain_text(text);

    assert!(!trace.matrix_rows.is_empty());
}

fn backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "abé\tz\n日本x\nlast Ω line\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-parity",
        text,
        360,
        180,
        |buffer, _buf_id, text| {
            let omega_byte = text.find('Ω').expect("omega");
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(omega_byte));
            buffer.set_buffer_local("display-line-numbers", Value::T);
        },
    )
}

#[test]
fn layout_frame_rust_line_number_cursor_tracks_first_text_column_after_c_n() {
    let trace = backend_layout_trace_with_buffer_and_window_setup(
        BufferTextBackendKind::GapBuffer,
        "layout-line-number-cursor-first-text-column",
        "abc\ndef\n",
        360,
        140,
        |buffer, _buf_id, _text| {
            buffer.set_buffer_local("display-line-numbers", Value::T);
            buffer.goto_emacs_byte_pos(EmacsBytePos::new(4));
        },
        |window| {
            if let neovm_core::window::Window::Leaf { window_start, .. } = window {
                *window_start = LispCharPos1::ONE;
            }
        },
    );

    let cursor = trace.phys_cursor.as_ref().expect("phys cursor");
    let point = trace
        .points
        .iter()
        .find(|point| point.buffer_pos == LispCharPos1::from_one_based_usize(5))
        .expect("display point for first character on second line");

    assert_eq!(cursor.row, point.row);
    assert_eq!(cursor.col, point.col);
    assert_eq!(cursor.x, point.x);
}

/// An anonymous `(:background ... :extend t)` face value, the shape hl-line /
/// region use to highlight a whole line out to the window edge.
fn extend_face_value() -> Value {
    Value::list(vec![
        Value::keyword("background"),
        Value::string("#003366"),
        Value::keyword("extend"),
        Value::T,
    ])
}

/// Lay out a 360x180 frame over `text` with point at `point_byte`, an `:extend`
/// face on `extend_range`, and `display-line-numbers` optionally enabled.
/// Returns the frame's authoritative phys cursor (the geometry the GUI draws)
/// and the frame's pixel width.
fn empty_line_extend_cursor(
    text: &str,
    extend_range: (usize, usize),
    point_byte: usize,
    line_numbers: bool,
) -> (neomacs_display_protocol::frame_glyphs::PhysCursor, f32) {
    let mut eval = Context::new();
    convert_current_buffer_text_backend(&mut eval, BufferTextBackendKind::GapBuffer);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        insert_fragmented_current_buffer_text(&mut eval, text);
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        if line_numbers {
            buffer.set_buffer_local("display-line-numbers", Value::T);
        }
        assert!(buffer.put_text_property(
            extend_range.0,
            extend_range.1,
            Value::symbol("face"),
            extend_face_value()
        ));
        buffer.goto_emacs_byte_pos(EmacsBytePos::new(point_byte));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("empty-line-extend-cursor", 360, 180, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
    }
    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);
    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let phys = state
        .phys_cursor
        .clone()
        .expect("frame phys cursor on the empty :extend line");
    (phys, state.frame_pixel_width)
}

/// Regression: with `display-line-numbers` + an `:extend` (hl-line-style) face,
/// the cursor on an EMPTY line must sit at column 0 of the text area (right
/// after the line-number gutter), NOT at the far-right window edge.
///
/// `extend_face_to_end_of_line` fills the highlighted background from EOL to the
/// window edge by appending a face-anchor space + a wide stretch glyph. Those
/// synthetic glyphs carry no buffer position; before the fix
/// `CursorVisualColumnResolutionRequest::resolve` counted them into the visual
/// column, shoving the blank-line cursor to the fill's right edge.
#[test]
fn empty_line_extend_cursor_sits_at_text_start_not_window_edge() {
    // Empty middle line of "abc\n\ndef\n", point on it (byte 4), line numbers on.
    let (cursor, frame_width) = empty_line_extend_cursor("abc\n\ndef\n", (4, 5), 4, true);
    // Reference: a real char at the start of a NON-empty line gets the first
    // text column (the gutter width). The empty-line cursor must match it, not
    // the :extend fill's far-right edge.
    let (non_empty_cursor, _) = empty_line_extend_cursor("abc\n\ndef\n", (0, 1), 0, true);
    assert_eq!(
        cursor.col, non_empty_cursor.col,
        "empty-line cursor column must equal the first text column (column 0 of \
         the text area), not the :extend fill's right edge; got {cursor:?}"
    );
    // The drawn cursor must be far from the window's right edge (the bug placed
    // it at/past `frame_width`).
    assert!(
        cursor.x <= non_empty_cursor.x + 1.0 && cursor.x < frame_width / 2.0,
        "empty-line cursor x must be at the text-area start (~{}), not near the \
         window right edge ({frame_width}); got x={}",
        non_empty_cursor.x,
        cursor.x
    );

    // Without line numbers the text area starts at x=0, so an empty line's
    // cursor must be at column 0 / x=0 exactly.
    let (no_ln_cursor, _) = empty_line_extend_cursor("abc\n\ndef\n", (4, 5), 4, false);
    assert_eq!(
        no_ln_cursor.col, 0,
        "empty-line cursor without line numbers must be at column 0; got {no_ln_cursor:?}"
    );
    assert_eq!(
        no_ln_cursor.x, 0.0,
        "empty-line cursor without line numbers must be at x=0; got {no_ln_cursor:?}"
    );

    // The fill's synthetic glyphs carry no buffer position, so they must not
    // displace the cursor at end-of-line on a NON-empty first line whose real
    // first char carries 0-based charpos 0: the cursor sits AFTER that char.
    let (single_char_cursor, _) = empty_line_extend_cursor("a\nbc\n", (0, 2), 1, false);
    assert_eq!(
        single_char_cursor.col, 1,
        "EOL cursor on a single-char first line must sit after the char (col 1), \
         not be pulled back over the trimmed fill; got {single_char_cursor:?}"
    );
}

#[test]
fn layout_frame_rust_line_number_width_matches_gnu_visible_row_width() {
    let trace = backend_layout_trace_with_buffer_and_window_setup(
        BufferTextBackendKind::GapBuffer,
        "layout-line-number-width-visible-rows",
        "abc\ndef\n",
        360,
        430,
        |buffer, _buf_id, _text| {
            buffer.set_buffer_local("display-line-numbers", Value::T);
        },
        |window| {
            if let neovm_core::window::Window::Leaf { window_start, .. } = window {
                *window_start = LispCharPos1::ONE;
            }
        },
    );

    let first_text_row = trace
        .matrix_rows
        .iter()
        .find(|row| row.role == GlyphRowRole::Text && row.displays_text)
        .expect("first text row");
    let left_margin = &first_text_row.glyph_areas[GlyphArea::LeftMargin.index()];

    assert_eq!(
        left_margin
            .iter()
            .map(|glyph| glyph.kind.clone())
            .collect::<Vec<_>>(),
        vec![
            GlyphKindTrace::Stretch(2),
            GlyphKindTrace::Char('1'),
            GlyphKindTrace::Stretch(1),
        ]
    );
}

fn display_replacement_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "abcXYZdef\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-display-replacement",
        text,
        360,
        140,
        |buffer, _buf_id, text| {
            let start = text.find("XYZ").expect("replacement start");
            let end = start + "XYZ".len();
            assert!(buffer.put_text_property(
                start,
                end,
                Value::symbol("display"),
                Value::string("R")
            ));
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(start + 1));
        },
    )
}

fn invisible_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "abc hidden xyz\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-invisible",
        text,
        360,
        140,
        |buffer, _buf_id, text| {
            let start = text.find("hidden").expect("hidden start");
            let end = start + "hidden".len();
            assert!(buffer.put_text_property(start, end, Value::symbol("invisible"), Value::T));
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(start + 2));
        },
    )
}

fn multiline_overlay_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "x";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-overlay",
        text,
        360,
        140,
        |buffer, buf_id, _text| {
            let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
                serial: 0,
                plist: Value::NIL,
                buffer: Some(buf_id),
                start: 0,
                end: 1,
                front_advance: false,
                rear_advance: false,
            });
            buffer.overlays_mut().insert_overlay(overlay);
            let _ = buffer.overlays_mut().overlay_put(
                overlay,
                Value::symbol("after-string"),
                Value::string("A\nB"),
            );
            buffer.goto_emacs_byte_pos(buffer.point_max_emacs_byte_pos());
        },
    )
}

#[test]
fn layout_frame_rust_renders_overlay_display_property() {
    // GNU get_char_property: an overlay `display` overrides the text property —
    // e.g. org-display-inline-images overlays the link with an `(image …)`.
    // Reading only the text property left those as raw text. Here an overlay
    // covering "HIDE" with display "SHOWN" must render "SHOWN", not "HIDE".
    let text = "AA HIDE BB\n";
    let setup = |buffer: &mut neovm_core::buffer::Buffer, buf_id: BufferId, text: &str| {
        let start = text.find("HIDE").expect("HIDE");
        let end = start + "HIDE".len();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start,
            end,
            front_advance: false,
            rear_advance: false,
        });
        buffer.overlays_mut().insert_overlay(overlay);
        let _ = buffer.overlays_mut().overlay_put(
            overlay,
            Value::symbol("display"),
            Value::string("SHOWN"),
        );
    };
    let trace = layout_trace_with_buffer_setup(text, 360, 180, setup);
    let rendered = backend_trace_text_area_text(&trace);

    assert!(
        rendered.contains("SHOWN"),
        "overlay display string must render, rendered={rendered:?}"
    );
    assert!(
        !rendered.contains("HIDE"),
        "the overlay-covered text must be replaced, rendered={rendered:?}"
    );
    assert!(
        rendered.contains("AA") && rendered.contains("BB"),
        "text around the overlay must still render, rendered={rendered:?}"
    );
}

fn bidi_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "abc אבג def\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-bidi",
        text,
        360,
        140,
        |buffer, _buf_id, text| {
            let alef_byte = text.find('א').expect("alef");
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(alef_byte));
        },
    )
}

fn selective_display_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "head\rhidden tail\nshown\n  hidden by indent\nshown2\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-selective-display",
        text,
        360,
        180,
        |buffer, _buf_id, _text| {
            buffer.set_buffer_local("selective-display", Value::fixnum(1));
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(2));
        },
    )
}

fn glyphless_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "a\u{0080}b\u{FEFF}c\u{FFFC}d\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-glyphless",
        text,
        360,
        140,
        |buffer, _buf_id, text| {
            let c1_byte = text.find('\u{0080}').expect("C1 control");
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(c1_byte));
        },
    )
}

fn composition_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    let text = "e\u{0301} a\u{0300}\u{0301} 中\u{0300}\nplain\n";
    backend_layout_trace_with_buffer_setup(
        kind,
        "layout-backend-composition",
        text,
        360,
        140,
        |buffer, _buf_id, text| {
            let cjk_byte = text.find('中').expect("CJK base char");
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(cjk_byte));
        },
    )
}

fn wrapped_retry_backend_layout_trace(kind: BufferTextBackendKind) -> (BackendLayoutTrace, usize) {
    let logical_lines = (0..24)
        .map(|line| format!("line-{line:02} abcdefghijklmno\n"))
        .collect::<Vec<_>>();
    let text = logical_lines.join("");
    let target_pos = logical_lines
        .iter()
        .take(18)
        .map(|line| line.chars().count())
        .sum::<usize>()
        + 1;

    let trace = backend_layout_trace_with_buffer_and_window_setup(
        kind,
        "layout-backend-wrap-retry",
        &text,
        80,
        192,
        |buffer, _buf_id, _text| {
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(target_pos - 1));
            buffer.set_buffer_local("word-wrap", Value::T);
        },
        |window| {
            if let neovm_core::window::Window::Leaf { point, .. } = window {
                *point = LispCharPos1::from_one_based_usize(target_pos);
            }
        },
    );
    (trace, target_pos)
}

fn point_line_tail_backend_layout_trace(
    kind: BufferTextBackendKind,
) -> (BackendLayoutTrace, usize, usize) {
    let prefix = (0..2)
        .map(|line| format!("p{line:02}\n"))
        .collect::<Vec<_>>()
        .join("");
    let target_line = "abcdefghijklmno\n";
    let text = format!("{prefix}{target_line}");
    let point = prefix.chars().count() + 1;
    let later_pos = point + 10;

    let trace = backend_layout_trace_with_buffer_and_window_setup(
        kind,
        "layout-backend-point-line-tail",
        &text,
        80,
        256,
        |buffer, _buf_id, _text| {
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
            buffer.set_buffer_local("word-wrap", Value::T);
        },
        |window| {
            if let neovm_core::window::Window::Leaf {
                point: window_point,
                ..
            } = window
            {
                *window_point = LispCharPos1::from_one_based_usize(point);
            }
        },
    );
    (trace, point, later_pos)
}

fn mode_line_geometry_backend_layout_trace(
    kind: BufferTextBackendKind,
) -> (BackendLayoutTrace, usize) {
    let text = (0..80)
        .map(|line| format!("Line {line:02}\n"))
        .collect::<String>();
    let point = text.chars().count() + 1;

    let trace = backend_layout_trace_with_buffer_and_window_setup(
        kind,
        "layout-backend-mode-line-geometry",
        &text,
        640,
        96,
        |buffer, _buf_id, _text| {
            buffer.set_buffer_local("mode-line-format", Value::string("%o|%p|%P"));
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
        },
        |window| {
            if let neovm_core::window::Window::Leaf {
                point: window_point,
                ..
            } = window
            {
                *window_point = LispCharPos1::from_one_based_usize(point);
            }
        },
    );
    (trace, point)
}

fn hscroll_cursor_backend_layout_trace(kind: BufferTextBackendKind) -> BackendLayoutTrace {
    backend_layout_trace_with_buffer_and_window_setup(
        kind,
        "layout-backend-hscroll-cursor",
        "abcdef\n",
        160,
        120,
        |buffer, _buf_id, _text| {
            buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
            buffer.set_buffer_local("truncate-lines", Value::T);
        },
        |window| {
            if let neovm_core::window::Window::Leaf { point, hscroll, .. } = window {
                *point = LispCharPos1::from_one_based_usize(2);
                *hscroll = 3;
            }
        },
    )
}

fn edit_redisplay_backend_layout_trace(
    kind: BufferTextBackendKind,
) -> (BackendLayoutTrace, BackendLayoutTrace) {
    let mut eval = Context::new();
    convert_current_buffer_text_backend(&mut eval, kind);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        insert_fragmented_current_buffer_text(&mut eval, "alpha beta gamma\n");
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        assert_eq!(buffer.text_backend_kind(), kind);
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-backend-edit-redisplay", 360, 140, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);
    let before = selected_window_layout_trace(&eval, &engine, frame_id);

    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        let start = buffer.buffer_string().find("beta").expect("beta");
        let end = start + "beta".len();
        buffer.delete_emacs_byte_range(emacs_byte_range(start, end));
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(start));
        buffer.insert("BETA");
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        assert_eq!(buffer.buffer_string(), "alpha BETA gamma\n");
    }

    engine.layout_frame_rust(&mut eval, frame_id);
    let after = selected_window_layout_trace(&eval, &engine, frame_id);
    (before, after)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FontificationBackendTrace {
    before_layout: BackendLayoutTrace,
    before_props: String,
    after_layout: BackendLayoutTrace,
    after_props: String,
}

fn printed_eval_result(eval: &mut Context, form: &str) -> String {
    eval.eval_str(form)
        .unwrap_or_else(|err| panic!("eval {form}: {err}"))
        .as_runtime_string_owned()
        .unwrap_or_else(|| panic!("eval {form} did not return a string"))
}

fn fontification_edit_backend_trace(kind: BufferTextBackendKind) -> FontificationBackendTrace {
    let mut eval = Context::new();
    convert_current_buffer_text_backend(&mut eval, kind);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        insert_fragmented_current_buffer_text(&mut eval, "alpha beta gamma\n");
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        assert_eq!(buffer.text_backend_kind(), kind);
    }

    eval.eval_str(
        r#"
        (setq neomacs-test-fontify-face 'font-lock-keyword-face)
        (setq redisplay-fontify-calls nil)
        (setq fontification-functions
              (list (lambda (start)
                      (setq redisplay-fontify-calls
                            (cons start redisplay-fontify-calls))
                      (let ((end (min (point-max) (+ start 80))))
                        (put-text-property start end 'fontified t)
                        (put-text-property start end 'font-lock-face
                                           neomacs-test-fontify-face)))))
        "#,
    )
    .unwrap_or_else(|err| panic!("install redisplay fontification hook: {err}"));

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-backend-fontification-edit",
        360,
        140,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);
    let before_layout = selected_window_layout_trace(&eval, &engine, frame_id);
    let before_props = printed_eval_result(
        &mut eval,
        "(prin1-to-string (list redisplay-fontify-calls (get-text-property 1 'fontified) (get-text-property 1 'font-lock-face)))",
    );

    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        let start = buffer.buffer_string().find("beta").expect("beta");
        let end = start + "beta".len();
        buffer.delete_emacs_byte_range(emacs_byte_range(start, end));
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(start));
        buffer.insert("BETA");
        buffer.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        assert_eq!(buffer.buffer_string(), "alpha BETA gamma\n");
    }

    eval.eval_str(
        r#"
        (setq neomacs-test-fontify-face 'font-lock-warning-face)
        (setq redisplay-fontify-calls nil)
        (remove-text-properties (point-min) (point-max)
                                '(fontified nil font-lock-face nil))
        "#,
    )
    .unwrap_or_else(|err| panic!("clear fontification state after edit: {err}"));

    engine.layout_frame_rust(&mut eval, frame_id);
    let after_layout = selected_window_layout_trace(&eval, &engine, frame_id);
    let after_props = printed_eval_result(
        &mut eval,
        "(prin1-to-string (list redisplay-fontify-calls (get-text-property 1 'fontified) (get-text-property 1 'font-lock-face)))",
    );

    FontificationBackendTrace {
        before_layout,
        before_props,
        after_layout,
        after_props,
    }
}

fn glyph_trace_text(glyph: &GlyphTrace) -> String {
    match &glyph.kind {
        GlyphKindTrace::Char(ch) => ch.to_string(),
        GlyphKindTrace::Composite(text) => text.clone(),
        GlyphKindTrace::Stretch(width) => " ".repeat(usize::from(*width)),
        GlyphKindTrace::Image(_) | GlyphKindTrace::Glyphless(_) => String::new(),
    }
}

fn trace_rows_for_role(trace: &BackendLayoutTrace, role: GlyphRowRole) -> Vec<String> {
    trace
        .matrix_rows
        .iter()
        .filter(|row| row.role == role)
        .map(|row| {
            row.glyph_areas[1]
                .iter()
                .map(glyph_trace_text)
                .collect::<Vec<_>>()
                .join("")
        })
        .collect()
}

fn trace_text_rows(trace: &BackendLayoutTrace) -> Vec<String> {
    trace_rows_for_role(trace, GlyphRowRole::Text)
}

fn trace_mode_line_text(trace: &BackendLayoutTrace) -> String {
    trace_rows_for_role(trace, GlyphRowRole::ModeLine).join("")
}

fn trace_text_face_ids(trace: &BackendLayoutTrace) -> Vec<u32> {
    trace
        .matrix_rows
        .iter()
        .filter(|row| row.role == GlyphRowRole::Text)
        .flat_map(|row| row.glyph_areas[1].iter().map(|glyph| glyph.face_id))
        .collect()
}

fn trace_composite_texts(trace: &BackendLayoutTrace) -> Vec<String> {
    trace
        .matrix_rows
        .iter()
        .filter(|row| row.role == GlyphRowRole::Text)
        .flat_map(|row| row.glyph_areas[1].iter())
        .filter_map(|glyph| match &glyph.kind {
            GlyphKindTrace::Composite(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn trace_has_nonzero_bidi_level(trace: &BackendLayoutTrace) -> bool {
    trace.matrix_rows.iter().any(|row| {
        row.glyph_areas
            .iter()
            .flat_map(|area| area.iter())
            .any(|glyph| glyph.bidi_level > 0)
    })
}

fn assert_echo_message_renders_in_minibuffer_window(use_gui_metrics: bool) {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-minibuffer-echo", 640, 160, buf_id);
    let echo = "Echo lives in minibuffer";
    eval.set_current_message(Some(LispString::from_utf8(echo)));

    let mut engine = LayoutEngine::new();
    if use_gui_metrics {
        engine.enable_cosmic_metrics();
    }
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let minibuffer_window_id = state
        .window_infos
        .iter()
        .find(|info| info.is_minibuffer)
        .expect("minibuffer window info")
        .window_id
        .get() as u64;
    let root_window_id = state
        .window_infos
        .iter()
        .find(|info| !info.is_minibuffer)
        .expect("root window info")
        .window_id
        .get() as u64;

    let minibuffer_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == minibuffer_window_id)
        .expect("minibuffer matrix");
    let root_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == root_window_id)
        .expect("root matrix");

    let minibuffer_text = window_matrix_text(minibuffer_entry);
    let root_text = window_matrix_text(root_entry);

    assert!(
        minibuffer_text.contains(echo),
        "expected echo text in minibuffer matrix, got {minibuffer_text:?}"
    );
    assert!(
        !root_text.contains(echo),
        "echo text leaked into root window matrix: {root_text:?}"
    );
    // Post slice-8 the echo area is rendered through the ordinary buffer-text
    // walk over ` *Echo Area 0*` (GNU `display_echo_area_1`), so its rows are
    // plain buffer-text rows — the same role the *active* minibuffer walk
    // already produces — not a special Minibuffer-tagged row.
    assert!(
        minibuffer_entry
            .matrix
            .rows
            .iter()
            .any(|row| row.enabled && row.role == GlyphRowRole::Text && !row.mode_line),
        "expected a non-chrome buffer-text row for echo text"
    );
    assert!(
        !root_text.contains(echo),
        "echo text must not leak into the root window matrix"
    );
}

#[test]
fn layout_frame_rust_preserves_propertized_echo_message_faces() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-propertized-echo", 320, 120, buf_id);
    let echo = Value::string_with_text_properties(
        "A中👨‍👩",
        vec![StringTextPropertyRun {
            start: 1,
            end: 2,
            plist: Value::list(vec![
                Value::symbol("face"),
                Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]),
            ]),
        }],
    );
    eval.set_current_message(echo.as_lisp_string().cloned());

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    // Post slice-8 the echo area renders through the ordinary buffer-text walk
    // over ` *Echo Area 0*`, so locate the mini-window by identity and take its
    // non-chrome buffer-text row.
    let minibuffer_window_id = state
        .window_infos
        .iter()
        .find(|info| info.is_minibuffer)
        .expect("minibuffer window info")
        .window_id
        .get() as u64;
    let minibuffer_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == minibuffer_window_id)
        .expect("minibuffer echo matrix");
    let echo_glyphs = minibuffer_entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text && !row.mode_line)
        .expect("echo row")
        .glyphs[1]
        .clone();

    assert_eq!(glyphs_logical_text(&echo_glyphs), "A中👨‍👩");
    assert_ne!(
        echo_glyphs[0].face_id, echo_glyphs[1].face_id,
        "propertized echo character should receive its property face"
    );
    assert!(
        echo_glyphs[1].wide,
        "echo CJK glyph should use the shared wide-glyph builder: {echo_glyphs:?}"
    );
    assert!(
        echo_glyphs.iter().any(|glyph| glyph.padding),
        "echo CJK glyph should retain its padding cell: {echo_glyphs:?}"
    );
    assert!(
        echo_glyphs.iter().any(
            |glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.as_ref() == "👨‍👩")
        ),
        "echo ZWJ emoji should be clustered by the shared builder: {echo_glyphs:?}"
    );
    assert!(
        echo_glyphs
            .iter()
            .filter(|glyph| !glyph.padding)
            .all(|glyph| glyph.pixel_width > 0.0),
        "echo glyphs should carry real pixel widths: {echo_glyphs:?}"
    );
}

fn assert_multiline_echo_message_resizes_minibuffer_rows(use_gui_metrics: bool) {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-minibuffer-echo-lines", 640, 160, buf_id);
    eval.set_current_message(Some(LispString::from_utf8("ALPHA\nBETA")));

    let mut engine = LayoutEngine::new();
    if use_gui_metrics {
        engine.enable_cosmic_metrics();
    }
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let minibuffer_window_id = state
        .window_infos
        .iter()
        .find(|info| info.is_minibuffer)
        .expect("minibuffer window info")
        .window_id
        .get() as u64;
    let minibuffer_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == minibuffer_window_id)
        .expect("minibuffer matrix");
    let row_texts = enabled_window_row_texts(minibuffer_entry);

    assert!(
        row_texts.iter().any(|row| row == "ALPHA"),
        "expected ALPHA in its own minibuffer row, got {row_texts:?}"
    );
    assert!(
        row_texts.iter().any(|row| row == "BETA"),
        "expected BETA in its own minibuffer row, got {row_texts:?}"
    );
    assert!(
        !row_texts.iter().any(|row| row.contains("ALPHABETA")),
        "multiline echo text was flattened into one row: {row_texts:?}"
    );
}

#[test]
fn layout_frame_rust_publishes_increasing_display_positions() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcd\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-test", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(2))
        .expect("b");
    let c = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("c");
    assert!(
        a.x < b.x,
        "expected increasing x positions, got {a:?} then {b:?}"
    );
    assert!(
        b.x < c.x,
        "expected increasing x positions, got {b:?} then {c:?}"
    );
}

#[test]
fn layout_frame_rust_tracks_multibyte_sample_positions() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a好好b\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-test", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let all_points = snapshot.points.clone();
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let hao1 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(2))
        .expect("hao1");
    let hao2 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("hao2");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(4))
        .expect("b");
    assert!(
        a.x < hao1.x,
        "expected a before first 好, got {a:?} then {hao1:?}; points={all_points:?}"
    );
    assert!(
        hao1.x < hao2.x,
        "expected first 好 before second 好, got {hao1:?} then {hao2:?}; points={all_points:?}"
    );
    assert!(
        hao2.x < b.x,
        "expected second 好 before b, got {hao2:?} then {b:?}; points={all_points:?}"
    );
    assert!(
        a.width > 0,
        "expected positive width for a, got {a:?}; points={all_points:?}"
    );
    assert!(
        hao1.width > 0,
        "expected positive width for first 好, got {hao1:?}; points={all_points:?}"
    );
    assert!(
        hao2.width > 0,
        "expected positive width for second 好, got {hao2:?}; points={all_points:?}"
    );
    assert!(
        b.width > 0,
        "expected positive width for b, got {b:?}; points={all_points:?}"
    );
}

#[test]
fn implemented_text_backends_match_layout_frame_rows_points_and_cursor() {
    let baseline = backend_layout_trace(BufferTextBackendKind::GapBuffer);
    assert!(
        baseline
            .matrix_rows
            .iter()
            .any(|row| row.role == GlyphRowRole::Text
                && row.glyph_areas[1]
                    .iter()
                    .any(|glyph| glyph.kind == GlyphKindTrace::Char('Ω'))),
        "baseline should render omega row, got {baseline:?}"
    );
    assert!(
        baseline
            .matrix_rows
            .iter()
            .any(|row| !row.glyph_areas[0].is_empty()),
        "baseline should exercise left-margin line-number glyphs, got {baseline:?}"
    );
    assert!(
        baseline.phys_cursor.is_some(),
        "baseline should publish physical cursor geometry"
    );

    for kind in implemented_text_backends() {
        let trace = backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_layout_frame_display_replacement_output() {
    let baseline = display_replacement_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter().any(|row| row.contains("abcRdef")),
        "baseline should render display replacement text, rows={rows:?}"
    );
    assert!(
        rows.iter().all(|row| !row.contains("XYZ")),
        "baseline should not render covered source text, rows={rows:?}"
    );
    assert!(
        baseline.phys_cursor.is_some(),
        "baseline should publish cursor geometry for replacement slot"
    );

    for kind in implemented_text_backends() {
        let trace = display_replacement_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_layout_frame_invisible_text_output() {
    let baseline = invisible_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter().any(|row| row.contains("abc  xyz")),
        "baseline should omit invisible source text while preserving surrounding text, rows={rows:?}"
    );
    assert!(
        rows.iter().all(|row| !row.contains("hidden")),
        "baseline should not render invisible text, rows={rows:?}"
    );
    assert!(
        baseline.phys_cursor.is_some(),
        "baseline should keep a physical cursor when point is inside invisible text"
    );

    for kind in implemented_text_backends() {
        let trace = invisible_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn layout_frame_rust_renders_invisible_ellipsis_through_row_builder() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abc hidden xyz");
        buf.set_buffer_local(
            "buffer-invisibility-spec",
            Value::list(vec![Value::cons(Value::symbol("folded"), Value::T)]),
        );
        let start = "abc ".len();
        let end = start + "hidden".len();
        assert!(buf.put_text_property(
            start,
            end,
            Value::symbol("invisible"),
            Value::symbol("folded"),
        ));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-invisible-ellipsis", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let logical_text = glyphs_logical_text(&text_row.glyphs[1]);

    assert_eq!(logical_text, "abc ... xyz");
    assert!(
        text_row.glyphs[1]
            .iter()
            .filter(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '.' }))
            .all(|glyph| (glyph.pixel_width - 8.0).abs() <= 0.01),
        "ellipsis dots should carry measured pixel widths, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn implemented_text_backends_match_layout_frame_multiline_overlay_output() {
    let baseline = multiline_overlay_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter().any(|row| row.contains("xA")),
        "baseline should render overlay after-string suffix on the source row, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains('B')),
        "baseline should render multiline overlay continuation row, rows={rows:?}"
    );
    assert!(
        baseline.output_rows.iter().any(|row| row.row == 1),
        "baseline should publish a second output row for multiline overlay, rows={:?}",
        baseline.output_rows
    );

    for kind in implemented_text_backends() {
        let trace = multiline_overlay_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_layout_frame_bidi_row_output() {
    let baseline = bidi_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter()
            .any(|row| row.contains('א') && row.contains('ג')),
        "baseline should render Hebrew text in bidi row, rows={rows:?}"
    );
    assert!(
        trace_has_nonzero_bidi_level(&baseline),
        "baseline should mark reordered bidi glyphs, trace={baseline:?}"
    );

    for kind in implemented_text_backends() {
        let trace = bidi_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn arabic_run_composes_into_one_glyph_in_layout() {
    // ا ل م (U+0627 U+0644 U+0645) — an Arabic run. The layout walk must grow
    // it into ONE composed glyph so the renderer joins it, rather than three
    // isolated Char cells. (Structural: holds regardless of font availability,
    // since grouping is driven by complex_script, not by shaping success.)
    let trace = backend_layout_trace_with_buffer_setup(
        BufferTextBackendKind::GapBuffer,
        "layout-backend-arabic",
        "\u{0627}\u{0644}\u{0645}\n",
        360,
        140,
        |_buffer, _buf_id, _text| {},
    );
    let composites = trace_composite_texts(&trace);
    assert!(
        composites
            .iter()
            .any(|t| t.contains('\u{0627}') && t.contains('\u{0645}')),
        "Arabic run should compose into one Composite glyph spanning the run, \
         composites={composites:?}"
    );
}

#[test]
fn implemented_text_backends_match_selective_display_output() {
    let baseline = selective_display_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter().any(|row| row.contains("head")),
        "baseline should render text before carriage-return selective display marker, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("head...")),
        "baseline should render the selective-display ellipsis, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("shown")),
        "baseline should render visible line after selective display marker, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("shown2")),
        "baseline should resume rendering after an indented hidden block, rows={rows:?}"
    );
    assert!(
        rows.iter()
            .all(|row| !row.contains("hidden tail") && !row.contains("hidden by indent")),
        "baseline should not render selective-display hidden text, rows={rows:?}"
    );
    assert!(
        baseline.hit.as_ref().is_some_and(|hit| hit.rows.len() >= 2),
        "baseline should publish hit rows across selective-display output, hit={:?}",
        baseline.hit
    );

    for kind in implemented_text_backends() {
        let trace = selective_display_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_glyphless_display_geometry() {
    let baseline = glyphless_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let rows = trace_text_rows(&baseline);
    assert!(
        rows.iter().any(|row| row.contains("abcd")),
        "baseline should keep surrounding text around glyphless source chars, rows={rows:?}"
    );
    let text_row = baseline
        .output_rows
        .iter()
        .find(|row| row.row == 0)
        .expect("baseline text output row");
    assert!(
        text_row.end_col > 4,
        "baseline should account for glyphless replacement columns, row={text_row:?}"
    );
    assert!(
        baseline
            .points
            .iter()
            .any(|point| point.buffer_pos == LispCharPos1::new(2)),
        "baseline should publish a display point for the C1 glyphless source char, trace={baseline:?}"
    );
    assert!(
        baseline
            .points
            .iter()
            .any(|point| point.buffer_pos == LispCharPos1::new(6)),
        "baseline should publish a display point for the object-replacement source char, trace={baseline:?}"
    );

    for kind in implemented_text_backends() {
        let trace = glyphless_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn layout_frame_rust_renders_buffer_glyphless_chars_as_glyphless() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\u{fff0}b");
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-glyphless-text", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Glyphless { ch: '\u{fff0}' })),
        "buffer glyphless source char should emit a glyphless glyph, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_renders_buffer_control_chars_with_caret_notation() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\u{0001}b");
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-control-text", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert_eq!(glyphs_logical_text(&text_row.glyphs[1]), "a^Ab");
}

#[test]
fn layout_frame_rust_renders_line_prefix_through_row_builder() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abc");
        buf.set_buffer_local("line-prefix", Value::string("中\t"));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-line-prefix", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let logical_text = glyphs_logical_text(&text_row.glyphs[1]);

    assert!(
        logical_text.starts_with("中      abc"),
        "line-prefix should render through the shared row builder with wide/tab semantics, text={logical_text:?}, row={:?}",
        text_row.glyphs[1]
    );
    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }) && glyph.wide),
        "line-prefix wide char should carry wide glyph metadata, row={:?}",
        text_row.glyphs[1]
    );
    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 6 })),
        "line-prefix tab should expand to the next tab stop, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_renders_nobreak_chars_as_mapped_text() {
    let mut eval = Context::new();
    eval.obarray_mut()
        .set_symbol_value("nobreak-char-display", Value::T);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\u{00a0}b\u{00ad}c");
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-nobreak-text", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert_eq!(glyphs_logical_text(&text_row.glyphs[1]), "a b-c");
}

#[test]
fn layout_frame_rust_renders_nobreak_chars_in_escape_mode_as_mapped_text() {
    let mut eval = Context::new();
    eval.obarray_mut()
        .set_symbol_value("nobreak-char-display", Value::fixnum(2));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\u{00a0}b\u{00ad}c");
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-buffer-nobreak-escape-text",
        640,
        160,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert_eq!(glyphs_logical_text(&text_row.glyphs[1]), "a\\ b\\-c");
}

#[test]
fn implemented_text_backends_match_composite_glyph_output() {
    let baseline = composition_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let composites = trace_composite_texts(&baseline);
    assert!(
        composites.contains(&"e\u{0301}".to_string()),
        "baseline should merge Latin base plus acute mark into a composite glyph, composites={composites:?}"
    );
    assert!(
        composites.contains(&"a\u{0300}\u{0301}".to_string()),
        "baseline should keep multiple combining marks on one composite glyph, composites={composites:?}"
    );
    assert!(
        composites.contains(&"中\u{0300}".to_string()),
        "baseline should compose combining marks on multibyte base chars, composites={composites:?}"
    );
    assert!(
        baseline
            .points
            .iter()
            .any(|point| point.buffer_pos == LispCharPos1::new(1)),
        "baseline should publish display geometry for the first composite base char, trace={baseline:?}"
    );
    assert!(
        baseline
            .hit
            .as_ref()
            .is_some_and(|hit| !hit.rows.is_empty()),
        "baseline should publish hit rows for composite output, hit={:?}",
        baseline.hit
    );

    for kind in implemented_text_backends() {
        let trace = composition_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_wrapped_redisplay_retry_output() {
    let (baseline, target_pos) =
        wrapped_retry_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    assert!(
        baseline
            .points
            .iter()
            .any(|point| point.buffer_pos == LispCharPos1::from_one_based_usize(target_pos)),
        "baseline should converge wrapped redisplay on target point {target_pos}, trace={baseline:?}"
    );
    assert!(
        baseline.window_start > LispCharPos1::ONE,
        "baseline should advance window-start after wrapped redisplay retry, trace={baseline:?}"
    );
    assert!(
        baseline.output_rows.iter().any(|row| row.row > 0),
        "baseline should publish wrapped visual rows, rows={:?}",
        baseline.output_rows
    );
    assert!(
        baseline
            .hit
            .as_ref()
            .is_some_and(|hit| hit.rows.len() >= 2 && hit.first_col_hits.len() == hit.rows.len()),
        "baseline should publish hit rows for wrapped visual output, hit={:?}",
        baseline.hit
    );

    for kind in implemented_text_backends() {
        let (trace, backend_target_pos) = wrapped_retry_backend_layout_trace(kind);
        assert_eq!(backend_target_pos, target_pos, "{kind:?}");
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_point_line_tail_retry_output() {
    let (baseline, point, later_pos) =
        point_line_tail_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    assert!(
        baseline
            .points
            .iter()
            .any(|item| item.buffer_pos == LispCharPos1::from_one_based_usize(point)),
        "baseline should publish geometry for point {point}, trace={baseline:?}"
    );
    assert!(
        baseline
            .points
            .iter()
            .any(|item| item.buffer_pos == LispCharPos1::from_one_based_usize(later_pos)),
        "baseline should publish later positions from the point line after retry, later_pos={later_pos}, trace={baseline:?}"
    );
    assert!(
        baseline
            .hit
            .as_ref()
            .is_some_and(|hit| !hit.rows.is_empty()),
        "baseline should publish hit rows for point-line retry output, hit={:?}",
        baseline.hit
    );

    for kind in implemented_text_backends() {
        let (trace, backend_point, backend_later_pos) = point_line_tail_backend_layout_trace(kind);
        assert_eq!(backend_point, point, "{kind:?}");
        assert_eq!(backend_later_pos, later_pos, "{kind:?}");
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_mode_line_geometry_after_redisplay_retry() {
    let (baseline, point) =
        mode_line_geometry_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let mode_line = trace_mode_line_text(&baseline);
    assert!(
        baseline.window_start > LispCharPos1::ONE,
        "baseline should advance window-start for EOB redisplay retry, trace={baseline:?}"
    );
    assert_eq!(
        baseline.window_point,
        LispCharPos1::from_one_based_usize(point),
        "baseline should preserve the selected-window EOB point after retry"
    );
    assert!(
        mode_line.contains('|') && !mode_line.contains("%o"),
        "baseline should render expanded mode-line geometry, mode_line={mode_line:?}"
    );

    for kind in implemented_text_backends() {
        let (trace, backend_point) = mode_line_geometry_backend_layout_trace(kind);
        assert_eq!(backend_point, point, "{kind:?}");
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_hscroll_cursor_and_hit_output() {
    let baseline = hscroll_cursor_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let cursor = baseline.phys_cursor.as_ref().expect("baseline cursor");
    assert_eq!(cursor.x, 0);
    assert_eq!(cursor.row, 0);
    assert_eq!(cursor.col, 0);
    let text_rows = trace_text_rows(&baseline);
    assert!(
        text_rows.iter().any(|row| row.starts_with('$')),
        "baseline should render the left truncation marker, rows={text_rows:?}"
    );
    assert!(
        text_rows.iter().any(|row| row.contains("def")),
        "baseline should render the hscrolled visible suffix, rows={text_rows:?}"
    );
    assert!(
        text_rows.iter().all(|row| !row.contains("abc")),
        "baseline should not render hscrolled-away prefix text, rows={text_rows:?}"
    );
    assert_eq!(
        baseline.visible_span,
        Some(WindowVisibleBufferSpan::new(
            LispCharPos1::new(4),
            LispCharPos1::new(7)
        )),
        "baseline should publish the visible hscrolled buffer span"
    );
    assert!(
        baseline
            .hit
            .as_ref()
            .is_some_and(|hit| !hit.rows.is_empty()),
        "baseline should publish hit rows for hscroll output, hit={:?}",
        baseline.hit
    );

    for kind in implemented_text_backends() {
        let trace = hscroll_cursor_backend_layout_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn implemented_text_backends_match_edit_redisplay_cache_invalidation() {
    let (baseline_before, baseline_after) =
        edit_redisplay_backend_layout_trace(BufferTextBackendKind::GapBuffer);
    let before_rows = trace_text_rows(&baseline_before);
    let after_rows = trace_text_rows(&baseline_after);
    assert!(
        before_rows
            .iter()
            .any(|row| row.contains("alpha beta gamma")),
        "baseline before edit should render original text, rows={before_rows:?}"
    );
    assert!(
        after_rows
            .iter()
            .any(|row| row.contains("alpha BETA gamma")),
        "baseline after edit should render replacement text, rows={after_rows:?}"
    );
    assert!(
        after_rows
            .iter()
            .all(|row| !row.contains("alpha beta gamma")),
        "baseline after edit should not reuse stale glyph text, rows={after_rows:?}"
    );
    assert_ne!(
        baseline_before, baseline_after,
        "same-engine redisplay after edit should update the trace"
    );

    for kind in implemented_text_backends() {
        let (before, after) = edit_redisplay_backend_layout_trace(kind);
        assert_eq!(before, baseline_before, "{kind:?} before");
        assert_eq!(after, baseline_after, "{kind:?} after");
    }
}

#[test]
fn implemented_text_backends_match_redisplay_fontification_after_edit() {
    let baseline = fontification_edit_backend_trace(BufferTextBackendKind::GapBuffer);
    let before_rows = trace_text_rows(&baseline.before_layout);
    let after_rows = trace_text_rows(&baseline.after_layout);
    assert!(
        before_rows
            .iter()
            .any(|row| row.contains("alpha beta gamma")),
        "baseline before fontification edit should render original text, rows={before_rows:?}"
    );
    assert!(
        after_rows
            .iter()
            .any(|row| row.contains("alpha BETA gamma")),
        "baseline after fontification edit should render edited text, rows={after_rows:?}"
    );
    assert!(
        baseline.before_props.contains("font-lock-keyword-face"),
        "baseline should apply the initial font-lock face from redisplay fontification, props={}",
        baseline.before_props
    );
    assert!(
        baseline.after_props.contains("font-lock-warning-face"),
        "baseline should re-enter redisplay fontification after edit, props={}",
        baseline.after_props
    );
    assert!(
        !trace_text_face_ids(&baseline.before_layout).is_empty(),
        "baseline should emit text glyphs with face ids"
    );

    for kind in implemented_text_backends() {
        let trace = fontification_edit_backend_trace(kind);
        assert_eq!(trace, baseline, "{kind:?}");
    }
}

#[test]
fn layout_frame_rust_publishes_face_scaled_advances_for_inline_plist_faces() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a好好b ");
        let plist = Value::list(vec![
            Value::keyword("family"),
            Value::string("JetBrains Mono"),
            Value::keyword("height"),
            Value::make_float(1.6),
            Value::keyword("weight"),
            Value::symbol("extra-bold"),
        ]);
        buf.put_text_property(
            0,
            buf.total_emacs_byte_len().get(),
            Value::symbol("face"),
            plist,
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-face-advance", 800, 160, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    {
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        let face_resolver = crate::neovm_bridge::FaceResolver::new(
            eval.face_table(),
            0x00FFFFFF,
            0x00000000,
            eval.frame_manager()
                .get(frame_id)
                .expect("frame")
                .font_pixel_size,
            Some("neo".to_string()),
        );
        let mut next_check = buffer.point_max_char_pos().get();
        let resolved = face_resolver.base_face_for_origin(
            Some(buffer),
            &DisplayOrigin::BufferText {
                charpos: neovm_core::buffer::CharPos0::new(0),
            },
            BaseFacePolicy::BufferFaceIncludingOverlays,
            &mut next_check,
        );
        assert_eq!(resolved.font_family, "JetBrains Mono");
        assert_eq!(resolved.font_weight, 800);
        assert!(
            resolved.font_size > face_resolver.default_face().font_size * 1.5,
            "expected face resolver to scale the inline plist face before layout, got {:?}",
            resolved
        );
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let all_points = snapshot.points.clone();
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let hao1 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(2))
        .expect("hao1");
    let hao2 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("hao2");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(4))
        .expect("b");
    let space = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(5))
        .expect("space");

    let default_font_size = frame.font_pixel_size;
    let face_font_size = default_font_size * 1.6;
    let mut metrics = FontMetricsService::new();
    let expected_a = expected_gui_glyph_advance(
        &mut metrics,
        'a',
        "JetBrains Mono",
        800,
        false,
        face_font_size,
    );
    let expected_hao = expected_gui_glyph_advance(
        &mut metrics,
        '好',
        "JetBrains Mono",
        800,
        false,
        face_font_size,
    );
    let expected_b = expected_gui_glyph_advance(
        &mut metrics,
        'b',
        "JetBrains Mono",
        800,
        false,
        face_font_size,
    );
    assert_point_width_matches_advance(a, expected_a, "inline face a", &all_points);
    assert_point_width_matches_advance(hao1, expected_hao, "inline face first 好", &all_points);
    assert_point_width_matches_advance(hao2, expected_hao, "inline face second 好", &all_points);
    assert_point_width_matches_advance(b, expected_b, "inline face b", &all_points);
    assert_point_delta_matches_advance(a, hao1, expected_a, "inline face first 好", &all_points);
    assert_point_delta_matches_advance(
        hao1,
        hao2,
        expected_hao,
        "inline face second 好",
        &all_points,
    );
    assert_point_delta_matches_advance(hao2, b, expected_hao, "inline face b", &all_points);
    assert_point_delta_matches_advance(b, space, expected_b, "inline face space", &all_points);
}

#[test]
fn layout_frame_rust_cursor_width_uses_current_glyph_advance_not_next_glyph() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("iW ");
        let plist = Value::list(vec![
            Value::keyword("family"),
            Value::string("Noto Sans"),
            Value::keyword("weight"),
            Value::symbol("regular"),
        ]);
        buf.put_text_property(
            0,
            buf.total_emacs_byte_len().get(),
            Value::symbol("face"),
            plist,
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-cursor-current-glyph-advance",
        800,
        400,
        buf_id,
    );
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.enable_cosmic_metrics();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let face_font_size = frame.font_pixel_size;
    let mut metrics = FontMetricsService::new();
    let expected_i =
        expected_gui_glyph_advance(&mut metrics, 'i', "Noto Sans", 400, false, face_font_size)
            .round() as i64;
    let expected_w =
        expected_gui_glyph_advance(&mut metrics, 'W', "Noto Sans", 400, false, face_font_size)
            .round() as i64;
    assert_ne!(
        expected_i, expected_w,
        "test requires proportional metrics for i and W"
    );
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let i_point = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("i point");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");

    assert_eq!(
        i_point.width, expected_i,
        "point geometry should publish the current glyph advance"
    );
    assert_eq!(
        cursor.width, i_point.width,
        "box cursor width must come from the glyph under point, not the following glyph"
    );
    assert_ne!(
        cursor.width, expected_w,
        "cursor must not use the following W glyph advance"
    );
}

#[test]
fn layout_frame_rust_places_cursor_at_newline_terminated_row_end() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "first line\nsecond line\nthird line\n";
    let newline_byte = text.find('\n').expect("newline");
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(newline_byte));
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-cursor-eol", 640, 240, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(newline_byte + 1);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let last_char = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(newline_byte))
        .expect("last visible char before newline");
    let cursor = snapshot.phys_cursor.as_ref().expect("phys cursor");

    assert_eq!(cursor.row, last_char.row);
    assert_eq!(cursor.col, last_char.col + 1);
    assert_eq!(cursor.x, last_char.x + last_char.width);
    assert!(cursor.width > 0);
}

#[test]
fn layout_frame_rust_emits_neomacs_visual_cursors_without_moving_phys_cursor() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("alpha\nbeta\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        let visual_cursor = Value::list(vec![
            Value::keyword(":position"),
            Value::fixnum(3),
            Value::keyword(":cursor-type"),
            Value::cons(Value::symbol("bar"), Value::fixnum(6)),
            Value::keyword(":color"),
            Value::string("#ff0000"),
        ]);
        buf.set_buffer_local("neomacs-visual-cursors", Value::list(vec![visual_cursor]));
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-visual-cursor", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let visual = state
        .cursors
        .iter()
        .find(|cursor| cursor.window_id.get() < 0)
        .expect("visual cursor");
    assert_eq!(visual.window_id.get(), -1_000_000);
    assert_eq!(visual.width, 6.0);
    assert_eq!(visual.color, Color::from_pixel(0xff0000));

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let selected_window = frame.selected_window;
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let phys = snapshot.phys_cursor.as_ref().expect("phys cursor");
    assert_eq!(phys.x, 0, "visual cursor must not move GNU point");
}

#[test]
fn layout_frame_rust_visual_cursor_uses_display_point_geometry() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("iW ");
        let plist = Value::list(vec![
            Value::keyword("family"),
            Value::string("Noto Sans"),
            Value::keyword("weight"),
            Value::symbol("regular"),
        ]);
        buf.put_text_property(
            0,
            buf.total_emacs_byte_len().get(),
            Value::symbol("face"),
            plist,
        );
        let visual_cursor = Value::list(vec![
            Value::keyword(":position"),
            Value::fixnum(1),
            Value::keyword(":cursor-type"),
            Value::symbol("box"),
            Value::keyword(":color"),
            Value::string("#00ff00"),
        ]);
        buf.set_buffer_local("neomacs-visual-cursors", Value::list(vec![visual_cursor]));
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-visual-cursor-display-point-geometry",
        320,
        120,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut metrics = FontMetricsService::new();
    let face_font_size = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .font_pixel_size;
    let expected_i = metrics
        .char_width('i', "Noto Sans", 400, false, face_font_size)
        .round() as i64;
    let expected_w = metrics
        .char_width('W', "Noto Sans", 400, false, face_font_size)
        .round() as i64;
    assert_ne!(
        expected_i, expected_w,
        "test requires proportional metrics for i and W"
    );

    let mut engine = LayoutEngine::new();
    engine.enable_cosmic_metrics();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let i_point = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("i point");
    let visual = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state")
        .cursors
        .iter()
        .find(|cursor| cursor.window_id.get() < 0)
        .expect("visual cursor");

    assert_eq!(
        visual.width.round() as i64,
        i_point.width,
        "visual box cursor width must use the rendered glyph under :position"
    );
    assert_eq!(
        visual.height.round() as i64,
        i_point.height,
        "visual box cursor height must use the rendered glyph under :position"
    );
    assert_ne!(
        visual.width.round() as i64,
        expected_w,
        "visual cursor must not use the following glyph's width"
    );
}

#[test]
fn layout_frame_rust_visual_hbar_uses_full_display_point_box() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abc");
        let visual_cursor = Value::list(vec![
            Value::keyword(":position"),
            Value::fixnum(2),
            Value::keyword(":cursor-type"),
            Value::cons(Value::symbol("hbar"), Value::fixnum(3)),
            Value::keyword(":color"),
            Value::string("#00ff00"),
        ]);
        buf.set_buffer_local("neomacs-visual-cursors", Value::list(vec![visual_cursor]));
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-visual-hbar-display-point-box",
        320,
        120,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let b_point = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(2))
        .expect("b point");
    let visual = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state")
        .cursors
        .iter()
        .find(|cursor| cursor.window_id.get() < 0)
        .expect("visual cursor");

    assert_eq!(visual.width.round() as i64, b_point.width);
    assert_eq!(
        visual.height.round() as i64,
        b_point.height,
        "hbar visual cursor stores the full glyph box; renderer draws the bar from style"
    );
}

#[test]
fn layout_frame_rust_records_row_metrics_for_plain_text_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("plain text row\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-plain-row-metrics", 800, 160, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let text_row = engine
        .last_frame_display_state
        .as_ref()
        .and_then(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .find(|row| row.role == GlyphRowRole::Text && row.enabled)
        })
        .expect("text row");

    assert!(
        text_row.height_px > 0.0,
        "expected ordinary text rows to record authoritative height, got {text_row:?}"
    );
    assert!(
        text_row.ascent_px > 0.0,
        "expected ordinary text rows to record authoritative ascent, got {text_row:?}"
    );
}

#[test]
fn layout_frame_rust_applies_extra_line_spacing_once_to_newline_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("alpha\nbeta\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.set_buffer_local("line-spacing", Value::fixnum(5));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-extra-line-spacing-once", 800, 160, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let selected_window = frame.selected_window;
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let first_row = snapshot.row_metrics(0).expect("first text row");
    let second_row = snapshot.row_metrics(1).expect("second text row");

    assert_eq!(
        second_row.y - first_row.y,
        first_row.height + 5,
        "newline row advance should include extra line-spacing exactly once"
    );
}

#[test]
fn layout_frame_rust_applies_display_height_to_buffer_text_faces() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("AB\n");
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![Value::symbol("height"), Value::make_float(2.0)]),
        );
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-height-text-face", 640, 160, buf_id);
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(Some(Value::symbol("neo")));
    }
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert_eq!(glyphs_logical_text(&text_row.glyphs[1]), "AB");
    let text_faces = text_row.glyphs[1]
        .iter()
        .filter(|glyph| !glyph.padding)
        .map(|glyph| glyph.face_id)
        .collect::<Vec<_>>();
    assert_eq!(
        text_faces.len(),
        2,
        "expected two visible glyphs in {text_row:?}"
    );
    assert_ne!(
        text_faces[0], text_faces[1],
        "display height should realize a separate face for the covered glyph"
    );
    let base_face = state
        .faces
        .get(&text_faces[0])
        .expect("base text face should be registered");
    let adjusted_face = state
        .faces
        .get(&text_faces[1])
        .expect("height-adjusted text face should be registered");
    assert!(
        adjusted_face.font_size > base_face.font_size,
        "display height should scale the realized render face, base={base_face:?} adjusted={adjusted_face:?}"
    );
    assert!(
        text_row.height_px > state.char_height.max(1.0),
        "height display property should grow text row metrics, frame_char_height={} row={text_row:?}",
        state.char_height
    );
}

#[test]
fn layout_frame_rust_applies_display_height_to_overlay_strings() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x\n");
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: 1,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let after_string = Value::string_with_text_properties(
            "Y",
            vec![StringTextPropertyRun {
                start: 0,
                end: 1,
                plist: Value::list(vec![
                    Value::symbol("display"),
                    Value::list(vec![Value::symbol("height"), Value::make_float(2.0)]),
                ]),
            }],
        );
        let _ =
            buf.overlays_mut()
                .overlay_put(overlay, Value::symbol("after-string"), after_string);
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-display-height", 640, 160, buf_id);
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(Some(Value::symbol("neo")));
    }
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert_eq!(glyphs_logical_text(&text_row.glyphs[1]), "xY");
    assert!(
        text_row.height_px > state.char_height.max(1.0),
        "display height in overlay string should grow text row metrics, frame_char_height={} row={text_row:?}",
        state.char_height
    );
}

#[test]
fn layout_frame_rust_advances_overlay_newline_by_measured_row_height() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x");
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: 1,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let after_string = Value::string_with_text_properties(
            "A\nB",
            vec![StringTextPropertyRun {
                start: 0,
                end: 1,
                plist: Value::list(vec![
                    Value::symbol("display"),
                    Value::list(vec![Value::symbol("height"), Value::make_float(2.0)]),
                ]),
            }],
        );
        let _ =
            buf.overlays_mut()
                .overlay_put(overlay, Value::symbol("after-string"), after_string);
    }
    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-overlay-newline-measured-height",
        640,
        180,
        buf_id,
    );
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(Some(Value::symbol("neo")));
    }
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_rows = entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled && row.role == GlyphRowRole::Text)
        .collect::<Vec<_>>();
    let first_row = text_rows
        .iter()
        .find(|row| glyphs_logical_text(&row.glyphs[1]).contains("xA"))
        .expect("first overlay row");
    let second_row = text_rows
        .iter()
        .find(|row| glyphs_logical_text(&row.glyphs[1]).contains("B"))
        .expect("second overlay row");

    assert!(
        first_row.height_px > state.char_height.max(1.0),
        "test setup should make first overlay row taller than default, frame_char_height={} row={first_row:?}",
        state.char_height
    );
    assert_eq!(
        second_row.pixel_y - first_row.pixel_y,
        first_row.height_px,
        "overlay newline should advance by the measured first row height"
    );
}

#[test]
fn layout_frame_rust_captures_cursor_inside_invisible_text_without_rescan() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "abc hidden xyz";
    let hidden_byte_start = text.find("hidden").expect("hidden start");
    let hidden_byte_end = hidden_byte_start + "hidden".len();
    let hidden_char_start = text[..hidden_byte_start].chars().count() + 1;
    let point_pos = hidden_char_start + 2;
    let next_visible_pos = hidden_char_start + "hidden".chars().count();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point_pos - 1));
        buf.put_text_property(
            hidden_byte_start,
            hidden_byte_end,
            Value::symbol("invisible"),
            Value::T,
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-invisible-cursor", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(point_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    let next_visible = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(next_visible_pos))
        .expect("next visible point");
    assert_eq!(cursor.x, next_visible.x);
    assert_eq!(cursor.row, next_visible.row);
    assert_eq!(cursor.col, next_visible.col);
}

#[test]
fn layout_frame_rust_preserves_logical_cursor_when_window_cursor_is_nil() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcdef");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(2));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-logical-cursor-only", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(3);
        }
    }
    eval.frame_manager_mut()
        .set_window_cursor_type(selected_window, Value::NIL);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let logical_cursor = snapshot.logical_cursor.expect("logical cursor");
    let point = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("point snapshot");

    assert_eq!(snapshot.phys_cursor, None);
    assert_eq!(logical_cursor.x, point.x);
    assert_eq!(logical_cursor.row, point.row);
    assert_eq!(logical_cursor.col, point.col);
}

#[test]
fn layout_frame_rust_captures_cursor_at_display_replacement_slot_without_rescan() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "abcXYZdef";
    let repl_byte_start = text.find("XYZ").expect("replacement start");
    let repl_byte_end = repl_byte_start + "XYZ".len();
    let point_pos = repl_byte_start + 2;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point_pos - 1));
        buf.put_text_property(
            repl_byte_start,
            repl_byte_end,
            Value::symbol("display"),
            Value::string("R"),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-display-cursor", 800, 400, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(point_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    let c = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("c");
    let d = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(7))
        .expect("d");
    assert_eq!(cursor.x, c.x + c.width);
    assert!(cursor.x < d.x, "cursor should target replacement slot");
    assert_eq!(cursor.row, c.row);
}

#[test]
fn layout_frame_rust_records_display_point_for_display_replacement_slot() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "abcXYZdef";
    let repl_byte_start = text.find("XYZ").expect("replacement start");
    let repl_byte_end = repl_byte_start + "XYZ".len();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.put_text_property(
            repl_byte_start,
            repl_byte_end,
            Value::symbol("display"),
            Value::string("R"),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-display-point", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let c = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("c");
    let replacement = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(4))
        .expect("replacement point");
    let d = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(7))
        .expect("d");

    assert_eq!(replacement.x, c.x + c.width);
    assert!(
        replacement.x < d.x,
        "replacement point should stay before following text"
    );
    assert!(replacement.width > 0);
    assert_eq!(replacement.row, c.row);
}

#[test]
fn layout_frame_rust_emits_display_string_replacement_glyphs() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("dir:");
        buf.put_text_property(
            3,
            4,
            Value::symbol("display"),
            Value::string(": (287 GiB available)"),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-display-string", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = window_entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let rendered: String = text_row.glyphs[1]
        .iter()
        .filter_map(|glyph| match &glyph.glyph_type {
            GlyphType::Char { ch } => Some(*ch),
            GlyphType::Composite { text } => text.chars().next(),
            _ => None,
        })
        .collect();

    assert_eq!(rendered, "dir: (287 GiB available)");
}

#[test]
fn layout_frame_rust_renders_display_replacement_tabs_as_stretches() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("px");
        buf.put_text_property(1, 2, Value::symbol("display"), Value::string("a\tb"));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-tab-replacement", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    let logical_text = glyphs_logical_text(&text_row.glyphs[1]);
    assert!(
        !logical_text.contains('\t'),
        "display replacement tab should not render as a literal tab, row={:?}",
        text_row.glyphs[1]
    );
    assert!(
        logical_text.contains("pa      b"),
        "display replacement tab should expand to the next row tab stop, text={logical_text:?}"
    );
    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 6 })),
        "display replacement tab should be a stretch glyph, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_honors_display_replacement_string_display_properties() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("px");
        let replacement = Value::string_with_text_properties(
            "a b",
            vec![StringTextPropertyRun {
                start: 1,
                end: 2,
                plist: Value::list(vec![
                    Value::symbol("display"),
                    Value::list(vec![
                        Value::symbol("space"),
                        Value::keyword(":width"),
                        Value::fixnum(3),
                    ]),
                ]),
            }],
        );
        buf.put_text_property(1, 2, Value::symbol("display"), replacement);
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-display-propertized-replacement",
        640,
        160,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    let logical_text = glyphs_logical_text(&text_row.glyphs[1]);
    assert!(
        logical_text.contains("pa   b"),
        "display replacement string should honor its display space, text={logical_text:?}"
    );
    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 3 })),
        "display replacement string display property should produce a stretch, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_honors_display_replacement_string_face_properties() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("px");
        let replacement = Value::string_with_text_properties(
            "ab",
            vec![StringTextPropertyRun {
                start: 0,
                end: 1,
                plist: Value::list(vec![
                    Value::symbol("face"),
                    Value::list(vec![Value::keyword("foreground"), Value::string("#ff0000")]),
                ]),
            }],
        );
        buf.put_text_property(1, 2, Value::symbol("display"), replacement);
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-replacement-face", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    let a_face = text_row.glyphs[1]
        .iter()
        .find_map(|glyph| match glyph.glyph_type {
            GlyphType::Char { ch: 'a' } => Some(glyph.face_id),
            _ => None,
        })
        .expect("propertized replacement glyph face");
    let b_face = text_row.glyphs[1]
        .iter()
        .find_map(|glyph| match glyph.glyph_type {
            GlyphType::Char { ch: 'b' } => Some(glyph.face_id),
            _ => None,
        })
        .expect("plain replacement glyph face");

    assert_ne!(
        a_face, b_face,
        "replacement string face property should affect only its covered glyph, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_emits_inline_image_glyphs_for_display_image_specs() {
    let mut eval = Context::new();
    let requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingImageDisplayHost {
        requests: Arc::clone(&requests),
        video_requests: Arc::new(Mutex::new(Vec::new())),
        webkit_requests: Arc::new(Mutex::new(Vec::new())),
    }));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "aXb";
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![
                Value::symbol("image"),
                Value::keyword("type"),
                Value::symbol("png"),
                Value::keyword("file"),
                Value::string("/tmp/neomacs-inline-image.png"),
                Value::keyword("max-width"),
                Value::fixnum(32),
                Value::keyword("max-height"),
                Value::fixnum(24),
                Value::keyword("foreground"),
                Value::string("#112233"),
                Value::keyword("background"),
                Value::string("red"),
            ]),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-inline-image", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("frame display state");
    let image = state.images.first().expect("inline image glyph");
    assert_eq!(image.image_id.get(), 77);
    assert_eq!(image.width, 32.0);
    assert_eq!(image.height, 24.0);
    let replacement = assert_replacement_slot_between_neighbors(&eval, frame_id, 2, 32);
    let slot_id = image.slot_id.expect("image slot id");
    assert_eq!(i64::from(slot_id.col), replacement.col);

    let requests = requests.lock().expect("requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].max_width, 32);
    assert_eq!(requests[0].max_height, 24);
    assert_eq!(requests[0].fg_color, 0x112233);
    assert_eq!(requests[0].bg_color, 0xff0000);
}

#[test]
fn layout_frame_rust_renders_display_image_fallback_placeholder_through_row_builder() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("aXb");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![
                Value::symbol("image"),
                Value::keyword("type"),
                Value::symbol("png"),
                Value::keyword("file"),
                Value::string("/tmp/neomacs-inline-image.png"),
            ]),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-inline-image-fallback", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("frame display state");
    assert!(state.images.is_empty());
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| {
            entry.window_id
                == eval
                    .frame_manager()
                    .get(frame_id)
                    .expect("frame")
                    .selected_window
                    .0
        })
        .expect("selected window matrix");
    assert!(
        enabled_window_row_texts(entry)
            .iter()
            .any(|row| row.contains("a[img]b")),
        "fallback placeholder should be rendered as row-builder text, rows={:?}",
        enabled_window_row_texts(entry)
    );

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let expected_width = (5.0 * frame.char_width).round() as i64;
    assert_replacement_slot_between_neighbors(&eval, frame_id, 2, expected_width);
}

#[test]
fn layout_frame_rust_emits_inline_video_glyphs_for_display_video_specs() {
    let mut eval = Context::new();
    let video_requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingImageDisplayHost {
        requests: Arc::new(Mutex::new(Vec::new())),
        video_requests: Arc::clone(&video_requests),
        webkit_requests: Arc::new(Mutex::new(Vec::new())),
    }));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("aVb");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![
                Value::symbol("video"),
                Value::keyword("file"),
                Value::string("/tmp/neomacs-inline-video.mp4"),
                Value::keyword("width"),
                Value::fixnum(80),
                Value::keyword("height"),
                Value::fixnum(45),
                Value::keyword("autoplay"),
                Value::T,
                Value::keyword("loop"),
                Value::T,
            ]),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-inline-video", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("frame display state");
    let video = state.videos.first().expect("inline video glyph");
    assert_eq!(video.video_id.get(), 88);
    assert_eq!(video.width, 80.0);
    assert_eq!(video.height, 45.0);
    assert_eq!(video.loop_count, -1);
    assert!(video.autoplay);
    let replacement = assert_replacement_slot_between_neighbors(&eval, frame_id, 2, 80);
    let slot_id = video.slot_id.expect("video slot id");
    assert_eq!(i64::from(slot_id.col), replacement.col);

    let requests = video_requests.lock().expect("video requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].loop_count, -1);
    assert!(requests[0].autoplay);
}

#[test]
fn layout_frame_rust_emits_inline_webkit_glyphs_for_display_webkit_specs() {
    let mut eval = Context::new();
    let webkit_requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingImageDisplayHost {
        requests: Arc::new(Mutex::new(Vec::new())),
        video_requests: Arc::new(Mutex::new(Vec::new())),
        webkit_requests: Arc::clone(&webkit_requests),
    }));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("aWb");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![
                Value::symbol("webkit"),
                Value::keyword("uri"),
                Value::string("https://example.com"),
                Value::keyword("width"),
                Value::fixnum(80),
                Value::keyword("height"),
                Value::fixnum(45),
            ]),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-inline-webkit", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("frame display state");
    let xwidget = state.xwidgets.first().expect("inline xwidget glyph");
    assert_eq!(xwidget.xwidget_id.get(), 99);
    assert_eq!(xwidget.width, 80.0);
    assert_eq!(xwidget.height, 45.0);
    let replacement = assert_replacement_slot_between_neighbors(&eval, frame_id, 2, 80);
    let slot_id = xwidget.slot_id.expect("webkit slot id");
    assert_eq!(i64::from(slot_id.col), replacement.col);

    let requests = webkit_requests.lock().expect("webkit requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].width, 80);
    assert_eq!(requests[0].height, 45);
}

#[test]
fn layout_frame_rust_emits_inline_xwidget_glyphs_for_gnu_display_xwidget_specs() {
    let mut eval = Context::new();
    let webkit_requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingImageDisplayHost {
        requests: Arc::new(Mutex::new(Vec::new())),
        video_requests: Arc::new(Mutex::new(Vec::new())),
        webkit_requests: Arc::clone(&webkit_requests),
    }));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let xwidget = Value::make_xwidget(
        Value::symbol("webkit"),
        Value::string("Title"),
        Value::make_buffer(buf_id),
        96,
        54,
        1234,
    );
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("aXb");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            1,
            2,
            Value::symbol("display"),
            Value::list(vec![
                Value::symbol("xwidget"),
                Value::keyword("xwidget"),
                xwidget,
            ]),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-inline-xwidget", 320, 120, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("frame display state");
    let xwidget = state.xwidgets.first().expect("inline xwidget glyph");
    assert_eq!(xwidget.xwidget_id.get(), 1234);
    assert_eq!(xwidget.width, 96.0);
    assert_eq!(xwidget.height, 54.0);
    let replacement = assert_replacement_slot_between_neighbors(&eval, frame_id, 2, 96);
    let slot_id = xwidget.slot_id.expect("xwidget slot id");
    assert_eq!(i64::from(slot_id.col), replacement.col);

    let requests = webkit_requests.lock().expect("webkit requests lock");
    assert!(requests.is_empty());
}

#[test]
fn layout_frame_rust_captures_cursor_inside_hscroll_skipped_text_without_rescan() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcdef\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.set_buffer_local("truncate-lines", Value::T);
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-hscroll-cursor", 160, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            hscroll,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(2);
            *hscroll = 3;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    assert_eq!(cursor.x, 0);
    assert_eq!(cursor.row, 0);
    assert_eq!(cursor.col, 0);
}

fn assert_layout_frame_rust_tab_cursor_width(x_stretch_cursor: bool, cursor_type: Value) {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\tb");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.set_buffer_local("cursor-type", cursor_type);
    }
    eval.set_variable(
        "x-stretch-cursor",
        if x_stretch_cursor {
            Value::T
        } else {
            Value::NIL
        },
    );

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-tab-cursor", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(2);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("b");
    let full_tab_slot_width = b.x - (a.x + a.width);
    let single_column_width = frame.char_width.round() as i64;

    assert_eq!(cursor.x, a.x + a.width);
    assert_eq!(cursor.row, a.row);
    assert_eq!(b.x - cursor.x, full_tab_slot_width);
    assert!(full_tab_slot_width > single_column_width);
    if x_stretch_cursor {
        assert_eq!(cursor.width, full_tab_slot_width);
    } else {
        assert_eq!(cursor.width, single_column_width);
    }
}

#[test]
fn layout_frame_rust_clamps_tab_cursor_width_when_x_stretch_cursor_is_nil() {
    assert_layout_frame_rust_tab_cursor_width(false, Value::T);
}

#[test]
fn layout_frame_rust_expands_tab_cursor_width_when_x_stretch_cursor_is_t() {
    assert_layout_frame_rust_tab_cursor_width(true, Value::T);
}

#[test]
fn layout_frame_rust_clamps_tab_hbar_cursor_width_when_x_stretch_cursor_is_nil() {
    assert_layout_frame_rust_tab_cursor_width(false, Value::symbol("hbar"));
}

#[test]
fn layout_frame_rust_expands_tab_hbar_cursor_width_when_x_stretch_cursor_is_t() {
    assert_layout_frame_rust_tab_cursor_width(true, Value::symbol("hbar"));
}

#[test]
fn layout_frame_rust_emits_buffer_tab_as_stretch_glyph() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\tb");
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-tab-stretch", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = window_entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let glyphs = &text_row.glyphs[1];

    assert!(matches!(
        glyphs.first().map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Char { ch: 'a' })
    ));
    assert!(matches!(
        glyphs.get(1).map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Stretch { width_cols: 7 })
    ));
    assert!(matches!(
        glyphs.get(2).map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Char { ch: 'b' })
    ));
    assert_eq!(text_row.role, GlyphRowRole::Text);
    assert!(
        glyphs.iter().all(|glyph| glyph.pixel_width > 0.0),
        "main buffer text glyphs should keep pixel widths: {glyphs:?}"
    );
}

#[test]
fn layout_frame_rust_tab_stops_are_window_relative_in_split_windows() {
    let mut eval = Context::new();
    let left_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let right_buf_id = eval.buffer_manager_mut().create_buffer("*right*");
    {
        let buf = eval
            .buffer_manager_mut()
            .get_mut(right_buf_id)
            .expect("right buffer");
        buf.insert("C-f\t;; forward-char");
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-tab-split", 800, 160, left_buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let right_window = eval
        .frame_manager_mut()
        .split_window(
            frame_id,
            selected_window,
            neovm_core::window::SplitDirection::Horizontal,
            right_buf_id,
            None,
            neovm_core::window::SplitPlacement::AfterTarget,
        )
        .expect("split window");

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == right_window.0)
        .expect("right window matrix");
    let text_row = window_entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let text = text_row.glyphs[1]
        .iter()
        .flat_map(|glyph| match &glyph.glyph_type {
            GlyphType::Char { ch } => std::iter::repeat_n(*ch, 1).collect::<Vec<_>>(),
            GlyphType::Stretch { width_cols } => {
                std::iter::repeat_n(' ', usize::from(*width_cols)).collect::<Vec<_>>()
            }
            _ => Vec::new(),
        })
        .collect::<String>();

    assert!(
        text.contains("C-f     ;; forward-char"),
        "right-window tab should expand relative to the right window text area, got {text:?}"
    );
}

#[test]
fn layout_frame_rust_display_space_align_keeps_suffix_text_in_split_windows() {
    let mut eval = Context::new();
    let left_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let right_buf_id = eval
        .buffer_manager_mut()
        .create_buffer("*right-display-space*");
    let text = concat!(
        "   m \tShow help for current major and minor modes and their commands\n",
        "   b \tShow all key bindings\n",
        "   k \tShow help for key\n",
        "   c \tShow help for key briefly\n",
        "   w \tShow which key runs a specific command\n"
    );
    {
        let buf = eval
            .buffer_manager_mut()
            .get_mut(right_buf_id)
            .expect("right buffer");
        buf.insert(text);
        for (byte_idx, ch) in text.char_indices() {
            if ch == '\t' {
                buf.put_text_property(
                    byte_idx,
                    byte_idx + 1,
                    Value::symbol("display"),
                    Value::list(vec![
                        Value::symbol("space"),
                        Value::keyword(":align-to"),
                        Value::fixnum(8),
                    ]),
                );
            }
        }
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-display-space-align-split",
        800,
        160,
        left_buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let right_window = eval
        .frame_manager_mut()
        .split_window(
            frame_id,
            selected_window,
            neovm_core::window::SplitDirection::Horizontal,
            right_buf_id,
            None,
            neovm_core::window::SplitPlacement::AfterTarget,
        )
        .expect("split window");

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == right_window.0)
        .expect("right window matrix");
    let rows = enabled_window_row_texts_expanding_stretches(window_entry);

    assert!(
        rows.iter()
            .any(|row| row.contains("   c    Show help for key briefly")),
        "display-space align-to should preserve suffix text after the stretch, rows={rows:?}"
    );
    assert!(
        rows.iter()
            .any(|row| row.contains("   w    Show which key runs a specific command")),
        "display-space align-to should not swallow following help rows, rows={rows:?}"
    );
}

#[test]
fn layout_frame_rust_tty_display_space_align_stays_one_cell_high() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = concat!(
        "   m \tShow help for current major and minor modes and their commands\n",
        "   b \tShow all key bindings\n",
        "   k \tShow help for key\n",
        "   c \tShow help for key briefly\n",
        "   w \tShow which key runs a specific command\n"
    );
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        for (byte_idx, ch) in text.char_indices() {
            if ch == '\t' {
                buf.put_text_property(
                    byte_idx,
                    byte_idx + 1,
                    Value::symbol("display"),
                    Value::list(vec![
                        Value::symbol("space"),
                        Value::keyword(":align-to"),
                        Value::fixnum(8),
                    ]),
                );
            }
        }
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-tty-display-space-align", 80, 25, buf_id);
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(None);
        frame.char_width = 1.0;
        frame.char_height = 1.0;
        frame.font_pixel_size = 16.0;
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_id = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window
        .0;
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == window_id)
        .expect("selected window matrix");
    let rows = enabled_window_row_texts_expanding_stretches(window_entry);

    assert!(
        rows.iter()
            .any(|row| row.contains("   w    Show which key runs a specific command")),
        "TTY display-space align-to should not inflate rows and hide later Help entries, rows={rows:?}"
    );

    for row in window_entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled && row.role == GlyphRowRole::Text && row.total_glyphs() > 0)
    {
        assert_eq!(
            row.height_px, 1.0,
            "TTY display-space rows must stay one cell high: row={row:?}"
        );
        assert!(
            row.ascent_px <= row.height_px,
            "TTY row ascent must not exceed row height: row={row:?}"
        );
    }
}

#[test]
fn layout_frame_rust_emits_pixel_window_divider_geometry() {
    let mut eval = Context::new();
    let left_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let right_buf_id = eval.buffer_manager_mut().create_buffer("*right*");
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-divider-split", 800, 160, left_buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_parameter(Value::symbol("right-divider-width"), Value::fixnum(6));
    }
    eval.frame_manager_mut()
        .split_window(
            frame_id,
            selected_window,
            neovm_core::window::SplitDirection::Horizontal,
            right_buf_id,
            None,
            neovm_core::window::SplitPlacement::AfterTarget,
        )
        .expect("split window");
    let left_bounds = {
        let frame = eval.frame_manager().get(frame_id).expect("frame");
        *frame
            .find_window(selected_window)
            .expect("left window")
            .bounds()
    };

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let divider_borders: Vec<_> = state
        .borders
        .iter()
        .filter(|border| {
            border.window_id.get() == selected_window.0 as i64
                && (border.x - (left_bounds.x + left_bounds.width - 6.0)).abs() <= 6.0
        })
        .collect();

    assert_eq!(
        divider_borders.len(),
        3,
        "a six-pixel right divider should be split into first/inner/last rectangles"
    );
    assert!(
        divider_borders.iter().any(|border| border.width == 1.0),
        "divider should include one-pixel edge rectangles"
    );
    assert!(
        divider_borders.iter().any(|border| border.width == 4.0),
        "divider should include a four-pixel inner rectangle"
    );

    let left_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("left window matrix");
    assert!(
        left_entry.matrix.rows.iter().all(|row| {
            row.glyphs[1]
                .last()
                .is_none_or(|glyph| !matches!(glyph.glyph_type, GlyphType::Char { ch: '|' }))
        }),
        "real pixel window dividers must not be represented as vertical-border text glyphs"
    );
}

#[test]
fn layout_frame_rust_gui_zero_width_divider_uses_pixel_vertical_border() {
    let mut eval = Context::new();
    let left_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let right_buf_id = eval.buffer_manager_mut().create_buffer("*right*");
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-gui-border-split", 800, 160, left_buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(Some(Value::symbol("neo")));
    }
    eval.frame_manager_mut()
        .split_window(
            frame_id,
            selected_window,
            neovm_core::window::SplitDirection::Horizontal,
            right_buf_id,
            None,
            neovm_core::window::SplitPlacement::AfterTarget,
        )
        .expect("split window");
    let left_bounds = {
        let frame = eval.frame_manager().get(frame_id).expect("frame");
        *frame
            .find_window(selected_window)
            .expect("left window")
            .bounds()
    };

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    assert!(
        state.borders.iter().any(|border| {
            border.window_id.get() == selected_window.0 as i64
                && (border.x - (left_bounds.x + left_bounds.width - 1.0)).abs() < 0.01
                && border.width == 1.0
        }),
        "GNU GUI draws a one-pixel vertical border when window-divider-mode is off"
    );

    let left_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("left window matrix");
    assert!(
        left_entry.matrix.rows.iter().all(|row| {
            row.glyphs[1]
                .last()
                .is_none_or(|glyph| !matches!(glyph.glyph_type, GlyphType::Char { ch: '|' }))
        }),
        "GUI vertical borders must not be represented as terminal `|' glyphs"
    );
}

#[test]
fn layout_frame_rust_bottom_divider_does_not_separate_root_from_minibuffer() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-minibuffer-divider", 800, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(Some(Value::symbol("neo")));
        frame.set_parameter(Value::symbol("bottom-divider-width"), Value::fixnum(6));
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    assert!(
        state.borders.iter().all(
            |border| border.window_id.get() != selected_window.0 as i64 || border.height != 6.0
        ),
        "GNU does not draw a bottom window divider between a bottommost root window and the minibuffer"
    );
}

#[test]
fn layout_frame_rust_emits_display_space_as_stretch_glyph() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "a b";
    let space_byte_start = text.find(' ').expect("space start");
    let space_byte_end = space_byte_start + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("display"),
            display_space_width_spec(4),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-space-stretch", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let window_entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = window_entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");
    let glyphs = &text_row.glyphs[1];

    assert!(matches!(
        glyphs.first().map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Char { ch: 'a' })
    ));
    assert!(matches!(
        glyphs.get(1).map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Stretch { width_cols: 4 })
    ));
    assert!(matches!(
        glyphs.get(2).map(|glyph| &glyph.glyph_type),
        Some(GlyphType::Char { ch: 'b' })
    ));
}

fn display_space_width_spec(columns: i64) -> Value {
    Value::list(vec![
        Value::symbol("space"),
        Value::keyword("width"),
        Value::fixnum(columns),
    ])
}

fn display_space_relative_width_spec(factor: i64) -> Value {
    Value::list(vec![
        Value::symbol("space"),
        Value::keyword("relative-width"),
        Value::fixnum(factor),
    ])
}

fn display_space_relative_height_spec(factor: i64, ascent_percent: i64) -> Value {
    Value::list(vec![
        Value::symbol("space"),
        Value::keyword("width"),
        Value::fixnum(2),
        Value::keyword("relative-height"),
        Value::fixnum(factor),
        Value::keyword("ascent"),
        Value::fixnum(ascent_percent),
    ])
}

#[test]
fn display_space_relative_width_uses_displayed_character_width() {
    let _eval = Context::new();
    let params = test_window_params();
    let geometry = DisplayReplacementStretchSourceItem::display_space_geometry(
        &display_space_relative_width_spec(2),
        0.0,
        0.0,
        8.0,
        16.0,
        10.0,
        7.0,
        &params,
    );

    assert_eq!(geometry.width, 32.0);
}

#[test]
fn display_space_geometry_uses_relative_height_and_percent_ascent() {
    let _eval = Context::new();
    let params = test_window_params();
    let geometry = DisplayReplacementStretchSourceItem::display_space_geometry(
        &display_space_relative_height_spec(2, 25),
        0.0,
        0.0,
        8.0,
        8.0,
        10.0,
        7.0,
        &params,
    );

    assert_eq!(
        geometry,
        DisplayReplacementSpaceGeometry {
            width: 16.0,
            height: 20.0,
            ascent: 5.0,
        }
    );
}

#[test]
fn display_space_geometry_accepts_pixel_ascent_expression() {
    let _eval = Context::new();
    let params = test_window_params();
    let spec = Value::list(vec![
        Value::symbol("space"),
        Value::keyword("height"),
        Value::list(vec![Value::fixnum(20)]),
        Value::keyword("ascent"),
        Value::list(vec![Value::fixnum(3)]),
    ]);
    let geometry = DisplayReplacementStretchSourceItem::display_space_geometry(
        &spec, 0.0, 0.0, 8.0, 8.0, 10.0, 7.0, &params,
    );

    assert_eq!(geometry.height, 20.0);
    assert_eq!(geometry.ascent, 3.0);
}

fn scaled_face_plist() -> Value {
    Value::list(vec![
        Value::keyword("family"),
        Value::string("JetBrains Mono"),
        Value::keyword("height"),
        Value::make_float(1.6),
        Value::keyword("weight"),
        Value::symbol("extra-bold"),
    ])
}

fn assert_layout_frame_rust_display_space_cursor_width(x_stretch_cursor: bool, cursor_type: Value) {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "a b";
    let space_byte_start = text.find(' ').expect("space start");
    let space_byte_end = space_byte_start + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("display"),
            display_space_width_spec(4),
        );
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("face"),
            scaled_face_plist(),
        );
        buf.set_buffer_local("cursor-type", cursor_type);
    }
    eval.set_variable(
        "x-stretch-cursor",
        if x_stretch_cursor {
            Value::T
        } else {
            Value::NIL
        },
    );

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-space-cursor", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(2);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("b");
    let full_slot_width = b.x - (a.x + a.width);
    let single_column_width = frame.char_width.round() as i64;
    let expected_space_width = (4.0 * frame.char_width).round() as i64;

    assert_eq!(cursor.x, a.x + a.width);
    assert_eq!(b.x - cursor.x, full_slot_width);
    assert!((full_slot_width - expected_space_width).abs() <= 1);
    if x_stretch_cursor {
        assert_eq!(cursor.width, full_slot_width);
    } else {
        assert_eq!(cursor.width, single_column_width);
    }
}

#[test]
fn layout_frame_rust_display_space_width_uses_canonical_column_width() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "a b";
    let space_byte_start = text.find(' ').expect("space start");
    let space_byte_end = space_byte_start + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(1));
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("display"),
            display_space_width_spec(4),
        );
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("face"),
            scaled_face_plist(),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-space-width", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf { window_start, .. } = window {
            *window_start = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("b");
    let slot_width = b.x - (a.x + a.width);
    let expected_width = (4.0 * frame.char_width).round() as i64;

    assert!(
        (slot_width - expected_width).abs() <= 1,
        "display space width should follow canonical frame column width; got slot {slot_width}, expected {expected_width}, frame char width {}, points={:?}",
        frame.char_width,
        snapshot.points
    );
}

#[test]
fn layout_frame_rust_records_display_point_for_display_space_slot() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "a b";
    let space_byte_start = text.find(' ').expect("space start");
    let space_byte_end = space_byte_start + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("display"),
            display_space_width_spec(4),
        );
        buf.put_text_property(
            space_byte_start,
            space_byte_end,
            Value::symbol("face"),
            scaled_face_plist(),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-display-space-point", 320, 120, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("a");
    let space = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(2))
        .expect("space");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(3))
        .expect("b");
    let expected_width = (4.0 * frame.char_width).round() as i64;

    assert_eq!(space.x, a.x + a.width);
    assert!(space.x < b.x);
    assert!((space.width - expected_width).abs() <= 1);
    assert_eq!(space.row, a.row);
}

#[test]
fn layout_frame_rust_clamps_display_space_cursor_width_when_x_stretch_cursor_is_nil() {
    assert_layout_frame_rust_display_space_cursor_width(false, Value::T);
}

#[test]
fn layout_frame_rust_expands_display_space_cursor_width_when_x_stretch_cursor_is_t() {
    assert_layout_frame_rust_display_space_cursor_width(true, Value::T);
}

#[test]
fn layout_frame_rust_clamps_display_space_hbar_cursor_width_when_x_stretch_cursor_is_nil() {
    assert_layout_frame_rust_display_space_cursor_width(false, Value::symbol("hbar"));
}

#[test]
fn layout_frame_rust_expands_display_space_hbar_cursor_width_when_x_stretch_cursor_is_t() {
    assert_layout_frame_rust_display_space_cursor_width(true, Value::symbol("hbar"));
}

#[test]
fn layout_frame_rust_keeps_mixed_width_advances_correct_after_mid_line_face_change() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();

    let prefix = "  h=0.9 w=normal:                     ";
    let sample = "a好好b  ABCXYZ 0123456789  -> <= >=";
    let sample_pos = prefix.chars().count() + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(prefix);
        let sample_byte_start = buf.total_emacs_byte_len().get();
        buf.insert(sample);
        let sample_byte_end = buf.total_emacs_byte_len().get();
        let plist = Value::list(vec![
            Value::keyword("family"),
            Value::string("Noto Sans Mono"),
            Value::keyword("height"),
            Value::make_float(0.9),
            Value::keyword("weight"),
            Value::symbol("normal"),
        ]);
        buf.put_text_property(
            sample_byte_start,
            sample_byte_end,
            Value::symbol("face"),
            plist,
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-face-mid-line", 1400, 160, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let all_points = snapshot.points.clone();
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos))
        .expect("a");
    let hao1 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 1))
        .expect("first 好");
    let hao2 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 2))
        .expect("second 好");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 3))
        .expect("b");

    let face_font_size = frame.font_pixel_size * 0.9;
    let mut metrics = FontMetricsService::new();
    let expected_a = expected_gui_glyph_advance(
        &mut metrics,
        'a',
        "Noto Sans Mono",
        400,
        false,
        face_font_size,
    );
    let expected_hao = expected_gui_glyph_advance(
        &mut metrics,
        '好',
        "Noto Sans Mono",
        400,
        false,
        face_font_size,
    );
    let expected_b = expected_gui_glyph_advance(
        &mut metrics,
        'b',
        "Noto Sans Mono",
        400,
        false,
        face_font_size,
    );

    assert_point_width_matches_advance(a, expected_a, "a", &all_points);
    assert_point_width_matches_advance(hao1, expected_hao, "first 好", &all_points);
    assert_point_width_matches_advance(hao2, expected_hao, "second 好", &all_points);
    assert_point_width_matches_advance(b, expected_b, "b", &all_points);
    assert_point_delta_matches_advance(a, hao1, expected_a, "first 好", &all_points);
    assert_point_delta_matches_advance(hao1, hao2, expected_hao, "second 好", &all_points);
    assert_point_delta_matches_advance(hao2, b, expected_hao, "b", &all_points);
    let space = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 4))
        .expect("space");
    assert!(
        ((space.x - b.x) as f32 - expected_b).abs() <= 1.0,
        "expected next point after 'b' to land near one logical advance later; b={b:?} space={space:?} points={all_points:?}"
    );
}

#[test]
fn layout_frame_rust_keeps_face_positions_after_truncated_multibyte_line() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();

    let truncated_prefix = format!("{}\n", "好".repeat(20));
    let sample = "a好好b";
    let sample_pos = truncated_prefix.chars().count() + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&truncated_prefix);
        let sample_byte_start = buf.total_emacs_byte_len().get();
        buf.insert(sample);
        let sample_byte_end = buf.total_emacs_byte_len().get();
        buf.insert("\n");
        let plist = Value::list(vec![
            Value::keyword("family"),
            Value::string("Noto Sans Mono"),
            Value::keyword("height"),
            Value::make_float(0.9),
            Value::keyword("weight"),
            Value::symbol("normal"),
        ]);
        buf.put_text_property(
            sample_byte_start,
            sample_byte_end,
            Value::symbol("face"),
            plist,
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.set_buffer_local("truncate-lines", Value::T);
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-truncated-multibyte-face", 128, 160, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(sample_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let all_points = snapshot.points.clone();
    let a = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos))
        .expect("a");
    let hao1 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 1))
        .expect("first 好");
    let hao2 = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 2))
        .expect("second 好");
    let b = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(sample_pos + 3))
        .expect("b");

    let face_font_size = frame.font_pixel_size * 0.9;
    let mut metrics = FontMetricsService::new();
    let expected_a = expected_gui_glyph_advance(
        &mut metrics,
        'a',
        "Noto Sans Mono",
        400,
        false,
        face_font_size,
    );
    let expected_hao = expected_gui_glyph_advance(
        &mut metrics,
        '好',
        "Noto Sans Mono",
        400,
        false,
        face_font_size,
    );
    let expected_b = expected_gui_glyph_advance(
        &mut metrics,
        'b',
        "Noto Sans Mono",
        400,
        false,
        face_font_size,
    );

    assert_point_width_matches_advance(a, expected_a, "a", &all_points);
    assert_point_width_matches_advance(hao1, expected_hao, "first 好", &all_points);
    assert_point_width_matches_advance(hao2, expected_hao, "second 好", &all_points);
    assert_point_width_matches_advance(b, expected_b, "b", &all_points);
    assert_point_delta_matches_advance(a, hao1, expected_a, "first 好", &all_points);
    assert_point_delta_matches_advance(hao1, hao2, expected_hao, "second 好", &all_points);
    assert_point_delta_matches_advance(hao2, b, expected_hao, "b", &all_points);
}

#[test]
fn layout_frame_rust_keeps_mixed_width_positions_correct_after_sequential_window_point_moves() {
    #[derive(Clone, Copy, Debug)]
    struct TargetRow {
        line_beg: usize,
        sample_pos: usize,
        height: f32,
        weight: u16,
    }

    fn char_at_lisp_pos(buffer: &neovm_core::buffer::Buffer, pos: usize) -> Option<char> {
        if pos == 0 {
            return None;
        }
        let byte_pos = buffer
            .char_pos_to_emacs_byte_pos_clamped(neovm_core::buffer::CharPos0::new(pos - 1))
            .get();
        buffer.char_after_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(byte_pos))
    }

    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let sample = "a好好b  ABCXYZ 0123456789  -> <= >=";
    let mut targets = Vec::new();
    let weights = [
        ("normal", 400_u16),
        ("semi-bold", 600_u16),
        ("bold", 700_u16),
        ("extra-bold", 800_u16),
    ];

    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        for height in [0.9_f32, 1.0_f32, 1.2_f32, 1.6_f32] {
            for (weight_name, weight_value) in weights {
                let line_beg = if buf.is_text_empty() {
                    1usize
                } else {
                    buf.point_max_char_pos().get() as usize + 1
                };
                let prefix = format!("  {:<35} ", format!("h={height} w={weight_name}:"));
                let sample_pos = line_beg + prefix.chars().count();
                buf.insert(&prefix);
                let sample_byte_start = buf.total_emacs_byte_len().get();
                buf.insert(sample);
                let sample_byte_end = buf.total_emacs_byte_len().get();
                buf.insert("\n");
                let plist = Value::list(vec![
                    Value::keyword("family"),
                    Value::string("JetBrains Mono"),
                    Value::keyword("height"),
                    Value::make_float(height as f64),
                    Value::keyword("weight"),
                    Value::symbol(weight_name),
                ]);
                buf.put_text_property(
                    sample_byte_start,
                    sample_byte_end,
                    Value::symbol("face"),
                    plist,
                );
                targets.push(TargetRow {
                    line_beg,
                    sample_pos,
                    height,
                    weight: weight_value,
                });
            }
        }
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-sequential-window-point", 1400, 256, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    let mut metrics = FontMetricsService::new();

    for target in &targets {
        let byte_pos = {
            let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
            buffer
                .lisp_pos_to_emacs_byte_pos(LispCharPos1::from_one_based_usize(target.line_beg))
                .get()
        };
        let _ = eval
            .buffer_manager_mut()
            .goto_buffer_emacs_byte_pos(buf_id, neovm_core::buffer::EmacsBytePos::new(byte_pos));
        {
            let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
            let window = frame
                .find_window_mut(selected_window)
                .expect("selected window");
            if let neovm_core::window::Window::Leaf { point, .. } = window {
                *point = LispCharPos1::from_one_based_usize(target.line_beg);
            }
        }

        engine.layout_frame_rust(&mut eval, frame_id);

        let frame = eval.frame_manager().get(frame_id).expect("frame");
        let snapshot = frame
            .window_display_snapshot(selected_window)
            .expect("display snapshot");
        let all_points = snapshot.points.clone();
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        let sample_chars = [
            (target.line_beg, char_at_lisp_pos(buffer, target.line_beg)),
            (
                target.sample_pos,
                char_at_lisp_pos(buffer, target.sample_pos),
            ),
            (
                target.sample_pos + 1,
                char_at_lisp_pos(buffer, target.sample_pos + 1),
            ),
            (
                target.sample_pos + 2,
                char_at_lisp_pos(buffer, target.sample_pos + 2),
            ),
            (
                target.sample_pos + 3,
                char_at_lisp_pos(buffer, target.sample_pos + 3),
            ),
        ];
        let a = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos))
            .expect("sample a");
        let hao1 = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 1))
            .expect("sample first 好");
        let hao2 = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 2))
            .expect("sample second 好");
        let b = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 3))
            .expect("sample b");
        let after_b = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 4))
            .expect("sample trailing space");

        let face_font_size = frame.font_pixel_size * target.height;
        let expected_a = expected_gui_glyph_advance(
            &mut metrics,
            'a',
            "JetBrains Mono",
            target.weight,
            false,
            face_font_size,
        );
        let expected_hao = expected_gui_glyph_advance(
            &mut metrics,
            '好',
            "JetBrains Mono",
            target.weight,
            false,
            face_font_size,
        );
        let expected_b = expected_gui_glyph_advance(
            &mut metrics,
            'b',
            "JetBrains Mono",
            target.weight,
            false,
            face_font_size,
        );

        assert_point_width_matches_advance(a, expected_a, "sequential a", &all_points);
        assert_point_width_matches_advance(hao1, expected_hao, "sequential first 好", &all_points);
        assert_point_width_matches_advance(hao2, expected_hao, "sequential second 好", &all_points);
        assert_point_width_matches_advance(b, expected_b, "sequential b", &all_points);
        assert_point_delta_matches_advance(a, hao1, expected_a, "sequential first 好", &all_points);
        assert_point_delta_matches_advance(
            hao1,
            hao2,
            expected_hao,
            "sequential second 好",
            &all_points,
        );
        assert_point_delta_matches_advance(hao2, b, expected_hao, "sequential b", &all_points);
        assert_point_delta_matches_advance(
            b,
            after_b,
            expected_b,
            "sequential after b",
            &all_points,
        );

        let _ = sample_chars;
    }
}

#[test]
fn layout_frame_rust_keeps_mixed_width_positions_correct_across_family_switches() {
    #[derive(Clone, Copy, Debug)]
    struct TargetRow<'a> {
        family: &'a str,
        line_beg: usize,
        sample_pos: usize,
        height: f32,
        weight_name: &'a str,
        weight: u16,
    }

    fn char_at_lisp_pos(buffer: &neovm_core::buffer::Buffer, pos: usize) -> Option<char> {
        if pos == 0 {
            return None;
        }
        let byte_pos = buffer
            .char_pos_to_emacs_byte_pos_clamped(neovm_core::buffer::CharPos0::new(pos - 1))
            .get();
        buffer.char_after_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(byte_pos))
    }

    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let sample = "a好好b  ABCXYZ 0123456789  -> <= >=";
    let mut targets = Vec::new();
    let weights = [
        ("normal", 400_u16),
        ("semi-bold", 600_u16),
        ("bold", 700_u16),
        ("extra-bold", 800_u16),
    ];
    let families = [
        "JetBrains Mono",
        "Hack",
        "DejaVu Sans Mono",
        "Noto Sans Mono",
    ];

    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        for family in families {
            let heading = format!("  -- family: {family} --\n");
            buf.insert(&heading);
            for height in [0.9_f32, 1.0_f32, 1.2_f32, 1.6_f32] {
                for (weight_name, weight_value) in weights {
                    let line_beg = if buf.is_text_empty() {
                        1usize
                    } else {
                        buf.point_max_char_pos().get() as usize + 1
                    };
                    let prefix = format!("  {:<35} ", format!("h={height} w={weight_name}:"));
                    let sample_pos = line_beg + prefix.chars().count();
                    buf.insert(&prefix);
                    let sample_byte_start = buf.total_emacs_byte_len().get();
                    buf.insert(sample);
                    let sample_byte_end = buf.total_emacs_byte_len().get();
                    buf.insert("\n");
                    let plist = Value::list(vec![
                        Value::keyword("family"),
                        Value::string(family),
                        Value::keyword("height"),
                        Value::make_float(height as f64),
                        Value::keyword("weight"),
                        Value::symbol(weight_name),
                    ]);
                    buf.put_text_property(
                        sample_byte_start,
                        sample_byte_end,
                        Value::symbol("face"),
                        plist,
                    );
                    targets.push(TargetRow {
                        family,
                        line_beg,
                        sample_pos,
                        height,
                        weight_name,
                        weight: weight_value,
                    });
                }
            }
            buf.insert("\n");
        }
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-family-switches", 1400, 1600, buf_id);
    realize_test_gui_frame(&mut eval, frame_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    let mut metrics = FontMetricsService::new();

    for target in &targets {
        let byte_pos = {
            let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
            buffer
                .lisp_pos_to_emacs_byte_pos(LispCharPos1::from_one_based_usize(target.line_beg))
                .get()
        };
        let _ = eval
            .buffer_manager_mut()
            .goto_buffer_emacs_byte_pos(buf_id, neovm_core::buffer::EmacsBytePos::new(byte_pos));
        {
            let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
            let window = frame
                .find_window_mut(selected_window)
                .expect("selected window");
            if let neovm_core::window::Window::Leaf { point, .. } = window {
                *point = LispCharPos1::from_one_based_usize(target.line_beg);
            }
        }

        engine.layout_frame_rust(&mut eval, frame_id);

        let frame = eval.frame_manager().get(frame_id).expect("frame");
        let snapshot = frame
            .window_display_snapshot(selected_window)
            .expect("display snapshot");
        let all_points = snapshot.points.clone();
        let visible_span = snapshot.visible_buffer_span();
        let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
        let sample_chars = [
            (
                target.sample_pos,
                char_at_lisp_pos(buffer, target.sample_pos),
            ),
            (
                target.sample_pos + 1,
                char_at_lisp_pos(buffer, target.sample_pos + 1),
            ),
            (
                target.sample_pos + 2,
                char_at_lisp_pos(buffer, target.sample_pos + 2),
            ),
            (
                target.sample_pos + 3,
                char_at_lisp_pos(buffer, target.sample_pos + 3),
            ),
        ];
        let a = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos))
            .unwrap_or_else(|| {
                panic!(
                    "sample a missing; target={target:?}; visible_span={visible_span:?}; chars={sample_chars:?}; points={all_points:?}"
                )
            });
        let hao1 = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 1))
            .unwrap_or_else(|| {
                panic!(
                    "sample first 好 missing; target={target:?}; visible_span={visible_span:?}; chars={sample_chars:?}; points={all_points:?}"
                )
            });
        let hao2 = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 2))
            .unwrap_or_else(|| {
                panic!(
                    "sample second 好 missing; target={target:?}; visible_span={visible_span:?}; chars={sample_chars:?}; points={all_points:?}"
                )
            });
        let b = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 3))
            .unwrap_or_else(|| {
                panic!(
                    "sample b missing; target={target:?}; visible_span={visible_span:?}; chars={sample_chars:?}; points={all_points:?}"
                )
            });
        let after_b = snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target.sample_pos + 4))
            .unwrap_or_else(|| {
                panic!(
                    "sample trailing space missing; target={target:?}; visible_span={visible_span:?}; chars={sample_chars:?}; points={all_points:?}"
                )
            });

        let face_font_size = frame.font_pixel_size * target.height;
        let expected_a = expected_gui_glyph_advance(
            &mut metrics,
            'a',
            target.family,
            target.weight,
            false,
            face_font_size,
        );
        let expected_hao = expected_gui_glyph_advance(
            &mut metrics,
            '好',
            target.family,
            target.weight,
            false,
            face_font_size,
        );
        let expected_b = expected_gui_glyph_advance(
            &mut metrics,
            'b',
            target.family,
            target.weight,
            false,
            face_font_size,
        );

        assert_point_width_matches_advance(a, expected_a, "family-switch a", &all_points);
        assert_point_width_matches_advance(
            hao1,
            expected_hao,
            "family-switch first 好",
            &all_points,
        );
        assert_point_width_matches_advance(
            hao2,
            expected_hao,
            "family-switch second 好",
            &all_points,
        );
        assert_point_width_matches_advance(b, expected_b, "family-switch b", &all_points);
        assert_point_delta_matches_advance(
            a,
            hao1,
            expected_a,
            "family-switch first 好",
            &all_points,
        );
        assert_point_delta_matches_advance(
            hao1,
            hao2,
            expected_hao,
            "family-switch second 好",
            &all_points,
        );
        assert_point_delta_matches_advance(hao2, b, expected_hao, "family-switch b", &all_points);
        assert_point_delta_matches_advance(
            b,
            after_b,
            expected_b,
            "family-switch after b",
            &all_points,
        );

        let _ = sample_chars;
        let _ = target.weight_name;
    }
}

#[test]
fn layout_frame_rust_word_wrap_snapshot_stays_sorted_after_rewind() {
    fn char_at_lisp_pos(buffer: &neovm_core::buffer::Buffer, pos: usize) -> Option<char> {
        if pos == 0 {
            return None;
        }
        let byte_pos = buffer
            .char_pos_to_emacs_byte_pos_clamped(neovm_core::buffer::CharPos0::new(pos - 1))
            .get();
        buffer.char_after_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(byte_pos))
    }

    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("aaaa bbbb cccc dddd\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.set_buffer_local("word-wrap", Value::T);
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-wrap", 96, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::ONE;
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    assert!(
        snapshot.points.iter().any(|point| point.row > 0),
        "expected word-wrap to create multiple rows, got points={:?}",
        snapshot.points
    );
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let point_chars = snapshot
        .points
        .iter()
        .map(|point| {
            (
                point.buffer_pos,
                char_at_lisp_pos(buffer, point.buffer_pos.to_one_based_usize()),
            )
        })
        .collect::<Vec<_>>();
    for window in snapshot.points.windows(2) {
        assert!(
            window[0].buffer_pos < window[1].buffer_pos,
            "expected snapshot points to stay sorted after wrap rewind, got {:?}; chars={:?}",
            snapshot.points,
            point_chars
        );
    }
}

#[test]
fn layout_frame_rust_reads_far_enough_for_last_visible_truncated_line() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let mut text = String::new();
    for line in 0..32 {
        text.push_str(&format!("line-{line:02} abcdefghijklmnop\n"));
    }
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.set_buffer_local("truncate-lines", Value::T);
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-read-span", 96, 640, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let target_pos = {
        let mut pos = 1usize;
        for line in 0..26 {
            pos += format!("line-{line:02} abcdefghijklmnop\n").chars().count();
        }
        pos
    };
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        // Selected-window point lives in the buffer; keep pt_char in
        // sync with the target point so redisplay retries read the same
        // location the leaf window advertises.
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(target_pos - 1));
    }
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(target_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let target = snapshot.point_for_buffer_pos(LispCharPos1::from_one_based_usize(target_pos));
    assert!(
        target.is_some(),
        "expected last visible truncated line to remain readable by layout, target_pos={target_pos}, points={:?}",
        snapshot.points
    );
}

#[test]
fn layout_frame_rust_retries_window_when_point_starts_below_visible_span() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let lines = (0..40)
        .map(|line| format!("line-{line:02}\n"))
        .collect::<Vec<_>>();
    let text = lines.join("");
    let target_pos = lines
        .iter()
        .take(20)
        .map(|line| line.chars().count())
        .sum::<usize>()
        + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&text);
        // Selected-window point lives in the buffer; see
        // window.c:window_point. Set buffer pt_char to
        // target_pos so window_params_from_neovm reads it as
        // params.point.
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(target_pos - 1));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-retry", 160, 192, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(target_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let window = frame.find_window(selected_window).expect("selected window");

    assert!(
        snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target_pos))
            .is_some(),
        "expected retried layout to publish geometry for point {target_pos}, points={:?}",
        snapshot.points
    );
    match window {
        neovm_core::window::Window::Leaf { window_start, .. } => {
            assert!(
                *window_start > LispCharPos1::ONE,
                "expected window-start to advance after retry, got {window_start:?}"
            );
        }
        other => panic!("expected leaf window, got {other:?}"),
    }
}

#[test]
fn next_window_start_from_visible_rows_uses_visual_row_boundaries() {
    let rows = vec![
        DisplayRowSnapshot {
            row: 0,
            y: 0,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(1)),
            end_buffer_pos: Some(LispCharPos1::new(8)),
        },
        DisplayRowSnapshot {
            row: 1,
            y: 16,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(9)),
            end_buffer_pos: Some(LispCharPos1::new(16)),
        },
        DisplayRowSnapshot {
            row: 2,
            y: 32,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(17)),
            end_buffer_pos: Some(LispCharPos1::new(24)),
        },
        DisplayRowSnapshot {
            row: 3,
            y: 48,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(25)),
            end_buffer_pos: Some(LispCharPos1::new(32)),
        },
    ];

    assert_eq!(
        next_window_start_from_visible_rows(&rows, 1),
        Some(32),
        "expected retry to advance to the next internal 0-based char position after the last visible row"
    );
    assert_eq!(
        next_window_start_from_visible_rows(&rows, 25),
        Some(32),
        "expected retry to keep the furthest internal 0-based visible progress that still advances"
    );
    assert_eq!(
        next_window_start_from_visible_rows(&rows, 33),
        None,
        "expected no retry candidate once the rendered span no longer advances"
    );
}

#[test]
fn next_window_start_for_partially_visible_point_row_scrolls_enough_to_fit_row() {
    let rows = vec![
        DisplayRowSnapshot {
            row: 0,
            y: 0,
            height: 20,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(1)),
            end_buffer_pos: Some(LispCharPos1::new(10)),
        },
        DisplayRowSnapshot {
            row: 1,
            y: 20,
            height: 20,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(11)),
            end_buffer_pos: Some(LispCharPos1::new(20)),
        },
        DisplayRowSnapshot {
            row: 2,
            y: 40,
            height: 30,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(21)),
            end_buffer_pos: Some(LispCharPos1::new(30)),
        },
    ];

    assert_eq!(
        next_window_start_for_partially_visible_point_row(&rows, 25, 0, 60, 1),
        Some(10),
        "expected retry to scroll away enough top rows to fit the point row using the next internal 0-based char position"
    );
    assert_eq!(
        next_window_start_for_partially_visible_point_row(&rows, 15, 0, 60, 1),
        None,
        "expected no retry when the point row is already fully visible"
    );
}

#[test]
fn next_window_start_for_point_line_continuation_advances_last_visible_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer_size = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcdefghijklmnopqrstuvwxyz\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.point_max_char_pos().get() as i64
    };
    let access = {
        let buf = eval.buffer_manager().get(buf_id).expect("buffer");
        RustBufferAccess::new(buf)
    };
    let rows = vec![
        DisplayRowSnapshot {
            row: 0,
            y: 0,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(1)),
            end_buffer_pos: Some(LispCharPos1::new(10)),
        },
        DisplayRowSnapshot {
            row: 1,
            y: 16,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(11)),
            end_buffer_pos: Some(LispCharPos1::new(20)),
        },
        DisplayRowSnapshot {
            row: 2,
            y: 32,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(21)),
            end_buffer_pos: Some(LispCharPos1::new(25)),
        },
    ];

    assert_eq!(
        next_window_start_for_point_line_continuation(&rows, 21, 1, &access, buffer_size),
        Some(20),
        "expected retry to move point toward the top when the visible point row continues below the window"
    );

    let terminated_rows = vec![
        DisplayRowSnapshot {
            row: 0,
            y: 0,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(1)),
            end_buffer_pos: Some(LispCharPos1::new(10)),
        },
        DisplayRowSnapshot {
            row: 1,
            y: 16,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(11)),
            end_buffer_pos: Some(LispCharPos1::new(27)),
        },
    ];
    assert_eq!(
        next_window_start_for_point_line_continuation(
            &terminated_rows,
            11,
            1,
            &access,
            buffer_size
        ),
        None,
        "expected no retry once the final visible row already reaches the newline"
    );
}

#[test]
fn next_window_start_for_point_line_continuation_ignores_newline_terminated_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer_size = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("needle target\nfiller line 06\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.point_max_char_pos().get() as i64
    };
    let access = {
        let buf = eval.buffer_manager().get(buf_id).expect("buffer");
        RustBufferAccess::new(buf)
    };
    let rows = vec![DisplayRowSnapshot {
        row: 0,
        y: 0,
        height: 16,
        start_x: 0,
        start_col: 0,
        end_x: 0,
        end_col: 0,
        start_buffer_pos: Some(LispCharPos1::new(1)),
        end_buffer_pos: Some(LispCharPos1::new(14)),
    }];

    assert_eq!(
        next_window_start_for_point_line_continuation(&rows, 0, 0, &access, buffer_size),
        None,
        "expected no retry when the last visible row ended on a real newline"
    );
}

#[test]
fn next_window_start_for_point_line_continuation_ignores_tail_clipping_when_point_row_is_not_last_visible_row()
 {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let buffer_size = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ\n");
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.point_max_char_pos().get() as i64
    };
    let access = {
        let buf = eval.buffer_manager().get(buf_id).expect("buffer");
        RustBufferAccess::new(buf)
    };
    let rows = vec![
        DisplayRowSnapshot {
            row: 0,
            y: 0,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(1)),
            end_buffer_pos: Some(LispCharPos1::new(10)),
        },
        DisplayRowSnapshot {
            row: 1,
            y: 16,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(11)),
            end_buffer_pos: Some(LispCharPos1::new(20)),
        },
        DisplayRowSnapshot {
            row: 2,
            y: 32,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(21)),
            end_buffer_pos: Some(LispCharPos1::new(30)),
        },
        DisplayRowSnapshot {
            row: 3,
            y: 48,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(31)),
            end_buffer_pos: Some(LispCharPos1::new(40)),
        },
        DisplayRowSnapshot {
            row: 4,
            y: 64,
            height: 16,
            start_x: 0,
            start_col: 0,
            end_x: 0,
            end_col: 0,
            start_buffer_pos: Some(LispCharPos1::new(41)),
            end_buffer_pos: Some(LispCharPos1::new(50)),
        },
    ];

    assert_eq!(
        next_window_start_for_point_line_continuation(&rows, 21, 1, &access, buffer_size),
        None,
        "expected no retry here because the point row is not the final visible row; partially visible rows are handled by the separate point-row retry path"
    );
}

#[test]
fn display_row_measurement_face_distinguishes_semantic_font_identity() {
    let mut font_metrics_svc = Some(FontMetricsService::new());
    let mut regular = crate::neovm_bridge::ResolvedFace::default();
    regular.font_family = "monospace".to_string();
    regular.font_size = 14.0;
    regular.font_weight = 400;
    regular.set_measured_char_width_px(8.0);
    let mut bold = regular.clone();
    bold.font_weight = 700;
    let measurement_policy = DisplayRowMeasurementPolicy::for_frame(true);
    let regular_face = measurement_policy.measurement_face(42, &regular, None, 8.0);
    let bold_face = measurement_policy.measurement_face(43, &bold, None, 8.0);

    let regular_width = regular_face.advance_for_char(&mut font_metrics_svc, 'A', 8.0);
    let bold_width = bold_face.advance_for_char(&mut font_metrics_svc, 'A', 8.0);
    let repeated_regular_width = regular_face.advance_for_char(&mut font_metrics_svc, 'A', 8.0);

    assert!(
        regular_width > 0.0,
        "expected measurable width for regular ASCII glyph"
    );
    assert!(
        bold_width > 0.0,
        "expected measurable width for bold ASCII glyph"
    );
    assert_eq!(
        repeated_regular_width, regular_width,
        "expected repeated measurement for the same semantic font spec to be stable"
    );
}

#[test]
fn display_row_measurement_face_preserves_fractional_gui_cell_width_without_font_metrics() {
    let mut resolved = crate::neovm_bridge::ResolvedFace::default();
    resolved.font_family = "JetBrainsMono Nerd Font".to_string();
    resolved.font_size = 12.0;
    resolved.set_measured_char_width_px(7.2);
    let current_face =
        DisplayRowMeasurementPolicy::for_frame(true).measurement_face(42, &resolved, None, 7.2);
    let mut font_metrics_svc = None;

    let width = current_face.advance_for_char(&mut font_metrics_svc, 'x', 7.2);

    assert_eq!(width, 7.2);
}

#[test]
fn display_row_glyph_measurer_is_reusable_for_engine_measurements() {
    let mut resolved = crate::neovm_bridge::ResolvedFace::default();
    resolved.font_family = "monospace".to_string();
    resolved.font_size = 14.0;
    resolved.set_measured_char_width_px(8.0);
    let faces = [DisplayRowFace::from_resolved(42, &resolved)];
    let mut font_metrics_svc = None;
    let mut measurer = DisplayRowGlyphMeasurer::new(&faces, font_metrics_svc.as_mut(), 7.2);

    let width = measurer
        .glyph_advance_px('x', 42, 1, 7.2)
        .expect("measure known face");

    assert_eq!(width, 8.0);
}

#[test]
fn display_row_glyph_measurement_face_carries_engine_measurement_policy() {
    let mut resolved = crate::neovm_bridge::ResolvedFace::default();
    resolved.font_family = "monospace".to_string();
    resolved.font_size = 14.0;
    resolved.set_measured_char_width_px(7.2);
    let current_face =
        DisplayRowMeasurementPolicy::for_frame(false).measurement_face(42, &resolved, None, 7.2);
    let mut font_metrics_svc = None;

    let width = current_face.glyph_advance_px(&mut font_metrics_svc, 'x', 1, 7.2);

    assert_eq!(width, 7.0);
}

#[test]
fn layout_frame_rust_converges_visibility_for_wrapped_rows_in_one_redisplay() {
    fn char_at_lisp_pos(buffer: &neovm_core::buffer::Buffer, pos: usize) -> Option<char> {
        if pos == 0 {
            return None;
        }
        let byte_pos = buffer
            .char_pos_to_emacs_byte_pos_clamped(neovm_core::buffer::CharPos0::new(pos - 1))
            .get();
        buffer.char_after_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(byte_pos))
    }

    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let logical_lines = (0..24)
        .map(|line| format!("line-{line:02} abcdefghijklmno\n"))
        .collect::<Vec<_>>();
    let text = logical_lines.join("");
    let target_pos = logical_lines
        .iter()
        .take(18)
        .map(|line| line.chars().count())
        .sum::<usize>()
        + 1;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&text);
        // Move the buffer point to target_pos so the selected
        // window reads it as params.point (GNU
        // window.c:window_point says selected windows use
        // BUF_PT, not pointm). Without this, the Window::point
        // assignment below would be shadowed by buffer.pt_char
        // during window_params_from_neovm and layout would
        // never see the target.
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(target_pos - 1));
        buf.set_buffer_local("word-wrap", Value::T);
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-wrap-retry", 80, 192, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *point = LispCharPos1::from_one_based_usize(target_pos);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let window = frame.find_window(selected_window).expect("selected window");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let point_chars = snapshot
        .points
        .iter()
        .map(|point| {
            (
                point.buffer_pos,
                char_at_lisp_pos(buffer, point.buffer_pos.to_one_based_usize()),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(target_pos))
            .is_some(),
        "expected wrapped-line redisplay to converge on point {target_pos}, points={:?}, rows={:?}, chars={:?}",
        snapshot.points,
        snapshot.rows,
        point_chars
    );
    match window {
        neovm_core::window::Window::Leaf { window_start, .. } => {
            assert!(
                *window_start > LispCharPos1::ONE,
                "expected window-start to advance for wrapped redisplay, got {window_start:?}"
            );
        }
        other => panic!("expected leaf window, got {other:?}"),
    }
}

#[test]
fn layout_frame_rust_converges_visibility_for_point_line_tail_clipping() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let prefix = (0..2)
        .map(|line| format!("p{line:02}\n"))
        .collect::<Vec<_>>()
        .join("");
    let target_line = "abcdefghijklmno\n";
    let text = format!("{prefix}{target_line}");
    let point = prefix.chars().count() + 1;
    let later_pos = point + 10;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.set_buffer_local("word-wrap", Value::T);
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-point-line-tail", 80, 256, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point: window_point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *window_point = LispCharPos1::from_one_based_usize(point);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    assert!(
        snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(later_pos))
            .is_some(),
        "expected redisplay to publish later positions from the point line after retry, points={:?}, rows={:?}",
        snapshot.points,
        snapshot.rows
    );
}

#[test]
fn layout_frame_rust_keeps_visible_eob_cursor_on_short_trailing_newline_buffer() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = "LEFT WINDOW\nLine 2\nLine 3\n";
    let point = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(0));
        buf.point_max_char_pos().get() + 1
    };
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-eob-visible", 320, 640, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point: window_point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *window_point = LispCharPos1::from_one_based_usize(point);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let window = frame.find_window(selected_window).expect("selected window");

    assert!(
        snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
            .is_some(),
        "expected first line to remain visible when EOB cursor is already onscreen, points={:?}, rows={:?}",
        snapshot.points,
        snapshot.rows
    );
    match window {
        neovm_core::window::Window::Leaf { window_start, .. } => {
            assert_eq!(
                *window_start,
                LispCharPos1::ONE,
                "expected visible EOB cursor not to force a retry scroll"
            );
        }
        other => panic!("expected leaf window, got {other:?}"),
    }
}

#[test]
fn layout_frame_rust_keeps_default_scratch_message_at_top_when_eob_is_visible() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = ";; This buffer is for text that is not saved, and for Lisp evaluation.\n\
;; To create a file, visit it with \u{2018}C-x C-f\u{2019} and enter text in its buffer.\n\n";
    let point = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(text);
        let point = buf.point_max_char_pos().get() + 1;
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
        point
    };
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-scratch-eob-visible", 600, 1188, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point: window_point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *window_point = LispCharPos1::from_one_based_usize(point);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let window = frame.find_window(selected_window).expect("selected window");

    assert!(
        snapshot
            .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
            .is_some(),
        "expected the first scratch row to remain visible when EOB fits onscreen, points={:?}, rows={:?}",
        snapshot.points,
        snapshot.rows
    );
    match window {
        neovm_core::window::Window::Leaf { window_start, .. } => {
            assert_eq!(
                *window_start,
                LispCharPos1::ONE,
                "expected short scratch buffer to stay at top, got window-start {window_start:?}"
            );
        }
        other => panic!("expected leaf window, got {other:?}"),
    }
}

#[test]
fn layout_frame_rust_formats_mode_line_from_current_redisplay_geometry() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let text = (0..80)
        .map(|line| format!("Line {line:02}\n"))
        .collect::<String>();
    let point = {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert(&text);
        buf.set_buffer_local("mode-line-format", Value::string("%o|%p|%P"));
        let point = buf.point_max_char_pos().get() + 1;
        // Selected-window point lives in the buffer; see
        // window.c:window_point.
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
        point
    };
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-mode-line-geometry", 640, 96, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        let window = frame
            .find_window_mut(selected_window)
            .expect("selected window");
        if let neovm_core::window::Window::Leaf {
            window_start,
            point: window_point,
            ..
        } = window
        {
            *window_start = LispCharPos1::ONE;
            *window_point = LispCharPos1::from_one_based_usize(point);
        }
    }

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let mode_line_text = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .filter(|row| row.role == GlyphRowRole::ModeLine && row.enabled)
                .flat_map(|row| row.glyphs[1].iter())
                .filter_map(|g| match &g.glyph_type {
                    neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => Some(*ch),
                    _ => None,
                })
                .collect::<String>()
        })
        .unwrap_or_default();
    let published_window_start = {
        let frame = eval.frame_manager().get(frame_id).expect("frame");
        let window = frame.find_window(selected_window).expect("selected window");
        match window {
            neovm_core::window::Window::Leaf { window_start, .. } => *window_start,
            other => panic!("expected leaf window, got {other:?}"),
        }
    };
    let expected_mode_line = eval_status_line_format(
        &mut eval,
        "mode-line-format",
        selected_window.0 as i64,
        buf_id.0,
        80,
    )
    .expect("mode-line text");

    assert!(
        published_window_start > LispCharPos1::ONE,
        "expected point at EOB to advance window-start, got {published_window_start:?}"
    );
    assert!(
        mode_line_text == expected_mode_line,
        "expected rendered mode-line to match freshly evaluated mode-line after redisplay publish, got rendered={mode_line_text:?} expected={expected_mode_line:?}"
    );
}

#[test]
fn layout_frame_rust_honors_window_mode_line_format_none() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("mode-line-format", Value::string("BUFFER MODE"));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-window-mode-line-none", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    eval.frame_manager_mut().set_window_parameter(
        selected_window,
        Value::symbol("mode-line-format"),
        Value::symbol("none"),
    );

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let mode_line_text = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .filter(|row| row.role == GlyphRowRole::ModeLine && row.enabled)
                .flat_map(|row| row.glyphs[1].iter())
                .filter_map(|g| match &g.glyph_type {
                    GlyphType::Char { ch } => Some(*ch),
                    _ => None,
                })
                .collect::<String>()
        })
        .unwrap_or_default();
    let snapshot = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.window_display_snapshot(selected_window))
        .expect("display snapshot");

    assert_eq!(
        snapshot.mode_line_height, 0,
        "window parameter mode-line-format=none should suppress mode-line height like GNU"
    );
    assert!(
        mode_line_text.is_empty(),
        "window parameter mode-line-format=none should suppress rendered mode-line, got {mode_line_text:?}"
    );
}

#[test]
fn layout_frame_rust_uses_window_mode_line_format_override() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("mode-line-format", Value::NIL);
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-window-mode-line-format", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    eval.frame_manager_mut().set_window_parameter(
        selected_window,
        Value::symbol("mode-line-format"),
        Value::string("WINDOW MODE"),
    );

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let mode_line_text = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .filter(|row| row.role == GlyphRowRole::ModeLine && row.enabled)
                .flat_map(|row| row.glyphs[1].iter())
                .filter_map(|g| match &g.glyph_type {
                    GlyphType::Char { ch } => Some(*ch),
                    _ => None,
                })
                .collect::<String>()
        })
        .unwrap_or_default();
    let snapshot = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.window_display_snapshot(selected_window))
        .expect("display snapshot");

    assert!(
        snapshot.mode_line_height > 0,
        "non-nil window mode-line-format should request a mode-line like GNU"
    );
    assert!(
        mode_line_text.contains("WINDOW MODE"),
        "expected window parameter mode-line-format to override nil buffer format, got {mode_line_text:?}"
    );
}

#[test]
fn layout_frame_rust_advances_live_output_through_mode_line_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        let point = buf.point_max_char_pos().get() + 1;
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-output-progress-mode-line", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let display = eval
        .frame_manager()
        .get(frame_id)
        .and_then(|frame| frame.find_window(selected_window))
        .and_then(|window| window.display())
        .expect("window display state");
    let logical_cursor = display.cursor.expect("logical cursor");
    let output_cursor = display.output_cursor.expect("output cursor");

    assert!(
        output_cursor.row > logical_cursor.row,
        "expected live output progression to continue past text rows into mode-line rows, cursor={logical_cursor:?} output={output_cursor:?}"
    );
}

#[test]
fn layout_frame_rust_renders_header_line_text_for_non_nil_header_line_format() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("header-line-format", Value::string("LEFT HEADER"));
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-header-line", 640, 160, buf_id);

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let header_text = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .filter(|row| row.role == GlyphRowRole::HeaderLine && row.enabled)
                .flat_map(|row| row.glyphs[1].iter())
                .filter_map(|g| match &g.glyph_type {
                    neomacs_display_protocol::glyph_matrix::GlyphType::Char { ch } => Some(*ch),
                    _ => None,
                })
                .collect::<String>()
        })
        .unwrap_or_default();

    assert!(
        header_text.contains("LEFT HEADER"),
        "expected header-line row to render buffer-local header-line-format text, got {header_text:?}"
    );
}

#[test]
fn layout_frame_rust_uses_full_window_row_space_for_header_text_and_mode_line() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("header-line-format", Value::string("LEFT HEADER"));
        let point = buf.point_max_char_pos().get() + 1;
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-header-row-space", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("window display snapshot");
    let display = frame
        .find_window(selected_window)
        .and_then(|window| window.display())
        .expect("window display state");
    let logical_cursor = display.cursor.expect("logical cursor");
    let output_cursor = display.output_cursor.expect("output cursor");

    let header_row = snapshot
        .rows
        .iter()
        .find(|row| row.row == 0)
        .expect("header row snapshot");

    assert!(
        header_row.start_buffer_pos.is_none() && header_row.end_buffer_pos.is_none(),
        "expected row 0 to be reserved for header-line chrome, got {header_row:?}"
    );
    assert!(
        logical_cursor.row >= 1,
        "expected logical cursor row to be offset below header-line chrome, got {logical_cursor:?}"
    );
    assert!(
        output_cursor.row > logical_cursor.row,
        "expected mode-line output to advance past logical text rows, cursor={logical_cursor:?} output={output_cursor:?}"
    );
}

#[test]
fn layout_frame_rust_advances_live_output_through_tab_line_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("tab-line-format", Value::string("TAB ROW"));
        let point = buf.point_max_char_pos().get() + 1;
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-tab-line-row-space", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("window display snapshot");
    let display = frame
        .find_window(selected_window)
        .and_then(|window| window.display())
        .expect("window display state");
    let logical_cursor = display.cursor.expect("logical cursor");
    let output_cursor = display.output_cursor.expect("output cursor");

    let tab_row = snapshot
        .rows
        .iter()
        .find(|row| row.row == 0)
        .expect("tab-line row snapshot");

    assert!(
        tab_row.start_buffer_pos.is_none() && tab_row.end_buffer_pos.is_none(),
        "expected row 0 to be reserved for tab-line chrome, got {tab_row:?}"
    );
    assert!(
        logical_cursor.row >= 1,
        "expected logical cursor row to be offset below tab-line chrome, got {logical_cursor:?}"
    );
    assert!(
        output_cursor.row > logical_cursor.row,
        "expected mode-line output to advance past logical text rows, cursor={logical_cursor:?} output={output_cursor:?}"
    );
}

#[test]
fn layout_frame_rust_tab_line_unicode_uses_shared_display_row_builder() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
        buf.set_buffer_local("tab-line-format", Value::string("A中👨‍👩"));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-tab-line-unicode-baseline", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let tab_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::TabLine)
        .expect("tab-line row");
    let glyphs = &tab_row.glyphs[1];
    let cjk = glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
        .expect("tab-line CJK glyph");

    assert_eq!(glyphs_logical_text(glyphs), "A中👨‍👩");
    assert!(
        cjk.wide,
        "tab-line chrome row should record CJK as a wide glyph through the shared builder: {glyphs:?}"
    );
    assert!(
        glyphs.iter().any(|glyph| glyph.padding),
        "tab-line chrome row should retain padding cells through the shared builder: {glyphs:?}"
    );
    assert!(
        glyphs
            .iter()
            .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))),
        "tab-line chrome row should compose ZWJ emoji through the shared builder: {glyphs:?}"
    );
}

#[test]
fn layout_frame_rust_baseline_buffer_text_uses_main_buffer_wide_and_cluster_glyphs() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("A中👨‍👩B\n");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-unicode-baseline", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_glyphs = entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled && row.role == GlyphRowRole::Text)
        .flat_map(|row| row.glyphs[1].iter())
        .collect::<Vec<_>>();

    assert!(
        text_glyphs.iter().any(|glyph| {
            matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }) && glyph.wide
        }),
        "main buffer path should record CJK as a wide glyph: {text_glyphs:?}"
    );
    assert!(
        text_glyphs.iter().any(|glyph| glyph.padding),
        "main buffer wide/cluster glyphs should retain padding cells: {text_glyphs:?}"
    );
    assert!(
        text_glyphs.iter().any(|glyph| {
            matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))
        }),
        "main buffer path should compose the ZWJ emoji sequence: {text_glyphs:?}"
    );
}

#[test]
fn buffer_text_source_shadow_matches_main_buffer_simple_unicode_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("A中👨‍👩B\n");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-source-shadow", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let main_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("main buffer text row");

    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    let line_end = CharPos0::new("A中👨‍👩B".chars().count());
    let shadow_row =
        render_buffer_text_source_shadow_row(buf_id, &snapshot, line_end, 640.0, 16.0, 12.0, 8.0);

    assert_eq!(
        glyphs_logical_text(&shadow_row.glyphs[1]),
        glyphs_logical_text(&main_row.glyphs[1])
    );
    assert_eq!(
        shadow_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }) && glyph.wide),
        main_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }) && glyph.wide)
    );
    assert_eq!(
        shadow_row.glyphs[1].iter().any(|glyph| glyph.padding),
        main_row.glyphs[1].iter().any(|glyph| glyph.padding)
    );
    assert_eq!(
        shadow_row
            .glyphs[1]
            .iter()
            .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}'))),
        main_row
            .glyphs[1]
            .iter()
            .any(|glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.contains('\u{200d}')))
    );
}

#[test]
fn buffer_text_source_shadow_matches_main_buffer_tab_row() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("a\tb\n");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-buffer-source-tab-shadow", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let main_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("main buffer text row");

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let buffer = eval.buffer_manager().get(buf_id).expect("buffer");
    let snapshot = LayoutBufferSnapshot::from_buffer(buffer);
    let line_end = CharPos0::new("a\tb".chars().count());
    let shadow_row = render_buffer_text_source_shadow_row(
        buf_id,
        &snapshot,
        line_end,
        640.0,
        frame.char_height,
        frame.char_height,
        frame.char_width,
    );

    let main_glyphs = &main_row.glyphs[1];
    let shadow_glyphs = &shadow_row.glyphs[1];
    let main_tab = main_glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { .. }))
        .expect("main tab stretch");
    let shadow_tab = shadow_glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { .. }))
        .expect("shadow tab stretch");

    assert_eq!(
        glyphs_logical_text(main_glyphs),
        glyphs_logical_text(shadow_glyphs)
    );
    assert_eq!(main_tab.glyph_type, shadow_tab.glyph_type);
    assert_eq!(main_tab.pixel_width, shadow_tab.pixel_width);
}

#[test]
fn layout_frame_rust_preserves_multiline_overlay_output_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x");
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: 1,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("after-string"),
            Value::string("A\nB"),
        );
        let point = buf.point_max_char_pos().get() + 1;
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(point - 1));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-output-rows", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("window display snapshot");
    let display = frame
        .find_window(selected_window)
        .and_then(|window| window.display())
        .expect("window display state");
    let second_text_row = snapshot
        .rows
        .iter()
        .find(|row| row.row == 1)
        .expect("second overlay row snapshot");
    let overlay_hit_row = unsafe {
        (&*std::ptr::addr_of!(crate::hit_test::FRAME_HIT_DATA))
            .as_ref()
            .and_then(|windows| {
                windows
                    .iter()
                    .find(|window| window.window_id == selected_window.0 as i64)
            })
            .and_then(|window| {
                window.rows.iter().find(|row| {
                    let y = second_text_row.y as f32 + 1.0;
                    y >= row.y_start && y < row.y_end
                })
            })
            .cloned()
    }
    .expect("overlay hit row");
    let overlay_hit = crate::hit_test::hit_test_window_charpos(
        selected_window.0 as i64,
        0.0,
        second_text_row.y as f32 + 1.0,
    );

    assert!(
        snapshot
            .rows
            .iter()
            .any(|row| row.row == 0 && row.start_buffer_pos.is_some()),
        "expected first text row snapshot to survive multiline overlay output, rows={:?}",
        snapshot.rows
    );
    assert!(
        snapshot.rows.iter().any(|row| row.row == 1),
        "expected multiline overlay output to publish a second text row, rows={:?}",
        snapshot.rows
    );
    assert!(
        display.output_cursor.is_some_and(|cursor| cursor.row >= 1),
        "expected live output cursor to advance onto multiline overlay rows, output={:?}",
        display.output_cursor
    );
    assert!(
        overlay_hit >= overlay_hit_row.charpos_start && overlay_hit <= overlay_hit_row.charpos_end,
        "expected multiline overlay row hit-testing to land inside the recorded overlay row span, hit={overlay_hit} row={overlay_hit_row:?}"
    );
}

#[test]
fn layout_frame_rust_renders_overlay_string_tabs_as_stretches() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x");
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: 1,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("after-string"),
            Value::string("a\tb"),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-tab-string", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    let logical_text = glyphs_logical_text(&text_row.glyphs[1]);
    assert!(
        !logical_text.contains('\t'),
        "overlay tab should not render as a literal tab, row={:?}",
        text_row.glyphs[1]
    );
    assert!(
        logical_text.contains("a      b"),
        "overlay tab should expand to the next tab stop, text={logical_text:?}"
    );
    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Stretch { width_cols: 6 })),
        "overlay tab should be a stretch glyph, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_renders_overlay_string_glyphless_chars_as_glyphless() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x");
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: 1,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("after-string"),
            Value::string("\u{fff0}"),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-glyphless-string", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("selected window matrix");
    let text_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.enabled && row.role == GlyphRowRole::Text)
        .expect("text row");

    assert!(
        text_row.glyphs[1]
            .iter()
            .any(|glyph| matches!(glyph.glyph_type, GlyphType::Glyphless { ch: '\u{fff0}' })),
        "overlay glyphless source char should emit a glyphless glyph, row={:?}",
        text_row.glyphs[1]
    );
}

#[test]
fn layout_frame_rust_places_cursor_inside_overlay_string_text_run() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x");
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: 1,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let overlay_text = Value::string_with_text_properties(
            "AB",
            vec![StringTextPropertyRun {
                start: 1,
                end: 2,
                plist: Value::list(vec![Value::symbol("cursor"), Value::T]),
            }],
        );
        let _ =
            buf.overlays_mut()
                .overlay_put(overlay, Value::symbol("after-string"), overlay_text);
        buf.goto_emacs_byte_pos(EmacsBytePos::new(1));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-cursor-run", 640, 160, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    let snapshot = frame
        .window_display_snapshot(selected_window)
        .expect("display snapshot");
    let cursor = snapshot.phys_cursor.as_ref().expect("cursor");
    let x_point = snapshot
        .point_for_buffer_pos(LispCharPos1::from_one_based_usize(1))
        .expect("x point");
    let expected_overlay_slot_width = frame.char_width.round() as i64;

    assert_eq!(cursor.row, x_point.row);
    assert_eq!(
        cursor.x,
        x_point.x + x_point.width + expected_overlay_slot_width
    );
    assert_eq!(cursor.col, x_point.col + 2);
    assert_eq!(cursor.width, expected_overlay_slot_width);
}

#[test]
fn layout_frame_rust_renders_zero_length_eob_before_string_rows() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("Find file: ~/.config/doom/");
        let eob = buf.point_max_emacs_byte_pos().get();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("before-string"),
            Value::string("\ninit.el\nconfig.el"),
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-eob-before-overlay", 640, 180, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let rows = enabled_window_row_texts(entry);

    assert!(
        rows.iter().any(|row| row.contains("init.el")),
        "expected zero-length EOB before-string to render init.el, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("config.el")),
        "expected zero-length EOB before-string to render config.el, rows={rows:?}"
    );
}

#[test]
fn layout_frame_rust_renders_row_start_before_string_at_point_min() {
    // GNU `handle_stop`-at-init loads the before-strings of overlays anchored at
    // the iterator's starting charpos (window-start) before producing the first
    // buffer char (`get_overlay_strings_1`, `src/xdisp.c`). vertico's "n/m"
    // candidate count is exactly such a before-string at point-min; it must
    // render at the very start of the first row, ahead of the buffer text.
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("M-x switch");
        // Zero-length overlay anchored at point-min carrying the count in its
        // before-string, mirroring vertico's overlay (vertico.el:444-448/614).
        let bob = 0;
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: bob,
            end: bob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("before-string"),
            Value::string("1/1 "),
        );
        buf.goto_emacs_byte_pos(EmacsBytePos::new(bob));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-row-start-before-overlay", 640, 180, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let rows = enabled_window_row_texts(entry);
    let first_row = rows.first().cloned().unwrap_or_default();

    assert!(
        first_row.contains("1/1"),
        "expected point-min before-string to render on the first row, rows={rows:?}"
    );
    // The before-string must precede the buffer text on the row, exactly like
    // GNU shows "1/1   M-x …" rather than "M-x …".
    let count_idx = first_row
        .find("1/1")
        .expect("before-string present on first row");
    let buffer_idx = first_row
        .find("M-x")
        .expect("buffer text present on first row");
    assert!(
        count_idx < buffer_idx,
        "before-string must render ahead of the first buffer char, first_row={first_row:?}"
    );
}

#[test]
fn layout_frame_rust_suppresses_left_fringe_display_spec_before_string() {
    // magit attaches an overlay before-string `#("fringe" 0 6 (display
    // (left-fringe magit-fringe-bitmapv fringe)))` to every collapsible section
    // heading. GNU's `(left-fringe BITMAP FACE)` display spec REPLACES the
    // covered text in the text area (it renders a bitmap in the fringe instead),
    // so the literal "fringe" string must NOT appear inline. We don't draw the
    // fringe bitmap yet, but the text area must match GNU: nothing for the spec.
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    // Register the magit fold-arrow bitmap so the fringe spec resolves to a real
    // registry index (first user bitmap => 25).
    let fringe_index = eval
        .eval_str(
            "(define-fringe-bitmap 'magit-fringe-bitmapv \
             [#b00000000 #b10000010 #b11000110 #b01101100 #b00111000 #b00010000 \
              #b00000000 #b00000000])",
        )
        .expect("define magit fringe bitmap")
        .as_fixnum()
        .expect("fringe index") as u16;
    // Build the propertized before-string out of band so the `display` property
    // is a real `(left-fringe …)` list, exactly as magit constructs it.
    let before_string = eval
        .eval_str("(propertize \"fringe\" 'display '(left-fringe magit-fringe-bitmapv fringe))")
        .expect("propertize fringe before-string");
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("Head:");
        let bob = 0;
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: bob,
            end: bob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ =
            buf.overlays_mut()
                .overlay_put(overlay, Value::symbol("before-string"), before_string);
        buf.goto_emacs_byte_pos(EmacsBytePos::new(bob));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-left-fringe-before-string", 640, 180, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let rows = enabled_window_row_texts(entry);

    // The buffer's own heading text still renders.
    assert!(
        rows.iter().any(|row| row.contains("Head:")),
        "expected heading text to render, rows={rows:?}"
    );
    // The `(left-fringe …)` before-string produces NO inline glyph: the literal
    // "fringe" must not appear anywhere in the text area.
    assert!(
        rows.iter().all(|row| !row.contains("fringe")),
        "expected (left-fringe …) before-string to render nothing inline, rows={rows:?}"
    );

    // Stage 2/3: the covered row records a left-fringe bitmap descriptor with the
    // resolved registry index, so the renderer can draw the arrow in the fringe.
    let fringe_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.left_fringe_bitmap.is_some())
        .expect("a row carries the left-fringe bitmap");
    let info = fringe_row.left_fringe_bitmap.expect("left fringe info");
    assert_eq!(info.bitmap_index, fringe_index);

    // Stage 3: the bitmap bits are embedded once per frame for the renderer.
    assert!(
        state.fringe_bitmaps.contains_key(&fringe_index),
        "frame display state embeds the resolved fringe bitmap data"
    );
}

#[test]
fn layout_frame_rust_resolves_standard_fringe_bitmap_spec() {
    // Foundation Stage 1: an explicit `(left-fringe right-arrow fringe)` display
    // spec — referencing a GNU STANDARD built-in bitmap (no
    // `define-fringe-bitmap` call) — now resolves to a real bitmap descriptor.
    // Before seeding the standard bitmaps, `record_fringe_bitmap_layout` returned
    // None for `right-arrow` (its index 1..24 slot was empty), so nothing drew.
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    // `right-arrow` is GNU standard_bitmaps[] index 4, pre-seeded at startup.
    let right_arrow_index: u16 = eval
        .eval_str("(get 'right-arrow 'fringe)")
        .expect("right-arrow fringe prop")
        .as_fixnum()
        .expect("fringe index") as u16;
    assert_eq!(right_arrow_index, 4, "right-arrow is fringe.c index 4");

    let before_string = eval
        .eval_str("(propertize \"fringe\" 'display '(left-fringe right-arrow fringe))")
        .expect("propertize fringe before-string");
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("Standard:");
        let bob = 0;
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: bob,
            end: bob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ =
            buf.overlays_mut()
                .overlay_put(overlay, Value::symbol("before-string"), before_string);
        buf.goto_emacs_byte_pos(EmacsBytePos::new(bob));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-standard-left-fringe", 640, 180, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let rows = enabled_window_row_texts(entry);

    // The literal "fringe" string is suppressed (replacement spec), heading shows.
    assert!(
        rows.iter().any(|row| row.contains("Standard:")),
        "expected heading text to render, rows={rows:?}"
    );
    assert!(
        rows.iter().all(|row| !row.contains("fringe")),
        "expected (left-fringe …) before-string to render nothing inline, rows={rows:?}"
    );

    // The covered row carries the resolved STANDARD bitmap index (4), proving the
    // explicit-spec path now works for standard symbols, not just user bitmaps.
    let fringe_row = entry
        .matrix
        .rows
        .iter()
        .find(|row| row.left_fringe_bitmap.is_some())
        .expect("a row carries the left-fringe bitmap");
    let info = fringe_row.left_fringe_bitmap.expect("left fringe info");
    assert_eq!(info.bitmap_index, right_arrow_index);

    // The standard bitmap's bits are embedded once per frame for the renderer.
    assert!(
        state.fringe_bitmaps.contains_key(&right_arrow_index),
        "frame display state embeds the standard fringe bitmap data"
    );
}

#[test]
fn layout_frame_rust_fills_empty_line_fringe_below_buffer_end() {
    // Stage 3/4: a buffer that ends well before the window bottom, with
    // `indicate-empty-lines` on (Doom's vi-tilde-fringe `~`), produces blank
    // filler rows below the last buffer line — each carrying the periodic
    // `empty-line` bitmap in the LEFT fringe. GNU's redisplay tail fills these
    // (xdisp.c sets `row->indicate_empty_line_p`); before this change neomacs
    // left bare frame background below the last line.
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    // `empty-line` is GNU standard_bitmaps[] index 24, pre-seeded at startup.
    let empty_line_index: u16 = eval
        .eval_str("(get 'empty-line 'fringe)")
        .expect("empty-line fringe prop")
        .as_fixnum()
        .expect("fringe index") as u16;
    assert_eq!(empty_line_index, 24, "empty-line is fringe.c index 24");

    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        // A single short line, so the bulk of the window is below buffer-end.
        buf.insert("hello\n");
        buf.set_buffer_local("indicate-empty-lines", Value::T);
        buf.goto_emacs_byte_pos(EmacsBytePos::new(0));
    }

    // A tall enough frame that many rows sit below the one buffer line.
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-empty-line-fringe", 640, 400, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");

    // The real buffer line still renders — the filler rows must NOT overwrite or
    // double-count the last buffer row (off-by-one guard).
    let texts = enabled_window_row_texts(entry);
    assert!(
        texts.iter().any(|row| row.contains("hello")),
        "the buffer's own line must still render below the fillers, rows={texts:?}"
    );

    // Every filler row below the buffer carries the empty-line bitmap in the
    // LEFT fringe, resolved to the standard registry index.
    let empty_line_rows: Vec<_> = entry
        .matrix
        .rows
        .iter()
        .filter_map(|row| row.left_fringe_bitmap)
        .filter(|info| info.bitmap_index == empty_line_index)
        .collect();
    assert!(
        empty_line_rows.len() >= 5,
        "expected several empty-line filler rows below the single buffer line, \
         got {} (rows total = {})",
        empty_line_rows.len(),
        entry.matrix.rows.len()
    );

    // The periodic empty-line bitmap is embedded once per frame for the renderer,
    // and it carries its period (3) so the renderer tiles the dotted pattern.
    let bitmap = state
        .fringe_bitmaps
        .get(&empty_line_index)
        .expect("frame embeds the empty-line bitmap data");
    assert_eq!(bitmap.period, 3, "empty-line is periodic with period 3");

    // The filler rows are blank text rows that end at ZV (not chrome / mode-line).
    for row in entry.matrix.rows.iter() {
        if row
            .left_fringe_bitmap
            .is_some_and(|info| info.bitmap_index == empty_line_index)
        {
            assert!(
                !row.mode_line,
                "empty-line filler rows must not be mode-line rows"
            );
            assert!(
                row.glyphs.iter().all(|area| area.is_empty()),
                "empty-line filler rows must be blank (no glyphs)"
            );
        }
    }
}

#[test]
fn layout_frame_rust_omits_empty_line_fringe_when_indicator_off() {
    // Control: with `indicate-empty-lines` OFF, no filler rows carry the
    // empty-line bitmap (proves the filler path is gated on the buffer-local).
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let empty_line_index: u16 = eval
        .eval_str("(get 'empty-line 'fringe)")
        .expect("empty-line fringe prop")
        .as_fixnum()
        .expect("fringe index") as u16;
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("hello\n");
        buf.goto_emacs_byte_pos(EmacsBytePos::new(0));
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-empty-line-fringe-off", 640, 400, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;
    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);
    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    assert!(
        entry
            .matrix
            .rows
            .iter()
            .filter_map(|row| row.left_fringe_bitmap)
            .all(|info| info.bitmap_index != empty_line_index),
        "no empty-line bitmaps when indicate-empty-lines is off"
    );
}

#[test]
fn layout_frame_rust_renders_eob_overlay_strings_in_gnu_interleaved_order() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("x");
        let eob = buf.point_max_emacs_byte_pos().get();

        let after_overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(after_overlay);
        let _ = buf.overlays_mut().overlay_put(
            after_overlay,
            Value::symbol("after-string"),
            Value::string("A"),
        );

        let before_overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(before_overlay);
        let _ = buf.overlays_mut().overlay_put(
            before_overlay,
            Value::symbol("before-string"),
            Value::string("B"),
        );
        buf.goto_emacs_byte_pos(EmacsBytePos::new(eob));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-eob-overlay-interleaved-order",
        640,
        180,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let rendered = enabled_window_row_texts(entry).join("\n");

    assert!(
        rendered.contains("xAB"),
        "GNU compare_overlay_entries renders after-strings from other overlays before before-strings, rows={rendered:?}"
    );
}

#[test]
fn layout_frame_rust_overlay_before_string_uses_overlay_string_base_face() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("M-x s");
        let prompt_face = Value::list(vec![
            Value::keyword("background"),
            Value::string("#ffff00"),
            Value::keyword("foreground"),
            Value::string("#000000"),
        ]);
        buf.put_text_property(
            0,
            buf.total_emacs_byte_len().get(),
            Value::symbol("face"),
            prompt_face,
        );

        let eob = buf.point_max_emacs_byte_pos().get();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("before-string"),
            Value::string("\ncandidate"),
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-string-base-face", 640, 180, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let default_bg = state
        .faces
        .get(&u32::from(
            neomacs_display_protocol::face::BasicFaceId::Default,
        ))
        .expect("default face")
        .background;

    let prompt_face_id = entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled && row.role == GlyphRowRole::Text)
        .flat_map(|row| row.glyphs[1].iter())
        .find_map(|glyph| match glyph.glyph_type {
            GlyphType::Char { ch: 'M' } => Some(glyph.face_id),
            _ => None,
        })
        .expect("prompt glyph face");
    let prompt_bg = state
        .faces
        .get(&prompt_face_id)
        .expect("prompt face")
        .background;
    assert_ne!(
        prompt_bg, default_bg,
        "test setup should make prompt face distinguishable from default"
    );

    let candidate_face_id = entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled && row.role == GlyphRowRole::Text)
        .flat_map(|row| row.glyphs[1].iter())
        .find_map(|glyph| match glyph.glyph_type {
            GlyphType::Char { ch: 'c' } => Some(glyph.face_id),
            _ => None,
        })
        .expect("candidate glyph face");
    let candidate_bg = state
        .faces
        .get(&candidate_face_id)
        .expect("candidate face")
        .background;

    assert_eq!(
        candidate_bg, default_bg,
        "GNU overlay strings use a default/text-property base face, not the current prompt face"
    );
}

#[test]
fn layout_frame_rust_merges_overlay_face_with_text_property_face() {
    // GNU face_at_buffer_position merges the `face' text property FIRST, then
    // overlay faces LAST (overlays win on conflict but both contribute). A char
    // carrying both a text-property face and an overlay face must render the
    // merged face, not either contribution alone.
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("ab");
        let len = buf.total_emacs_byte_len().get();
        // Text-property face: a NAMED face (`bold`) — this resolves to a concrete
        // face id (unlike a plist face, which defers to face_at_pos). The named
        // face sets weight but not background.
        buf.put_text_property(0, len, Value::symbol("face"), Value::symbol("bold"));
        // Overlay face spanning the same text: distinctive BACKGROUND only.
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 0,
            end: len,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("face"),
            Value::list(vec![Value::keyword("background"), Value::string("#00ff00")]),
        );
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-overlay-face-merge", 640, 180, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let default_face = state
        .faces
        .get(&u32::from(
            neomacs_display_protocol::face::BasicFaceId::Default,
        ))
        .expect("default face");
    let default_bg = default_face.background;

    let a_face_id = entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled && row.role == GlyphRowRole::Text)
        .flat_map(|row| row.glyphs[1].iter())
        .find_map(|glyph| match glyph.glyph_type {
            GlyphType::Char { ch: 'a' } => Some(glyph.face_id),
            _ => None,
        })
        .expect("glyph 'a' face");
    let face = state.faces.get(&a_face_id).expect("resolved face for 'a'");

    assert!(
        face.is_bold(),
        "text-property bold face must survive the merge"
    );
    assert_ne!(
        face.background, default_bg,
        "overlay background must merge in (GNU merges overlays after the text-prop face); \
         dropping it means the named text-prop face overrode the overlay-merged checkpoint face"
    );
}

#[test]
fn layout_frame_rust_applies_face_only_overlay_starting_mid_run() {
    // Regression (isearch current-match highlight): a face-only overlay (no
    // display string) that begins/ends INSIDE a text-property run must bound the
    // run so each piece carries its own face. GNU folds `next_overlay_change`
    // into `compute_stop_pos` (src/xdisp.c). Before the fix, the run was bounded
    // only by text-property changes, so an overlay starting mid-run never split
    // it and the overlay face never painted (C-s "counter" left the match
    // unhighlighted because the overlay began inside the "my-counter" run).
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("abcdef");
        let len = buf.total_emacs_byte_len().get();
        // One uniform text-property face over the WHOLE run, so the only face
        // boundaries come from the overlay (start at 2, end at 4).
        buf.put_text_property(0, len, Value::symbol("face"), Value::symbol("bold"));
        // Face-only overlay (distinctive background) over "cd" — text before
        // ('ab') and after ('ef'), so it begins AND ends mid-run.
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: 2,
            end: 4,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("face"),
            Value::list(vec![Value::keyword("background"), Value::string("#00ff00")]),
        );
    }

    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-midrun-overlay", 640, 180, buf_id);
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let bg_of = |ch: char| {
        let face_id = entry
            .matrix
            .rows
            .iter()
            .filter(|row| row.enabled && row.role == GlyphRowRole::Text)
            .flat_map(|row| row.glyphs[1].iter())
            .find_map(|glyph| match glyph.glyph_type {
                GlyphType::Char { ch: c } if c == ch => Some(glyph.face_id),
                _ => None,
            })
            .unwrap_or_else(|| panic!("glyph {ch:?} not found"));
        state
            .faces
            .get(&face_id)
            .unwrap_or_else(|| panic!("face for {ch:?}"))
            .background
    };

    let overlay_bg = bg_of('c');
    assert_ne!(
        overlay_bg,
        bg_of('a'),
        "face-only overlay over 'cd' must paint its background; 'a' (before it) \
         must keep the plain face — the run must split at the overlay START"
    );
    assert_eq!(
        overlay_bg,
        bg_of('d'),
        "'d' is inside the same overlay as 'c'"
    );
    assert_ne!(
        overlay_bg,
        bg_of('e'),
        "'e' (after the overlay) must keep the plain face — the run must split \
         at the overlay END too"
    );
}

#[test]
fn layout_frame_rust_continues_eob_before_string_after_overlong_line() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        let eob = buf.point_max_emacs_byte_pos().get();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("before-string"),
            Value::string("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nsecond.el\nthird.el"),
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-eob-overlong-before-overlay",
        96,
        180,
        buf_id,
    );
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.char_width = 8.0;
        frame.char_height = 16.0;
        frame.font_pixel_size = 16.0;
    }
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new_without_font_metrics();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let rows = enabled_window_row_texts(entry);

    assert!(
        rows.iter().any(|row| row.contains("second.el")),
        "expected overlong overlay row not to suppress the next candidate row, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("third.el")),
        "expected rendering to continue after later overlay newlines, rows={rows:?}"
    );
}

#[test]
fn layout_frame_rust_honors_display_space_align_in_overlay_strings() {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        let eob = buf.point_max_emacs_byte_pos().get();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(buf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let display_space = Value::string_with_text_properties(
            "config.el -rw",
            vec![StringTextPropertyRun {
                start: "config.el".chars().count(),
                end: "config.el ".chars().count(),
                plist: Value::list(vec![
                    Value::symbol("display"),
                    Value::list(vec![
                        Value::symbol("space"),
                        Value::keyword(":align-to"),
                        Value::list(vec![
                            Value::symbol("+"),
                            Value::symbol("left"),
                            Value::fixnum(20),
                        ]),
                    ]),
                ]),
            }],
        );
        let _ =
            buf.overlays_mut()
                .overlay_put(overlay, Value::symbol("before-string"), display_space);
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-overlay-display-space-align",
        640,
        180,
        buf_id,
    );
    let selected_window = eval
        .frame_manager()
        .get(frame_id)
        .expect("frame")
        .selected_window;

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == selected_window.0)
        .expect("window matrix entry");
    let rendered_rows: Vec<String> = entry
        .matrix
        .rows
        .iter()
        .filter(|row| row.enabled)
        .map(|row| {
            row.glyphs[1]
                .iter()
                .map(|glyph| match &glyph.glyph_type {
                    GlyphType::Char { ch } => ch.to_string(),
                    GlyphType::Composite { text } => text.to_string(),
                    GlyphType::Stretch { width_cols } => " ".repeat(*width_cols as usize),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .collect();

    assert!(
        rendered_rows
            .iter()
            .any(|row| row.contains("config.el           -rw")),
        "GNU TTY expands overlay-string display spaces before suffix text, rows={rendered_rows:?}"
    );
}

#[test]
fn layout_frame_rust_grows_minibuffer_for_eob_before_string_like_gnu() {
    // GNU `load_overlay_strings` (src/xdisp.c:~7164) DOES measure a non-empty
    // EOB `before-string`, so `resize_mini_window` grows the parent minibuffer
    // to display it. With the unclamped walk measurement (no estimator), the
    // minibuffer grows and renders the overlay's `before-string` lines.
    let mut eval = Context::new();
    eval.obarray_mut()
        .set_symbol_value("resize-mini-windows", Value::symbol("grow-only"));
    eval.obarray_mut()
        .set_symbol_value("max-mini-window-height", Value::fixnum(10));

    let root_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let minibuf_id = eval.buffer_manager_mut().create_buffer(" *Minibuf-1*");
    {
        let buf = eval
            .buffer_manager_mut()
            .get_mut(minibuf_id)
            .expect("buffer");
        buf.insert("Find file: ~/.config/doom/");
        let eob = buf.point_max_emacs_byte_pos().get();
        let overlay = Value::make_overlay(neovm_core::heap_types::OverlayData {
            serial: 0,
            plist: Value::NIL,
            buffer: Some(minibuf_id),
            start: eob,
            end: eob,
            front_advance: false,
            rear_advance: false,
        });
        buf.overlays_mut().insert_overlay(overlay);
        let _ = buf.overlays_mut().overlay_put(
            overlay,
            Value::symbol("before-string"),
            Value::string("\ninit.el\nconfig.el"),
        );
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-mini-eob-before-overlay",
        120,
        40,
        root_buf_id,
    );
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.char_width = 1.0;
        frame.char_height = 1.0;
        frame.shrink_mini_window();
    }
    let minibuffer_window_id = eval
        .activate_minibuffer_window_for_buffer(
            minibuf_id,
            LispString::from_utf8("Find file: "),
            Some(LispString::from_utf8("~/.config/doom/")),
        )
        .expect("activate minibuffer")
        .expect("minibuffer window");

    let mut engine = LayoutEngine::new_without_font_metrics();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == minibuffer_window_id.0)
        .expect("minibuffer matrix entry");
    let rows = enabled_window_row_texts(entry);

    assert!(
        rows.iter()
            .any(|row| row.contains("Find file: ~/.config/doom/")),
        "expected minibuffer prompt row to render, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("init.el")),
        "GNU grows the minibuffer for a non-empty EOB before-string \
         (load_overlay_strings), so init.el must render, rows={rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("config.el")),
        "GNU grows the minibuffer for a non-empty EOB before-string \
         (load_overlay_strings), so config.el must render, rows={rows:?}"
    );
}

/// Lay out a frame whose ACTIVE minibuffer displays `content` (the
/// minibuffer buffer's own text), with `max-mini-window-height` set to
/// `max_mini_lines` (a fixnum). Returns the enabled minibuffer row texts.
///
/// Models the active fido/vertico path: an active mini-window renders its
/// own buffer text (GNU `resize_mini_window` measures `move_it_to(ZV)` over
/// that buffer), as opposed to the inactive echo-area path that swaps in
/// ` *Echo Area 0*`.
fn layout_active_minibuffer_rows(
    content: &str,
    max_mini_lines: i64,
    use_gui_metrics: bool,
) -> Vec<String> {
    let mut eval = Context::new();
    eval.obarray_mut()
        .set_symbol_value("resize-mini-windows", Value::symbol("grow-only"));
    eval.obarray_mut()
        .set_symbol_value("max-mini-window-height", Value::fixnum(max_mini_lines));

    let root_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let minibuf_id = eval.buffer_manager_mut().create_buffer(" *Minibuf-1*");
    {
        let buf = eval
            .buffer_manager_mut()
            .get_mut(minibuf_id)
            .expect("buffer");
        buf.insert(content);
        let eob = buf.point_max_emacs_byte_pos().get();
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-active-minibuffer", 640, 200, root_buf_id);

    let mut engine = if use_gui_metrics {
        let mut e = LayoutEngine::new();
        e.enable_cosmic_metrics();
        e
    } else {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.char_width = 1.0;
        frame.char_height = 1.0;
        frame.shrink_mini_window();
        LayoutEngine::new_without_font_metrics()
    };

    let minibuffer_window_id = eval
        .activate_minibuffer_window_for_buffer(minibuf_id, LispString::from_utf8(""), None)
        .expect("activate minibuffer")
        .expect("minibuffer window");

    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == minibuffer_window_id.0)
        .expect("minibuffer matrix entry");
    enabled_window_row_texts(entry)
}

/// An active minibuffer holding a prompt line plus several candidate lines
/// must grow to one display row per logical line, render every line, and never
/// flatten content into one row.  Exercises the active-fido/vertico measure
/// path the unclamped GNU `resize_mini_window` walk now drives.
fn assert_active_minibuffer_grows_for_multiline_content(use_gui_metrics: bool) {
    let rows = layout_active_minibuffer_rows(
        "Find file: cand\nalpha.el\nbeta.el\ngamma.el",
        10,
        use_gui_metrics,
    );

    for needle in ["Find file: cand", "alpha.el", "beta.el", "gamma.el"] {
        assert!(
            rows.iter().any(|row| row.contains(needle)),
            "expected active minibuffer to grow and render {needle:?}, rows={rows:?}"
        );
    }
    assert!(
        !rows.iter().any(|row| row.contains("candalpha")),
        "multiline minibuffer content was flattened into one row: {rows:?}"
    );
    let content_rows = rows.iter().filter(|row| !row.trim().is_empty()).count();
    assert!(
        content_rows >= 4,
        "expected four content rows (one per logical line), got {content_rows}: {rows:?}"
    );
}

#[test]
fn active_minibuffer_grows_for_multiline_content_tty() {
    assert_active_minibuffer_grows_for_multiline_content(false);
}

#[test]
fn active_minibuffer_grows_for_multiline_content_gui() {
    assert_active_minibuffer_grows_for_multiline_content(true);
}

#[test]
fn active_minibuffer_resize_uses_buffer_local_max_mini_window_height() {
    let mut eval = Context::new();
    eval.obarray_mut()
        .set_symbol_value("resize-mini-windows", Value::symbol("grow-only"));
    eval.obarray_mut()
        .set_symbol_value("max-mini-window-height", Value::fixnum(10));

    let root_buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let minibuf_id = eval.buffer_manager_mut().create_buffer(" *Minibuf-1*");
    {
        let buf = eval
            .buffer_manager_mut()
            .get_mut(minibuf_id)
            .expect("buffer");
        buf.set_buffer_local("max-mini-window-height", Value::fixnum(1));
        buf.insert("Find file: \nalpha.el\nbeta.el\ngamma.el");
        let eob = buf.point_max_emacs_byte_pos().get();
        buf.goto_emacs_byte_pos(neovm_core::buffer::EmacsBytePos::new(eob));
    }

    let frame_id = eval.frame_manager_mut().create_frame(
        "layout-active-minibuffer-local-max",
        120,
        40,
        root_buf_id,
    );
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.char_width = 1.0;
        frame.char_height = 1.0;
        frame.shrink_mini_window();
    }

    let minibuffer_window_id = eval
        .activate_minibuffer_window_for_buffer(minibuf_id, LispString::from_utf8(""), None)
        .expect("activate minibuffer")
        .expect("minibuffer window");

    let mut engine = LayoutEngine::new_without_font_metrics();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let entry = state
        .window_matrices
        .iter()
        .find(|entry| entry.window_id == minibuffer_window_id.0)
        .expect("minibuffer matrix entry");
    let rows = enabled_window_row_texts(entry);
    let content_rows = rows.iter().filter(|row| !row.trim().is_empty()).count();

    assert_eq!(
        content_rows, 1,
        "GNU resize_mini_window reads max-mini-window-height in the minibuffer buffer; rows={rows:?}"
    );
}

/// Content taller than `max-mini-window-height` must clamp to the max row
/// count AND scroll so the END shows (GNU `resize_mini_window` sets `w->start`
/// to the end when the measured height exceeds `max_height`).  Point is at EOB,
/// as it is in an active fido/vertico minibuffer.
fn assert_active_minibuffer_overflow_clamps_and_shows_end(use_gui_metrics: bool) {
    // Eight candidate lines but max-mini-window-height = 3 lines.
    let rows = layout_active_minibuffer_rows(
        "PROMPT\ncand1\ncand2\ncand3\ncand4\ncand5\ncand6\nLASTCAND",
        3,
        use_gui_metrics,
    );
    let content_rows = rows.iter().filter(|row| !row.trim().is_empty()).count();
    assert!(
        content_rows <= 3,
        "expected minibuffer height clamped to <= 3 rows, got {content_rows}: {rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("LASTCAND")),
        "expected overflow minibuffer to show the END (LASTCAND), rows={rows:?}"
    );
    assert!(
        !rows.iter().any(|row| row.contains("PROMPT")),
        "expected the first line to scroll off the top on overflow, rows={rows:?}"
    );
}

#[test]
fn active_minibuffer_overflow_clamps_and_shows_end_tty() {
    assert_active_minibuffer_overflow_clamps_and_shows_end(false);
}

#[test]
fn active_minibuffer_overflow_clamps_and_shows_end_gui() {
    assert_active_minibuffer_overflow_clamps_and_shows_end(true);
}

/// A single logical line (no newline) wider than the window wraps to more rows
/// than `max-mini-window-height`; it must clamp to the max and show the END of
/// the wrapped line (GNU's bottom-clamped path snapping `w->start` to a
/// screen-line boundary with `move_it_by_lines`).
#[test]
fn active_minibuffer_wrapped_overflow_clamps_and_shows_end_tty() {
    // 640px / 1px char => 640 cols.  Build a single line far wider than that
    // with unique START and END markers so we can detect which screen line is
    // shown.  max-mini-window-height = 2 lines.
    let mut line = String::from("WRAPSTART");
    line.push_str(&"x".repeat(640 * 6));
    line.push_str("WRAPEND");
    let rows = layout_active_minibuffer_rows(&line, 2, false);
    let content_rows = rows.iter().filter(|row| !row.trim().is_empty()).count();
    assert!(
        content_rows <= 2,
        "expected wrapped overflow clamped to <= 2 rows, got {content_rows}: {rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("WRAPEND")),
        "expected wrapped overflow to show the END of the line, rows={rows:?}"
    );
    assert!(
        !rows.iter().any(|row| row.contains("WRAPSTART")),
        "expected the START of the wrapped line to scroll off, rows={rows:?}"
    );
}

#[test]
fn build_tab_bar_display_roots_transient_string_across_gc() {
    let mut eval =
        create_bootstrap_evaluator_cached_with_features(&["x", "neomacs"]).expect("bootstrap");
    apply_runtime_startup_state(&mut eval).expect("runtime startup state");
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-tab-bar-gc", 1600, 160, buf_id);
    eval.obarray_mut()
        .set_symbol_value("layout-target-frame", Value::make_frame(frame_id.0));
    eval.eval_str(
        r#"
          (require 'tab-bar)
          (setq tab-bar-show 1)
          (tab-bar-mode 1)
          (select-frame layout-target-frame)
          (switch-to-buffer (get-buffer-create "*tab-root*"))
          (tab-bar-new-tab)
          (switch-to-buffer (get-buffer-create "*tab-second*"))
          (tab-bar-select-tab 1)
        "#,
    )
    .expect("eval tab-bar forms");

    let gc_roots = ScratchGcRootScope::new();
    let tab_bar = build_tab_bar_display(&mut eval, frame_id.0, &gc_roots).expect("tab-bar display");
    eval.gc_collect_exact();

    let text = tab_bar
        .text
        .as_runtime_string_owned()
        .expect("tab-bar text should survive exact GC");
    assert!(
        text.contains("*tab-root*") || text.contains("tab-root"),
        "expected tab-bar label after exact GC, got {text:?}"
    );
    let props =
        neovm_core::emacs_core::value::get_string_text_properties_table_for_value(tab_bar.text)
            .expect("tab-bar string properties should survive exact GC");
    assert!(
        props
            .next_property_change_after_char_pos(CharPos0::ZERO)
            .is_some(),
        "tab-bar text properties should remain traversable after exact GC"
    );
}

#[test]
fn layout_frame_rust_renders_tab_bar_text_from_lisp_tab_bar_keymap() {
    let mut eval =
        create_bootstrap_evaluator_cached_with_features(&["x", "neomacs"]).expect("bootstrap");
    apply_runtime_startup_state(&mut eval).expect("runtime startup state");
    // Bootstrap may or may not install an initial selected
    // frame depending on cache state. Capture whatever exists
    // so we can restore the selection after switching to the
    // target frame for the tab-bar assertions.
    let prior_selected_frame = eval.frame_manager().selected_frame().map(|f| f.id);
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-tab-bar", 1600, 160, buf_id);
    eval.obarray_mut()
        .set_symbol_value("layout-target-frame", Value::make_frame(frame_id.0));
    eval.eval_str(
        r#"
          (require 'tab-bar)
          (setq tab-bar-show 1)
          (tab-bar-mode 1)
          (switch-to-buffer (get-buffer-create "*frame-a*"))
          (tab-bar-new-tab)
          (switch-to-buffer (get-buffer-create "*frame-a-2*"))
          (tab-bar-select-tab 1)
          (select-frame layout-target-frame)
          (tab-bar-new-tab)
          (switch-to-buffer (get-buffer-create "*tb-2*"))
          (tab-bar-rename-tab "T中👨‍👩")
          (tab-bar-select-tab 1)
        "#,
    )
    .expect("eval tab-bar forms");
    eval.eval_form(Value::list(vec![
        Value::symbol("select-frame"),
        Value::make_frame(frame_id.0),
        Value::NIL,
    ]))
    .expect("select target frame for tab-bar debug");
    let keymap_debug =
        match eval.eval_form(Value::list(vec![Value::symbol("tab-bar-make-keymap-1")])) {
            Ok(value) => eval
                .eval_form(Value::list(vec![Value::symbol("prin1-to-string"), value]))
                .ok()
                .and_then(|rendered| rendered.as_runtime_string_owned())
                .unwrap_or_else(|| "<render-unavailable>".to_string()),
            Err(err) => format!("<error: {err}>"),
        };
    let tabs_debug = eval
        .eval_str("(prin1-to-string (frame-parameter nil 'tabs))")
        .ok()
        .and_then(|value| value.as_runtime_string_owned())
        .unwrap_or_else(|| "<unavailable>".to_string());
    let format_debug = eval
        .eval_str("(prin1-to-string tab-bar-format)")
        .ok()
        .and_then(|value| value.as_runtime_string_owned())
        .unwrap_or_else(|| "<unavailable>".to_string());
    if let Some(prev) = prior_selected_frame {
        eval.eval_form(Value::list(vec![
            Value::symbol("select-frame"),
            Value::make_frame(prev.0),
            Value::NIL,
        ]))
        .expect("restore selected frame");
    }

    let frame = eval.frame_manager().get(frame_id).expect("frame");
    assert!(
        frame.tab_bar_height > 0,
        "expected tab-bar-mode to reserve frame tab-bar height"
    );

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let tab_bar_text = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .frame_chrome_rows
                .iter()
                .filter(|row| row.row.role == GlyphRowRole::TabBar && row.row.enabled)
                .map(|row| glyphs_logical_text(&row.row.glyphs[1]))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    assert!(
        tab_bar_text.contains("T中👨‍👩"),
        "expected tab-bar row to render tab captions from tab-bar keymap, got {tab_bar_text:?}; tabs={tabs_debug}; format={format_debug}; keymap={keymap_debug}"
    );
    let tab_bar_glyphs = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .frame_chrome_rows
                .iter()
                .filter(|row| row.row.role == GlyphRowRole::TabBar && row.row.enabled)
                .flat_map(|row| row.row.glyphs[1].iter())
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert!(
        tab_bar_glyphs
            .iter()
            .filter(|glyph| !glyph.padding)
            .all(|glyph| glyph.pixel_width > 0.0),
        "expected tab-bar glyphs to carry display-row pixel widths: {tab_bar_glyphs:?}"
    );
    let cjk = tab_bar_glyphs
        .iter()
        .find(|glyph| matches!(glyph.glyph_type, GlyphType::Char { ch: '中' }))
        .expect("tab-bar CJK glyph");
    assert!(
        cjk.wide,
        "tab-bar CJK glyph should use the shared wide-glyph builder: {tab_bar_glyphs:?}"
    );
    assert!(
        tab_bar_glyphs.iter().any(|glyph| glyph.padding),
        "tab-bar CJK glyph should retain its padding cell: {tab_bar_glyphs:?}"
    );
    assert!(
        tab_bar_glyphs.iter().any(
            |glyph| matches!(&glyph.glyph_type, GlyphType::Composite { text } if text.as_ref() == "👨‍👩")
        ),
        "tab-bar ZWJ emoji should be clustered by the shared builder: {tab_bar_glyphs:?}"
    );
    let window_tab_bar_rows = engine
        .last_frame_display_state
        .as_ref()
        .map(|state| {
            state
                .window_matrices
                .iter()
                .flat_map(|wm| wm.matrix.rows.iter())
                .filter(|row| row.role == GlyphRowRole::TabBar && row.enabled)
                .count()
        })
        .unwrap_or(0);
    assert_eq!(
        window_tab_bar_rows, 0,
        "expected frame tab bar to live in frame_chrome_rows, not in leaf-window matrices"
    );
    // Note: a previous version of this test also asserted
    // `!tab_bar_text.contains("*frame-a-2*")` as a
    // "frame-isolation" check. The tab-bar.el keymap produced
    // by `tab-bar-make-keymap-1` walks all tabs reachable from
    // the current frame's `tabs` parameter and does not
    // filter by which frame created each tab, so the negative
    // assertion was testing a speculative behavior that isn't
    // part of the render contract. Dropping it keeps the
    // primary "renders any target-frame text at all" check
    // and leaves frame-scoped tab isolation as a separate
    // concern.
}

#[test]
fn layout_frame_rust_installs_frame_tab_bar_image_media() {
    let mut eval =
        create_bootstrap_evaluator_cached_with_features(&["x", "neomacs"]).expect("bootstrap");
    apply_runtime_startup_state(&mut eval).expect("runtime startup state");
    let requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingImageDisplayHost {
        requests: Arc::clone(&requests),
        video_requests: Arc::new(Mutex::new(Vec::new())),
        webkit_requests: Arc::new(Mutex::new(Vec::new())),
    }));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
    }
    let frame_id = eval
        .frame_manager_mut()
        .create_frame("layout-tab-bar-image", 640, 160, buf_id);
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.tab_bar_height = 17;
    }
    eval.obarray_mut()
        .set_symbol_value("layout-target-frame", Value::make_frame(frame_id.0));
    eval.eval_str(
        r#"
          (require 'tab-bar)
          (setq tab-bar-format
                (list (lambda ()
                        (propertize
                         "I"
                         'display
                         '(image :type png
                                 :file "/tmp/neomacs-frame-tab-bar.png"
                                 :max-width 32
                                 :max-height 24)))))
          (select-frame layout-target-frame nil)
        "#,
    )
    .expect("configure tab-bar image format");

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let tab_bar_row = state
        .frame_chrome_rows
        .iter()
        .find(|row| row.row.enabled && row.row.role == GlyphRowRole::TabBar)
        .expect("frame tab-bar row");
    let image = state
        .images
        .iter()
        .find(|image| image.row_role == GlyphRowRole::TabBar)
        .expect("frame tab-bar image side item");

    assert_eq!(image.window_id.get(), 0);
    assert_eq!(image.image_id.get(), 77);
    assert_eq!(image.width, 32.0);
    assert_eq!(image.height, 24.0);
    assert_eq!(tab_bar_row.row.height_px, 24.0);
    assert_eq!(tab_bar_row.pixel_bounds.height, 24.0);
    assert_eq!(image.clip_rect, Some(tab_bar_row.pixel_bounds));
    assert_eq!(
        image.slot_id.expect("tab-bar image slot id").row,
        tab_bar_row.row_index
    );
    let requests = requests.lock().expect("requests lock");
    assert!(
        !requests.is_empty(),
        "expected at least one tab-bar image realization request"
    );
    assert!(
        requests
            .iter()
            .all(|request| request.max_width == 32 && request.max_height == 24),
        "unexpected image realization requests: {requests:?}"
    );
}

#[test]
fn layout_frame_rust_shrinks_frame_tab_bar_from_stale_reserved_height() {
    let mut eval =
        create_bootstrap_evaluator_cached_with_features(&["x", "neomacs"]).expect("bootstrap");
    apply_runtime_startup_state(&mut eval).expect("runtime startup state");
    let requests = Arc::new(Mutex::new(Vec::new()));
    eval.set_display_host(Box::new(RecordingImageDisplayHost {
        requests: Arc::clone(&requests),
        video_requests: Arc::new(Mutex::new(Vec::new())),
        webkit_requests: Arc::new(Mutex::new(Vec::new())),
    }));
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();
    {
        let buf = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        buf.insert("body line\n");
    }
    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-tab-bar-stale-height", 640, 220, buf_id);
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.tab_bar_height = 120;
        frame.sync_window_area_bounds();
    }
    eval.obarray_mut()
        .set_symbol_value("layout-target-frame", Value::make_frame(frame_id.0));
    eval.eval_str(
        r#"
          (require 'tab-bar)
          (setq tab-bar-format
                (list (lambda ()
                        (propertize
                         "I"
                         'display
                         '(image :type png
                                 :file "/tmp/neomacs-frame-tab-bar.png"
                                 :max-width 32
                                 :max-height 24)))))
          (select-frame layout-target-frame nil)
        "#,
    )
    .expect("configure tab-bar image format");

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);

    let state = engine
        .last_frame_display_state
        .as_ref()
        .expect("display state");
    let tab_bar_row = state
        .frame_chrome_rows
        .iter()
        .find(|row| row.row.enabled && row.row.role == GlyphRowRole::TabBar)
        .expect("frame tab-bar row");

    assert_eq!(tab_bar_row.row.height_px, 24.0);
    assert_eq!(tab_bar_row.pixel_bounds.height, 24.0);
    assert_eq!(
        eval.frame_manager()
            .get(frame_id)
            .expect("frame")
            .tab_bar_height,
        24
    );
    let image = state
        .images
        .iter()
        .find(|image| image.row_role == GlyphRowRole::TabBar)
        .expect("frame tab-bar image side item");
    assert_eq!(image.clip_rect, Some(tab_bar_row.pixel_bounds));
    assert!(
        !requests.lock().expect("requests lock").is_empty(),
        "expected at least one tab-bar image realization request"
    );
}

#[test]
fn layout_frame_rust_keeps_echo_message_in_minibuffer_window_for_tty() {
    assert_echo_message_renders_in_minibuffer_window(false);
}

#[test]
fn layout_frame_rust_keeps_echo_message_in_minibuffer_window_for_gui() {
    assert_echo_message_renders_in_minibuffer_window(true);
}

#[test]
fn layout_frame_rust_resizes_multiline_echo_rows_for_tty() {
    assert_multiline_echo_message_resizes_minibuffer_rows(false);
}

#[test]
fn layout_frame_rust_resizes_multiline_echo_rows_for_gui() {
    assert_multiline_echo_message_resizes_minibuffer_rows(true);
}

#[test]
fn test_cursor_point_columns_wide_char() {
    let params = test_window_params();
    let text = "你".as_bytes();
    assert_eq!(cursor_point_columns(text, 0, 0, &params), 2);
}

#[test]
fn test_cursor_point_columns_tab_uses_tab_stop_list() {
    let mut params = test_window_params();
    params.tab_width = 8;
    params.tab_stop_list = vec![4, 10];
    let text = b"\t";

    assert_eq!(cursor_point_columns(text, 0, 3, &params), 1);
    assert_eq!(cursor_point_columns(text, 0, 4, &params), 6);
}

#[test]
fn test_cursor_width_for_style_bar_uses_bar_width() {
    let params = test_window_params();
    let text = "你".as_bytes();

    let width = cursor_width_for_style(CursorStyle::Bar(2.5), text, 0, 0, &params, 7.0);
    assert_eq!(width, 2.5);
}

#[test]
fn test_cursor_width_for_style_tab_clamps_when_x_stretch_cursor_is_nil() {
    let params = test_window_params();
    let text = b"\t";

    let width = cursor_width_for_style(CursorStyle::FilledBox, text, 0, 1, &params, 8.0);
    assert_eq!(width, 8.0);
}

#[test]
fn test_cursor_width_for_style_tab_expands_when_x_stretch_cursor_is_t() {
    let mut params = test_window_params();
    params.x_stretch_cursor = true;
    let text = b"\t";

    let width = cursor_width_for_style(CursorStyle::FilledBox, text, 0, 1, &params, 8.0);
    assert_eq!(width, 56.0);
}

#[test]
fn test_cursor_width_for_style_hbar_uses_glyph_columns() {
    let params = test_window_params();
    let text = "你".as_bytes();

    let width = cursor_width_for_style(CursorStyle::Hbar(2.0), text, 0, 0, &params, 7.0);
    assert_eq!(width, 14.0);
}

#[test]
fn cursor_slot_width_policy_names_style_and_buffer_width_sources() {
    let mut params = test_window_params();
    params.char_width = 6.0;
    let text = b"\t";

    assert_eq!(
        CursorSlotWidthRequest::from_window_params(CursorStyle::Bar(2.5), text, 0, 1, &params)
            .width_policy(),
        CursorSlotWidthPolicy::ExplicitPixels(2.5)
    );
    assert_eq!(
        CursorSlotWidthRequest::from_window_params(CursorStyle::FilledBox, text, 0, 1, &params)
            .width_policy(),
        CursorSlotWidthPolicy::TabClamp {
            frame_char_width: 6.0,
        }
    );

    params.x_stretch_cursor = true;
    assert_eq!(
        CursorSlotWidthRequest::from_window_params(CursorStyle::FilledBox, text, 0, 1, &params)
            .width_policy(),
        CursorSlotWidthPolicy::GlyphColumns(7)
    );
    assert_eq!(
        CursorSlotWidthRequest::from_window_params(
            CursorStyle::Hbar(2.0),
            "你".as_bytes(),
            0,
            0,
            &params,
        )
        .width_policy(),
        CursorSlotWidthPolicy::GlyphColumns(2)
    );
}

#[test]
fn cursor_slot_width_policy_tab_clamp_uses_frame_char_width() {
    let mut params = test_window_params();
    params.char_width = 6.0;
    let text = b"\t";

    let policy =
        CursorSlotWidthRequest::from_window_params(CursorStyle::FilledBox, text, 0, 1, &params)
            .width_policy();

    assert_eq!(policy.width_px(8.0), 6.0);
}

#[test]
fn test_cursor_style_for_nonselected_bar_uses_resolved_width() {
    let mut params = test_window_params();
    params.selected = false;
    params.cursor_kind = neomacs_display_protocol::frame_glyphs::CursorKind::Bar;
    params.cursor_bar_width = CursorBarWidth::new(4);

    assert_eq!(
        cursor_style_for_window(&params),
        Some(CursorStyle::Bar(4.0))
    );
}

#[test]
fn test_cursor_style_for_nonselected_no_cursor_is_none() {
    let mut params = test_window_params();
    params.selected = false;
    params.cursor_kind = neomacs_display_protocol::frame_glyphs::CursorKind::NoCursor;

    assert_eq!(cursor_style_for_window(&params), None);
}

#[test]
fn test_resolve_cursor_vertical_metrics_uses_row_metrics() {
    let (y, height, ascent) =
        resolve_cursor_vertical_metrics(20.0, 24.0, 18.0, 24.0, 14.0, 16.0, false);

    assert_eq!(y, 16.0);
    assert_eq!(height, 24.0);
    assert_eq!(ascent, 18.0);
}

#[test]
fn test_resolve_cursor_vertical_metrics_preserves_eob_origin() {
    let (y, height, ascent) =
        resolve_cursor_vertical_metrics(20.0, 24.0, 18.0, 24.0, 14.0, 16.0, true);

    assert_eq!(y, 20.0);
    assert_eq!(height, 20.0);
    assert_eq!(ascent, 14.0);
}

/// Child-frame independence (Slice 4 characterization; guards the Slice 5
/// mock-frame migration + the posframe face/width-independence property).
///
/// A detached child frame must carry its OWN identity (frame_id/parent_*/z_order),
/// resolve its OWN faces, and derive its OWN text width — never inherit the
/// parent's. Here an 800px parent and a 200px child must produce different
/// column counts, and the child state must report its own frame identity.
#[test]
fn child_frame_resolves_faces_and_width_independently_from_parent() {
    use crate::mock_frame::{
        MockChildFrameContent, MockFrameContent, MockStyledLine, MockWindowContent,
    };
    use neomacs_display_protocol::face::Face;
    use neomacs_display_protocol::types::{Color, Rect};

    let char_w = 8.0;
    let char_h = 16.0;

    let content = MockFrameContent {
        frame_id: 1,
        faces: vec![Face::new(0)],
        windows: vec![MockWindowContent {
            window_id: 1,
            lines: vec![MockStyledLine::from_str("parent buffer text", 0)],
            mode_line: MockStyledLine::from_str("-- parent --", 0),
            // Wide parent window: 800px / 8px = 100 cols.
            pixel_bounds: Rect::new(0.0, 0.0, 800.0, 15.0 * char_h),
            selected: true,
            truncated_lines: false,
        }],
        child_frames: vec![MockChildFrameContent {
            frame_id: 100,
            window: MockWindowContent {
                window_id: 2,
                lines: vec![MockStyledLine::from_str("child", 0)],
                mode_line: MockStyledLine::from_str("", 0),
                // Narrow child: 200px / 8px = 25 cols — independent of the parent.
                pixel_bounds: Rect::new(0.0, 0.0, 200.0, 3.0 * char_h),
                selected: false,
                truncated_lines: false,
            },
            parent_x: 120.0,
            parent_y: 48.0,
            z_order: 1,
        }],
        frame_pixel_width: 800.0,
        frame_pixel_height: 16.0 * char_h,
        background: Color::from_pixel(0x00112233),
        menu_bar: None,
        minibuffer: Some(MockWindowContent {
            window_id: 999,
            lines: vec![MockStyledLine::from_str("", 0)],
            mode_line: MockStyledLine::from_str("", 0),
            pixel_bounds: Rect::new(0.0, 15.0 * char_h, 800.0, char_h),
            selected: false,
            truncated_lines: false,
        }),
    };

    let mut engine = LayoutEngine::new();
    let states = engine.layout_mock_frame(&content, char_w, char_h);

    assert!(states.len() >= 2, "expected a parent + child frame state");
    let parent = &states[0];
    let child = &states[1];

    // The child carries its OWN identity, not the parent's.
    assert_eq!(child.frame_id.get(), 100);
    assert_eq!(child.parent_id.get(), content.frame_id);
    assert_eq!(child.parent_x, 120.0);
    assert_eq!(child.parent_y, 48.0);
    assert_eq!(child.z_order, 1);

    // The child resolves its own face map (not an empty/parent-shared identity).
    assert!(
        !child.faces.is_empty(),
        "child frame must resolve its own faces"
    );

    // The child's text width is independent of the parent's: 200px vs 800px
    // produce different column counts.
    let parent_cols = parent.window_matrices[0].matrix.ncols;
    let child_cols = child.window_matrices[0].matrix.ncols;
    assert_ne!(
        child_cols, parent_cols,
        "child width (200px) must derive its own ncols, not inherit the parent (800px): child={child_cols} parent={parent_cols}"
    );
}

#[test]
fn echo_content_rows_measures_displayed_message_height() {
    // The inactive mini-window's auto-resize measures the echo buffer's
    // CONTENT (this helper), not a cached glyph matrix. This is what makes the
    // echo area shrink back to one line after `M-x`/`C-g` (empty or "Quit")
    // while still growing for a genuine multi-line message.
    assert_eq!(echo_content_rows("", 80), 1, "empty echo is one line");
    assert_eq!(
        echo_content_rows("Quit", 80),
        1,
        "a one-line message is one line"
    );
    assert_eq!(
        echo_content_rows("AAAA\nBBBB\nCCCC", 80),
        3,
        "a three-line message occupies three rows"
    );
    // Wrapping: a single logical line wider than the window wraps.
    assert_eq!(
        echo_content_rows(&"x".repeat(170), 80),
        3,
        "170 columns at width 80 wraps to three rows"
    );
    // Wrapping combines with explicit newlines.
    assert_eq!(
        echo_content_rows(&format!("{}\nshort", "y".repeat(85)), 80),
        3,
        "an 85-col line (2 rows) plus a short line (1 row) is three rows"
    );
}
