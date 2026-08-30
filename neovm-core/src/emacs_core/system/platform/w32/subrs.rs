//! Native Lisp declarations for the Windows platform surface.

use super::*;
use crate::emacs_core::subr::{NativeFn, SubrArity, SubrSpec};

const SUBRS: &[SubrSpec] = &[
    SubrSpec::new(
        "w32-short-file-name",
        NativeFn::Context1(w32_short_file_name),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "w32-long-file-name",
        NativeFn::Context1(w32_long_file_name),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "w32-get-valid-codepages",
        NativeFn::Context0(w32_get_valid_codepages),
        SubrArity::new(0, Some(0)),
    ),
    SubrSpec::new(
        "w32-get-console-codepage",
        NativeFn::Context0(w32_get_console_codepage),
        SubrArity::new(0, Some(0)),
    ),
    SubrSpec::new(
        "w32-set-console-codepage",
        NativeFn::Context1(w32_set_console_codepage),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "w32-get-console-output-codepage",
        NativeFn::Context0(w32_get_console_output_codepage),
        SubrArity::new(0, Some(0)),
    ),
    SubrSpec::new(
        "w32-set-console-output-codepage",
        NativeFn::Context1(w32_set_console_output_codepage),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "w32-get-codepage-charset",
        NativeFn::Context1(w32_get_codepage_charset),
        SubrArity::new(1, Some(1)),
    ),
];

pub(crate) fn register_subrs(ctx: &mut Context) {
    ctx.register_subrs(SUBRS);
}
