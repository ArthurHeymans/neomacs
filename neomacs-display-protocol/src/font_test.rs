use super::*;

fn sample_font(id: u32) -> ResolvedFont {
    ResolvedFont {
        id: ResolvedFontId(id),
        identity: ResolvedFontIdentity::from_file(
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            0,
            Some("DejaVuSansMono".to_string()),
        ),
        family: "DejaVu Sans Mono".to_string(),
        full_name: None,
        postscript_name: Some("DejaVuSansMono".to_string()),
        weight: 400,
        slant: FontSlantKind::Normal,
        width: 5,
        pixel_size: 15.0,
        ascent_px: 12.0,
        descent_px: 3.0,
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
fn resolved_font_serde_round_trip() {
    let font = sample_font(7);
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
