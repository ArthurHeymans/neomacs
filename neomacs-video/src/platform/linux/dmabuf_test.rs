use super::packed_format;

#[test]
fn vulkan_import_accepts_alpha_packed_formats_only() {
    assert!(packed_format(0x3432_5241).is_some()); // AR24
    assert!(packed_format(0x3432_4241).is_some()); // AB24
    assert!(packed_format(0x3432_5258).is_none()); // XR24
    assert!(packed_format(0x3432_4258).is_none()); // XB24
}
