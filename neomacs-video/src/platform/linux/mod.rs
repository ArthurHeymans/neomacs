mod codec;
mod decoder;
mod dmabuf;
mod frame;
mod importer;
mod loader;

#[cfg(test)]
#[path = "dynamic_backend_test.rs"]
mod dynamic_backend_test;

use crate::backend::{Platform, ProductionPlatform};
use crate::sampling::GpuVideoContext;
use crate::{FrameTransferPolicy, GpuVideoFrame, VideoDecodeBackend, VideoInitError, VideoWake};

use decoder::GstreamerDecoder;
use frame::LinuxFrameLease;
use importer::LinuxFrameImporter;

pub(crate) struct LinuxPlatform;

impl Platform for LinuxPlatform {
    const BACKEND: VideoDecodeBackend = VideoDecodeBackend::GStreamer;
    type Frame = LinuxFrameLease;
    type Sampled = GpuVideoFrame;
    type Decoder = GstreamerDecoder;
    type Importer = LinuxFrameImporter;
}

impl ProductionPlatform for LinuxPlatform {
    fn create(
        gpu: GpuVideoContext,
        policy: FrameTransferPolicy,
        wake: VideoWake,
    ) -> Result<(Self::Decoder, Self::Importer), VideoInitError> {
        let renderer_drm_device = gpu.linux_render_device();
        let renderer_features = gpu.device().features();
        let decoder = GstreamerDecoder::new(wake, policy, renderer_drm_device, renderer_features)
            .map_err(|message| VideoInitError::Backend {
            backend: VideoDecodeBackend::GStreamer,
            message,
        })?;
        Ok((decoder, LinuxFrameImporter::new(gpu)))
    }
}
