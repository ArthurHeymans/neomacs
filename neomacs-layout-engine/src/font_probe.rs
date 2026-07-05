//! Pixel-size font metric probe, porting GNU's cairo freetype driver.
//!
//! `font-info` on a font ENTITY opens the font at the entity's size — for a
//! scalable entity, `font_open_entity` (GNU src/font.c) bumps pixel size 0
//! upward until `average_width > 0 && height > 0`, which lands at 1px for
//! ordinary scalable fonts. The metrics GNU reports come from
//! `ftcrfont_open` (src/ftcrfont.c): a cairo scaled font's per-glyph
//! `x_advance` rounded via `lround` (cairo hint-metrics rounds advances to
//! integer pixels over FreeType's hinted loads) and `lround`ed cairo font
//! extents for ascent/descent.
//!
//! This module reproduces that with FreeType directly:
//! - per-glyph width = hinted `FT_Load_Char(FT_LOAD_DEFAULT)` advance,
//!   rounded from 26.6 to integer pixels (cairo hint-metrics equivalent);
//! - ascent/descent = rounded FT size metrics (what cairo's font extents
//!   report for an FT backend with hint-metrics on).
//!
//! Byte-exactness is enforced by tests against captured GNU output for
//! concrete font files; if a future font/hinting configuration diverges,
//! the tests say so instead of the probe silently guessing.

use freetype::Library;
use freetype::face::LoadFlag;

/// Metrics of one font file probed at an exact pixel size, shaped like the
/// `font-info` elements GNU fills in `ftcrfont_open` + `font_open_entity`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontPxMetrics {
    /// The pixel size the font actually opened at (`font_open_entity` may
    /// bump the requested size upward until the font is "manageable").
    pub pixel_size: u32,
    pub height: i32,
    pub ascent: i32,
    pub descent: i32,
    pub max_width: i32,
    pub space_width: i32,
    pub average_width: i32,
}

/// Probe `file`[`face_index`] like GNU `font_open_entity`: try
/// `pixel_size`, bumping upward (at most 15 times) until average width and
/// height are positive.
pub fn probe_font_px_metrics(
    file: &str,
    face_index: u32,
    pixel_size: u32,
) -> Option<FontPxMetrics> {
    let library = Library::init().ok()?;
    let face = library.new_face(file, face_index as isize).ok()?;
    let start = pixel_size.max(1);
    for psize in start..=start + 15 {
        if let Some(metrics) = probe_at_exact_px(&face, psize)
            && metrics.average_width > 0
            && metrics.height > 0
        {
            return Some(metrics);
        }
    }
    None
}

fn probe_at_exact_px(face: &freetype::Face, pixel_size: u32) -> Option<FontPxMetrics> {
    face.set_pixel_sizes(pixel_size, pixel_size).ok()?;

    // ASCII printables loop (ftcrfont.c ftcrfont_open): per-glyph width is
    // the hinted advance rounded to integer pixels (cairo lround of
    // x_advance with hint-metrics on). Glyphs a char is missing fall back
    // to glyph id 0, mirroring the cairo text_to_glyphs failure path.
    let mut max_width = 0i32;
    let mut space_width = 0i32;
    let mut average_width = 0i64;
    let mut n = 0i64;
    // Cairo under fontconfig's default hintstyle=hintslight loads with
    // FT_LOAD_TARGET_LIGHT: vertical-only hinting, so horizontal advances
    // stay fractional and hint-metrics rounding (lround) decides the pixel
    // width. Full bytecode hinting (LOAD_DEFAULT) would widen some glyphs
    // (Noto '@'-class) to 2px where GNU reports 1.
    let load_flags = LoadFlag::TARGET_LIGHT;
    for c in 32u8..127 {
        if face.load_char(c as usize, load_flags).is_err()
            && face.load_glyph(0, load_flags).is_err()
        {
            continue;
        }
        // 26.6 fixed-point hinted advance → integer pixels, round half up
        // (lround semantics for the non-negative advances fonts produce).
        let advance = face.glyph().advance().x;
        let this_width = ((advance + 32) >> 6) as i32;
        if this_width > 0 {
            if this_width > max_width {
                max_width = this_width;
            }
            if c == 32 {
                space_width = this_width;
            }
            average_width += this_width as i64;
            n += 1;
        }
    }
    if n > 0 {
        average_width /= n;
    }

    // Font extents (cairo_scaled_font_extents → lround): for an FT backend
    // with hint-metrics on these are the grid-fitted size metrics.
    let size_metrics = face.size_metrics()?;
    let ascent = ((size_metrics.ascender + 32) >> 6) as i32;
    let descent = ((-size_metrics.descender + 32) >> 6) as i32;

    Some(FontPxMetrics {
        pixel_size,
        height: ascent + descent,
        ascent,
        descent,
        max_width,
        space_width,
        average_width: average_width as i32,
    })
}

#[cfg(test)]
#[path = "font_probe_test.rs"]
mod tests;
