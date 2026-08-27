use super::*;

fn sample_font(id: u32) -> ResolvedFont {
    ResolvedFont {
        id: ResolvedFontId(id),
        identity: ResolvedFontIdentity::from_file(
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            0,
            Some("DejaVuSansMono".to_string()),
        ),
        replay: FontReplay::Swash,
        family: "DejaVu Sans Mono".to_string(),
        full_name: None,
        postscript_name: Some("DejaVuSansMono".to_string()),
        weight: 400,
        slant: FontSlantKind::Normal,
        width: 5,
        pixel_size: 15.0,
        ascent_px: 12.0,
        descent_px: 3.0,
        space_advance_px: 8.0,
        source: FontResolutionSource::FacePrimary,
    }
}

#[test]
fn identity_from_file_builds_stable_key() {
    let identity = ResolvedFontIdentity::from_file("/fonts/a.ttc", 3, None);
    assert_eq!(identity.stable_key, "/fonts/a.ttc#3");
    assert_eq!(identity.file_path.as_deref(), Some("/fonts/a.ttc"));
    assert_eq!(identity.backend_selector(), 3);
    assert_eq!(identity.backend, FontBackendKind::Fontconfig);
}

#[test]
fn identity_equality_and_hash_are_field_exact() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let a = ResolvedFontIdentity::from_file("/fonts/a.ttf", 0, None);
    let b = ResolvedFontIdentity::from_file("/fonts/a.ttf", 0, None);
    let c = ResolvedFontIdentity::from_file("/fonts/a.ttf", 1, None);
    assert_eq!(a, b);
    assert_ne!(a, c);

    let hash = |v: &ResolvedFontIdentity| {
        let mut h = DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    };
    assert_eq!(hash(&a), hash(&b));
    assert_ne!(hash(&a), hash(&c));
}

#[test]
fn variation_coord_round_trips_value() {
    let coord = FontVariationCoord::new(u32::from_be_bytes(*b"wght"), 650.5);
    assert_eq!(coord.value(), 650.5);
}

#[test]
fn identity_stable_key_canonicalizes_variation_coordinates() {
    let weight = FontVariationCoord::new(u32::from_be_bytes(*b"wght"), 650.0);
    let width = FontVariationCoord::new(u32::from_be_bytes(*b"wdth"), 90.0);

    let a = ResolvedFontIdentity::from_file_with_variations(
        "/fonts/a.ttf",
        0,
        Some("VariableA".to_string()),
        vec![weight, width],
    );
    let reordered = ResolvedFontIdentity::from_file_with_variations(
        "/fonts/a.ttf",
        0,
        Some("VariableA".to_string()),
        vec![width, weight],
    );
    let different_weight = ResolvedFontIdentity::from_file_with_variations(
        "/fonts/a.ttf",
        0,
        Some("VariableA".to_string()),
        vec![FontVariationCoord::new(u32::from_be_bytes(*b"wght"), 700.0)],
    );

    assert_eq!(a, reordered);
    assert_eq!(a.stable_key, reordered.stable_key);
    assert!(a.stable_key.contains("wght=44228000"));
    assert!(a.stable_key.contains("wdth=42b40000"));
    assert_ne!(a, different_weight);
    assert_ne!(a.stable_key, different_weight.stable_key);
}

#[test]
fn fontconfig_identity_distinguishes_backend_and_file_face_indices() {
    // FreeType/Fontconfig encode named instance 7 of collection face 3 as
    // 0x0007_0003. fontdb/ttf-parser, however, can only open collection face
    // 3 and apply the requested variation attributes separately.
    let identity = ResolvedFontIdentity::from_file("/fonts/variable.ttc", 0x0007_0003, None);

    assert_eq!(identity.backend_selector(), 0x0007_0003);
    assert_eq!(identity.file_face_index(), 3);
    assert_eq!(identity.named_instance_index(), Some(7));

    let ordinary_collection_face = ResolvedFontIdentity::from_file("/fonts/ordinary.ttc", 12, None);
    assert_eq!(ordinary_collection_face.file_face_index(), 12);
    assert_eq!(ordinary_collection_face.named_instance_index(), None);
}

#[test]
fn native_file_identity_preserves_platform_and_exact_instance() {
    let weight = FontVariationCoord::new(u32::from_be_bytes(*b"wght"), 650.0);
    let width = FontVariationCoord::new(u32::from_be_bytes(*b"wdth"), 90.0);
    let core_text = ResolvedFontIdentity::from_platform_file_with_variations(
        FontBackendKind::CoreText,
        "/System/Library/Fonts/SFNSMono.ttf",
        2,
        Some("SFMono-Semibold".to_string()),
        vec![weight, width],
    );
    let direct_write = ResolvedFontIdentity::from_platform_file_with_variations(
        FontBackendKind::DirectWrite,
        "C:\\Windows\\Fonts\\CascadiaCode.ttf",
        3,
        Some("CascadiaCode-SemiBold".to_string()),
        vec![width, weight],
    );

    assert_eq!(core_text.backend, FontBackendKind::CoreText);
    assert_eq!(core_text.file_face_index(), 2);
    assert_eq!(core_text.backend_selector(), 2);
    assert_eq!(core_text.freetype_selector(), None);
    assert_eq!(core_text.named_instance_index(), None);
    assert!(core_text.stable_key.starts_with("coretext:"));
    assert!(core_text.stable_key.contains("wght=44228000"));
    assert!(core_text.stable_key.contains("wdth=42b40000"));

    assert_eq!(direct_write.backend, FontBackendKind::DirectWrite);
    assert_eq!(direct_write.file_face_index(), 3);
    assert!(direct_write.stable_key.starts_with("directwrite:"));
    assert_ne!(core_text, direct_write);
}

#[test]
fn generic_platform_constructor_keeps_one_canonical_fontconfig_identity() {
    let coords = vec![FontVariationCoord::new(u32::from_be_bytes(*b"wght"), 650.0)];
    let canonical = ResolvedFontIdentity::from_file_with_variations(
        "/fonts/variable.ttf",
        0x0007_0000,
        Some("Fixture-Semibold".to_string()),
        coords.clone(),
    );
    let through_platform = ResolvedFontIdentity::from_platform_file_with_variations(
        FontBackendKind::Fontconfig,
        "/fonts/variable.ttf",
        0x0007_0000,
        Some("Fixture-Semibold".to_string()),
        coords,
    );

    assert_eq!(through_platform, canonical);
}

#[test]
fn resolved_font_serde_round_trip() {
    let font = sample_font(7);
    let json = serde_json::to_string(&font).unwrap();
    let back: ResolvedFont = serde_json::from_str(&json).unwrap();
    assert_eq!(back, font);
}

#[test]
fn fixed_bitmap_replay_round_trips_with_the_resolved_font() {
    let mut font = sample_font(8);
    font.replay = FontReplay::FreeTypeBitmap {
        strike: BitmapStrikeKey {
            index: 2,
            x_ppem_26_6: 13 << 6,
            y_ppem_26_6: 13 << 6,
        },
        sampling: GlyphSampling::Nearest,
        spacing: FixedFontSpacing::MonospaceOrCharacterCell,
    };

    let json = serde_json::to_string(&font).unwrap();
    let back: ResolvedFont = serde_json::from_str(&json).unwrap();

    assert_eq!(back, font);
}

#[test]
fn resolved_font_table_serde_round_trip() {
    let mut table = ResolvedFontTable::new();
    table.insert(ResolvedFontId(1), sample_font(1));
    table.insert(ResolvedFontId(2), sample_font(2));
    let json = serde_json::to_string(&table).unwrap();
    let back: ResolvedFontTable = serde_json::from_str(&json).unwrap();
    assert_eq!(back, table);
}

#[test]
fn resolved_glyph_id_preserves_the_full_freetype_domain() {
    let glyph = ResolvedGlyphId::new(u32::from(u16::MAX) + 17);
    let encoded = serde_json::to_string(&glyph).expect("serialize glyph id");
    let decoded: ResolvedGlyphId = serde_json::from_str(&encoded).expect("deserialize glyph id");

    assert_eq!(decoded, glyph);
    assert_eq!(decoded.get(), 65_552);
}
