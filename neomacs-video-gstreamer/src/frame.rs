use std::os::fd::OwnedFd;

use gstreamer as gst;

pub(crate) struct DmaBufPlane {
    pub(crate) fd: OwnedFd,
    pub(crate) stride: u32,
    pub(crate) offset: u32,
}

pub(crate) struct DmaBufSurface {
    pub(crate) planes: Vec<DmaBufPlane>,
    pub(crate) fourcc: u32,
    pub(crate) modifier: u64,
}

pub(crate) struct CpuPackedSurface {
    pub(crate) bytes: Vec<u8>,
    pub(crate) stride: u32,
}

pub(crate) enum LinuxFrameStorage {
    DmaBuf(DmaBufSurface),
    CpuPacked(CpuPackedSurface),
}

pub(crate) struct LinuxFrameLease {
    pub(crate) _sample: gst::Sample,
    pub(crate) storage: LinuxFrameStorage,
    pub(crate) transfer_path: crate::VideoTransferPath,
}
