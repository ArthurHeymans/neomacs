//! Heap object headers and layouts for the tagged pointer GC.
//!
//! # Object categories
//!
//! **Cons cells** — no header, just `(car, cdr)` = 16 bytes.
//! GC uses an external mark bitmap in the cons block allocator.
//!
//! **Strings, Floats** — have a `GcHeader` for mark bit and sweep list.
//!
//! **Vectorlike objects** — have a `VecLikeHeader` (extends `GcHeader`)
//! with a `type_tag` field distinguishing vectors, hash tables, lambdas,
//! macros, bytecode, buffers, markers, overlays, records, etc.

use super::value::TaggedValue;
use malachite::integer::Integer;
use num_enum::{IntoPrimitive, TryFromPrimitive};

// ---------------------------------------------------------------------------
// ConsCell — no header, minimal size
// ---------------------------------------------------------------------------

/// A cons cell: two tagged values, no header.
///
/// 16 bytes on 64-bit. GC marks cons cells via an external bitmap
/// in the block allocator, not via an in-object flag.
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

    #[inline]
    pub unsafe fn free_next(&self) -> *mut ConsCell {
        unsafe { self.cdr_or_next.next_free }
    }

    #[inline]
    pub unsafe fn set_free_next(&mut self, next: *mut ConsCell) {
        self.car = TaggedValue::NIL;
        self.cdr_or_next.next_free = next;
    }
}

// ---------------------------------------------------------------------------
// GcHeader — shared header for all non-cons heap objects
// ---------------------------------------------------------------------------

/// GC header prepended to every non-cons heap object.
///
/// Provides mark bit for garbage collection and an intrusive linked list
/// pointer for sweep-phase traversal.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
pub enum HeapObjectKind {
    String = 0,
    Float = 1,
    VecLike = 2,
}

#[repr(C)]
pub struct GcHeader {
    /// Mark bit: set during mark phase, cleared before each GC cycle.
    pub marked: bool,
    /// Exact object category for typed sweep/deallocation.
    pub kind: HeapObjectKind,
    /// Intrusive linked list of all GC-managed objects (for sweep).
    pub next: *mut GcHeader,
}

impl GcHeader {
    pub fn new(kind: HeapObjectKind) -> Self {
        Self {
            marked: false,
            kind,
            next: std::ptr::null_mut(),
        }
    }
}

// ---------------------------------------------------------------------------
// Typed heap objects
// ---------------------------------------------------------------------------

/// Heap-allocated string object.
#[repr(C)]
pub struct StringObj {
    pub header: GcHeader,
    pub data: crate::heap_types::LispString,
}

/// Heap-allocated float object.
#[repr(C)]
pub struct FloatObj {
    pub header: GcHeader,
    pub value: f64,
}

// ---------------------------------------------------------------------------
// Vectorlike — catch-all for complex heap types
// ---------------------------------------------------------------------------

/// Sub-type tag for vectorlike objects.
/// Stored in the `VecLikeHeader`, distinguishes the many heap types
/// that share the GNU `Lisp_Vectorlike` pointer tag.
///
/// Discriminants mirror GNU's `enum pvec_type` for every runtime object that
/// has a GNU counterpart.  Neomacs-only transitional tags live after
/// `PVEC_FONT`; those are explicit compatibility debt, not GNU semantics.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive)]
pub enum VecLikeType {
    Vector = 0,
    /// Arbitrary-precision integer (GNU `PVEC_BIGNUM`).
    Bignum = 2,
    Marker = 3,
    Overlay = 4,
    /// Symbol with source position (GNU `PVEC_SYMBOL_WITH_POS`).
    SymbolWithPos = 6,
    /// User pointer for dynamic module API (GNU `PVEC_USER_PTR`).
    UserPtr = 8,
    Frame = 10,
    Window = 11,
    Buffer = 13,
    HashTable = 14,
    /// Obarray object (GNU `PVEC_OBARRAY`).
    Obarray = 15,
    /// Built-in function (like GNU's PVEC_SUBR).
    Subr = 18,
    /// Embedded widget model object (GNU `PVEC_XWIDGET`).
    Xwidget = 20,
    /// Embedded widget view object (GNU `PVEC_XWIDGET_VIEW`).
    XwidgetView = 21,
    /// Dynamic module function (GNU `PVEC_MODULE_FUNCTION`).
    ModuleFunction = 25,
    /// SQLite database or statement object (like GNU's PVEC_SQLITE).
    Sqlite = 30,
    /// Lisp closures are GNU `PVEC_CLOSURE`.
    Lambda = 31,
    /// Character table (like GNU's PVEC_CHAR_TABLE).
    CharTable = 32,
    /// Internal sub character table (like GNU's PVEC_SUB_CHAR_TABLE).
    SubCharTable = 33,
    Record = 34,
    Macro = 36,
    ByteCode = 37,
    Timer = 38,
}

impl VecLikeType {
    pub fn gnu_pvec_type(self) -> Option<GnuPvecType> {
        Some(match self {
            Self::Vector => GnuPvecType::NormalVector,
            Self::Bignum => GnuPvecType::Bignum,
            Self::Marker => GnuPvecType::Marker,
            Self::Overlay => GnuPvecType::Overlay,
            Self::SymbolWithPos => GnuPvecType::SymbolWithPos,
            Self::UserPtr => GnuPvecType::UserPtr,
            Self::Frame => GnuPvecType::Frame,
            Self::Window => GnuPvecType::Window,
            Self::Buffer => GnuPvecType::Buffer,
            Self::HashTable => GnuPvecType::HashTable,
            Self::Obarray => GnuPvecType::Obarray,
            Self::Subr => GnuPvecType::Subr,
            Self::Xwidget => GnuPvecType::Xwidget,
            Self::XwidgetView => GnuPvecType::XwidgetView,
            Self::ModuleFunction => GnuPvecType::ModuleFunction,
            Self::Sqlite => GnuPvecType::Sqlite,
            Self::Lambda => GnuPvecType::Closure,
            Self::CharTable => GnuPvecType::CharTable,
            Self::SubCharTable => GnuPvecType::SubCharTable,
            Self::Record => GnuPvecType::Record,
            Self::Macro | Self::ByteCode | Self::Timer => return None,
        })
    }

    pub fn gnu_pvec_code(self) -> Option<u8> {
        self.gnu_pvec_type().map(GnuPvecType::gnu_code)
    }
}

/// Complete GNU `enum pvec_type` domain (`src/lisp.h`). This records all
/// public GNU pseudovector tag codes even when Neomacs does not yet allocate
/// a corresponding runtime object.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive)]
pub enum GnuPvecType {
    NormalVector = 0,
    Free = 1,
    Bignum = 2,
    Marker = 3,
    Overlay = 4,
    Finalizer = 5,
    SymbolWithPos = 6,
    MiscPtr = 7,
    UserPtr = 8,
    Process = 9,
    Frame = 10,
    Window = 11,
    BoolVector = 12,
    Buffer = 13,
    HashTable = 14,
    Obarray = 15,
    Terminal = 16,
    WindowConfiguration = 17,
    Subr = 18,
    Other = 19,
    Xwidget = 20,
    XwidgetView = 21,
    Thread = 22,
    Mutex = 23,
    Condvar = 24,
    ModuleFunction = 25,
    NativeCompUnit = 26,
    TsParser = 27,
    TsNode = 28,
    TsCompiledQuery = 29,
    Sqlite = 30,
    Closure = 31,
    CharTable = 32,
    SubCharTable = 33,
    Record = 34,
    Font = 35,
}

impl GnuPvecType {
    pub fn from_gnu_code(code: u8) -> Option<Self> {
        Self::try_from(code).ok()
    }

    pub fn gnu_code(self) -> u8 {
        self.into()
    }
}

use std::sync::OnceLock;

/// Slot storage for vectorlike objects that can either be ordinary Rust-owned
/// storage or a borrowed slice in a mapped pdump image.
pub struct LispValueVec {
    storage: LispValueVecStorage,
}

#[repr(transparent)]
pub struct LispValueSlice([TaggedValue]);

impl LispValueSlice {
    pub fn from_slice(slice: &[TaggedValue]) -> &Self {
        unsafe { &*(slice as *const [TaggedValue] as *const Self) }
    }

    pub fn as_slice(&self) -> &[TaggedValue] {
        &self.0
    }

    pub fn to_vec(&self) -> Vec<TaggedValue> {
        self.0.to_vec()
    }

    pub fn clone(&self) -> Vec<TaggedValue> {
        self.to_vec()
    }
}

impl std::ops::Deref for LispValueSlice {
    type Target = [TaggedValue];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl std::fmt::Debug for LispValueSlice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl PartialEq<Vec<TaggedValue>> for LispValueSlice {
    fn eq(&self, other: &Vec<TaggedValue>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl PartialEq<LispValueSlice> for Vec<TaggedValue> {
    fn eq(&self, other: &LispValueSlice) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<'a> IntoIterator for &'a LispValueSlice {
    type Item = &'a TaggedValue;
    type IntoIter = std::slice::Iter<'a, TaggedValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a> IntoIterator for &'a &'a LispValueSlice {
    type Item = &'a TaggedValue;
    type IntoIter = std::slice::Iter<'a, TaggedValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

enum LispValueVecStorage {
    Owned(Vec<TaggedValue>),
    Mapped { ptr: *const TaggedValue, len: usize },
}

// Mapped slots are read-only through shared references.  Mutation paths use
// `ensure_owned` before exposing `&mut Vec<TaggedValue>`.
unsafe impl Send for LispValueVecStorage {}
unsafe impl Sync for LispValueVecStorage {}

impl LispValueVec {
    pub fn owned(items: Vec<TaggedValue>) -> Self {
        Self {
            storage: LispValueVecStorage::Owned(items),
        }
    }

    /// Build slot storage whose contents live in a mapped pdump image.
    ///
    /// # Safety
    /// `ptr..ptr+len` must remain mapped and immutable for the lifetime of the
    /// returned storage unless a mutation first copies the slots into owned
    /// storage.
    pub(crate) unsafe fn mapped(ptr: *const TaggedValue, len: usize) -> Self {
        Self {
            storage: LispValueVecStorage::Mapped { ptr, len },
        }
    }

    pub fn as_slice(&self) -> &[TaggedValue] {
        match self.storage {
            LispValueVecStorage::Owned(ref items) => items,
            LispValueVecStorage::Mapped { ptr, len } => {
                if len == 0 {
                    &[]
                } else {
                    unsafe { std::slice::from_raw_parts(ptr, len) }
                }
            }
        }
    }

    pub fn ensure_owned(&mut self) -> &mut Vec<TaggedValue> {
        if let LispValueVecStorage::Mapped { .. } = self.storage {
            let items = self.as_slice().to_vec();
            self.storage = LispValueVecStorage::Owned(items);
        }
        match self.storage {
            LispValueVecStorage::Owned(ref mut items) => items,
            LispValueVecStorage::Mapped { .. } => {
                unreachable!("mapped vector storage was copied to owned slots")
            }
        }
    }

    pub fn owned_capacity(&self) -> usize {
        match self.storage {
            LispValueVecStorage::Owned(ref items) => items.capacity(),
            LispValueVecStorage::Mapped { .. } => 0,
        }
    }
}

impl From<Vec<TaggedValue>> for LispValueVec {
    fn from(value: Vec<TaggedValue>) -> Self {
        Self::owned(value)
    }
}

impl Clone for LispValueVec {
    fn clone(&self) -> Self {
        Self::owned(self.as_slice().to_vec())
    }
}

impl std::ops::Deref for LispValueVec {
    type Target = [TaggedValue];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl std::ops::DerefMut for LispValueVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ensure_owned().as_mut_slice()
    }
}

impl<'a> IntoIterator for &'a LispValueVec {
    type Item = &'a TaggedValue;
    type IntoIter = std::slice::Iter<'a, TaggedValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

/// Header for all vectorlike heap objects.
///
/// Extends `GcHeader` with a type tag. The type-specific data follows
/// this header in memory (accessed via pointer cast to the concrete type).
#[repr(C)]
pub struct VecLikeHeader {
    pub gc: GcHeader,
    pub type_tag: VecLikeType,
}

impl VecLikeHeader {
    pub fn new(type_tag: VecLikeType) -> Self {
        Self {
            gc: GcHeader::new(HeapObjectKind::VecLike),
            type_tag,
        }
    }
}

// -- Concrete vectorlike types --

/// Heap-allocated vector (dynamic array of Values).
#[repr(C)]
pub struct VectorObj {
    pub header: VecLikeHeader,
    pub data: LispValueVec,
}

/// Number of slots in GNU's top-level char-table contents vector.
pub const CHAR_TABLE_TOP_SLOTS: usize = 64;

/// Heap-allocated character table.
///
/// Mirrors GNU Emacs's `struct Lisp_Char_Table`: default, parent, purpose,
/// ASCII cache, 64 top-level contents slots, then extra slots.
#[repr(C)]
pub struct CharTableObj {
    pub header: VecLikeHeader,
    pub defalt: TaggedValue,
    pub parent: TaggedValue,
    pub purpose: TaggedValue,
    pub ascii: TaggedValue,
    pub contents: [TaggedValue; CHAR_TABLE_TOP_SLOTS],
    pub extras: LispValueVec,
}

/// Heap-allocated sub character table.
///
/// Mirrors GNU Emacs's `struct Lisp_Sub_Char_Table`: depth, minimum
/// character, and a depth-dependent contents vector.
#[repr(C)]
pub struct SubCharTableObj {
    pub header: VecLikeHeader,
    pub depth: i32,
    pub min_char: i32,
    pub contents: LispValueVec,
}

/// Heap-allocated hash table.
#[repr(C)]
pub struct HashTableObj {
    pub header: VecLikeHeader,
    pub table: crate::emacs_core::value::LispHashTable,
}

/// Heap-allocated obarray.
///
/// Mirrors GNU Emacs's `struct Lisp_Obarray`: a vectorlike object with
/// bucket storage and a symbol count.  Legacy vector obarrays are still
/// accepted by `check_obarray` compatibility code.
#[repr(C)]
pub struct ObarrayObj {
    pub header: VecLikeHeader,
    pub buckets: LispValueVec,
    pub count: u32,
}

/// Heap-allocated lambda (interpreted closure).
///
/// Matches GNU Emacs's PVEC_CLOSURE: a plain vector of Lisp_Object slots.
/// The GC traces ALL slots uniformly — no type-specific tracing needed.
///
/// Slot layout (GNU Emacs compatible):
///   [0] CLOSURE_ARGLIST    — parameter list (e.g., (x y &optional z))
///   [1] CLOSURE_CODE       — body forms as Lisp list (interpreted) or bytecode
///   [2] CLOSURE_CONSTANTS  — lexical environment (interpreted) or constants vector
///   [3] CLOSURE_STACK_DEPTH — nil for interpreted, fixnum for bytecode
///   [4] CLOSURE_DOC_STRING — docstring or doc-form
///   [5] CLOSURE_INTERACTIVE — interactive spec
///   [6..] extra slots for oclosures
#[repr(C)]
pub struct LambdaObj {
    pub header: VecLikeHeader,
    /// All closure data as GC-managed Value slots.
    pub data: LispValueVec,
    /// Parsed lambda params cached from slot 0 for fast calls/arity checks.
    pub parsed_params: OnceLock<crate::emacs_core::value::LambdaParams>,
}

/// Closure slot indices matching GNU Emacs (lisp.h).
pub const CLOSURE_ARGLIST: usize = 0;
pub const CLOSURE_CODE: usize = 1;
pub const CLOSURE_CONSTANTS: usize = 2;
pub const CLOSURE_STACK_DEPTH: usize = 3;
pub const CLOSURE_DOC_STRING: usize = 4;
pub const CLOSURE_INTERACTIVE: usize = 5;
/// Minimum number of slots in a closure vector.
pub const CLOSURE_MIN_SLOTS: usize = 6;

/// Heap-allocated macro — same layout as Lambda but with VecLikeType::Macro.
#[repr(C)]
pub struct MacroObj {
    pub header: VecLikeHeader,
    pub data: LispValueVec,
    /// Parsed lambda params cached from slot 0 for fast calls/arity checks.
    pub parsed_params: OnceLock<crate::emacs_core::value::LambdaParams>,
}

/// Heap-allocated bytecode function.
#[repr(C)]
pub struct ByteCodeObj {
    pub header: VecLikeHeader,
    pub data: crate::emacs_core::bytecode::ByteCodeFunction,
}

/// Heap-allocated record (like vector with a type tag in slot 0).
#[repr(C)]
pub struct RecordObj {
    pub header: VecLikeHeader,
    pub data: LispValueVec,
}

/// Heap-allocated overlay.
#[repr(C)]
pub struct OverlayObj {
    pub header: VecLikeHeader,
    pub data: crate::heap_types::OverlayData,
}

/// Heap-allocated marker.
#[repr(C)]
pub struct MarkerObj {
    pub header: VecLikeHeader,
    pub data: crate::heap_types::LispMarker,
}

/// Heap-allocated buffer reference (wraps a BufferId).
#[repr(C)]
pub struct BufferObj {
    pub header: VecLikeHeader,
    pub id: crate::buffer::BufferId,
}

/// Heap-allocated window reference (wraps a u64 id).
#[repr(C)]
pub struct WindowObj {
    pub header: VecLikeHeader,
    pub id: u64,
}

/// Heap-allocated frame reference (wraps a u64 id).
#[repr(C)]
pub struct FrameObj {
    pub header: VecLikeHeader,
    pub id: u64,
}

/// Heap-allocated timer reference (wraps a u64 id).
#[repr(C)]
pub struct TimerObj {
    pub header: VecLikeHeader,
    pub id: u64,
}

/// Heap-allocated xwidget model object.
///
/// Mirrors GNU `struct xwidget`: Lisp-traced fields come first
/// (`plist`, `type`, `buffer`, `title`, `script_callbacks`), followed by
/// native geometry/lifetime fields.
#[repr(C)]
pub struct XwidgetObj {
    pub header: VecLikeHeader,
    pub plist: TaggedValue,
    pub type_: TaggedValue,
    pub buffer: TaggedValue,
    pub title: TaggedValue,
    pub script_callbacks: TaggedValue,
    pub height: i32,
    pub width: i32,
    pub xwidget_id: u32,
    /// GNU stores `kill_without_query`; query-on-exit returns nil when this is
    /// true and t otherwise.
    pub kill_without_query: bool,
}

/// Heap-allocated xwidget view object.
///
/// GNU's public view object keeps the model and window as Lisp references.
/// Native window-system payload is owned by frontend/backends, not by this VM
/// object.
#[repr(C)]
pub struct XwidgetViewObj {
    pub header: VecLikeHeader,
    pub model: TaggedValue,
    pub window: TaggedValue,
    pub x: i32,
    pub y: i32,
    pub clip_right: i32,
    pub clip_bottom: i32,
    pub clip_top: i32,
    pub clip_left: i32,
    pub redisplayed: bool,
    pub hidden: bool,
}

/// Heap-allocated built-in function (like GNU's PVEC_SUBR).
/// Contains a GNU-shaped fixed-arity or variadic entry point together with
/// arity metadata stored on the SubrObj itself.
pub type SubrFnMany = fn(
    &mut crate::emacs_core::eval::Context,
    Vec<super::value::TaggedValue>,
) -> crate::emacs_core::error::EvalResult;
pub type SubrFnManySlice = fn(
    &mut crate::emacs_core::eval::Context,
    &[super::value::TaggedValue],
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
pub type SubrFn3 = fn(
    &mut crate::emacs_core::eval::Context,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
) -> crate::emacs_core::error::EvalResult;
pub type SubrFn4 = fn(
    &mut crate::emacs_core::eval::Context,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
) -> crate::emacs_core::error::EvalResult;
pub type SubrFn5 = fn(
    &mut crate::emacs_core::eval::Context,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
) -> crate::emacs_core::error::EvalResult;
pub type SubrFn6 = fn(
    &mut crate::emacs_core::eval::Context,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
) -> crate::emacs_core::error::EvalResult;
pub type SubrFn7 = fn(
    &mut crate::emacs_core::eval::Context,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
) -> crate::emacs_core::error::EvalResult;
pub type SubrFn8 = fn(
    &mut crate::emacs_core::eval::Context,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
    super::value::TaggedValue,
) -> crate::emacs_core::error::EvalResult;

#[derive(Clone, Copy)]
pub enum SubrFn {
    Many(SubrFnMany),
    ManySlice(SubrFnManySlice),
    A0(SubrFn0),
    A1(SubrFn1),
    A2(SubrFn2),
    A3(SubrFn3),
    A4(SubrFn4),
    A5(SubrFn5),
    A6(SubrFn6),
    A7(SubrFn7),
    A8(SubrFn8),
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum SubrDispatchKind {
    Builtin,
    ContextCallable,
    SpecialForm,
}

#[repr(C)]
pub struct SubrObj {
    pub header: VecLikeHeader,
    /// The canonical symbol identity for this primitive function.
    pub sym_id: crate::emacs_core::intern::SymId,
    /// The runtime-local name atom for the subr's public name.
    pub name: crate::emacs_core::intern::NameId,
    /// Minimum number of arguments.
    pub min_args: u16,
    /// Maximum number of arguments (None = unlimited/&rest).
    pub max_args: Option<u16>,
    /// How the evaluator should dispatch this public subr surface.
    pub dispatch_kind: SubrDispatchKind,
    /// Native Rust entry point for the builtin, if fully registered.
    pub function: Option<SubrFn>,
}

/// Heap-allocated arbitrary-precision integer (mirrors GNU
/// `struct Lisp_Bignum` in `src/bignum.h`).
///
/// GNU stores an `mpz_t` directly inside the struct. NeoMacs wraps
/// `malachite::Integer`, a pure-Rust bignum derived from GMP/FLINT
/// algorithms. The GC has no Lisp_Object children to trace — the only
/// owned resource is the `Integer`'s internal limb buffer, which is
/// freed when `Drop` runs in `free_gc_object`.
#[repr(C)]
pub struct BignumObj {
    pub header: VecLikeHeader,
    pub value: Integer,
}

/// A symbol annotated with its source byte offset.
/// Mirrors GNU `struct Lisp_Symbol_With_Pos` (`lisp.h:958`).
/// Both fields are `TaggedValue` (GC-traced), matching GNU's LISPSIZE=2.
#[repr(C)]
pub struct SymbolWithPosObj {
    pub header: VecLikeHeader,
    /// The bare symbol. Must always be a plain symbol (TAG_SYMBOL).
    pub sym: TaggedValue,
    /// Source byte offset. Must always be a fixnum.
    pub pos: TaggedValue,
}

/// Heap-allocated SQLite database or statement object.
///
/// The native SQLite resources are owned by the sqlite module's runtime maps;
/// this object is the opaque Lisp identity and carries the handle key plus the
/// database/statement discriminator, mirroring GNU's single PVEC_SQLITE tag.
#[repr(C)]
pub struct SqliteObj {
    pub header: VecLikeHeader,
    pub is_statement: bool,
    pub id: i64,
}

/// Heap-allocated user pointer for dynamic module API.
///
/// Mirrors GNU `struct Lisp_User_Ptr` (`emacs-module.c`).
/// Carries a raw C `void *` pointer plus an optional finalizer.
/// The GC never traces the raw pointer — it calls the finalizer on sweep.
///
/// The finalizer function pointer signature follows GNU Emacs:
/// `void (*fin)(void *ptr)`.
pub type EmacsFinalizer = Option<unsafe extern "C" fn(*mut std::ffi::c_void)>;

#[repr(C)]
pub struct UserPtrObj {
    pub header: VecLikeHeader,
    /// The raw C pointer owned by the module.
    pub ptr: *mut std::ffi::c_void,
    /// Optional finalizer invoked when the user-ptr is garbage-collected.
    pub finalizer: EmacsFinalizer,
}

/// Heap-allocated module function for dynamic module API.
///
/// Mirrors GNU `struct Lisp_Module_Function` (`emacs-module.c`).
/// Stores the C function pointer, closure data, optional finalizer,
/// arity metadata, and Lisp-visible doc/interactive slots.
#[repr(C)]
pub struct ModuleFunctionObj {
    pub header: VecLikeHeader,
    /// Minimum number of required arguments.
    pub min_arity: isize,
    /// Maximum number of arguments (-2 = GNU `emacs_variadic_function`).
    pub max_arity: isize,
    /// The raw C function pointer (emacs_function from emacs-module.h).
    ///
    /// Signature: `emacs_value (*)(emacs_env *env, ptrdiff_t nargs,
    ///                              emacs_value *args, void *data)`.
    pub subr: *const std::ffi::c_void,
    /// User-supplied closure data pointer.
    pub data: *mut std::ffi::c_void,
    /// Optional finalizer invoked when the module-function is GC'd.
    pub finalizer: EmacsFinalizer,
    /// Docstring (Lisp string value).
    pub documentation: TaggedValue,
    /// Interactive form (Lisp value).
    pub interactive_form: TaggedValue,
}

#[cfg(test)]
mod tests {
    use super::{LispValueSlice, LispValueVec};
    use crate::tagged::value::TaggedValue;

    #[test]
    fn mapped_lisp_value_vec_borrows_until_mutation() {
        let slots = vec![TaggedValue::fixnum(1), TaggedValue::fixnum(2)];
        let mut values = unsafe { LispValueVec::mapped(slots.as_ptr(), slots.len()) };

        assert_eq!(values.as_slice(), slots.as_slice());
        values.ensure_owned().push(TaggedValue::fixnum(3));

        drop(slots);
        assert_eq!(
            values.as_slice(),
            &[
                TaggedValue::fixnum(1),
                TaggedValue::fixnum(2),
                TaggedValue::fixnum(3)
            ]
        );
    }

    #[test]
    fn lisp_value_slice_clone_returns_owned_vec_for_compat_callers() {
        let slots = vec![TaggedValue::fixnum(1), TaggedValue::fixnum(2)];
        let slice = LispValueSlice::from_slice(&slots);

        let owned = slice.clone();
        drop(slots);
        assert_eq!(owned, vec![TaggedValue::fixnum(1), TaggedValue::fixnum(2)]);
    }
}
