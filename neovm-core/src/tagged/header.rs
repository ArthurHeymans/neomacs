//! Core type definitions for the tagged pointer value system.
//!
//! # Object categories
//!
//! **Cons cells** — `ConsCell` with `(car, cdr)` = 16 bytes.
//!
//! **Vectorlike sub-types** — discriminated by `VecLikeType` enum stored
//! at offset 0 of each GcXxx payload (in `gc_trace_impls.rs`).
//!
//! **Builtin functions** — `SubrFn` variants for different arities.

use super::value::TaggedValue;

// ---------------------------------------------------------------------------
// ConsCell
// ---------------------------------------------------------------------------

/// A cons cell: two tagged values.
///
/// 16 bytes on 64-bit. With neovm-gc, cons cells are allocated as
/// `GcCons` (in `gc_trace_impls.rs`) which has the same memory layout
/// via `GcSlot` (repr(transparent) over `UnsafeCell<TaggedValue>`).
#[derive(Clone, Copy)]
#[repr(C)]
pub union ConsCdrOrNext {
    pub cdr: TaggedValue,
    pub next_free: *mut ConsCell,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ConsCell {
    pub car: TaggedValue,
    pub cdr_or_next: ConsCdrOrNext,
}

impl ConsCell {
    #[inline]
    pub unsafe fn cdr(&self) -> TaggedValue {
        unsafe { self.cdr_or_next.cdr }
    }

    #[inline]
    pub unsafe fn set_car(&mut self, value: TaggedValue) {
        self.car = value;
    }

    #[inline]
    pub unsafe fn set_cdr(&mut self, value: TaggedValue) {
        self.cdr_or_next.cdr = value;
    }
}

// ---------------------------------------------------------------------------
// VecLikeType — sub-type discriminant for vectorlike objects
// ---------------------------------------------------------------------------

/// Sub-type tag for vectorlike objects.
///
/// Stored at offset 0 of every vectorlike GcXxx payload struct
/// (in `gc_trace_impls.rs`). Read by `TaggedValue::kind()` to
/// determine the concrete type behind a `011`-tagged pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VecLikeType {
    Vector = 0,
    HashTable = 1,
    Lambda = 2,
    Macro = 3,
    ByteCode = 4,
    Record = 5,
    Overlay = 6,
    Marker = 7,
    Buffer = 8,
    Window = 9,
    Frame = 10,
    Timer = 11,
    /// Built-in function (like GNU's PVEC_SUBR).
    Subr = 12,
    /// Arbitrary-precision integer (like GNU's PVEC_BIGNUM).
    Bignum = 13,
    /// Symbol with source position (like GNU's PVEC_SYMBOL_WITH_POS).
    SymbolWithPos = 14,
}

/// Compatibility alias for call sites that still treat vectorlike payloads as
/// an opaque header-sized pointer target before tagging it as a `TaggedValue`.
pub type VecLikeHeader = u8;

/// Compatibility alias for marker payload pointers after the neovm-gc
/// migration. Marker storage now lives in `GcMarker`.
pub type MarkerObj = crate::tagged::gc_trace_impls::GcMarker;

// ---------------------------------------------------------------------------
// Closure slot indices (GNU Emacs compatible)
// ---------------------------------------------------------------------------

/// Closure slot indices matching GNU Emacs (lisp.h).
pub const CLOSURE_ARGLIST: usize = 0;
pub const CLOSURE_CODE: usize = 1;
pub const CLOSURE_CONSTANTS: usize = 2;
pub const CLOSURE_STACK_DEPTH: usize = 3;
pub const CLOSURE_DOC_STRING: usize = 4;
pub const CLOSURE_INTERACTIVE: usize = 5;
/// Minimum number of slots in a closure vector.
pub const CLOSURE_MIN_SLOTS: usize = 6;

// ---------------------------------------------------------------------------
// Subr function types
// ---------------------------------------------------------------------------

/// Heap-allocated built-in function (like GNU's PVEC_SUBR).
/// Contains a GNU-shaped fixed-arity or variadic entry point together with
/// arity metadata stored on the SubrObj itself.
pub type SubrFnMany = fn(
    &mut crate::emacs_core::eval::Context,
    Vec<super::value::TaggedValue>,
) -> crate::emacs_core::error::EvalResult;
pub type SubrFn0 =
    fn(&mut crate::emacs_core::eval::Context) -> crate::emacs_core::error::EvalResult;
pub type SubrFn1 = fn(
    &mut crate::emacs_core::eval::Context,
    super::value::TaggedValue,
) -> crate::emacs_core::error::EvalResult;
pub type SubrFn2 = fn(
    &mut crate::emacs_core::eval::Context,
    super::value::TaggedValue,
    super::value::TaggedValue,
) -> crate::emacs_core::error::EvalResult;

#[derive(Clone, Copy)]
pub enum SubrFn {
    Many(SubrFnMany),
    A0(SubrFn0),
    A1(SubrFn1),
    A2(SubrFn2),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SubrDispatchKind {
    Builtin,
    ContextCallable,
    SpecialForm,
}
