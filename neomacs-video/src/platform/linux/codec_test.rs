use super::encode_supported_formats;
use neomacs_video_backend_abi as abi;

#[test]
fn renderer_features_bound_native_backend_negotiation() {
    assert_eq!(encode_supported_formats(wgpu::Features::empty()), 0);
    assert_eq!(
        encode_supported_formats(wgpu::Features::TEXTURE_FORMAT_NV12),
        abi::FORMAT_SUPPORT_NV12
    );
    assert_eq!(
        encode_supported_formats(
            wgpu::Features::TEXTURE_FORMAT_NV12 | wgpu::Features::TEXTURE_FORMAT_P010,
        ),
        abi::FORMAT_SUPPORT_NV12 | abi::FORMAT_SUPPORT_P010
    );
}
