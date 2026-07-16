//! Bootstrap-facing subset of GNU Emacs's `alloc.c`.
//!
//! GNU exposes several GC / memory-management variables from C before Lisp
//! startup runs.  Keep those defaults here so Lisp like `jit-lock.el` can rely
//! on the same low-level variables during runtime and bootstrap.

use crate::emacs_core::symbol::Obarray;
use crate::emacs_core::value::Value;

/// Register bootstrap variables owned by the allocation / GC subsystem.
pub fn register_bootstrap_vars(obarray: &mut Obarray) {
    obarray.set_symbol_value("gc-cons-threshold", Value::fixnum(800_000));
    obarray.make_special("gc-cons-threshold");
    obarray.set_symbol_value("gc-cons-percentage", Value::make_float(0.1));
    obarray.make_special("gc-cons-percentage");
    obarray.set_symbol_value("garbage-collection-messages", Value::NIL);
    obarray.make_special("garbage-collection-messages");
    obarray.set_symbol_value("post-gc-hook", Value::NIL);
    obarray.make_special("post-gc-hook");
    obarray.set_symbol_value(
        "memory-signal-data",
        Value::list(vec![
            Value::symbol("error"),
            Value::string(
                "Memory exhausted--use M-x save-some-buffers then exit and restart Emacs",
            ),
        ]),
    );
    obarray.make_special("memory-signal-data");
    obarray.set_symbol_value("memory-full", Value::NIL);
    obarray.make_special("memory-full");
    obarray.set_symbol_value("gc-elapsed", Value::make_float(0.0));
    obarray.make_special("gc-elapsed");
    obarray.set_symbol_value("gcs-done", Value::fixnum(0));
    obarray.make_special("gcs-done");
    obarray.set_symbol_value("pure-bytes-used", Value::fixnum(0));
    obarray.make_special("pure-bytes-used");
}

#[cfg(test)]
#[path = "alloc_test.rs"]
mod tests;

/// Register this module's subrs. GNU: `syms_of_alloc` in `src/alloc.c`.
/// Extracted verbatim from the former flat `builtins::init_builtins`.
pub(crate) fn syms_of_alloc(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.defsubr(
        "garbage-collect",
        crate::emacs_core::builtins::misc_eval::builtin_garbage_collect,
        0,
        Some(0),
    );
    ctx.defsubr(
        "make-marker",
        |_ctx, args| super::marker::builtin_make_marker(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "make-string",
        |_ctx, args| crate::emacs_core::builtins::strings::builtin_make_string(args),
        2,
        Some(3),
    );
    ctx.defsubr(
        "garbage-collect-heapsize",
        |_ctx, args| crate::emacs_core::builtins::stubs::builtin_garbage_collect_heapsize(args),
        0,
        None,
    );
    ctx.defsubr(
        "garbage-collect-maybe",
        crate::emacs_core::builtins::stubs::builtin_garbage_collect_maybe,
        1,
        Some(1),
    );
    ctx.defsubr(
        "malloc-info",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_malloc_info(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "malloc-trim",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_malloc_trim(args),
        0,
        Some(1),
    );
    ctx.defsubr(
        "make-byte-code",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_make_byte_code(args),
        4,
        None,
    );
    ctx.defsubr(
        "make-closure",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_make_closure(args),
        1,
        None,
    );
    ctx.defsubr(
        "make-finalizer",
        crate::emacs_core::builtins::symbols::builtin_make_finalizer,
        1,
        Some(1),
    );
    ctx.defsubr(
        "make-record",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_make_record(args),
        3,
        Some(3),
    );
    ctx.defsubr(
        "memory-info",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_memory_info(args),
        0,
        Some(0),
    );
    ctx.defsubr(
        "record",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_record(args),
        1,
        None,
    );

    // -- Cons / List --
    ctx.defsubr_2(
        "cons",
        crate::emacs_core::builtins::cons_list::builtin_cons_2,
        2,
    );
    ctx.defsubr_slice(
        "list",
        crate::emacs_core::builtins::cons_list::builtin_list_slice,
        0,
        None,
    );
    ctx.defsubr(
        "make-list",
        |_ctx, args| super::misc::builtin_make_list(args),
        2,
        Some(2),
    );

    // -- Vector --
    ctx.defsubr(
        "make-vector",
        |_ctx, args| crate::emacs_core::builtins::collections::builtin_make_vector(args),
        2,
        Some(2),
    );
    ctx.defsubr_slice(
        "vector",
        crate::emacs_core::builtins::collections::builtin_vector_slice,
        0,
        None,
    );
    ctx.defsubr_1(
        "make-symbol",
        crate::emacs_core::builtins::misc_pure::builtin_make_symbol_1,
        1,
    );
    ctx.defsubr(
        "bool-vector",
        |_ctx, args| super::chartable::builtin_bool_vector(args),
        0,
        None,
    );
    ctx.defsubr(
        "make-bool-vector",
        |_ctx, args| super::chartable::builtin_make_bool_vector(args),
        2,
        Some(2),
    );
}
