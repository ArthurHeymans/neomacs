pub mod pure;

#[cfg(test)]
mod tests;

/// Register this module's subrs. GNU: `syms_of_keyboard` in `src/keyboard.c`.
/// Extracted verbatim from the former flat `builtins::init_builtins`.
pub(crate) fn syms_of_keyboard(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.defsubr(
        "posn-at-point",
        super::xdisp::builtin_posn_at_point,
        0,
        Some(2),
    );
    ctx.defsubr("posn-at-x-y", super::xdisp::builtin_posn_at_x_y, 2, Some(4));
    ctx.defsubr(
        "current-idle-time",
        crate::emacs_core::builtins::misc_eval::builtin_current_idle_time,
        0,
        Some(0),
    );
    ctx.defsubr(
        "this-command-keys",
        super::interactive::builtin_this_command_keys,
        0,
        Some(0),
    );
    ctx.defsubr(
        "input-pending-p",
        super::reader::builtin_input_pending_p,
        0,
        Some(1),
    );
    ctx.defsubr(
        "discard-input",
        super::reader::builtin_discard_input,
        0,
        Some(0),
    );
    ctx.defsubr(
        "current-input-mode",
        super::reader::builtin_current_input_mode,
        0,
        Some(0),
    );
    ctx.defsubr(
        "set-input-mode",
        super::reader::builtin_set_input_mode,
        3,
        Some(4),
    );
    ctx.defsubr(
        "set-input-interrupt-mode",
        super::reader::builtin_set_input_interrupt_mode,
        1,
        Some(1),
    );
    // Keyboard audit Finding 16: register insert-special-event
    // (mirrors GNU `Finsert_special_event` at
    // `src/keyboard.c:12060`). Routes to the same unread queue
    // helper as `unread-command-events`, since neomacs treats
    // every Lisp-side event push the same way.
    ctx.defsubr(
        "insert-special-event",
        super::reader::builtin_insert_special_event,
        1,
        Some(1),
    );
    ctx.defsubr(
        "read-key-sequence",
        super::reader::builtin_read_key_sequence,
        1,
        Some(6),
    );
    ctx.defsubr(
        "read-key-sequence-vector",
        super::reader::builtin_read_key_sequence_vector,
        1,
        Some(6),
    );
    ctx.defsubr(
        "recent-keys",
        crate::emacs_core::builtins::keymaps::builtin_recent_keys,
        0,
        Some(1),
    );
    ctx.defsubr(
        "recursion-depth",
        super::misc::builtin_recursion_depth,
        0,
        Some(0),
    );
    ctx.defsubr(
        "exit-recursive-edit",
        super::minibuffer::builtin_exit_recursive_edit,
        0,
        Some(0),
    );
    ctx.defsubr(
        "abort-recursive-edit",
        super::minibuffer::builtin_abort_recursive_edit,
        0,
        Some(0),
    );
    ctx.defsubr(
        "read-char-exclusive",
        super::lread::builtin_read_char_exclusive,
        0,
        Some(3),
    );
    ctx.defsubr("read-char", super::reader::builtin_read_char, 0, Some(3));
    ctx.defsubr(
        "recursive-edit",
        super::minibuffer::builtin_recursive_edit,
        0,
        Some(0),
    );
    ctx.defsubr("read-event", super::lread::builtin_read_event, 0, Some(3));
    ctx.defsubr(
        "event-convert-list",
        |_ctx, args| crate::emacs_core::builtins::keymaps::builtin_event_convert_list(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "internal--track-mouse",
        |ctx, args| crate::emacs_core::builtins::symbols::builtin_internal_track_mouse(ctx, args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "internal-event-symbol-parse-modifiers",
        |ctx, args| {
            crate::emacs_core::builtins::symbols::builtin_internal_event_symbol_parse_modifiers(
                ctx, args,
            )
        },
        1,
        Some(1),
    );
    ctx.defsubr(
        "internal-handle-focus-in",
        |ctx, args| {
            crate::emacs_core::builtins::symbols::builtin_internal_handle_focus_in(ctx, args)
        },
        1,
        Some(1),
    );
    ctx.defsubr(
        "open-dribble-file",
        crate::emacs_core::builtins::symbols::builtin_open_dribble_file,
        1,
        Some(1),
    );
    ctx.defsubr(
        "set--this-command-keys",
        crate::emacs_core::builtins::symbols::builtin_set_this_command_keys,
        1,
        Some(1),
    );
    crate::emacs_core::builtins::register_builtin(
        ctx,
        crate::emacs_core::builtins::BuiltinRegistration::placeholder(
            "suspend-emacs",
            |_ctx, args| crate::emacs_core::builtins::symbols::builtin_suspend_emacs(args),
            0,
            Some(1),
            crate::emacs_core::builtins::BuiltinNoEvalPlaceholder::Nil,
        ),
    );
    ctx.defsubr(
        "lossage-size",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_lossage_size(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "set-input-meta-mode",
        |_ctx, args| super::reader::builtin_set_input_meta_mode(args),
        1,
        Some(2),
    );
    ctx.defsubr(
        "set-output-flow-control",
        |_ctx, args| super::reader::builtin_set_output_flow_control(args),
        1,
        Some(2),
    );
    ctx.defsubr(
        "set-quit-char",
        super::reader::builtin_set_quit_char,
        1,
        Some(1),
    );
    ctx.defsubr(
        "top-level",
        |_ctx, args| super::minibuffer::builtin_top_level(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "this-command-keys-vector",
        super::interactive::builtin_this_command_keys_vector,
        0,
        Some(0),
    );
    crate::emacs_core::builtins::register_builtin(
        ctx,
        crate::emacs_core::builtins::BuiltinRegistration::placeholder(
            "this-single-command-keys",
            super::interactive::builtin_this_single_command_keys,
            0,
            Some(0),
            crate::emacs_core::builtins::BuiltinNoEvalPlaceholder::Nil,
        ),
    );
    crate::emacs_core::builtins::register_builtin(
        ctx,
        crate::emacs_core::builtins::BuiltinRegistration::placeholder(
            "this-single-command-raw-keys",
            super::interactive::builtin_this_single_command_raw_keys,
            0,
            Some(0),
            crate::emacs_core::builtins::BuiltinNoEvalPlaceholder::Nil,
        ),
    );
    ctx.defsubr(
        "clear-this-command-keys",
        super::interactive::builtin_clear_this_command_keys,
        0,
        Some(1),
    );
    ctx.defsubr(
        "command-error-default-function",
        |_ctx, args| {
            crate::emacs_core::builtins::buffers::builtin_command_error_default_function(args)
        },
        3,
        Some(3),
    );
}
