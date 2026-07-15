//! Platform font backend trait (render-boundary design §7).
//!
//! The shared resolver in [`crate::font_metrics::FontMetricsService`] owns
//! GNU-compatible selection policy; this trait is the platform-specific half
//! it queries: family alias resolution and per-character coverage matching.
//! Linux implements it with fontconfig. macOS (CoreText descriptors) and
//! Windows (DirectWrite identities) implement it when those ports arrive —
//! the resolver itself stays platform-neutral.
//!
//! Not yet routed through the trait: the monospace-preference heuristic in
//! frame column-width derivation (a metrics policy detail) and the
//! renderer's emergency fallback (deliberately render-side and counted).

use neomacs_display_protocol::font::ResolvedFontIdentity;
use neovm_core::face::FontSlant;

/// One exact font selected by a platform backend.
///
/// This is deliberately deeper than a file path: collection and variable-font
/// named instances can share a file while representing different drawable
/// fonts.  Layout consumes this complete answer and transports its identity to
/// the renderer; neither layer reconstructs selection from family attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformFontMatch {
    pub identity: ResolvedFontIdentity,
    pub family: String,
    pub weight: Option<u16>,
    pub slant: FontSlant,
}

impl PlatformFontMatch {
    fn from_fontconfig(mut matched: crate::fontconfig::FontMatch) -> Option<Self> {
        let file = matched.file.as_deref()?;
        if matched.variation_coords.is_empty() && (matched.face_index >> 16) & 0x7fff != 0 {
            matched.variation_coords =
                crate::font_probe::named_instance_variation_coords(file, matched.face_index);
        }
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
            family: matched.family,
            weight,
            slant: matched.slant,
        })
    }

    pub fn file_path(&self) -> Option<&str> {
        self.identity.file_path.as_deref()
    }
}

pub trait FontBackend: Send {
    /// Resolve a generic family alias ("monospace", "sans-serif", …) to the
    /// concrete family the platform would pick. Concrete names pass through
    /// unchanged.
    fn resolve_family(&self, family: &str) -> String;

    /// Whether the platform considers this family monospace-preferring
    /// (drives fallback ordering for per-char matches).
    fn family_prefers_monospace(&self, family: &str) -> bool;

    /// Platform per-character coverage match: the font the platform would
    /// substitute for `ch` starting from `family`. Returns `None` for ASCII
    /// (the face's primary font always applies) or when nothing matches.
    fn match_font_for_char(
        &self,
        family: &str,
        ch: char,
        prefer_monospace: bool,
        requested_weight: u16,
        italic: bool,
    ) -> Option<PlatformFontMatch>;

    /// The exact font the platform would open for a primary face request.
    /// This is the same concrete entity `find-font` / GNU select, including
    /// collection or variable-font named-instance identity.
    fn match_primary_font(
        &self,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> Option<PlatformFontMatch>;
}

/// Linux backend: fontconfig via [`crate::fontconfig`].
pub struct FontconfigBackend;

impl FontBackend for FontconfigBackend {
    fn resolve_family(&self, family: &str) -> String {
        crate::fontconfig::resolve_family(family).to_string()
    }

    fn family_prefers_monospace(&self, family: &str) -> bool {
        crate::fontconfig::family_prefers_monospace(family)
    }

    fn match_font_for_char(
        &self,
        family: &str,
        ch: char,
        prefer_monospace: bool,
        requested_weight: u16,
        italic: bool,
    ) -> Option<PlatformFontMatch> {
        crate::fontconfig::match_font_for_char(
            family,
            ch,
            prefer_monospace,
            requested_weight,
            italic,
        )
        .and_then(PlatformFontMatch::from_fontconfig)
    }

    fn match_primary_font(
        &self,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> Option<PlatformFontMatch> {
        use neovm_core::face::{FontSlant, FontWeight};
        let slant = if italic {
            FontSlant::Italic
        } else {
            FontSlant::Normal
        };
        crate::fontconfig::find_font_for_spec(
            Some(family),
            None,
            None,
            Some(FontWeight::from_css_weight(weight)),
            Some(slant),
            Some(neovm_core::face::FontWidth::Normal),
        )
        .and_then(|matched| {
            PlatformFontMatch::from_fontconfig(crate::fontconfig::FontMatch {
                family: matched.family,
                file: matched.file,
                face_index: matched.face_index,
                variation_coords: matched.variation_coords,
                postscript_name: matched.postscript_name,
                weight: matched.weight,
                slant: matched.slant,
            })
        })
    }
}

/// The platform's default backend.
pub fn default_font_backend() -> Box<dyn FontBackend> {
    Box::new(FontconfigBackend)
}
