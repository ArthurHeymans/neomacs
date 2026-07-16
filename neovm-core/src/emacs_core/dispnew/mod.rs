pub mod pure;

#[cfg(test)]
mod tests;

/// Register this module's subrs. GNU: `syms_of_dispnew` in `src/dispnew.c`.
/// Extracted verbatim from the former flat `builtins::init_builtins`.
pub(crate) fn syms_of_dispnew(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.defsubr("sleep-for", super::timer::builtin_sleep_for, 1, Some(2));
    ctx.defsubr(
        "send-string-to-terminal",
        super::dispnew::pure::builtin_send_string_to_terminal,
        1,
        Some(2),
    );
    ctx.defsubr(
        "internal-show-cursor",
        super::dispnew::pure::builtin_internal_show_cursor,
        2,
        Some(2),
    );
    ctx.defsubr(
        "internal-show-cursor-p",
        super::dispnew::pure::builtin_internal_show_cursor_p,
        0,
        None,
    );
    ctx.defsubr(
        "redraw-frame",
        super::dispnew::pure::builtin_redraw_frame,
        0,
        Some(1),
    );
    ctx.defsubr(
        "display--update-for-mouse-movement",
        |ctx, args| {
            crate::emacs_core::builtins::stubs::builtin_display_update_for_mouse_movement(ctx, args)
        },
        3,
        Some(3),
    );
    ctx.defsubr(
        "frame-or-buffer-changed-p",
        |_ctx, args| crate::emacs_core::builtins::stubs::builtin_frame_or_buffer_changed_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "redisplay",
        crate::emacs_core::builtins::symbols::builtin_redisplay,
        0,
        Some(1),
    );

    // -- Dispnew --
    ctx.defsubr(
        "redraw-display",
        |_ctx, args| super::dispnew::pure::builtin_redraw_display(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "open-termscript",
        |_ctx, args| super::dispnew::pure::builtin_open_termscript(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "ding",
        |_ctx, args| super::dispnew::pure::builtin_ding(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "frame--z-order-lessp",
        |_ctx, args| super::dispnew::pure::builtin_frame_z_order_lessp(args),
        0,
        None,
    );
}
