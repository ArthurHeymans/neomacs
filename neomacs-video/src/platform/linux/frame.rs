use std::os::fd::{AsRawFd, OwnedFd};

use gstreamer as gst;

/// One plane of a DRM DMA-BUF surface. FDs are duplicated at the GStreamer
/// boundary so the lease remains valid independently of allocator internals.
pub(super) struct DmaBufPlane {
    pub(super) fd: OwnedFd,
    pub(super) stride: u32,
    pub(super) offset: u32,
}

pub(super) struct DmaBufSurface {
    pub(super) planes: Vec<DmaBufPlane>,
    pub(super) fourcc: u32,
    pub(super) modifier: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct DmaBufSurfaceKey {
    planes: Vec<DmaBufPlaneKey>,
    fourcc: u32,
    modifier: u64,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DmaBufPlaneKey {
    device: u64,
    inode: u64,
    stride: u32,
    offset: u32,
}

impl DmaBufSurface {
    /// Stable identity of one decoder-pool allocation. Duplicated descriptors
    /// for the same DMA-BUF retain the same device/inode pair, unlike raw FD
    /// numbers, which the process may recycle immediately.
    pub(super) fn cache_key(&self, width: u32, height: u32) -> Result<DmaBufSurfaceKey, String> {
        let planes = self
            .planes
            .iter()
            .map(|plane| {
                let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
                if unsafe { libc::fstat(plane.fd.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
                    return Err(format!(
                        "failed to identify DMA-BUF plane: {}",
                        std::io::Error::last_os_error()
                    ));
                }
                let stat = unsafe { stat.assume_init() };
                Ok(DmaBufPlaneKey {
                    device: stat.st_dev,
                    inode: stat.st_ino,
                    stride: plane.stride,
                    offset: plane.offset,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(DmaBufSurfaceKey {
            planes,
            fourcc: self.fourcc,
            modifier: self.modifier,
            width,
            height,
        })
    }
}

pub(super) struct CpuPackedSurface {
    pub(super) bytes: Vec<u8>,
    pub(super) stride: u32,
}

pub(super) enum LinuxFrameStorage {
    DmaBuf(DmaBufSurface),
    CpuPacked(CpuPackedSurface),
}

/// Affine native lease. Retaining `sample` keeps decoder pool ownership and
/// implicit producer synchronization alive until the GPU frame is retired.
pub(crate) struct LinuxFrameLease {
    pub(super) _sample: gst::Sample,
    pub(super) storage: LinuxFrameStorage,
    pub(super) transfer_path: crate::VideoTransferPath,
}
