use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphRow, GlyphType};

/// Attach a cluster-extender char (combining mark / ZWJ / variation
/// selector) to the last non-padding glyph in `area`, upgrading a
/// `Char` glyph into `Composite` or appending to an existing
/// `Composite`. Returns true when the extender was merged; false when
/// there is no preceding base glyph (caller should fall back to
/// emitting a standalone glyph).
fn merge_extender_into_last_glyph(area: &mut Vec<Glyph>, ch: char) -> bool {
    // Walk back past padding cells (the right half of a preceding wide
    // char); the combining mark attaches to the wide base, not the
    // padding slot.
    for glyph in area.iter_mut().rev() {
        if glyph.padding {
            continue;
        }
        match &mut glyph.glyph_type {
            GlyphType::Char { ch: base } => {
                let mut s = String::with_capacity(base.len_utf8() + ch.len_utf8());
                s.push(*base);
                s.push(ch);
                glyph.glyph_type = GlyphType::Composite {
                    text: s.into_boxed_str(),
                };
                return true;
            }
            GlyphType::Composite { text } => {
                let mut s = String::with_capacity(text.len() + ch.len_utf8());
                s.push_str(text);
                s.push(ch);
                glyph.glyph_type = GlyphType::Composite {
                    text: s.into_boxed_str(),
                };
                return true;
            }
            GlyphType::Glyphless { .. } | GlyphType::Stretch { .. } | GlyphType::Image { .. } => {
                return false;
            }
        }
    }
    false
}

/// Append `ch` to a glyph's character/cluster text (Char -> Composite,
/// Composite grows). Used to extend the per-cell grapheme of a complex run's
/// member.
fn extend_glyph_grapheme(glyph: &mut Glyph, ch: char) {
    match &mut glyph.glyph_type {
        GlyphType::Char { ch: base } => {
            let mut s = String::with_capacity(base.len_utf8() + ch.len_utf8());
            s.push(*base);
            s.push(ch);
            glyph.glyph_type = GlyphType::Composite {
                text: s.into_boxed_str(),
            };
        }
        GlyphType::Composite { text } => {
            let mut s = String::with_capacity(text.len() + ch.len_utf8());
            s.push_str(text);
            s.push(ch);
            glyph.glyph_type = GlyphType::Composite {
                text: s.into_boxed_str(),
            };
        }
        _ => {}
    }
}

/// Whether `glyph` is a complex-run member's padding cell carrying its own
/// per-cell grapheme (a non-blank Char or a Composite), as opposed to a
/// blank wide-char padding slot. Such cells let the TTY decompose the run.
fn is_run_member_padding(glyph: &Glyph) -> bool {
    glyph.padding
        && match &glyph.glyph_type {
            GlyphType::Char { ch } => *ch != ' ',
            GlyphType::Composite { .. } => true,
            _ => false,
        }
}

pub(crate) fn push_char_to_row(
    row: &mut GlyphRow,
    ch: char,
    face_id: u32,
    charpos: usize,
    pixel_width: f32,
) {
    let area = &mut row.glyphs[GlyphArea::Text.index()];
    if crate::unicode::is_cluster_extender(ch) && merge_extender_into_last_glyph(area, ch) {
        return;
    }
    area.push(Glyph::char(ch, face_id, charpos).with_pixel_width(pixel_width));
    row.displays_text = true;
}

pub(crate) fn push_wide_char_to_row(
    row: &mut GlyphRow,
    ch: char,
    face_id: u32,
    charpos: usize,
    pixel_width: f32,
) {
    let area = &mut row.glyphs[GlyphArea::Text.index()];
    if crate::unicode::is_cluster_extender(ch) && merge_extender_into_last_glyph(area, ch) {
        return;
    }
    let mut glyph = Glyph::char(ch, face_id, charpos);
    glyph.wide = true;
    glyph.pixel_width = if pixel_width.is_finite() && pixel_width > 0.0 {
        pixel_width
    } else {
        0.0
    };
    area.push(glyph);
    area.push(Glyph::padding_for(face_id, charpos));
    row.displays_text = true;
}

/// Append a grapheme-cluster continuation character — a ZWJ-joined emoji, the
/// second regional indicator of a flag, a combining mark, a variation selector,
/// etc. — to the last emitted text glyph, upgrading it to a `Composite` so the
/// renderer shapes the whole cluster as one unit. Falls back to a standalone
/// glyph when there is no mergeable base.
pub(crate) fn push_cluster_continuation_to_row(
    row: &mut GlyphRow,
    ch: char,
    face_id: u32,
    charpos: usize,
) {
    let area = &mut row.glyphs[GlyphArea::Text.index()];
    if let Some(last) = area.last_mut()
        && is_run_member_padding(last)
    {
        extend_glyph_grapheme(last, ch);
    }
    if merge_extender_into_last_glyph(area, ch) {
        return;
    }
    area.push(Glyph::char(ch, face_id, charpos));
    row.displays_text = true;
}

/// Grow a contextual-shaping run by appending `ch` to the last text glyph's
/// composed cluster and pushing a padding cell carrying `ch`'s own buffer
/// position.
pub(crate) fn push_run_member_to_row(
    row: &mut GlyphRow,
    ch: char,
    face_id: u32,
    charpos: usize,
    pixel_width: f32,
) {
    let area = &mut row.glyphs[GlyphArea::Text.index()];
    if merge_extender_into_last_glyph(area, ch) {
        let member_width = if pixel_width.is_finite() && pixel_width > 0.0 {
            pixel_width
        } else {
            0.0
        };
        if let Some(base) = area.iter_mut().rev().find(|g| !g.padding) {
            base.pixel_width += member_width;
        }
        let mut pad = Glyph::padding_for(face_id, charpos);
        pad.glyph_type = GlyphType::Char { ch };
        pad.pixel_width = member_width;
        area.push(pad);
        return;
    }
    area.push(Glyph::char(ch, face_id, charpos).with_pixel_width(pixel_width));
    row.displays_text = true;
}

pub(crate) fn push_stretch_to_row(
    row: &mut GlyphRow,
    width_cols: u16,
    face_id: u32,
    pixel_width: f32,
    pixel_height: f32,
    pixel_ascent: f32,
) {
    let glyph = Glyph::stretch(width_cols, face_id).with_pixel_geometry(
        pixel_width,
        pixel_height,
        pixel_ascent,
    );
    row.glyphs[GlyphArea::Text.index()].push(glyph);
    row.displays_text = true;
}

/// Normalize a standalone row built outside the window-matrix walker.
pub(crate) fn normalize_external_row(row: &mut GlyphRow) {
    row.displays_text = !row.glyphs[GlyphArea::Text.index()].is_empty();
    let _ = crate::matrix_builder::GlyphMatrixBuilder::reorder_row_bidi(row, None);
}
