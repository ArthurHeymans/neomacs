//! Native Lisp declarations owned by GNU `src/font.c`'s mirror.

use super::*;
use crate::emacs_core::subr::SubrSpec;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::many_no_context("fontp", fontp, 1, Some(2)),
    SubrSpec::many_no_context("font-spec", font_spec, 0, None),
    SubrSpec::many_no_context("font-get", font_get, 2, Some(2)),
    SubrSpec::many_no_context("font-face-attributes", font_face_attributes, 1, Some(2)),
    SubrSpec::many_no_context("font-put", font_put, 3, Some(3)),
    SubrSpec::many("list-fonts", list_fonts, 1, Some(4)),
    SubrSpec::many("font-family-list", font_family_list, 0, Some(1)),
    SubrSpec::many("find-font", find_font, 1, Some(2)),
    SubrSpec::many_no_context("font-xlfd-name", font_xlfd_name, 1, Some(3)),
    SubrSpec::many_no_context("clear-font-cache", clear_font_cache, 0, Some(0)),
    SubrSpec::many("font-shape-gstring", font_shape_gstring, 2, Some(2)),
    SubrSpec::many_no_context("font-variation-glyphs", font_variation_glyphs, 2, Some(2)),
    SubrSpec::many("internal-char-font", internal_char_font, 1, Some(2)),
    SubrSpec::many_no_context("close-font", close_font, 1, Some(2)),
    SubrSpec::many("query-font", query_font, 1, Some(1)),
    SubrSpec::many_no_context("font-get-glyphs", font_get_glyphs, 3, Some(4)),
    SubrSpec::many_no_context("font-has-char-p", font_has_char_p, 2, Some(3)),
    SubrSpec::many_no_context("font-match-p", font_match_p, 2, Some(2)),
    SubrSpec::many_requires_eval_state("font-at", font_at, 1, Some(3)),
    SubrSpec::many("font-info", font_info, 1, Some(2)),
];

/// The central startup registrar sequences this module after sqlite.c,
/// matching GNU `emacs.c`.
pub(crate) fn register_subrs(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.register_subrs(SUBRS);
}
