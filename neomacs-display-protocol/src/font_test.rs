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
    assert_eq!(identity.face_index, 3);
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
