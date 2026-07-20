//! GNU's overlay arrow, drawn into the text area (xdisp.c
//! `overlay_arrow_at_row` + the `display_line` tail that copies its glyphs).
//!
//! GNU picks one of two renderings. On a window-system frame with a left
//! fringe it returns a fringe bitmap; **otherwise** — a terminal frame, or a
//! GUI window with no left fringe — it returns `overlay-arrow-string` and
//! copies that string's glyphs OVER the leading glyphs of the marked row,
//! replacing them rather than shifting them along. A line "beta" marked with
//! the default "=>" therefore displays as "=>ta".
//!
//! This module implements the string branch only, which is exactly the branch
//! whose frames have uniform cell widths, so overwriting the leading glyphs in
//! place reproduces GNU's glyph copy without needing to re-lay the row. The
//! fringe-bitmap branch belongs with the other fringe indicators.

use crate::neovm_bridge::FaceResolver;
use crate::window_output::TextWindowOutputTarget;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{
    Glyph, GlyphArea, GlyphType, NO_BUFFER_POSITION_CHARPOS,
};
use neovm_core::buffer::BufferId;
use neovm_core::emacs_core::intern::intern;
use neovm_core::emacs_core::{Context, Value};

use crate::display_face_id::FrameFaceIdAllocator;

/// Draw every applicable overlay arrow into this window's already-installed
/// body rows. Call after the body walk has installed its rows (their
/// `start_charpos`/`end_charpos` must be final) and before chrome.
pub(crate) fn draw_text_area_overlay_arrows(
    mut output: TextWindowOutputTarget<'_>,
    evaluator: &Context,
    buffer_id: BufferId,
    face_resolver: &FaceResolver,
    face_ids: &mut FrameFaceIdAllocator,
    char_width: f32,
) {
    let vars = arrow_variables(evaluator);
    if vars.is_empty() {
        return;
    }

    // GNU renders the arrow string through the display iterator with the
    // window's default face, so the arrow does not inherit the face of the
    // text it covers (font-lock keywords, region, ...).
    let resolved = face_resolver.resolve_named_face("default");
    let arrow_face_id = face_ids.allocate();
    output.install_resolved_face(arrow_face_id, &resolved, None);

    // GNU's `overlay_arrow_seen` is per redisplay pass: an arrow is drawn on a
    // row that displays text, or on a non-text row only while no arrow has
    // been drawn yet.
    let mut arrow_seen = false;

    for var in vars {
        let Some(charpos) = arrow_marker_charpos(evaluator, var, buffer_id) else {
            continue;
        };
        let Some(text) = arrow_string(evaluator, var) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }

        let Some(row_index) = find_marked_row(&mut output, charpos, arrow_seen) else {
            continue;
        };
        if overwrite_leading_glyphs(&mut output, row_index, &text, arrow_face_id, char_width) {
            arrow_seen = true;
        }
    }
}

/// The symbols in `overlay-arrow-variable-list`, skipping non-symbols exactly
/// as GNU's loop does.
fn arrow_variables(evaluator: &Context) -> Vec<Value> {
    let Some(list) = evaluator
        .obarray()
        .symbol_value("overlay-arrow-variable-list")
        .copied()
    else {
        return Vec::new();
    };
    let mut vars = Vec::new();
    let mut tail = list;
    while tail.is_cons() {
        let var = tail.cons_car();
        if var.as_symbol_id().is_some() {
            vars.push(var);
        }
        tail = tail.cons_cdr();
    }
    vars
}

/// 0-based buffer position of `var`'s marker, when it is a marker pointing
/// into `buffer_id` (GNU additionally requires `current_buffer`).
fn arrow_marker_charpos(evaluator: &Context, var: Value, buffer_id: BufferId) -> Option<usize> {
    let sym = var.as_symbol_id()?;
    let value = evaluator.obarray().symbol_value_id(sym).copied()?;
    let marker = value.as_marker_data()?;
    (marker.buffer == Some(buffer_id)).then_some(marker.charpos)
}

/// GNU `overlay_arrow_string_or_property`: the symbol's own
/// `overlay-arrow-string` property when it is a string, else the global
/// `overlay-arrow-string`.
fn arrow_string(evaluator: &Context, var: Value) -> Option<String> {
    let sym = var.as_symbol_id()?;
    let prop_key = intern("overlay-arrow-string");
    let mut plist = evaluator.obarray().symbol_plist_id(sym);
    while plist.is_cons() {
        let key = plist.cons_car();
        let rest = plist.cons_cdr();
        if !rest.is_cons() {
            break;
        }
        if key.as_symbol_id() == Some(prop_key)
            && let Some(text) = rest.cons_car().as_str_owned()
        {
            return Some(text);
        }
        plist = rest.cons_cdr();
    }
    evaluator
        .obarray()
        .symbol_value("overlay-arrow-string")
        .and_then(|value| value.as_str_owned())
}

/// GNU's row test — the marker lies within the row, and the row either
/// displays text or no arrow has been drawn yet this pass.
///
/// GNU writes this as `MATRIX_ROW_START_CHARPOS (row) <= pos <
/// MATRIX_ROW_END_CHARPOS (row)`, but its end is one PAST the newline the row
/// consumed, whereas ours is the newline's own position (which is why the
/// engine's other consumers either add 1 to it or, like
/// `find_text_row_for_charpos`, compare inclusively). Translating GNU's
/// half-open test into this convention makes the upper bound inclusive.
///
/// The difference is only observable on an empty line, whose entire content is
/// that newline: there `start == end`, so an exclusive test matches nothing
/// and the arrow silently fails to draw — measured against GNU, which draws it.
fn find_marked_row(
    output: &mut TextWindowOutputTarget<'_>,
    charpos: usize,
    arrow_seen: bool,
) -> Option<usize> {
    let builder = output.builder();
    let mut index = 0usize;
    while let Some(row) = builder.current_window_row(index) {
        if row.enabled
            && row.role == GlyphRowRole::Text
            && (row.displays_text || !arrow_seen)
            && row.start_charpos <= charpos
            && charpos <= row.end_charpos
        {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// Copy the arrow's characters over the row's leading text-area glyphs,
/// GNU's `*p++ = *glyph++`. Returns whether anything was drawn.
///
/// Where the row already has a glyph, only its character and face are replaced
/// and its geometry is kept. That is equivalent to GNU's glyph copy on the
/// frames that take this branch (uniform cell widths) and, unlike a copy,
/// cannot leave the row with a stale advance. A glyph the arrow only partly
/// covers (a double-width character) is consumed whole, which is GNU's
/// padding-glyph cleanup.
///
/// Where the row runs out — an empty line has no glyphs at all, and a line
/// shorter than the arrow runs out partway — the remaining characters are
/// appended. They carry no buffer position: the arrow is decoration painted
/// over the line, so a click or cursor placement must not resolve into it.
fn overwrite_leading_glyphs(
    output: &mut TextWindowOutputTarget<'_>,
    row_index: usize,
    text: &str,
    face_id: neomacs_display_protocol::types::FaceId,
    char_width: f32,
) -> bool {
    let Some(mut row) = output.builder().current_window_row(row_index).cloned() else {
        return false;
    };
    let glyphs = &mut row.glyphs[GlyphArea::Text.index()];
    let mut drawn = false;
    for (offset, ch) in text.chars().enumerate() {
        match glyphs.get_mut(offset) {
            Some(slot) => {
                slot.glyph_type = GlyphType::Char { ch };
                slot.face_id = face_id;
                slot.wide = false;
            }
            None => {
                let mut glyph = Glyph::char(ch, face_id, NO_BUFFER_POSITION_CHARPOS);
                glyph.pixel_width = char_width;
                glyphs.push(glyph);
            }
        }
        drawn = true;
    }
    if drawn {
        output
            .builder()
            .install_finalized_output_row(row_index, row);
    }
    drawn
}
