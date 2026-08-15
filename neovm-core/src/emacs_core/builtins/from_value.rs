//! Typed argument extraction for builtins.
//!
//! GNU C primitives hand-write `CHECK_*` macros per argument because C has
//! no other option; the Rust port accumulated the same shape as repeated
//! `expect_int`/`expect_lisp_string` match blocks. `FromValue` centralizes
//! that pattern: the Rust parameter TYPE names the Lisp contract, and the
//! extraction derives the `wrong-type-argument` predicate from the type.
//!
//! Extraction is evaluator-aware because GNU coerces marker designators
//! wherever `integer-or-marker-p` applies, and a marker's position is read
//! from its live buffer (`marker_position_as_int_eval`). Purely structural
//! extractions simply ignore the evaluator.
//!
//! `typed_subr!` generates the fixed-arity `SubrFn`-shaped wrapper
//! (`fn(&mut Context, Value, ...) -> EvalResult`) that extracts each
//! argument before the body runs, so a builtin body starts with its
//! arguments already typed:
//!
//! ```ignore
//! typed_subr! {
//!     /// Doc comment passes through to the generated fn.
//!     pub(crate) fn builtin_example(eval, s: String, n: Option<i64>) -> EvalResult {
//!         let n = n.unwrap_or(1);
//!         Ok(Value::string(s.repeat(n.max(0) as usize)))
//!     }
//! }
//! ```
//!
//! The generated fn keeps the exact `SubrFn::A{N}` signature, so
//! `defsubr_N` registration, the bytecode VM fast-path delegates, and the
//! JIT builtin table all consume it unchanged.

use crate::buffer::LispCharPos1;
use crate::emacs_core::error::expect_fixnum;
use crate::heap_types::LispString;

use super::*;

/// Extract a typed argument from a `Value`, signaling
/// `(wrong-type-argument PREDICATE value)` on mismatch. The implementing
/// type fixes PREDICATE, mirroring the GNU `CHECK_*` macro family.
pub(crate) trait FromValue: Sized {
    fn from_value(eval: &mut eval::Context, value: Value) -> Result<Self, Flow>;
}

/// Identity: accepts any value. Lets a typed signature keep raw `Value`
/// parameters for arguments with no single predicate.
impl FromValue for Value {
    fn from_value(_eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        Ok(value)
    }
}

/// `integerp` — fixnum-valued integer (mirrors `expect_int` / GNU
/// `CHECK_INTEGER` at fixnum call sites).
impl FromValue for i64 {
    fn from_value(_eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        expect_int(&value)
    }
}

/// `numberp` — fixnum, float, or bignum lowered to f64 (mirrors
/// `expect_number` / GNU `CHECK_NUMBER` + `XFLOATINT`).
impl FromValue for f64 {
    fn from_value(_eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        expect_number(&value)
    }
}

/// `stringp` — borrow the heap string payload. The borrow is only valid
/// while no GC runs, exactly like `expect_lisp_string`.
impl FromValue for &'static LispString {
    fn from_value(_eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        expect_lisp_string(&value)
    }
}

/// `stringp` — lossy UTF-8 decode for text-only processing (mirrors
/// `expect_string_lossy`; raw eight-bit bytes become U+FFFD).
impl FromValue for String {
    fn from_value(_eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        expect_string_lossy(&value)
    }
}

/// Lisp boolean: nil is false, anything else is true. Never signals.
impl FromValue for bool {
    fn from_value(_eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        Ok(!value.is_nil())
    }
}

/// `symbolp` — the symbol's identity (nil and keywords are symbols).
/// Honors `symbols-with-pos-enabled`: a symbol-with-pos unwraps to its
/// bare symbol exactly as GNU's `maybe_remove_pos_from_symbol` path does.
impl FromValue for SymId {
    fn from_value(eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        expect_symbol_id_checked(&value, eval.symbols_with_pos_enabled)
    }
}

/// Optional argument: nil maps to `None`. The arity dispatcher pads
/// omitted `&optional` arguments with nil, so `Option<T>` models GNU's
/// optional-argument convention directly.
impl<T: FromValue> FromValue for Option<T> {
    fn from_value(eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        if value.is_nil() {
            Ok(None)
        } else {
            T::from_value(eval, value).map(Some)
        }
    }
}

/// `number-or-marker-p` — marker positions are read from their live
/// buffer (mirrors `expect_number_or_marker_eval`).
impl FromValue for NumberOrMarker {
    fn from_value(eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        expect_number_or_marker_eval(eval, &value)
    }
}

/// `integer-or-marker-p` — a 1-based Lisp buffer position. Extraction
/// types the raw coordinate; range validation against the (possibly
/// narrowed) buffer stays with the caller, as in GNU
/// `CHECK_FIXNUM_COERCE_MARKER` + `validate_region`.
impl FromValue for LispCharPos1 {
    fn from_value(eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        expect_integer_or_marker_eval(eval, &value).map(LispCharPos1::new)
    }
}

/// `fixnump` — strictly a fixnum (bignums rejected), mirroring
/// `expect_fixnum` / GNU `CHECK_FIXNUM`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) struct Fixnum(pub(crate) i64);

impl FromValue for Fixnum {
    fn from_value(_eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        expect_fixnum(&value).map(Fixnum)
    }
}

/// `wholenump` — a non-negative fixnum, mirroring `expect_wholenump` /
/// GNU `CHECK_FIXNAT`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) struct Wholenum(pub(crate) i64);

impl FromValue for Wholenum {
    fn from_value(_eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        expect_wholenump(&value).map(Wholenum)
    }
}

/// `characterp` — a valid Emacs character code (0..=0x3FFFFF), mirroring
/// `expect_character_code` / GNU `CHECK_CHARACTER`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) struct CharacterCode(pub(crate) i64);

impl FromValue for CharacterCode {
    fn from_value(_eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        expect_character_code(&value).map(CharacterCode)
    }
}

/// `stringp` — a borrowed string designator: strings pass through, symbols
/// contribute their exact name object (mirrors `expect_string_comparison_operand`,
/// GNU `string-equal`/`string-lessp` operand coercion).  The reference makes
/// cloning an operand into the comparison hot path impossible by construction.
///
/// GNU's `SYMBOLP` also accepts a symbol-with-pos while
/// `symbols-with-pos-enabled` is non-nil.  Resolve that dynamic view here so
/// every typed string-designator builtin has the same interpreter, bytecode,
/// and JIT contract.
///
/// The inner reference is `&'static` only because that is how the tagged heap
/// hands out object interiors; it is NOT a claim that the string outlives the
/// designator.  The field is therefore private and the only way out is
/// [`StringDesignator::text`], whose result is reborrowed from `&self`, so a
/// caller cannot park a `&'static LispString` beyond the designator's scope.
/// (`Value::as_lisp_string` still launders `&'static` for the ~675 call sites
/// that predate this type; narrowing those is a separate, much larger job.)
#[derive(Clone, Copy, Debug)]
pub(crate) struct StringDesignator(&'static LispString);

impl StringDesignator {
    /// Borrow the designated string, for no longer than the designator lives.
    pub(crate) fn text(&self) -> &LispString {
        self.0
    }
}

impl FromValue for StringDesignator {
    fn from_value(eval: &mut eval::Context, value: Value) -> Result<Self, Flow> {
        let value = eval.unwrap_symbol(value);
        expect_string_comparison_operand(&value).map(StringDesignator)
    }
}

/// Define fixed-arity builtins with typed arguments.
///
/// Expands to a plain `fn(&mut Context, Value, ...) -> EvalResult` (the
/// `SubrFn::A{N}` shape): each argument is extracted via [`FromValue`]
/// before the body runs, signaling `wrong-type-argument` with the
/// predicate derived from the parameter type. Register the result with
/// `defsubr_N` exactly like a hand-written builtin; `min_args` still
/// controls arity, with omitted optionals arriving as nil (use
/// `Option<T>`).
macro_rules! typed_subr {
    ($($(#[$meta:meta])* $vis:vis fn $name:ident(
        $eval:ident $(, $arg:ident : $ty:ty)* $(,)?
    ) -> EvalResult $body:block)+) => {$(
        $(#[$meta])*
        $vis fn $name(
            $eval: &mut crate::emacs_core::eval::Context
            $(, $arg: crate::emacs_core::value::Value)*
        ) -> crate::emacs_core::error::EvalResult {
            $(
                let $arg = <$ty as crate::emacs_core::builtins::FromValue>::from_value(
                    $eval, $arg,
                )?;
            )*
            $body
        }
    )+};
}
pub(crate) use typed_subr;

#[cfg(test)]
#[path = "from_value_test.rs"]
mod tests;
