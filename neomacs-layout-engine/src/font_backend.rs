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

use crate::fontconfig::FontMatch;

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
    ) -> Option<FontMatch>;

    /// The font FILE the platform would open for a PRIMARY (family, weight,
    /// slant) request — fontconfig's authoritative choice. This is what
    /// `find-font` / GNU pick, and notably prefers a variable font over a
    /// same-family static face. `None` on platforms without fontconfig (the
    /// caller then keeps cosmic-text/fontdb's own selection).
    fn find_primary_font_file(&self, family: &str, weight: u16, italic: bool) -> Option<String>;
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
    ) -> Option<FontMatch> {
        crate::fontconfig::match_font_for_char(
            family,
            ch,
            prefer_monospace,
            requested_weight,
            italic,
        )
    }

    fn find_primary_font_file(&self, family: &str, weight: u16, italic: bool) -> Option<String> {
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
        )
        .and_then(|matched| matched.file)
    }
}

/// The platform's default backend.
pub fn default_font_backend() -> Box<dyn FontBackend> {
    Box::new(FontconfigBackend)
}
