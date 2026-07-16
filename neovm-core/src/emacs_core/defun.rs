//! `defun!` — one definition point per ported GNU `DEFUN`.
//!
//! GNU's `DEFUN` macro colocates the lisp name, C function, arity, intspec,
//! and docstring. The neovm equivalent colocates what the Rust side owns —
//! lisp name, typed Rust signature, and arity — while docstrings and
//! interactive specs stay in the generated GNU-verbatim tables
//! (`subr_docs/gnu_table.rs`, `interactive.rs`): extraction from upstream is
//! the parity guarantee, hand-copying would reintroduce drift.
//!
//! Each invocation expands to:
//! - a **typed `pub(crate)` Rust fn** — internal Rust code calls it directly,
//!   the analogue of GNU C calling `Fzlib_decompress_region`;
//! - a `SubrDecl` const registered from the module's `syms_of_*` via
//!   `Context::defsubr_decl`.
//!
//! Omitted optional arguments arrive as `Value::NIL` (GNU's `Qnil`
//! convention; the dispatcher nil-pads typed `SubrFn::A1..A8` calls), and
//! arity is enforced by the dispatcher against the declared min/max before
//! the typed fn runs.
//!
//! ```ignore
//! defun!(ZLIB_DECOMPRESS_REGION: "zlib-decompress-region", min = 2,
//!     fn zlib_decompress_region(ctx: &mut Context, start: Value, end: Value,
//!                               allow_partial: Value) -> EvalResult {
//!         ...
//!     });
//!
//! pub(crate) fn syms_of_decompress(ctx: &mut Context) {
//!     ctx.defsubr_decl(&ZLIB_DECOMPRESS_REGION);
//! }
//! ```

use crate::tagged::header::SubrFn;

/// Everything `syms_of_*` needs to register one subr: the GNU-visible name,
/// the typed dispatch entry, and the declared arity. `defun!` is the
/// intended constructor.
pub(crate) struct SubrDecl {
    pub name: &'static str,
    pub func: SubrFn,
    pub min_args: u16,
    pub max_args: Option<u16>,
}

impl crate::emacs_core::eval::Context {
    /// Register a `defun!`-declared subr. Equivalent to the matching
    /// `defsubr_N` call; exists so `syms_of_*` lists read as declarations.
    pub(crate) fn defsubr_decl(&mut self, decl: &SubrDecl) {
        self.defsubr_with_entry(decl.name, decl.func, decl.min_args, decl.max_args);
    }
}

/// Colocate one subr's lisp name, arity, and typed body (see module docs).
///
/// Forms, by parameter shape after the context argument:
/// - `(ctx: &mut Context)`                          -> `SubrFn::A0`, arity (0,0)
/// - 1..=4 typed `Value` args, `min = N`            -> `SubrFn::A1..A4`,
///   max = arg count (declare optionals as trailing args; omitted ones are nil)
/// - `(ctx: &mut Context, args: Vec<Value>), min = N, max = M|None`
///                                                  -> `SubrFn::Many`
macro_rules! defun {
    // fixed arity 0
    ($decl:ident: $name:literal,
     fn $rust:ident($ctx:ident: &mut Context) -> EvalResult $body:block) => {
        pub(crate) fn $rust(
            $ctx: &mut crate::emacs_core::eval::Context,
        ) -> crate::emacs_core::error::EvalResult
        $body
        pub(crate) const $decl: crate::emacs_core::defun::SubrDecl =
            crate::emacs_core::defun::SubrDecl {
                name: $name,
                func: crate::tagged::header::SubrFn::A0($rust),
                min_args: 0,
                max_args: Some(0),
            };
    };
    // fixed arity 1..=4; `min` covers &optional trailing args
    ($decl:ident: $name:literal, min = $min:expr,
     fn $rust:ident($ctx:ident: &mut Context $(, $arg:ident: Value)+) -> EvalResult $body:block) => {
        pub(crate) fn $rust(
            $ctx: &mut crate::emacs_core::eval::Context
            $(, $arg: crate::emacs_core::value::Value)+
        ) -> crate::emacs_core::error::EvalResult
        $body
        pub(crate) const $decl: crate::emacs_core::defun::SubrDecl =
            crate::emacs_core::defun::SubrDecl {
                name: $name,
                func: defun!(@variant $rust $($arg)+),
                min_args: $min,
                max_args: Some(defun!(@count $($arg)+)),
            };
    };
    // variadic form; `max = None` for unbounded
    ($decl:ident: $name:literal, min = $min:expr, max = $max:expr,
     fn $rust:ident($ctx:ident: &mut Context, $args:ident: Vec<Value>) -> EvalResult $body:block) => {
        pub(crate) fn $rust(
            $ctx: &mut crate::emacs_core::eval::Context,
            $args: Vec<crate::emacs_core::value::Value>,
        ) -> crate::emacs_core::error::EvalResult
        $body
        pub(crate) const $decl: crate::emacs_core::defun::SubrDecl =
            crate::emacs_core::defun::SubrDecl {
                name: $name,
                func: crate::tagged::header::SubrFn::Many($rust),
                min_args: $min,
                max_args: $max,
            };
    };
    (@count $a:ident) => { 1u16 };
    (@count $a:ident $b:ident) => { 2u16 };
    (@count $a:ident $b:ident $c:ident) => { 3u16 };
    (@count $a:ident $b:ident $c:ident $d:ident) => { 4u16 };
    (@variant $rust:ident $a:ident) => { crate::tagged::header::SubrFn::A1($rust) };
    (@variant $rust:ident $a:ident $b:ident) => { crate::tagged::header::SubrFn::A2($rust) };
    (@variant $rust:ident $a:ident $b:ident $c:ident) => { crate::tagged::header::SubrFn::A3($rust) };
    (@variant $rust:ident $a:ident $b:ident $c:ident $d:ident) => { crate::tagged::header::SubrFn::A4($rust) };
}
pub(crate) use defun;
