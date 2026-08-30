use std::collections::HashMap;
use std::num::NonZeroU32;
use std::os::fd::{FromRawFd, OwnedFd};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_allocators as gst_allocators;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use neomacs_display_protocol::types::VideoId;

use crate::backend::{
    BackendEvent, BackendInbox, BackendPublisher, DecodedFrame, DecoderBackend, backend_bridge,
};
use crate::sampling::LinuxDrmDevice;
use crate::{
    FrameTiming, FrameTransferPolicy, InitialPlayback, LoopMode, MediaTime, PixelAspectRatio,
    PlaybackAction, PlaybackEpoch, VideoCommand, VideoGeometry, VideoRotation, VideoSampling,
    VideoSessionState, VideoSource, VideoTransferPath, VideoWake,
};

use super::frame::{
    CpuPackedSurface, DmaBufPlane, DmaBufSurface, LinuxFrameLease, LinuxFrameStorage,
};

enum WorkerCommand {
    Play,
    Pause,
    Stop,
    Seek(MediaTime),
    SetRate(f64),
    SetLoop(LoopMode),
    SetPresentation(crate::PresentationVisibility),
    Close,
}

pub(crate) struct GstreamerDecoder {
    output: BackendPublisher<LinuxFrameLease>,
    incoming: BackendInbox<LinuxFrameLease>,
    workers: HashMap<VideoId, Worker>,
    worker_reaper: Sender<Worker>,
    transfer_policy: FrameTransferPolicy,
    renderer_drm_device: Option<LinuxDrmDevice>,
}

struct Worker {
    commands: Sender<WorkerCommand>,
    shutting_down: Arc<AtomicBool>,
    join: thread::JoinHandle<()>,
}

impl Worker {
    fn begin_close(self, worker_reaper: &Sender<Self>) -> Result<(), String> {
        self.shutting_down.store(true, Ordering::Release);
        // A backend error may already have ended the thread. The command is a
        // wake-up hint, not an acknowledgement protocol. Joining belongs to
        // the dedicated reaper because command() runs on the render thread.
        let _ = self.commands.send(WorkerCommand::Close);
        worker_reaper
            .send(self)
            .map_err(|_| "GStreamer worker reaper has exited".to_string())
    }
}

impl GstreamerDecoder {
    pub(super) fn new(
        wake: VideoWake,
        transfer_policy: FrameTransferPolicy,
        renderer_drm_device: Option<LinuxDrmDevice>,
    ) -> Result<Self, String> {
        gst::init().map_err(|error| error.to_string())?;
        let (output, incoming) = backend_bridge(wake);
        let (worker_reaper, workers_to_reap) = crossbeam_channel::unbounded::<Worker>();
        thread::Builder::new()
            .name("neomacs-video-reaper".into())
            .spawn(move || {
                for worker in workers_to_reap {
                    let _ = worker.join.join();
                }
            })
            .map_err(|error| format!("failed to spawn GStreamer worker reaper: {error}"))?;
        Ok(Self {
            output,
            incoming,
            workers: HashMap::new(),
            worker_reaper,
            transfer_policy,
            renderer_drm_device,
        })
    }

    fn open(
        &mut self,
        id: VideoId,
        source: VideoSource,
        initial_playback: InitialPlayback,
        loop_mode: LoopMode,
    ) -> Result<(), String> {
        if self.workers.contains_key(&id) {
            return Err(format!("video {} is already open", id.get()));
        }
        let (command_tx, command_rx) = crossbeam_channel::unbounded();
        let output = self.output.clone();
        let transfer_policy = self.transfer_policy;
        let renderer_drm_device = self.renderer_drm_device;
        let shutting_down = Arc::new(AtomicBool::new(false));
        let worker_shutdown = Arc::clone(&shutting_down);
        let join = thread::Builder::new()
            .name(format!("neomacs-video-{}", id.get()))
            .spawn(move || {
                run_worker(
                    id,
                    source,
                    initial_playback,
                    loop_mode,
                    transfer_policy,
                    renderer_drm_device,
                    command_rx,
                    output,
                    worker_shutdown,
                )
            })
            .map_err(|error| format!("failed to spawn GStreamer worker: {error}"))?;
        self.workers.insert(
            id,
            Worker {
                commands: command_tx,
                shutting_down,
                join,
            },
        );
        Ok(())
    }

    fn send(&mut self, id: VideoId, command: WorkerCommand) -> Result<(), String> {
        let worker = self
            .workers
            .get(&id)
            .ok_or_else(|| format!("video {} is not open", id.get()))?;
        worker
            .commands
            .send(command)
            .map_err(|_| format!("video {} worker has exited", id.get()))
    }
}

impl DecoderBackend for GstreamerDecoder {
    type Frame = LinuxFrameLease;

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
            VideoCommand::Playback { id, action } => {
                let command = match action {
                    PlaybackAction::Play => WorkerCommand::Play,
                    PlaybackAction::Pause => WorkerCommand::Pause,
                    PlaybackAction::Stop => WorkerCommand::Stop,
                    PlaybackAction::Seek(time) => WorkerCommand::Seek(time),
                    PlaybackAction::SetRate(rate) => WorkerCommand::SetRate(rate.get()),
                    PlaybackAction::SetLoop(mode) => WorkerCommand::SetLoop(mode),
                };
                self.send(id, command).map_err(Into::into)
            }
            VideoCommand::Presentation { id, visibility } => self
                .send(id, WorkerCommand::SetPresentation(visibility))
                .map_err(Into::into),
            VideoCommand::Close { id } => {
                let worker = self
                    .workers
                    .remove(&id)
                    .ok_or(crate::VideoCommandError::SessionNotOpen { id: id.get() })?;
                self.incoming.remove_frame(id);
                worker.begin_close(&self.worker_reaper).map_err(Into::into)
            }
        }
    }

    fn drain_events(&mut self) -> Vec<BackendEvent<Self::Frame>> {
        self.incoming.drain()
    }
}

fn run_worker(
    id: VideoId,
    source: VideoSource,
    initial_playback: InitialPlayback,
    mut loop_mode: LoopMode,
    transfer_policy: FrameTransferPolicy,
    renderer_drm_device: Option<LinuxDrmDevice>,
    commands: Receiver<WorkerCommand>,
    output: BackendPublisher<LinuxFrameLease>,
    shutting_down: Arc<AtomicBool>,
) {
    if let Err(error) = run_worker_inner(
        id,
        source,
        initial_playback,
        &mut loop_mode,
        transfer_policy,
        renderer_drm_device,
        &commands,
        &output,
        &shutting_down,
    ) {
        output.event(BackendEvent::Failed { id, error });
    }
}

fn run_worker_inner(
    id: VideoId,
    source: VideoSource,
    initial_playback: InitialPlayback,
    loop_mode: &mut LoopMode,
    transfer_policy: FrameTransferPolicy,
    renderer_drm_device: Option<LinuxDrmDevice>,
    commands: &Receiver<WorkerCommand>,
    output: &BackendPublisher<LinuxFrameLease>,
    shutting_down: &AtomicBool,
) -> Result<(), crate::VideoCommandError> {
    let uri = source_uri(source)?;
    let caps = preferred_sink_caps(transfer_policy);
    let appsink = gst_app::AppSink::builder()
        .caps(&caps)
        .max_buffers(2)
        .drop(true)
        // Let GStreamer pace decoded output against its media clock. With an
        // unbounded-rate sink, a local file can decode to EOS and repeatedly
        // replace the one-slot mailbox before the compositor presents frame
        // one.
        .sync(true)
        .enable_last_sample(false)
        .build();
    let audio_sink = gst::ElementFactory::make("fakesink")
        .build()
        .map_err(|error| format!("failed to create audio sink: {error}"))?;
    let playbin_factory = if gst::ElementFactory::find("playbin3").is_some() {
        "playbin3"
    } else {
        "playbin"
    };
    let pipeline = gst::ElementFactory::make(playbin_factory)
        .property("uri", uri.as_str())
        .property("video-sink", &appsink)
        .property("audio-sink", &audio_sink)
        .build()
        .map_err(|error| format!("failed to create {playbin_factory}: {error}"))?;
    let bus = pipeline
        .bus()
        .ok_or_else(|| "GStreamer playback element has no bus".to_string())?;
    let initial_state = match initial_playback {
        InitialPlayback::Playing => gst::State::Playing,
        InitialPlayback::Paused => gst::State::Paused,
    };
    pipeline
        .set_state(initial_state)
        .map_err(|error| format!("failed to start GStreamer pipeline: {error:?}"))?;

    let mut announced = false;
    let mut playing = matches!(initial_playback, InitialPlayback::Playing);
    let mut presented = true;
    let mut need_preroll = !playing;
    let mut closed = false;
    let mut epoch = PlaybackEpoch::INITIAL;
    let mut rotation = VideoRotation::None;
    while !closed {
        while let Ok(command) = commands.try_recv() {
            closed = apply_command(
                id,
                command,
                &pipeline,
                loop_mode,
                &mut playing,
                &mut presented,
                &mut need_preroll,
                &mut epoch,
                output,
            )?;
            if closed {
                break;
            }
        }
        if closed {
            break;
        }

        // Hidden and fully quiescent paused sessions block on their command
        // channel. They consume no decoder or polling cadence until the
        // compositor presents them again (or the user changes playback).
        if !presented || (!playing && !need_preroll) {
            match commands.recv() {
                Ok(command) => {
                    closed = apply_command(
                        id,
                        command,
                        &pipeline,
                        loop_mode,
                        &mut playing,
                        &mut presented,
                        &mut need_preroll,
                        &mut epoch,
                        output,
                    )?;
                    continue;
                }
                Err(_) => break,
            }
        }

        // Tags are posted before decoded samples. Consume them before pulling
        // a frame so orientation participates in the very first published
        // geometry and Ready dimensions. Defer EOS until after the appsink is
        // drained so the terminal frame cannot be published after Ended.
        let mut reached_eos = false;
        while let Some(message) = bus.pop() {
            match message.view() {
                gst::MessageView::Tag(tag) => {
                    if let Some(orientation) = tag.tags().get::<gst::tags::ImageOrientation>() {
                        rotation = rotation_from_gstreamer_tag(orientation.get());
                    }
                }
                gst::MessageView::Eos(..) => {
                    reached_eos = true;
                }
                gst::MessageView::Error(error) => {
                    return Err(format!(
                        "GStreamer error from {:?}: {} ({:?})",
                        error.src().map(|source| source.path_string()),
                        error.error(),
                        error.debug()
                    )
                    .into());
                }
                _ => {}
            }
        }

        let sample = if need_preroll {
            appsink
                .try_pull_preroll(gst::ClockTime::from_mseconds(10))
                .inspect(|_| need_preroll = false)
        } else {
            appsink.try_pull_sample(gst::ClockTime::from_mseconds(10))
        };
        if let Some(sample) = sample
            && let Some(frame) = decode_sample(
                sample,
                shutting_down,
                epoch,
                rotation,
                renderer_drm_device,
                pipeline_drm_identity(&pipeline),
            )?
        {
            if !announced {
                output.event(BackendEvent::Opened {
                    id,
                    width: frame.geometry.display_width,
                    height: frame.geometry.display_height,
                    initial_state: if playing {
                        VideoSessionState::Playing
                    } else {
                        VideoSessionState::Paused
                    },
                });
                announced = true;
            }
            output.frame(id, frame);
        }
        if reached_eos {
            if loop_mode.consume_replay() {
                epoch = epoch.next();
                output.event(BackendEvent::Looped {
                    id,
                    remaining: *loop_mode,
                });
                pipeline
                    .seek_simple(
                        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                        gst::ClockTime::ZERO,
                    )
                    .map_err(|error| format!("failed to loop video: {error}"))?;
            } else {
                playing = false;
                output.event(BackendEvent::Ended { id });
            }
        }
    }
    let _ = pipeline.set_state(gst::State::Null);
    output.event(BackendEvent::StateChanged {
        id,
        state: VideoSessionState::Closed,
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_command(
    id: VideoId,
    command: WorkerCommand,
    pipeline: &gst::Element,
    loop_mode: &mut LoopMode,
    playing: &mut bool,
    presented: &mut bool,
    need_preroll: &mut bool,
    epoch: &mut PlaybackEpoch,
    output: &BackendPublisher<LinuxFrameLease>,
) -> Result<bool, String> {
    let state = match command {
        WorkerCommand::Play => {
            if *presented {
                pipeline
                    .set_state(gst::State::Playing)
                    .map_err(|error| format!("failed to play video: {error:?}"))?;
            }
            *playing = true;
            *need_preroll = false;
            Some(VideoSessionState::Playing)
        }
        WorkerCommand::Pause => {
            pipeline
                .set_state(gst::State::Paused)
                .map_err(|error| format!("failed to pause video: {error:?}"))?;
            *playing = false;
            *need_preroll = *presented;
            Some(VideoSessionState::Paused)
        }
        WorkerCommand::Stop => {
            pipeline
                .set_state(gst::State::Paused)
                .map_err(|error| format!("failed to stop video: {error:?}"))?;
            pipeline
                .seek_simple(gst::SeekFlags::FLUSH, gst::ClockTime::ZERO)
                .map_err(|error| format!("failed to rewind stopped video: {error}"))?;
            *playing = false;
            *need_preroll = *presented;
            *epoch = epoch.next();
            Some(VideoSessionState::Paused)
        }
        WorkerCommand::Seek(position) => {
            pipeline
                .seek_simple(
                    gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                    gst::ClockTime::from_nseconds(position.as_nanos()),
                )
                .map_err(|error| format!("failed to seek video: {error}"))?;
            if !*playing {
                *need_preroll = true;
            }
            *epoch = epoch.next();
            None
        }
        WorkerCommand::SetRate(new_rate) => {
            let position = pipeline
                .query_position::<gst::ClockTime>()
                .unwrap_or(gst::ClockTime::ZERO);
            pipeline
                .seek(
                    new_rate,
                    gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                    gst::SeekType::Set,
                    position,
                    gst::SeekType::None,
                    gst::ClockTime::NONE,
                )
                .map_err(|error| format!("failed to change video rate: {error}"))?;
            None
        }
        WorkerCommand::SetLoop(mode) => {
            *loop_mode = mode;
            None
        }
        WorkerCommand::SetPresentation(visibility) => {
            *presented = matches!(visibility, crate::PresentationVisibility::Presented);
            if *presented {
                pipeline
                    .set_state(if *playing {
                        gst::State::Playing
                    } else {
                        gst::State::Paused
                    })
                    .map_err(|error| {
                        format!("failed to resume visible video pipeline: {error:?}")
                    })?;
                *need_preroll = !*playing;
            } else {
                pipeline.set_state(gst::State::Paused).map_err(|error| {
                    format!("failed to suspend hidden video pipeline: {error:?}")
                })?;
                *need_preroll = false;
            }
            None
        }
        WorkerCommand::Close => return Ok(true),
    };
    if let Some(state) = state {
        output.event(BackendEvent::StateChanged { id, state });
    }
    Ok(false)
}

impl Drop for GstreamerDecoder {
    fn drop(&mut self) {
        for (_, worker) in self.workers.drain() {
            // Decoder teardown is also render-thread owned. The detached
            // reaper owns the final join and exits after this sender drops.
            let _ = worker.begin_close(&self.worker_reaper);
        }
    }
}

fn source_uri(source: VideoSource) -> Result<String, String> {
    match source {
        VideoSource::File(path) => gst::glib::filename_to_uri(path, None)
            .map(String::from)
            .map_err(|error| format!("invalid video path: {error}")),
        VideoSource::Uri(uri) => Ok(uri),
    }
}

fn preferred_sink_caps(policy: FrameTransferPolicy) -> gst::Caps {
    let builder = gst::Caps::builder_full().structure_with_features(
        gst::Structure::builder("video/x-raw")
            .field("format", "DMA_DRM")
            .field("colorimetry", "sRGB")
            // Keep the compositor contract one packed sampled texture.
            // Hardware decoders may use a GPU video processor to convert
            // NV12/P010, which remains a zero-CPU-copy interop path.
            .field("drm-format", gst::List::new(["AR24", "AB24"]))
            .build(),
        gst::CapsFeatures::new(["memory:DMABuf"]),
    );
    if policy.permits(VideoTransferPath::CpuUpload) {
        builder
            .structure(
                gst::Structure::builder("video/x-raw")
                    .field("format", gst::List::new(["RGBA", "BGRA"]))
                    .field("colorimetry", "sRGB")
                    .build(),
            )
            .build()
    } else {
        builder.build()
    }
}

fn decode_sample(
    sample: gst::Sample,
    shutting_down: &AtomicBool,
    epoch: PlaybackEpoch,
    rotation: VideoRotation,
    renderer_drm_device: Option<LinuxDrmDevice>,
    pipeline_drm_topology: PipelineDrmTopology,
) -> Result<Option<DecodedFrame<LinuxFrameLease>>, crate::VideoCommandError> {
    let caps = sample
        .caps()
        .ok_or_else(|| "decoded video sample has no caps".to_string())?;
    let buffer = sample
        .buffer()
        .ok_or_else(|| "decoded video sample has no buffer".to_string())?;
    let timing = FrameTiming {
        pts: MediaTime::from_nanos(buffer.pts().map_or(0, |time| time.nseconds())),
        duration: MediaTime::from_nanos(buffer.duration().map_or(0, |time| time.nseconds())),
        epoch,
    };

    if let Ok(drm_info) = gst_video::VideoInfoDmaDrm::from_caps(caps) {
        let info = drm_info
            .to_video_info()
            .map_err(|error| format!("invalid DMA_DRM video info: {error}"))?;
        let geometry =
            geometry_from_info(&info, buffer.meta::<gst_video::VideoCropMeta>(), rotation);
        let surface = extract_dmabuf(buffer, &info, drm_info.fourcc(), drm_info.modifier())?;
        let transfer_path = dma_buf_transfer_path(renderer_drm_device, pipeline_drm_topology)?;
        if !wait_for_decoder_write(&surface, shutting_down)? {
            return Ok(None);
        }
        let sampling = sampling_from_fourcc(drm_info.fourcc())?;
        return Ok(Some(DecodedFrame {
            lease: LinuxFrameLease {
                _sample: sample,
                storage: LinuxFrameStorage::DmaBuf(surface),
                transfer_path,
            },
            timing,
            geometry,
            sampling,
        }));
    }

    let info = gst_video::VideoInfo::from_caps(caps)
        .map_err(|error| format!("invalid packed video caps: {error}"))?;
    let sampling = match info.format() {
        gst_video::VideoFormat::Rgba => VideoSampling::Rgba8,
        gst_video::VideoFormat::Bgra => VideoSampling::Bgra8,
        format => return Err(format!("unsupported packed video format {format:?}").into()),
    };
    let geometry = geometry_from_info(&info, buffer.meta::<gst_video::VideoCropMeta>(), rotation);
    let bytes = {
        let map = buffer
            .map_readable()
            .map_err(|error| format!("failed to map packed video sample: {error}"))?;
        map.as_slice().to_vec()
    };
    let storage = CpuPackedSurface {
        bytes,
        stride: u32::try_from(info.stride()[0])
            .map_err(|_| "negative video row stride is unsupported".to_string())?,
    };
    Ok(Some(DecodedFrame {
        lease: LinuxFrameLease {
            _sample: sample,
            storage: LinuxFrameStorage::CpuPacked(storage),
            transfer_path: VideoTransferPath::CpuUpload,
        },
        timing,
        geometry,
        sampling,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineDrmIdentity {
    Unknown,
    Single(LinuxDrmDevice),
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PipelineDrmTopology {
    /// Devices reported specifically by decoder elements.
    decoder: PipelineDrmIdentity,
    /// Devices reported by any element that can participate in producing the
    /// final packed DMA-BUF, including converters and upload/postprocess nodes.
    surface_path: PipelineDrmIdentity,
    inspection_failed: bool,
}

impl PipelineDrmTopology {
    const UNKNOWN: Self = Self {
        decoder: PipelineDrmIdentity::Unknown,
        surface_path: PipelineDrmIdentity::Unknown,
        inspection_failed: false,
    };
}

impl PipelineDrmIdentity {
    fn observe(self, device: LinuxDrmDevice) -> Self {
        match self {
            Self::Unknown => Self::Single(device),
            Self::Single(existing) if existing == device => self,
            Self::Single(_) | Self::Conflict => Self::Conflict,
        }
    }
}

fn pipeline_drm_identity(pipeline: &gst::Element) -> PipelineDrmTopology {
    let Some(bin) = pipeline.downcast_ref::<gst::Bin>() else {
        return PipelineDrmTopology::UNKNOWN;
    };
    let mut elements = bin.iterate_recurse();
    let mut topology = PipelineDrmTopology::UNKNOWN;
    loop {
        match elements.next() {
            Ok(Some(element)) => {
                if element.find_property("device-path").is_none() {
                    continue;
                }
                let Ok(path) = element
                    .property_value("device-path")
                    .get::<Option<String>>()
                else {
                    continue;
                };
                let Some(path) = path else {
                    continue;
                };
                if let Some(device) = LinuxDrmDevice::from_path(std::path::Path::new(&path)) {
                    topology.surface_path = topology.surface_path.observe(device);
                    if element.is::<gst_video::VideoDecoder>() {
                        topology.decoder = topology.decoder.observe(device);
                    }
                }
            }
            Ok(None) => return topology,
            Err(gst::IteratorError::Error) => {
                topology.inspection_failed = true;
                return topology;
            }
            Err(gst::IteratorError::Resync) => elements.resync(),
        }
    }
}

fn dma_buf_transfer_path(
    renderer: Option<LinuxDrmDevice>,
    pipeline: PipelineDrmTopology,
) -> Result<VideoTransferPath, crate::VideoCommandError> {
    // The current appsink contract requests a packed sRGB DMA-BUF. Even when
    // the pipeline and compositor resolve to the same DRM device, a hardware
    // decoder that produces NV12/P010 may require a native GPU conversion to
    // that packed surface. It remains CPU-zero-copy, but strict direct replay
    // is reserved for the future native-plane sampling path which can prove
    // that no conversion/copy participated.
    if pipeline.inspection_failed {
        return Err(crate::VideoCommandError::AdapterMismatch {
            details: "GStreamer pipeline device inspection failed before the DMA-BUF producer topology could be proven".into(),
        });
    }
    for (role, identity) in [
        ("decoder", pipeline.decoder),
        ("DMA-BUF surface path", pipeline.surface_path),
    ] {
        match (renderer, identity) {
            (_, PipelineDrmIdentity::Conflict) => {
                return Err(crate::VideoCommandError::AdapterMismatch {
                    details: format!(
                        "video pipeline {role} spans multiple DRM render nodes; cross-adapter DMA-BUF import is unsupported"
                    ),
                });
            }
            (Some(renderer), PipelineDrmIdentity::Single(decoder)) if renderer != decoder => {
                return Err(crate::VideoCommandError::AdapterMismatch {
                    details: format!(
                        "video pipeline {role} DRM device {decoder:?} does not match compositor device {renderer:?}; cross-adapter DMA-BUF import is unsupported"
                    ),
                });
            }
            _ => {}
        }
    }
    Ok(VideoTransferPath::GpuInteropCopy)
}

/// GStreamer/media drivers commonly publish producer completion through the
/// DMA-BUF reservation object. Vulkan is explicitly synchronized, so wait on
/// that implicit write fence on the decoder worker before the render thread
/// imports the memory. This never blocks the UI thread.
fn wait_for_decoder_write(
    surface: &DmaBufSurface,
    shutting_down: &AtomicBool,
) -> Result<bool, String> {
    let mut fds: Vec<_> = surface
        .planes
        .iter()
        .map(|plane| libc::pollfd {
            fd: std::os::fd::AsRawFd::as_raw_fd(&plane.fd),
            events: libc::POLLIN,
            revents: 0,
        })
        .collect();
    fds.sort_unstable_by_key(|fd| fd.fd);
    fds.dedup_by_key(|fd| fd.fd);
    loop {
        if shutting_down.load(Ordering::Acquire) {
            return Ok(false);
        }
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as _, 100) };
        if ready > 0 {
            return Ok(true);
        }
        if ready == 0 {
            continue;
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(format!("waiting for DMA-BUF decoder fence failed: {error}"));
        }
    }
}

fn extract_dmabuf(
    buffer: &gst::BufferRef,
    info: &gst_video::VideoInfo,
    fourcc: u32,
    modifier: u64,
) -> Result<DmaBufSurface, String> {
    let meta = buffer.meta::<gst_video::VideoMeta>();
    let offsets = meta.as_ref().map_or(info.offset(), |meta| meta.offset());
    let strides = meta.as_ref().map_or(info.stride(), |meta| meta.stride());
    let n_planes = meta
        .as_ref()
        .map_or(info.n_planes(), |meta| meta.n_planes()) as usize;
    if n_planes == 0 || n_planes > 4 || buffer.n_memory() == 0 {
        return Err(format!("invalid DMA-BUF plane count {n_planes}"));
    }
    let memory_layout = DmaBufMemoryLayout::classify(buffer.n_memory(), n_planes)?;

    let mut planes = Vec::with_capacity(n_planes);
    for plane in 0..n_planes {
        // One GstMemory may legitimately contain every plane at different
        // offsets. Otherwise each plane needs its own corresponding memory;
        // never silently reuse the last descriptor of a partial list.
        let memory_index = memory_layout.memory_index(plane);
        let memory = buffer.peek_memory(memory_index);
        let raw_fd =
            if let Some(memory) = memory.downcast_memory_ref::<gst_allocators::DmaBufMemory>() {
                memory.fd()
            } else if let Some(memory) = memory.downcast_memory_ref::<gst_allocators::FdMemory>() {
                memory.fd()
            } else {
                return Err(format!("DMA-BUF plane {plane} is not fd-backed"));
            };
        let duplicated = unsafe { libc::dup(raw_fd) };
        if duplicated < 0 {
            return Err(format!("failed to duplicate DMA-BUF fd for plane {plane}"));
        }
        planes.push(DmaBufPlane {
            // SAFETY: `dup` returned a new owned descriptor above.
            fd: unsafe { OwnedFd::from_raw_fd(duplicated) },
            stride: u32::try_from(strides[plane])
                .map_err(|_| format!("negative stride for DMA-BUF plane {plane}"))?,
            offset: u32::try_from(offsets[plane])
                .map_err(|_| format!("offset too large for DMA-BUF plane {plane}"))?,
        });
    }
    Ok(DmaBufSurface {
        planes,
        fourcc,
        modifier,
    })
}

#[derive(Clone, Copy)]
enum DmaBufMemoryLayout {
    Shared,
    PerPlane,
}

impl DmaBufMemoryLayout {
    fn classify(memory_count: usize, plane_count: usize) -> Result<Self, String> {
        match memory_count {
            1 => Ok(Self::Shared),
            count if count >= plane_count => Ok(Self::PerPlane),
            count => Err(format!(
                "DMA-BUF advertises {plane_count} planes but supplies only {count} memory objects"
            )),
        }
    }

    const fn memory_index(self, plane: usize) -> usize {
        match self {
            Self::Shared => 0,
            Self::PerPlane => plane,
        }
    }
}

fn geometry_from_info(
    info: &gst_video::VideoInfo,
    crop: Option<gst::MetaRef<'_, gst_video::VideoCropMeta>>,
    rotation: VideoRotation,
) -> VideoGeometry {
    let par = info.par();
    let numerator =
        NonZeroU32::new(u32::try_from(par.numer()).unwrap_or(1)).unwrap_or(NonZeroU32::MIN);
    let denominator =
        NonZeroU32::new(u32::try_from(par.denom()).unwrap_or(1)).unwrap_or(NonZeroU32::MIN);
    let visible_rect = crop.map_or(
        crate::PixelRect {
            x: 0,
            y: 0,
            width: info.width(),
            height: info.height(),
        },
        |crop| {
            let (x, y, width, height) = crop.rect();
            crate::PixelRect {
                x,
                y,
                width,
                height,
            }
        },
    );
    VideoGeometry::with_pixel_aspect_ratio(
        info.width(),
        info.height(),
        visible_rect,
        PixelAspectRatio {
            numerator,
            denominator,
        },
        rotation,
    )
}

fn rotation_from_gstreamer_tag(orientation: &str) -> VideoRotation {
    match orientation {
        "rotate-90" => VideoRotation::Clockwise90,
        "rotate-180" => VideoRotation::Clockwise180,
        "rotate-270" => VideoRotation::Clockwise270,
        _ => VideoRotation::None,
    }
}

fn sampling_from_fourcc(fourcc: u32) -> Result<VideoSampling, String> {
    match fourcc {
        0x3432_5241 => Ok(VideoSampling::Bgra8),
        0x3432_4241 => Ok(VideoSampling::Rgba8),
        _ => Err(format!("unsupported DRM video format {fourcc:#010x}")),
    }
}

#[cfg(test)]
#[path = "decoder_test.rs"]
mod tests;
