//! Central grapheme-cluster composition rules for the layout walks.
//!
//! GNU Emacs groups characters into composed grapheme clusters via its
//! automatic-composition machinery, carrying a `struct composition_it`
//! (src/dispextern.h) on the display iterator so every text-producing path
//! groups clusters identically. neomacs's layout walks historically each
//! made their own ad-hoc `is_cluster_extender` checks; this module is the
//! single source of truth they share.
//!
//! Phase 2 covers grapheme clusters (combining marks, variation selectors,
//! ZWJ emoji sequences, regional-indicator flag pairs) — the cases GNU
//! composes by default. Contextual-shaping scripts (Arabic, Indic) and the
//! glyph-id gstring arrive in a later phase.

use crate::unicode::{is_cluster_extender, is_regional_indicator, is_wide_char};

/// Display columns occupied by a base character before clustering.
///
/// Regional indicators are forced to 2 columns so a composed flag fills a
/// full 2-column cell instead of overlapping the next glyph; everything
/// else defers to the shared char-width table (GNU's default
/// `char-width-table`).
pub(crate) fn base_width_cols(ch: char) -> u8 {
    if is_wide_char(ch) || is_regional_indicator(ch as u32) {
        2
    } else {
        1
    }
}

/// Whether `ch` continues the grapheme cluster of the previously emitted
/// text glyph, given that glyph's `tail` — `(last_char,
/// is_lone_regional_indicator)` from
/// `GlyphMatrixBuilder::last_text_cluster_tail`, or `None` at a row start.
///
/// A character continues the cluster when it is a cluster extender
/// (combining mark, variation selector, ZWJ, skin-tone modifier), when it
/// follows a ZWJ (a member of an emoji ZWJ sequence), or when it is the
/// second regional indicator after a lone one (a flag pair). This is the
/// single rule every layout char loop consults so clustering is identical
/// across buffer text, overlay strings, and display strings — neomacs's
/// stand-in for GNU's shared `composition_it` walk.
pub(crate) fn continues_cluster(ch: char, tail: Option<(char, bool)>) -> bool {
    is_cluster_extender(ch)
        || matches!(tail, Some((prev, _)) if prev == '\u{200D}')
        || (is_regional_indicator(ch as u32) && matches!(tail, Some((_, true))))
}

#[cfg(test)]
#[path = "composition_test.rs"]
mod tests;
