//! Platform font backend trait (render-boundary design §7).
//!
//! [`crate::font::resolver::FontResolver`] owns GNU-compatible fontset policy
//! and entity scoring. This module is the deliberately smaller platform half:
//! resolve native generic aliases, enumerate concrete candidates, report
//! coverage/spacing/metrics, and preserve an exact identity.

use neomacs_display_protocol::font::{FontBackendKind, ResolvedFontIdentity};
use neomacs_display_protocol::geometry::DeviceScale;
use neovm_core::face::{FontSlant, FontWidth};
use std::fmt::{Display, Formatter};
#[cfg(any(target_os = "macos", windows))]
use std::path::Path;

#[cfg(target_os = "macos")]
mod core_text;
#[cfg(windows)]
mod direct_write;

#[cfg(target_os = "macos")]
pub use core_text::CoreTextBackend;
#[cfg(windows)]
pub use direct_write::DirectWriteBackend;

/// A non-empty platform font-family name.
///
/// Native APIs are allowed to return missing or empty metadata.  Keeping that
/// state out of the shared resolver means every family that crosses the
/// platform boundary is immediately usable by Lisp and by candidate lookup.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FontFamilyName(String);

impl FontFamilyName {
    pub fn new(name: impl Into<String>) -> Option<Self> {
        let name = name.into();
        let name = name.trim();
        (!name.is_empty()).then(|| Self(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for FontFamilyName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for FontFamilyName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Native design-unit metrics transported with an exact candidate.
///
/// The platform backend attaches these only after shared policy selects the
/// winner, avoiding per-candidate metric work and avoiding a FreeType reopen of
/// a CoreText/DirectWrite selection. Values are scaled to the layout pixel size
/// at the layout boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformFontDesignMetrics {
    pub units_per_em: u32,
    pub ascent: i32,
    pub descent: i32,
    pub line_gap: i32,
    pub max_advance: i32,
    pub space_advance: i32,
    pub average_advance: i32,
}

impl PlatformFontDesignMetrics {
    pub fn at_pixel_size(self, pixel_size: f32) -> Option<crate::font::probe::FontPxMetrics> {
        if self.units_per_em == 0 || !pixel_size.is_finite() || pixel_size <= 0.0 {
            return None;
        }
        let scale = pixel_size / self.units_per_em as f32;
        let scaled = |value: i32| (value as f32 * scale).round() as i32;
        let ascent = scaled(self.ascent).max(0);
        let descent = scaled(self.descent).max(0);
        let line_gap = scaled(self.line_gap).max(0);
        Some(crate::font::probe::FontPxMetrics {
            pixel_size: pixel_size.round().max(1.0) as u32,
            height: (ascent + descent + line_gap).max(1),
            ascent,
            descent,
            max_width: scaled(self.max_advance).max(0),
            space_width: scaled(self.space_advance).max(0),
            average_width: scaled(self.average_advance).max(0),
        })
    }
}

/// Selector metadata for one exact native font.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformFontMetadata {
    pub foundry: Option<String>,
    pub family: String,
    pub weight: Option<u16>,
    pub slant: FontSlant,
    pub width: Option<FontWidth>,
    /// GNU/Fontconfig spacing code: proportional=0, dual=90, mono=100,
    /// charcell=110.
    pub spacing: Option<i32>,
    pub design_metrics: Option<PlatformFontDesignMetrics>,
    pub size: PlatformFontSize,
}

/// Size capability of one platform-discovered font entity.
///
/// `Unknown` is intentionally not treated as scalable. Backends such as
/// CoreText and DirectWrite do not always expose fixed strikes during native
/// enumeration, so the shared resolver asks the materializer to classify the
/// exact file before GNU size scoring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlatformFontSize {
    Scalable,
    Fixed {
        device_ppem_26_6: u32,
    },
    #[default]
    Unknown,
}

impl PlatformFontSize {
    pub const fn selected_device_ppem_26_6(self) -> Option<u32> {
        match self {
            Self::Fixed { device_ppem_26_6 } => Some(device_ppem_26_6),
            Self::Scalable | Self::Unknown => None,
        }
    }

    pub const fn is_fixed(self) -> bool {
        matches!(self, Self::Fixed { .. })
    }
}

impl PlatformFontMetadata {
    pub fn fixed_spacing_policy(&self) -> neomacs_display_protocol::font::FixedFontSpacing {
        match self.spacing {
            Some(100..) => {
                neomacs_display_protocol::font::FixedFontSpacing::MonospaceOrCharacterCell
            }
            _ => neomacs_display_protocol::font::FixedFontSpacing::ProportionalOrDual,
        }
    }

    pub fn width_class(&self) -> u16 {
        match self.width.unwrap_or(FontWidth::Normal) {
            FontWidth::UltraCondensed => 1,
            FontWidth::ExtraCondensed => 2,
            FontWidth::Condensed => 3,
            FontWidth::SemiCondensed => 4,
            FontWidth::Normal => 5,
            FontWidth::SemiExpanded => 6,
            FontWidth::Expanded => 7,
            FontWidth::ExtraExpanded => 8,
            FontWidth::UltraExpanded => 9,
        }
    }
}

/// One exact candidate discovered by a platform backend.
///
/// This is deliberately deeper than a file path: collection and variable-font
/// named instances can share a file while representing different drawable
/// fonts. Layout consumes this complete answer and transports its identity to
/// the renderer; neither layer reconstructs selection from family attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformFontMatch {
    pub identity: ResolvedFontIdentity,
    pub metadata: PlatformFontMetadata,
}

impl PlatformFontMatch {
    fn from_fontconfig(
        matched: crate::font::fontconfig::FontMatch,
        foundry: Option<String>,
        width: Option<FontWidth>,
        spacing: Option<i32>,
    ) -> Option<Self> {
        let file = matched.file.as_deref()?;
        let weight = matched
            .variation_coords
            .iter()
            .find(|coord| coord.tag == u32::from_be_bytes(*b"wght"))
            .map(|coord| coord.value().round().clamp(1.0, 1000.0) as u16)
            .or(matched.weight);
        let identity = ResolvedFontIdentity::from_file_with_variations(
            file,
            matched.face_index,
            matched.postscript_name.clone(),
            matched.variation_coords,
        );
        Some(Self {
            identity,
            metadata: PlatformFontMetadata {
                foundry,
                family: matched.family,
                weight,
                slant: matched.slant,
                width,
                spacing,
                design_metrics: None,
                size: matched.size,
            },
        })
    }

    fn finalize_fontconfig(mut self) -> Self {
        let Some(file) = self.identity.file_path.clone() else {
            return self;
        };
        let Some(face_selector) = self.identity.freetype_selector() else {
            return self;
        };
        let variation_coords =
            if self.identity.variation_coords.is_empty() && (face_selector >> 16) & 0x7fff != 0 {
                crate::font::probe::named_instance_variation_coords(&file, face_selector)
            } else {
                self.identity.variation_coords.clone()
            };
        let postscript_name = self
            .identity
            .postscript_name
            .clone()
            .or_else(|| crate::font::probe::postscript_name(&file, face_selector));
        if let Some(weight) = variation_coords
            .iter()
            .find(|coord| coord.tag == u32::from_be_bytes(*b"wght"))
            .map(|coord| coord.value().round().clamp(1.0, 1000.0) as u16)
        {
            self.metadata.weight = Some(weight);
        }
        self.identity = ResolvedFontIdentity::from_file_with_variations(
            &file,
            face_selector,
            postscript_name,
            variation_coords,
        );
        self
    }

    pub fn file_path(&self) -> Option<&str> {
        self.identity.file_path.as_deref()
    }

    pub fn family(&self) -> &str {
        &self.metadata.family
    }

    pub fn weight(&self) -> Option<u16> {
        self.metadata.weight
    }

    pub fn slant(&self) -> FontSlant {
        self.metadata.slant
    }

    pub fn pixel_metrics(&self, pixel_size: f32) -> Option<crate::font::probe::FontPxMetrics> {
        self.metadata
            .design_metrics
            .and_then(|metrics| metrics.at_pixel_size(pixel_size))
    }

    #[cfg(any(target_os = "macos", windows))]
    fn from_platform_file(
        backend: FontBackendKind,
        file: &Path,
        face_index: u32,
        postscript_name: Option<String>,
        variation_coords: Vec<neomacs_display_protocol::font::FontVariationCoord>,
        metadata: PlatformFontMetadata,
    ) -> Option<Self> {
        let file = file.to_str()?;
        Some(Self {
            identity: ResolvedFontIdentity::from_platform_file_with_variations(
                backend,
                file,
                face_index,
                postscript_name,
                variation_coords,
            ),
            metadata,
        })
    }
}

/// Text direction needed by native fallback APIs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

/// Requested realization size carried through native candidate selection.
/// The bit-preserving fields make it safe in resolver cache keys; the derived
/// device ppem is the domain GNU scores fixed-size entities in.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FontSelectionSize {
    layout_px_bits: u32,
    device_scale_bits: u32,
    device_px_26_6: u32,
}

impl FontSelectionSize {
    pub fn new(layout_px: f32, device_scale: DeviceScale) -> Self {
        let layout_px = if layout_px.is_finite() && layout_px > 0.0 {
            layout_px
        } else {
            1.0
        };
        let device_px = layout_px * device_scale.get();
        Self {
            layout_px_bits: layout_px.to_bits(),
            device_scale_bits: device_scale.get().to_bits(),
            device_px_26_6: (device_px * 64.0).round().clamp(1.0, u32::MAX as f32) as u32,
        }
    }

    pub const fn device_px_26_6(self) -> u32 {
        self.device_px_26_6
    }
}

impl TextDirection {
    pub fn for_char(ch: char) -> Self {
        use crate::bidi::BidiClass;
        match crate::bidi::bidi_class(ch) {
            BidiClass::R | BidiClass::AL | BidiClass::RLE | BidiClass::RLO | BidiClass::RLI => {
                Self::RightToLeft
            }
            _ => Self::LeftToRight,
        }
    }
}

/// Discovery request passed from shared policy to one native backend.
#[derive(Clone, Debug)]
pub struct FontCandidateQuery {
    pub scope: FontCandidateScope,
    pub required_char: Option<char>,
    pub charset_ranges: Vec<(u32, u32)>,
    pub languages: Vec<String>,
    pub requested_weight: u16,
    pub requested_slant: FontSlant,
    pub requested_width: FontWidth,
    pub direction: TextDirection,
}

/// Which native candidate population one query addresses.
///
/// `All` and `NativeFallback` were previously both encoded as `family: None`.
/// Keeping them distinct makes every backend implement GNU `list-fonts`
/// enumeration separately from per-character platform fallback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontCandidateScope {
    Family(FontFamilyName),
    All,
    NativeFallback { base_family: FontFamilyName },
}

impl FontCandidateScope {
    pub fn queried_family(&self) -> Option<&str> {
        match self {
            Self::Family(family) => Some(family.as_str()),
            Self::All | Self::NativeFallback { .. } => None,
        }
    }
}

/// One raw candidate plus attributes used exclusively by shared scoring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontCandidate {
    pub matched: PlatformFontMatch,
}

pub trait FontBackend: Send {
    /// Native platform implementation represented by this adapter.
    fn kind(&self) -> FontBackendKind;

    /// Enumerate the font families visible to this platform adapter.
    ///
    /// This is GNU's `font_driver::list_family` operation.  It is deliberately
    /// required rather than defaulted: a new platform backend must implement
    /// the public `font-family-list` contract before it can compile.
    fn list_families(&self) -> Vec<FontFamilyName>;

    /// Resolve a generic family alias ("monospace", "sans-serif", …) to the
    /// concrete family the platform would pick. Concrete names pass through
    /// unchanged.
    fn resolve_family(&self, family: &str) -> String;

    /// Whether the platform considers this family monospace-preferring
    /// (drives fallback ordering for per-char matches).
    fn family_prefers_monospace(&self, family: &str) -> bool;

    /// Enumerate candidates for one shared-policy pass.
    ///
    /// Ordering must preserve the native discovery/cascade order. The backend
    /// may filter by coverage but must not score weight/slant/spacing; that is
    /// [`crate::font::resolver::FontResolver`]'s responsibility.
    fn list_candidates(&self, query: &FontCandidateQuery) -> Vec<FontCandidate>;

    /// Enrich the selected candidate into an exact renderer identity.
    ///
    /// Discovery deliberately asks only for policy-relevant metadata because
    /// Fontconfig changes enumeration order when renderer metadata is included
    /// in `FcFontList`.  This hook runs once after shared policy chooses a
    /// candidate, never while candidates are being scored.
    fn finalize_match(&self, matched: PlatformFontMatch) -> PlatformFontMatch {
        matched
    }

    /// Native metrics for the already selected exact candidate.
    ///
    /// Called once on a resolver cache miss, never for every enumerated
    /// candidate. Backends whose ordinary metric probe already consumes the
    /// exact identity (Fontconfig/FreeType) may return `None`.
    fn design_metrics(&self, _matched: &PlatformFontMatch) -> Option<PlatformFontDesignMetrics> {
        None
    }
}

/// Linux backend: fontconfig via [`crate::font::fontconfig`].
pub struct FontconfigBackend;

impl FontBackend for FontconfigBackend {
    fn kind(&self) -> FontBackendKind {
        FontBackendKind::Fontconfig
    }

    fn list_families(&self) -> Vec<FontFamilyName> {
        crate::font::fontconfig::list_families()
    }

    fn resolve_family(&self, family: &str) -> String {
        crate::font::fontconfig::resolve_family(family).to_string()
    }

    fn family_prefers_monospace(&self, family: &str) -> bool {
        crate::font::fontconfig::family_prefers_monospace(family)
    }

    fn list_candidates(&self, query: &FontCandidateQuery) -> Vec<FontCandidate> {
        let family = match &query.scope {
            FontCandidateScope::Family(family) => Some(family.as_str()),
            FontCandidateScope::All | FontCandidateScope::NativeFallback { .. } => None,
        };
        crate::font::fontconfig::fc_list_candidates(
            family,
            &query.charset_ranges,
            query.required_char.map(u32::from),
            &query.languages,
        )
        .into_iter()
        .filter_map(|candidate| {
            Some(FontCandidate {
                matched: PlatformFontMatch::from_fontconfig(
                    candidate.matched,
                    candidate.foundry,
                    candidate.width,
                    candidate.spacing,
                )?,
            })
        })
        .collect()
    }

    fn finalize_match(&self, matched: PlatformFontMatch) -> PlatformFontMatch {
        matched.finalize_fontconfig()
    }
}

/// The platform's default backend.
pub fn default_font_backend() -> Box<dyn FontBackend> {
    std::cfg_select! {
        target_os = "macos" => {
            Box::new(CoreTextBackend)
        }
        windows => {
            Box::new(DirectWriteBackend)
        }
        _ => {
            Box::new(FontconfigBackend)
        }
    }
}

#[cfg(target_os = "macos")]
fn file_face_index_for_postscript_name(path: &Path, postscript_name: &str) -> Option<u32> {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

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

#[cfg(test)]
mod tests {
    use neomacs_display_protocol::font::FontBackendKind;

    #[test]
    fn default_backend_matches_the_build_target() {
        let backend = super::default_font_backend();
        #[cfg(target_os = "linux")]
        assert_eq!(backend.kind(), FontBackendKind::Fontconfig);
        #[cfg(target_os = "macos")]
        assert_eq!(backend.kind(), FontBackendKind::CoreText);
        #[cfg(windows)]
        assert_eq!(backend.kind(), FontBackendKind::DirectWrite);
    }

    #[test]
    fn native_design_metrics_scale_at_the_layout_boundary() {
        let metrics = super::PlatformFontDesignMetrics {
            units_per_em: 1_000,
            ascent: 800,
            descent: 200,
            line_gap: 100,
            max_advance: 700,
            space_advance: 500,
            average_advance: 600,
        }
        .at_pixel_size(20.0)
        .expect("valid design metrics");

        assert_eq!(metrics.ascent, 16);
        assert_eq!(metrics.descent, 4);
        assert_eq!(metrics.height, 22);
        assert_eq!(metrics.max_width, 14);
        assert_eq!(metrics.space_width, 10);
        assert_eq!(metrics.average_width, 12);
    }

    #[test]
    fn native_fallback_direction_uses_unicode_bidi_class() {
        assert_eq!(
            super::TextDirection::for_char('א'),
            super::TextDirection::RightToLeft
        );
        assert_eq!(
            super::TextDirection::for_char('م'),
            super::TextDirection::RightToLeft
        );
        assert_eq!(
            super::TextDirection::for_char('好'),
            super::TextDirection::LeftToRight
        );
    }
}
