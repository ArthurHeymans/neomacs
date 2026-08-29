//! Native Lisp declarations for shader surfaces.

use super::*;
use crate::emacs_core::subr::SubrSpec;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::many("neomacs-surface-create", create, 0, None),
    SubrSpec::many("neomacs-surface-set-uniform", set_uniform, 3, Some(3)),
    SubrSpec::many("neomacs-surface-destroy", destroy, 1, Some(1)),
    SubrSpec::many("neomacs-surface-available-p", available, 0, Some(0)),
    SubrSpec::many("neomacs-frame-shader", set_frame_shader, 1, Some(3)),
    SubrSpec::many(
        "neomacs-frame-shader-set-uniform",
        set_frame_shader_uniform,
        2,
        Some(2),
    ),
];

pub(crate) fn register_subrs(ctx: &mut Context) {
    ctx.register_subrs(SUBRS);
}
