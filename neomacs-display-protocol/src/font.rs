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
/// Ids are allocated from the complete realized instance (durable identity,
/// replay method/strike, and size) and are stable for the lifetime of that
/// resolver, so consecutive frame snapshots reuse ids.
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
/// is populated whenever a backend exposes a durable local file.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ResolvedFontIdentity {
    pub backend: FontBackendKind,
    /// Durable backend-specific key (Linux:
    /// `"{file_path}#{face_index}@{axis}={value_bits},..."`).
    pub stable_key: String,
    /// Absolute font file path when the backend exposes one.
    pub file_path: Option<String>,
    /// Backend-native face selector.
    ///
    /// For Fontconfig/FreeType, bits 0-15 are the face index within the font
    /// file and bits 16-30 select a named variable-font instance. Consumers
    /// which use `fontdb`/`ttf-parser` must call [`Self::file_face_index`]
    /// instead of passing this value through directly.
    face_selector: BackendFontSelector,
    pub postscript_name: Option<String>,
    /// Variable font instance coordinates, if any.
    pub variation_coords: Vec<FontVariationCoord>,
}

/// Opaque selector understood by the platform font backend.
///
/// This is intentionally not interchangeable with a collection face index:
/// Fontconfig/FreeType also encode a named variable-font instance in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BackendFontSelector(u32);

impl BackendFontSelector {
    const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    const fn raw(self) -> u32 {
        self.0
    }
}

impl ResolvedFontIdentity {
    /// Linux fontconfig/fontdb identity from a file path + face index.
    pub fn from_file(file_path: &str, face_index: u32, postscript_name: Option<String>) -> Self {
        Self::from_file_with_variations(file_path, face_index, postscript_name, Vec::new())
    }

    /// Linux fontconfig/fontdb identity for an exact variable-font instance.
    ///
    /// Variation coordinates are sorted by OpenType tag so backend ordering
    /// cannot create distinct identities for the same instance. Their raw
    /// floating-point bits are part of the stable key; renderer caches must
    /// never alias two instances from the same file and collection index.
    pub fn from_file_with_variations(
        file_path: &str,
        face_index: u32,
        postscript_name: Option<String>,
        mut variation_coords: Vec<FontVariationCoord>,
    ) -> Self {
        variation_coords.sort_unstable_by_key(|coord| (coord.tag, coord.value_bits));

        let mut stable_key = format!("{file_path}#{face_index}");
        append_variation_key(&mut stable_key, &variation_coords);

        Self {
            backend: FontBackendKind::Fontconfig,
            stable_key,
            file_path: Some(file_path.to_string()),
            face_selector: BackendFontSelector::from_raw(face_index),
            postscript_name,
            variation_coords,
        }
    }

    /// Exact file-backed identity selected by a native platform backend.
    ///
    /// CoreText and DirectWrite selection remains distinguishable from a
    /// Fontconfig selection of the same file. The native adapters preserve
    /// their backend kind while exposing the collection face index required
    /// by the shared fontdb/Swash materialization path.
    pub fn from_platform_file_with_variations(
        backend: FontBackendKind,
        file_path: &str,
        face_selector: u32,
        postscript_name: Option<String>,
        mut variation_coords: Vec<FontVariationCoord>,
    ) -> Self {
        if backend == FontBackendKind::Fontconfig {
            return Self::from_file_with_variations(
                file_path,
                face_selector,
                postscript_name,
                variation_coords,
            );
        }
        variation_coords.sort_unstable_by_key(|coord| (coord.tag, coord.value_bits));
        let prefix = match backend {
            FontBackendKind::Fontconfig => unreachable!("handled above"),
            FontBackendKind::CoreText => "coretext",
            FontBackendKind::DirectWrite => "directwrite",
        };
        let mut stable_key = format!("{prefix}:{file_path}#{face_selector}");
        append_variation_key(&mut stable_key, &variation_coords);

        Self {
            backend,
            stable_key,
            file_path: Some(file_path.to_string()),
            face_selector: BackendFontSelector::from_raw(face_selector),
            postscript_name,
            variation_coords,
        }
    }

    /// Identity for a font already resident in the layout font database.
    pub fn from_memory(
        backend: FontBackendKind,
        stable_key: String,
        backend_selector: u32,
        postscript_name: Option<String>,
    ) -> Self {
        Self {
            backend,
            stable_key,
            file_path: None,
            face_selector: BackendFontSelector::from_raw(backend_selector),
            postscript_name,
            variation_coords: Vec::new(),
        }
    }

    /// The opaque selector value for diagnostics and platform-native APIs.
    pub fn backend_selector(&self) -> u32 {
        self.face_selector.raw()
    }

    /// Selector accepted by FreeType, including named-instance bits.
    pub fn freetype_selector(&self) -> Option<u32> {
        (self.backend == FontBackendKind::Fontconfig).then(|| self.face_selector.raw())
    }

    /// Face index understood by font-file parsers such as fontdb and
    /// ttf-parser.
    ///
    /// Those parsers enumerate collection faces but do not enumerate
    /// FreeType's named variable-font instances. Keeping this conversion at
    /// the identity boundary prevents layout and rendering from confusing a
    /// Fontconfig selector such as `0x0007_0000` with collection face 458752.
    pub fn file_face_index(&self) -> u32 {
        match self.backend {
            FontBackendKind::Fontconfig => self.face_selector.raw() & 0x0000_ffff,
            FontBackendKind::CoreText | FontBackendKind::DirectWrite => self.face_selector.raw(),
        }
    }

    /// FreeType named-instance index carried by a Fontconfig selector.
    pub fn named_instance_index(&self) -> Option<u32> {
        match self.backend {
            FontBackendKind::Fontconfig => {
                let index = (self.face_selector.raw() >> 16) & 0x7fff;
                (index != 0).then_some(index)
            }
            FontBackendKind::CoreText | FontBackendKind::DirectWrite => None,
        }
    }
}

fn append_variation_key(stable_key: &mut String, variation_coords: &[FontVariationCoord]) {
    if variation_coords.is_empty() {
        return;
    }
    stable_key.push('@');
    for (index, coord) in variation_coords.iter().enumerate() {
        if index != 0 {
            stable_key.push(',');
        }
        let tag = coord.tag.to_be_bytes();
        stable_key.extend(tag.into_iter().map(char::from));
        stable_key.push('=');
        stable_key.push_str(&format!("{:08x}", coord.value_bits));
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

/// Stable identity of the fixed bitmap strike selected during realization.
/// The ppem values use FreeType's 26.6 representation and let the renderer
/// reject a stale or mismatched face instead of silently selecting another
/// strike at replay time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BitmapStrikeKey {
    pub index: u32,
    pub x_ppem_26_6: i64,
    pub y_ppem_26_6: i64,
}

/// Sampling policy attached to a realized glyph source.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum GlyphSampling {
    #[default]
    Linear,
    Nearest,
}

/// GNU `ftfont_open`'s horizontal-metric policy for a fixed font.
///
/// This is part of replay identity: the render thread must reopen the exact
/// instance with the same spacing semantics selected by layout.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum FixedFontSpacing {
    /// GNU proportional and dual-width entities measure printable ASCII and
    /// retain the actual space-glyph advance.
    #[default]
    ProportionalOrDual,
    /// GNU mono and charcell entities use the face maximum advance for both
    /// average and space width.
    MonospaceOrCharacterCell,
}

/// Durable instructions for reopening one exact resolved font on the render
/// thread. Process-local font handles never cross the display protocol.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum FontReplay {
    #[default]
    Swash,
    FreeTypeBitmap {
        strike: BitmapStrikeKey,
        sampling: GlyphSampling,
        #[serde(default)]
        spacing: FixedFontSpacing,
    },
}

impl FontReplay {
    pub const fn sampling(self) -> GlyphSampling {
        match self {
            Self::Swash => GlyphSampling::Linear,
            Self::FreeTypeBitmap { sampling, .. } => sampling,
        }
    }
}

/// The resolver's canonical answer for one concrete font instance.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResolvedFont {
    pub id: ResolvedFontId,
    pub identity: ResolvedFontIdentity,
    #[serde(default)]
    pub replay: FontReplay,
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
    /// GNU `font->space_width`: also the advance used when an ASCII glyph is
    /// unavailable in this primary font.
    #[serde(default)]
    pub space_advance_px: f32,
    pub source: FontResolutionSource,
}

/// Resolved font table carried by frame state, keyed by [`ResolvedFontId`].
pub type ResolvedFontTable = HashMap<ResolvedFontId, ResolvedFont>;

/// Backend-neutral glyph index in one exact [`ResolvedFont`].
///
/// FreeType exposes the full unsigned 32-bit glyph-index domain. Keeping that
/// domain in the display protocol prevents fixed bitmap fonts from being
/// truncated merely because Swash currently uses 16-bit indices.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct ResolvedGlyphId(u32);

impl ResolvedGlyphId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub const fn as_u16(self) -> Option<u16> {
        if self.0 <= u16::MAX as u32 {
            Some(self.0 as u16)
        } else {
            None
        }
    }
}

impl From<u16> for ResolvedGlyphId {
    fn from(value: u16) -> Self {
        Self(u32::from(value))
    }
}

/// One shaped glyph past semantic selection and shaping: the renderable
/// unit. Positions/advances are logical (scale 1.0) pixels; the renderer
/// applies its own scale factor.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResolvedGlyph {
    /// Font this glyph id belongs to, in the frame's font table.
    pub resolved_font_id: ResolvedFontId,
    /// Glyph index within that font.
    pub glyph_id: ResolvedGlyphId,
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

/// Exact layout answer for one visible scalar under one face.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResolvedCharGlyph {
    pub resolved_font_id: ResolvedFontId,
    pub glyph_id: ResolvedGlyphId,
    /// Logical horizontal advance measured from the same opened font.
    pub advance_px: f32,
}

/// Per-frame character glyph table: `face_id → scalar → exact font/glyph`.
///
/// This is the layout side's projection of GNU's realized face/fontset lookup
/// for characters actually on screen. Both primary and fallback characters
/// carry the exact glyph index, so rendering performs neither font selection
/// nor a second charmap lookup.
pub type CharFontTable = HashMap<FaceId, HashMap<char, ResolvedCharGlyph>>;

#[cfg(test)]
#[path = "font_test.rs"]
mod tests;
