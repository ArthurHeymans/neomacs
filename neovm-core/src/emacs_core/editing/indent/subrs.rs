//! Native Lisp declarations owned by GNU `src/indent.c`'s mirror.

use super::*;
use crate::emacs_core::subr::SubrSpec;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::many("current-indentation", current_indentation, 0, Some(0)),
    SubrSpec::many("indent-to", indent_to, 1, Some(2)).interactive(
        crate::emacs_core::interactive::BuiltinInteractiveSpec::String("NIndent to column: "),
    ),
    SubrSpec::many("current-column", current_column, 0, Some(0)),
    SubrSpec::many("move-to-column", move_to_column, 1, Some(2)).interactive(
        crate::emacs_core::interactive::BuiltinInteractiveSpec::String("NMove to column: "),
    ),
    SubrSpec::many(
        "line-number-display-width",
        line_number_display_width,
        0,
        Some(1),
    ),
    SubrSpec::many_requires_eval_state("vertical-motion", vertical_motion, 1, Some(3)),
    SubrSpec::many("compute-motion", compute_motion, 7, Some(7)),
];

pub(crate) fn register_subrs(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.register_subrs(SUBRS);
}
