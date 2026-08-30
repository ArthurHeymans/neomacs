//! Renderer-facing facade over the cross-platform native video subsystem.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use neomacs_display_protocol::types::VideoId;
use neomacs_video::{
    FrameTransferPolicy, GpuGeneration, InitialPlayback, LoopMode, PlaybackAction,
    PresentationVisibility, VideoCommand, VideoEvent, VideoServiceResult, VideoSessionState,
    VideoSource, VideoSystem, VideoWake,
};

use neomacs_video::VideoRecoveryManifest as PlaybackRecoveryManifest;

/// Stable renderer identity paired with identity-free playback recovery data.
///
/// This is the device-loss payload retained by the render runtime. The inner
/// playback manifest cannot carry either an editor id or a native-session id,
/// so crossing this boundary cannot silently confuse those identity domains.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoRecoveryManifest {
    id: VideoId,
    playback: PlaybackRecoveryManifest,
    state: VideoState,
}

impl VideoRecoveryManifest {
    pub const fn id(&self) -> VideoId {
        self.id
    }
}

/// Ephemeral identity of one native decoder/player incarnation.
///
/// This deliberately cannot be confused with the stable [`VideoId`] carried
/// by Lisp/layout. Reopening a parked video allocates a new value, so delayed
/// events from the old native session cannot target the new incarnation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct NativeVideoSessionId(VideoId);

impl NativeVideoSessionId {
    const fn protocol(self) -> VideoId {
        self.0
    }
}

/// Compatibility presentation of the typed native session lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoState {
    Loading,
    Playing,
    Paused,
    Stopped,
    EndOfStream,
    Error,
}

impl From<VideoSessionState> for VideoState {
    fn from(state: VideoSessionState) -> Self {
        match state {
            VideoSessionState::Opening => Self::Loading,
            VideoSessionState::Playing => Self::Playing,
            VideoSessionState::Paused => Self::Paused,
            VideoSessionState::Ended => Self::EndOfStream,
            VideoSessionState::Failed => Self::Error,
            VideoSessionState::Closed => Self::Stopped,
        }
    }
}

/// Renderer-facing metadata. The authoritative frame, GPU handles, and native
/// lease remain together in [`VideoSystem`].
pub struct CachedVideo {
    pub id: VideoId,
    pub width: u32,
    pub height: u32,
    pub state: VideoState,
    pub frame_count: u64,
    native_id: Option<NativeVideoSessionId>,
    parked: Option<PlaybackRecoveryManifest>,
}

/// Renderer preparation keyed by stable declarative ids while the native
/// subsystem is free to replace parked decoder-session ids.
pub struct PreparedVideoDraws<'a> {
    native: neomacs_video::PreparedVideoDraws<'a>,
    native_ids: HashMap<VideoId, NativeVideoSessionId>,
}

impl<'a> PreparedVideoDraws<'a> {
    pub fn get(&self, id: VideoId) -> Option<neomacs_video::PreparedVideoDraw<'a>> {
        self.native.get(self.native_ids.get(&id)?.protocol())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VideoGpuAccountingChange {
    Unchanged,
    Register(usize),
    Free,
}

#[derive(Default)]
struct VideoGpuAccounting {
    bytes: usize,
}

impl VideoGpuAccounting {
    fn observe(&mut self, bytes: usize) -> VideoGpuAccountingChange {
        let change = match (self.bytes, bytes) {
            (previous, current) if previous == current => VideoGpuAccountingChange::Unchanged,
            (_, 0) => VideoGpuAccountingChange::Free,
            (_, current) => VideoGpuAccountingChange::Register(current),
        };
        self.bytes = bytes;
        change
    }
}

/// Video import pools are shared across sessions, so accounting them under a
/// fabricated per-session size would double-count and make frees unbalanced.
/// Renderer media IDs start at one; zero is reserved for this aggregate pool.
pub(crate) const VIDEO_GPU_POOL_ACCOUNTING_ID: u32 = 0;

/// Cross-platform video cache. Decode and native-surface import belong to
/// `neomacs-video`; this facade maintains renderer metadata and budget events.
pub struct VideoCache {
    system: Option<VideoSystem>,
    initialization_error: Option<String>,
    videos: HashMap<VideoId, CachedVideo>,
    next_id: u32,
    next_native_id: u32,
    native_to_video: HashMap<NativeVideoSessionId, VideoId>,
    accounting: Vec<crate::media_budget::MediaAccounting>,
    gpu_accounting: VideoGpuAccounting,
    last_service: VideoServiceResult,
}

impl VideoCache {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        generation: GpuGeneration,
        wake: VideoWake,
    ) -> Self {
        let system = VideoSystem::with_sampling_resources(
            device.clone(),
            queue.clone(),
            bind_group_layout.clone(),
            sampler.clone(),
            generation,
            FrameTransferPolicy::AllowCpuUpload,
            wake,
        );
        let (system, initialization_error) = match system {
            Ok(system) => (Some(system), None),
            Err(error) => {
                tracing::error!(%error, "native video subsystem is unavailable");
                (None, Some(error.to_string()))
            }
        };
        Self {
            system,
            initialization_error,
            videos: HashMap::new(),
            next_id: 1,
            next_native_id: 1,
            native_to_video: HashMap::new(),
            accounting: Vec::new(),
            gpu_accounting: VideoGpuAccounting::default(),
            last_service: VideoServiceResult::default(),
        }
    }

    pub fn load_file(&mut self, path: &str) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.load_file_with_id(id, path, 0, false);
        id
    }

    pub fn load_file_with_id(&mut self, id: u32, path: &str, loop_count: i32, autoplay: bool) {
        self.load_source_with_id(id, VideoSource::File(path.into()), loop_count, autoplay);
    }

    pub fn load_uri_with_id(&mut self, id: u32, uri: &str, loop_count: i32, autoplay: bool) {
        self.load_source_with_id(id, VideoSource::Uri(uri.to_owned()), loop_count, autoplay);
    }

    fn load_source_with_id(
        &mut self,
        id: u32,
        source: VideoSource,
        loop_count: i32,
        autoplay: bool,
    ) {
        self.next_id = self.next_id.max(id.saturating_add(1));
        let typed_id = VideoId::new(id);
        let native_id = self.allocate_native_id();
        let loop_mode = match LoopMode::from_legacy(loop_count) {
            Ok(loop_mode) => loop_mode,
            Err(error) => {
                self.videos.insert(
                    typed_id,
                    CachedVideo {
                        id: typed_id,
                        width: 0,
                        height: 0,
                        state: VideoState::Error,
                        frame_count: 0,
                        native_id: None,
                        parked: None,
                    },
                );
                self.fail(id, error.to_string());
                return;
            }
        };
        self.videos.insert(
            typed_id,
            CachedVideo {
                id: typed_id,
                width: 0,
                height: 0,
                state: VideoState::Loading,
                frame_count: 0,
                native_id: Some(native_id),
                parked: None,
            },
        );
        self.native_to_video.insert(native_id, typed_id);
        let result = self.command(VideoCommand::Open {
            id: native_id.protocol(),
            source,
            initial_playback: if autoplay {
                InitialPlayback::Playing
            } else {
                InitialPlayback::Paused
            },
            loop_mode,
        });
        if let Err(error) = result {
            self.native_to_video.remove(&native_id);
            if let Some(video) = self.videos.get_mut(&typed_id) {
                video.native_id = None;
            }
            self.fail(id, error);
        }
    }

    fn allocate_native_id(&mut self) -> NativeVideoSessionId {
        let id = NativeVideoSessionId(VideoId::new(self.next_native_id));
        self.next_native_id = self
            .next_native_id
            .checked_add(1)
            .expect("native video session id space exhausted");
        id
    }

    fn command(&mut self, command: VideoCommand) -> Result<(), String> {
        self.system
            .as_mut()
            .ok_or_else(|| {
                self.initialization_error
                    .clone()
                    .unwrap_or_else(|| "native video subsystem is unavailable".into())
            })?
            .command(command)
            .map_err(|error| error.to_string())
    }

    pub fn get_state(&self, id: u32) -> Option<VideoState> {
        self.videos.get(&VideoId::new(id)).map(|video| video.state)
    }

    pub fn get_dimensions(&self, id: u32) -> Option<(u32, u32)> {
        self.videos
            .get(&VideoId::new(id))
            .map(|video| (video.width, video.height))
    }

    pub fn get(&self, id: u32) -> Option<&CachedVideo> {
        self.videos.get(&VideoId::new(id))
    }

    /// Prepare one immutable, generation-checked view of the video resources
    /// needed by a renderer pass. Native leases and frame ownership stay in
    /// the video system.
    pub fn prepare_draws(
        &self,
        ids: impl IntoIterator<Item = VideoId>,
    ) -> Option<PreparedVideoDraws<'_>> {
        let native_ids: HashMap<_, _> = ids
            .into_iter()
            .filter_map(|id| Some((id, self.videos.get(&id)?.native_id?)))
            .collect();
        let native = self
            .system
            .as_ref()?
            .prepare_draws(native_ids.values().map(|id| id.protocol()));
        Some(PreparedVideoDraws { native, native_ids })
    }

    pub fn play(&mut self, id: u32) {
        let typed_id = VideoId::new(id);
        let result = if let Some(native_id) = self.videos.get(&typed_id).and_then(|v| v.native_id) {
            self.command(VideoCommand::Playback {
                id: native_id.protocol(),
                action: PlaybackAction::Play,
            })
        } else {
            self.update_parked(typed_id, |manifest| manifest.with_desired_playing(true))
        };
        match result {
            Ok(()) => self.set_state(typed_id, VideoState::Playing),
            Err(error) => self.fail(id, error),
        }
    }

    pub fn pause(&mut self, id: u32) {
        let typed_id = VideoId::new(id);
        let result = if let Some(native_id) = self.videos.get(&typed_id).and_then(|v| v.native_id) {
            self.command(VideoCommand::Playback {
                id: native_id.protocol(),
                action: PlaybackAction::Pause,
            })
        } else {
            self.update_parked(typed_id, |manifest| manifest.with_desired_playing(false))
        };
        match result {
            Ok(()) => self.set_state(typed_id, VideoState::Paused),
            Err(error) => self.fail(id, error),
        }
    }

    pub fn stop(&mut self, id: u32) {
        let typed_id = VideoId::new(id);
        let result = if let Some(native_id) = self.videos.get(&typed_id).and_then(|v| v.native_id) {
            self.command(VideoCommand::Playback {
                id: native_id.protocol(),
                action: PlaybackAction::Stop,
            })
        } else {
            self.update_parked(typed_id, PlaybackRecoveryManifest::stopped)
        };
        match result {
            Ok(()) => self.set_state(typed_id, VideoState::Stopped),
            Err(error) => self.fail(id, error),
        }
    }

    pub fn set_loop(&mut self, id: u32, count: i32) {
        let typed_id = VideoId::new(id);
        let result = LoopMode::from_legacy(count)
            .map_err(|error| error.to_string())
            .and_then(|mode| {
                if self
                    .videos
                    .get(&typed_id)
                    .and_then(|video| video.native_id)
                    .is_none()
                {
                    return self.update_parked(typed_id, |manifest| manifest.with_loop_mode(mode));
                }
                let native_id = self.videos[&typed_id]
                    .native_id
                    .expect("checked active native video session");
                self.command(VideoCommand::Playback {
                    id: native_id.protocol(),
                    action: PlaybackAction::SetLoop(mode),
                })
            });
        if let Err(error) = result {
            self.fail(id, error);
        }
    }

    pub fn remove(&mut self, id: u32) {
        let id = VideoId::new(id);
        if let Some(native_id) = self.videos.get(&id).and_then(|video| video.native_id) {
            let _ = self.command(VideoCommand::Close {
                id: native_id.protocol(),
            });
            self.native_to_video.remove(&native_id);
        }
        self.videos.remove(&id);
    }

    fn update_parked(
        &mut self,
        id: VideoId,
        update: impl FnOnce(PlaybackRecoveryManifest) -> PlaybackRecoveryManifest,
    ) -> Result<(), String> {
        let video = self
            .videos
            .get_mut(&id)
            .ok_or_else(|| format!("video {} is not open", id.get()))?;
        let manifest = video
            .parked
            .take()
            .ok_or_else(|| format!("video {} has no active or parked session", id.get()))?;
        video.parked = Some(update(manifest));
        Ok(())
    }

    pub fn process_pending(
        &mut self,
        now: Instant,
        presented: &HashSet<VideoId>,
    ) -> &VideoServiceResult {
        let Some(mut system) = self.system.take() else {
            return &self.last_service;
        };
        for external_id in self.videos.keys().copied().collect::<Vec<_>>() {
            let result = if presented.contains(&external_id) {
                self.resume_presented(&mut system, external_id)
            } else {
                self.park_hidden(&mut system, external_id)
            };
            if let Err(error) = result {
                self.fail(external_id.get(), error);
            }
        }

        let native_result = system.service(now);
        let mut events = Vec::with_capacity(native_result.events.len());
        for event in native_result.events {
            let native_id = NativeVideoSessionId(event_id(&event));
            let Some(&external_id) = self.native_to_video.get(&native_id) else {
                continue;
            };
            let event = remap_event(event, external_id);
            self.observe_event(&event, &mut system, native_id);
            events.push(event);
        }

        let mut ready_frames = Vec::with_capacity(native_result.ready_frames.len());
        for ready in native_result.ready_frames {
            let native_id = NativeVideoSessionId(ready.id);
            let Some(&external_id) = self.native_to_video.get(&native_id) else {
                continue;
            };
            let draws = system.prepare_draws(std::iter::once(native_id.protocol()));
            let Some(frame) = draws.get(native_id.protocol()) else {
                continue;
            };
            let geometry = frame.geometry();
            if let Some(video) = self.videos.get_mut(&external_id) {
                video.width = geometry.display_width;
                video.height = geometry.display_height;
                video.frame_count = video.frame_count.saturating_add(1);
            }
            ready_frames.push(neomacs_video::VideoFrameReady {
                id: external_id,
                pts: ready.pts,
                transfer_path: ready.transfer_path,
            });
        }
        match self.gpu_accounting.observe(system.gpu_memory_bytes()) {
            VideoGpuAccountingChange::Unchanged => {}
            VideoGpuAccountingChange::Register(size_bytes) => {
                self.accounting
                    .push(crate::media_budget::MediaAccounting::Registered {
                        media_type: crate::media_budget::MediaType::Video,
                        id: VIDEO_GPU_POOL_ACCOUNTING_ID,
                        size_bytes,
                    });
            }
            VideoGpuAccountingChange::Free => {
                self.accounting
                    .push(crate::media_budget::MediaAccounting::Freed {
                        media_type: crate::media_budget::MediaType::Video,
                        id: VIDEO_GPU_POOL_ACCOUNTING_ID,
                    });
            }
        }
        self.system = Some(system);
        self.last_service = VideoServiceResult {
            events,
            ready_frames,
            next_deadline: native_result.next_deadline,
        };
        &self.last_service
    }

    fn resume_presented(
        &mut self,
        system: &mut VideoSystem,
        external_id: VideoId,
    ) -> Result<(), String> {
        if let Some(native_id) = self
            .videos
            .get(&external_id)
            .and_then(|video| video.native_id)
        {
            return system
                .set_presentation(native_id.protocol(), PresentationVisibility::Presented)
                .map_err(|error| error.to_string());
        }

        let Some(manifest) = self
            .videos
            .get_mut(&external_id)
            .and_then(|video| video.parked.take())
        else {
            return Ok(());
        };
        let native_id = self.allocate_native_id();
        let native_manifest = manifest
            .clone()
            .with_presentation(PresentationVisibility::Presented);
        if let Err(message) = system.open_from_manifest(native_id.protocol(), &native_manifest) {
            self.videos
                .get_mut(&external_id)
                .expect("parked video remains registered")
                .parked = Some(manifest);
            return Err(message.to_string());
        }

        self.native_to_video.insert(native_id, external_id);
        let video = self
            .videos
            .get_mut(&external_id)
            .expect("resumed video remains registered");
        video.native_id = Some(native_id);
        video.state = VideoState::Loading;
        Ok(())
    }

    fn park_hidden(
        &mut self,
        system: &mut VideoSystem,
        external_id: VideoId,
    ) -> Result<(), String> {
        let Some(native_id) = self
            .videos
            .get(&external_id)
            .and_then(|video| video.native_id)
        else {
            return Ok(());
        };

        let visibility_result =
            system.set_presentation(native_id.protocol(), PresentationVisibility::Hidden);
        let manifest = system
            .recovery_sessions()
            .into_iter()
            .find(|recovery| recovery.id() == native_id.protocol())
            .map(|recovery| {
                recovery
                    .into_manifest()
                    .with_presentation(PresentationVisibility::Hidden)
            });
        let close_result = system.command(VideoCommand::Close {
            id: native_id.protocol(),
        });
        self.native_to_video.remove(&native_id);
        if let Some(video) = self.videos.get_mut(&external_id) {
            video.native_id = None;
            video.parked = manifest;
        }

        visibility_result
            .and(close_result)
            .map_err(|error| error.to_string())
    }

    pub fn last_service(&self) -> &VideoServiceResult {
        &self.last_service
    }

    pub fn drain_accounting(&mut self) -> Vec<crate::media_budget::MediaAccounting> {
        std::mem::take(&mut self.accounting)
    }

    pub fn recovery_manifests(&self) -> Vec<VideoRecoveryManifest> {
        let mut manifests: Vec<_> = self
            .system
            .as_ref()
            .map_or_else(Vec::new, VideoSystem::recovery_sessions)
            .into_iter()
            .filter_map(|recovery| {
                let external_id = self
                    .native_to_video
                    .get(&NativeVideoSessionId(recovery.id()))?;
                let state = self.videos.get(external_id)?.state;
                Some(VideoRecoveryManifest {
                    id: *external_id,
                    playback: recovery.into_manifest(),
                    state,
                })
            })
            .collect();
        manifests.extend(self.videos.values().filter_map(|video| {
            Some(VideoRecoveryManifest {
                id: video.id,
                playback: video.parked.clone()?,
                state: video.state,
            })
        }));
        manifests
    }

    pub fn restore_after_device_loss(&mut self, manifests: Vec<VideoRecoveryManifest>) {
        let Some(mut system) = self.system.take() else {
            let message = self
                .initialization_error
                .clone()
                .unwrap_or_else(|| "native video subsystem is unavailable".into());
            for manifest in manifests {
                let external_id = manifest.id();
                self.next_id = self.next_id.max(external_id.get().saturating_add(1));
                self.videos.insert(
                    external_id,
                    CachedVideo {
                        id: external_id,
                        width: 0,
                        height: 0,
                        state: VideoState::Error,
                        frame_count: 0,
                        native_id: None,
                        parked: Some(manifest.playback),
                    },
                );
                self.fail(external_id.get(), message.clone());
            }
            return;
        };

        for manifest in manifests {
            let external_id = manifest.id();
            let is_presented =
                manifest.playback.presentation() == PresentationVisibility::Presented;
            self.next_id = self.next_id.max(external_id.get().saturating_add(1));
            self.videos.insert(
                external_id,
                CachedVideo {
                    id: external_id,
                    width: 0,
                    height: 0,
                    state: manifest.state,
                    frame_count: 0,
                    native_id: None,
                    parked: Some(manifest.playback),
                },
            );
            if is_presented && let Err(error) = self.resume_presented(&mut system, external_id) {
                self.fail(external_id.get(), error);
            }
        }
        self.system = Some(system);
    }

    fn observe_event(
        &mut self,
        event: &VideoEvent,
        system: &mut VideoSystem,
        native_id: NativeVideoSessionId,
    ) {
        match event {
            VideoEvent::Ready { id, width, height } => {
                if let Some(video) = self.videos.get_mut(id) {
                    video.width = *width;
                    video.height = *height;
                    video.state = system
                        .state(native_id.protocol())
                        .map_or(VideoState::Paused, VideoState::from);
                }
            }
            VideoEvent::StateChanged { id, state } => {
                if *state == VideoSessionState::Failed {
                    self.close_and_detach_failed_native_session(
                        system,
                        *id,
                        native_id,
                        "native video backend entered failed state".into(),
                    );
                } else if let Some(video) = self.videos.get_mut(id) {
                    video.state = (*state).into();
                }
            }
            VideoEvent::Ended { id } => {
                if let Some(video) = self.videos.get_mut(id) {
                    video.state = VideoState::EndOfStream;
                }
            }
            VideoEvent::Failed { id, error } => {
                self.close_and_detach_failed_native_session(
                    system,
                    *id,
                    native_id,
                    error.to_string(),
                );
            }
        }
    }

    fn close_and_detach_failed_native_session(
        &mut self,
        system: &mut VideoSystem,
        id: VideoId,
        native_id: NativeVideoSessionId,
        error: String,
    ) {
        // `VideoSystem` has already quiesced the native pipeline and retained
        // its failed session for diagnostics. Remove that ephemeral
        // incarnation now: a presented stable video must not try to resume a
        // decoder that terminal cleanup closed.
        if let Err(close_error) = system.command(VideoCommand::Close {
            id: native_id.protocol(),
        }) {
            tracing::debug!(
                video_id = id.get(),
                %close_error,
                "failed native video session was already detached"
            );
        }
        self.detach_failed_native_session(id, native_id, error);
    }

    fn detach_failed_native_session(
        &mut self,
        id: VideoId,
        native_id: NativeVideoSessionId,
        error: String,
    ) {
        if self.native_to_video.get(&native_id) == Some(&id) {
            self.native_to_video.remove(&native_id);
        }
        if let Some(video) = self.videos.get_mut(&id)
            && video.native_id == Some(native_id)
        {
            video.native_id = None;
            video.parked = None;
        }
        self.fail(id.get(), error);
    }

    fn fail(&mut self, id: u32, error: String) {
        tracing::error!(video_id = id, %error, "video playback failed");
        if let Some(video) = self.videos.get_mut(&VideoId::new(id)) {
            video.state = VideoState::Error;
        }
    }

    fn set_state(&mut self, id: VideoId, state: VideoState) {
        if let Some(video) = self.videos.get_mut(&id) {
            video.state = state;
        }
    }
}

fn event_id(event: &VideoEvent) -> VideoId {
    match event {
        VideoEvent::Ready { id, .. }
        | VideoEvent::StateChanged { id, .. }
        | VideoEvent::Ended { id }
        | VideoEvent::Failed { id, .. } => *id,
    }
}

fn remap_event(event: VideoEvent, id: VideoId) -> VideoEvent {
    match event {
        VideoEvent::Ready { width, height, .. } => VideoEvent::Ready { id, width, height },
        VideoEvent::StateChanged { state, .. } => VideoEvent::StateChanged { id, state },
        VideoEvent::Ended { .. } => VideoEvent::Ended { id },
        VideoEvent::Failed { error, .. } => VideoEvent::Failed { id, error },
    }
}

#[cfg(test)]
#[path = "video_cache_test.rs"]
mod tests;
