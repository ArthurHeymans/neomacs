use super::{
    DmaBufMemoryLayout, PipelineDrmIdentity, PipelineDrmTopology, dma_buf_transfer_path,
    rotation_from_gstreamer_tag, sampling_from_fourcc,
};
use crate::sampling::LinuxDrmDevice;
use crate::{LoopMode, VideoRotation, VideoTransferPath};
use std::num::NonZeroU32;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use super::super::frame::{DmaBufPlane, DmaBufSurface};

#[test]
fn finite_loop_count_means_additional_replays() {
    let mut mode = LoopMode::Count(NonZeroU32::new(2).unwrap());
    assert!(mode.consume_replay());
    assert_eq!(mode, LoopMode::Count(NonZeroU32::new(1).unwrap()));
    assert!(mode.consume_replay());
    assert_eq!(mode, LoopMode::Off);
    assert!(!mode.consume_replay());
}

#[test]
fn dmabuf_plane_memory_layout_accepts_shared_or_complete_descriptors_only() {
    let shared = DmaBufMemoryLayout::classify(1, 3).unwrap();
    assert_eq!(shared.memory_index(0), 0);
    assert_eq!(shared.memory_index(2), 0);

    let per_plane = DmaBufMemoryLayout::classify(3, 3).unwrap();
    assert_eq!(per_plane.memory_index(0), 0);
    assert_eq!(per_plane.memory_index(2), 2);

    assert!(DmaBufMemoryLayout::classify(2, 3).is_err());
}

#[test]
fn dmabuf_cache_identity_survives_descriptor_duplication() {
    let mut fds = [-1; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let _write = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let duplicate = unsafe { libc::dup(read.as_raw_fd()) };
    assert!(duplicate >= 0);
    let duplicate = unsafe { OwnedFd::from_raw_fd(duplicate) };

    let first = DmaBufSurface {
        planes: vec![DmaBufPlane {
            fd: read,
            stride: 256,
            offset: 16,
        }],
        fourcc: 0x3432_5241,
        modifier: 7,
    };
    let second = DmaBufSurface {
        planes: vec![DmaBufPlane {
            fd: duplicate,
            stride: 256,
            offset: 16,
        }],
        fourcc: 0x3432_5241,
        modifier: 7,
    };

    assert_eq!(
        first.cache_key(64, 32).unwrap(),
        second.cache_key(64, 32).unwrap()
    );
    assert_ne!(
        first.cache_key(64, 32).unwrap(),
        second.cache_key(128, 32).unwrap(),
        "geometry participates in the imported image identity"
    );
}

#[test]
fn gstreamer_orientation_tag_enters_the_common_sampling_transform() {
    assert_eq!(
        rotation_from_gstreamer_tag("rotate-90"),
        VideoRotation::Clockwise90
    );
    assert_eq!(
        rotation_from_gstreamer_tag("rotate-270"),
        VideoRotation::Clockwise270
    );
    assert_eq!(
        rotation_from_gstreamer_tag("flip-rotate-90"),
        VideoRotation::None,
        "mirrored orientation must not be mislabeled as a pure rotation"
    );
}

#[test]
fn packed_dmabuf_is_an_interop_path_even_on_the_same_physical_gpu() {
    let renderer = LinuxDrmDevice::from_device_numbers(226, 128);
    let same_decoder = LinuxDrmDevice::from_device_numbers(226, 128);
    let other_decoder = LinuxDrmDevice::from_device_numbers(226, 129);

    assert_eq!(
        dma_buf_transfer_path(
            Some(renderer),
            PipelineDrmTopology {
                decoder: PipelineDrmIdentity::Single(same_decoder),
                surface_path: PipelineDrmIdentity::Single(same_decoder),
                inspection_failed: false,
            },
        )
        .unwrap(),
        VideoTransferPath::GpuInteropCopy,
        "the packed sink can require a native GPU colorspace conversion"
    );
    assert!(
        dma_buf_transfer_path(
            Some(renderer),
            PipelineDrmTopology {
                decoder: PipelineDrmIdentity::Single(other_decoder),
                surface_path: PipelineDrmIdentity::Single(other_decoder),
                inspection_failed: false,
            },
        )
        .is_err(),
        "a proven cross-adapter DMA-BUF must not be imported by the renderer"
    );
    assert!(
        dma_buf_transfer_path(
            Some(renderer),
            PipelineDrmTopology {
                decoder: PipelineDrmIdentity::Conflict,
                surface_path: PipelineDrmIdentity::Conflict,
                inspection_failed: false,
            },
        )
        .is_err(),
        "a pipeline that reports multiple DRM devices is a proven conflict"
    );
    assert_eq!(
        dma_buf_transfer_path(Some(renderer), PipelineDrmTopology::UNKNOWN).unwrap(),
        VideoTransferPath::GpuInteropCopy
    );
    assert_eq!(
        dma_buf_transfer_path(
            None,
            PipelineDrmTopology {
                decoder: PipelineDrmIdentity::Single(same_decoder),
                surface_path: PipelineDrmIdentity::Single(same_decoder),
                inspection_failed: false,
            },
        )
        .unwrap(),
        VideoTransferPath::GpuInteropCopy
    );

    assert!(
        dma_buf_transfer_path(
            Some(renderer),
            PipelineDrmTopology {
                decoder: PipelineDrmIdentity::Single(same_decoder),
                surface_path: PipelineDrmIdentity::Conflict,
                inspection_failed: false,
            },
        )
        .is_err(),
        "a same-GPU decoder cannot hide a cross-GPU packed-surface producer"
    );
    assert!(
        dma_buf_transfer_path(
            Some(renderer),
            PipelineDrmTopology {
                decoder: PipelineDrmIdentity::Single(same_decoder),
                surface_path: PipelineDrmIdentity::Single(same_decoder),
                inspection_failed: true,
            },
        )
        .is_err(),
        "an incomplete topology inspection must fail closed"
    );
}

#[test]
fn non_drm_character_devices_are_not_physical_gpu_identities() {
    assert!(LinuxDrmDevice::from_path(std::path::Path::new("/dev/null")).is_none());
}

#[test]
fn opaque_xrgb_dmabufs_do_not_enter_the_alpha_sampling_pipeline() {
    assert_eq!(
        sampling_from_fourcc(0x3432_5241).unwrap(),
        crate::VideoSampling::Bgra8
    );
    assert_eq!(
        sampling_from_fourcc(0x3432_4241).unwrap(),
        crate::VideoSampling::Rgba8
    );
    assert!(sampling_from_fourcc(0x3432_5258).is_err());
    assert!(sampling_from_fourcc(0x3432_4258).is_err());
}
