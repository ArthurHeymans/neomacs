//! Bytecode virtual machine and decoder.
//!
//! Provides:
//! - `opcode::Op` — bytecode instruction set
//! - `chunk::ByteCodeFunction` — compiled function representation
//! - `vm::Vm` — stack-based bytecode interpreter
//! - `decode` — GNU .elc bytecode decoder

pub mod chunk;
pub mod decode;
pub mod opcode;
pub mod vm;

// Re-export main types
pub use chunk::ByteCodeFunction;
pub(crate) use chunk::fresh_bytecode_source_id;
pub use opcode::Op;
pub use vm::Vm;

/// Register this module's subrs. GNU: `syms_of_bytecode` in `src/bytecode.c`.
/// Extracted verbatim from the former flat `builtins::init_builtins`.
pub(crate) fn syms_of_bytecode(ctx: &mut crate::emacs_core::eval::Context) {
    ctx.defsubr(
        "internal-stack-stats",
        |_ctx, args| crate::emacs_core::builtins::symbols::builtin_internal_stack_stats(args),
        0,
        Some(0),
    );
}
