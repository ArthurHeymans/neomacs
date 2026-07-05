use super::{
    FontconfigSubpixelOrder, GlyphAtlasError, GlyphKey, RasterizeResult, SubpixelBin,
    WgpuGlyphAtlas, effective_font_size, glyph_font_identity, key_uses_default_font_metrics,
    normalize_subpixel_mask,
};

#[test]
fn normalize_subpixel_mask_preserves_rgb_order() {
    let out = normalize_subpixel_mask(&[10, 20, 30], 1, FontconfigSubpixelOrder::Rgb);
    assert_eq!(out, vec![10, 20, 30, 30]);
}

#[test]
fn normalize_subpixel_mask_swaps_bgr_order() {
    let out = normalize_subpixel_mask(&[10, 20, 30], 1, FontconfigSubpixelOrder::Bgr);
    assert_eq!(out, vec![30, 20, 10, 30]);
}

#[test]
fn default_metrics_ignore_nondefault_face_zero_font_size() {
    let key = GlyphKey {
        charcode: 'F' as u32,
        face_id: 0,
        font_size_bits: 27.0_f32.to_bits(),
        font_identity: 0,
        x_bin: SubpixelBin::Zero,
        y_bin: SubpixelBin::Zero,
    };

    assert!(!key_uses_default_font_metrics(&key, 13.0));
}

#[test]
fn default_metrics_accept_unspecified_default_font_size() {
    let key = GlyphKey {
        charcode: 'F' as u32,
        face_id: 0,
        font_size_bits: 0.0_f32.to_bits(),
        font_identity: 0,
        x_bin: SubpixelBin::Zero,
        y_bin: SubpixelBin::Zero,
    };

    assert!(key_uses_default_font_metrics(&key, 13.0));
}

#[test]
fn default_metrics_accept_explicit_default_font_size() {
    let key = GlyphKey {
        charcode: 'F' as u32,
        face_id: 0,
        font_size_bits: 13.05_f32.to_bits(),
        font_identity: 0,
        x_bin: SubpixelBin::Zero,
        y_bin: SubpixelBin::Zero,
    };

    assert!(key_uses_default_font_metrics(&key, 13.0));
}

#[test]
fn effective_font_size_resolves_zero_sentinel_to_default() {
    // An explicit, positive size is honored verbatim.
    assert_eq!(effective_font_size(Some(27.0), 13.0), 27.0);
    // font_size 0.0 is the "unspecified" sentinel (see
    // key_uses_default_font_metrics): a face that inherits the frame default
    // font (minibuffer/echo-area) carries it, and it MUST resolve to the
    // default. Feeding 0 into cosmic-text's Metrics panics ("line height
    // cannot be 0").
    assert_eq!(effective_font_size(Some(0.0), 13.0), 13.0);
    // A degenerate (negative / non-finite) size is equally unusable and falls
    // back to the default.
    assert_eq!(effective_font_size(Some(-5.0), 13.0), 13.0);
    assert_eq!(effective_font_size(Some(f32::NAN), 13.0), 13.0);
    // A missing face resolves to the default as before.
    assert_eq!(effective_font_size(None, 13.0), 13.0);
}

#[test]
fn rasterize_result_to_pixels_rejects_mismatched_alpha_length() {
    let result = RasterizeResult {
        width: 2,
        height: 2,
        pixel_data: vec![255],
        bearing_x: 0.0,
        bearing_y: 0.0,
        is_color: false,
        is_subpixel: false,
        advance_width: 0.0,
    };

    let err = WgpuGlyphAtlas::rasterize_result_to_pixels(&result).unwrap_err();
    assert!(matches!(
        err,
        GlyphAtlasError::PixelDataLength {
            expected: 4,
            actual: 1,
            ..
        }
    ));
}

#[test]
fn rasterize_result_to_pixels_rejects_zero_size() {
    let result = RasterizeResult {
        width: 0,
        height: 2,
        pixel_data: Vec::new(),
        bearing_x: 0.0,
        bearing_y: 0.0,
        is_color: false,
        is_subpixel: false,
        advance_width: 0.0,
    };

    let err = WgpuGlyphAtlas::rasterize_result_to_pixels(&result).unwrap_err();
    assert_eq!(err, GlyphAtlasError::ZeroSize);
}

#[test]
fn glyph_font_identity_discriminates_resolved_font_id() {
    use neomacs_display_protocol::face::Face;
    use neomacs_display_protocol::font::ResolvedFontId;

    let mut a = Face::new(5);
    a.font_family = "Mono".to_string();
    let mut b = a.clone();

    // Same request fields, different realized fonts -> different identity.
    a.default_resolved_font_id = Some(ResolvedFontId(1));
    b.default_resolved_font_id = Some(ResolvedFontId(2));
    assert_ne!(glyph_font_identity(Some(&a)), glyph_font_identity(Some(&b)));

    // Same realized font -> same identity.
    b.default_resolved_font_id = Some(ResolvedFontId(1));
    assert_eq!(glyph_font_identity(Some(&a)), glyph_font_identity(Some(&b)));

    // Unresolved differs from resolved.
    b.default_resolved_font_id = None;
    assert_ne!(glyph_font_identity(Some(&a)), glyph_font_identity(Some(&b)));
}
