//! Native Lisp declarations for the Windows platform surface.

use super::*;
use crate::emacs_core::subr::SubrSpec;

const SUBRS: &[SubrSpec] = &[
    SubrSpec::a1("w32-short-file-name", w32_short_file_name).required_args(1),
    SubrSpec::a1("w32-long-file-name", w32_long_file_name).required_args(1),
    SubrSpec::a0("w32-get-valid-codepages", w32_get_valid_codepages),
    SubrSpec::a0("w32-get-console-codepage", w32_get_console_codepage),
    SubrSpec::a1("w32-set-console-codepage", w32_set_console_codepage).required_args(1),
    SubrSpec::a0(
        "w32-get-console-output-codepage",
        w32_get_console_output_codepage,
    ),
    SubrSpec::a1(
        "w32-set-console-output-codepage",
        w32_set_console_output_codepage,
    )
    .required_args(1),
    SubrSpec::a1("w32-get-codepage-charset", w32_get_codepage_charset).required_args(1),
];

pub(crate) fn register_subrs(ctx: &mut Context) {
    ctx.register_subrs(SUBRS);
}
