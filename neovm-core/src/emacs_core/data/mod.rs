//! Placeholder for data.c Rust rewrite module.
//!
//! Type predicates, accessors, arithmetic, and symbol operations are
//! already dispatched via builtins.rs. This module will hold dedicated
//! implementations as they are extracted from the monolithic dispatcher.

/// Register this module's subrs. GNU: `syms_of_data` in `src/data.c`.
/// Extracted verbatim from the former flat `builtins::init_builtins`.
pub(crate) fn syms_of_data(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.defsubr(
        "defalias",
        crate::emacs_core::builtins::misc_eval::builtin_defalias,
        2,
        Some(3),
    );
    ctx.defsubr_1(
        "boundp",
        crate::emacs_core::builtins::symbols::builtin_boundp_1,
        1,
    );
    ctx.defsubr(
        "default-boundp",
        crate::emacs_core::builtins::symbols::builtin_default_boundp,
        1,
        Some(1),
    );
    ctx.defsubr_1(
        "fboundp",
        crate::emacs_core::builtins::symbols::builtin_fboundp_1,
        1,
    );
    crate::emacs_core::builtins::register_builtin(
        ctx,
        crate::emacs_core::builtins::BuiltinRegistration::requires_eval_state(
            "indirect-variable",
            crate::emacs_core::builtins::symbols::builtin_indirect_variable,
            1,
            Some(1),
        ),
    );
    ctx.defsubr_1(
        "symbol-value",
        crate::emacs_core::builtins::symbols::builtin_symbol_value_1,
        1,
    );
    ctx.defsubr_1(
        "symbol-function",
        crate::emacs_core::builtins::symbols::builtin_symbol_function_1,
        1,
    );
    ctx.defsubr_2(
        "set",
        crate::emacs_core::builtins::symbols::builtin_set_2,
        2,
    );
    ctx.defsubr(
        "fset",
        crate::emacs_core::builtins::symbols::builtin_fset,
        2,
        Some(2),
    );
    ctx.defsubr(
        "makunbound",
        crate::emacs_core::builtins::symbols::builtin_makunbound,
        1,
        Some(1),
    );
    ctx.defsubr(
        "fmakunbound",
        crate::emacs_core::builtins::symbols::builtin_fmakunbound,
        1,
        Some(1),
    );
    crate::emacs_core::builtins::register_builtin(
        ctx,
        crate::emacs_core::builtins::BuiltinRegistration::requires_eval_state(
            "setplist",
            crate::emacs_core::builtins::symbols::builtin_setplist,
            2,
            Some(2),
        ),
    );
    ctx.defsubr(
        "symbol-plist",
        crate::emacs_core::builtins::symbols::builtin_symbol_plist_fn,
        1,
        Some(1),
    );
    ctx.defsubr(
        "indirect-function",
        crate::emacs_core::builtins::symbols::builtin_indirect_function,
        1,
        Some(2),
    );
    ctx.defsubr(
        "local-variable-if-set-p",
        crate::emacs_core::builtins::symbols::builtin_local_variable_if_set_p,
        1,
        Some(2),
    );
    ctx.defsubr(
        "variable-binding-locus",
        crate::emacs_core::builtins::symbols::builtin_variable_binding_locus,
        1,
        Some(1),
    );
    ctx.defsubr(
        "interactive-form",
        crate::emacs_core::builtins::symbols::builtin_interactive_form,
        1,
        Some(1),
    );
    ctx.defsubr(
        "command-modes",
        super::interactive::builtin_command_modes,
        1,
        Some(1),
    );
    // Timer functions (run-at-time, run-with-timer, run-with-idle-timer,
    // cancel-timer, timerp, timer-activate) are NOT C primitives in GNU
    // Emacs — they're defined in timer.el as Elisp functions.
    // The C layer only provides timer-check (in keyboard.rs) which reads
    // timer-list / timer-idle-list and calls timer-event-handler.
    // Registering them as Rust builtins would shadow the Elisp definitions
    // and create an incompatible parallel timer system.
    crate::emacs_core::builtins::register_builtin(
        ctx,
        crate::emacs_core::builtins::BuiltinRegistration::requires_eval_state(
            "add-variable-watcher",
            super::advice::builtin_add_variable_watcher,
            2,
            Some(2),
        ),
    );
    crate::emacs_core::builtins::register_builtin(
        ctx,
        crate::emacs_core::builtins::BuiltinRegistration::requires_eval_state(
            "remove-variable-watcher",
            super::advice::builtin_remove_variable_watcher,
            2,
            Some(2),
        ),
    );
    crate::emacs_core::builtins::register_builtin(
        ctx,
        crate::emacs_core::builtins::BuiltinRegistration::requires_eval_state(
            "get-variable-watchers",
            super::advice::builtin_get_variable_watchers,
            1,
            Some(1),
        ),
    );
    ctx.defsubr(
        "make-local-variable",
        super::custom::builtin_make_local_variable,
        1,
        Some(1),
    );
    ctx.defsubr(
        "local-variable-p",
        super::custom::builtin_local_variable_p,
        1,
        Some(2),
    );
    ctx.defsubr(
        "kill-local-variable",
        super::custom::builtin_kill_local_variable,
        0,
        None,
    );
    ctx.defsubr(
        "default-value",
        super::custom::builtin_default_value,
        1,
        Some(1),
    );
    ctx.defsubr(
        "set-default",
        super::custom::builtin_set_default,
        2,
        Some(2),
    );
    ctx.defsubr("threadp", super::threads::builtin_threadp, 1, Some(1));
    ctx.defsubr("mutexp", super::threads::builtin_mutexp, 1, Some(1));
    ctx.defsubr(
        "condition-variable-p",
        super::threads::builtin_condition_variable_p,
        0,
        None,
    );
    ctx.defsubr(
        "integer-or-marker-p",
        |_ctx, args| crate::emacs_core::builtins::types::builtin_integer_or_marker_p(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "number-or-marker-p",
        |_ctx, args| crate::emacs_core::builtins::types::builtin_number_or_marker_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "vector-or-char-table-p",
        |_ctx, args| crate::emacs_core::builtins::types::builtin_vector_or_char_table_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "markerp",
        |_ctx, args| super::marker::builtin_markerp(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "bool-vector-p",
        |_ctx, args| super::chartable::builtin_bool_vector_p(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "module-function-p",
        |_ctx, args| crate::emacs_core::builtins::types::builtin_module_function_p(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "user-ptrp",
        |_ctx, args| crate::emacs_core::builtins::types::builtin_user_ptrp(args),
        1,
        Some(1),
    );
    ctx.defsubr_1(
        "symbol-with-pos-p",
        crate::emacs_core::builtins::types::builtin_symbol_with_pos_p_1,
        1,
    );
    ctx.defsubr_1(
        "symbol-with-pos-pos",
        crate::emacs_core::builtins::types::builtin_symbol_with_pos_pos_1,
        1,
    );
    ctx.defsubr_1(
        "bare-symbol",
        super::builtins_extra::builtin_bare_symbol_1,
        1,
    );
    ctx.defsubr(
        "logcount",
        |_ctx, args| super::editfns::builtin_logcount(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "position-symbol",
        |ctx, args| crate::emacs_core::builtins::symbols::builtin_position_symbol(ctx, args),
        2,
        Some(2),
    );
    ctx.defsubr_1(
        "recordp",
        crate::emacs_core::builtins::symbols::builtin_recordp_1,
        1,
    );
    ctx.defsubr(
        "remove-pos-from-symbol",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_remove_pos_from_symbol(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "subr-native-lambda-list",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_subr_native_lambda_list(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "subr-type",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_subr_type(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "make-variable-buffer-local",
        super::custom::builtin_make_variable_buffer_local,
        1,
        Some(1),
    );
    ctx.defsubr_1("closurep", super::builtins_extra::builtin_closurep_1, 1);

    // -----------------------------------------------------------------------
    // Additional builtins registered via defsubr.
    // -----------------------------------------------------------------------

    // -- Arithmetic --
    ctx.defsubr_slice("+", super::builtins::arithmetic::builtin_add_slice, 0, None);
    ctx.defsubr_slice("-", super::builtins::arithmetic::builtin_sub_slice, 0, None);
    ctx.defsubr(
        "*",
        |_ctx, args| crate::emacs_core::builtins::arithmetic::builtin_mul(args),
        0,
        None,
    );
    ctx.defsubr(
        "/",
        |_ctx, args| crate::emacs_core::builtins::arithmetic::builtin_div(args),
        1,
        None,
    );
    ctx.defsubr(
        "%",
        |_ctx, args| crate::emacs_core::builtins::arithmetic::builtin_percent(args),
        2,
        Some(2),
    );
    ctx.defsubr(
        "mod",
        |_ctx, args| crate::emacs_core::builtins::arithmetic::builtin_mod(args),
        2,
        Some(2),
    );
    ctx.defsubr_1(
        "1+",
        crate::emacs_core::builtins::arithmetic::builtin_add1_1,
        1,
    );
    ctx.defsubr_1(
        "1-",
        crate::emacs_core::builtins::arithmetic::builtin_sub1_1,
        1,
    );
    ctx.defsubr_slice(
        "max",
        crate::emacs_core::builtins::arithmetic::builtin_max_slice,
        1,
        None,
    );
    ctx.defsubr_slice(
        "min",
        crate::emacs_core::builtins::arithmetic::builtin_min_slice,
        1,
        None,
    );

    // -- Logical / bitwise --
    ctx.defsubr_slice(
        "logand",
        |_ctx, args| crate::emacs_core::builtins::arithmetic::builtin_logand_slice(args),
        0,
        None,
    );
    ctx.defsubr_slice(
        "logior",
        |_ctx, args| crate::emacs_core::builtins::arithmetic::builtin_logior_slice(args),
        0,
        None,
    );
    ctx.defsubr_slice(
        "logxor",
        |_ctx, args| crate::emacs_core::builtins::arithmetic::builtin_logxor_slice(args),
        0,
        None,
    );
    ctx.defsubr(
        "lognot",
        |_ctx, args| crate::emacs_core::builtins::arithmetic::builtin_lognot(args),
        1,
        Some(1),
    );
    ctx.defsubr_slice(
        "ash",
        |_ctx, args| crate::emacs_core::builtins::arithmetic::builtin_ash_slice(args),
        2,
        Some(2),
    );

    // -- Numeric comparisons --
    ctx.defsubr_slice(
        "=",
        crate::emacs_core::builtins::arithmetic::builtin_num_eq_slice,
        1,
        None,
    );
    ctx.defsubr_slice(
        "<",
        crate::emacs_core::builtins::arithmetic::builtin_num_lt_slice,
        1,
        None,
    );
    ctx.defsubr_slice(
        "<=",
        crate::emacs_core::builtins::arithmetic::builtin_num_le_slice,
        1,
        None,
    );
    ctx.defsubr_slice(
        ">",
        crate::emacs_core::builtins::arithmetic::builtin_num_gt_slice,
        1,
        None,
    );
    ctx.defsubr_slice(
        ">=",
        crate::emacs_core::builtins::arithmetic::builtin_num_ge_slice,
        1,
        None,
    );
    ctx.defsubr_2(
        "/=",
        crate::emacs_core::builtins::arithmetic::builtin_num_ne_2,
        2,
    );

    // -- Type predicates --
    ctx.defsubr_1(
        "null",
        crate::emacs_core::builtins::types::builtin_null_1,
        1,
    );
    ctx.defsubr_1(
        "atom",
        crate::emacs_core::builtins::types::builtin_atom_1,
        1,
    );
    ctx.defsubr_1(
        "consp",
        crate::emacs_core::builtins::types::builtin_consp_1,
        1,
    );
    ctx.defsubr_1(
        "listp",
        crate::emacs_core::builtins::types::builtin_listp_1,
        1,
    );
    ctx.defsubr_1(
        "nlistp",
        crate::emacs_core::builtins::types::builtin_nlistp_1,
        1,
    );
    ctx.defsubr_1(
        "symbolp",
        crate::emacs_core::builtins::types::builtin_symbolp_1,
        1,
    );
    ctx.defsubr_1(
        "numberp",
        crate::emacs_core::builtins::types::builtin_numberp_1,
        1,
    );
    ctx.defsubr_1(
        "integerp",
        crate::emacs_core::builtins::types::builtin_integerp_1,
        1,
    );
    ctx.defsubr_1(
        "floatp",
        crate::emacs_core::builtins::types::builtin_floatp_1,
        1,
    );
    ctx.defsubr_1(
        "stringp",
        crate::emacs_core::builtins::types::builtin_stringp_1,
        1,
    );
    ctx.defsubr_1(
        "vectorp",
        crate::emacs_core::builtins::types::builtin_vectorp_1,
        1,
    );
    ctx.defsubr_1(
        "keywordp",
        crate::emacs_core::builtins::types::builtin_keywordp_1,
        1,
    );
    ctx.defsubr(
        "bufferp",
        |_ctx, args| crate::emacs_core::builtins::buffers::builtin_bufferp(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "type-of",
        super::builtins::types::builtin_type_of_with_ctx,
        1,
        Some(1),
    );
    ctx.defsubr(
        "sequencep",
        |_ctx, args| crate::emacs_core::builtins::types::builtin_sequencep(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "arrayp",
        |_ctx, args| crate::emacs_core::builtins::types::builtin_arrayp(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "cl-type-of",
        |_ctx, args| crate::emacs_core::builtins::types::builtin_cl_type_of(args),
        1,
        Some(1),
    );

    // -- Equality --
    ctx.defsubr_2("eq", crate::emacs_core::builtins::types::builtin_eq_2, 2);
    ctx.defsubr_1(
        "car",
        crate::emacs_core::builtins::cons_list::builtin_car_1,
        1,
    );
    ctx.defsubr_1(
        "cdr",
        crate::emacs_core::builtins::cons_list::builtin_cdr_1,
        1,
    );
    ctx.defsubr_1(
        "car-safe",
        crate::emacs_core::builtins::cons_list::builtin_car_safe_1,
        1,
    );
    ctx.defsubr_1(
        "cdr-safe",
        crate::emacs_core::builtins::cons_list::builtin_cdr_safe_1,
        1,
    );
    ctx.defsubr_2(
        "setcar",
        crate::emacs_core::builtins::cons_list::builtin_setcar_2,
        2,
    );
    ctx.defsubr_2(
        "setcdr",
        crate::emacs_core::builtins::cons_list::builtin_setcdr_2,
        2,
    );
    ctx.defsubr(
        "string-to-number",
        |_ctx, args| crate::emacs_core::builtins::strings::builtin_string_to_number(args),
        1,
        Some(2),
    );
    ctx.defsubr(
        "number-to-string",
        |ctx, args| crate::emacs_core::builtins::strings::builtin_number_to_string(ctx, args),
        1,
        Some(1),
    );
    ctx.defsubr_2(
        "aref",
        crate::emacs_core::builtins::collections::builtin_aref_2,
        2,
    );
    ctx.defsubr(
        "aset",
        |_ctx, args| crate::emacs_core::builtins::collections::builtin_aset(args),
        3,
        Some(3),
    );

    // -- Symbol --
    ctx.defsubr_1(
        "symbol-name",
        crate::emacs_core::builtins::misc_pure::builtin_symbol_name_1,
        1,
    );

    // -- Subr introspection --
    ctx.defsubr(
        "subr-name",
        |_ctx, args| super::subr_info::builtin_subr_name(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "subr-arity",
        super::subr_info::builtin_subr_arity,
        1,
        Some(1),
    );
    ctx.defsubr(
        "native-comp-function-p",
        |_ctx, args| super::subr_info::builtin_native_comp_function_p(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "interpreted-function-p",
        |_ctx, args| super::subr_info::builtin_interpreted_function_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "multibyte-string-p",
        |_ctx, args| crate::encoding::builtin_multibyte_string_p(args),
        0,
        None,
    );
    ctx.defsubr(
        "char-or-string-p",
        |_ctx, args| crate::encoding::builtin_char_or_string_p(args),
        1,
        Some(1),
    );

    // -- Char-table / bool-vector --
    ctx.defsubr(
        "char-table-p",
        |_ctx, args| super::chartable::builtin_char_table_p(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "bool-vector-count-population",
        |_ctx, args| super::chartable::builtin_bool_vector_count_population(args),
        1,
        Some(1),
    );
    ctx.defsubr(
        "bool-vector-count-consecutive",
        |_ctx, args| super::chartable::builtin_bool_vector_count_consecutive(args),
        3,
        Some(3),
    );
    ctx.defsubr(
        "bool-vector-intersection",
        |_ctx, args| super::chartable::builtin_bool_vector_intersection(args),
        2,
        Some(3),
    );
    ctx.defsubr(
        "bool-vector-not",
        |_ctx, args| super::chartable::builtin_bool_vector_not(args),
        1,
        Some(2),
    );
    ctx.defsubr(
        "bool-vector-set-difference",
        |_ctx, args| super::chartable::builtin_bool_vector_set_difference(args),
        0,
        None,
    );
    ctx.defsubr(
        "bool-vector-union",
        |_ctx, args| super::chartable::builtin_bool_vector_union(args),
        0,
        None,
    );
    ctx.defsubr(
        "bool-vector-exclusive-or",
        |_ctx, args| super::chartable::builtin_bool_vector_exclusive_or(args),
        2,
        Some(3),
    );
    ctx.defsubr(
        "bool-vector-subsetp",
        |_ctx, args| super::chartable::builtin_bool_vector_subsetp(args),
        2,
        Some(2),
    );
}
