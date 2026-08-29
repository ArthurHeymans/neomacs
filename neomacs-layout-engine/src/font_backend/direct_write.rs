use super::{
    FontBackend, FontCandidate, FontCandidateQuery, PlatformFontDesignMetrics, PlatformFontMatch,
    PlatformFontMetadata, TextDirection,
};
use dwrote::{
    Font, FontCollection, FontFallback, FontStretch, FontStyle, FontWeight, InformationalStringId,
    TextAnalysisSource, TextAnalysisSourceMethods,
};
use neomacs_display_protocol::font::{FontBackendKind, FontVariationCoord};
use neovm_core::face::{FontSlant, FontWidth};
use std::borrow::Cow;

/// DirectWrite adapter for native Windows matching and system fallback.
pub struct DirectWriteBackend;

impl FontBackend for DirectWriteBackend {
    fn kind(&self) -> FontBackendKind {
        FontBackendKind::DirectWrite
    }

    fn resolve_family(&self, family: &str) -> String {
        resolve_generic_family(family).unwrap_or_else(|| family.to_string())
    }

    fn family_prefers_monospace(&self, family: &str) -> bool {
        FontCollection::system()
            .font_family_by_name(&self.resolve_family(family))
            .ok()
            .flatten()
            .and_then(|family| family.font(0).ok())
            .and_then(|font| font.is_monospace())
            .unwrap_or_default()
    }

    fn list_candidates(&self, query: &FontCandidateQuery) -> Vec<FontCandidate> {
        match query.family.as_deref() {
            Some(family) => {
                let Some(family) = FontCollection::system()
                    .font_family_by_name(family)
                    .ok()
                    .flatten()
                else {
                    return Vec::new();
                };
                (0..family.get_font_count())
                    .filter_map(|index| family.font(index).ok())
                    .filter(|font| {
                        query.required_char.is_none_or(|ch| {
                            font.create_font_face()
                                .glyph_indices(&[ch as u32])
                                .ok()
                                .and_then(|glyphs| glyphs.first().copied())
                                .is_some_and(|glyph| glyph != 0)
                        })
                    })
                    .filter_map(font_candidate_from_font)
                    .collect()
            }
            None => native_fallback_candidate(query).into_iter().collect(),
        }
    }

    fn design_metrics(&self, matched: &PlatformFontMatch) -> Option<PlatformFontDesignMetrics> {
        let family = FontCollection::system()
            .font_family_by_name(matched.family())
            .ok()
            .flatten()?;
        (0..family.get_font_count())
            .filter_map(|index| family.font(index).ok())
            .find(|font| font_matches(font, matched))
            .and_then(|font| font_design_metrics(&font.create_font_face()))
    }
}

fn resolve_generic_family(family: &str) -> Option<String> {
    let candidates: &[&str] = match family.trim().to_ascii_lowercase().as_str() {
        "fixed" | "mono" | "monospace" => &[
            "Cascadia Mono",
            "Cascadia Code",
            "Consolas",
            "Lucida Console",
            "Courier New",
        ],
        "sans" | "sans-serif" | "sans serif" => &["Segoe UI", "Arial"],
        "serif" => &["Times New Roman", "Georgia"],
        _ => return None,
    };
    let collection = FontCollection::system();
    candidates.iter().find_map(|candidate| {
        collection
            .font_family_by_name(candidate)
            .ok()
            .flatten()
            .map(|_| (*candidate).to_string())
    })
}

fn to_directwrite_weight(weight: u16) -> FontWeight {
    FontWeight::from_u32(u32::from(weight.clamp(1, 999)))
}

fn native_fallback_candidate(query: &FontCandidateQuery) -> Option<FontCandidate> {
    let ch = query.required_char?;
    let fallback = FontFallback::get_system_fallback()?;
    let utf16: Vec<u16> = ch.to_string().encode_utf16().collect();
    let text_len = u32::try_from(utf16.len()).ok()?;
    let locale = query
        .languages
        .first()
        .cloned()
        .or_else(user_default_locale)
        .unwrap_or_else(|| "und".to_string());
    let analysis = TextAnalysisSource::from_text(
        Box::new(SingleLocaleAnalysis {
            text_len,
            locale,
            direction: query.direction,
        }),
        Cow::Owned(utf16),
    );
    let collection = FontCollection::system();
    fallback
        .map_characters(
            &analysis,
            0,
            text_len,
            &collection,
            Some(&query.fallback_family),
            to_directwrite_weight(query.requested_weight),
            if query.requested_slant.is_italic() {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            },
            to_directwrite_stretch(query.requested_width),
        )
        .mapped_font
        .and_then(font_candidate_from_font)
}

fn font_candidate_from_font(font: Font) -> Option<FontCandidate> {
    let face = font.create_font_face();
    let files = face.files().ok()?;
    if files.len() != 1 {
        return None;
    }
    let path = files[0].font_file_path().ok()?;
    let variations = face
        .variations()
        .ok()?
        .into_iter()
        .map(|axis| FontVariationCoord::new(axis.axisTag.swap_bytes(), axis.value))
        .collect();
    let postscript_name = font.informational_string(InformationalStringId::PostscriptName);
    let spacing = if font.is_monospace().unwrap_or_default() {
        100
    } else {
        0
    };
    let width = from_directwrite_stretch(font.stretch());
    let matched = PlatformFontMatch::from_platform_file(
        FontBackendKind::DirectWrite,
        &path,
        face.get_index(),
        postscript_name,
        variations,
        PlatformFontMetadata {
            family: font.family_name(),
            weight: Some(font.weight().to_u32().clamp(1, u32::from(u16::MAX)) as u16),
            slant: match font.style() {
                FontStyle::Italic => FontSlant::Italic,
                FontStyle::Oblique => FontSlant::Oblique,
                FontStyle::Normal => FontSlant::Normal,
            },
            width: Some(width),
            spacing: Some(spacing),
            design_metrics: None,
            // DirectWrite exposes these candidates as scalable faces. If a
            // future adapter enumerates fixed strikes, it must provide the
            // selected device ppem here so shared GNU scoring can classify it.
            size: super::PlatformFontSize::Unknown,
        },
    )?;
    Some(FontCandidate { matched })
}

fn font_matches(font: &Font, matched: &PlatformFontMatch) -> bool {
    let face = font.create_font_face();
    if face.get_index() != matched.identity.file_face_index() {
        return false;
    }
    if let Some(expected_name) = matched.identity.postscript_name.as_deref()
        && font
            .informational_string(InformationalStringId::PostscriptName)
            .as_deref()
            != Some(expected_name)
    {
        return false;
    }
    let Ok(mut variations) = face.variations().map(|axes| {
        axes.into_iter()
            .map(|axis| FontVariationCoord::new(axis.axisTag.swap_bytes(), axis.value))
            .collect::<Vec<_>>()
    }) else {
        return false;
    };
    variations.sort_unstable_by_key(|coord| (coord.tag, coord.value_bits));
    if variations != matched.identity.variation_coords {
        return false;
    }
    let Some(expected_path) = matched.file_path() else {
        return false;
    };
    let Ok(files) = face.files() else {
        return false;
    };
    files.len() == 1
        && files[0]
            .font_file_path()
            .is_ok_and(|path| path.as_os_str() == std::ffi::OsStr::new(expected_path))
}

fn font_design_metrics(face: &dwrote::FontFace) -> Option<PlatformFontDesignMetrics> {
    let metrics = face.metrics().metrics0();
    let codepoints: Vec<u32> = (32..=126).collect();
    let glyphs = face.glyph_indices(&codepoints).ok()?;
    let advances: Vec<i32> = face
        .design_glyph_metrics(&glyphs, false)
        .ok()?
        .into_iter()
        .map(|metrics| metrics.advanceWidth as i32)
        .filter(|advance| *advance > 0)
        .collect();
    let max_advance = advances.iter().copied().max().unwrap_or(0);
    let average_advance = if advances.is_empty() {
        0
    } else {
        advances.iter().sum::<i32>() / advances.len() as i32
    };
    Some(PlatformFontDesignMetrics {
        units_per_em: u32::from(metrics.designUnitsPerEm),
        ascent: i32::from(metrics.ascent),
        descent: i32::from(metrics.descent),
        line_gap: i32::from(metrics.lineGap),
        max_advance,
        space_advance: advances.first().copied().unwrap_or(0),
        average_advance,
    })
}

fn to_directwrite_stretch(width: FontWidth) -> FontStretch {
    match width {
        FontWidth::UltraCondensed => FontStretch::UltraCondensed,
        FontWidth::ExtraCondensed => FontStretch::ExtraCondensed,
        FontWidth::Condensed => FontStretch::Condensed,
        FontWidth::SemiCondensed => FontStretch::SemiCondensed,
        FontWidth::Normal => FontStretch::Normal,
        FontWidth::SemiExpanded => FontStretch::SemiExpanded,
        FontWidth::Expanded => FontStretch::Expanded,
        FontWidth::ExtraExpanded => FontStretch::ExtraExpanded,
        FontWidth::UltraExpanded => FontStretch::UltraExpanded,
    }
}

fn from_directwrite_stretch(width: FontStretch) -> FontWidth {
    match width {
        FontStretch::Undefined | FontStretch::Normal => FontWidth::Normal,
        FontStretch::UltraCondensed => FontWidth::UltraCondensed,
        FontStretch::ExtraCondensed => FontWidth::ExtraCondensed,
        FontStretch::Condensed => FontWidth::Condensed,
        FontStretch::SemiCondensed => FontWidth::SemiCondensed,
        FontStretch::SemiExpanded => FontWidth::SemiExpanded,
        FontStretch::Expanded => FontWidth::Expanded,
        FontStretch::ExtraExpanded => FontWidth::ExtraExpanded,
        FontStretch::UltraExpanded => FontWidth::UltraExpanded,
    }
}

fn user_default_locale() -> Option<String> {
    sys_locale::get_locale().filter(|locale| !locale.is_empty())
}

struct SingleLocaleAnalysis {
    text_len: u32,
    locale: String,
    direction: TextDirection,
}

impl TextAnalysisSourceMethods for SingleLocaleAnalysis {
    fn get_locale_name(&self, text_position: u32) -> (Cow<'_, str>, u32) {
        (
            Cow::Borrowed(&self.locale),
            self.text_len.saturating_sub(text_position),
        )
    }

    fn get_paragraph_reading_direction(&self) -> winapi::um::dwrite::DWRITE_READING_DIRECTION {
        match self.direction {
            TextDirection::LeftToRight => {
                winapi::um::dwrite::DWRITE_READING_DIRECTION_LEFT_TO_RIGHT
            }
            TextDirection::RightToLeft => {
                winapi::um::dwrite::DWRITE_READING_DIRECTION_RIGHT_TO_LEFT
            }
        }
    }
}
