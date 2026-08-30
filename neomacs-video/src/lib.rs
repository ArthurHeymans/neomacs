//! Cross-platform GPU-resident video playback.
//!
//! The public interface deliberately contains no GStreamer, CoreVideo,
//! Media Foundation, DMA-BUF, Metal, or Direct3D types. Platform-native frame
//! ownership and import stay behind the private platform seam.

mod backend;
mod clock;
mod mailbox;
mod model;
mod platform;
mod sampling;
mod surface_pool;
mod system;

pub(crate) use model::{FrameTiming, VideoSampling};
pub use model::{
    FrameTransferPolicy, InitialPlayback, LoopMode, MediaTime, PixelAspectRatio, PixelRect,
    PlaybackAction, PlaybackEpoch, PlaybackRate, PresentationVisibility, VideoCommand,
    VideoCommandError, VideoDecodeBackend, VideoDiagnostics, VideoEvent, VideoFrameReady,
    VideoGeometry, VideoInitError, VideoModelError, VideoRecoveryManifest, VideoRotation,
    VideoSamplingTransform, VideoServiceResult, VideoSessionDiagnostics, VideoSessionRecovery,
    VideoSessionState, VideoSource, VideoTextureCoordinates, VideoTransferPath,
};
pub(crate) use sampling::GpuVideoFrame;
pub use sampling::{GpuGeneration, PreparedVideoDraw, PreparedVideoDraws};
pub use system::{VideoSystem, VideoWake};

#[cfg(test)]
#[path = "system_test.rs"]
mod system_test;

#[cfg(test)]
#[path = "backend_test.rs"]
mod backend_test;

#[cfg(test)]
#[path = "surface_pool_test.rs"]
mod surface_pool_test;

#[cfg(test)]
#[path = "model_test.rs"]
mod model_test;

#[cfg(test)]
#[path = "sampling_test.rs"]
mod sampling_test;
