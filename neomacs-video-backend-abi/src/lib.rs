//! Stable, C-compatible interface between Neomacs and optional video decoder
//! adapters. This crate contains data declarations only and links no native
//! multimedia libraries.

use core::ffi::c_void;

pub const BACKEND_ABI_VERSION: u32 = 2;
pub const BACKEND_ENTRY_SYMBOL: &[u8] = b"neomacs_video_backend_v2\0";
pub const BACKEND_ERROR_CAPACITY: usize = 512;
pub const MAX_DMABUF_PLANES: usize = 4;

pub const STATUS_OK: u32 = 0;
pub const STATUS_ERROR: u32 = 1;

pub const POLL_EMPTY: u32 = 0;
pub const POLL_EVENT: u32 = 1;
pub const POLL_ERROR: u32 = 2;

pub const TRANSFER_REQUIRE_DIRECT: u32 = 0;
pub const TRANSFER_ALLOW_GPU_COPY: u32 = 1;
pub const TRANSFER_ALLOW_CPU: u32 = 2;

pub const COMMAND_OPEN: u32 = 1;
pub const COMMAND_PLAY: u32 = 2;
pub const COMMAND_PAUSE: u32 = 3;
pub const COMMAND_STOP: u32 = 4;
pub const COMMAND_SEEK: u32 = 5;
pub const COMMAND_SET_RATE: u32 = 6;
pub const COMMAND_SET_LOOP: u32 = 7;
pub const COMMAND_SET_PRESENTATION: u32 = 8;
pub const COMMAND_CLOSE: u32 = 9;

pub const SOURCE_FILE: u32 = 1;
pub const SOURCE_URI: u32 = 2;
pub const INITIAL_PAUSED: u32 = 0;
pub const INITIAL_PLAYING: u32 = 1;
pub const LOOP_OFF: u32 = 0;
pub const LOOP_INFINITE: u32 = 1;
pub const LOOP_COUNT: u32 = 2;
pub const PRESENTATION_HIDDEN: u32 = 0;
pub const PRESENTATION_PRESENTED: u32 = 1;

pub const EVENT_NONE: u32 = 0;
pub const EVENT_OPENED: u32 = 1;
pub const EVENT_FRAME: u32 = 2;
pub const EVENT_FRAMES_REPLACED: u32 = 3;
pub const EVENT_STATE_CHANGED: u32 = 4;
pub const EVENT_LOOPED: u32 = 5;
pub const EVENT_ENDED: u32 = 6;
pub const EVENT_FAILED: u32 = 7;

pub const STATE_LOADING: u32 = 0;
pub const STATE_PLAYING: u32 = 1;
pub const STATE_PAUSED: u32 = 2;
pub const STATE_ENDED: u32 = 3;
pub const STATE_FAILED: u32 = 4;
pub const STATE_CLOSED: u32 = 5;

pub const STORAGE_NONE: u32 = 0;
pub const STORAGE_CPU_PACKED: u32 = 1;
pub const STORAGE_DMABUF: u32 = 2;
pub const FORMAT_RGBA8: u32 = 1;
pub const FORMAT_BGRA8: u32 = 2;
pub const FORMAT_NV12: u32 = 3;
pub const FORMAT_P010: u32 = 4;
pub const COLOR_PRIMARIES_BT601_525: u32 = 1;
pub const COLOR_PRIMARIES_BT601_625: u32 = 2;
pub const COLOR_PRIMARIES_BT709: u32 = 3;
pub const COLOR_PRIMARIES_BT2020: u32 = 4;
pub const COLOR_TRANSFER_SRGB: u32 = 1;
pub const COLOR_TRANSFER_BT709: u32 = 2;
pub const COLOR_TRANSFER_PQ: u32 = 3;
pub const COLOR_TRANSFER_HLG: u32 = 4;
pub const COLOR_MATRIX_IDENTITY: u32 = 1;
pub const COLOR_MATRIX_BT601: u32 = 2;
pub const COLOR_MATRIX_BT709: u32 = 3;
pub const COLOR_MATRIX_BT2020_NCL: u32 = 4;
pub const COLOR_RANGE_LIMITED: u32 = 1;
pub const COLOR_RANGE_FULL: u32 = 2;
pub const CHROMA_LOCATION_LEFT: u32 = 1;
pub const CHROMA_LOCATION_CENTER: u32 = 2;
pub const CHROMA_LOCATION_TOP_LEFT: u32 = 3;
pub const SYNCHRONIZATION_IMPLICIT: u32 = 1;
pub const TRANSFER_DIRECT_EXTERNAL: u32 = 1;
pub const TRANSFER_GPU_INTEROP_COPY: u32 = 2;
pub const TRANSFER_CPU_UPLOAD: u32 = 3;
pub const ROTATION_NONE: u32 = 0;
pub const ROTATION_90: u32 = 1;
pub const ROTATION_180: u32 = 2;
pub const ROTATION_270: u32 = 3;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendApiHeader {
    pub abi_version: u32,
    pub struct_size: usize,
}

impl BackendApiHeader {
    pub const fn for_struct<T>() -> Self {
        Self {
            abi_version: BACKEND_ABI_VERSION,
            struct_size: size_of::<T>(),
        }
    }

    pub fn validate(self) -> Result<(), BackendApiValidationError> {
        if self.abi_version != BACKEND_ABI_VERSION {
            return Err(BackendApiValidationError::IncompatibleAbi {
                expected: BACKEND_ABI_VERSION,
                actual: self.abi_version,
            });
        }
        if self.struct_size < size_of::<BackendApi>() {
            return Err(BackendApiValidationError::Truncated {
                expected_at_least: size_of::<BackendApi>(),
                actual: self.struct_size,
            });
        }
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct BackendError {
    pub len: u32,
    pub bytes: [u8; BACKEND_ERROR_CAPACITY],
}

impl Default for BackendError {
    fn default() -> Self {
        Self {
            len: 0,
            bytes: [0; BACKEND_ERROR_CAPACITY],
        }
    }
}

impl BackendError {
    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn write(&mut self, message: &str) {
        let mut len = message.len().min(BACKEND_ERROR_CAPACITY);
        while !message.is_char_boundary(len) {
            len -= 1;
        }
        self.bytes[..len].copy_from_slice(&message.as_bytes()[..len]);
        self.len = len as u32;
    }

    pub fn message(&self) -> &str {
        let len = usize::try_from(self.len)
            .unwrap_or(BACKEND_ERROR_CAPACITY)
            .min(BACKEND_ERROR_CAPACITY);
        core::str::from_utf8(&self.bytes[..len]).unwrap_or("invalid UTF-8 from video backend")
    }
}

pub type WakeCallback = unsafe extern "C" fn(userdata: *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BackendCreateOptions {
    pub transfer_policy: u32,
    pub renderer_drm_major: i32,
    pub renderer_drm_minor: i32,
    pub wake: Option<WakeCallback>,
    pub wake_userdata: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BackendCommand {
    pub kind: u32,
    pub id: u32,
    pub source_kind: u32,
    pub source_ptr: *const u8,
    pub source_len: usize,
    pub initial_playback: u32,
    pub loop_kind: u32,
    pub loop_count: u32,
    pub media_time_ns: u64,
    pub playback_rate: f64,
    pub presentation: u32,
}

impl Default for BackendCommand {
    fn default() -> Self {
        Self {
            kind: 0,
            id: 0,
            source_kind: 0,
            source_ptr: core::ptr::null(),
            source_len: 0,
            initial_playback: INITIAL_PAUSED,
            loop_kind: LOOP_OFF,
            loop_count: 0,
            media_time_ns: 0,
            playback_rate: 1.0,
            presentation: PRESENTATION_HIDDEN,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BackendFrameInfo {
    pub storage: u32,
    pub format: u32,
    pub color_primaries: u32,
    pub color_transfer: u32,
    pub color_matrix: u32,
    pub color_range: u32,
    pub chroma_location: u32,
    pub transfer_path: u32,
    pub synchronization: u32,
    pub object_count: u32,
    pub plane_count: u32,
    pub plane_object_indices: [u32; MAX_DMABUF_PLANES],
    pub object_modifiers: [u64; MAX_DMABUF_PLANES],
    pub plane_strides: [u32; MAX_DMABUF_PLANES],
    pub plane_offsets: [u32; MAX_DMABUF_PLANES],
    pub cpu_len: usize,
    pub stride: u32,
    pub fourcc: u32,
    pub pts_ns: u64,
    pub duration_ns: u64,
    pub epoch: u64,
    pub coded_width: u32,
    pub coded_height: u32,
    pub visible_x: u32,
    pub visible_y: u32,
    pub visible_width: u32,
    pub visible_height: u32,
    pub display_width: u32,
    pub display_height: u32,
    pub pixel_aspect_numerator: u32,
    pub pixel_aspect_denominator: u32,
    pub rotation: u32,
}

#[repr(C)]
#[derive(Clone)]
pub struct BackendEvent {
    pub kind: u32,
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub state: u32,
    pub count: u64,
    pub loop_kind: u32,
    pub loop_count: u32,
    pub frame: *mut c_void,
    pub frame_info: BackendFrameInfo,
    pub error: BackendError,
}

impl Default for BackendEvent {
    fn default() -> Self {
        Self {
            kind: EVENT_NONE,
            id: 0,
            width: 0,
            height: 0,
            state: STATE_LOADING,
            count: 0,
            loop_kind: LOOP_OFF,
            loop_count: 0,
            frame: core::ptr::null_mut(),
            frame_info: BackendFrameInfo::default(),
            error: BackendError::default(),
        }
    }
}

pub type BackendCreateFn = unsafe extern "C" fn(
    options: *const BackendCreateOptions,
    error: *mut BackendError,
) -> *mut c_void;
pub type BackendDestroyFn = unsafe extern "C" fn(backend: *mut c_void);
pub type BackendCommandFn = unsafe extern "C" fn(
    backend: *mut c_void,
    command: *const BackendCommand,
    error: *mut BackendError,
) -> u32;
pub type BackendPollEventFn = unsafe extern "C" fn(
    backend: *mut c_void,
    event: *mut BackendEvent,
    error: *mut BackendError,
) -> u32;
pub type BackendCopyFrameFn = unsafe extern "C" fn(
    frame: *mut c_void,
    destination: *mut u8,
    destination_len: usize,
    error: *mut BackendError,
) -> u32;
/// Duplicate one DMA-BUF memory-object descriptor. Planes refer to these
/// objects through `BackendFrameInfo::plane_object_indices`. The caller owns
/// a non-negative result and must close it; the backend retains its original.
pub type BackendDuplicateFrameObjectFdFn =
    unsafe extern "C" fn(frame: *mut c_void, object: u32, error: *mut BackendError) -> i32;
pub type BackendReleaseFrameFn = unsafe extern "C" fn(frame: *mut c_void);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BackendApi {
    pub header: BackendApiHeader,
    pub create: Option<BackendCreateFn>,
    pub destroy: Option<BackendDestroyFn>,
    pub command: Option<BackendCommandFn>,
    pub poll_event: Option<BackendPollEventFn>,
    pub copy_frame: Option<BackendCopyFrameFn>,
    pub duplicate_frame_object_fd: Option<BackendDuplicateFrameObjectFdFn>,
    pub release_frame: Option<BackendReleaseFrameFn>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendApiValidationError {
    IncompatibleAbi {
        expected: u32,
        actual: u32,
    },
    Truncated {
        expected_at_least: usize,
        actual: usize,
    },
    MissingOperation(&'static str),
}

impl BackendApi {
    pub const CURRENT: Self = Self {
        header: BackendApiHeader::for_struct::<Self>(),
        create: None,
        destroy: None,
        command: None,
        poll_event: None,
        copy_frame: None,
        duplicate_frame_object_fd: None,
        release_frame: None,
    };

    pub fn validate(self) -> Result<(), BackendApiValidationError> {
        self.header.validate()?;
        for (operation, present) in [
            ("create", self.create.is_some()),
            ("destroy", self.destroy.is_some()),
            ("command", self.command.is_some()),
            ("poll_event", self.poll_event.is_some()),
            ("copy_frame", self.copy_frame.is_some()),
            (
                "duplicate_frame_object_fd",
                self.duplicate_frame_object_fd.is_some(),
            ),
            ("release_frame", self.release_frame.is_some()),
        ] {
            if !present {
                return Err(BackendApiValidationError::MissingOperation(operation));
            }
        }
        Ok(())
    }
}

pub type BackendEntryFn = unsafe extern "C" fn() -> *const BackendApi;

#[cfg(test)]
mod tests;
