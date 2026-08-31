use std::path::{Path, PathBuf};

use neomacs_video_backend_abi::{BACKEND_ABI_VERSION, BackendApi, BackendApiHeader};

use super::loader::{
    BackendLoadError, backend_library_candidates, configured_backend_library_candidates,
    load_backend_from_candidates, validate_backend_api, validate_backend_header,
};

#[test]
fn backend_discovery_covers_build_bundle_and_installed_archlib_layouts() {
    let executable = Path::new("/opt/neomacs/bin/neomacs");
    let candidates = backend_library_candidates(executable);

    assert_eq!(
        candidates,
        [
            PathBuf::from("/opt/neomacs/bin/libneomacs_video_gstreamer.so"),
            PathBuf::from("/opt/neomacs/bin/libexec/libneomacs_video_gstreamer.so"),
            PathBuf::from(format!(
                "/opt/neomacs/libexec/neomacs/{}/{}/libneomacs_video_gstreamer.so",
                env!("CARGO_PKG_VERSION"),
                env!("NEOMACS_VIDEO_HOST_TRIPLE"),
            )),
        ]
    );
}

#[test]
fn explicit_backend_path_replaces_automatic_discovery() {
    let executable = Path::new("/opt/neomacs/bin/neomacs");
    let configured = Path::new("/srv/neomacs/video/backend.so");

    assert_eq!(
        configured_backend_library_candidates(executable, Some(configured.as_os_str()))
            .expect("absolute overrides are accepted"),
        [configured.to_path_buf()]
    );
}

#[test]
fn relative_backend_override_is_rejected_instead_of_using_the_working_directory() {
    let executable = Path::new("/opt/neomacs/bin/neomacs");
    let configured = Path::new("plugins/video.so");

    assert_eq!(
        configured_backend_library_candidates(executable, Some(configured.as_os_str())),
        Err(BackendLoadError::RelativeOverride {
            path: configured.to_path_buf(),
        })
    );
}

#[test]
fn absent_backend_is_a_typed_unavailable_result() {
    let candidates = [
        PathBuf::from("/definitely-absent/neomacs-video-one.so"),
        PathBuf::from("/definitely-absent/neomacs-video-two.so"),
    ];

    let error = match load_backend_from_candidates(&candidates) {
        Ok(_) => panic!("an absent optional backend must not load"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        BackendLoadError::Unavailable {
            attempted: candidates.into(),
        }
    );
}

#[test]
fn incompatible_backend_abi_is_rejected_before_any_function_is_called() {
    let path = Path::new("/opt/neomacs/libneomacs_video_gstreamer.so");
    let incompatible = BackendApiHeader {
        abi_version: BACKEND_ABI_VERSION + 1,
        struct_size: size_of::<BackendApiHeader>(),
    };

    assert_eq!(
        validate_backend_header(path, incompatible),
        Err(BackendLoadError::IncompatibleAbi {
            path: path.to_path_buf(),
            expected: BACKEND_ABI_VERSION,
            actual: BACKEND_ABI_VERSION + 1,
        })
    );
}

#[test]
fn truncated_backend_table_is_rejected_before_reading_function_pointers() {
    let path = Path::new("/opt/neomacs/libneomacs_video_gstreamer.so");
    let truncated = BackendApiHeader {
        abi_version: BACKEND_ABI_VERSION,
        struct_size: size_of::<BackendApiHeader>(),
    };

    assert_eq!(
        validate_backend_header(path, truncated),
        Err(BackendLoadError::TruncatedAbi {
            path: path.to_path_buf(),
            expected_at_least: size_of::<BackendApi>(),
            actual: size_of::<BackendApiHeader>(),
        })
    );
}

#[test]
fn backend_table_must_supply_every_required_operation() {
    let path = Path::new("/opt/neomacs/libneomacs_video_gstreamer.so");

    assert_eq!(
        validate_backend_api(path, BackendApi::CURRENT),
        Err(BackendLoadError::MissingOperation {
            path: path.to_path_buf(),
            operation: "create",
        })
    );
}
