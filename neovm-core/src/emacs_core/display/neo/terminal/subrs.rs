//! Native Lisp declarations for compositor-owned terminals.

use super::{Context, create, destroy, get_text, resize, set_float, write};
use crate::emacs_core::subr::SubrSpec;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::many("neomacs-terminal-create", create, 3, Some(4)),
    SubrSpec::many("neomacs-terminal-write", write, 2, Some(2)),
    SubrSpec::many("neomacs-terminal-resize", resize, 3, Some(3)),
    SubrSpec::many("neomacs-terminal-destroy", destroy, 1, Some(1)),
    SubrSpec::many("neomacs-terminal-set-float", set_float, 4, Some(4)),
    SubrSpec::many("neomacs-terminal-get-text", get_text, 1, Some(1)),
];

pub(crate) fn register_subrs(ctx: &mut Context) {
    ctx.register_subrs(SUBRS);
}
