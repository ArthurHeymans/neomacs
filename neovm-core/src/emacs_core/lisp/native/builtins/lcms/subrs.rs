//! Native Lisp declarations for Little CMS support.

use super::*;
use crate::emacs_core::subr::SubrSpec;

#[cfg(neomacs_have_lcms2)]
const SUBRS: &[SubrSpec] = &[
    SubrSpec::many_no_context("lcms-cie-de2000", lcms_cie_de2000, 2, Some(5)),
    SubrSpec::many_no_context("lcms-xyz->jch", lcms_xyz_to_jch, 1, Some(3)),
    SubrSpec::many_no_context("lcms-jch->xyz", lcms_jch_to_xyz, 1, Some(3)),
    SubrSpec::many_no_context("lcms-jch->jab", lcms_jch_to_jab, 1, Some(3)),
    SubrSpec::many_no_context("lcms-jab->jch", lcms_jab_to_jch, 1, Some(3)),
    SubrSpec::many_no_context("lcms-cam02-ucs", lcms_cam02_ucs, 2, Some(4)),
    SubrSpec::many_no_context("lcms2-available-p", lcms2_available_p, 0, Some(0)),
    SubrSpec::many_no_context(
        "lcms-temp->white-point",
        lcms_temp_to_white_point,
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
