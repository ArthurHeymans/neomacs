use std::ffi::{OsStr, c_void};
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;

use neomacs_video_backend_abi as abi;

const BACKEND_LIBRARY_NAME: &str = "libneomacs_video_gstreamer.so";
pub(super) const BACKEND_OVERRIDE_ENV: &str = "NEOMACS_VIDEO_BACKEND";

/// Why the optional Linux decoder adapter could not be loaded. Absence is a
/// supported installation state; malformed or incompatible adapters remain
/// distinguishable so packaging defects are actionable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum BackendLoadError {
    #[error("optional video backend is unavailable; tried {attempted:?}")]
    Unavailable { attempted: Vec<PathBuf> },
    #[error("NEOMACS_VIDEO_BACKEND must be an absolute path, got {path}")]
    RelativeOverride { path: PathBuf },
    #[error("failed to load video backend {path}: {message}")]
    InvalidBackend { path: PathBuf, message: String },
    #[error("video backend {path} uses ABI {actual}, but this Neomacs requires ABI {expected}")]
    IncompatibleAbi {
        path: PathBuf,
        expected: u32,
        actual: u32,
    },
    #[error(
        "video backend {path} exposes a {actual}-byte table; at least {expected_at_least} bytes are required"
    )]
    TruncatedAbi {
        path: PathBuf,
        expected_at_least: usize,
        actual: usize,
    },
    #[error("video backend {path} does not provide required operation {operation}")]
    MissingOperation {
        path: PathBuf,
        operation: &'static str,
    },
}

pub(crate) struct LoadedBackend {
    // Must outlive every copied function pointer and opaque frame handle.
    _library: libloading::Library,
    create: abi::BackendCreateFn,
    destroy: abi::BackendDestroyFn,
    command: abi::BackendCommandFn,
    poll_event: abi::BackendPollEventFn,
    copy_frame: abi::BackendCopyFrameFn,
    duplicate_frame_object_fd: abi::BackendDuplicateFrameObjectFdFn,
    release_frame: abi::BackendReleaseFrameFn,
}

impl LoadedBackend {
    pub(super) fn create(
        &self,
        options: &abi::BackendCreateOptions,
    ) -> Result<NonNull<c_void>, String> {
        let mut error = abi::BackendError::default();
        // SAFETY: the loaded table was validated before construction, and
        // both pointers remain valid for the duration of this call.
        let instance = unsafe { (self.create)(options, &mut error) };
        NonNull::new(instance).ok_or_else(|| backend_message(&error, "backend creation failed"))
    }

    pub(super) fn destroy(&self, instance: NonNull<c_void>) {
        // SAFETY: this is the instance returned by `create`, destroyed once.
        unsafe { (self.destroy)(instance.as_ptr()) };
    }

    pub(super) fn command(
        &self,
        instance: NonNull<c_void>,
        command: &abi::BackendCommand,
    ) -> Result<(), String> {
        let mut error = abi::BackendError::default();
        // SAFETY: instance and command obey the validated v2 contract.
        let status = unsafe { (self.command)(instance.as_ptr(), command, &mut error) };
        match status {
            abi::STATUS_OK => Ok(()),
            abi::STATUS_ERROR => Err(backend_message(&error, "video command failed")),
            status => Err(format!(
                "video backend returned unknown command status {status}"
            )),
        }
    }

    pub(super) fn poll_event(
        &self,
        instance: NonNull<c_void>,
        event: &mut abi::BackendEvent,
    ) -> Result<u32, String> {
        let mut error = abi::BackendError::default();
        // SAFETY: instance and output storage obey the validated v2 contract.
        let status = unsafe { (self.poll_event)(instance.as_ptr(), event, &mut error) };
        match status {
            abi::POLL_EMPTY | abi::POLL_EVENT => Ok(status),
            abi::POLL_ERROR => Err(backend_message(&error, "video event polling failed")),
            status => Err(format!(
                "video backend returned unknown poll status {status}"
            )),
        }
    }

    pub(super) fn copy_frame(
        &self,
        frame: NonNull<c_void>,
        destination: &mut [u8],
    ) -> Result<(), String> {
        let mut error = abi::BackendError::default();
        // SAFETY: the opaque frame belongs to this backend, and the mutable
        // slice describes the complete writable destination.
        let status = unsafe {
            (self.copy_frame)(
                frame.as_ptr(),
                destination.as_mut_ptr(),
                destination.len(),
                &mut error,
            )
        };
        match status {
            abi::STATUS_OK => Ok(()),
            abi::STATUS_ERROR => Err(backend_message(&error, "video frame copy failed")),
            status => Err(format!(
                "video backend returned unknown frame-copy status {status}"
            )),
        }
    }

    pub(super) fn duplicate_frame_object_fd(
        &self,
        frame: NonNull<c_void>,
        object: u32,
    ) -> Result<OwnedFd, String> {
        let mut error = abi::BackendError::default();
        // SAFETY: the opaque frame belongs to this backend; a non-negative
        // result transfers ownership of a duplicated descriptor to us.
        let fd = unsafe { (self.duplicate_frame_object_fd)(frame.as_ptr(), object, &mut error) };
        if fd < 0 {
            Err(backend_message(
                &error,
                "video backend failed to duplicate a DMA-BUF descriptor",
            ))
        } else {
            // SAFETY: the plugin contract transfers ownership on success.
            Ok(unsafe { OwnedFd::from_raw_fd(fd) })
        }
    }

    pub(super) fn release_frame(&self, frame: NonNull<c_void>) {
        // SAFETY: each frame event transfers one handle, released exactly once.
        unsafe { (self.release_frame)(frame.as_ptr()) };
    }
}

/// Candidate locations shared by uninstalled builds, app bundles, and GNU's
/// installed architecture-dependent `libexec` layout.
pub(crate) fn backend_library_candidates(executable: &Path) -> Vec<PathBuf> {
    let executable_dir = executable.parent().unwrap_or_else(|| Path::new("."));
    let mut candidates = vec![
        executable_dir.join(BACKEND_LIBRARY_NAME),
        executable_dir.join("libexec").join(BACKEND_LIBRARY_NAME),
    ];
    if let Some(prefix) = executable_dir.parent() {
        candidates.push(
            prefix
                .join("libexec")
                .join("neomacs")
                .join(env!("CARGO_PKG_VERSION"))
                .join(env!("NEOMACS_VIDEO_HOST_TRIPLE"))
                .join(BACKEND_LIBRARY_NAME),
        );
    }
    candidates
}

pub(crate) fn configured_backend_library_candidates(
    executable: &Path,
    configured: Option<&OsStr>,
) -> Result<Vec<PathBuf>, BackendLoadError> {
    match configured {
        Some(path) => {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                return Err(BackendLoadError::RelativeOverride { path });
            }
            Ok(vec![path])
        }
        None => Ok(backend_library_candidates(executable)),
    }
}

pub(crate) fn load_backend_from_candidates(
    candidates: &[PathBuf],
) -> Result<LoadedBackend, BackendLoadError> {
    let mut first_rejection = None;
    for path in candidates {
        if !path.is_file() {
            continue;
        }
        match load_backend(path) {
            Ok(backend) => return Ok(backend),
            Err(error) => {
                first_rejection.get_or_insert(error);
            }
        }
    }
    if let Some(error) = first_rejection {
        Err(error)
    } else {
        Err(BackendLoadError::Unavailable {
            attempted: candidates.to_vec(),
        })
    }
}

fn load_backend(path: &Path) -> Result<LoadedBackend, BackendLoadError> {
    // SAFETY: loading executes third-party initializers by definition. Search
    // paths are explicit and never include the process working directory.
    let library = unsafe { libloading::Library::new(path) }.map_err(|error| {
        BackendLoadError::InvalidBackend {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    })?;
    // SAFETY: the symbol name and C signature are fixed by ABI v2.
    let entry = unsafe { library.get::<abi::BackendEntryFn>(abi::BACKEND_ENTRY_SYMBOL) }.map_err(
        |error| BackendLoadError::InvalidBackend {
            path: path.to_path_buf(),
            message: error.to_string(),
        },
    )?;
    // SAFETY: the entry function returns immutable process-lifetime table data.
    let api_pointer = unsafe { entry() };
    let api_pointer =
        NonNull::new(api_pointer.cast_mut()).ok_or_else(|| BackendLoadError::InvalidBackend {
            path: path.to_path_buf(),
            message: "entry point returned a null function table".to_owned(),
        })?;
    // Read only the fixed prefix until its size proves the full table exists.
    // SAFETY: a non-null v2 entry must expose at least its header prefix.
    let header = unsafe { api_pointer.cast::<abi::BackendApiHeader>().as_ptr().read() };
    validate_backend_header(path, header)?;
    // SAFETY: `validate_backend_header` proved the complete table is readable.
    let api = unsafe { api_pointer.as_ptr().read() };
    validate_backend_api(path, api)?;
    Ok(LoadedBackend {
        _library: library,
        create: api.create.expect("validated create operation"),
        destroy: api.destroy.expect("validated destroy operation"),
        command: api.command.expect("validated command operation"),
        poll_event: api.poll_event.expect("validated poll operation"),
        copy_frame: api.copy_frame.expect("validated frame-copy operation"),
        duplicate_frame_object_fd: api
            .duplicate_frame_object_fd
            .expect("validated DMA-BUF duplication operation"),
        release_frame: api
            .release_frame
            .expect("validated frame-release operation"),
    })
}

pub(crate) fn validate_backend_header(
    path: &Path,
    header: abi::BackendApiHeader,
) -> Result<(), BackendLoadError> {
    match header.validate() {
        Ok(()) => Ok(()),
        Err(abi::BackendApiValidationError::IncompatibleAbi { expected, actual }) => {
            Err(BackendLoadError::IncompatibleAbi {
                path: path.to_path_buf(),
                expected,
                actual,
            })
        }
        Err(abi::BackendApiValidationError::Truncated {
            expected_at_least,
            actual,
        }) => Err(BackendLoadError::TruncatedAbi {
            path: path.to_path_buf(),
            expected_at_least,
            actual,
        }),
        Err(abi::BackendApiValidationError::MissingOperation(_)) => {
            unreachable!("header validation does not inspect operations")
        }
    }
}

pub(crate) fn validate_backend_api(
    path: &Path,
    api: abi::BackendApi,
) -> Result<(), BackendLoadError> {
    match api.validate() {
        Ok(()) => Ok(()),
        Err(abi::BackendApiValidationError::MissingOperation(operation)) => {
            Err(BackendLoadError::MissingOperation {
                path: path.to_path_buf(),
                operation,
            })
        }
        Err(abi::BackendApiValidationError::IncompatibleAbi { .. })
        | Err(abi::BackendApiValidationError::Truncated { .. }) => {
            validate_backend_header(path, api.header)
        }
    }
}

pub(super) fn backend_message(error: &abi::BackendError, fallback: &str) -> String {
    let message = error.message();
    if message.is_empty() {
        fallback.to_owned()
    } else {
        message.to_owned()
    }
}
