use neomacs_display_protocol::types::VideoId;
use std::time::Instant;

use crate::sampling::GpuVideoContext;
use crate::system::VideoWake;
use crate::{
    FrameTiming, VideoColorimetry, VideoCommand, VideoCommandError, VideoFrameFormat,
    VideoGeometry, VideoInitError, VideoSessionState, VideoTransferPath,
};

/// Decoder output with all information needed to replay the exact frame.
pub(crate) struct DecodedFrame<F> {
    pub(crate) lease: F,
    pub(crate) timing: FrameTiming,
    pub(crate) geometry: VideoGeometry,
    pub(crate) format: VideoFrameFormat,
    pub(crate) colorimetry: VideoColorimetry,
}

/// Event emitted by a platform decoder adapter.
pub(crate) enum BackendEvent<F> {
    Opened {
        id: VideoId,
        width: u32,
        height: u32,
        initial_state: VideoSessionState,
    },
    Frame {
        id: VideoId,
        frame: DecodedFrame<F>,
    },
    /// Frames replaced in the native latest-frame slot before the compositor
    /// serviced it. This aggregates instead of turning diagnostic accounting
    /// into an unbounded per-frame control queue.
    #[cfg(any(target_os = "linux", test))]
    FramesReplaced {
        id: VideoId,
        count: u64,
    },
    StateChanged {
        id: VideoId,
        state: VideoSessionState,
    },
    /// Native loop handling consumed one replay permission.
    Looped {
        id: VideoId,
        remaining: crate::LoopMode,
    },
    Ended {
        id: VideoId,
    },
    Failed {
        id: VideoId,
        error: VideoCommandError,
    },
}

impl<F> BackendEvent<F> {
    pub(crate) const fn id(&self) -> VideoId {
        match self {
            Self::Opened { id, .. }
            | Self::Frame { id, .. }
            | Self::StateChanged { id, .. }
            | Self::Looped { id, .. }
            | Self::Ended { id }
            | Self::Failed { id, .. } => *id,
            #[cfg(any(target_os = "linux", test))]
            Self::FramesReplaced { id, .. } => *id,
        }
    }
}

pub(crate) trait DecoderBackend {
    type Frame;

    fn command(&mut self, command: VideoCommand) -> Result<(), VideoCommandError>;
    fn drain_events(&mut self) -> Vec<BackendEvent<Self::Frame>>;

    /// Earliest time at which the platform adapter needs another service
    /// pass even when it has not published a cross-thread wake event.
    ///
    /// Most decoders push frame availability and therefore return `None`.
    /// Pull-based native APIs use this seam to participate in the common
    /// render scheduler instead of installing an independent display clock.
    fn next_service_deadline(&self, _now: Instant) -> Option<Instant> {
        None
    }
}

pub(crate) struct ImportedFrame<S> {
    pub(crate) sampled: S,
    pub(crate) path: VideoTransferPath,
}

/// Result of attempting to transfer one decoded frame into compositor-owned
/// GPU state. Backpressure is an expected bounded-resource condition: the
/// caller drops this already-stale decoded frame and waits for the next one
/// instead of poisoning the playback session.
pub(crate) enum FrameImportOutcome<S> {
    Ready(ImportedFrame<S>),
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    Backpressured,
}

pub(crate) trait FrameImporter<F> {
    type Sampled;

    /// Classify the transfer before performing native import, mapping, copy,
    /// or upload work. The policy boundary uses this to reject forbidden
    /// fallback paths without causing their side effects first.
    fn transfer_path(&self, frame: &DecodedFrame<F>) -> VideoTransferPath;

    fn import(
        &mut self,
        frame: DecodedFrame<F>,
    ) -> Result<FrameImportOutcome<Self::Sampled>, String>;
}

pub(crate) trait Platform {
    const BACKEND: crate::VideoDecodeBackend;
    type Frame;
    type Sampled;
    type Decoder: DecoderBackend<Frame = Self::Frame>;
    type Importer: FrameImporter<Self::Frame, Sampled = Self::Sampled>;
}

pub(crate) trait ProductionPlatform: Platform {
    fn create(
        gpu: GpuVideoContext,
        policy: crate::FrameTransferPolicy,
        wake: VideoWake,
    ) -> Result<(Self::Decoder, Self::Importer), VideoInitError>;
}
