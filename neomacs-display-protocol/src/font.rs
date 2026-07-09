//! Resolved font identity types.
//!
//! Semantic font selection (family alias resolution, fontset fallback,
//! weight/slant substitution, per-char coverage) happens on the
//! evaluator/layout side. The render thread receives an exact, already
//! resolved font identity and only rasterizes. See
//! `docs/plans/2026-07-05-font-realization-render-boundary-design.md`.

use crate::types::FaceId;
use std::collections::HashMap;

/// Snapshot-local id referencing an entry in a frame state's resolved
/// font table (`FrameDisplayState::fonts`).
///
/// Ids are allocated by the layout-side resolver and are stable for the
/// lifetime of that resolver, so consecutive frame snapshots reuse ids.
/// Renderer caches must still key on [`ResolvedFontIdentity`] (or a hash
/// of it), never on the raw id, so id renumbering after a font-database
/// change can never alias a cached glyph to the wrong font.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct ResolvedFontId(pub u32);

/// Which platform font backend produced an identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum FontBackendKind {
    /// Linux fontconfig / fontdb file identities.
    Fontconfig,
    /// macOS CoreText descriptors.
    CoreText,
    /// Windows DirectWrite face identities.
    DirectWrite,
}

/// One variation-axis coordinate of a variable font instance.
///
/// The value is stored as raw `f32` bits so the identity stays `Eq + Hash`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FontVariationCoord {
    /// OpenType axis tag (e.g. `wght` as big-endian bytes).
    pub tag: u32,
    /// Axis value as `f32::to_bits`.
    pub value_bits: u32,
}

impl FontVariationCoord {
    pub fn new(tag: u32, value: f32) -> Self {
        Self {
            tag,
            value_bits: value.to_bits(),
        }
    }

    pub fn value(self) -> f32 {
        f32::from_bits(self.value_bits)
    }
}

/// Exact, platform-openable font identity.
///
/// Not "file path only": macOS/Windows may need native descriptors, so
/// `stable_key` is the durable cross-snapshot cache key and `file_path`
/// is populated only where reliably available (Linux fontconfig).
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ResolvedFontIdentity {
    pub backend: FontBackendKind,
    /// Durable backend-specific key (Linux: `"{file_path}#{face_index}"`).
    pub stable_key: String,
    /// Absolute font file path when the backend exposes one.
    pub file_path: Option<String>,
    /// Face index within a collection file (0 for single-face files).
    pub face_index: u32,
    pub postscript_name: Option<String>,
    /// Variable font instance coordinates, if any.
    pub variation_coords: Vec<FontVariationCoord>,
}

impl ResolvedFontIdentity {
    /// Linux fontconfig/fontdb identity from a file path + face index.
    pub fn from_file(file_path: &str, face_index: u32, postscript_name: Option<String>) -> Self {
        Self {
            backend: FontBackendKind::Fontconfig,
            stable_key: format!("{file_path}#{face_index}"),
            file_path: Some(file_path.to_string()),
            face_index,
            postscript_name,
            variation_coords: Vec::new(),
        }
    }
}

/// How a resolved font was chosen. Distinguishing fallback tiers keeps
/// traces and oracle runs able to flag unexpected selection paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum FontResolutionSource {
    /// The realized face's primary font.
    FacePrimary,
    /// Chosen via fontset / per-character coverage fallback.
    FontsetFallback,
    /// Chosen via emoji presentation fallback.
    EmojiFallback,
    /// Chosen by the platform's last-resort matching.
    PlatformFallback,
    /// Renderer-side emergency fallback: text reached the render thread
    /// without a resolved identity. Must be zero for normal GUI text.
    EmergencyFallback,
}

/// Font slant as carried across the display protocol.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum FontSlantKind {
    #[default]
    Normal,
    Italic,
    Oblique,
}

/// The resolver's canonical answer for one concrete font instance.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResolvedFont {
    pub id: ResolvedFontId,
    pub identity: ResolvedFontIdentity,
    /// Family name as realized (selector semantics, not file metadata).
    pub family: String,
    pub full_name: Option<String>,
    pub postscript_name: Option<String>,
    /// CSS weight (400 = normal, 700 = bold).
    pub weight: u16,
    pub slant: FontSlantKind,
    /// OS/2 usWidthClass-style stretch number (5 = normal).
    pub width: u16,
    pub pixel_size: f32,
    pub ascent_px: f32,
    pub descent_px: f32,
    pub source: FontResolutionSource,
}

/// Resolved font table carried by frame state, keyed by [`ResolvedFontId`].
pub type ResolvedFontTable = HashMap<ResolvedFontId, ResolvedFont>;

/// One shaped glyph past semantic selection and shaping: the renderable
/// unit. Positions/advances are logical (scale 1.0) pixels; the renderer
/// applies its own scale factor.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResolvedGlyph {
    /// Font this glyph id belongs to, in the frame's font table.
    pub resolved_font_id: ResolvedFontId,
    /// Glyph index within that font.
    pub glyph_id: u16,
    /// Pen x offset within the cluster/run.
    pub x: f32,
    /// Pen y offset (baseline-relative).
    pub y: f32,
    /// Horizontal advance.
    pub x_advance: f32,
    /// Source-text byte range (cluster) this glyph covers.
    pub cluster_start: u32,
    pub cluster_end: u32,
}

/// Per-frame shaped composed-cluster table: `face_id → cluster text →
/// shaped glyphs`.
///
/// For grapheme clusters the layout side shapes (emoji ZWJ sequences,
/// combining marks, contextual scripts emitted as `GlyphType::Composite`),
/// this publishes the exact shaped output — glyph ids in exact fonts — so
/// the render thread rasterizes those glyphs instead of re-shaping the
/// cluster text and risking a different font or cluster segmentation.
pub type ShapedClusterTable = HashMap<FaceId, HashMap<Box<str>, Vec<ResolvedGlyph>>>;

/// Per-frame character fallback font table: `face_id → representative char →
/// resolved font`.
///
/// This is the layout side's projection of GNU's fontset lookup for the
/// characters actually on screen: for text a face's primary font does not
/// cover (CJK, emoji, symbols), layout resolves the covering font during
/// measurement and publishes the answer here so the render thread rasterizes
/// the same font instead of re-running its own per-character matching.
/// Entries reference [`ResolvedFontTable`] ids.
pub type CharFontTable = HashMap<FaceId, HashMap<char, ResolvedFontId>>;

#[cfg(test)]
#[path = "font_test.rs"]
mod tests;
