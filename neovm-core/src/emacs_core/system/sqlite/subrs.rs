//! Native Lisp declarations owned by GNU `src/sqlite.c`'s mirror.

use super::*;
use crate::emacs_core::subr::{NativeFn, SubrArity, SubrSpec};

// Keep the feature-enabled declarations in GNU `syms_of_sqlite` order.  This
// table is also the compile-time capability boundary: no operational adapter
// exists in a build that omits the backend.
#[cfg(feature = "sqlite")]
const SUBRS: &[SubrSpec] = &[
    SubrSpec::new(
        "sqlite-open",
        NativeFn::ContextVec(open),
        SubrArity::new(0, Some(3)),
    ),
    SubrSpec::new(
        "sqlite-close",
        NativeFn::NoContextVec(close),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "sqlite-execute",
        NativeFn::ContextVec(execute),
        SubrArity::new(2, Some(3)),
    ),
    SubrSpec::new(
        "sqlite-select",
        NativeFn::ContextVec(select),
        SubrArity::new(2, Some(4)),
    ),
    SubrSpec::new(
        "sqlite-execute-batch",
        NativeFn::ContextVec(execute_batch),
        SubrArity::new(2, Some(2)),
    ),
    SubrSpec::new(
        "sqlite-transaction",
        NativeFn::NoContextVec(transaction),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "sqlite-commit",
        NativeFn::NoContextVec(commit),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "sqlite-rollback",
        NativeFn::NoContextVec(rollback),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "sqlite-pragma",
        NativeFn::NoContextVec(pragma),
        SubrArity::new(2, Some(2)),
    ),
    SubrSpec::new(
        "sqlite-load-extension",
        NativeFn::ContextVec(load_extension),
        SubrArity::new(2, Some(2)),
    ),
    SubrSpec::new(
        "sqlite-next",
        NativeFn::NoContextVec(next),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "sqlite-columns",
        NativeFn::NoContextVec(columns),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "sqlite-more-p",
        NativeFn::NoContextVec(more_p),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "sqlite-finalize",
        NativeFn::NoContextVec(finalize),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "sqlite-version",
        NativeFn::NoContextVec(version),
        SubrArity::new(0, Some(0)),
    ),
    SubrSpec::new(
        "sqlitep",
        NativeFn::NoContextVec(predicate),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "sqlite-available-p",
        NativeFn::NoContextVec(available_p),
        SubrArity::new(0, Some(0)),
    ),
];

#[cfg(not(feature = "sqlite"))]
const SUBRS: &[SubrSpec] = &[
    SubrSpec::new(
        "sqlitep",
        NativeFn::NoContextVec(predicate),
        SubrArity::new(1, Some(1)),
    ),
    SubrSpec::new(
        "sqlite-available-p",
        NativeFn::NoContextVec(available_p),
        SubrArity::new(0, Some(0)),
    ),
];

/// The central startup registrar calls this at GNU's sqlite.c milestone.
pub(crate) fn register_subrs(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.register_subrs(SUBRS);
}
