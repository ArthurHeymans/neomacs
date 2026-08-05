//! Unit tests for the trailing-whitespace highlight mutation (GNU
//! `highlight_trailing_whitespace`).
//!
//! These exercise `HighlightTrailingWhitespaceMutation` directly against a
//! freshly built `GlyphRow`: the run of space `Char` glyphs and tab `Stretch`
//! glyphs at the END of the TEXT area is re-faced with the trailing-whitespace
//! face; earlier glyphs and any interior (non-trailing) whitespace keep their
//! original face.

use super::HighlightTrailingWhitespaceMutation;
use crate::output::row_request::DisplayCurrentRowMutation;
use neomacs_display_protocol::frame_glyphs::GlyphRowRole;
use neomacs_display_protocol::glyph_matrix::{Glyph, GlyphArea, GlyphRow};
use neomacs_display_protocol::types::FaceId;

const TEXT_FACE: FaceId = FaceId::new(1);
const TWS_FACE: FaceId = FaceId::new(42);

fn push_char(row: &mut GlyphRow, ch: char, charpos: usize) {
    row.glyphs[GlyphArea::Text.index()].push(Glyph::char(ch, TEXT_FACE, charpos));
}

fn push_tab(row: &mut GlyphRow) {
    row.glyphs[GlyphArea::Text.index()].push(Glyph::stretch(8, TEXT_FACE));
}

fn face_ids(row: &GlyphRow) -> Vec<FaceId> {
    row.glyphs[GlyphArea::Text.index()]
        .iter()
        .map(|g| g.face_id)
        .collect()
}

#[test]
fn trailing_spaces_are_refaced_leading_text_kept() {
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    for (i, ch) in "ab  ".chars().enumerate() {
        push_char(&mut row, ch, i);
    }

    HighlightTrailingWhitespaceMutation { face_id: TWS_FACE }.apply(&mut row);

    assert_eq!(
        face_ids(&row),
        vec![TEXT_FACE, TEXT_FACE, TWS_FACE, TWS_FACE],
        "only the two trailing spaces are re-faced"
    );
}

#[test]
fn trailing_tab_stretch_is_refaced() {
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    push_char(&mut row, 'x', 0);
    push_tab(&mut row);

    HighlightTrailingWhitespaceMutation { face_id: TWS_FACE }.apply(&mut row);

    assert_eq!(
        face_ids(&row),
        vec![TEXT_FACE, TWS_FACE],
        "a trailing tab (Stretch) counts as trailing whitespace"
    );
}

#[test]
fn interior_whitespace_is_not_refaced() {
    // "ab  cd  " — only the FINAL run of spaces is trailing.
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    for (i, ch) in "ab  cd  ".chars().enumerate() {
        push_char(&mut row, ch, i);
    }

    HighlightTrailingWhitespaceMutation { face_id: TWS_FACE }.apply(&mut row);

    assert_eq!(
        face_ids(&row),
        vec![
            TEXT_FACE, TEXT_FACE, // a b
            TEXT_FACE, TEXT_FACE, // interior "  " — NOT trailing
            TEXT_FACE, TEXT_FACE, // c d
            TWS_FACE, TWS_FACE, // trailing "  "
        ],
        "interior whitespace between words must be left untouched"
    );
}

#[test]
fn line_with_no_trailing_whitespace_is_unchanged() {
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    for (i, ch) in "abc".chars().enumerate() {
        push_char(&mut row, ch, i);
    }

    HighlightTrailingWhitespaceMutation { face_id: TWS_FACE }.apply(&mut row);

    assert_eq!(
        face_ids(&row),
        vec![TEXT_FACE, TEXT_FACE, TEXT_FACE],
        "a line ending in a non-space glyph keeps every face"
    );
}

#[test]
fn all_whitespace_row_is_fully_refaced() {
    let mut row = GlyphRow::new(GlyphRowRole::Text);
    push_char(&mut row, ' ', 0);
    push_tab(&mut row);
    push_char(&mut row, ' ', 1);

    HighlightTrailingWhitespaceMutation { face_id: TWS_FACE }.apply(&mut row);

    assert_eq!(
        face_ids(&row),
        vec![TWS_FACE, TWS_FACE, TWS_FACE],
        "an all-whitespace row is entirely trailing whitespace"
    );
}
