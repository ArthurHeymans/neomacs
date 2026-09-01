//! Safe macOS catalog adapter.
//!
//! GNU-compatible selection remains in the shared resolver. CoreText calls and
//! ownership contracts are isolated in `core_text_calls`; this module sees only
//! owned Rust data and exposes no Apple framework type.

mod core_text_calls;

use super::{
    FontBackend, FontCandidate, FontCandidateQuery, FontFamilyName, PlatformFontDesignMetrics,
    PlatformFontMatch, PlatformFontMetadata,
};
use neomacs_display_protocol::font::FontBackendKind;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// CoreText adapter for native macOS family matching and cascade fallback.
pub struct CoreTextBackend;

impl FontBackend for CoreTextBackend {
    fn kind(&self) -> FontBackendKind {
        FontBackendKind::CoreText
    }

    fn list_families(&self) -> Vec<FontFamilyName> {
        core_text_calls::available_family_names()
            .into_iter()
            .filter_map(FontFamilyName::new)
            .collect()
    }

    fn resolve_family(&self, family: &str) -> String {
        core_text_calls::resolve_generic_family(family).unwrap_or_else(|| family.to_string())
    }

    fn family_prefers_monospace(&self, family: &str) -> bool {
        core_text_calls::family_prefers_monospace(&self.resolve_family(family))
    }

    fn list_candidates(&self, query: &FontCandidateQuery) -> Vec<FontCandidate> {
        core_text_calls::candidates(query)
            .into_iter()
            .filter_map(|candidate| {
                // The normal GPU glyph engine needs stable bytes. URL-less
                // CoreText faces will become the typed Native asset variant;
                // until that store lands they are reported at this one seam
                // instead of being disguised as a file identity.
                let path = candidate.path?;
                let face_index =
                    file_face_index_for_postscript_name(&path, &candidate.postscript_name)?;
                let matched = PlatformFontMatch::from_platform_file(
                    FontBackendKind::CoreText,
                    &path,
                    face_index,
                    Some(candidate.postscript_name),
                    candidate.variation_coords,
                    PlatformFontMetadata {
                        foundry: None,
                        family: candidate.family,
                        weight: Some(candidate.weight),
                        slant: candidate.slant,
                        width: Some(candidate.width),
                        spacing: Some(candidate.spacing),
                        design_metrics: None,
                        size: super::PlatformFontSize::Unknown,
                    },
                )?;
                Some(FontCandidate { matched })
            })
            .collect()
    }

    fn design_metrics(&self, matched: &PlatformFontMatch) -> Option<PlatformFontDesignMetrics> {
        core_text_calls::design_metrics(
            matched.identity.postscript_name.as_deref()?,
            &matched.identity.variation_coords,
        )
    }
}

fn file_face_index_for_postscript_name(path: &Path, postscript_name: &str) -> Option<u32> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, HashMap<String, u32>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(cache) = cache.lock()
        && let Some(faces) = cache.get(path)
    {
        return faces.get(postscript_name).copied();
    }

    let data = std::fs::read(path).ok()?;
    let face_count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
    let mut faces = HashMap::new();
    if face_count == 1 {
        ttf_parser::Face::parse(&data, 0).ok()?;
        faces.insert(postscript_name.to_string(), 0);
    } else {
        for face_index in 0..face_count {
            let name = ttf_parser::Face::parse(&data, face_index)
                .ok()
                .and_then(|face| {
                    face.names()
                        .into_iter()
                        .find(|name| {
                            name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME
                                && name.is_unicode()
                        })
                        .and_then(|name| name.to_string())
                });
            if let Some(name) = name {
                faces.insert(name, face_index);
            }
        }
    }
    let selected = faces.get(postscript_name).copied();
    if let Ok(mut cache) = cache.lock() {
        cache.insert(path.to_path_buf(), faces);
    }
    selected
}
