use super::{
    FrameImportPolicy, VideoCompositorImport, VideoDecodeResidency, VideoFramePath,
    VideoPresentationPath,
};

#[test]
fn frame_path_keeps_decoder_import_and_presentation_evidence_independent() {
    let path = VideoFramePath::new(
        VideoDecodeResidency::Unknown,
        VideoCompositorImport::BorrowedNativeSurface,
        VideoPresentationPath::WgpuComposited,
    );

    assert_eq!(path.decode_residency(), VideoDecodeResidency::Unknown);
    assert_eq!(
        path.compositor_import(),
        VideoCompositorImport::BorrowedNativeSurface
    );
    assert_eq!(path.presentation(), VideoPresentationPath::WgpuComposited);
}

#[test]
fn import_policy_considers_only_compositor_work() {
    assert!(
        FrameImportPolicy::RequireDirectSurface
            .permits(VideoCompositorImport::BorrowedNativeSurface)
    );
    assert!(!FrameImportPolicy::RequireDirectSurface.permits(VideoCompositorImport::GpuBlit));
    assert!(FrameImportPolicy::AllowGpuBlit.permits(VideoCompositorImport::GpuBlit));
    assert!(!FrameImportPolicy::AllowGpuBlit.permits(VideoCompositorImport::CpuUpload));
    assert!(FrameImportPolicy::AllowCpuUpload.permits(VideoCompositorImport::CpuUpload));
}
