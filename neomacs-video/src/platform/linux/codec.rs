use std::ffi::c_void;
use std::num::NonZeroU32;
use std::os::unix::ffi::OsStrExt;
use std::ptr::NonNull;
use std::sync::Arc;

use neomacs_display_protocol::types::VideoId;
use neomacs_video_backend_abi as abi;

use crate::backend::{BackendEvent, DecodedFrame, DecodedFrameTransfer};
use crate::sampling::LinuxDrmDevice;
use crate::{
    BiPlanarVideoFormat, FrameTiming, FrameTransferPolicy, InitialPlayback, LoopMode, MediaTime,
    PackedVideoFormat, PixelAspectRatio, PixelRect, PlaybackAction, PlaybackEpoch,
    PresentationVisibility, VideoChromaLocation, VideoColorPrimaries, VideoColorRange,
    VideoColorimetry, VideoCommand, VideoCommandError, VideoFrameFormat, VideoGeometry,
    VideoMatrixCoefficients, VideoRotation, VideoSessionState, VideoSource,
    VideoTransferCharacteristic, VideoTransferPath,
};

use super::frame::{
    CpuPackedSurface, DmaBufObject, DmaBufPlane, DmaBufSurface, LinuxFrameLease, LinuxFrameStorage,
    PluginFrameLease,
};
use super::loader::{LoadedBackend, backend_message};

pub(super) fn encode_renderer_drm_device(
    device: Option<LinuxDrmDevice>,
) -> Result<(i32, i32), String> {
    let Some(device) = device else {
        return Ok((-1, -1));
    };
    let (major, minor) = device.device_numbers();
    let major = i32::try_from(major)
        .map_err(|_| format!("renderer DRM major number {major} exceeds the backend ABI"))?;
    let minor = i32::try_from(minor)
        .map_err(|_| format!("renderer DRM minor number {minor} exceeds the backend ABI"))?;
    Ok((major, minor))
}

pub(super) fn encode_transfer_policy(policy: FrameTransferPolicy) -> u32 {
    match policy {
        FrameTransferPolicy::RequireDirectSurface => abi::TRANSFER_REQUIRE_DIRECT,
        FrameTransferPolicy::AllowGpuInteropCopy => abi::TRANSFER_ALLOW_GPU_COPY,
        FrameTransferPolicy::AllowCpuUpload => abi::TRANSFER_ALLOW_CPU,
    }
}

pub(super) fn encode_supported_formats(features: wgpu::Features) -> u32 {
    let mut formats = 0;
    if features.contains(wgpu::Features::TEXTURE_FORMAT_NV12) {
        formats |= abi::FORMAT_SUPPORT_NV12;
    }
    if features.contains(wgpu::Features::TEXTURE_FORMAT_P010) {
        formats |= abi::FORMAT_SUPPORT_P010;
    }
    formats
}

pub(super) fn encode_command(command: &VideoCommand) -> (abi::BackendCommand, Vec<u8>) {
    let mut encoded = abi::BackendCommand::default();
    let source = match command {
        VideoCommand::Open {
            id,
            source,
            initial_playback,
            loop_mode,
        } => {
            encoded.kind = abi::COMMAND_OPEN;
            encoded.id = id.get();
            encoded.initial_playback = match initial_playback {
                InitialPlayback::Paused => abi::INITIAL_PAUSED,
                InitialPlayback::Playing => abi::INITIAL_PLAYING,
            };
            (encoded.loop_kind, encoded.loop_count) = encode_loop_mode(*loop_mode);
            match source {
                VideoSource::File(path) => {
                    encoded.source_kind = abi::SOURCE_FILE;
                    path.as_os_str().as_bytes().to_vec()
                }
                VideoSource::Uri(uri) => {
                    encoded.source_kind = abi::SOURCE_URI;
                    uri.as_bytes().to_vec()
                }
            }
        }
        VideoCommand::Playback { id, action } => {
            encoded.id = id.get();
            match action {
                PlaybackAction::Play => encoded.kind = abi::COMMAND_PLAY,
                PlaybackAction::Pause => encoded.kind = abi::COMMAND_PAUSE,
                PlaybackAction::Stop => encoded.kind = abi::COMMAND_STOP,
                PlaybackAction::Seek(position) => {
                    encoded.kind = abi::COMMAND_SEEK;
                    encoded.media_time_ns = position.as_nanos();
                }
                PlaybackAction::SetRate(rate) => {
                    encoded.kind = abi::COMMAND_SET_RATE;
                    encoded.playback_rate = rate.get();
                }
                PlaybackAction::SetLoop(loop_mode) => {
                    encoded.kind = abi::COMMAND_SET_LOOP;
                    (encoded.loop_kind, encoded.loop_count) = encode_loop_mode(*loop_mode);
                }
            }
            Vec::new()
        }
        VideoCommand::Presentation { id, visibility } => {
            encoded.kind = abi::COMMAND_SET_PRESENTATION;
            encoded.id = id.get();
            encoded.presentation = match visibility {
                PresentationVisibility::Hidden => abi::PRESENTATION_HIDDEN,
                PresentationVisibility::Presented => abi::PRESENTATION_PRESENTED,
            };
            Vec::new()
        }
        VideoCommand::Close { id } => {
            encoded.kind = abi::COMMAND_CLOSE;
            encoded.id = id.get();
            Vec::new()
        }
    };
    encoded.source_ptr = source.as_ptr();
    encoded.source_len = source.len();
    (encoded, source)
}

fn encode_loop_mode(mode: LoopMode) -> (u32, u32) {
    match mode {
        LoopMode::Off => (abi::LOOP_OFF, 0),
        LoopMode::Infinite => (abi::LOOP_INFINITE, 0),
        LoopMode::Count(count) => (abi::LOOP_COUNT, count.get()),
    }
}

pub(super) fn decode_event(
    backend: Arc<LoadedBackend>,
    event: abi::BackendEvent,
) -> Result<BackendEvent<LinuxFrameLease>, String> {
    let id = VideoId::new(event.id);
    if event.kind != abi::EVENT_FRAME && !event.frame.is_null() {
        if let Some(frame) = NonNull::new(event.frame) {
            backend.release_frame(frame);
        }
        return Err("non-frame video event carried an opaque frame handle".to_owned());
    }
    match event.kind {
        abi::EVENT_OPENED => Ok(BackendEvent::Opened {
            id,
            width: event.width,
            height: event.height,
            initial_state: decode_state(event.state)?,
        }),
        abi::EVENT_FRAME => Ok(BackendEvent::Frame {
            id,
            frame: decode_frame(backend, event.frame, event.frame_info)?,
        }),
        abi::EVENT_FRAMES_REPLACED => Ok(BackendEvent::FramesReplaced {
            id,
            count: event.count,
        }),
        abi::EVENT_STATE_CHANGED => Ok(BackendEvent::StateChanged {
            id,
            state: decode_state(event.state)?,
        }),
        abi::EVENT_LOOPED => Ok(BackendEvent::Looped {
            id,
            remaining: decode_loop_mode(event.loop_kind, event.loop_count)?,
        }),
        abi::EVENT_ENDED => Ok(BackendEvent::Ended { id }),
        abi::EVENT_FAILED => Ok(BackendEvent::Failed {
            id,
            error: VideoCommandError::Backend {
                message: backend_message(&event.error, "video backend failed"),
            },
        }),
        kind => Err(format!("video backend returned unknown event kind {kind}")),
    }
}

fn decode_frame(
    backend: Arc<LoadedBackend>,
    frame: *mut c_void,
    info: abi::BackendFrameInfo,
) -> Result<DecodedFrame<LinuxFrameLease>, String> {
    let frame =
        NonNull::new(frame).ok_or_else(|| "frame event carried a null handle".to_owned())?;
    let plugin_frame = PluginFrameLease::new(backend, frame);
    let format = match info.format {
        abi::FORMAT_RGBA8 => VideoFrameFormat::Packed(PackedVideoFormat::Rgba8),
        abi::FORMAT_BGRA8 => VideoFrameFormat::Packed(PackedVideoFormat::Bgra8),
        abi::FORMAT_NV12 => VideoFrameFormat::BiPlanar420(BiPlanarVideoFormat::Nv12),
        abi::FORMAT_P010 => VideoFrameFormat::BiPlanar420(BiPlanarVideoFormat::P010),
        format => {
            return Err(format!(
                "video backend returned unknown frame format {format}"
            ));
        }
    };
    let colorimetry = decode_colorimetry(info)?;
    let transfer_path = match info.transfer_path {
        abi::TRANSFER_DIRECT_EXTERNAL => VideoTransferPath::DirectExternalSurface,
        abi::TRANSFER_GPU_INTEROP_COPY => VideoTransferPath::GpuInteropCopy,
        abi::TRANSFER_CPU_UPLOAD => VideoTransferPath::CpuUpload,
        path => {
            return Err(format!(
                "video backend returned unknown transfer path {path}"
            ));
        }
    };
    let geometry = decode_geometry(info)?;
    format
        .allocation_bytes(geometry)
        .map_err(|error| error.to_string())?;
    let epoch = PlaybackEpoch::from_raw(info.epoch)
        .ok_or_else(|| "video backend returned a zero playback epoch".to_owned())?;
    let storage = match info.storage {
        abi::STORAGE_CPU_PACKED => {
            if !matches!(format, VideoFrameFormat::Packed(_)) {
                return Err("CPU frame declared a bi-planar format".to_owned());
            }
            if transfer_path != VideoTransferPath::CpuUpload {
                return Err("packed CPU frame declared a non-CPU transfer path".to_owned());
            }
            let minimum_len = usize::try_from(info.stride)
                .ok()
                .and_then(|stride| stride.checked_mul(info.coded_height as usize))
                .ok_or_else(|| "packed video frame size overflow".to_owned())?;
            if info.cpu_len < minimum_len {
                return Err(format!(
                    "packed video frame has {} bytes, requires at least {minimum_len}",
                    info.cpu_len
                ));
            }
            let mut bytes = vec![0; info.cpu_len];
            plugin_frame.copy_to(&mut bytes)?;
            LinuxFrameStorage::CpuPacked(CpuPackedSurface {
                bytes,
                stride: info.stride,
            })
        }
        abi::STORAGE_DMABUF => {
            if transfer_path == VideoTransferPath::CpuUpload {
                return Err("DMA-BUF frame declared a CPU transfer path".to_owned());
            }
            if info.synchronization != abi::SYNCHRONIZATION_IMPLICIT {
                return Err(format!(
                    "video backend returned unsupported DMA-BUF synchronization {}",
                    info.synchronization
                ));
            }
            let object_count = usize::try_from(info.object_count)
                .map_err(|_| "invalid DMA-BUF object count".to_owned())?;
            if !(1..=abi::MAX_DMABUF_PLANES).contains(&object_count) {
                return Err(format!(
                    "video backend returned {object_count} DMA-BUF objects; expected 1..={}",
                    abi::MAX_DMABUF_PLANES
                ));
            }
            let plane_count = usize::try_from(info.plane_count)
                .map_err(|_| "invalid DMA-BUF plane count".to_owned())?;
            if !(1..=abi::MAX_DMABUF_PLANES).contains(&plane_count) {
                return Err(format!(
                    "video backend returned {plane_count} DMA-BUF planes; expected 1..={}",
                    abi::MAX_DMABUF_PLANES
                ));
            }
            let mut objects = Vec::with_capacity(object_count);
            for index in 0..object_count {
                objects.push(DmaBufObject {
                    fd: plugin_frame.duplicate_object_fd(index as u32)?,
                    modifier: info.object_modifiers[index],
                });
            }
            let mut planes = Vec::with_capacity(plane_count);
            for index in 0..plane_count {
                let object_index = usize::try_from(info.plane_object_indices[index])
                    .map_err(|_| format!("invalid object index for DMA-BUF plane {index}"))?;
                if object_index >= object_count {
                    return Err(format!(
                        "DMA-BUF plane {index} refers to missing object {object_index}"
                    ));
                }
                planes.push(DmaBufPlane {
                    object_index,
                    stride: info.plane_strides[index],
                    offset: info.plane_offsets[index],
                });
            }
            LinuxFrameStorage::DmaBuf(DmaBufSurface {
                objects,
                planes,
                fourcc: info.fourcc,
            })
        }
        storage => {
            return Err(format!(
                "video backend returned unknown frame storage {storage}"
            ));
        }
    };
    Ok(DecodedFrame {
        lease: LinuxFrameLease {
            _plugin_frame: plugin_frame,
            storage,
            transfer_path,
        },
        timing: FrameTiming {
            pts: MediaTime::from_nanos(info.pts_ns),
            duration: MediaTime::from_nanos(info.duration_ns),
            epoch,
        },
        geometry,
        format,
        colorimetry,
        decoder_transfer: DecodedFrameTransfer::Deferred,
    })
}

fn decode_colorimetry(info: abi::BackendFrameInfo) -> Result<VideoColorimetry, String> {
    let primaries = match info.color_primaries {
        abi::COLOR_PRIMARIES_BT601_525 => VideoColorPrimaries::Bt601_525,
        abi::COLOR_PRIMARIES_BT601_625 => VideoColorPrimaries::Bt601_625,
        abi::COLOR_PRIMARIES_BT709 => VideoColorPrimaries::Bt709,
        abi::COLOR_PRIMARIES_BT2020 => VideoColorPrimaries::Bt2020,
        value => {
            return Err(format!(
                "video backend returned unknown color primaries {value}"
            ));
        }
    };
    let transfer = match info.color_transfer {
        abi::COLOR_TRANSFER_SRGB => VideoTransferCharacteristic::Srgb,
        abi::COLOR_TRANSFER_BT709 => VideoTransferCharacteristic::Bt709,
        abi::COLOR_TRANSFER_PQ => VideoTransferCharacteristic::Pq,
        abi::COLOR_TRANSFER_HLG => VideoTransferCharacteristic::Hlg,
        value => {
            return Err(format!(
                "video backend returned unknown color transfer {value}"
            ));
        }
    };
    let matrix = match info.color_matrix {
        abi::COLOR_MATRIX_IDENTITY => VideoMatrixCoefficients::Identity,
        abi::COLOR_MATRIX_BT601 => VideoMatrixCoefficients::Bt601,
        abi::COLOR_MATRIX_BT709 => VideoMatrixCoefficients::Bt709,
        abi::COLOR_MATRIX_BT2020_NCL => VideoMatrixCoefficients::Bt2020NonConstantLuminance,
        value => {
            return Err(format!(
                "video backend returned unknown color matrix {value}"
            ));
        }
    };
    let range = match info.color_range {
        abi::COLOR_RANGE_LIMITED => VideoColorRange::Limited,
        abi::COLOR_RANGE_FULL => VideoColorRange::Full,
        value => {
            return Err(format!(
                "video backend returned unknown color range {value}"
            ));
        }
    };
    let chroma_location = match info.chroma_location {
        abi::CHROMA_LOCATION_LEFT => VideoChromaLocation::Left,
        abi::CHROMA_LOCATION_CENTER => VideoChromaLocation::Center,
        abi::CHROMA_LOCATION_TOP_LEFT => VideoChromaLocation::TopLeft,
        value => {
            return Err(format!(
                "video backend returned unknown chroma location {value}"
            ));
        }
    };
    Ok(VideoColorimetry {
        primaries,
        transfer,
        matrix,
        range,
        chroma_location,
    })
}

fn decode_geometry(info: abi::BackendFrameInfo) -> Result<VideoGeometry, String> {
    let numerator = NonZeroU32::new(info.pixel_aspect_numerator)
        .ok_or_else(|| "video backend returned a zero pixel-aspect numerator".to_owned())?;
    let denominator = NonZeroU32::new(info.pixel_aspect_denominator)
        .ok_or_else(|| "video backend returned a zero pixel-aspect denominator".to_owned())?;
    let visible_right = info
        .visible_x
        .checked_add(info.visible_width)
        .ok_or_else(|| "video visible rectangle overflow".to_owned())?;
    let visible_bottom = info
        .visible_y
        .checked_add(info.visible_height)
        .ok_or_else(|| "video visible rectangle overflow".to_owned())?;
    if info.coded_width == 0
        || info.coded_height == 0
        || info.visible_width == 0
        || info.visible_height == 0
        || info.display_width == 0
        || info.display_height == 0
        || visible_right > info.coded_width
        || visible_bottom > info.coded_height
    {
        return Err("video backend returned invalid frame geometry".to_owned());
    }
    let rotation = match info.rotation {
        abi::ROTATION_NONE => VideoRotation::None,
        abi::ROTATION_90 => VideoRotation::Clockwise90,
        abi::ROTATION_180 => VideoRotation::Clockwise180,
        abi::ROTATION_270 => VideoRotation::Clockwise270,
        rotation => {
            return Err(format!(
                "video backend returned unknown rotation {rotation}"
            ));
        }
    };
    Ok(VideoGeometry {
        coded_width: info.coded_width,
        coded_height: info.coded_height,
        visible_rect: PixelRect {
            x: info.visible_x,
            y: info.visible_y,
            width: info.visible_width,
            height: info.visible_height,
        },
        display_width: info.display_width,
        display_height: info.display_height,
        pixel_aspect_ratio: PixelAspectRatio {
            numerator,
            denominator,
        },
        rotation,
    })
}

fn decode_state(state: u32) -> Result<VideoSessionState, String> {
    match state {
        abi::STATE_LOADING => Ok(VideoSessionState::Opening),
        abi::STATE_PLAYING => Ok(VideoSessionState::Playing),
        abi::STATE_PAUSED => Ok(VideoSessionState::Paused),
        abi::STATE_ENDED => Ok(VideoSessionState::Ended),
        abi::STATE_FAILED => Ok(VideoSessionState::Failed),
        abi::STATE_CLOSED => Ok(VideoSessionState::Closed),
        state => Err(format!(
            "video backend returned unknown session state {state}"
        )),
    }
}

fn decode_loop_mode(kind: u32, count: u32) -> Result<LoopMode, String> {
    match kind {
        abi::LOOP_OFF => Ok(LoopMode::Off),
        abi::LOOP_INFINITE => Ok(LoopMode::Infinite),
        abi::LOOP_COUNT => NonZeroU32::new(count)
            .map(LoopMode::Count)
            .ok_or_else(|| "video backend returned a zero finite loop count".to_owned()),
        kind => Err(format!("video backend returned unknown loop mode {kind}")),
    }
}

#[cfg(test)]
#[path = "codec_test.rs"]
mod tests;
