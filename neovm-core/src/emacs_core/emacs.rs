//! Mirror of GNU `src/emacs.c` — the bootstrap-facing subset.
//!
//! GNU emacs.c owns process-level state: invocation identity, daemon mode,
//! kill-emacs, dump. Only the pieces neovm currently ports live here; the
//! rest of emacs.c's DEFUN surface (kill-emacs, dump-emacs-portable, ...) is
//! still registered from `builtins::init_builtins` and should migrate here
//! as it is touched (docs/design/neovm-core-layout.md, rule 1).

use super::error::EvalResult;
use super::value::*;
use crate::emacs_core::defun::defun;

// (daemonp) — neomacs never runs as a daemon yet, so this is constantly nil,
// matching GNU's return for a non-daemon session.
defun!(DAEMONP: "daemonp",
fn daemonp(_ctx: &mut Context) -> EvalResult {
    Ok(Value::NIL)
});

// (daemon-initialized) — GNU signals unless running as a daemon.
defun!(DAEMON_INITIALIZED: "daemon-initialized",
fn daemon_initialized(_ctx: &mut Context) -> EvalResult {
    Err(super::error::signal(
        "error",
        vec![Value::string(
            "This function can only be called if emacs is run as a daemon",
        )],
    ))
});

// (invocation-directory) — like GNU Finvocation_directory: returns a copy of
// the `invocation-directory` variable so callers cannot mutate the original.
defun!(INVOCATION_DIRECTORY: "invocation-directory",
fn invocation_directory(ctx: &mut Context) -> EvalResult {
    let value = ctx.eval_symbol_by_id(super::intern::intern("invocation-directory"))?;
    crate::emacs_core::builtins::builtin_copy_sequence(vec![value])
});

// (invocation-name) — like GNU Finvocation_name.
defun!(INVOCATION_NAME: "invocation-name",
fn invocation_name(ctx: &mut Context) -> EvalResult {
    let value = ctx.eval_symbol_by_id(super::intern::intern("invocation-name"))?;
    crate::emacs_core::builtins::builtin_copy_sequence(vec![value])
});

/// Register this module's subrs. GNU: `syms_of_emacs` in `src/emacs.c`.
pub(crate) fn syms_of_emacs(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.defsubr_decl(&DAEMONP);
    ctx.defsubr_decl(&DAEMON_INITIALIZED);
    ctx.defsubr_decl(&INVOCATION_DIRECTORY);
    ctx.defsubr_decl(&INVOCATION_NAME);
}
