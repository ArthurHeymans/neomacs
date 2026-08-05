use super::{
    FontconfigSubpixelOrder, GlyphAtlasError, GlyphKey, RasterizeResult, SingleCharGlyph,
    SubpixelBin, WgpuGlyphAtlas, effective_font_size, glyph_font_identity,
    key_uses_default_font_metrics, normalize_subpixel_mask, rasterize_missing_glyph_box,
};
use neomacs_display_protocol::types::FaceId;

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
fn missing_glyph_box_uses_layout_advance_and_face_line_metrics() {
    let result = rasterize_missing_glyph_box(5.0, 10.0, 7.0, 2.0);

    assert_eq!(result.width, 10);
    assert_eq!(result.height, 20);
    assert_eq!(result.advance_width, 10.0);
    assert_eq!(result.bearing_y, 14.0);
    assert_eq!(result.pixel_data.len(), 200);
    for y in 0..result.height {
        for x in 0..result.width {
            let expected = if x == 0 || x + 1 == result.width || y == 0 || y + 1 == result.height {
                255
            } else {
                0
            };
            assert_eq!(result.pixel_data[(y * result.width + x) as usize], expected);
        }
    }
}

#[test]
fn default_metrics_ignore_nondefault_face_zero_font_size() {
    let key = GlyphKey {
        charcode: 'F' as u32,
        face_id: FaceId::new(0),
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
        face_id: FaceId::new(0),
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
        face_id: FaceId::new(0),
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

    let mut a = Face::new(FaceId::new(5));
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

fn try_test_atlas() -> Option<WgpuGlyphAtlas> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    let (device, _) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("glyph-atlas-font-boundary-test"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: Default::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        trace: wgpu::Trace::Off,
    }))
    .ok()?;
    Some(WgpuGlyphAtlas::new(&device))
}

#[cfg(unix)]
#[test]
fn renderer_keeps_missing_ascii_on_primary_font() {
    use neomacs_display_protocol::face::Face;
    use neomacs_display_protocol::font::{
        FontResolutionSource, FontSlantKind, ResolvedFont, ResolvedFontId,
    };
    use neomacs_layout_engine::font::metrics::FontMetricsService;

    let Some(resolved) = FontMetricsService::new().resolved_font_for_face(
        "Symbols Nerd Font Mono",
        400,
        false,
        10.0,
    ) else {
        return;
    };
    if !resolved
        .family
        .eq_ignore_ascii_case("Symbols Nerd Font Mono")
    {
        return;
    }
    let Some(mut atlas) = try_test_atlas() else {
        return;
    };
    let family = resolved.family;
    let weight = resolved.weight;
    let identity = resolved.identity;
    let id = ResolvedFontId(1);
    let font = ResolvedFont {
        id,
        identity: identity.clone(),
        family,
        full_name: None,
        postscript_name: identity.postscript_name.clone(),
        weight,
        slant: FontSlantKind::Normal,
        width: 5,
        pixel_size: 10.0,
        ascent_px: 8.0,
        descent_px: 2.0,
        space_advance_px: 5.0,
        source: FontResolutionSource::FacePrimary,
    };
    atlas.install_frame_fonts(
        &[(id, font)].into_iter().collect(),
        &Default::default(),
        &Default::default(),
    );
    let mut face = Face::new(FaceId::new(7));
    face.font_family = "Symbols Nerd Font Mono".to_string();
    face.font_size = 10.0;
    face.font_ascent = 8;
    face.font_descent = 2;
    face.default_resolved_font_id = Some(id);

    assert!(matches!(
        atlas.try_fast_single_char_glyph(' ', Some(&face)),
        Some(SingleCharGlyph::MissingPrimaryAscii { advance_width: 5.0 })
    ));
    let result = atlas
        .rasterize_glyph(
            ' ',
            Some(&face),
            SubpixelBin::Zero,
            SubpixelBin::Zero,
            false,
        )
        .expect("missing ASCII renders GNU's empty box");
    assert_eq!(result.width, 5);
    assert_eq!(result.advance_width, 5.0);
    assert_eq!(result.height, 10);
    assert!(result.pixel_data.contains(&255));
}

#[cfg(unix)]
#[test]
#[tracing_test::traced_test]
fn renderer_replays_named_instance_weight_on_the_exact_raw_face() {
    use cosmic_text::{Buffer, Metrics, Shaping};
    use neomacs_display_protocol::font::{
        FontResolutionSource, FontSlantKind, ResolvedFont, ResolvedFontId,
    };
    use neomacs_layout_engine::font::metrics::FontMetricsService;

    let Some(resolved) =
        FontMetricsService::new().resolved_font_for_face("Noto Sans", 700, false, 18.0)
    else {
        tracing::info!("skipping: Noto Sans Bold is not installed");
        return;
    };
    if resolved.identity.file_path.is_none() {
        tracing::info!("skipping: Fontconfig match has no file");
        return;
    }
    let Some(mut atlas) = try_test_atlas() else {
        tracing::info!("skipping: no headless wgpu adapter");
        return;
    };
    let family = resolved.family;
    let weight = resolved.weight;
    let identity = resolved.identity;
    let font = ResolvedFont {
        id: ResolvedFontId(1),
        identity: identity.clone(),
        family,
        full_name: None,
        postscript_name: identity.postscript_name.clone(),
        weight,
        slant: FontSlantKind::Normal,
        width: 5,
        pixel_size: 18.0,
        ascent_px: 0.0,
        descent_px: 0.0,
        space_advance_px: 0.0,
        source: FontResolutionSource::FacePrimary,
    };

    let attrs = atlas
        .exact_attrs_for_resolved_font(&font)
        .expect("renderer must open the layout-resolved face");
    let mut buffer = Buffer::new(&mut atlas.font_system, Metrics::new(18.0, 24.0));
    buffer.set_size(&mut atlas.font_system, Some(72.0), Some(36.0));
    buffer.set_text(&mut atlas.font_system, "M", &attrs, Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut atlas.font_system, false);
    let cache_key = buffer
        .layout_runs()
        .find_map(|run| run.glyphs.first())
        .expect("shaped glyph")
        .physical((0.0, 0.0), 1.0)
        .cache_key;
    let face = atlas
        .font_system
        .db()
        .face(cache_key.font_id)
        .expect("renderer fontdb face");

    assert_eq!(face.index, identity.file_face_index());
    assert_eq!(cache_key.font_weight.0, 700);
    assert!(atlas.render_cache_key_image(cache_key, false).is_some());
}

#[test]
fn renderer_exact_attrs_reject_an_unopenable_identity() {
    use neomacs_display_protocol::font::{
        FontResolutionSource, FontSlantKind, ResolvedFont, ResolvedFontId, ResolvedFontIdentity,
    };

    let Some(mut atlas) = try_test_atlas() else {
        return;
    };
    let font = ResolvedFont {
        id: ResolvedFontId(1),
        identity: ResolvedFontIdentity::from_file("/neomacs/missing/font.ttf", 0, None),
        family: "missing".to_string(),
        full_name: None,
        postscript_name: None,
        weight: 400,
        slant: FontSlantKind::Normal,
        width: 5,
        pixel_size: 14.0,
        ascent_px: 0.0,
        descent_px: 0.0,
        space_advance_px: 0.0,
        source: FontResolutionSource::FacePrimary,
    };

    assert!(atlas.exact_attrs_for_resolved_font(&font).is_none());
}

#[test]
fn reused_resolved_font_id_invalidates_renderer_identity_caches() {
    use neomacs_display_protocol::font::{
        FontResolutionSource, FontSlantKind, ResolvedFont, ResolvedFontId, ResolvedFontIdentity,
        ResolvedFontTable,
    };

    let Some(mut atlas) = try_test_atlas() else {
        return;
    };
    let id = ResolvedFontId(9);
    let font = |path: &str| ResolvedFont {
        id,
        identity: ResolvedFontIdentity::from_file(path, 0, None),
        family: "Fixture".to_string(),
        full_name: None,
        postscript_name: None,
        weight: 400,
        slant: FontSlantKind::Normal,
        width: 5,
        pixel_size: 14.0,
        ascent_px: 0.0,
        descent_px: 0.0,
        space_advance_px: 0.0,
        source: FontResolutionSource::FacePrimary,
    };
    let mut first = ResolvedFontTable::new();
    first.insert(id, font("/fonts/first.ttf"));
    atlas.install_frame_fonts(&first, &Default::default(), &Default::default());
    atlas.resolved_fontdb_ids.insert(id, None);

    let mut replacement = ResolvedFontTable::new();
    replacement.insert(id, font("/fonts/replacement.ttf"));
    atlas.install_frame_fonts(&replacement, &Default::default(), &Default::default());

    assert!(!atlas.resolved_fontdb_ids.contains_key(&id));
    assert_eq!(
        atlas.frame_fonts.get(&id).unwrap().identity,
        replacement.get(&id).unwrap().identity
    );
}
