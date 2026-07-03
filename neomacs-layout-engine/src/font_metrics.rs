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

/// Safe wrapper around cosmic_text::Metrics that ensures font_size and
/// line_height are never zero.  cosmic-text panics with "line height
/// cannot be 0" if either value is 0.0.  GNU Emacs TTY frames use
/// 1x1 cell metrics; we enforce a minimum of 1.0 for safety.
fn safe_metrics(font_size: f32, line_height: f32) -> cosmic_text::Metrics {
    cosmic_text::Metrics::new(font_size.max(1.0), line_height.max(1.0))
}
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
}

#[derive(Debug, Clone, Copy)]
struct FontVerticalMetrics {
    ascent: f32,
    descent: f32,
    line_height: f32,
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
        }
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
    fn build_attrs(&mut self, family: &str, weight: u16, slant: FontSlant) -> Attrs<'static> {
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
        match font_slant_to_cosmic_style(slant) {
            Some(style) => attrs = attrs.style(style),
            None => {}
        }

        attrs
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
        let metrics = safe_metrics(font_size, font_size * 1.3);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        // No width bound: lay the whole run out on a single line so shaping
        // spans the entire run instead of wrapping mid-word.
        buffer.set_size(&mut self.font_system, None, None);
        buffer.set_text(
            &mut self.font_system,
            text,
            &attrs,
            cosmic_text::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut glyphs = Vec::new();
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let phys = glyph.physical((0.0, 0.0), 1.0);
                glyphs.push(ShapedGlyph {
                    font_id: phys.cache_key.font_id,
                    glyph_id: phys.cache_key.glyph_id,
                    x: phys.x as f32,
                    y: phys.y as f32,
                    x_advance: glyph.w,
                    cluster_start: glyph.start,
                    cluster_end: glyph.end,
                });
            }
        }
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
        Some(SelectedFontInfo {
            // TTC/variable collections frequently expose several regional
            // aliases, and fontdb may report the file's first alias instead of
            // the family we explicitly resolved for this character. Preserve
            // the selector's family so `font-at` mirrors GNU Emacs' realized
            // face semantics.
            family: resolved.family.clone(),
            file: fontdb_face_file(face),
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
            let resolved_family =
                self.resolve_family(crate::fontconfig::resolve_family(family), None);
            return ResolvedCharFont {
                family: resolved_family,
                weight,
                slant: requested_slant,
            };
        }

        let prefer_monospace = crate::fontconfig::family_prefers_monospace(family);
        if let Some(matched) =
            crate::fontconfig::match_font_for_char(family, ch, prefer_monospace, weight, italic)
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
        for i in 0..32 {
            widths[i] = space_width;
        }
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

        let (selected_font_id, measured_space_width) =
            self.selected_font_id_and_space_width(family, weight, italic, font_size);
        let ascii_widths = self.fill_ascii_widths(family, weight, italic, font_size);
        let advances = FontAdvanceMetrics::from_ascii_widths(measured_space_width, &ascii_widths);

        let vertical = if let Some(font_id) = selected_font_id {
            self.font_metrics_from_selected_face(font_id, font_size)
                .unwrap_or_else(|| {
                    self.glyph_box_fallback_vertical_metrics(family, weight, italic, font_size)
                })
        } else {
            self.glyph_box_fallback_vertical_metrics(family, weight, italic, font_size)
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

        if let Some(layout) = buffer.line_layout(&mut self.font_system, 0) {
            if let Some(line) = layout.first() {
                ascent = line.max_ascent.ceil().max(1.0);
                descent = line.max_descent.ceil().max(0.0);
                actual_line_height = (ascent + descent).max(1.0);
            }
        }

        FontVerticalMetrics {
            ascent,
            descent,
            line_height: actual_line_height,
        }
    }

    /// Clear all caches. Call when fonts change (e.g., text-scale-adjust).
    pub fn clear_caches(&mut self) {
        self.ascii_cache.clear();
        self.char_cache.clear();
        self.metrics_cache.clear();
        self.shaped_run_cache.clear();
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
