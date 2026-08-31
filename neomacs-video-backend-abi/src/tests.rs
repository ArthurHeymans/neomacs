use super::*;

#[test]
fn current_header_covers_the_complete_function_table() {
    assert_eq!(BackendApi::CURRENT.header.abi_version, BACKEND_ABI_VERSION);
    assert_eq!(
        BackendApi::CURRENT.header.struct_size,
        size_of::<BackendApi>()
    );
}

#[test]
fn validation_rejects_incompatible_truncated_and_incomplete_tables() {
    assert_eq!(
        BackendApiHeader {
            abi_version: BACKEND_ABI_VERSION + 1,
            struct_size: size_of::<BackendApi>(),
        }
        .validate(),
        Err(BackendApiValidationError::IncompatibleAbi {
            expected: BACKEND_ABI_VERSION,
            actual: BACKEND_ABI_VERSION + 1,
        })
    );
    assert_eq!(
        BackendApiHeader {
            abi_version: BACKEND_ABI_VERSION,
            struct_size: size_of::<BackendApiHeader>(),
        }
        .validate(),
        Err(BackendApiValidationError::Truncated {
            expected_at_least: size_of::<BackendApi>(),
            actual: size_of::<BackendApiHeader>(),
        })
    );
    assert_eq!(
        BackendApi::CURRENT.validate(),
        Err(BackendApiValidationError::MissingOperation("create"))
    );
}

#[test]
fn fixed_error_buffer_round_trips_utf8_and_truncates_at_a_character_boundary() {
    let mut error = BackendError::default();
    error.write("decoder unavailable: αβγ");
    assert_eq!(error.message(), "decoder unavailable: αβγ");

    let oversized = "界".repeat(BACKEND_ERROR_CAPACITY);
    error.write(&oversized);
    assert!(error.message().len() <= BACKEND_ERROR_CAPACITY);
    assert!(error.message().chars().all(|ch| ch == '界'));
}

#[test]
fn frame_event_starts_without_an_owned_plugin_frame() {
    let event = BackendEvent::default();
    assert!(event.frame.is_null());
    assert_eq!(event.kind, EVENT_NONE);
    assert_eq!(event.frame_info.plane_count, 0);
}

#[test]
fn version_two_frame_layout_describes_format_color_and_plane_objects() {
    assert_eq!(BACKEND_ABI_VERSION, 2);
    assert_eq!(BACKEND_ENTRY_SYMBOL, b"neomacs_video_backend_v2\0");

    let frame = BackendFrameInfo {
        format: FORMAT_NV12,
        color_primaries: COLOR_PRIMARIES_BT709,
        color_transfer: COLOR_TRANSFER_BT709,
        color_matrix: COLOR_MATRIX_BT709,
        color_range: COLOR_RANGE_LIMITED,
        chroma_location: CHROMA_LOCATION_LEFT,
        object_count: 1,
        plane_count: 2,
        plane_object_indices: [0, 0, 0, 0],
        plane_strides: [1920, 1920, 0, 0],
        plane_offsets: [0, 1920 * 1080, 0, 0],
        ..BackendFrameInfo::default()
    };

    assert_eq!(frame.object_count, 1);
    assert_eq!(frame.plane_count, 2);
    assert_eq!(frame.plane_object_indices[..2], [0, 0]);
}

#[test]
fn validation_rejects_the_previous_abi_version() {
    assert_eq!(
        BackendApiHeader {
            abi_version: 1,
            struct_size: size_of::<BackendApi>(),
        }
        .validate(),
        Err(BackendApiValidationError::IncompatibleAbi {
            expected: 2,
            actual: 1,
        })
    );
}
