//! AVFoundation playback and zero-copy CoreVideo -> Metal replay.

use std::collections::HashMap;
use std::ptr::NonNull;
use std::time::{Duration, Instant};

use neomacs_display_protocol::types::VideoId;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker};
use objc2_av_foundation::{
    AVMediaTypeVideo, AVPlayer, AVPlayerItem, AVPlayerItemStatus, AVPlayerItemVideoOutput,
    AVVideoColorPrimaries_ITU_R_709_2, AVVideoColorPrimariesKey, AVVideoColorPropertiesKey,
    AVVideoTransferFunction_IEC_sRGB, AVVideoTransferFunctionKey,
};
use objc2_core_foundation::{CFRetained, CFString};
use objc2_core_media::CMTime;
use objc2_core_video::{
    CVImageBufferGetCleanRect, CVImageBufferGetDisplaySize, CVMetalTexture, CVMetalTextureCache,
    CVMetalTextureGetTexture, CVPixelBuffer, CVPixelBufferGetHeight, CVPixelBufferGetWidth,
    kCVPixelBufferMetalCompatibilityKey, kCVPixelBufferPixelFormatTypeKey,
    kCVPixelFormatType_32BGRA,
};
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};
use objc2_metal::{MTLPixelFormat, MTLTextureType};

use crate::backend::{
    BackendEvent, DecodedFrame, DecoderBackend, FrameImportOutcome, FrameImporter, ImportedFrame,
    Platform, ProductionPlatform,
};
use crate::sampling::{GpuVideoContext, PreparedSampledTexture};
use crate::surface_pool::{BoundedSurfacePool, SurfacePoolAcquire};
use crate::{
    FrameTiming, GpuVideoFrame, InitialPlayback, LoopMode, MediaTime, PackedVideoFormat,
    PlaybackAction, PlaybackEpoch, VideoColorimetry, VideoCommand, VideoDecodeBackend,
    VideoFrameFormat, VideoGeometry, VideoInitError, VideoSessionState, VideoSource,
    VideoTransferPath, VideoWake,
};

pub(crate) struct MacPlatform;

/// Pull AVPlayerItemVideoOutput through the common scheduler often enough for
/// 120-Hz media without installing a second display clock. Once a frame is
/// published, its PTS becomes the more precise compositor deadline.
const MAC_MEDIA_POLL_INTERVAL: Duration = Duration::from_micros(8_000);
const MAX_IN_FLIGHT_MAC_VIDEO_SURFACES: usize = 8;

/// Affine ownership of one CoreVideo decoder surface.
pub(crate) struct MacFrame {
    pixel_buffer: Retained<CVPixelBuffer>,
}

// CVPixelBuffer is explicitly designed for cross-queue video pipelines. This
// private lease states that guarantee at the native boundary; Objective-C
// player objects remain main-thread-confined.
unsafe impl Send for MacFrame {}
unsafe impl Sync for MacFrame {}

struct MacSession {
    player: Retained<AVPlayer>,
    item: Retained<AVPlayerItem>,
    output: Retained<AVPlayerItemVideoOutput>,
    state: VideoSessionState,
    loop_mode: LoopMode,
    playback_rate: f32,
    announced: bool,
    presented: bool,
    awaiting_frame: bool,
    ended: bool,
    epoch: PlaybackEpoch,
    rotation: Option<crate::VideoRotation>,
}

impl MacSession {
    fn needs_media_poll(&self) -> bool {
        self.presented
            && (matches!(
                self.state,
                VideoSessionState::Opening | VideoSessionState::Playing
            ) || self.awaiting_frame)
    }
}

pub(crate) struct MacDecoder {
    sessions: HashMap<VideoId, MacSession>,
    pending: Vec<BackendEvent<MacFrame>>,
}

impl MacDecoder {
    fn new(_wake: VideoWake) -> Result<Self, String> {
        MainThreadMarker::new().ok_or_else(|| {
            "AVFoundation video must initialize on the macOS main thread".to_string()
        })?;
        Ok(Self {
            sessions: HashMap::new(),
            pending: Vec::new(),
        })
    }

    fn open(
        &mut self,
        id: VideoId,
        source: VideoSource,
        initial: InitialPlayback,
        loop_mode: LoopMode,
    ) -> Result<(), String> {
        if self.sessions.contains_key(&id) {
            return Err(format!("video {} is already open", id.get()));
        }
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            "AVFoundation video commands must run on the macOS main thread".to_string()
        })?;
        let url = source_url(source)?;
        let output_settings = bgra_srgb_output_settings()?;
        let output = unsafe {
            AVPlayerItemVideoOutput::initWithOutputSettings(
                AVPlayerItemVideoOutput::alloc(),
                Some(&output_settings),
            )
        };
        let item = unsafe { AVPlayerItem::playerItemWithURL(&url, mtm) };
        unsafe { item.addOutput(&output) };
        let player = unsafe { AVPlayer::playerWithPlayerItem(Some(&item), mtm) };
        // Inline Neomacs video has historically been visual-only. Keep that
        // contract consistent with Linux's fakesink until audio is modeled as
        // a separate, focus-aware subsystem.
        unsafe { player.setMuted(true) };
        let state = match initial {
            InitialPlayback::Playing => {
                unsafe { player.play() };
                VideoSessionState::Playing
            }
            InitialPlayback::Paused => {
                unsafe { player.pause() };
                VideoSessionState::Opening
            }
        };
        self.sessions.insert(
            id,
            MacSession {
                player,
                item,
                output,
                state,
                loop_mode,
                playback_rate: 1.0,
                announced: false,
                presented: true,
                awaiting_frame: true,
                ended: false,
                epoch: PlaybackEpoch::INITIAL,
                rotation: None,
            },
        );
        Ok(())
    }

    fn playback(&mut self, id: VideoId, action: PlaybackAction) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(&id)
            .ok_or_else(|| format!("video {} is not open", id.get()))?;
        match action {
            PlaybackAction::Play => {
                session.state = VideoSessionState::Playing;
                session.ended = false;
                if session.presented {
                    unsafe { session.player.playImmediatelyAtRate(session.playback_rate) };
                }
            }
            PlaybackAction::Pause => {
                session.state = VideoSessionState::Paused;
                unsafe { session.player.pause() };
            }
            PlaybackAction::Stop => {
                session.epoch = session.epoch.next();
                session.state = VideoSessionState::Paused;
                session.awaiting_frame = true;
                session.ended = false;
                unsafe {
                    session.player.pause();
                    session.player.seekToTime(CMTime::new(0, 1_000_000_000));
                }
            }
            PlaybackAction::Seek(time) => {
                session.epoch = session.epoch.next();
                session.awaiting_frame = true;
                session.ended = false;
                unsafe {
                    session
                        .player
                        .seekToTime(CMTime::new(time.as_nanos() as i64, 1_000_000_000));
                }
            }
            PlaybackAction::SetRate(rate) => {
                session.playback_rate = rate.get() as f32;
                if session.presented && session.state == VideoSessionState::Playing {
                    unsafe { session.player.setRate(session.playback_rate) };
                }
            }
            PlaybackAction::SetLoop(mode) => session.loop_mode = mode,
        }
        self.pending.push(BackendEvent::StateChanged {
            id,
            state: session.state,
        });
        Ok(())
    }

    fn poll_sessions(&mut self) {
        autoreleasepool(|_| {
            let mut events = Vec::new();
            let mut failed = Vec::new();
            for (&id, session) in &mut self.sessions {
                if unsafe { session.item.status() } == AVPlayerItemStatus::Failed {
                    let detail = unsafe { session.item.error() }
                        .map(|error| format!("{error:?}"))
                        .unwrap_or_else(|| "unknown AVFoundation error".into());
                    failed.push((id, format!("AVFoundation failed to load video: {detail}")));
                    continue;
                }

                // Presentation visibility suspends both AVPlayer and native
                // pixel-buffer pulls. Another visible session may drive this
                // global service pass; it must not wake hidden decoders.
                if !session.presented {
                    continue;
                }

                let item_time = unsafe { session.item.currentTime() };
                if unsafe { session.output.hasNewPixelBufferForItemTime(item_time) } {
                    let mut display_time = item_time;
                    if let Some(pixel_buffer) = unsafe {
                        session
                            .output
                            .copyPixelBufferForItemTime_itemTimeForDisplay(
                                item_time,
                                &mut display_time,
                            )
                    } {
                        let rotation = *session
                            .rotation
                            .get_or_insert_with(|| player_item_rotation(&session.item));
                        let geometry = geometry_from_pixel_buffer(&pixel_buffer, rotation);
                        if !session.announced {
                            let initial_state = match session.state {
                                VideoSessionState::Opening => VideoSessionState::Paused,
                                state => state,
                            };
                            session.state = initial_state;
                            session.announced = true;
                            events.push(BackendEvent::Opened {
                                id,
                                width: geometry.display_width,
                                height: geometry.display_height,
                                initial_state,
                            });
                        }
                        session.awaiting_frame = false;
                        let pts = media_time(display_time);
                        // AVPlayerItemVideoOutput exposes the current item's
                        // display PTS but not a reliable per-frame duration.
                        // Zero means unknown; the common late-drop policy
                        // must not attach the previous frame's interval to
                        // this variable-rate frame.
                        let duration = MediaTime::ZERO;
                        events.push(BackendEvent::Frame {
                            id,
                            frame: DecodedFrame {
                                lease: MacFrame { pixel_buffer },
                                timing: FrameTiming {
                                    pts,
                                    duration,
                                    epoch: session.epoch,
                                },
                                geometry,
                                format: VideoFrameFormat::Packed(PackedVideoFormat::Bgra8),
                                colorimetry: VideoColorimetry::SRGB,
                            },
                        });
                    }
                }

                let duration = unsafe { session.item.duration().seconds() };
                let current = unsafe { item_time.seconds() };
                if duration.is_finite()
                    && duration > 0.0
                    && current >= duration - 0.001
                    && !session.ended
                {
                    if session.loop_mode.consume_replay() {
                        session.epoch = session.epoch.next();
                        session.awaiting_frame = true;
                        events.push(BackendEvent::Looped {
                            id,
                            remaining: session.loop_mode,
                        });
                        unsafe {
                            session.player.seekToTime(CMTime::new(0, 1_000_000_000));
                            if session.state == VideoSessionState::Playing && session.presented {
                                session.player.playImmediatelyAtRate(session.playback_rate);
                            }
                        }
                    } else {
                        session.ended = true;
                        session.state = VideoSessionState::Ended;
                        events.push(BackendEvent::Ended { id });
                    }
                }
            }
            for (id, message) in failed {
                self.sessions.remove(&id);
                events.push(BackendEvent::Failed {
                    id,
                    error: message.into(),
                });
            }
            self.pending.extend(events);
        });
    }
}

fn geometry_from_pixel_buffer(
    pixel_buffer: &CVPixelBuffer,
    rotation: crate::VideoRotation,
) -> VideoGeometry {
    let coded_width = CVPixelBufferGetWidth(pixel_buffer) as u32;
    let coded_height = CVPixelBufferGetHeight(pixel_buffer) as u32;
    let clean = CVImageBufferGetCleanRect(pixel_buffer);
    let display = CVImageBufferGetDisplaySize(pixel_buffer);
    let clean_width = finite_rounded_dimension(clean.size.width, coded_width);
    let clean_height = finite_rounded_dimension(clean.size.height, coded_height);
    let clean_x = finite_rounded_coordinate(clean.origin.x, 0).min(coded_width);
    // CoreVideo reports the clean aperture from a lower-left origin; wgpu's
    // video texture convention is top-left.
    let lower_y = finite_rounded_coordinate(clean.origin.y, 0);
    let clean_y = coded_height.saturating_sub(lower_y.saturating_add(clean_height));
    let display_width = finite_rounded_dimension(display.width, clean_width);
    let display_height = finite_rounded_dimension(display.height, clean_height);
    let (display_width, display_height) = match rotation {
        crate::VideoRotation::Clockwise90 | crate::VideoRotation::Clockwise270 => {
            (display_height, display_width)
        }
        crate::VideoRotation::None | crate::VideoRotation::Clockwise180 => {
            (display_width, display_height)
        }
    };
    VideoGeometry::with_visible_rect_and_display_size(
        coded_width,
        coded_height,
        crate::PixelRect {
            x: clean_x,
            y: clean_y,
            width: clean_width.min(coded_width.saturating_sub(clean_x)),
            height: clean_height.min(coded_height.saturating_sub(clean_y)),
        },
        display_width,
        display_height,
        rotation,
    )
}

#[allow(deprecated)]
fn player_item_rotation(item: &AVPlayerItem) -> crate::VideoRotation {
    let Some(media_type) = (unsafe { AVMediaTypeVideo }) else {
        return crate::VideoRotation::None;
    };
    let tracks = unsafe { item.asset().tracksWithMediaType(media_type) };
    let Some(track) = tracks.firstObject() else {
        return crate::VideoRotation::None;
    };
    let transform = unsafe { track.preferredTransform() };
    let near = |value: f64, target: f64| (value - target).abs() <= 0.001;
    if near(transform.b, 1.0) && near(transform.c, -1.0) {
        crate::VideoRotation::Clockwise90
    } else if near(transform.a, -1.0) && near(transform.d, -1.0) {
        crate::VideoRotation::Clockwise180
    } else if near(transform.b, -1.0) && near(transform.c, 1.0) {
        crate::VideoRotation::Clockwise270
    } else {
        crate::VideoRotation::None
    }
}

fn finite_rounded_dimension(value: f64, fallback: u32) -> u32 {
    if value.is_finite() && value > 0.0 && value <= f64::from(u32::MAX) {
        value.round() as u32
    } else {
        fallback.max(1)
    }
}

fn finite_rounded_coordinate(value: f64, fallback: u32) -> u32 {
    if value.is_finite() && value >= 0.0 && value <= f64::from(u32::MAX) {
        value.round() as u32
    } else {
        fallback
    }
}

impl DecoderBackend for MacDecoder {
    type Frame = MacFrame;

    fn command(&mut self, command: VideoCommand) -> Result<(), crate::VideoCommandError> {
        match command {
            VideoCommand::Open {
                id,
                source,
                initial_playback,
                loop_mode,
            } => self
                .open(id, source, initial_playback, loop_mode)
                .map_err(Into::into),
            VideoCommand::Playback { id, action } => self.playback(id, action).map_err(Into::into),
            VideoCommand::Presentation { id, visibility } => {
                let session = self
                    .sessions
                    .get_mut(&id)
                    .ok_or_else(|| format!("video {} is not open", id.get()))?;
                let presented = matches!(visibility, crate::PresentationVisibility::Presented);
                if session.presented == presented {
                    return Ok(());
                }
                session.presented = presented;
                session.awaiting_frame = presented;
                unsafe {
                    if presented && session.state == VideoSessionState::Playing {
                        session.player.playImmediatelyAtRate(session.playback_rate);
                    } else {
                        session.player.pause();
                    }
                }
                Ok(())
            }
            VideoCommand::Close { id } => {
                if self.sessions.remove(&id).is_none() {
                    return Err(crate::VideoCommandError::SessionNotOpen { id: id.get() });
                }
                self.pending.push(BackendEvent::StateChanged {
                    id,
                    state: VideoSessionState::Closed,
                });
                Ok(())
            }
        }
    }

    fn drain_events(&mut self) -> Vec<BackendEvent<Self::Frame>> {
        self.poll_sessions();
        std::mem::take(&mut self.pending)
    }

    fn next_service_deadline(&self, now: Instant) -> Option<Instant> {
        self.sessions
            .values()
            .any(MacSession::needs_media_poll)
            .then_some(now + MAC_MEDIA_POLL_INTERVAL)
    }
}

fn source_url(source: VideoSource) -> Result<Retained<NSURL>, String> {
    match source {
        VideoSource::File(path) => Ok(NSURL::fileURLWithPath(&NSString::from_str(
            &path.to_string_lossy(),
        ))),
        VideoSource::Uri(uri) => NSURL::URLWithString(&NSString::from_str(&uri))
            .ok_or_else(|| format!("invalid video URI {uri:?}")),
    }
}

fn bgra_srgb_output_settings() -> Result<Retained<NSDictionary<NSString, AnyObject>>, String> {
    // CoreFoundation/NSString and CFNumber/NSNumber are toll-free bridged.
    let format_key =
        unsafe { &*(kCVPixelBufferPixelFormatTypeKey as *const CFString as *const NSString) };
    let metal_key =
        unsafe { &*(kCVPixelBufferMetalCompatibilityKey as *const CFString as *const NSString) };
    let format = NSNumber::new_u32(kCVPixelFormatType_32BGRA);
    let compatible = NSNumber::new_bool(true);
    let (color_key, primaries_key, primaries, transfer_key, transfer) = unsafe {
        (
            AVVideoColorPropertiesKey,
            AVVideoColorPrimariesKey,
            AVVideoColorPrimaries_ITU_R_709_2,
            AVVideoTransferFunctionKey,
            AVVideoTransferFunction_IEC_sRGB,
        )
    };
    let color_key = color_key.ok_or_else(|| {
        "this macOS runtime cannot request AVFoundation color properties".to_owned()
    })?;
    let primaries_key = primaries_key
        .ok_or_else(|| "this macOS runtime cannot request video color primaries".to_owned())?;
    let primaries = primaries
        .ok_or_else(|| "this macOS runtime cannot request BT.709 color primaries".to_owned())?;
    let transfer_key = transfer_key
        .ok_or_else(|| "this macOS runtime cannot request a video transfer function".to_owned())?;
    let transfer = transfer
        .ok_or_else(|| "this macOS runtime cannot request the sRGB transfer function".to_owned())?;
    let color_properties = NSDictionary::from_slices(
        &[primaries_key, transfer_key],
        &[primaries as &AnyObject, transfer as &AnyObject],
    );
    let values: [&AnyObject; 3] = [
        format.as_ref(),
        compatible.as_ref(),
        color_properties.as_ref(),
    ];
    Ok(NSDictionary::from_slices(
        &[format_key, metal_key, color_key],
        &values,
    ))
}

fn media_time(time: CMTime) -> MediaTime {
    let seconds = unsafe { time.seconds() };
    if seconds.is_finite() && seconds > 0.0 {
        MediaTime::from_nanos((seconds * 1_000_000_000.0).round() as u64)
    } else {
        MediaTime::ZERO
    }
}

pub(crate) struct MacImporter {
    gpu: GpuVideoContext,
    texture_cache: CFRetained<CVMetalTextureCache>,
    surfaces: BoundedSurfacePool<MacSurfaceKey, MacSurface>,
}

impl MacImporter {
    fn new(gpu: GpuVideoContext) -> Result<Self, String> {
        use wgpu::hal::api::Metal;
        let texture_cache = unsafe {
            let hal = gpu
                .device()
                .as_hal::<Metal>()
                .ok_or_else(|| "CoreVideo import requires wgpu's Metal backend".to_string())?;
            let mut raw = std::ptr::null_mut();
            let out = NonNull::new(&mut raw as *mut *mut CVMetalTextureCache)
                .expect("address of output pointer is non-null");
            let status = CVMetalTextureCache::create(None, None, hal.raw_device(), None, out);
            if status != 0 {
                return Err(format!("CVMetalTextureCacheCreate failed with {status}"));
            }
            CFRetained::from_raw(
                NonNull::new(raw)
                    .ok_or_else(|| "CVMetalTextureCacheCreate returned a null cache".to_string())?,
            )
        };
        Ok(Self {
            gpu,
            texture_cache,
            surfaces: BoundedSurfacePool::new(MAX_IN_FLIGHT_MAC_VIDEO_SURFACES),
        })
    }

    fn allocate_surface(&self, frame: &MacFrame) -> Result<MacSurface, String> {
        use wgpu::hal::api::Metal;
        let width = CVPixelBufferGetWidth(&frame.pixel_buffer) as u32;
        let height = CVPixelBufferGetHeight(&frame.pixel_buffer) as u32;
        let cv_texture = unsafe {
            let mut raw = std::ptr::null_mut();
            let out = NonNull::new(&mut raw as *mut *mut CVMetalTexture)
                .expect("address of output pointer is non-null");
            let status = CVMetalTextureCache::create_texture_from_image(
                None,
                &self.texture_cache,
                &frame.pixel_buffer,
                None,
                MTLPixelFormat::BGRA8Unorm_sRGB,
                width as usize,
                height as usize,
                0,
                out,
            );
            if status != 0 {
                return Err(format!(
                    "CVMetalTextureCacheCreateTextureFromImage failed with {status}"
                ));
            }
            CFRetained::from_raw(
                NonNull::new(raw)
                    .ok_or_else(|| "CoreVideo returned a null Metal texture".to_string())?,
            )
        };
        let metal_texture = CVMetalTextureGetTexture(&cv_texture)
            .ok_or_else(|| "CVMetalTexture has no MTLTexture".to_string())?;
        let hal_texture = unsafe {
            wgpu_hal::metal::Device::texture_from_raw(
                metal_texture,
                wgpu::TextureFormat::Bgra8UnormSrgb,
                MTLTextureType::Type2D,
                1,
                1,
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                }
                .into(),
                None,
            )
        };
        let texture = unsafe {
            self.gpu.device().create_texture_from_hal::<Metal>(
                hal_texture,
                &wgpu::TextureDescriptor {
                    label: Some("Neomacs zero-copy CoreVideo surface"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::TextureUses::RESOURCE,
            )
        };
        Ok(MacSurface {
            sampled: self.gpu.prepare_texture(
                texture,
                VideoFrameFormat::Packed(PackedVideoFormat::Bgra8)
                    .allocation_bytes(VideoGeometry::packed(width, height))
                    .map_err(|error| error.to_string())?,
            ),
            _cv_texture: cv_texture,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct MacSurfaceKey {
    pixel_buffer: usize,
    width: u32,
    height: u32,
}

struct MacSurface {
    sampled: PreparedSampledTexture,
    _cv_texture: CFRetained<CVMetalTexture>,
}

struct MetalFrameLease {
    _frame: MacFrame,
    _surface: crate::surface_pool::SurfaceLease<MacSurfaceKey, MacSurface>,
}

unsafe impl Send for MetalFrameLease {}
unsafe impl Sync for MetalFrameLease {}

impl FrameImporter<MacFrame> for MacImporter {
    type Sampled = GpuVideoFrame;

    fn transfer_path(&self, _frame: &DecodedFrame<MacFrame>) -> VideoTransferPath {
        // CVPixelBuffer -> Metal is zero-copy at the compositor boundary,
        // but requesting packed BGRA can make AVFoundation run a native GPU
        // YUV/color conversion. Do not call that strict direct replay.
        VideoTransferPath::GpuInteropCopy
    }

    fn import(
        &mut self,
        frame: DecodedFrame<MacFrame>,
    ) -> Result<FrameImportOutcome<Self::Sampled>, String> {
        debug_assert_eq!(
            frame.format,
            VideoFrameFormat::Packed(PackedVideoFormat::Bgra8)
        );
        let width = frame.geometry.coded_width;
        let height = frame.geometry.coded_height;
        let key = MacSurfaceKey {
            pixel_buffer: (&*frame.lease.pixel_buffer as *const CVPixelBuffer) as usize,
            width,
            height,
        };
        let surface = match self.surfaces.acquire(key) {
            SurfacePoolAcquire::Reused(surface) => surface,
            SurfacePoolAcquire::Allocate(reservation) => {
                reservation.fulfill(self.allocate_surface(&frame.lease)?)
            }
            SurfacePoolAcquire::Backpressured => {
                return Ok(FrameImportOutcome::Backpressured);
            }
        };
        let prepared = surface.value().sampled.clone();
        let sampled = self.gpu.wrap_prepared_texture(
            frame.geometry,
            prepared,
            MetalFrameLease {
                _frame: frame.lease,
                _surface: surface,
            },
        );
        Ok(FrameImportOutcome::Ready(ImportedFrame {
            sampled,
            path: VideoTransferPath::GpuInteropCopy,
        }))
    }
}

impl Platform for MacPlatform {
    const BACKEND: VideoDecodeBackend = VideoDecodeBackend::AvFoundation;
    type Frame = MacFrame;
    type Sampled = GpuVideoFrame;
    type Decoder = MacDecoder;
    type Importer = MacImporter;
}

impl ProductionPlatform for MacPlatform {
    fn create(
        gpu: GpuVideoContext,
        _policy: crate::FrameTransferPolicy,
        wake: VideoWake,
    ) -> Result<(Self::Decoder, Self::Importer), VideoInitError> {
        let importer = MacImporter::new(gpu).map_err(|message| VideoInitError::Backend {
            backend: VideoDecodeBackend::AvFoundation,
            message,
        })?;
        let decoder = MacDecoder::new(wake).map_err(|message| VideoInitError::Backend {
            backend: VideoDecodeBackend::AvFoundation,
            message,
        })?;
        Ok((decoder, importer))
    }
}
