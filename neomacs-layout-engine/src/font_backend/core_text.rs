use super::{
    FontBackend, FontCandidate, FontCandidateQuery, FontCandidateScope, FontFamilyName,
    PlatformFontDesignMetrics, PlatformFontMatch, PlatformFontMetadata,
    file_face_index_for_postscript_name,
};
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_text::font::{self, CTFont};
use core_text::font_collection;
use core_text::font_descriptor::{
    self, SymbolicTraitAccessors, TraitAccessors, kCTFontOrientationHorizontal,
};
use neomacs_display_protocol::font::{FontBackendKind, FontVariationCoord};
use neovm_core::face::{FontSlant, FontWidth};

/// CoreText adapter for native macOS family matching and cascade fallback.
pub struct CoreTextBackend;

impl FontBackend for CoreTextBackend {
    fn kind(&self) -> FontBackendKind {
        FontBackendKind::CoreText
    }

    fn list_families(&self) -> Vec<FontFamilyName> {
        core_text::font_manager::copy_available_font_family_names()
            .iter()
            .filter_map(|family| FontFamilyName::new(family.to_string()))
            .collect()
    }

    fn resolve_family(&self, family: &str) -> String {
        resolve_generic_family(family).unwrap_or_else(|| family.to_string())
    }

    fn family_prefers_monospace(&self, family: &str) -> bool {
        first_family_font(&self.resolve_family(family))
            .is_some_and(|font| font.symbolic_traits().is_monospace())
    }

    fn list_candidates(&self, query: &FontCandidateQuery) -> Vec<FontCandidate> {
        let fonts: Vec<CTFont> = match &query.scope {
            FontCandidateScope::Family(family) => {
                font_collection::create_for_family(family.as_str())
                    .and_then(|collection| collection.get_descriptors())
                    .map(|descriptors| {
                        descriptors
                            .iter()
                            .map(|descriptor| font::new_from_descriptor(&descriptor, 0.0))
                            .collect()
                    })
                    .unwrap_or_default()
            }
            FontCandidateScope::All => font_collection::create_for_all_families()
                .get_descriptors()
                .map(|descriptors| {
                    descriptors
                        .iter()
                        .map(|descriptor| font::new_from_descriptor(&descriptor, 0.0))
                        .collect()
                })
                .unwrap_or_default(),
            FontCandidateScope::NativeFallback { base_family } => {
                let Ok(base) = font::new_from_name(base_family.as_str(), 0.0) else {
                    return Vec::new();
                };
                let languages: Vec<CFString> = query
                    .languages
                    .iter()
                    .map(|lang| CFString::new(lang))
                    .collect();
                let languages = CFArray::<CFString>::from_CFTypes(&languages);
                font::cascade_list_for_languages(&base, &languages)
                    .iter()
                    .map(|descriptor| font::new_from_descriptor(&descriptor, 0.0))
                    .collect()
            }
        };

        fonts
            .into_iter()
            .filter(|candidate| {
                query
                    .required_char
                    .is_none_or(|ch| font_supports_char(candidate, ch))
            })
            .filter_map(font_candidate_from_font)
            .collect()
    }

    fn design_metrics(&self, matched: &PlatformFontMatch) -> Option<PlatformFontDesignMetrics> {
        let postscript_name = matched.identity.postscript_name.as_deref()?;
        let base = font::new_from_name(postscript_name, 0.0).ok()?;
        let font = font_with_variations(base, &matched.identity.variation_coords)?;
        font_design_metrics(&font)
    }
}

fn resolve_generic_family(family: &str) -> Option<String> {
    let normalized = family.trim().to_ascii_lowercase();
    let ui_type = match normalized.as_str() {
        "fixed" | "mono" | "monospace" => font::kCTFontUserFixedPitchFontType,
        "sans" | "sans-serif" | "sans serif" => font::kCTFontUserFontType,
        _ => return None,
    };
    Some(font::new_ui_font_for_language(ui_type, 0.0, None).family_name())
}

fn first_family_font(family: &str) -> Option<CTFont> {
    let collection = font_collection::create_for_family(family)?;
    collection
        .get_descriptors()?
        .iter()
        .map(|descriptor| font::new_from_descriptor(&descriptor, 0.0))
        .next()
}

fn core_text_weight_to_css(weight: f64) -> u16 {
    const MAPPING: [f64; 9] = [-0.7, -0.5, -0.23, 0.0, 0.2, 0.3, 0.4, 0.6, 0.8];
    let upper = MAPPING.partition_point(|candidate| *candidate < weight);
    let index = if upper == 0 {
        0.0
    } else if upper >= MAPPING.len() {
        MAPPING.len() as f64
    } else {
        let lower = upper - 1;
        let distance = (weight - MAPPING[lower]) / (MAPPING[upper] - MAPPING[lower]);
        lower as f64 + distance
    };
    ((index + 1.0) * 100.0).round().clamp(100.0, 900.0) as u16
}

fn font_supports_char(font: &CTFont, ch: char) -> bool {
    let mut utf16 = [0_u16; 2];
    let encoded = ch.encode_utf16(&mut utf16);
    let mut glyphs = [0_u16; 2];
    unsafe {
        font.get_glyphs_for_characters(
            encoded.as_ptr(),
            glyphs.as_mut_ptr(),
            encoded.len() as isize,
        )
    }
}

fn font_candidate_from_font(font: CTFont) -> Option<FontCandidate> {
    let descriptor = font.copy_descriptor();
    let path = descriptor.font_path()?;
    let postscript_name = font.postscript_name();
    let face_index = file_face_index_for_postscript_name(&path, &postscript_name)?;
    let traits = font.all_traits();
    let symbolic = font.symbolic_traits();
    let width = core_text_width(traits.normalized_width());
    let spacing = if symbolic.is_monospace() { 100 } else { 0 };
    let matched = PlatformFontMatch::from_platform_file(
        FontBackendKind::CoreText,
        &path,
        face_index,
        Some(postscript_name),
        variation_coords(&descriptor),
        PlatformFontMetadata {
            foundry: None,
            family: font.family_name(),
            weight: Some(core_text_weight_to_css(traits.normalized_weight())),
            slant: if symbolic.is_italic() {
                FontSlant::Italic
            } else {
                FontSlant::Normal
            },
            width: Some(width),
            spacing: Some(spacing),
            design_metrics: None,
            // CoreText exposes these candidates as scalable faces. Fixed
            // strike metadata, when available, belongs in this typed field
            // rather than being reconstructed during materialization.
            size: super::PlatformFontSize::Unknown,
        },
    )?;
    Some(FontCandidate { matched })
}

fn core_text_width(width: f64) -> FontWidth {
    match width {
        ..=-0.75 => FontWidth::UltraCondensed,
        ..=-0.5 => FontWidth::ExtraCondensed,
        ..=-0.25 => FontWidth::Condensed,
        ..=-0.1 => FontWidth::SemiCondensed,
        ..=0.1 => FontWidth::Normal,
        ..=0.25 => FontWidth::SemiExpanded,
        ..=0.5 => FontWidth::Expanded,
        ..=0.75 => FontWidth::ExtraExpanded,
        _ => FontWidth::UltraExpanded,
    }
}

fn font_with_variations(base: CTFont, coords: &[FontVariationCoord]) -> Option<CTFont> {
    if coords.is_empty() {
        return Some(base);
    }
    let values: Vec<(CFNumber, CFNumber)> = coords
        .iter()
        .map(|coord| {
            (
                CFNumber::from(i64::from(coord.tag)),
                CFNumber::from(f64::from(coord.value())),
            )
        })
        .collect();
    let values = CFDictionary::from_CFType_pairs(&values);
    let variation_key =
        unsafe { CFString::wrap_under_get_rule(font_descriptor::kCTFontVariationAttribute) };
    let attributes = CFDictionary::from_CFType_pairs(&[(variation_key, values)]);
    let descriptor = base
        .copy_descriptor()
        .create_copy_with_attributes(attributes.to_untyped())
        .ok()?;
    Some(font::new_from_descriptor(&descriptor, 0.0))
}

fn font_design_metrics(font: &CTFont) -> Option<PlatformFontDesignMetrics> {
    let units_per_em = font.units_per_em();
    let point_size = font.pt_size();
    if units_per_em == 0 || point_size <= 0.0 {
        return None;
    }
    let to_design = |value: f64| (value * f64::from(units_per_em) / point_size).round() as i32;

    let chars: Vec<u16> = (32_u16..=126).collect();
    let mut glyphs = vec![0_u16; chars.len()];
    unsafe {
        font.get_glyphs_for_characters(chars.as_ptr(), glyphs.as_mut_ptr(), chars.len() as isize);
    }
    let mut advances = vec![Default::default(); glyphs.len()];
    unsafe {
        font.get_advances_for_glyphs(
            kCTFontOrientationHorizontal,
            glyphs.as_ptr(),
            advances.as_mut_ptr(),
            glyphs.len() as isize,
        );
    }
    let advances: Vec<i32> = advances
        .into_iter()
        .map(|advance| to_design(advance.width))
        .filter(|advance| *advance > 0)
        .collect();
    let max_advance = advances.iter().copied().max().unwrap_or(0);
    let average_advance = if advances.is_empty() {
        0
    } else {
        advances.iter().sum::<i32>() / advances.len() as i32
    };
    let space_advance = advances.first().copied().unwrap_or(0);
    Some(PlatformFontDesignMetrics {
        units_per_em,
        ascent: to_design(font.ascent()),
        descent: to_design(font.descent()),
        line_gap: to_design(font.leading()),
        max_advance,
        space_advance,
        average_advance,
    })
}

fn variation_coords(descriptor: &font_descriptor::CTFontDescriptor) -> Vec<FontVariationCoord> {
    let attributes = descriptor.attributes();
    let variation_key =
        unsafe { CFString::wrap_under_get_rule(font_descriptor::kCTFontVariationAttribute) };
    let Some(variations) = attributes
        .find(variation_key)
        .and_then(|value| value.downcast::<CFDictionary>())
    else {
        return Vec::new();
    };
    let (keys, values) = variations.get_keys_and_values();
    keys.into_iter()
        .zip(values)
        .filter_map(|(key, value)| unsafe {
            let tag = CFType::wrap_under_get_rule(key as CFTypeRef)
                .downcast::<CFNumber>()?
                .to_i64()?;
            let value = CFType::wrap_under_get_rule(value as CFTypeRef)
                .downcast::<CFNumber>()?
                .to_f64()?;
            Some(FontVariationCoord::new(
                u32::try_from(tag).ok()?,
                value as f32,
            ))
        })
        .collect()
}
