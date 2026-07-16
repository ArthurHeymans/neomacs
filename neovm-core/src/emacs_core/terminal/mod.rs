pub mod pure;

#[cfg(test)]
mod tests;

/// Register this module's subrs. GNU: `syms_of_terminal` in `src/terminal.c`.
/// Extracted verbatim from the former flat `builtins::init_builtins`.
pub(crate) fn syms_of_terminal(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.defsubr(
        "frame-initial-p",
        super::window_cmds::builtin_frame_initial_p,
        0,
        Some(1),
    );
    crate::emacs_core::builtins::register_builtin(
        ctx,
        crate::emacs_core::builtins::BuiltinRegistration::requires_eval_state(
            "terminal-name",
            super::terminal::pure::builtin_terminal_name,
            0,
            Some(1),
        ),
    );
    crate::emacs_core::builtins::register_builtin(
        ctx,
        crate::emacs_core::builtins::BuiltinRegistration::requires_eval_state(
            "terminal-live-p",
            super::terminal::pure::builtin_terminal_live_p,
            1,
            Some(1),
        ),
    );
    ctx.defsubr(
        "terminal-parameter",
        super::terminal::pure::builtin_terminal_parameter,
        2,
        Some(2),
    );
    ctx.defsubr(
        "terminal-parameters",
        super::terminal::pure::builtin_terminal_parameters,
        0,
        Some(1),
    );
    ctx.defsubr(
        "set-terminal-parameter",
        super::terminal::pure::builtin_set_terminal_parameter,
        3,
        Some(3),
    );
    ctx.defsubr(
        "frame-terminal",
        super::terminal::pure::builtin_frame_terminal,
        0,
        Some(1),
    );
    ctx.defsubr(
        "terminal-list",
        |_ctx, args| super::terminal::pure::builtin_terminal_list(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "delete-terminal",
        |ctx, args| super::terminal::pure::builtin_delete_terminal(ctx, args),
        0,
        Some(2),
    );
}
