//! Native Lisp declarations owned by GNU `src/data.c`'s mirror.

use super::*;
use crate::emacs_core::subr::SubrSpec;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::many("default-boundp", default_boundp, 1, Some(1)),
    SubrSpec::many("default-value", default_value, 1, Some(1)),
    SubrSpec::many("set-default", set_default, 2, Some(2)),
];

pub(crate) fn register_subrs(ctx: &mut Context) {
    ctx.register_subrs(SUBRS);
}
