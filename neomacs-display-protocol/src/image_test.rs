use super::{ImageColorContext, ImageRgb};

#[test]
fn image_rgb_preserves_black_as_a_real_opaque_color() {
    let black = ImageRgb::from_pixel(0x0000_0000);

    assert_eq!(black.rgb24(), 0x0000_0000);
    assert_eq!(black.rgba8(), [0, 0, 0, 0xff]);
}

#[test]
fn image_color_context_keeps_foreground_and_background_roles_distinct() {
    let colors = ImageColorContext::from_pixels(0xaa_12_34_56, 0xbb_65_43_21);

    assert_eq!(colors.foreground().rgb24(), 0x12_34_56);
    assert_eq!(colors.background().rgb24(), 0x65_43_21);
}

#[test]
fn unresolved_image_color_context_preserves_the_visible_monochrome_fallback() {
    let colors = ImageColorContext::default();

    assert_eq!(colors.foreground().rgb24(), 0x00ff_ffff);
    assert_eq!(colors.background().rgb24(), 0x0000_0000);
}
