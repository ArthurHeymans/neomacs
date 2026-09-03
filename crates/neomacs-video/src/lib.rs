//! Cross-platform GPU-resident video playback.
//!
//! The public interface deliberately contains no GStreamer, CoreVideo,
//! Media Foundation, DMA-BUF, Metal, or Direct3D types. Platform-native frame
//! ownership and import stay behind the private platform seam.

mod backend;
mod clock;
mod color;
mod mailbox;
mod model;
mod platform;
mod sampling;
mod surface_pool;
mod system;

pub(crate) use model::FrameTiming;
pub use model::{
    BiPlanarVideoFormat, FrameImportPolicy, InitialPlayback, LoopMode, MediaTime,
    MissingVideoPlugin, MissingVideoPlugins, PackedVideoFormat, PixelAspectRatio, PixelRect,
    PlaybackAction, PlaybackEpoch, PlaybackRate, PresentationVisibility, VideoChromaLocation,
    VideoColorPrimaries, VideoColorRange, VideoColorimetry, VideoCommand, VideoCommandError,
    VideoCompositorImport, VideoDecodeBackend, VideoDecodeResidency, VideoDiagnostics, VideoEvent,
    VideoFrameFormat, VideoFrameLayoutError, VideoFramePath, VideoFrameReady, VideoGeometry,
    VideoGpuTiming, VideoGpuTimingStatus, VideoImportCounts, VideoInitError, VideoInstallerHint,
    VideoMatrixCoefficients, VideoModelError, VideoOpenRequest, VideoPlaneFormat,
    VideoPresentationCounts, VideoPresentationPath, VideoPresentationTiming, VideoRecoveryManifest,
    VideoRotation, VideoSampleKind, VideoSamplingTransform, VideoServiceRequest,
    VideoServiceResult, VideoServiceTiming, VideoSessionDiagnostics, VideoSessionRecovery,
    VideoSessionState, VideoSource, VideoSurfacePoolDiagnostics, VideoSurfacePoolRole,
    VideoTextureCoordinates, VideoTransferCharacteristic,
};
pub(crate) use sampling::GpuVideoFrame;
pub use sampling::{GpuGeneration, PreparedVideoDraw, PreparedVideoDraws, VideoSamplingResources};
pub use system::{VideoSystem, VideoWake};

#[cfg(test)]
#[path = "system_test.rs"]
mod system_test;

#[cfg(test)]
#[path = "surface_pool_test.rs"]
mod surface_pool_test;

#[cfg(test)]
#[path = "model_test.rs"]
mod model_test;

#[cfg(test)]
#[path = "sampling_test.rs"]
mod sampling_test;

#[cfg(test)]
#[path = "color_test.rs"]
mod color_test;

#[cfg(test)]
#[path = "mailbox_test.rs"]
mod mailbox_test;
