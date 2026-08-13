//! Process-global symbol registry backed by a separate string atom table.
//!
//! `SymId` is Lisp symbol identity and must stay stable across evaluator
//! creation/destruction so values can be formatted, compared, and moved
//! between contexts without keeping an old `Context` alive just for name
//! resolution. The runtime therefore uses a single append-only process symbol
//! registry.
//!
//! Name atoms are tracked separately via [`NameId`]. This mirrors GNU's model
//! more closely: a symbol is an object with a name, not just "slot N in the
//! string interner".

use hashbrown::HashMap;
use parking_lot::RwLock;
use rustc_hash::{FxBuildHasher, FxHashMap};
use std::borrow::Cow;
use std::cell::RefCell;
use std::hash::{BuildHasher, Hash, Hasher};
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::heap_types::LispString;
use crate::tagged::value::TaggedValue;

/// A compact handle to a Lisp symbol object. Copy, 4 bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct SymId(pub(crate) u32);

impl std::fmt::Debug for SymId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Resolve the symbol name so logs read `SymId(696 peculiar-error)`
        // instead of a bare id — otherwise a signal in a bug report is
        // undiagnosable. NEVER block: `Debug` can fire while this or another
        // thread holds the registry *write* lock (mid-intern), so a blocking
        // read could deadlock. `try_read` degrades to the id on contention.
        match global_symbol_registry().try_read() {
            Some(registry) => match registry.slot(*self) {
                Some(slot) => write!(f, "SymId({} {})", self.0, registry.names.resolve(slot.name)),
                None => write!(f, "SymId({})", self.0),
            },
            None => write!(f, "SymId({})", self.0),
        }
    }
}

/// A compact handle to a deduplicated symbol-name atom. Runtime-local only.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct NameId(pub(crate) u32);

pub const NIL_SYM_ID: SymId = SymId(0);
pub const T_SYM_ID: SymId = SymId(1);
pub const UNBOUND_SYM_ID: SymId = SymId(2);

/// Number of symbol-name atoms stored in each non-moving allocation.
const NAME_ATOM_CHUNK: usize = 4096;

/// Append-only, process-lifetime storage for interned symbol names.
///
/// The symbol registry exposes name atoms as `&'static LispString`, so the
/// backing allocations are intentionally leaked just as the former per-name
/// `Box::leak` allocations were. Chunking removes one allocator allocation and
/// one `Vec` pointer per distinct name while preserving stable addresses.
struct NameAtomStorage {
    chunks: Vec<NonNull<LispString>>,
    len: usize,
}

// SAFETY: chunks are appended and initialized only through `&mut self` while
// the enclosing `StringInterner` is write-locked. Published `LispString`s are
// immutable and their leaked backing allocations never move or disappear.
unsafe impl Send for NameAtomStorage {}
unsafe impl Sync for NameAtomStorage {}

impl NameAtomStorage {
    fn new() -> Self {
        Self {
            chunks: Vec::new(),
            len: 0,
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    fn reserve(&mut self, additional: usize) {
        let required_chunks = self
            .len
            .saturating_add(additional)
            .div_ceil(NAME_ATOM_CHUNK);
        self.chunks
            .reserve(required_chunks.saturating_sub(self.chunks.len()));
    }

    fn push(&mut self, value: LispString) -> &'static LispString {
        let chunk_index = self.len / NAME_ATOM_CHUNK;
        let slot_index = self.len % NAME_ATOM_CHUNK;
        if slot_index == 0 {
            let chunk = Box::<[LispString]>::new_uninit_slice(NAME_ATOM_CHUNK);
            let raw = Box::into_raw(chunk) as *mut MaybeUninit<LispString>;
            self.chunks.push(
                NonNull::new(raw.cast::<LispString>())
                    .expect("a non-empty Box allocation must not be null"),
            );
        }

        // SAFETY: `chunk_index` exists after the allocation above and
        // `slot_index` selects the next, as-yet-uninitialized slot. No slot is
        // ever written twice. The allocation is intentionally leaked and the
        // initialized value is never mutated, so the returned reference stays
        // valid for the remainder of the process.
        let slot = unsafe { self.chunks[chunk_index].as_ptr().add(slot_index) };
        unsafe { slot.write(value) };
        self.len += 1;
        unsafe { &*slot }
    }

    #[inline]
    fn get(&self, id: NameId) -> &'static LispString {
        let index = id.0 as usize;
        assert!(index < self.len, "invalid symbol name id {id:?}");
        let chunk_index = index / NAME_ATOM_CHUNK;
        let slot_index = index % NAME_ATOM_CHUNK;
        // SAFETY: the bounds check above proves this slot was initialized.
        // Chunks are leaked and initialized values never move or mutate.
        unsafe { &*self.chunks[chunk_index].as_ptr().add(slot_index) }
    }

    fn iter(&self) -> impl Iterator<Item = &'static LispString> + '_ {
        (0..self.len).map(|index| self.get(NameId(index as u32)))
    }
}

/// Append-only string interner used only for symbol names.
pub struct StringInterner {
    strings: NameAtomStorage,
    map: HashMap<&'static LispString, NameId, FxBuildHasher>,
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Fold a symbol name to the representation symbol IDENTITY is decided on.
///
/// GNU's `oblookup` compares a name's char count, byte count and bytes, never
/// its multibyte FLAG (lread.c), so an ascii-only multibyte spelling and its
/// unibyte spelling name the same symbol. Folding ascii-only multibyte names to
/// unibyte reproduces that on our flag-sensitive `LispString` comparison.
///
/// This is about identity ONLY. Which string object `symbol-name` returns is a
/// separate question, answered by the name object a symbol was created from --
/// GNU keeps that unfolded, multibyte flag and all.
pub(crate) fn normalize_symbol_name_lisp_string(s: &LispString) -> Cow<'_, LispString> {
    if s.is_ascii() && s.is_multibyte() {
        Cow::Owned(LispString::from_unibyte(s.as_bytes().to_vec()))
    } else {
        Cow::Borrowed(s)
    }
}

impl StringInterner {
    fn normalize_symbol_name_lisp_string<'a>(s: &'a LispString) -> Cow<'a, LispString> {
        normalize_symbol_name_lisp_string(s)
    }

    pub fn new() -> Self {
        Self {
            strings: NameAtomStorage::new(),
            map: HashMap::with_hasher(FxBuildHasher),
        }
    }

    fn reserve_additional_names(&mut self, additional: usize) {
        self.strings.reserve(additional);
        self.map.reserve(additional);
    }

    #[inline]
    fn hash_name_parts(&self, bytes: &[u8], multibyte: bool) -> u64 {
        // Keep this exactly aligned with `LispString::hash` so a borrowed
        // byte/representation query lands in the canonical map's bucket.
        let mut hasher = self.map.hasher().build_hasher();
        bytes.hash(&mut hasher);
        multibyte.hash(&mut hasher);
        hasher.finish()
    }

    #[inline]
    fn lookup_name_parts(&self, bytes: &[u8], multibyte: bool) -> Option<NameId> {
        let hash = self.hash_name_parts(bytes, multibyte);
        self.map
            .raw_entry()
            .from_hash(hash, |candidate| {
                candidate.is_multibyte() == multibyte && candidate.as_bytes() == bytes
            })
            .map(|(_, id)| *id)
    }

    fn name_atom_from_str(s: &str) -> LispString {
        if s.is_ascii() {
            LispString::from_unibyte(s.as_bytes().to_vec())
        } else {
            LispString::from_utf8(s)
        }
    }

    /// Intern a symbol-name atom, returning its unique id.
    pub fn intern(&mut self, s: &str) -> NameId {
        let multibyte = !s.is_ascii();
        if let Some(idx) = self.lookup_name_parts(s.as_bytes(), multibyte) {
            return idx;
        }
        let atom = Self::name_atom_from_str(s);
        self.intern_lisp_string(&atom)
    }

    /// Intern a symbol-name atom from an exact Lisp string representation.
    pub fn intern_lisp_string(&mut self, s: &LispString) -> NameId {
        let normalized = Self::normalize_symbol_name_lisp_string(s);
        if let Some(idx) = self.lookup_name_parts(normalized.as_bytes(), normalized.is_multibyte())
        {
            return idx;
        }
        let idx = NameId(self.strings.len() as u32);
        // `NameId(u32::MAX)` is reserved as the obarray empty-slot presence
        // sentinel (`symbol::SYMBOL_NAME_SENTINEL`): a `LispSymbol` slot is
        // "empty" iff its atomic `name` equals it. NameIds mint densely from 0,
        // so reaching `u32::MAX` (4.3B distinct symbol names) means the sentinel
        // would alias a real name and a live slot could read as empty.
        debug_assert_ne!(
            idx,
            crate::emacs_core::symbol::SYMBOL_NAME_SENTINEL,
            "NameId space exhausted: a real symbol name collided with the \
             obarray empty-slot presence sentinel (u32::MAX)",
        );
        let interned = self.strings.push(normalized.into_owned());
        self.map.insert(interned, idx);
        idx
    }

    /// Look up a symbol-name atom without interning it.
    pub fn lookup(&self, s: &str) -> Option<NameId> {
        self.lookup_name_parts(s.as_bytes(), !s.is_ascii())
    }

    /// Look up a symbol-name atom without interning it.
    pub fn lookup_lisp_string(&self, s: &LispString) -> Option<NameId> {
        let normalized = Self::normalize_symbol_name_lisp_string(s);
        self.lookup_name_parts(normalized.as_bytes(), normalized.is_multibyte())
    }

    /// Resolve a name id back to its string. Panics if id is invalid.
    #[inline]
    pub fn resolve(&self, id: NameId) -> &'static str {
        self.resolve_lisp_string(id)
            .as_utf8_str()
            .unwrap_or_else(|| panic!("symbol name {:?} is not valid UTF-8", id))
    }

    /// Resolve a name id back to its exact Lisp-string storage.
    #[inline]
    pub fn resolve_lisp_string(&self, id: NameId) -> &'static LispString {
        self.strings.get(id)
    }
}

/// Identity of the tagged heap that owns a Lisp-visible symbol name object.
///
/// Keeping this distinct from object identity makes it impossible to index the
/// per-heap root table with a raw object address (or vice versa).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
struct SymbolNameHeapId(usize);

/// Pointer identity of one Lisp-visible symbol name object.
///
/// `TaggedValue`'s `Eq`/`Hash` are structural, while GC roots are identities:
/// two equal strings must remain separate roots and one shared string must be
/// seeded only once.  This key makes that distinction explicit in the type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
struct SymbolNameObjectId(usize);

impl SymbolNameObjectId {
    fn of(value: TaggedValue) -> Self {
        Self(value.bits())
    }
}

#[derive(Clone, Copy, Debug)]
struct SymbolNameValue {
    value: TaggedValue,
    heap_id: SymbolNameHeapId,
}

/// The unique name objects that must be seeded for each live tagged heap.
///
/// Many uninterned symbols may deliberately share one exact name string (GNU
/// `make-symbol` preserves the argument object).  Root cardinality therefore
/// follows name-object identity, not symbol cardinality.
#[derive(Debug, Default)]
struct SymbolNameRootIndex {
    by_heap: FxHashMap<SymbolNameHeapId, FxHashMap<SymbolNameObjectId, TaggedValue>>,
}

impl SymbolNameRootIndex {
    fn insert(&mut self, name: SymbolNameValue) {
        let object_id = SymbolNameObjectId::of(name.value);
        let old = self
            .by_heap
            .entry(name.heap_id)
            .or_default()
            .insert(object_id, name.value);
        debug_assert!(
            old.is_none_or(|old| old.bits() == name.value.bits()),
            "one symbol-name object identity mapped to different values"
        );
    }

    fn extend_roots(&self, roots: &mut Vec<TaggedValue>, heap_id: SymbolNameHeapId) {
        if let Some(by_object) = self.by_heap.get(&heap_id) {
            roots.extend(by_object.values().copied());
        }
    }

    #[cfg(all(test, debug_assertions))]
    fn root_count(&self, heap_id: SymbolNameHeapId) -> usize {
        self.by_heap.get(&heap_id).map_or(0, FxHashMap::len)
    }
}

/// The name a freshly allocated symbol carries, stated at every construction
/// site so "no Lisp name object" is a claim rather than a forgotten argument.
///
/// GNU has only the second case: `intern_driver` and `Fmake_symbol` both store
/// THE STRING OBJECT they were handed (lread.c:4705-4708), so `symbol-name`
/// returns it with its text properties, its multibyteness and any later
/// mutation intact. We additionally construct symbols from Rust text -- the
/// reader, the dumper, bootstrap -- where no Lisp object exists to store; those
/// sites say `AtomOnly` and `symbol-name` materializes a string from the name
/// atom instead.
#[derive(Clone, Copy, Debug)]
enum NewSymbolName {
    /// No Lisp string object was involved; the interned name atom is the whole
    /// of the symbol's name.
    AtomOnly,
    /// GNU's case: this string object IS the symbol's name.
    LispObject(SymbolNameValue),
}

impl NewSymbolName {
    /// Adopt a Lisp string object as a new symbol's name, as GNU's
    /// `Fmake_symbol (string)` does.
    fn from_lisp_object(value: TaggedValue) -> Self {
        let heap_id = crate::tagged::gc::current_tagged_heap_identity()
            .expect("a Lisp symbol name value requires an installed tagged heap");
        Self::LispObject(SymbolNameValue {
            value,
            heap_id: SymbolNameHeapId(heap_id),
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct SymbolSlot {
    name: NameId,
    canonical: bool,
}

pub(crate) struct DumpedSymbolTable {
    pub names: Vec<LispString>,
    pub symbol_names: Vec<u32>,
    pub canonical: Vec<bool>,
}

#[derive(Debug)]
pub(crate) struct RestoredDumpSymbolTable {
    pub names: Vec<NameId>,
    pub symbols: Vec<SymId>,
}

/// Process-global append-only registry of Lisp symbols.
struct SymbolRegistry {
    names: StringInterner,
    symbols: Vec<SymbolSlot>,
    canonical_by_name: FxHashMap<NameId, SymId>,
    /// Exact heap string objects used as symbol names when Lisp supplies one
    /// directly. GNU stores that object in the symbol, so `(symbol-name
    /// (make-symbol NAME))` is `eq` to NAME and sees later string mutation.
    /// Keeping this rare case out of `SymbolSlot` makes every ordinary symbol
    /// substantially smaller.
    name_values: FxHashMap<SymId, SymbolNameValue>,
    /// Per-heap set of exact Lisp name objects. This is deliberately indexed
    /// by object identity rather than symbol id: many uninterned symbols can
    /// share one name object, and seeding it once is sufficient.
    name_value_roots: SymbolNameRootIndex,
}

impl Default for SymbolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolRegistry {
    fn new() -> Self {
        let mut registry = Self {
            names: StringInterner::new(),
            symbols: Vec::new(),
            canonical_by_name: FxHashMap::default(),
            name_values: FxHashMap::default(),
            name_value_roots: SymbolNameRootIndex::default(),
        };
        let nil_name = registry.names.intern("nil");
        let nil_id = registry.alloc_symbol(nil_name, true, NewSymbolName::AtomOnly);
        debug_assert_eq!(nil_id, NIL_SYM_ID);

        let t_name = registry.names.intern("t");
        let t_id = registry.alloc_symbol(t_name, true, NewSymbolName::AtomOnly);
        debug_assert_eq!(t_id, T_SYM_ID);

        let unbound_name = registry.names.intern("unbound");
        let unbound_id = registry.alloc_symbol(unbound_name, false, NewSymbolName::AtomOnly);
        debug_assert_eq!(unbound_id, UNBOUND_SYM_ID);

        registry
    }

    fn alloc_symbol(&mut self, name: NameId, canonical: bool, name_value: NewSymbolName) -> SymId {
        let id = SymId(self.symbols.len() as u32);
        self.symbols.push(SymbolSlot { name, canonical });
        if let NewSymbolName::LispObject(name_value) = name_value {
            let old = self.name_values.insert(id, name_value);
            debug_assert!(old.is_none(), "new symbol id already had a name value");
            self.name_value_roots.insert(name_value);
        }
        if canonical {
            self.canonical_by_name.insert(name, id);
        }
        id
    }

    fn slot(&self, id: SymId) -> Option<&SymbolSlot> {
        self.symbols.get(id.0 as usize)
    }

    fn intern(&mut self, s: &str) -> SymId {
        let name = self.names.intern(s);
        if let Some(existing) = self.canonical_by_name.get(&name) {
            return *existing;
        }
        self.alloc_symbol(name, true, NewSymbolName::AtomOnly)
    }

    fn intern_lisp_string(&mut self, s: &LispString) -> SymId {
        let name = self.names.intern_lisp_string(s);
        if let Some(existing) = self.canonical_by_name.get(&name) {
            return *existing;
        }
        self.alloc_symbol(name, true, NewSymbolName::AtomOnly)
    }

    fn intern_uninterned(&mut self, s: &str) -> SymId {
        let name = self.names.intern(s);
        self.alloc_symbol(name, false, NewSymbolName::AtomOnly)
    }

    fn intern_uninterned_lisp_string(&mut self, s: &LispString) -> SymId {
        let name = self.names.intern_lisp_string(s);
        self.alloc_symbol(name, false, NewSymbolName::AtomOnly)
    }

    /// GNU `Fintern` on a Lisp string: return the canonical symbol of that
    /// name, and when this call is the one that CREATES it, adopt the argument
    /// as the symbol's name object (lread.c:4796-4805 -> `intern_driver` ->
    /// `Fmake_symbol (string)`). An already-interned symbol keeps the name it
    /// was created with, so the argument's text properties stay invisible --
    /// GNU reaches `intern_driver` only when `oblookup` found nothing.
    ///
    /// Name IDENTITY still runs through the normalized name atom, so a unibyte
    /// and an ascii-only multibyte spelling name the same symbol either way;
    /// only which string object `symbol-name` hands back is at stake here.
    fn intern_lisp_value(&mut self, name_value: TaggedValue) -> SymId {
        let name = self
            .names
            .intern_lisp_string(name_value.as_lisp_string().expect("string name"));
        if let Some(existing) = self.canonical_by_name.get(&name) {
            return *existing;
        }
        self.alloc_symbol(name, true, NewSymbolName::from_lisp_object(name_value))
    }

    fn make_uninterned_symbol_with_name_value(&mut self, name_value: TaggedValue) -> SymId {
        let name = self
            .names
            .intern_lisp_string(name_value.as_lisp_string().expect("string name"));
        self.alloc_symbol(name, false, NewSymbolName::from_lisp_object(name_value))
    }

    fn lookup(&self, s: &str) -> Option<SymId> {
        let name = self.names.lookup(s)?;
        self.canonical_by_name.get(&name).copied()
    }

    fn lookup_lisp_string(&self, s: &LispString) -> Option<SymId> {
        let name = self.names.lookup_lisp_string(s)?;
        self.canonical_by_name.get(&name).copied()
    }

    #[inline]
    fn is_canonical_id(&self, id: SymId) -> bool {
        self.slot(id).map(|slot| slot.canonical).unwrap_or(false)
    }

    #[inline]
    fn resolve(&self, id: SymId) -> &'static str {
        self.resolve_lisp_string(id)
            .as_utf8_str()
            .unwrap_or_else(|| panic!("symbol name {:?} is not valid UTF-8", id))
    }

    /// A symbol's NAME ATOM: process-lifetime, byte-exact, and the footing
    /// symbol identity is decided on. Never the Lisp name object.
    ///
    /// The two must not be conflated. The atom lives for the process, so
    /// callers may cache it as `&'static` (`thread_local_resolve` does); the
    /// name object is an ordinary GC-managed heap string belonging to one heap,
    /// and handing it out here would let a `&'static` outlive it. Lisp-visible
    /// name reads go through [`Self::resolve_name_value`] instead, which is
    /// what `symbol-name` prefers.
    #[inline]
    fn resolve_lisp_string(&self, id: SymId) -> &'static LispString {
        let slot = self
            .slot(id)
            .unwrap_or_else(|| panic!("invalid symbol id {:?}", id));
        self.names.resolve_lisp_string(slot.name)
    }

    #[inline]
    fn resolve_name_value(&self, id: SymId) -> Option<TaggedValue> {
        self.slot(id)
            .unwrap_or_else(|| panic!("invalid symbol id {:?}", id));
        let name_value = self.name_values.get(&id).copied()?;
        (crate::tagged::gc::current_tagged_heap_identity().map(SymbolNameHeapId)
            == Some(name_value.heap_id))
        .then_some(name_value.value)
    }

    #[inline]
    fn name_id(&self, id: SymId) -> NameId {
        self.slot(id)
            .unwrap_or_else(|| panic!("invalid symbol id {:?}", id))
            .name
    }

    #[inline]
    fn resolve_name(&self, id: NameId) -> &'static str {
        self.names.resolve(id)
    }

    #[inline]
    #[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
    fn resolve_name_lisp_string(&self, id: NameId) -> &'static LispString {
        self.names.resolve_lisp_string(id)
    }

    fn dump_symbol_table(&self) -> DumpedSymbolTable {
        let names = self.names.strings.iter().map(LispString::clone).collect();
        let mut symbol_names = Vec::with_capacity(self.symbols.len());
        let mut canonical = Vec::with_capacity(self.symbols.len());
        for slot in &self.symbols {
            symbol_names.push(slot.name.0);
            canonical.push(slot.canonical);
        }
        DumpedSymbolTable {
            names,
            symbol_names,
            canonical,
        }
    }

    fn restore_dump_symbol_table(
        &mut self,
        names: &[LispString],
        symbol_names: &[u32],
        canonical: Option<&[bool]>,
    ) -> Result<RestoredDumpSymbolTable, String> {
        self.names.reserve_additional_names(names.len());
        self.symbols.reserve(symbol_names.len());
        let mut name_remap = Vec::with_capacity(names.len());
        for name in names {
            name_remap.push(self.names.intern_lisp_string(name));
        }

        let derived_flags;
        let canonical = match canonical {
            Some(flags) if flags.len() == symbol_names.len() => flags,
            Some([]) => {
                derived_flags = derive_legacy_canonical_flags_from_names(names, symbol_names)?;
                &derived_flags
            }
            None => {
                derived_flags = derive_legacy_canonical_flags_from_names(names, symbol_names)?;
                &derived_flags
            }
            Some(flags) => {
                return Err(format!(
                    "pdump symbol metadata is inconsistent: {} symbols but {} canonical flags",
                    symbol_names.len(),
                    flags.len()
                ));
            }
        };

        if symbol_names.len() != canonical.len() {
            return Err(format!(
                "pdump symbol metadata is inconsistent: {} symbols but {} canonical flags",
                symbol_names.len(),
                canonical.len()
            ));
        }

        self.canonical_by_name
            .reserve(canonical.iter().filter(|&&flag| flag).count());

        let mut dump_canonical_slots: FxHashMap<NameId, usize> = FxHashMap::default();

        let symbol_remap = symbol_names
            .iter()
            .copied()
            .zip(canonical.iter().copied())
            .enumerate()
            .map(|(slot, (dump_name_id, is_canonical))| {
                let runtime_name = name_remap
                    .get(dump_name_id as usize)
                    .copied()
                    .ok_or_else(|| {
                        format!(
                            "pdump symbol metadata is inconsistent: symbol name id {} out of range for {} names",
                            dump_name_id,
                            names.len()
                        )
                    })?;
                if is_canonical {
                    if let Some(previous_slot) = dump_canonical_slots.insert(runtime_name, slot) {
                        return Err(format!(
                            "pdump symbol metadata is inconsistent: canonical symbol slots {} and {} both name {}",
                            previous_slot,
                            slot,
                            self.names.resolve(runtime_name)
                        ));
                    }
                    Ok::<SymId, String>(
                        self.canonical_by_name
                            .get(&runtime_name)
                            .copied()
                            .unwrap_or_else(|| {
                                self.alloc_symbol(runtime_name, true, NewSymbolName::AtomOnly)
                            }),
                    )
                } else {
                    Ok::<SymId, String>(self.alloc_symbol(
                        runtime_name,
                        false,
                        NewSymbolName::AtomOnly,
                    ))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(RestoredDumpSymbolTable {
            names: name_remap,
            symbols: symbol_remap,
        })
    }

    #[inline]
    fn canonical_symbol_for_name(&self, name: NameId) -> Option<SymId> {
        self.canonical_by_name.get(&name).copied()
    }

    fn unintern_canonical_symbol(&mut self, id: SymId) -> bool {
        let Some(slot) = self.symbols.get_mut(id.0 as usize) else {
            return false;
        };
        if !slot.canonical {
            return false;
        }
        if self.canonical_by_name.get(&slot.name).copied() != Some(id) {
            return false;
        }
        self.canonical_by_name.remove(&slot.name);
        slot.canonical = false;
        true
    }

    fn collect_name_value_roots(&self, roots: &mut Vec<TaggedValue>, heap_id: usize) {
        let heap_id = SymbolNameHeapId(heap_id);
        // Cross-check the identity set against the authoritative per-symbol
        // metadata in debug tests. The production root walk stays O(unique
        // name objects), independent of how many symbols share each object.
        #[cfg(all(test, debug_assertions))]
        {
            let mut unique = FxHashMap::default();
            for name_value in self
                .name_values
                .values()
                .filter(|name_value| name_value.heap_id == heap_id)
            {
                unique.insert(SymbolNameObjectId::of(name_value.value), name_value.value);
            }
            assert_eq!(
                self.name_value_roots.root_count(heap_id),
                unique.len(),
                "per-heap name-object root index diverged"
            );
        }
        self.name_value_roots.extend_roots(roots, heap_id);
    }
}

// Dumped Lisp strings are immutable for this pass; their GC-aware interior
// mutability does not invalidate the temporary content-keyed map.
#[allow(clippy::mutable_key_type)]
fn derive_legacy_canonical_flags_from_names(
    names: &[LispString],
    symbol_names: &[u32],
) -> Result<Vec<bool>, String> {
    let mut seen = FxHashMap::default();
    symbol_names
        .iter()
        .copied()
        .map(|dump_name_id| {
            let name = names.get(dump_name_id as usize).ok_or_else(|| {
                format!(
                    "pdump symbol metadata is inconsistent: symbol name id {} out of range for {} names",
                    dump_name_id,
                    names.len()
                )
            })?;
            Ok(seen.insert(name.clone(), ()).is_none())
        })
        .collect()
}

fn global_symbol_registry() -> &'static RwLock<SymbolRegistry> {
    static GLOBAL_SYMBOL_REGISTRY: OnceLock<RwLock<SymbolRegistry>> = OnceLock::new();
    GLOBAL_SYMBOL_REGISTRY.get_or_init(|| RwLock::new(SymbolRegistry::new()))
}

fn symbol_registry_epoch() -> &'static AtomicU64 {
    static SYMBOL_REGISTRY_EPOCH: AtomicU64 = AtomicU64::new(0);
    &SYMBOL_REGISTRY_EPOCH
}

pub(crate) fn dump_runtime_interner() -> DumpedSymbolTable {
    let registry = global_symbol_registry().read();
    registry.dump_symbol_table()
}

pub(crate) fn restore_runtime_interner(
    names: &[LispString],
    symbol_names: &[u32],
    canonical: Option<&[bool]>,
) -> Result<RestoredDumpSymbolTable, String> {
    let mut registry = global_symbol_registry().write();
    registry.restore_dump_symbol_table(names, symbol_names, canonical)
}

/// Intern a string using the global runtime symbol registry.
#[inline]
pub fn intern(s: &str) -> SymId {
    ensure_thread_local_cache_epoch_current();
    if let Some(sym_id) = thread_local_interned_str(s) {
        return sym_id;
    }
    let mut registry = global_symbol_registry().write();
    let sym_id = registry.intern(s);
    let canonical_name = registry.resolve(sym_id);
    drop(registry);
    thread_local_record_interned_str(canonical_name, sym_id);
    sym_id
}

/// Intern an exact Lisp-string name using the global runtime symbol registry.
#[inline]
pub fn intern_lisp_string(s: &LispString) -> SymId {
    let mut registry = global_symbol_registry().write();
    registry.intern_lisp_string(s)
}

/// Create an uninterned symbol using the global runtime symbol registry.
/// Always creates a new unique SymId, never reuses an existing one.
#[inline]
pub fn intern_uninterned(s: &str) -> SymId {
    let mut registry = global_symbol_registry().write();
    registry.intern_uninterned(s)
}

/// Create an uninterned symbol using an exact Lisp-string name.
#[inline]
pub fn intern_uninterned_lisp_string(s: &LispString) -> SymId {
    let mut registry = global_symbol_registry().write();
    registry.intern_uninterned_lisp_string(s)
}

/// Intern NAME_VALUE in the global obarray, adopting it as the new symbol's
/// exact name object when this call creates the symbol, matching GNU `intern`.
///
/// Prefer this over [`intern_lisp_string`] wherever the caller holds the Lisp
/// string OBJECT a symbol is being named from: `intern_lisp_string` keeps only
/// the name atom, which drops the string's text properties and its
/// multibyteness.
#[inline]
pub fn intern_lisp_value(name_value: TaggedValue) -> SymId {
    let mut registry = global_symbol_registry().write();
    registry.intern_lisp_value(name_value)
}

/// Create an uninterned symbol that stores NAME_VALUE as its exact name
/// object, matching GNU `make-symbol`.
#[inline]
pub fn make_uninterned_symbol_with_name_value(name_value: TaggedValue) -> SymId {
    let mut registry = global_symbol_registry().write();
    registry.make_uninterned_symbol_with_name_value(name_value)
}

/// Look up the canonical interned symbol id for a string without interning it.
#[inline]
pub fn lookup_interned(s: &str) -> Option<SymId> {
    let registry = global_symbol_registry().read();
    registry.lookup(s)
}

#[inline]
pub fn lookup_interned_lisp_string(s: &LispString) -> Option<SymId> {
    let registry = global_symbol_registry().read();
    registry.lookup_lisp_string(s)
}

#[inline]
pub fn is_canonical_id(id: SymId) -> bool {
    ensure_thread_local_cache_epoch_current();
    if let Some(is_canonical) = thread_local_is_canonical(id) {
        return is_canonical;
    }
    let registry = global_symbol_registry().read();
    let is_canonical = registry.is_canonical_id(id);
    drop(registry);
    thread_local_record_canonical(id, is_canonical);
    is_canonical
}

#[inline]
pub(crate) fn is_keyword_id(id: SymId) -> bool {
    if let Some(is_keyword) = thread_local_keyword(id) {
        return is_keyword;
    }
    let registry = global_symbol_registry().read();
    let is_keyword = registry
        .slot(id)
        .map(|slot| {
            slot.canonical
                && registry
                    .names
                    .resolve_lisp_string(slot.name)
                    .as_bytes()
                    .first()
                    .is_some_and(|byte| *byte == b':')
        })
        .unwrap_or(false);
    drop(registry);
    thread_local_record_keyword(id, is_keyword);
    is_keyword
}

#[inline]
pub fn resolve_sym_metadata(id: SymId) -> (&'static str, bool) {
    ensure_thread_local_cache_epoch_current();
    if let Some(is_canonical) = thread_local_is_canonical(id)
        && is_canonical
        && let Some(name) = thread_local_resolve(id)
    {
        return (name, true);
    }
    let registry = global_symbol_registry().read();
    let name_value = registry.resolve_lisp_string(id);
    let name = name_value
        .as_utf8_str()
        .unwrap_or_else(|| panic!("symbol name {:?} is not valid UTF-8", id));
    let is_canonical = registry.is_canonical_id(id);
    drop(registry);
    thread_local_record_canonical(id, is_canonical);
    if is_canonical {
        thread_local_record_name(id, name_value);
    }
    (name, is_canonical)
}

#[inline]
pub(crate) fn symbol_name_id(id: SymId) -> NameId {
    if let Some(name_id) = thread_local_name_id(id) {
        return name_id;
    }
    let registry = global_symbol_registry().read();
    let name_id = registry.name_id(id);
    drop(registry);
    thread_local_record_name_id(id, name_id);
    name_id
}

#[inline]
pub(crate) fn resolve_name(id: NameId) -> &'static str {
    let registry = global_symbol_registry().read();
    registry.resolve_name(id)
}

#[inline]
#[allow(dead_code)] // grandfathered when dead_code lint was enabled; delete or wire up
pub(crate) fn resolve_name_lisp_string(id: NameId) -> &'static LispString {
    let registry = global_symbol_registry().read();
    registry.resolve_name_lisp_string(id)
}

#[inline]
pub(crate) fn canonical_symbol_for_name(id: NameId) -> Option<SymId> {
    ensure_thread_local_cache_epoch_current();
    if let Some(sym_id) = thread_local_canonical_symbol_for_name(id) {
        return Some(sym_id);
    }
    let registry = global_symbol_registry().read();
    let sym_id = registry.canonical_symbol_for_name(id)?;
    drop(registry);
    thread_local_record_canonical_symbol_for_name(id, sym_id);
    Some(sym_id)
}

/// Resolve a SymId to its string using the global runtime symbol registry.
#[inline]
pub fn resolve_sym(id: SymId) -> &'static str {
    ensure_thread_local_cache_epoch_current();
    if let Some(is_canonical) = thread_local_is_canonical(id)
        && is_canonical
        && let Some(s) = thread_local_resolve(id)
    {
        return s;
    }
    let registry = global_symbol_registry().read();
    let name_value = registry.resolve_lisp_string(id);
    let s = name_value
        .as_utf8_str()
        .unwrap_or_else(|| panic!("symbol name {:?} is not valid UTF-8", id));
    let is_canonical = registry.is_canonical_id(id);
    drop(registry);
    thread_local_record_canonical(id, is_canonical);
    if is_canonical {
        thread_local_record_name(id, name_value);
    }
    s
}

/// Resolve a SymId to its exact Lisp-string name using the global runtime
/// symbol registry.
#[inline]
pub fn resolve_sym_lisp_string(id: SymId) -> &'static LispString {
    let registry = global_symbol_registry().read();
    registry.resolve_lisp_string(id)
}

/// Remove ID from the process-global canonical name map.
///
/// GNU `unintern` unlinks a symbol object from the obarray and marks that
/// object uninterned; a later `intern` of the same name allocates a distinct
/// symbol.  Neomacs keeps symbol objects in a process-global registry, so the
/// obarray layer must explicitly remove the registry's canonical name mapping
/// when the initial obarray uninterns a canonical symbol.
pub(crate) fn unintern_canonical_id(id: SymId) -> bool {
    let changed = {
        let mut registry = global_symbol_registry().write();
        registry.unintern_canonical_symbol(id)
    };
    if changed {
        symbol_registry_epoch().fetch_add(1, Ordering::AcqRel);
        ensure_thread_local_cache_epoch_current();
    }
    changed
}

/// The string a symbol's name READS as from Lisp: the name object the symbol
/// was created from when it has one on the current heap, else its name atom.
///
/// GNU has only the first case -- a symbol's name is the string object it was
/// created from -- so everything Lisp can observe follows the object:
/// `symbol-name`, printing, and obarray lookup, including through a later
/// `aset` on that string.
///
/// Do NOT cache the result. Unlike the process-lifetime atom from
/// [`resolve_sym_lisp_string`], a name object is an ordinary GC-managed heap
/// string owned by one heap.
#[inline]
pub fn resolve_sym_lisp_name(id: SymId) -> &'static LispString {
    resolve_sym_name_value(id)
        .and_then(|name_value| name_value.as_lisp_string())
        .unwrap_or_else(|| resolve_sym_lisp_string(id))
}

#[inline]
pub fn resolve_sym_name_value(id: SymId) -> Option<TaggedValue> {
    let registry = global_symbol_registry().read();
    registry.resolve_name_value(id)
}

pub(crate) fn collect_symbol_name_gc_roots(roots: &mut Vec<TaggedValue>, heap_id: usize) {
    let registry = global_symbol_registry().read();
    registry.collect_name_value_roots(roots, heap_id);
}

// ---------------------------------------------------------------------------
// Thread-local lockless cache for SymId -> &'static str
// ---------------------------------------------------------------------------
//
// `resolve_sym` is called from many bytecode hot paths (e.g. `is_keyword`,
// debug formatting) and acquiring the global RwLock — even with parking_lot
// — is many extra atomic ops per call. Once a SymId is interned, the
// underlying interned `&'static LispString` is permanently valid, so the
// (id -> name) mapping is monotonic and stable for the lifetime of the process.

#[derive(Clone, Copy, Debug)]
struct SymbolCacheEntry {
    /// A thin pointer is enough here. Converting the interned `LispString` to
    /// `&str` on a hit avoids paying for a 16-byte fat pointer in every dense
    /// cache entry.
    name: Option<&'static LispString>,
    /// `NameId(u32::MAX)` is reserved globally and doubles as the cache-miss
    /// sentinel, avoiding the padding of `Option<NameId>`.
    name_id: u32,
    canonical: Option<bool>,
    keyword: Option<bool>,
}

impl Default for SymbolCacheEntry {
    fn default() -> Self {
        Self {
            name: None,
            name_id: SYMBOL_NAME_CACHE_MISSING,
            canonical: None,
            keyword: None,
        }
    }
}

const SYMBOL_NAME_CACHE_MISSING: u32 = u32::MAX;
// Canonical `nil` has SymId 0. Leaving its name-cache slot at zero merely
// makes that one lookup miss the cache; every other canonical SymId can be
// stored directly, which keeps this dense NameId-indexed table at four bytes
// per entry instead of eight for `Option<SymId>`.
const NAME_CANONICAL_CACHE_MISSING: u32 = NIL_SYM_ID.0;

thread_local! {
    static THREAD_CACHE_EPOCH: RefCell<u64> = const { RefCell::new(0) };
    static INTERN_STR_CACHE: RefCell<FxHashMap<&'static str, SymId>> = RefCell::new(FxHashMap::default());
    static SYMBOL_CACHE: RefCell<Vec<SymbolCacheEntry>> = const { RefCell::new(Vec::new()) };
    static NAME_CANONICAL_SYMBOL_CACHE: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn ensure_thread_local_cache_epoch_current() {
    let current = symbol_registry_epoch().load(Ordering::Acquire);
    THREAD_CACHE_EPOCH.with(|epoch| {
        let mut epoch = epoch.borrow_mut();
        if *epoch == current {
            return;
        }
        *epoch = current;
        INTERN_STR_CACHE.with(|cache| cache.borrow_mut().clear());
        SYMBOL_CACHE.with(|cache| cache.borrow_mut().clear());
        NAME_CANONICAL_SYMBOL_CACHE.with(|cache| cache.borrow_mut().clear());
    });
}

#[inline]
fn thread_local_interned_str(s: &str) -> Option<SymId> {
    INTERN_STR_CACHE.with(|cache| cache.borrow().get(s).copied())
}

#[inline]
fn thread_local_record_interned_str(s: &'static str, id: SymId) {
    INTERN_STR_CACHE.with(|cache| {
        cache.borrow_mut().insert(s, id);
    });
}

#[inline]
fn thread_local_resolve(id: SymId) -> Option<&'static str> {
    SYMBOL_CACHE.with(|cache| {
        let cache = cache.borrow();
        cache
            .get(id.0 as usize)
            .and_then(|entry| entry.name)
            .map(|name| {
                name.as_utf8_str()
                    .expect("only UTF-8 symbol names enter the resolution cache")
            })
    })
}

#[inline]
fn thread_local_record_name(id: SymId, name: &'static LispString) {
    SYMBOL_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let idx = id.0 as usize;
        if cache.len() <= idx {
            cache.resize(idx + 1, SymbolCacheEntry::default());
        }
        cache[idx].name = Some(name);
    });
}

#[inline]
fn thread_local_name_id(id: SymId) -> Option<NameId> {
    SYMBOL_CACHE.with(|cache| {
        let cache = cache.borrow();
        cache
            .get(id.0 as usize)
            .map(|entry| entry.name_id)
            .filter(|&name_id| name_id != SYMBOL_NAME_CACHE_MISSING)
            .map(NameId)
    })
}

#[inline]
fn thread_local_record_name_id(id: SymId, name_id: NameId) {
    debug_assert_ne!(name_id.0, SYMBOL_NAME_CACHE_MISSING);
    SYMBOL_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let idx = id.0 as usize;
        if cache.len() <= idx {
            cache.resize(idx + 1, SymbolCacheEntry::default());
        }
        cache[idx].name_id = name_id.0;
    });
}

#[inline]
fn thread_local_is_canonical(id: SymId) -> Option<bool> {
    SYMBOL_CACHE.with(|cache| {
        let cache = cache.borrow();
        cache.get(id.0 as usize).and_then(|entry| entry.canonical)
    })
}

#[inline]
fn thread_local_record_canonical(id: SymId, is_canonical: bool) {
    SYMBOL_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let idx = id.0 as usize;
        if cache.len() <= idx {
            cache.resize(idx + 1, SymbolCacheEntry::default());
        }
        cache[idx].canonical = Some(is_canonical);
    });
}

#[inline]
fn thread_local_canonical_symbol_for_name(id: NameId) -> Option<SymId> {
    NAME_CANONICAL_SYMBOL_CACHE.with(|cache| {
        let cache = cache.borrow();
        cache
            .get(id.0 as usize)
            .copied()
            .filter(|&sym_id| sym_id != NAME_CANONICAL_CACHE_MISSING)
            .map(SymId)
    })
}

#[inline]
fn thread_local_record_canonical_symbol_for_name(id: NameId, sym_id: SymId) {
    NAME_CANONICAL_SYMBOL_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let idx = id.0 as usize;
        if cache.len() <= idx {
            cache.resize(idx + 1, NAME_CANONICAL_CACHE_MISSING);
        }
        cache[idx] = sym_id.0;
    });
}

#[inline]
fn thread_local_keyword(id: SymId) -> Option<bool> {
    SYMBOL_CACHE.with(|cache| {
        let cache = cache.borrow();
        cache.get(id.0 as usize).and_then(|entry| entry.keyword)
    })
}

#[inline]
fn thread_local_record_keyword(id: SymId, is_keyword: bool) {
    SYMBOL_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let idx = id.0 as usize;
        if cache.len() <= idx {
            cache.resize(idx + 1, SymbolCacheEntry::default());
        }
        cache[idx].keyword = Some(is_keyword);
    });
}

/// Resolve a SymId to its string using the global runtime symbol registry.
///
/// Returns `None` if the id is outside the current symbol range instead of
/// panicking. This is useful at serialization boundaries where we want a
/// structured error instead of aborting the process on malformed runtime data.
#[inline]
pub fn try_resolve_sym(id: SymId) -> Option<&'static str> {
    let registry = global_symbol_registry().read();
    registry
        .slot(id)
        .map(|slot| registry.names.resolve(slot.name))
}

#[inline]
pub fn try_resolve_sym_lisp_string(id: SymId) -> Option<&'static LispString> {
    let registry = global_symbol_registry().read();
    registry
        .slot(id)
        .map(|slot| registry.names.resolve_lisp_string(slot.name))
}

#[cfg(test)]
#[path = "intern_test.rs"]
mod tests;
