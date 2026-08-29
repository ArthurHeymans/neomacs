//! Native Lisp declarations owned by GNU `src/composite.c`'s mirror.

use super::*;
use crate::emacs_core::subr::SubrSpec;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::many(
        "compose-region-internal",
        compose_region_internal,
        2,
        Some(4),
    ),
    SubrSpec::many("compose-string-internal", compose_string, 3, Some(5)),
    SubrSpec::many(
        "find-composition-internal",
        find_composition_internal,
        4,
        Some(4),
    ),
    SubrSpec::many(
        "composition-get-gstring",
        composition_get_gstring,
        4,
        Some(4),
    ),
    SubrSpec::many("clear-composition-cache", clear_cache, 0, Some(0)),
    SubrSpec::many("composition-sort-rules", sort_rules, 1, Some(1)),
];

pub(crate) fn register_subrs(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.register_subrs(SUBRS);
}
