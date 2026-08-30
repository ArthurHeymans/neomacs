use crate::backend::{DecodedFrame, FrameImportOutcome, FrameImporter, ImportedFrame};
use crate::sampling::{GpuVideoContext, PreparedSampledTexture};
use crate::surface_pool::{BoundedSurfacePool, SurfacePoolAcquire};
use crate::{GpuVideoFrame, VideoTransferPath};

use super::dmabuf::{ImportedDmaBufSurface, import_packed_dmabuf};
use super::frame::{DmaBufSurfaceKey, LinuxFrameLease, LinuxFrameStorage};

const IMPORTED_SURFACE_CAPACITY: usize = 64;

struct CachedImportedSurface {
    prepared: PreparedSampledTexture,
    imported: ImportedDmaBufSurface,
}

pub(crate) struct LinuxFrameImporter {
    gpu: GpuVideoContext,
    imported: BoundedSurfacePool<DmaBufSurfaceKey, CachedImportedSurface>,
}

impl LinuxFrameImporter {
    pub(super) fn new(gpu: GpuVideoContext) -> Self {
        Self {
            gpu,
            imported: BoundedSurfacePool::new(IMPORTED_SURFACE_CAPACITY),
        }
    }
}

impl FrameImporter<LinuxFrameLease> for LinuxFrameImporter {
    type Sampled = GpuVideoFrame;

    fn transfer_path(&self, frame: &DecodedFrame<LinuxFrameLease>) -> VideoTransferPath {
        frame.lease.transfer_path
    }

    fn import(
        &mut self,
        frame: DecodedFrame<LinuxFrameLease>,
    ) -> Result<FrameImportOutcome<Self::Sampled>, String> {
        let DecodedFrame {
            lease,
            geometry,
            sampling,
            ..
        } = frame;
        match &lease.storage {
            LinuxFrameStorage::DmaBuf(surface) => {
                let path = lease.transfer_path;
                let key = surface.cache_key(geometry.coded_width, geometry.coded_height)?;
                let cached = match self.imported.acquire(key) {
                    SurfacePoolAcquire::Reused(lease) => lease,
                    SurfacePoolAcquire::Allocate(reservation) => {
                        let (texture, imported) = import_packed_dmabuf(
                            self.gpu.device(),
                            surface,
                            geometry.coded_width,
                            geometry.coded_height,
                        )?;
                        let prepared = self
                            .gpu
                            .prepare_texture(texture, sampling.allocation_bytes(geometry)?);
                        reservation.fulfill(CachedImportedSurface { prepared, imported })
                    }
                    SurfacePoolAcquire::Backpressured => {
                        return Ok(FrameImportOutcome::Backpressured);
                    }
                };
                cached
                    .value()
                    .imported
                    .acquire(self.gpu.device(), self.gpu.queue())?;
                let prepared = cached.value().prepared.clone();
                let release = cached.value().imported.release();
                let sampled = self.gpu.wrap_prepared_texture_with_release(
                    geometry,
                    prepared,
                    release,
                    (lease, cached),
                );
                Ok(FrameImportOutcome::Ready(ImportedFrame { sampled, path }))
            }
            LinuxFrameStorage::CpuPacked(surface) => {
                let sampled =
                    self.gpu
                        .upload_rgba(geometry, sampling, &surface.bytes, surface.stride)?;
                Ok(FrameImportOutcome::Ready(ImportedFrame {
                    sampled,
                    path: VideoTransferPath::CpuUpload,
                }))
            }
        }
    }
}
