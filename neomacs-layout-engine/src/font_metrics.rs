//! Cosmic-text based font metrics service for the layout engine.
//!
//! This module provides font measurement using cosmic-text, the same font
//! system used by the render thread for glyph rasterization. By using the
//! same font resolution logic for both measurement and rendering, we
//! guarantee that character widths computed during layout match the actual
//! rendered glyph widths — eliminating gaps and overlaps caused by the
//! C fontconfig and cosmic-text resolving different font files.

use crate::font_loader::FontFileCache;
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Style, Weight};
use neomacs_display_protocol::types::FaceId;

/// Safe wrapper around cosmic_text::Metrics that ensures font_size and
/// line_height are never zero.  cosmic-text panics with "line height
/// cannot be 0" if either value is 0.0.  GNU Emacs TTY frames use
/// 1x1 cell metrics; we enforce a minimum of 1.0 for safety.
fn safe_metrics(font_size: f32, line_height: f32) -> cosmic_text::Metrics {
    cosmic_text::Metrics::new(font_size.max(1.0), line_height.max(1.0))
}
use neomacs_display_protocol::font::{
    FontBackendKind, FontResolutionSource, FontSlantKind, ResolvedFont, ResolvedFontId,
    ResolvedFontIdentity, ResolvedGlyph,
};
use neovm_core::face::{FontSlant, FontWeight, FontWidth};
use std::collections::HashMap;
use ttf_parser::Face as TtfFace;

/// Font metrics returned for a given face configuration.
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    /// Baseline offset from the top of the line box.
    pub ascent: f32,
    /// Distance from the baseline to the bottom of the line box.
    pub descent: f32,
    /// Total font height in pixels.
    pub line_height: f32,
    /// Default character width (space character width for monospace)
    pub char_width: f32,
    /// Advance of the primary font's space glyph.  This is distinct from
    /// `char_width` for proportional fonts and remains the face font's value
    /// even when a concrete space character would be shaped by a fallback.
    pub space_width: f32,
}

#[derive(Debug, Clone, Copy)]
struct FontVerticalMetrics {
    ascent: f32,
    descent: f32,
    line_height: f32,
}

struct CosmicPrimaryProbe {
    file: String,
    matches_requested_family: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MetricConfidence {
    Validated,
    Degraded,
}

#[derive(Debug, Clone, Copy)]
struct FontAdvanceMetrics {
    space_width: f32,
    average_width: f32,
    max_width: f32,
    fixed_pitch: bool,
}

impl FontAdvanceMetrics {
    fn from_ascii_widths(measured_space_width: f32, ascii_widths: &[f32; 128]) -> Self {
        let mut total = 0.0;
        let mut count = 0;
        let mut min_width = f32::INFINITY;
        let mut max_width = 0.0f32;

        for width in ascii_widths[32..127].iter().copied() {
            if width.is_finite() && width > 0.0 {
                total += width;
                count += 1;
                min_width = min_width.min(width);
                max_width = max_width.max(width);
            }
        }

        let space_width = if measured_space_width.is_finite() && measured_space_width > 0.0 {
            measured_space_width
        } else {
            ascii_widths[32]
        };
        let average_width = if count > 0 { total / count as f32 } else { 0.0 };
        let min_width = if count > 0 { min_width } else { 0.0 };

        let tolerance = max_width.max(1.0) * 0.02;
        let fixed_pitch = count > 0 && (max_width - min_width).abs() <= tolerance.max(0.25);

        Self {
            space_width,
            average_width,
            max_width,
            fixed_pitch,
        }
    }

    fn from_font_probe(metrics: crate::font_probe::FontPxMetrics) -> Self {
        let space_width = metrics.space_width.max(0) as f32;
        let average_width = metrics.average_width.max(0) as f32;
        let max_width = metrics.max_width.max(0) as f32;
        let tolerance = max_width.max(1.0) * 0.02;
        let fixed_pitch = valid_advance(max_width)
            && valid_advance(average_width)
            && (max_width - average_width).abs() <= tolerance.max(0.25);
        Self {
            space_width,
            average_width,
            max_width,
            fixed_pitch,
        }
    }

    fn monospace_column_width(self, minimum_width: f32) -> Option<FrameColumnWidth> {
        if valid_advance(self.max_width) && self.max_width >= minimum_width {
            return Some(if self.fixed_pitch {
                FrameColumnWidth::validated(self.max_width)
            } else {
                FrameColumnWidth::degraded(self.max_width)
            });
        }
        if valid_advance(self.average_width) && self.average_width >= minimum_width {
            return Some(FrameColumnWidth::validated(self.average_width));
        }
        if valid_advance(self.space_width) && self.space_width >= minimum_width {
            return Some(FrameColumnWidth::validated(self.space_width));
        }
        None
    }

    fn proportional_column_width(self) -> Option<FrameColumnWidth> {
        if valid_advance(self.average_width) {
            return Some(FrameColumnWidth::validated(self.average_width));
        }
        if valid_advance(self.space_width) {
            return Some(FrameColumnWidth::validated(self.space_width));
        }
        None
    }
}

#[derive(Debug, Clone, Copy)]
struct FrameColumnWidth {
    pixels: f32,
    confidence: MetricConfidence,
}

impl FrameColumnWidth {
    fn from_advances(family: &str, font_size: f32, advances: FontAdvanceMetrics) -> Self {
        let fallback = Self::degraded((font_size * 0.6).max(1.0));
        let selected = if crate::fontconfig::family_prefers_monospace(family) {
            advances.monospace_column_width(font_size * 0.5)
        } else {
            advances.proportional_column_width()
        };

        selected
            .filter(|width| valid_advance(width.pixels))
            .unwrap_or(fallback)
    }

    fn validated(pixels: f32) -> Self {
        Self {
            pixels,
            confidence: MetricConfidence::Validated,
        }
    }

    fn degraded(pixels: f32) -> Self {
        Self {
            pixels,
            confidence: MetricConfidence::Degraded,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FrameCellMetrics {
    column_width: f32,
    line_height: f32,
    ascent: f32,
    descent: f32,
    confidence: MetricConfidence,
}

impl FrameCellMetrics {
    fn derive(
        family: &str,
        font_size: f32,
        vertical: FontVerticalMetrics,
        advances: FontAdvanceMetrics,
    ) -> Self {
        let column = FrameColumnWidth::from_advances(family, font_size, advances);

        Self {
            column_width: column.pixels,
            line_height: vertical.line_height,
            ascent: vertical.ascent,
            descent: vertical.descent,
            confidence: column.confidence,
        }
    }
}

fn valid_advance(width: f32) -> bool {
    width.is_finite() && width > 0.0
}

fn is_generic_font_family(family: &str) -> bool {
    matches!(
        family.trim().to_ascii_lowercase().as_str(),
        "" | "mono" | "monospace" | "serif" | "sans" | "sans-serif" | "sans serif"
    )
}

fn fontdb_face_file(face: &fontdb::FaceInfo) -> Option<String> {
    match &face.source {
        fontdb::Source::Binary(_) => None,
        fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) => {
            Some(path.display().to_string())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedFontInfo {
    pub family: String,
    pub foundry: Option<String>,
    pub file: Option<String>,
    pub postscript_name: Option<String>,
    pub weight: FontWeight,
    pub slant: FontSlant,
    pub width: FontWidth,
}

/// One shaped glyph produced by [`FontMetricsService::shape_run`]: the
/// resolved font glyph plus its position, advance, and the byte range of
/// the source text it covers (its cluster). This is neomacs's layout-side
/// equivalent of a GNU lglyph (CODE / WIDTH / cluster FROM..TO) — the
/// per-glyph output of running HarfBuzz-class shaping over a text run.
/// It is the building block of the composed glyph rows that contextual
/// scripts (Arabic, Indic) and programming ligatures need.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph {
    /// Resolved font the glyph belongs to (after fallback).
    pub font_id: fontdb::ID,
    /// Glyph index within `font_id`.
    pub glyph_id: u16,
    /// Pen x offset of the glyph within the run, in pixels from the origin.
    pub x: f32,
    /// Pen y offset (baseline-relative), in pixels.
    pub y: f32,
    /// Horizontal advance of the glyph, in pixels.
    pub x_advance: f32,
    /// Start byte index (inclusive) of the source cluster in the shaped text.
    pub cluster_start: usize,
    /// End byte index (exclusive) of the source cluster in the shaped text.
    pub cluster_end: usize,
}

/// Cache key for font metrics lookups.
/// Groups: (family, weight, italic, font_size_centipx)
/// font_size is stored as integer centipixels (size * 100) to avoid float key issues.
#[derive(Hash, Eq, PartialEq, Clone)]
struct MetricsCacheKey {
    family: String,
    weight: u16,
    italic: bool,
    font_size_centipx: i32,
}

#[derive(Debug, Clone)]
struct ResolvedCharFont {
    family: String,
    weight: u16,
    slant: FontSlant,
}

impl MetricsCacheKey {
    fn new(family: &str, weight: u16, italic: bool, font_size: f32) -> Self {
        Self {
            family: family.to_string(),
            weight,
            italic,
            font_size_centipx: (font_size * 100.0) as i32,
        }
    }
}

/// Upper bound on `shaped_run_cache` entries before it is cleared, mirroring
/// GNU's bounded composition cache. Shaped runs are typically words or
/// property spans, so a frame's working set stays well under this; clearing on
/// overflow keeps memory bounded without per-entry LRU bookkeeping.
const SHAPED_RUN_CACHE_CAP: usize = 8192;

/// Cosmic-text based font metrics service.
///
/// Runs on the Emacs/layout thread. Creates its own `FontSystem` which scans
/// the same fontconfig database as the render thread's `FontSystem`, ensuring
/// identical font resolution.
pub struct FontMetricsService {
    font_system: FontSystem,
    /// Cache: face attrs → ASCII advance widths (chars 0-127)
    ascii_cache: HashMap<MetricsCacheKey, [f32; 128]>,
    /// Cache: face attrs → single char width (for non-ASCII)
    char_cache: HashMap<(MetricsCacheKey, char), f32>,
    /// Cache: face attrs → font metrics (ascent, descent, etc.)
    metrics_cache: HashMap<MetricsCacheKey, FontMetrics>,
    /// Interned font family strings for cosmic-text Attrs (requires 'static)
    interned_families: HashMap<String, &'static str>,
    /// Cache for pre-loading font files and resolving fontdb family names
    font_file_cache: FontFileCache,
    /// Cache: (face, run text) → shaped glyphs. A run is shaped by BOTH the
    /// measure pass (wrap/cursor advance) and the render pass (glyph
    /// production); this makes the second a cache hit so cosmic-text shapes
    /// each (run, face) once instead of twice. Keyed on the same integer-centipx
    /// face identity as the advance caches, so two runs with identical text but
    /// different faces never share an entry.
    ///
    /// Like the `ascii_cache`/`char_cache`/`metrics_cache`, entries are only
    /// valid for the current fontdb generation: `clear_caches` drops them on a
    /// font change, but `prime_file` (reachable from `resolve_family` /
    /// `resolve_font_for_char`) can load a font mid-session WITHOUT invalidating
    /// the cache. Production primes a face's file before shaping it, so a stale
    /// entry does not arise in practice; this matches the existing advance
    /// caches' unstated contract. The cached `font_id`/`glyph_id` are likewise
    /// only valid for that generation — production consumers read only
    /// `cluster_start`/`x_advance` (see `DisplayTextRunClusterAdvances`), so a
    /// stale entry degrades to a stale advance, never a wrong rasterized glyph.
    /// Do NOT thread the cached glyph ids into rasterization without adding
    /// fontdb-generation keying (cf. `font_match::resolve_weight_in_family`,
    /// which folds `db().len()` into its key for exactly this reason).
    shaped_run_cache: HashMap<(MetricsCacheKey, String), Vec<ShapedGlyph>>,
    /// Entry cap for `shaped_run_cache` before clear-on-overflow. Defaults to
    /// `SHAPED_RUN_CACHE_CAP`; lowered by tests to exercise the overflow path.
    shaped_run_cache_cap: usize,
    /// Number of actual cosmic-text shaping invocations (`shaped_run_cache`
    /// misses). Lets tests prove the measure/render double-shape is deduped.
    n_shape_calls: usize,
    /// Platform font backend: family alias resolution + per-char coverage
    /// matching (design §7). Linux: fontconfig.
    backend: Box<dyn crate::font_backend::FontBackend>,
    /// Shaping engine behind the TextShaper seam (design §8).
    shaper: Box<dyn crate::text_shaper::TextShaper>,
    /// Cache: face attrs → the face's resolved primary font. Same generation
    /// contract as the other caches: cleared by `clear_caches`.
    resolved_face_font_cache: HashMap<MetricsCacheKey, Option<ResolvedFont>>,
    /// Interner: exact font identity → stable [`ResolvedFontId`]. NOT cleared
    /// by `clear_caches`: ids stay stable for the service's lifetime so
    /// consecutive frame snapshots reference the same font by the same id.
    /// Renderer caches key on the identity anyway, so a stale id can never
    /// alias a glyph to the wrong font.
    resolved_font_ids: HashMap<ResolvedFontIdentity, ResolvedFontId>,
    /// Cache: (face attrs, char) → the char's resolved fallback font. Same
    /// generation contract as the other caches: cleared by `clear_caches`.
    resolved_char_font_cache: HashMap<(MetricsCacheKey, char), Option<ResolvedFont>>,
    /// Cache: (face attrs, cluster text) → shaped glyphs with interned font
    /// identities. Same generation contract; clear-on-overflow like
    /// `shaped_run_cache`.
    // A `type` alias for this cache value would not materially aid readability.
    #[allow(clippy::type_complexity)]
    resolved_cluster_cache:
        HashMap<(MetricsCacheKey, String), Option<(Vec<ResolvedGlyph>, Vec<ResolvedFont>)>>,
    /// Cache: `"{file}#{index}"` → a synthetic fontdb family name registered
    /// for that exact face, so cosmic-text selects THAT file verbatim (see
    /// [`Self::pin_file_as_family`]). NOT cleared by `clear_caches`: the
    /// pinned faces live in the fontdb for the service's lifetime.
    pinned_families: HashMap<String, &'static str>,
    /// Cache: (family, weight, italic) → the synthetic family to use when
    /// fontconfig's chosen file differs from cosmic-text's own pick
    /// (`Some`), or `None` when they agree (the common case, no pinning).
    /// See [`Self::pinned_primary_family`].
    primary_pin_cache: HashMap<(String, u16, bool), Option<&'static str>>,
}

/// Whether primary-font pinning is enabled (default on). Pinning routes the
/// primary font through fontconfig's file choice — matching GNU/`find-font`,
/// which prefer a variable font over a same-family static face. Set
/// `NEOMACS_DISABLE_FONT_PIN` to fall back to cosmic-text/fontdb selection.
fn font_pin_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("NEOMACS_DISABLE_FONT_PIN").is_none())
}

impl Default for FontMetricsService {
    fn default() -> Self {
        Self::new()
    }
}

impl FontMetricsService {
    /// Create a new FontMetricsService.
    ///
    /// This scans the system font database via fontconfig, which takes ~50ms.
    /// Should be lazily initialized on first use.
    pub fn new() -> Self {
        tracing::info!("FontMetricsService: initializing cosmic-text FontSystem");
        let font_system = FontSystem::new();
        tracing::info!("FontMetricsService: FontSystem ready");
        Self {
            font_system,
            ascii_cache: HashMap::new(),
            char_cache: HashMap::new(),
            metrics_cache: HashMap::new(),
            interned_families: HashMap::new(),
            font_file_cache: FontFileCache::new(),
            shaped_run_cache: HashMap::new(),
            shaped_run_cache_cap: SHAPED_RUN_CACHE_CAP,
            n_shape_calls: 0,
            backend: crate::font_backend::default_font_backend(),
            shaper: crate::text_shaper::default_text_shaper(),
            resolved_face_font_cache: HashMap::new(),
            resolved_font_ids: HashMap::new(),
            resolved_char_font_cache: HashMap::new(),
            resolved_cluster_cache: HashMap::new(),
            pinned_families: HashMap::new(),
            primary_pin_cache: HashMap::new(),
        }
    }

    /// Register the exact font FILE (a specific face index of it) under a
    /// unique synthetic fontdb family, so `Attrs::family(Family::Name(..))`
    /// selects THAT face verbatim instead of cosmic-text re-picking among
    /// every face that shares the real family name.
    ///
    /// This is how we pin the file fontconfig chose: GNU/fontconfig prefer a
    /// variable font's named instance over a same-family static face, but
    /// cosmic-text/fontdb would pick the static exact-weight face. We load
    /// the file, clone the target face's metadata under a synthetic family,
    /// and drop the freshly-loaded originals so the real family name is not
    /// duplicated. Cached per `(file, index)`; returns the interned
    /// synthetic family name, or `None` if the file can't be loaded.
    fn pin_file_as_family(&mut self, file: &str, face_index: u32) -> Option<&'static str> {
        let key = format!("{file}#{face_index}");
        if let Some(&existing) = self.pinned_families.get(&key) {
            return Some(existing);
        }
        let synthetic = format!("neomacs-pin-{}", self.pinned_families.len());
        {
            let db = self.font_system.db_mut();
            let ids = db.load_font_source(fontdb::Source::File(file.into()));
            let target = ids
                .iter()
                .copied()
                .find(|&id| db.face(id).map(|f| f.index) == Some(face_index))
                .or_else(|| ids.first().copied())?;
            let mut info = db.face(target)?.clone();
            // Drop the copies we just loaded; the file's real-family faces
            // from the initial system scan stay untouched, so pinning never
            // adds a duplicate "Noto Sans" the normal path could pick.
            for id in &ids {
                db.remove_face(*id);
            }
            info.families = vec![(synthetic.clone(), fontdb::Language::English_UnitedStates)];
            db.push_face_info(info);
        }
        let interned = self.intern_family(&synthetic);
        self.pinned_families.insert(key, interned);
        Some(interned)
    }

    /// Resolve the effective font family name for a face.
    ///
    /// If `font_file_path` is provided, pre-loads the exact font file into fontdb
    /// while preserving the exact family name that Fontconfig selected.
    pub fn resolve_family(&mut self, emacs_family: &str, font_file_path: Option<&str>) -> String {
        if let Some(path) = font_file_path {
            let _ = self.font_file_cache.prime_file(&mut self.font_system, path);
        }
        emacs_family.to_string()
    }

    fn intern_family(&mut self, family: &str) -> &'static str {
        if let Some(&existing) = self.interned_families.get(family) {
            existing
        } else {
            let leaked: &'static str = Box::leak(family.to_string().into_boxed_str());
            self.interned_families.insert(family.to_string(), leaked);
            leaked
        }
    }

    /// Build cosmic-text `Attrs` from face parameters.
    /// Mirrors the logic in `glyph_atlas.rs:face_to_attrs()`.
    ///
    /// When fontconfig's authoritative file for this (family, weight, slant)
    /// differs from what cosmic-text/fontdb would otherwise pick, pin
    /// fontconfig's file under a synthetic family so shaping and metrics use
    /// the same primary font GNU opens. In the common case (they agree) this
    /// is byte-identical to the plain path.
    fn build_attrs(&mut self, family: &str, weight: u16, slant: FontSlant) -> Attrs<'static> {
        if let Some(synthetic) = self.pinned_primary_family(family, weight, slant.is_italic()) {
            let effective_weight = crate::font_match::resolve_weight_in_family(
                &self.font_system,
                synthetic,
                weight,
                slant.is_italic(),
            );
            let mut attrs = Attrs::new()
                .family(Family::Name(synthetic))
                .weight(Weight(effective_weight));
            if let Some(style) = font_slant_to_cosmic_style(slant) {
                attrs = attrs.style(style);
            }
            return attrs;
        }
        self.build_attrs_unpinned(family, weight, slant)
    }

    fn build_attrs_unpinned(
        &mut self,
        family: &str,
        weight: u16,
        slant: FontSlant,
    ) -> Attrs<'static> {
        let mut attrs = Attrs::new();

        attrs = match crate::font_match::select_cosmic_family(&self.font_system, family) {
            crate::font_match::CosmicFamilySelection::Name(family) => {
                let interned = self.intern_family(family);
                attrs.family(Family::Name(interned))
            }
            crate::font_match::CosmicFamilySelection::Monospace => attrs.family(Family::Monospace),
            crate::font_match::CosmicFamilySelection::Serif => attrs.family(Family::Serif),
            crate::font_match::CosmicFamilySelection::SansSerif => attrs.family(Family::SansSerif),
        };

        // Font weight (CSS 100-900): clamp to closest available in this family.
        let effective_weight = crate::font_match::resolve_weight_in_family(
            &self.font_system,
            family,
            weight,
            slant.is_italic(),
        );
        attrs = attrs.weight(Weight(effective_weight));

        // Font style
        if let Some(style) = font_slant_to_cosmic_style(slant) {
            attrs = attrs.style(style)
        }

        attrs
    }

    /// The synthetic family to shape a primary (family, weight, italic)
    /// request through when fontconfig's file differs from cosmic-text's own
    /// pick, else `None` (agree → no pinning, unchanged behavior). Cached.
    fn pinned_primary_family(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> Option<&'static str> {
        if !font_pin_enabled() {
            return None;
        }
        let key = (family.to_string(), weight, italic);
        if let Some(&cached) = self.primary_pin_cache.get(&key) {
            return cached;
        }
        let result = self.compute_primary_pin(family, weight, italic);
        self.primary_pin_cache.insert(key, result);
        result
    }

    fn compute_primary_pin(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> Option<&'static str> {
        let fc_file = self
            .backend
            .find_primary_font_file(family, weight, italic)?;
        // What file would cosmic-text/fontdb pick on its own?
        let cosmic = self.cosmic_primary_probe(family, weight, italic)?;
        // GNU's platform font backend is authoritative for the primary file.
        // Correct the two disagreements we can identify without replacing
        // fontdb's ordinary same-family selection policy: fontconfig chose a
        // variable instance, or fontdb missed an explicitly named family and
        // silently shaped with a generic fallback.
        if cosmic.file == fc_file {
            return None;
        }
        let fontconfig_selected_variable =
            !crate::font_probe::named_instance_wght_values(&fc_file, 0).is_empty();
        if !fontconfig_selected_variable
            && (is_generic_font_family(family) || cosmic.matches_requested_family)
        {
            return None;
        }
        tracing::debug!(
            target: "font_boundary",
            family,
            weight,
            italic,
            fontconfig_file = %fc_file,
            cosmic_file = %cosmic.file,
            "primary-font pin: fontconfig and fontdb disagree; pinning fontconfig file"
        );
        self.pin_file_as_family(&fc_file, 0)
    }

    /// Probe the platform-selected primary face when fontdb cannot represent
    /// it as the primary ASCII font.  Symbol-only fonts are the important
    /// case: shaping an ASCII probe necessarily falls through to another
    /// font, but GNU still uses the requested face font's global metrics for
    /// stretch spaces produced by `(space-width ...)`.
    fn platform_primary_metrics_override(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> Option<crate::font_probe::FontPxMetrics> {
        if is_generic_font_family(family) {
            return None;
        }
        let platform_file = self
            .backend
            .find_primary_font_file(family, weight, italic)?;
        let cosmic = self.cosmic_primary_probe(family, weight, italic)?;
        if cosmic.file == platform_file || cosmic.matches_requested_family {
            return None;
        }
        let variable_weight = (!crate::font_probe::named_instance_wght_values(&platform_file, 0)
            .is_empty())
        .then_some(f32::from(weight));
        crate::font_probe::probe_font_px_metrics(
            &platform_file,
            0,
            font_size.round().max(1.0) as u32,
            variable_weight,
        )
    }

    /// The font file cosmic-text/fontdb selects on its own for this request
    /// (probe by shaping a representative ASCII glyph, unpinned).
    #[cfg(test)]
    fn cosmic_probe_file(&mut self, family: &str, weight: u16, italic: bool) -> Option<String> {
        self.cosmic_primary_probe(family, weight, italic)
            .map(|probe| probe.file)
    }

    fn cosmic_primary_probe(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> Option<CosmicPrimaryProbe> {
        let slant = if italic {
            FontSlant::Italic
        } else {
            FontSlant::Normal
        };
        let attrs = self.build_attrs_unpinned(family, weight, slant);
        let metrics = safe_metrics(24.0, 24.0 * 1.3);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(&mut self.font_system, Some(96.0), Some(48.0));
        buffer.set_text(
            &mut self.font_system,
            "n",
            &attrs,
            cosmic_text::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
        let font_id = buffer
            .layout_runs()
            .find_map(|run| run.glyphs.iter().next())?
            .physical((0.0, 0.0), 1.0)
            .cache_key
            .font_id;
        let face = self.font_system.db().face(font_id)?;
        Some(CosmicPrimaryProbe {
            file: fontdb_face_file(face)?,
            matches_requested_family: face
                .families
                .iter()
                .any(|(name, _)| name.eq_ignore_ascii_case(family)),
        })
    }

    fn selected_font_id_and_space_width(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> (Option<fontdb::ID>, f32) {
        let attrs = self.build_attrs(
            family,
            weight,
            if italic {
                FontSlant::Italic
            } else {
                FontSlant::Normal
            },
        );
        let metrics = safe_metrics(font_size, font_size * 1.3);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(
            &mut self.font_system,
            Some(font_size * 4.0),
            Some(font_size * 2.0),
        );
        buffer.set_text(
            &mut self.font_system,
            " ",
            &attrs,
            cosmic_text::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);

        for run in buffer.layout_runs() {
            if let Some(glyph) = run.glyphs.first() {
                return (
                    Some(glyph.physical((0.0, 0.0), 1.0).cache_key.font_id),
                    glyph.w,
                );
            }
        }

        (None, font_size * 0.6)
    }

    /// Shape a run of `text` with the given face attributes and return its
    /// glyphs in visual order, with positions, advances, and source-cluster
    /// byte ranges.
    ///
    /// This is the layout-side counterpart of GNU's `font-shape-gstring` and
    /// the font driver's `->shape` method: it runs cosmic-text's
    /// `Shaping::Advanced` (HarfBuzz-class) so contextual scripts (Arabic
    /// joining, Indic reordering) and ligatures resolve correctly across the
    /// whole run rather than per character. The cluster byte ranges map each
    /// glyph back to the characters it covers, which a composed glyph row
    /// needs for cursor positioning. Returns an empty vec for empty text.
    pub fn shape_run(
        &mut self,
        text: &str,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> Vec<ShapedGlyph> {
        if text.is_empty() {
            return Vec::new();
        }
        let key = (
            MetricsCacheKey::new(family, weight, italic, font_size),
            text.to_string(),
        );
        if let Some(cached) = self.shaped_run_cache.get(&key) {
            return cached.clone();
        }
        self.n_shape_calls += 1;
        let attrs = self.build_attrs(
            family,
            weight,
            if italic {
                FontSlant::Italic
            } else {
                FontSlant::Normal
            },
        );
        let glyphs = self.shaper.shape_run(
            &mut self.font_system,
            text,
            &attrs,
            font_size.max(1.0),
            font_size.max(1.0) * 1.3,
        );
        if self.shaped_run_cache.len() >= self.shaped_run_cache_cap {
            self.shaped_run_cache.clear();
        }
        self.shaped_run_cache.insert(key, glyphs.clone());
        glyphs
    }

    fn font_metrics_from_selected_face(
        &mut self,
        font_id: fontdb::ID,
        font_size: f32,
    ) -> Option<FontVerticalMetrics> {
        let probe_target = self
            .font_system
            .db()
            .face(font_id)
            .and_then(|face| fontdb_face_file(face).map(|file| (file, face.index)));
        if let Some((file, face_index)) = probe_target {
            let pixel_size = font_size.round().max(1.0) as u32;
            if let Some(metrics) =
                crate::font_probe::probe_font_px_metrics(&file, face_index, pixel_size, None)
            {
                return Some(FontVerticalMetrics {
                    ascent: metrics.ascent.max(0) as f32,
                    descent: metrics.descent.max(0) as f32,
                    line_height: metrics.height.max(1) as f32,
                });
            }
        }

        self.font_system
            .db()
            .with_face_data(font_id, |font_data, face_index| {
                let face = TtfFace::parse(font_data, face_index).ok()?;
                let units_per_em = face.units_per_em().max(1) as f32;
                let scale = font_size / units_per_em;
                // GNU GUI backends publish frame line height as the font
                // backend's integer ascent plus integer descent.  Do the
                // same here instead of trusting the typographic height table
                // or a synthetic multiplier.
                let ascent = (face.ascender() as f32 * scale).ceil().max(0.0);
                let descent = (-(face.descender() as f32) * scale).ceil().max(0.0);
                let line_height = (ascent + descent).max(1.0);

                // GNU xdisp.c prefers font-global metrics (FONT_BASE /
                // FONT_DESCENT) and only falls back to per-glyph extents for
                // pathological fonts. Reject obviously bogus table data here
                // and let the caller fall back to glyph-box probing.
                if !ascent.is_finite()
                    || !descent.is_finite()
                    || !line_height.is_finite()
                    || ascent <= 0.0
                    || descent <= 0.0
                    || line_height <= 0.0
                    || line_height > font_size * 4.0
                {
                    return None;
                }

                Some(FontVerticalMetrics {
                    ascent,
                    descent,
                    line_height,
                })
            })
            .flatten()
    }

    pub fn select_font_for_char(
        &mut self,
        ch: char,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> Option<SelectedFontInfo> {
        let resolved = self.resolve_font_for_char(ch, family, weight, italic);
        let attrs = self.build_attrs(&resolved.family, resolved.weight, resolved.slant);
        let line_height = font_size * 1.3;
        let metrics = safe_metrics(font_size, line_height);

        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(
            &mut self.font_system,
            Some(font_size * 4.0),
            Some(font_size * 2.0),
        );

        let text = String::from(ch);
        buffer.set_text(
            &mut self.font_system,
            &text,
            &attrs,
            cosmic_text::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);

        let glyph = buffer
            .layout_runs()
            .find_map(|run| run.glyphs.iter().next())?;
        let face = self
            .font_system
            .db()
            .face(glyph.physical((0.0, 0.0), 1.0).cache_key.font_id)?;
        let file = fontdb_face_file(face);
        Some(SelectedFontInfo {
            foundry: file
                .as_deref()
                .and_then(crate::fontconfig::foundry_for_file),
            // TTC/variable collections frequently expose several regional
            // aliases, and fontdb may report the file's first alias instead of
            // the family we explicitly resolved for this character. Preserve
            // the selector's family so `font-at` mirrors GNU Emacs' realized
            // face semantics.
            family: resolved.family.clone(),
            file,
            postscript_name: Some(face.post_script_name.clone()).filter(|name| !name.is_empty()),
            // Variable fonts often report the container face's metadata weight
            // here even when shaping used a different requested instance.
            // Preserve the resolved CSS weight so `font-at` mirrors GNU Emacs'
            // realized face semantics.
            weight: FontWeight::from_css_weight(resolved.weight),
            slant: font_slant_from_fontdb(face.style),
            width: font_width_from_stretch_number(face.stretch.to_number()),
        })
    }

    /// Resolve a face's primary font to an exact identity.
    ///
    /// This is the face-level half of the render-boundary design: the same
    /// probe path that produces the face's layout metrics
    /// (`selected_font_id_and_space_width`) yields the concrete fontdb face,
    /// so the identity the renderer rasterizes is the font the metrics came
    /// from by construction. GNU analog: `font_open_for_lface` filling the
    /// realized `face->font` that both `font-at` and the draw path consume.
    pub fn resolved_font_for_face(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> Option<ResolvedFont> {
        let key = MetricsCacheKey::new(family, weight, italic, font_size);
        if let Some(cached) = self.resolved_face_font_cache.get(&key) {
            return cached.clone();
        }
        let resolved = self.resolve_face_font_uncached(family, weight, italic, font_size);
        self.resolved_face_font_cache.insert(key, resolved.clone());
        resolved
    }

    fn resolve_face_font_uncached(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> Option<ResolvedFont> {
        // Same alias + probe-shape path as the ASCII metrics code
        // (`resolve_font_for_char`), so identity == metrics font.
        let resolved_family = self.resolve_family(&self.backend.resolve_family(family), None);
        let (font_id, _space_width) =
            self.selected_font_id_and_space_width(&resolved_family, weight, italic, font_size);
        let font_id = font_id?;
        let effective_weight = crate::font_match::resolve_weight_in_family(
            &self.font_system,
            &resolved_family,
            weight,
            italic,
        );
        let vertical = self.font_metrics_from_selected_face(font_id, font_size);
        let (file, face_index, postscript_name, style, stretch) = {
            let face = self.font_system.db().face(font_id)?;
            (
                fontdb_face_file(face),
                face.index,
                Some(face.post_script_name.clone()).filter(|name| !name.is_empty()),
                face.style,
                face.stretch,
            )
        };
        let identity = match file.as_deref() {
            Some(path) => {
                ResolvedFontIdentity::from_file(path, face_index, postscript_name.clone())
            }
            // fontdb Source::Binary faces have no path; key on the
            // postscript name (or family) so the identity is still durable.
            None => ResolvedFontIdentity {
                backend: FontBackendKind::Fontconfig,
                stable_key: format!(
                    "mem:{}#{face_index}",
                    postscript_name.as_deref().unwrap_or(&resolved_family)
                ),
                file_path: None,
                face_index,
                postscript_name: postscript_name.clone(),
                variation_coords: Vec::new(),
            },
        };
        let id = self.intern_resolved_font_id(&identity);
        Some(ResolvedFont {
            id,
            identity,
            family: resolved_family,
            full_name: None,
            postscript_name,
            // Preserve the resolved CSS weight, not the container face's
            // metadata weight (variable fonts; cf. `select_font_for_char`).
            weight: effective_weight,
            slant: font_slant_kind_from_fontdb(style),
            width: stretch.to_number(),
            pixel_size: font_size,
            ascent_px: vertical.as_ref().map(|v| v.ascent).unwrap_or(0.0),
            descent_px: vertical.as_ref().map(|v| v.descent).unwrap_or(0.0),
            source: FontResolutionSource::FacePrimary,
        })
    }

    /// Resolve the covering font for one character under a face, as an exact
    /// interned identity.
    ///
    /// This is the fallback half of the render-boundary design: it runs the
    /// SAME per-char resolution the measurement path uses
    /// (`resolve_font_for_char` → fontconfig `match_font_for_char`) and then
    /// pins the concrete fontdb face a probe shape of `ch` selects, so the
    /// published identity is the font the char's advance width came from.
    /// GNU analog: `fontset_font` realizing the fontset entry for a
    /// character.
    pub fn resolved_font_for_char(
        &mut self,
        ch: char,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> Option<ResolvedFont> {
        let key = (MetricsCacheKey::new(family, weight, italic, font_size), ch);
        if let Some(cached) = self.resolved_char_font_cache.get(&key) {
            return cached.clone();
        }
        let resolved = self.resolve_char_font_uncached(ch, family, weight, italic, font_size);
        self.resolved_char_font_cache.insert(key, resolved.clone());
        resolved
    }

    fn resolve_char_font_uncached(
        &mut self,
        ch: char,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> Option<ResolvedFont> {
        let resolved = self.resolve_font_for_char(ch, family, weight, italic);
        let attrs = self.build_attrs(&resolved.family, resolved.weight, resolved.slant);
        let metrics = safe_metrics(font_size, font_size * 1.3);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(
            &mut self.font_system,
            Some(font_size * 4.0),
            Some(font_size * 2.0),
        );
        let text = String::from(ch);
        buffer.set_text(
            &mut self.font_system,
            &text,
            &attrs,
            cosmic_text::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
        let font_id = buffer
            .layout_runs()
            .find_map(|run| run.glyphs.iter().next())?
            .physical((0.0, 0.0), 1.0)
            .cache_key
            .font_id;
        let vertical = self.font_metrics_from_selected_face(font_id, font_size);
        let (file, face_index, postscript_name, style, stretch) = {
            let face = self.font_system.db().face(font_id)?;
            (
                fontdb_face_file(face),
                face.index,
                Some(face.post_script_name.clone()).filter(|name| !name.is_empty()),
                face.style,
                face.stretch,
            )
        };
        let identity = match file.as_deref() {
            Some(path) => {
                ResolvedFontIdentity::from_file(path, face_index, postscript_name.clone())
            }
            None => ResolvedFontIdentity {
                backend: FontBackendKind::Fontconfig,
                stable_key: format!(
                    "mem:{}#{face_index}",
                    postscript_name.as_deref().unwrap_or(&resolved.family)
                ),
                file_path: None,
                face_index,
                postscript_name: postscript_name.clone(),
                variation_coords: Vec::new(),
            },
        };
        let id = self.intern_resolved_font_id(&identity);
        Some(ResolvedFont {
            id,
            identity,
            family: resolved.family.clone(),
            full_name: None,
            postscript_name,
            weight: resolved.weight,
            slant: font_slant_kind_from_fontdb(style),
            width: stretch.to_number(),
            pixel_size: font_size,
            ascent_px: vertical.as_ref().map(|v| v.ascent).unwrap_or(0.0),
            descent_px: vertical.as_ref().map(|v| v.descent).unwrap_or(0.0),
            source: FontResolutionSource::FontsetFallback,
        })
    }

    /// Shape a composed cluster and return its glyphs with exact interned
    /// font identities plus the distinct fonts they reference — the
    /// renderable payload the render thread rasterizes without re-shaping.
    ///
    /// Shapes the cluster text standalone (the same input the renderer's
    /// composed path uses), so replaying these glyphs reproduces current
    /// visual behavior with the re-selection risk removed.
    pub fn resolved_glyphs_for_cluster(
        &mut self,
        text: &str,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> Option<(Vec<ResolvedGlyph>, Vec<ResolvedFont>)> {
        if text.is_empty() {
            return None;
        }
        let key = (
            MetricsCacheKey::new(family, weight, italic, font_size),
            text.to_string(),
        );
        if let Some(cached) = self.resolved_cluster_cache.get(&key) {
            return cached.clone();
        }
        let result = self.resolve_cluster_uncached(text, family, weight, italic, font_size);
        if self.resolved_cluster_cache.len() >= self.shaped_run_cache_cap {
            self.resolved_cluster_cache.clear();
        }
        self.resolved_cluster_cache.insert(key, result.clone());
        result
    }

    fn resolve_cluster_uncached(
        &mut self,
        text: &str,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> Option<(Vec<ResolvedGlyph>, Vec<ResolvedFont>)> {
        // Route the cluster through the same representative-char font
        // resolution the renderer applies (emoji presentation via U+FE0F →
        // the color emoji font, CJK → covering font), so e.g. an emoji
        // keycap shapes to the emoji font's single color glyph instead of
        // the face font's digit + combining-keycap parts.
        let (effective_family, effective_weight, effective_italic) =
            match crate::composition::representative_char_for_cluster(text) {
                Some(repr) => {
                    let resolved = self.resolve_font_for_char(repr, family, weight, italic);
                    let italic = resolved.slant.is_italic();
                    (resolved.family, resolved.weight, italic)
                }
                None => (family.to_string(), weight, italic),
            };
        let shaped = self.shape_run(
            text,
            &effective_family,
            effective_weight,
            effective_italic,
            font_size,
        );
        if shaped.is_empty() {
            return None;
        }
        let mut fonts: Vec<ResolvedFont> = Vec::new();
        let mut by_fontdb: HashMap<fontdb::ID, ResolvedFontId> = HashMap::new();
        let mut glyphs = Vec::with_capacity(shaped.len());
        for shaped_glyph in &shaped {
            let resolved_font_id = match by_fontdb.get(&shaped_glyph.font_id) {
                Some(&id) => id,
                None => {
                    // The generation-local fontdb::ID becomes a durable
                    // identity here, immediately, in the same generation
                    // that shaped it — the conversion the ShapedGlyph docs
                    // require before any glyph id reaches rasterization.
                    let font = self.resolved_font_from_fontdb_id(
                        shaped_glyph.font_id,
                        font_size,
                        FontResolutionSource::FontsetFallback,
                    )?;
                    let id = font.id;
                    by_fontdb.insert(shaped_glyph.font_id, id);
                    if !fonts.iter().any(|f| f.id == id) {
                        fonts.push(font);
                    }
                    id
                }
            };
            glyphs.push(ResolvedGlyph {
                resolved_font_id,
                glyph_id: shaped_glyph.glyph_id,
                x: shaped_glyph.x,
                y: shaped_glyph.y,
                x_advance: shaped_glyph.x_advance,
                cluster_start: shaped_glyph.cluster_start as u32,
                cluster_end: shaped_glyph.cluster_end as u32,
            });
        }
        Some((glyphs, fonts))
    }

    /// Build a [`ResolvedFont`] for a concrete fontdb face chosen by
    /// shaping. Unlike the face/char resolvers (which preserve selector
    /// family/weight semantics), this records the file's own metadata: the
    /// font was picked by shaping fallback, not by a request.
    fn resolved_font_from_fontdb_id(
        &mut self,
        font_id: fontdb::ID,
        font_size: f32,
        source: FontResolutionSource,
    ) -> Option<ResolvedFont> {
        let vertical = self.font_metrics_from_selected_face(font_id, font_size);
        let (file, face_index, postscript_name, style, stretch, family, file_weight) = {
            let face = self.font_system.db().face(font_id)?;
            (
                fontdb_face_file(face),
                face.index,
                Some(face.post_script_name.clone()).filter(|name| !name.is_empty()),
                face.style,
                face.stretch,
                face.families
                    .first()
                    .map(|(name, _)| name.clone())
                    .unwrap_or_default(),
                face.weight.0,
            )
        };
        let identity = match file.as_deref() {
            Some(path) => {
                ResolvedFontIdentity::from_file(path, face_index, postscript_name.clone())
            }
            None => ResolvedFontIdentity {
                backend: FontBackendKind::Fontconfig,
                stable_key: format!(
                    "mem:{}#{face_index}",
                    postscript_name.as_deref().unwrap_or(&family)
                ),
                file_path: None,
                face_index,
                postscript_name: postscript_name.clone(),
                variation_coords: Vec::new(),
            },
        };
        let id = self.intern_resolved_font_id(&identity);
        Some(ResolvedFont {
            id,
            identity,
            family,
            full_name: None,
            postscript_name,
            weight: file_weight,
            slant: font_slant_kind_from_fontdb(style),
            width: stretch.to_number(),
            pixel_size: font_size,
            ascent_px: vertical.as_ref().map(|v| v.ascent).unwrap_or(0.0),
            descent_px: vertical.as_ref().map(|v| v.descent).unwrap_or(0.0),
            source,
        })
    }

    fn intern_resolved_font_id(&mut self, identity: &ResolvedFontIdentity) -> ResolvedFontId {
        if let Some(&id) = self.resolved_font_ids.get(identity) {
            return id;
        }
        // Ids start at 1; 0 stays unused so an uninitialized id is visible.
        let id = ResolvedFontId(self.resolved_font_ids.len() as u32 + 1);
        self.resolved_font_ids.insert(identity.clone(), id);
        id
    }

    /// Measure a single character's advance width using cosmic-text shaping.
    fn measure_char(
        &mut self,
        ch: char,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> f32 {
        let resolved = self.resolve_font_for_char(ch, family, weight, italic);
        let attrs = self.build_attrs(&resolved.family, resolved.weight, resolved.slant);
        let line_height = font_size * 1.3;
        let metrics = safe_metrics(font_size, line_height);

        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(
            &mut self.font_system,
            Some(font_size * 4.0),
            Some(font_size * 2.0),
        );

        let text = String::from(ch);
        buffer.set_text(
            &mut self.font_system,
            &text,
            &attrs,
            cosmic_text::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);

        buffer
            .layout_runs()
            .find_map(|run| run.glyphs.iter().next().map(|glyph| glyph.w))
            .unwrap_or(font_size * 0.6)
    }

    fn resolve_font_for_char(
        &mut self,
        ch: char,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> ResolvedCharFont {
        let requested_slant = if italic {
            FontSlant::Italic
        } else {
            FontSlant::Normal
        };
        if ch.is_ascii() {
            let resolved_family = self.resolve_family(&self.backend.resolve_family(family), None);
            // Snap to the family's available/instance weight, matching the
            // font actually opened (and what `build_attrs` renders with), so
            // `font-at` reports the opened instance's weight like GNU — e.g.
            // a semi-light request on variable Noto Sans reports light.
            let resolved_weight = crate::font_match::resolve_weight_in_family(
                &self.font_system,
                &resolved_family,
                weight,
                italic,
            );
            return ResolvedCharFont {
                family: resolved_family,
                weight: resolved_weight,
                slant: requested_slant,
            };
        }

        let prefer_monospace = self.backend.family_prefers_monospace(family);
        if let Some(matched) =
            self.backend
                .match_font_for_char(family, ch, prefer_monospace, weight, italic)
        {
            let resolved_family = self.resolve_family(&matched.family, matched.file.as_deref());
            return ResolvedCharFont {
                weight: crate::font_match::resolve_weight_in_family(
                    &self.font_system,
                    &resolved_family,
                    weight,
                    italic,
                ),
                family: resolved_family,
                slant: requested_slant,
            };
        }

        ResolvedCharFont {
            family: family.to_string(),
            weight,
            slant: requested_slant,
        }
    }

    /// Get the advance width for a single character.
    pub fn char_width(
        &mut self,
        ch: char,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> f32 {
        let key = MetricsCacheKey::new(family, weight, italic, font_size);

        // For ASCII, check the ASCII cache first
        let cp = ch as u32;
        if cp < 128 {
            if let Some(widths) = self.ascii_cache.get(&key) {
                return widths[cp as usize];
            }
            // Fill the whole ASCII cache on miss
            let widths = self.fill_ascii_widths_inner(family, weight, italic, font_size);
            let w = widths[cp as usize];
            self.ascii_cache.insert(key, widths);
            return w;
        }

        // Non-ASCII: resolve the actual covering font for this character.
        // GNU's font_range starts from the selected font and advances only
        // while font_encode_char accepts each concrete character; a broad
        // Unicode script cache is too coarse for Common/emoji symbols.
        let resolved = self.resolve_font_for_char(ch, family, weight, italic);
        let resolved_italic = resolved.slant.is_italic();
        let resolved_key = MetricsCacheKey::new(
            &resolved.family,
            resolved.weight,
            resolved_italic,
            font_size,
        );

        let char_key = (resolved_key, ch);
        if let Some(&w) = self.char_cache.get(&char_key) {
            return w;
        }

        let w = self.measure_char(
            ch,
            &resolved.family,
            resolved.weight,
            resolved_italic,
            font_size,
        );
        self.char_cache.insert(char_key, w);
        w
    }

    /// Fill ASCII width array (0-127) for given face attributes.
    /// Returns the cached array. Populates the cache on miss.
    pub fn fill_ascii_widths(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> [f32; 128] {
        let key = MetricsCacheKey::new(family, weight, italic, font_size);
        if let Some(widths) = self.ascii_cache.get(&key) {
            return *widths;
        }

        let widths = self.fill_ascii_widths_inner(family, weight, italic, font_size);
        self.ascii_cache.insert(key, widths);
        widths
    }

    /// Internal: measure all 128 ASCII characters in a single buffer.
    fn fill_ascii_widths_inner(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> [f32; 128] {
        let mut widths = [0.0f32; 128];
        let attrs = self.build_attrs(
            family,
            weight,
            if italic {
                FontSlant::Italic
            } else {
                FontSlant::Normal
            },
        );
        let line_height = font_size * 1.3;
        let metrics = safe_metrics(font_size, line_height);

        // Measure each printable ASCII character individually.
        // Characters 0-31 are control chars — use space width as fallback.
        let space_width = {
            let mut buffer = Buffer::new(&mut self.font_system, metrics);
            buffer.set_size(
                &mut self.font_system,
                Some(font_size * 4.0),
                Some(font_size * 2.0),
            );
            buffer.set_text(
                &mut self.font_system,
                " ",
                &attrs,
                cosmic_text::Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            buffer
                .layout_runs()
                .find_map(|run| run.glyphs.iter().next().map(|glyph| glyph.w))
                .unwrap_or(font_size * 0.6)
        };

        // Control chars (0-31) and DEL (127) get space width
        widths[..32].fill(space_width);
        widths[127] = space_width;

        // Measure printable ASCII (32-126) using a single buffer with all chars.
        // Shape them individually to get per-character advances.
        for cp in 32u32..127 {
            let ch = char::from_u32(cp).unwrap();
            let mut buffer = Buffer::new(&mut self.font_system, metrics);
            buffer.set_size(
                &mut self.font_system,
                Some(font_size * 4.0),
                Some(font_size * 2.0),
            );
            let text = String::from(ch);
            buffer.set_text(
                &mut self.font_system,
                &text,
                &attrs,
                cosmic_text::Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);

            widths[cp as usize] = buffer
                .layout_runs()
                .find_map(|run| run.glyphs.iter().next().map(|glyph| glyph.w))
                .unwrap_or(space_width);
        }

        widths
    }

    /// Get font metrics (ascent, descent, line height, char width) for a face.
    pub fn font_metrics(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> FontMetrics {
        let key = MetricsCacheKey::new(family, weight, italic, font_size);
        if let Some(m) = self.metrics_cache.get(&key) {
            return *m;
        }

        let primary_override =
            self.platform_primary_metrics_override(family, weight, italic, font_size);
        let (vertical, advances) = if let Some(probe) = primary_override {
            (
                FontVerticalMetrics {
                    ascent: probe.ascent.max(0) as f32,
                    descent: probe.descent.max(0) as f32,
                    line_height: probe.height.max(1) as f32,
                },
                FontAdvanceMetrics::from_font_probe(probe),
            )
        } else {
            let (selected_font_id, measured_space_width) =
                self.selected_font_id_and_space_width(family, weight, italic, font_size);
            let ascii_widths = self.fill_ascii_widths(family, weight, italic, font_size);
            let advances =
                FontAdvanceMetrics::from_ascii_widths(measured_space_width, &ascii_widths);
            let vertical = if let Some(font_id) = selected_font_id {
                self.font_metrics_from_selected_face(font_id, font_size)
                    .unwrap_or_else(|| {
                        self.glyph_box_fallback_vertical_metrics(family, weight, italic, font_size)
                    })
            } else {
                self.glyph_box_fallback_vertical_metrics(family, weight, italic, font_size)
            };
            (vertical, advances)
        };
        let frame_cell = FrameCellMetrics::derive(family, font_size, vertical, advances);
        if frame_cell.confidence == MetricConfidence::Degraded {
            tracing::debug!(
                "font_metrics: degraded frame cell width fallback for family={family:?} size={font_size}"
            );
        }
        let fm = FontMetrics {
            ascent: frame_cell.ascent,
            descent: frame_cell.descent,
            line_height: frame_cell.line_height,
            char_width: frame_cell.column_width,
            space_width: advances.space_width,
        };

        self.metrics_cache.insert(key, fm);
        fm
    }

    fn glyph_box_fallback_vertical_metrics(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        font_size: f32,
    ) -> FontVerticalMetrics {
        let attrs = self.build_attrs(
            family,
            weight,
            if italic {
                FontSlant::Italic
            } else {
                FontSlant::Normal
            },
        );
        let line_height = font_size * 1.3;
        let metrics = safe_metrics(font_size, line_height);

        // Fallback only: measure a representative glyph box when the selected
        // font's global tables are unavailable or obviously pathological.
        let sample = " Mg";
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(
            &mut self.font_system,
            Some(font_size * 8.0),
            Some(font_size * 2.0),
        );
        buffer.set_text(
            &mut self.font_system,
            sample,
            &attrs,
            cosmic_text::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut ascent = font_size.ceil().max(1.0);
        let mut descent = (line_height.ceil() - ascent).max(0.0);
        let mut actual_line_height = (ascent + descent).max(1.0);

        if let Some(layout) = buffer.line_layout(&mut self.font_system, 0)
            && let Some(line) = layout.first()
        {
            ascent = line.max_ascent.ceil().max(1.0);
            descent = line.max_descent.ceil().max(0.0);
            actual_line_height = (ascent + descent).max(1.0);
        }

        FontVerticalMetrics {
            ascent,
            descent,
            line_height: actual_line_height,
        }
    }

    /// Clear all caches. Call when fonts change (e.g., text-scale-adjust).
    /// `resolved_font_ids` intentionally survives: identities are durable and
    /// ids must stay stable across generations (see field doc).
    pub fn clear_caches(&mut self) {
        self.ascii_cache.clear();
        self.char_cache.clear();
        self.metrics_cache.clear();
        self.shaped_run_cache.clear();
        self.resolved_face_font_cache.clear();
        self.resolved_char_font_cache.clear();
        self.resolved_cluster_cache.clear();
    }

    /// Number of times `shape_run` actually invoked cosmic-text shaping (i.e.
    /// `shaped_run_cache` misses). Used by tests to assert the measure/render
    /// double-shape is deduped to one shape per (run, face).
    #[cfg(test)]
    pub(crate) fn shape_calls(&self) -> usize {
        self.n_shape_calls
    }

    /// Lower the `shaped_run_cache` entry cap so tests can exercise the
    /// clear-on-overflow path without shaping `SHAPED_RUN_CACHE_CAP` runs.
    #[cfg(test)]
    pub(crate) fn set_shaped_run_cache_cap(&mut self, cap: usize) {
        self.shaped_run_cache_cap = cap;
    }
}

/// Realize resolved font identities for every face of a finished frame.
///
/// Runs at the engine's frame-output boundary (after all install paths have
/// filled `state.faces`), so it covers every face regardless of which layout
/// path produced it. For each face it resolves the primary font through the
/// same `FontMetricsService` that produced the face's layout metrics, then
/// publishes `Face::default_resolved_font_id`, the frame's font table, and
/// the `font_file_path` bridge the renderer already primes.
///
/// No-op when `service` is `None` (TTY frames have no GUI font realization).
pub fn realize_frame_fonts(
    state: &mut neomacs_display_protocol::glyph_matrix::FrameDisplayState,
    service: &mut Option<FontMetricsService>,
) {
    let Some(svc) = service.as_mut() else {
        return;
    };
    // Deterministic interner allocation order across identical frames.
    let mut face_ids: Vec<FaceId> = state.faces.keys().copied().collect();
    face_ids.sort_unstable();
    for face_id in face_ids {
        let Some(face) = state.faces.get_mut(&face_id) else {
            continue;
        };
        let family = if face.font_family.is_empty() {
            "monospace"
        } else {
            face.font_family.as_str()
        };
        let italic = face.is_italic();
        match svc.resolved_font_for_face(family, face.font_weight, italic, face.font_size.max(1.0))
        {
            Some(font) => {
                face.default_resolved_font_id = Some(font.id);
                if face.font_file_path.is_none() {
                    face.font_file_path = font.identity.file_path.clone();
                }
                state.fonts.entry(font.id).or_insert(font);
            }
            None => {
                // Phase 0 divergence instrumentation: this face will reach
                // the render thread without a resolved identity and trigger
                // its independent semantic fallback.
                tracing::warn!(
                    target: "font_boundary",
                    face_id = face_id.get(),
                    family = %face.font_family,
                    weight = face.font_weight,
                    "GUI face has no resolvable primary font; renderer will re-select"
                );
            }
        }
    }

    realize_frame_char_fonts(state, svc);
}

/// Stamp per-character fallback fonts for the non-ASCII characters actually
/// on this frame's grid (`FrameDisplayState::char_fonts`).
///
/// For each (face, representative char) pair present in the window matrices
/// and chrome rows, resolves the covering font through the same per-char path
/// the measurement code uses and publishes the exact identity, so the render
/// thread's CJK/emoji/symbol fallback becomes a table lookup instead of its
/// own fontconfig match.
fn realize_frame_char_fonts(
    state: &mut neomacs_display_protocol::glyph_matrix::FrameDisplayState,
    svc: &mut FontMetricsService,
) {
    use neomacs_display_protocol::glyph_matrix::{GlyphRow, GlyphType};

    // Pass 1: collect the (face, repr char) pairs and composed clusters on
    // screen. Bounded by the number of distinct non-ASCII chars/clusters
    // visible, not by grid size.
    let mut wanted: Vec<(FaceId, char)> = Vec::new();
    let mut seen: std::collections::HashSet<(FaceId, char)> = std::collections::HashSet::new();
    let mut wanted_clusters: Vec<(FaceId, Box<str>)> = Vec::new();
    let mut seen_clusters: std::collections::HashSet<(FaceId, Box<str>)> =
        std::collections::HashSet::new();
    let mut collect_row = |row: &GlyphRow| {
        if !row.enabled {
            return;
        }
        for area in &row.glyphs {
            for glyph in area {
                if glyph.padding {
                    continue;
                }
                let repr = match &glyph.glyph_type {
                    GlyphType::Char { ch } => {
                        if ch.is_ascii() || crate::composition::is_composition_joiner(*ch) {
                            continue;
                        }
                        *ch
                    }
                    GlyphType::Composite { text } => {
                        if seen_clusters.insert((glyph.face_id, text.clone())) {
                            wanted_clusters.push((glyph.face_id, text.clone()));
                        }
                        match crate::composition::representative_char_for_cluster(text) {
                            Some(ch) => ch,
                            None => continue,
                        }
                    }
                    _ => continue,
                };
                if seen.insert((glyph.face_id, repr)) {
                    wanted.push((glyph.face_id, repr));
                }
            }
        }
    };
    for entry in &state.window_matrices {
        for row in &entry.matrix.rows {
            collect_row(row);
        }
    }
    for band in state.frame_chrome.bands() {
        if let neomacs_display_protocol::frame_chrome::FrameChromeContent::DisplayRow(content) =
            band.content()
        {
            collect_row(content.row());
        }
    }

    // Pass 2: resolve and publish. Steady state is one cache-hit per pair.
    for (face_id, repr) in wanted {
        if state
            .char_fonts
            .get(&face_id)
            .is_some_and(|by_char| by_char.contains_key(&repr))
        {
            continue;
        }
        let Some(face) = state.faces.get(&face_id) else {
            continue;
        };
        let family = if face.font_family.is_empty() {
            "monospace"
        } else {
            face.font_family.as_str()
        };
        match svc.resolved_font_for_char(
            repr,
            family,
            face.font_weight,
            face.is_italic(),
            face.font_size.max(1.0),
        ) {
            Some(font) => {
                state
                    .char_fonts
                    .entry(face_id)
                    .or_default()
                    .insert(repr, font.id);
                state.fonts.entry(font.id).or_insert(font);
            }
            None => {
                tracing::trace!(
                    target: "font_boundary",
                    face_id = face_id.get(),
                    ch = %repr,
                    "no per-char fallback font resolved; renderer will re-select"
                );
            }
        }
    }

    // Pass 3: shape composed clusters and publish their exact glyphs so the
    // renderer replays them instead of re-shaping the cluster text.
    for (face_id, text) in wanted_clusters {
        if state
            .shaped_clusters
            .get(&face_id)
            .is_some_and(|by_text| by_text.contains_key(&text))
        {
            continue;
        }
        let Some(face) = state.faces.get(&face_id) else {
            continue;
        };
        let family = if face.font_family.is_empty() {
            "monospace"
        } else {
            face.font_family.as_str()
        };
        match svc.resolved_glyphs_for_cluster(
            &text,
            family,
            face.font_weight,
            face.is_italic(),
            face.font_size.max(1.0),
        ) {
            Some((glyphs, fonts)) => {
                for font in fonts {
                    state.fonts.entry(font.id).or_insert(font);
                }
                state
                    .shaped_clusters
                    .entry(face_id)
                    .or_default()
                    .insert(text, glyphs);
            }
            None => {
                tracing::trace!(
                    target: "font_boundary",
                    face_id = face_id.get(),
                    cluster = %text,
                    "cluster did not shape; renderer will re-shape"
                );
            }
        }
    }
}

fn font_slant_kind_from_fontdb(style: Style) -> FontSlantKind {
    match style {
        Style::Normal => FontSlantKind::Normal,
        Style::Italic => FontSlantKind::Italic,
        Style::Oblique => FontSlantKind::Oblique,
    }
}

fn font_slant_from_fontdb(style: Style) -> FontSlant {
    match style {
        Style::Normal => FontSlant::Normal,
        Style::Italic => FontSlant::Italic,
        Style::Oblique => FontSlant::Oblique,
    }
}

fn font_slant_to_cosmic_style(slant: FontSlant) -> Option<Style> {
    match slant {
        FontSlant::Normal => None,
        FontSlant::Italic | FontSlant::ReverseItalic => Some(Style::Italic),
        FontSlant::Oblique | FontSlant::ReverseOblique => Some(Style::Oblique),
    }
}

fn font_width_from_stretch_number(stretch: u16) -> FontWidth {
    match stretch {
        1 => FontWidth::UltraCondensed,
        2 => FontWidth::ExtraCondensed,
        3 => FontWidth::Condensed,
        4 => FontWidth::SemiCondensed,
        5 => FontWidth::Normal,
        6 => FontWidth::SemiExpanded,
        7 => FontWidth::Expanded,
        8 => FontWidth::ExtraExpanded,
        9 => FontWidth::UltraExpanded,
        _ => {
            tracing::debug!(
                "font_metrics: unexpected OpenType width class {}, defaulting to normal",
                stretch
            );
            FontWidth::Normal
        }
    }
}

#[cfg(test)]
#[path = "font_metrics_test.rs"]
mod tests;
