//! Native Lisp declarations owned by GNU `src/font.c`'s mirror.

use super::*;
use crate::emacs_core::subr::SubrSpec;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::many("fontp", |_ctx, args| fontp(args), 1, Some(2)),
    SubrSpec::many("font-spec", |_ctx, args| font_spec(args), 0, None),
    SubrSpec::many("font-get", |_ctx, args| font_get(args), 2, Some(2)),
    SubrSpec::many(
        "font-face-attributes",
        |_ctx, args| font_face_attributes(args),
        1,
        Some(2),
    ),
    SubrSpec::many("font-put", |_ctx, args| font_put(args), 3, Some(3)),
    SubrSpec::many("list-fonts", list_fonts, 1, Some(4)),
    SubrSpec::many("font-family-list", font_family_list, 0, Some(1)),
    SubrSpec::many("find-font", find_font, 1, Some(2)),
    SubrSpec::many(
        "font-xlfd-name",
        |_ctx, args| font_xlfd_name(args),
        1,
        Some(3),
    ),
    SubrSpec::many(
        "clear-font-cache",
        |_ctx, args| clear_font_cache(args),
        0,
        Some(0),
    ),
    SubrSpec::many("font-shape-gstring", font_shape_gstring, 2, Some(2)),
    SubrSpec::many(
        "font-variation-glyphs",
        |_ctx, args| font_variation_glyphs(args),
        2,
        Some(2),
    ),
    SubrSpec::many("internal-char-font", internal_char_font, 1, Some(2)),
    SubrSpec::many("close-font", |_ctx, args| close_font(args), 1, Some(2)),
    SubrSpec::many("query-font", query_font, 1, Some(1)),
    SubrSpec::many(
        "font-get-glyphs",
        |_ctx, args| font_get_glyphs(args),
        3,
        Some(4),
    ),
    SubrSpec::many(
        "font-has-char-p",
        |_ctx, args| font_has_char_p(args),
        2,
        Some(3),
    ),
    SubrSpec::many("font-match-p", |_ctx, args| font_match_p(args), 2, Some(2)),
    SubrSpec::many_requires_eval_state("font-at", font_at, 1, Some(3)),
    SubrSpec::many("font-info", font_info, 1, Some(2)),
];

/// The central startup registrar sequences this module after sqlite.c,
/// matching GNU `emacs.c`.
pub(crate) fn register_subrs(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.register_subrs(SUBRS);
}
