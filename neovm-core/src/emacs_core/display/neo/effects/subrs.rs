//! Native Lisp declarations for renderer effects.

use super::*;
use crate::emacs_core::subr::SubrSpec;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::many("neomacs-effect-set", set, 1, None),
    SubrSpec::many("neomacs-effect-get", get, 1, Some(1)),
    SubrSpec::many("neomacs-effect-reset", reset, 1, Some(1)),
    SubrSpec::many("neomacs-effects-apply", apply, 1, Some(1)),
    SubrSpec::many("neomacs-effect-names", names, 0, Some(1)),
];

pub(crate) fn register_subrs(ctx: &mut Context) {
    ctx.register_subrs(SUBRS);
}
