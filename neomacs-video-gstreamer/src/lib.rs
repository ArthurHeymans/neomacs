//! GStreamer implementation of Neomacs's versioned optional video-backend
//! interface. The editor loads this shared library only when video is first
//! initialized, so its native dependencies never enter the main executable's
//! loader closure.

#![cfg(target_os = "linux")]

use std::collections::VecDeque;
use std::ffi::OsString;
use std::num::NonZeroU32;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStringExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;

use neomacs_display_protocol::types::VideoId;
use neomacs_video_backend_abi as abi;

mod backend;
mod decoder;
mod frame;
mod sampling;

#[cfg(test)]
#[path = "lib_test.rs"]
mod lib_test;

pub(crate) use neomacs_video_model::{
    FrameTiming, FrameTransferPolicy, InitialPlayback, LoopMode, MediaTime, PixelAspectRatio,
    PixelRect, PlaybackAction, PlaybackEpoch, PlaybackRate, PresentationVisibility, VideoCommand,
    VideoCommandError, VideoGeometry, VideoRotation, VideoSampling, VideoSessionState, VideoSource,
    VideoTransferPath,
};

use backend::{BackendEvent as DecoderEvent, DecodedFrame, DecoderBackend};
use decoder::GstreamerDecoder;
use frame::{LinuxFrameLease, LinuxFrameStorage};
use sampling::LinuxDrmDevice;

#[derive(Clone)]
pub(crate) struct VideoWake(Arc<dyn Fn() + Send + Sync>);

impl VideoWake {
    pub(crate) fn new(wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self(Arc::new(wake))
    }

    pub(crate) fn notify_for_backend(&self) {
        (self.0)();
    }
}

struct PluginBackend {
    decoder: GstreamerDecoder,
    pending: VecDeque<DecoderEvent<LinuxFrameLease>>,
}

struct PluginFrame(DecodedFrame<LinuxFrameLease>);

pub static BACKEND_API: abi::BackendApi = abi::BackendApi {
    header: abi::BackendApiHeader::for_struct::<abi::BackendApi>(),
    create: Some(backend_create),
    destroy: Some(backend_destroy),
    command: Some(backend_command),
    poll_event: Some(backend_poll_event),
    copy_frame: Some(backend_copy_frame),
    duplicate_frame_fd: Some(backend_duplicate_frame_fd),
    release_frame: Some(backend_release_frame),
};

/// Return the immutable v1 function table. The table contains no Rust ABI
/// types, and every entry catches panics before they can cross this boundary.
#[unsafe(no_mangle)]
pub extern "C" fn neomacs_video_backend_v1() -> *const abi::BackendApi {
    &BACKEND_API
}

unsafe extern "C" fn backend_create(
    options: *const abi::BackendCreateOptions,
    error: *mut abi::BackendError,
) -> *mut core::ffi::c_void {
    clear_error(error);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let options = unsafe { ptr_ref(options, "backend create options") }?;
        let transfer_policy = decode_transfer_policy(options.transfer_policy)?;
        let renderer_drm_device = decode_drm_device(options)?;
        let wake_callback = options.wake;
        let wake_userdata = options.wake_userdata as usize;
        let wake = VideoWake::new(move || {
            if let Some(callback) = wake_callback {
                // The host owns this context until `destroy` returns. Decoder
                // teardown joins every worker before allowing that to happen.
                unsafe { callback(wake_userdata as *mut core::ffi::c_void) };
            }
        });
        let decoder = GstreamerDecoder::new(wake, transfer_policy, renderer_drm_device)?;
        Ok::<_, String>(Box::into_raw(Box::new(PluginBackend {
            decoder,
            pending: VecDeque::new(),
        })) as *mut core::ffi::c_void)
    }));
    match result {
        Ok(Ok(backend)) => backend,
        Ok(Err(message)) => {
            write_error(error, &message);
            core::ptr::null_mut()
        }
        Err(payload) => {
            write_error(error, &panic_message(payload));
            core::ptr::null_mut()
        }
    }
}

unsafe extern "C" fn backend_destroy(backend: *mut core::ffi::c_void) {
    if backend.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `backend_create` is the sole producer, and the host calls
        // destroy exactly once after it has stopped issuing backend calls.
        drop(unsafe { Box::from_raw(backend.cast::<PluginBackend>()) });
    }));
}

unsafe extern "C" fn backend_command(
    backend: *mut core::ffi::c_void,
    command: *const abi::BackendCommand,
    error: *mut abi::BackendError,
) -> u32 {
    clear_error(error);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let backend = unsafe { ptr_mut(backend.cast::<PluginBackend>(), "video backend") }?;
        let command = decode_command(unsafe { ptr_ref(command, "video command") }?)?;
        backend
            .decoder
            .command(command)
            .map_err(|error| error.to_string())
    }));
    status_from_result(result, error)
}

unsafe extern "C" fn backend_poll_event(
    backend: *mut core::ffi::c_void,
    event: *mut abi::BackendEvent,
    error: *mut abi::BackendError,
) -> u32 {
    clear_error(error);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let backend = unsafe { ptr_mut(backend.cast::<PluginBackend>(), "video backend") }?;
        let event_out = unsafe { ptr_mut(event, "video event output") }?;
        if backend.pending.is_empty() {
            backend.pending.extend(backend.decoder.drain_events());
        }
        let Some(event) = backend.pending.pop_front() else {
            *event_out = abi::BackendEvent::default();
            return Ok::<u32, String>(abi::POLL_EMPTY);
        };
        *event_out = encode_event(event)?;
        Ok::<u32, String>(abi::POLL_EVENT)
    }));
    match result {
        Ok(Ok(status)) => status,
        Ok(Err(message)) => {
            write_error(error, &message);
            abi::POLL_ERROR
        }
        Err(payload) => {
            write_error(error, &panic_message(payload));
            abi::POLL_ERROR
        }
    }
}

unsafe extern "C" fn backend_copy_frame(
    frame: *mut core::ffi::c_void,
    destination: *mut u8,
    destination_len: usize,
    error: *mut abi::BackendError,
) -> u32 {
    clear_error(error);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let frame = unsafe { ptr_ref(frame.cast::<PluginFrame>(), "video frame") }?;
        let LinuxFrameStorage::CpuPacked(surface) = &frame.0.lease.storage else {
            return Err("DMA-BUF frame cannot be copied as packed CPU data".to_owned());
        };
        if destination_len < surface.bytes.len() {
            return Err(format!(
                "video frame destination has {destination_len} bytes, requires {}",
                surface.bytes.len()
            ));
        }
        if !surface.bytes.is_empty() && destination.is_null() {
            return Err("video frame destination is null".to_owned());
        }
        // SAFETY: the host promises a writable buffer of `destination_len`;
        // the length check above proves the source fits.
        unsafe {
            core::ptr::copy_nonoverlapping(surface.bytes.as_ptr(), destination, surface.bytes.len())
        };
        Ok(())
    }));
    status_from_result(result, error)
}

unsafe extern "C" fn backend_duplicate_frame_fd(
    frame: *mut core::ffi::c_void,
    plane: u32,
    error: *mut abi::BackendError,
) -> i32 {
    clear_error(error);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let frame = unsafe { ptr_ref(frame.cast::<PluginFrame>(), "video frame") }?;
        let LinuxFrameStorage::DmaBuf(surface) = &frame.0.lease.storage else {
            return Err("packed CPU frame has no DMA-BUF descriptor".to_owned());
        };
        let plane = usize::try_from(plane).map_err(|_| "invalid DMA-BUF plane index")?;
        let plane = surface
            .planes
            .get(plane)
            .ok_or_else(|| "DMA-BUF plane index is out of range".to_owned())?;
        let duplicate = unsafe { libc::dup(plane.fd.as_raw_fd()) };
        if duplicate < 0 {
            Err(format!(
                "failed to duplicate DMA-BUF descriptor: {}",
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(duplicate)
        }
    }));
    match result {
        Ok(Ok(fd)) => fd,
        Ok(Err(message)) => {
            write_error(error, &message);
            -1
        }
        Err(payload) => {
            write_error(error, &panic_message(payload));
            -1
        }
    }
}

unsafe extern "C" fn backend_release_frame(frame: *mut core::ffi::c_void) {
    if frame.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: every frame event transfers exactly one handle to the host,
        // which returns it through this operation exactly once.
        drop(unsafe { Box::from_raw(frame.cast::<PluginFrame>()) });
    }));
}

fn decode_transfer_policy(value: u32) -> Result<FrameTransferPolicy, String> {
    match value {
        abi::TRANSFER_REQUIRE_DIRECT => Ok(FrameTransferPolicy::RequireDirectSurface),
        abi::TRANSFER_ALLOW_GPU_COPY => Ok(FrameTransferPolicy::AllowGpuInteropCopy),
        abi::TRANSFER_ALLOW_CPU => Ok(FrameTransferPolicy::AllowCpuUpload),
        value => Err(format!("unknown video transfer policy {value}")),
    }
}

fn decode_drm_device(
    options: &abi::BackendCreateOptions,
) -> Result<Option<LinuxDrmDevice>, String> {
    match (options.renderer_drm_major, options.renderer_drm_minor) {
        (-1, -1) => Ok(None),
        (major, minor) if major >= 0 && minor >= 0 => {
            let major = u32::try_from(major).map_err(|_| "invalid renderer DRM major number")?;
            let minor = u32::try_from(minor).map_err(|_| "invalid renderer DRM minor number")?;
            Ok(Some(LinuxDrmDevice::from_device_numbers(major, minor)))
        }
        (major, minor) => Err(format!(
            "invalid renderer DRM device numbers ({major}, {minor})"
        )),
    }
}

fn decode_command(command: &abi::BackendCommand) -> Result<VideoCommand, String> {
    let id = VideoId::new(command.id);
    match command.kind {
        abi::COMMAND_OPEN => Ok(VideoCommand::Open {
            id,
            source: decode_source(command)?,
            initial_playback: match command.initial_playback {
                abi::INITIAL_PAUSED => InitialPlayback::Paused,
                abi::INITIAL_PLAYING => InitialPlayback::Playing,
                value => return Err(format!("unknown initial playback state {value}")),
            },
            loop_mode: decode_loop_mode(command.loop_kind, command.loop_count)?,
        }),
        abi::COMMAND_PLAY => Ok(VideoCommand::Playback {
            id,
            action: PlaybackAction::Play,
        }),
        abi::COMMAND_PAUSE => Ok(VideoCommand::Playback {
            id,
            action: PlaybackAction::Pause,
        }),
        abi::COMMAND_STOP => Ok(VideoCommand::Playback {
            id,
            action: PlaybackAction::Stop,
        }),
        abi::COMMAND_SEEK => Ok(VideoCommand::Playback {
            id,
            action: PlaybackAction::Seek(MediaTime::from_nanos(command.media_time_ns)),
        }),
        abi::COMMAND_SET_RATE => Ok(VideoCommand::Playback {
            id,
            action: PlaybackAction::SetRate(
                PlaybackRate::new(command.playback_rate).map_err(|error| error.to_string())?,
            ),
        }),
        abi::COMMAND_SET_LOOP => Ok(VideoCommand::Playback {
            id,
            action: PlaybackAction::SetLoop(decode_loop_mode(
                command.loop_kind,
                command.loop_count,
            )?),
        }),
        abi::COMMAND_SET_PRESENTATION => Ok(VideoCommand::Presentation {
            id,
            visibility: match command.presentation {
                abi::PRESENTATION_HIDDEN => PresentationVisibility::Hidden,
                abi::PRESENTATION_PRESENTED => PresentationVisibility::Presented,
                value => return Err(format!("unknown video presentation state {value}")),
            },
        }),
        abi::COMMAND_CLOSE => Ok(VideoCommand::Close { id }),
        kind => Err(format!("unknown video command kind {kind}")),
    }
}

fn decode_source(command: &abi::BackendCommand) -> Result<VideoSource, String> {
    if command.source_len != 0 && command.source_ptr.is_null() {
        return Err("video source pointer is null".to_owned());
    }
    // SAFETY: the host retains this storage for the duration of the command
    // call; a zero-length slice never dereferences its pointer.
    let bytes = if command.source_len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(command.source_ptr, command.source_len) }
    };
    match command.source_kind {
        abi::SOURCE_FILE => Ok(VideoSource::File(PathBuf::from(OsString::from_vec(
            bytes.to_vec(),
        )))),
        abi::SOURCE_URI => String::from_utf8(bytes.to_vec())
            .map(VideoSource::Uri)
            .map_err(|_| "video URI is not valid UTF-8".to_owned()),
        kind => Err(format!("unknown video source kind {kind}")),
    }
}

fn decode_loop_mode(kind: u32, count: u32) -> Result<LoopMode, String> {
    match kind {
        abi::LOOP_OFF => Ok(LoopMode::Off),
        abi::LOOP_INFINITE => Ok(LoopMode::Infinite),
        abi::LOOP_COUNT => NonZeroU32::new(count)
            .map(LoopMode::Count)
            .ok_or_else(|| "finite video loop count must be non-zero".to_owned()),
        kind => Err(format!("unknown video loop mode {kind}")),
    }
}

fn encode_event(event: DecoderEvent<LinuxFrameLease>) -> Result<abi::BackendEvent, String> {
    let mut encoded = abi::BackendEvent::default();
    match event {
        DecoderEvent::Opened {
            id,
            width,
            height,
            initial_state,
        } => {
            encoded.kind = abi::EVENT_OPENED;
            encoded.id = id.get();
            encoded.width = width;
            encoded.height = height;
            encoded.state = encode_state(initial_state);
        }
        DecoderEvent::Frame { id, frame } => {
            encoded.kind = abi::EVENT_FRAME;
            encoded.id = id.get();
            encoded.frame_info = encode_frame_info(&frame)?;
            encoded.frame = Box::into_raw(Box::new(PluginFrame(frame))).cast();
        }
        DecoderEvent::FramesReplaced { id, count } => {
            encoded.kind = abi::EVENT_FRAMES_REPLACED;
            encoded.id = id.get();
            encoded.count = count;
        }
        DecoderEvent::StateChanged { id, state } => {
            encoded.kind = abi::EVENT_STATE_CHANGED;
            encoded.id = id.get();
            encoded.state = encode_state(state);
        }
        DecoderEvent::Looped { id, remaining } => {
            encoded.kind = abi::EVENT_LOOPED;
            encoded.id = id.get();
            (encoded.loop_kind, encoded.loop_count) = encode_loop_mode(remaining);
        }
        DecoderEvent::Ended { id } => {
            encoded.kind = abi::EVENT_ENDED;
            encoded.id = id.get();
        }
        DecoderEvent::Failed { id, error } => {
            encoded.kind = abi::EVENT_FAILED;
            encoded.id = id.get();
            encoded.error.write(&error.to_string());
        }
    }
    Ok(encoded)
}

fn encode_frame_info(
    frame: &DecodedFrame<LinuxFrameLease>,
) -> Result<abi::BackendFrameInfo, String> {
    let mut info = abi::BackendFrameInfo {
        sampling: match frame.sampling {
            VideoSampling::Rgba8 => abi::SAMPLING_RGBA8,
            VideoSampling::Bgra8 => abi::SAMPLING_BGRA8,
        },
        transfer_path: match frame.lease.transfer_path {
            VideoTransferPath::DirectExternalSurface => abi::TRANSFER_DIRECT_EXTERNAL,
            VideoTransferPath::GpuInteropCopy => abi::TRANSFER_GPU_INTEROP_COPY,
            VideoTransferPath::CpuUpload => abi::TRANSFER_CPU_UPLOAD,
        },
        pts_ns: frame.timing.pts.as_nanos(),
        duration_ns: frame.timing.duration.as_nanos(),
        epoch: frame.timing.epoch.get(),
        coded_width: frame.geometry.coded_width,
        coded_height: frame.geometry.coded_height,
        visible_x: frame.geometry.visible_rect.x,
        visible_y: frame.geometry.visible_rect.y,
        visible_width: frame.geometry.visible_rect.width,
        visible_height: frame.geometry.visible_rect.height,
        display_width: frame.geometry.display_width,
        display_height: frame.geometry.display_height,
        pixel_aspect_numerator: frame.geometry.pixel_aspect_ratio.numerator.get(),
        pixel_aspect_denominator: frame.geometry.pixel_aspect_ratio.denominator.get(),
        rotation: match frame.geometry.rotation {
            VideoRotation::None => abi::ROTATION_NONE,
            VideoRotation::Clockwise90 => abi::ROTATION_90,
            VideoRotation::Clockwise180 => abi::ROTATION_180,
            VideoRotation::Clockwise270 => abi::ROTATION_270,
        },
        ..abi::BackendFrameInfo::default()
    };
    match &frame.lease.storage {
        LinuxFrameStorage::CpuPacked(surface) => {
            info.storage = abi::STORAGE_CPU_PACKED;
            info.cpu_len = surface.bytes.len();
            info.stride = surface.stride;
        }
        LinuxFrameStorage::DmaBuf(surface) => {
            if surface.planes.is_empty() || surface.planes.len() > abi::MAX_DMABUF_PLANES {
                return Err(format!(
                    "video frame has {} DMA-BUF planes; expected 1..={}",
                    surface.planes.len(),
                    abi::MAX_DMABUF_PLANES
                ));
            }
            info.storage = abi::STORAGE_DMABUF;
            info.plane_count = surface.planes.len() as u32;
            info.fourcc = surface.fourcc;
            info.modifier = surface.modifier;
            for (index, plane) in surface.planes.iter().enumerate() {
                info.plane_strides[index] = plane.stride;
                info.plane_offsets[index] = plane.offset;
            }
        }
    }
    Ok(info)
}

const fn encode_state(state: VideoSessionState) -> u32 {
    match state {
        VideoSessionState::Opening => abi::STATE_LOADING,
        VideoSessionState::Playing => abi::STATE_PLAYING,
        VideoSessionState::Paused => abi::STATE_PAUSED,
        VideoSessionState::Ended => abi::STATE_ENDED,
        VideoSessionState::Failed => abi::STATE_FAILED,
        VideoSessionState::Closed => abi::STATE_CLOSED,
    }
}

fn encode_loop_mode(mode: LoopMode) -> (u32, u32) {
    match mode {
        LoopMode::Off => (abi::LOOP_OFF, 0),
        LoopMode::Infinite => (abi::LOOP_INFINITE, 0),
        LoopMode::Count(count) => (abi::LOOP_COUNT, count.get()),
    }
}

fn status_from_result(
    result: Result<Result<(), String>, Box<dyn std::any::Any + Send>>,
    error: *mut abi::BackendError,
) -> u32 {
    match result {
        Ok(Ok(())) => abi::STATUS_OK,
        Ok(Err(message)) => {
            write_error(error, &message);
            abi::STATUS_ERROR
        }
        Err(payload) => {
            write_error(error, &panic_message(payload));
            abi::STATUS_ERROR
        }
    }
}

fn clear_error(error: *mut abi::BackendError) {
    // SAFETY: the ABI permits a null error sink; otherwise the host provides
    // writable storage for one `BackendError`.
    if let Some(error) = unsafe { error.as_mut() } {
        error.clear();
    }
}

fn write_error(error: *mut abi::BackendError, message: &str) {
    // SAFETY: the ABI permits a null error sink; otherwise the host provides
    // writable storage for one `BackendError`.
    if let Some(error) = unsafe { error.as_mut() } {
        error.write(message);
    }
}

unsafe fn ptr_ref<'a, T>(pointer: *const T, description: &str) -> Result<&'a T, String> {
    // SAFETY: callers use only pointers received through the ABI for the
    // duration of the corresponding call.
    unsafe { pointer.as_ref() }.ok_or_else(|| format!("{description} is null"))
}

unsafe fn ptr_mut<'a, T>(pointer: *mut T, description: &str) -> Result<&'a mut T, String> {
    // SAFETY: callers use only uniquely borrowed output/instance pointers
    // received through the ABI for the duration of the corresponding call.
    unsafe { pointer.as_mut() }.ok_or_else(|| format!("{description} is null"))
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        format!("video backend panicked: {message}")
    } else if let Some(message) = payload.downcast_ref::<String>() {
        format!("video backend panicked: {message}")
    } else {
        "video backend panicked".to_owned()
    }
}
