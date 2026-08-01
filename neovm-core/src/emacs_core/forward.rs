//! Forwarder descriptors for `SYMBOL_FORWARDED` symbols.
//!
//! Mirrors GNU Emacs's `Lisp_Fwd` family in `src/lisp.h:3060-3145`. A
//! forwarded symbol stores a pointer to a static [`LispFwd`] descriptor;
//! reads and writes go through the descriptor instead of touching the
//! symbol's value cell directly. This is how variables like
//! `buffer-file-name`, `point`, `mark-active`, and `case-fold-search`
//! get their backing storage in dedicated C-side slots.
//!
//! # Implementation status
//!
//! `BufferObj` (per-buffer slots, GNU's `Lisp_Buffer_Objfwd`) and `Bool`
//! (GNU's `Lisp_Boolfwd`) are wired into the matcher. The remaining three
//! GNU variants — `Int`, `Obj`, `KboardObj` — have descriptor types declared
//! here and a stub fallback in
//! [`crate::emacs_core::symbol::Obarray::find_symbol_value`].
//!
//! Buffer-local audit HIGH 3 in
//! `drafts/buffer-local-variables-audit.md` flags the missing arms.
//! Neomacs still backs most `DEFVAR_INT` / `DEFVAR_LISP` variables as a
//! `Plainval` symbol stored in the obarray rather than
//! as a static C global. The Rust subsystems that GNU would consult
//! via `Vgc_cons_threshold` / `Vinhibit_quit` / etc. all read the
//! obarray directly through `eval.obarray.symbol_value(...)`, so
//! there is no "C side reads stale static, Lisp side wrote to a
//! different storage" desync to fix.
//!
//! Wiring the descriptors up to a real `Int`/`Bool`/`Obj`/`KboardObj`
//! dispatch in `find_symbol_value` would be a code-organization
//! improvement (and prerequisite for porting GNU's `variable-binding-locus`
//! reading the kboard slot). It is tracked as audit HIGH 3 + Phase 8b
//! of `drafts/symbol-redirect-plan.md`. The descriptor types below
//! are kept ready for that work.

use super::value::Value;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::sync::atomic::{AtomicBool, Ordering};

/// Discriminant for [`LispFwd`]. Mirrors GNU `enum Lisp_Fwd_Type`
/// (`src/lisp.h:3046-3055`). Always read the first field of any `*Fwd`
/// struct to determine its concrete type — exactly the GNU trick.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum LispFwdType {
    /// `Lisp_Intfwd`: forward to a static `intmax_t`.
    Int = 0,
    /// `Lisp_Boolfwd`: forward to a static `bool`.
    Bool = 1,
    /// `Lisp_Objfwd`: forward to a static `Lisp_Object` (a top-level
    /// global variable).
    Obj = 2,
    /// `Lisp_Buffer_Objfwd`: forward to a slot inside the current
    /// buffer's per-buffer storage.
    BufferObj = 3,
    /// `Lisp_Kboard_Objfwd`: forward to a slot inside the current
    /// keyboard's per-kboard storage.
    KboardObj = 4,
}

impl LispFwdType {
    pub fn from_gnu_code(code: u8) -> Option<Self> {
        Self::try_from(code).ok()
    }

    pub fn gnu_code(self) -> u8 {
        self.into()
    }
}

/// Common header. Every `Lisp_*Fwd` struct begins with this so the
/// dispatch code can read the discriminant from a `*const LispFwd`
/// without knowing the concrete type. Mirrors GNU `lispfwd` (`lisp.h:760`)
/// + the `type` field on each `Lisp_*fwd` body (`lisp.h:3060-3094`).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct LispFwd {
    pub ty: LispFwdType,
    // The body fields differ per variant. Code that has a `*const LispFwd`
    // matches on `ty` and re-casts to the concrete `Lisp*Fwd` pointer.
}

/// `Lisp_Intfwd`: forward to a static integer. Phase 8 wires this up.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct LispIntFwd {
    pub ty: LispFwdType,
    pub get: fn() -> i64,
    pub set: fn(i64),
}

/// `Lisp_Boolfwd`: forward to a native Boolean cell.
///
/// GNU stores a pointer to a C `bool`.  Each Neomacs context owns an
/// independently leaked descriptor instead, avoiding process-global state
/// between evaluators while retaining the same forwarded-value semantics.
#[repr(C)]
pub struct LispBoolFwd {
    pub ty: LispFwdType,
    value: AtomicBool,
}

impl LispBoolFwd {
    #[inline]
    pub fn get(&self) -> bool {
        self.value.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set(&self, value: bool) {
        self.value.store(value, Ordering::Relaxed);
    }
}

/// `Lisp_Objfwd`: forward to a static `Value` global. Phase 8 wires this up.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct LispObjFwd {
    pub ty: LispFwdType,
    pub get: fn() -> Value,
    pub set: fn(Value),
}

/// `Lisp_Buffer_Objfwd`: forward to a per-buffer slot. The `offset`
/// field indexes into `Buffer::slots: [Value; BUFFER_SLOT_COUNT]`,
/// playing the same role as GNU's `Lisp_Buffer_Objfwd::offset` (a byte
/// offset into `struct buffer`). Phase 8 wires this up.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct LispBufferObjFwd {
    pub ty: LispFwdType,
    /// Index into `Buffer::slots`. Mirrors GNU `Lisp_Buffer_Objfwd::offset`.
    pub offset: u16,
    /// Index into `buffer_local_flags` for "is this buffer-local in the
    /// current buffer?" tests. -1 means "always local everywhere",
    /// matching GNU's `PER_BUFFER_IDX(idx) == -1`.
    pub local_flags_idx: i16,
    /// Closed write predicate checked for live-slot writes. Mirrors GNU
    /// `enum Lisp_Fwd_Predicate` instead of encoding this finite domain as an
    /// open-ended Lisp symbol.
    pub predicate: crate::buffer::buffer::BufferSlotPredicate,
    /// Default value copied into `Buffer::slots[offset]` at buffer
    /// creation. Mirrors GNU `buffer_defaults`.
    pub default: Value,
}

/// `Lisp_Kboard_Objfwd`: forward to a per-keyboard slot. Phase 8 stubs
/// this with a single global `KBoard`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct LispKboardObjFwd {
    pub ty: LispFwdType,
    pub offset: u16,
}

// ===========================================================================
// Phase 8a — BUFFER_OBJFWD allocation and registration
// ===========================================================================

/// Leak a fresh [`LispBufferObjFwd`] descriptor into a `'static`
/// pointer. Mirrors GNU's `defvar_per_buffer` (`buffer.c:4990-5012`):
/// every per-buffer forwarder is allocated once at process init and
/// lives until exit. NeoMacs uses `Box::leak` instead of static
/// initialization because the per-process forwarders are constructed
/// from runtime data (slot index assignments).
///
/// `offset` is the index into [`crate::buffer::buffer::Buffer::slots`].
/// `local_flags_idx` mirrors GNU's `local-flags` index: `-1` means
/// "always-local in every buffer" (e.g. `buffer-file-name`,
/// `point`); a positive index points at a bit in
/// `Buffer::local_flags` (currently unused — Phase 8b will wire it).
/// `predicate` is the closed predicate used by `store_symval_forwarding`.
/// `default` is the value copied into every fresh buffer's slot.
pub fn alloc_buffer_objfwd(
    offset: u16,
    local_flags_idx: i16,
    predicate: crate::buffer::buffer::BufferSlotPredicate,
    default: Value,
) -> &'static LispBufferObjFwd {
    let fwd = Box::new(LispBufferObjFwd {
        ty: LispFwdType::BufferObj,
        offset,
        local_flags_idx,
        predicate,
        default,
    });
    Box::leak(fwd)
}

/// Allocate a process-lifetime native Boolean forwarder.
///
/// GNU's `DEFVAR_BOOL` descriptors are static C objects.  Neomacs constructs
/// contexts dynamically, so leaking one tiny descriptor per registered
/// variable and context provides the same stable-pointer contract without
/// coupling otherwise independent evaluators through a global Boolean.
pub fn alloc_boolfwd(initial: bool) -> &'static LispBoolFwd {
    Box::leak(Box::new(LispBoolFwd {
        ty: LispFwdType::Bool,
        value: AtomicBool::new(initial),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lisp_fwd_type_codes_match_gnu_lisp_fwd_type() {
        let cases = [
            (LispFwdType::Int, 0),
            (LispFwdType::Bool, 1),
            (LispFwdType::Obj, 2),
            (LispFwdType::BufferObj, 3),
            (LispFwdType::KboardObj, 4),
        ];

        for (ty, code) in cases {
            assert_eq!(ty.gnu_code(), code);
            assert_eq!(LispFwdType::from_gnu_code(code), Some(ty));
        }
        assert_eq!(LispFwdType::from_gnu_code(5), None);
    }
}
