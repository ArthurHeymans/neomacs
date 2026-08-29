//! Native Lisp declarations for Little CMS support.

use super::*;
use crate::emacs_core::subr::SubrSpec;

#[cfg(neomacs_have_lcms2)]
const SUBRS: &[SubrSpec] = &[
    SubrSpec::many(
        "lcms-cie-de2000",
        |_ctx, args| lcms_cie_de2000(args),
        2,
        Some(5),
    ),
    SubrSpec::many(
        "lcms-xyz->jch",
        |_ctx, args| lcms_xyz_to_jch(args),
        1,
        Some(3),
    ),
    SubrSpec::many(
        "lcms-jch->xyz",
        |_ctx, args| lcms_jch_to_xyz(args),
        1,
        Some(3),
    ),
    SubrSpec::many(
        "lcms-jch->jab",
        |_ctx, args| lcms_jch_to_jab(args),
        1,
        Some(3),
    ),
    SubrSpec::many(
        "lcms-jab->jch",
        |_ctx, args| lcms_jab_to_jch(args),
        1,
        Some(3),
    ),
    SubrSpec::many(
        "lcms-cam02-ucs",
        |_ctx, args| lcms_cam02_ucs(args),
        2,
        Some(4),
    ),
    SubrSpec::many(
        "lcms2-available-p",
        |_ctx, args| lcms2_available_p(args),
        0,
        Some(0),
    ),
    SubrSpec::many(
        "lcms-temp->white-point",
        |_ctx, args| lcms_temp_to_white_point(args),
        1,
        Some(1),
    ),
];

pub(crate) fn register_subrs(ctx: &mut crate::emacs_core::eval::Context) {
    #[cfg(neomacs_have_lcms2)]
    ctx.register_subrs(SUBRS);

    #[cfg(not(neomacs_have_lcms2))]
    let _ = ctx;
}
