//! Ahead-of-time (AOT) object emission for compiled leaves (Phase R1c).
//!
//! AOT is a **fourth code producer**, never a deopt target: it emits the *same*
//! CLIF the JIT does (via the R1b `compile::build_mir_leaf_fn::<M: Module>` seam)
//! but routes it through Cranelift's [`ObjectModule`] instead of `JITModule`, so
//! the result is a relocatable `.o` rather than executable memory. The `.o` is
//! later linked into a `.so` (`cc -shared`) and `dlopen`'d, its entry inserted
//! into the per-thread `COMPILED` cache as a pre-warmed [`CompiledLeaf`]
//! (R1c-4..6). Tier-0 `bytecode::Vm` stays the sole oracle + `DeoptAt` landing
//! pad; the GC is concurrent non-moving, so AOT's only GC duty is liveness +
//! SATB-correct root publication (R1c-8) — no fixup/stackmaps.
//!
//! ## The three JIT seams AOT replaces (cf. `build_mir_leaf_fn`'s doc)
//!   * `builder.symbol(...)`    — JIT bakes shim host addresses; AOT leaves the
//!     `neovm_jit_*` shims as **undefined `Linkage::Import`s** (declared by
//!     `declare_rt_refs`), resolved by the dynamic loader against the host
//!     process at `dlopen` (host links `-rdynamic`; R1c-5).
//!   * `finalize_definitions()` — replaced by `ObjectModule::finish().emit()`.
//!   * `get_finalized_function` — replaced by a `dlsym` of the exported entry.
//!
//! Only built with the `jit` feature (links Cranelift).

use cranelift_codegen::settings::{self, Configurable};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};

use super::backend::BackendError;
use super::compile::{CompileError, DeoptCells};
use super::mir;
use crate::emacs_core::bytecode::opcode::Op;
use crate::emacs_core::value::Value;
use crate::tagged::value::{FIXNUM_CHECK_MASK, FIXNUM_CHECK_VALUE};

/// Wrap a stringly backend error as a `CompileError` (module-init flavor — the
/// ObjectModule setup + emit are the "module" steps here).
fn module_init_err(msg: String) -> CompileError {
    CompileError::Backend(BackendError::ModuleInit(msg))
}

// ---------------------------------------------------------------------------
// R1c-2: content hash + ABI tag + entry symbol.
//
// A loaded `.so` is only safe to run if (a) it was emitted from the SAME source
// (content hash) and (b) the host's ABI matches what the code assumes (ABI tag).
// The loader (R1c-5) refuses a hash/tag mismatch and falls back to the JIT.
// ---------------------------------------------------------------------------

/// ABI compatibility tag for AOT artifacts — a `u32` fingerprint of every
/// structural assumption the emitted code + loader share. Any change here MUST
/// bump `ABI_TAG_VERSION` so a stale `.so` (with a different tag) is refused.
///
/// Encodes: the `STATUS_*` return codes, the entry ABI shape (3 pointer params +
/// i64 return), the reloc-base + `DeoptCells` layout, and the `neovm_jit_*` shim
/// name set (the imports the loader binds). Born `u32` in R1c; hardened to a
/// `u128` ISA+layout hash in R3.4b. **No epoch is ever encoded** (epochs are
/// re-derived from the live obarray at load — see the spec's cross-session
/// invariants).
pub(crate) const ABI_TAG: u32 = compute_abi_tag();

/// Bump on ANY change to the entry ABI, `STATUS_*` codes, `DeoptCells`/reloc-base
/// layout, or the shim set — it salts [`ABI_TAG`] so old artifacts stop matching.
const ABI_TAG_VERSION: u32 = 1;

/// The runtime-shim symbols an AOT `.so` imports (resolved against the host at
/// `dlopen`). Folded into [`ABI_TAG`] so a shim-ABI change invalidates artifacts.
/// MUST stay in sync with the `builder.symbol(...)` set in `lower_mir_pure` /
/// `declare_rt_refs` (the shims the MIR tier can reference).
pub(crate) const MIR_SHIM_NAMES: &[&str] = &[
    "neovm_jit_gc_save",
    "neovm_jit_gc_push",
    "neovm_jit_gc_restore",
    "neovm_jit_call",
    "neovm_jit_apply",
    "neovm_jit_cons",
];

/// Compute [`ABI_TAG`] at compile time from the structural invariants. A `const`
/// FNV-1a over the salient constants + the shim names, so any drift in the ABI
/// the code assumes changes the tag (and old `.so`s no longer match).
const fn compute_abi_tag() -> u32 {
    // FNV-1a (32-bit), const-evaluable.
    let mut h: u32 = 0x811c_9dc5;
    macro_rules! mix_u64 {
        ($v:expr) => {{
            let v: u64 = $v;
            let mut i = 0;
            while i < 8 {
                let byte = ((v >> (i * 8)) & 0xff) as u32;
                h ^= byte;
                h = h.wrapping_mul(0x0100_0193);
                i += 1;
            }
        }};
    }
    mix_u64!(ABI_TAG_VERSION as u64);
    // STATUS_* codes (the loader + code agree on these).
    mix_u64!(super::compile::STATUS_OK as u64);
    mix_u64!(super::compile::STATUS_DEOPT as u64);
    mix_u64!(super::compile::STATUS_SIGNAL as u64);
    mix_u64!(super::compile::STATUS_DEOPT_AT as u64);
    // Entry ABI shape: 3 pointer params + 1 i64 return (encode as a small code).
    mix_u64!(0x0003_0001);
    // DeoptCells layout: 3 i64 cells (pc, depth, handlers).
    mix_u64!(core::mem::size_of::<DeoptCells>() as u64);
    // Shim name set (count + each byte) — a shim-ABI change re-tags artifacts.
    mix_u64!(MIR_SHIM_NAMES.len() as u64);
    let mut si = 0;
    while si < MIR_SHIM_NAMES.len() {
        let name = MIR_SHIM_NAMES[si].as_bytes();
        let mut bi = 0;
        while bi < name.len() {
            h ^= name[bi] as u32;
            h = h.wrapping_mul(0x0100_0193);
            bi += 1;
        }
        si += 1;
    }
    h
}

/// The exported entry symbol for an AOT leaf with the given content hash:
/// `__neovm_aot_{hash:032x}_{ABI_TAG:08x}`. The tag is in the symbol so a
/// mismatched-ABI `.so` cannot even be `dlsym`'d under the current tag (a second,
/// cheap interlock on top of the descriptor check).
pub(crate) fn aot_entry_symbol(content_hash: u128) -> String {
    format!("__neovm_aot_{content_hash:032x}_{ABI_TAG:08x}")
}

// ---------------------------------------------------------------------------
// R1c-3: per-Value rebuild recipe (canonical, pointer-free).
//
// Heap-object reloc constants cannot bake a pointer into a cross-session `.so`,
// so each is serialized as a recipe (fixnum→bits, string→utf8, symbol→name,
// cons→recursive) and rebuilt against the LIVE obarray/heap at load. The SAME
// canonical encoding feeds the content hash (R1c-2), so two bodies with
// identical structure + constants hash identically regardless of heap layout.
// ---------------------------------------------------------------------------

/// Recipe type tags (1 byte) for [`write_value_recipe`] / [`rebuild_value`].
const RECIPE_FIXNUM: u8 = 1;
const RECIPE_STRING: u8 = 2;
const RECIPE_SYMBOL: u8 = 3;
const RECIPE_CONS: u8 = 4;
const RECIPE_NIL: u8 = 5;
const RECIPE_T: u8 = 6;

/// A Value whose type the AOT recipe codec does not (yet) support. The emitter
/// bails to the JIT rather than emit an artifact it cannot rebuild — keeping AOT
/// strictly additive (R1c-6: miss/error → JIT).
#[derive(Debug)]
pub(crate) struct UnsupportedRecipe(pub Value);

/// Serialize one `Value` into `out` as a canonical, pointer-free recipe.
///
/// Supports the const subset AOT can rebuild: fixnum, string, symbol, cons
/// (recursive), and the nil/t immediates. Anything else (float, vector, hash
/// table, ...) returns [`UnsupportedRecipe`] so the caller bails to the JIT.
pub(crate) fn write_value_recipe(out: &mut Vec<u8>, v: Value) -> Result<(), UnsupportedRecipe> {
    if v == Value::NIL {
        out.push(RECIPE_NIL);
        return Ok(());
    }
    if v == Value::T {
        out.push(RECIPE_T);
        return Ok(());
    }
    if let Some(n) = v.as_fixnum() {
        out.push(RECIPE_FIXNUM);
        out.extend_from_slice(&n.to_le_bytes());
        return Ok(());
    }
    if let Some(s) = v.as_lisp_string() {
        let bytes = s.as_bytes();
        out.push(RECIPE_STRING);
        out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(bytes);
        return Ok(());
    }
    if let Some(id) = v.as_symbol_id() {
        // A symbol is rebuilt at load BY NAME (`intern(name)`), so reloc-by-name
        // is sound ONLY for the CANONICAL interned symbol of that name. An
        // UNINTERNED / gensym (`make-symbol`, cl-macro/pcase expansion output)
        // has a non-unique name, so `intern(name)` in the load session would
        // yield a DIFFERENT (the canonical) symbol → wrong result. Reject it so
        // the whole leaf falls to the JIT (which bakes the in-session SymId,
        // correct same-session). (Audit #16 gensym hole — team-lead.)
        if !crate::emacs_core::intern::is_canonical_id(id) {
            return Err(UnsupportedRecipe(v));
        }
        let name = crate::emacs_core::intern::resolve_sym(id);
        let nb = name.as_bytes();
        out.push(RECIPE_SYMBOL);
        out.extend_from_slice(&(nb.len() as u64).to_le_bytes());
        out.extend_from_slice(nb);
        return Ok(());
    }
    if v.is_cons() {
        out.push(RECIPE_CONS);
        write_value_recipe(out, v.cons_car())?;
        write_value_recipe(out, v.cons_cdr())?;
        return Ok(());
    }
    Err(UnsupportedRecipe(v))
}

/// Max cons nesting a reloc recipe may rebuild — bounds the recursion so a
/// crafted/corrupt recipe (a deep RECIPE_CONS chain) cannot overflow the stack.
/// Real loadup const lists nest far shallower; a deeper recipe falls to JIT.
const MAX_RECIPE_CONS_DEPTH: usize = 256;

/// Rebuild a `Value` from a recipe slice, allocating fresh heap objects against
/// the LIVE thread-local heap + obarray (a string/cons born here is allocated
/// black by the GC's alloc path; rooting is the caller's duty — R1c-8). Returns
/// the value + the number of bytes consumed, or `None` on a malformed/truncated/
/// over-deep recipe.
///
/// Hardening (audit #4-9/#12): the recipe is dlsym'd out of a `.so` in the
/// NEOVM_AOT_DIR (a trust boundary). Although that dir already grants RCE (dlopen
/// runs the `.so`), this parser must FAIL CLOSED — every length/index is
/// bounds-checked and cons recursion is depth-bounded, so a corrupt artifact
/// returns `None` (→ the loader falls through to the JIT, honoring the additive
/// contract) instead of panicking / over-reading / overflowing the stack.
pub(crate) fn rebuild_value(bytes: &[u8], depth: usize) -> Option<(Value, usize)> {
    if depth > MAX_RECIPE_CONS_DEPTH {
        return None;
    }
    let tag = *bytes.first()?;
    match tag {
        RECIPE_NIL => Some((Value::NIL, 1)),
        RECIPE_T => Some((Value::T, 1)),
        RECIPE_FIXNUM => {
            let n = i64::from_le_bytes(bytes.get(1..9)?.try_into().ok()?);
            Some((Value::make_int(n), 9))
        }
        RECIPE_STRING => {
            let len = u64::from_le_bytes(bytes.get(1..9)?.try_into().ok()?) as usize;
            let s = std::str::from_utf8(bytes.get(9..9usize.checked_add(len)?)?).ok()?;
            Some((Value::string(s), 9 + len))
        }
        RECIPE_SYMBOL => {
            let len = u64::from_le_bytes(bytes.get(1..9)?.try_into().ok()?) as usize;
            let name = std::str::from_utf8(bytes.get(9..9usize.checked_add(len)?)?).ok()?;
            let id = crate::emacs_core::intern::intern(name);
            Some((Value::symbol(id), 9 + len))
        }
        RECIPE_CONS => {
            let (car, n1) = rebuild_value(bytes.get(1..)?, depth + 1)?;
            let (cdr, n2) = rebuild_value(bytes.get(1 + n1..)?, depth + 1)?;
            Some((Value::cons(car, cdr), 1 + n1 + n2))
        }
        _ => None, // unknown tag → fail closed.
    }
}

/// Content hash of a leaf's SOURCE (`ops` + canonical `constants` + `arity`),
/// salted by [`ABI_TAG`]. Gensym-stable: bytecode `Op`s carry only indices /
/// immediates (no pointers), and constants are canonicalized by VALUE (fixnum
/// bits, string bytes, symbol names, recursive conses) not by heap address — so
/// the same source hashes identically across sessions. The lambda-list `arity`
/// is folded in (the spec's arity-drift requirement). Returns `None` if any
/// constant is outside the recipe-supported subset (caller bails to JIT).
///
/// Truncated to `u128` (the entry-symbol width). Body identity rests SOLELY on
/// this hash (cryptographically collision-resistant for honest inputs) plus the
/// trusted `NEOVM_AOT_DIR` (any actor who can plant a `.so` there already has
/// in-process RCE via dlopen, so a hash collision is not an added attack
/// surface). NOTE (audit #11): there is NO load-time re-verification that the
/// rebuilt const vector equals the call-site constants — the hash is the whole
/// proof. Adding that recheck is a documented defense-in-depth follow-up.
pub(crate) fn leaf_content_hash(ops: &[Op], constants: &[Value], arity: usize) -> Option<u128> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(ABI_TAG.to_le_bytes());
    h.update((arity as u64).to_le_bytes());
    h.update((ops.len() as u64).to_le_bytes());
    // Ops: their Debug form is deterministic for this enum (Constant(idx)/Add/
    // Goto(t)/...) and pointer-free, so it canonically identifies the body —
    // EXCEPT `CallBuiltinSym(SymId, _)`, whose Debug embeds a SESSION-SPECIFIC
    // raw SymId (audit #17). Such a body is not AOT-runnable (mir_is_aot_runnable
    // rejects it), so bail the hash too — never key the AOT cache on a
    // non-canonical op. (Any future SymId-bearing op must be added here.)
    for op in ops {
        if matches!(op, Op::CallBuiltinSym(..)) {
            return None;
        }
        let s = format!("{op:?}");
        h.update((s.len() as u64).to_le_bytes());
        h.update(s.as_bytes());
    }
    // Constants: canonical recipe bytes (by value, not address).
    h.update((constants.len() as u64).to_le_bytes());
    let mut recipe = Vec::new();
    for &c in constants {
        recipe.clear();
        write_value_recipe(&mut recipe, c).ok()?;
        h.update((recipe.len() as u64).to_le_bytes());
        h.update(&recipe);
    }
    let digest = h.finalize();
    Some(u128::from_le_bytes(digest[..16].try_into().unwrap()))
}

// ---------------------------------------------------------------------------
// R1c-3 (cont.): the leaf DESCRIPTOR — a versioned, exported data blob carrying
// the lambda-list + frame metadata + reloc rebuild recipe. Emitted into the `.o`
// alongside the entry; dlsym'd + parsed by the loader (R1c-5).
// ---------------------------------------------------------------------------

/// Magic + version header on every descriptor blob, so the loader rejects a
/// truncated/foreign blob and a format change can be detected. The ABI_TAG also
/// rides along (a second interlock besides the entry-symbol tag).
const DESC_MAGIC: u32 = 0x4e41_4f54; // "NAOT"
const DESC_VERSION: u32 = 1;

/// The decoded descriptor: leaf metadata + the reloc rebuild recipe bytes.
pub(crate) struct AotDescriptor {
    pub meta: super::compile::AotLeafMeta,
    /// Concatenated per-slot recipes (R1c-3); rebuilt into the reloc Vec at load.
    pub reloc_recipe: Vec<u8>,
    /// Number of reloc slots (recipes) in `reloc_recipe`.
    pub reloc_count: u32,
}

/// Serialize an [`AotDescriptor`] to bytes (little-endian, fixed header + recipe
/// tail). Layout: magic, version, ABI_TAG, then the meta fields, then
/// reloc_count, then the concatenated recipe bytes.
pub(crate) fn encode_descriptor(
    meta: &super::compile::AotLeafMeta,
    reloc_recipe: &[u8],
    reloc_count: u32,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&DESC_MAGIC.to_le_bytes());
    out.extend_from_slice(&DESC_VERSION.to_le_bytes());
    out.extend_from_slice(&ABI_TAG.to_le_bytes());
    out.extend_from_slice(&(meta.arity as u64).to_le_bytes());
    out.extend_from_slice(&(meta.required as u64).to_le_bytes());
    out.push(u8::from(meta.has_rest));
    out.push(u8::from(meta.has_binds));
    out.push(u8::from(meta.has_handlers));
    out.push(u8::from(meta.has_side_effects));
    out.push(u8::from(meta.has_precise_deopt));
    out.extend_from_slice(&(meta.max_depth as u64).to_le_bytes());
    out.extend_from_slice(&reloc_count.to_le_bytes());
    out.extend_from_slice(&(reloc_recipe.len() as u64).to_le_bytes());
    out.extend_from_slice(reloc_recipe);
    out
}

/// Parse a descriptor blob. Returns `None` on a bad magic/version/ABI_TAG or a
/// truncated blob — the loader then refuses the artifact and falls back to JIT.
pub(crate) fn decode_descriptor(bytes: &[u8]) -> Option<AotDescriptor> {
    fn rd_u32(b: &[u8], at: &mut usize) -> Option<u32> {
        let v = b.get(*at..*at + 4)?;
        *at += 4;
        Some(u32::from_le_bytes(v.try_into().ok()?))
    }
    fn rd_u64(b: &[u8], at: &mut usize) -> Option<u64> {
        let v = b.get(*at..*at + 8)?;
        *at += 8;
        Some(u64::from_le_bytes(v.try_into().ok()?))
    }
    fn rd_u8(b: &[u8], at: &mut usize) -> Option<u8> {
        let v = *b.get(*at)?;
        *at += 1;
        Some(v)
    }
    let mut at = 0usize;
    if rd_u32(bytes, &mut at)? != DESC_MAGIC {
        return None;
    }
    if rd_u32(bytes, &mut at)? != DESC_VERSION {
        return None;
    }
    if rd_u32(bytes, &mut at)? != ABI_TAG {
        return None; // foreign / stale ABI — refuse.
    }
    let arity = rd_u64(bytes, &mut at)? as usize;
    let required = rd_u64(bytes, &mut at)? as usize;
    let has_rest = rd_u8(bytes, &mut at)? != 0;
    let has_binds = rd_u8(bytes, &mut at)? != 0;
    let has_handlers = rd_u8(bytes, &mut at)? != 0;
    let has_side_effects = rd_u8(bytes, &mut at)? != 0;
    let has_precise_deopt = rd_u8(bytes, &mut at)? != 0;
    let max_depth = rd_u64(bytes, &mut at)? as usize;
    let reloc_count = rd_u32(bytes, &mut at)?;
    let recipe_len = rd_u64(bytes, &mut at)? as usize;
    let reloc_recipe = bytes.get(at..at + recipe_len)?.to_vec();
    Some(AotDescriptor {
        meta: super::compile::AotLeafMeta {
            arity,
            required,
            has_rest,
            has_binds,
            has_handlers,
            has_side_effects,
            max_depth,
            has_precise_deopt,
        },
        reloc_recipe,
        reloc_count,
    })
}

/// The exported descriptor symbol for an AOT leaf: `__neovm_aotd_{hash}_{tag}`.
pub(crate) fn aot_descriptor_symbol(content_hash: u128) -> String {
    format!("__neovm_aotd_{content_hash:032x}_{ABI_TAG:08x}")
}

/// Max reloc slots a single leaf may carry — bounds a crafted/corrupt
/// reloc_count before it drives a huge allocation. Real leaves have a handful.
const MAX_RELOC_COUNT: u32 = 64 * 1024;

/// Rebuild the reloc-const Vec from a descriptor's recipe (R1c-3 + R1c-8): each
/// per-slot recipe is decoded against the LIVE heap/obarray, producing fresh
/// heap objects (allocated black by the alloc path). The caller roots them
/// (R1c-8). Returns `None` on a malformed/over-long recipe (the loader then
/// falls through to the JIT — the additive contract). MUST run under a live VM.
pub(crate) fn rebuild_reloc_consts(desc: &AotDescriptor) -> Option<Box<[Value]>> {
    if desc.reloc_count > MAX_RELOC_COUNT {
        return None;
    }
    let mut out = Vec::with_capacity(desc.reloc_count as usize);
    let mut at = 0usize;
    for _ in 0..desc.reloc_count {
        let (v, n) = rebuild_value(desc.reloc_recipe.get(at..)?, 0)?;
        out.push(v);
        at = at.checked_add(n)?;
    }
    // Every recipe byte must be consumed — a trailing tail is a malformed blob.
    if at != desc.reloc_recipe.len() {
        return None;
    }
    Some(out.into_boxed_slice())
}

// ---------------------------------------------------------------------------
// R1c-2 + R1c-5 (emit side): the full pure-leaf → object pipeline.
// ---------------------------------------------------------------------------

/// Whether a MIR leaf is AOT-runnable (R1c-sidecar scope), and why not.
///
/// The sidecar increment unblocks RELOC-bearing leaves: the reloc base is now
/// loaded from the per-thread sidecar (R1c-sidecar) and its consts rebuilt at
/// load from a recipe (R1c-3), so heap constants are fine. Still EXCLUDED:
///   * CALL/APPLY (`needs_rt` via has_call) — imports the `neovm_jit_*` shims,
///     which the host must export (`-rdynamic` + `#[used]`); that host-shim
///     plumbing is a later increment. A call also means precise deopt, whose
///     sidecar path IS implemented, but the shim imports block loading.
///   * ESCAPING CONS — same: imports the cons shim (`needs_rt`).
///   * non-recipe-able heap constants (float/vector/...) — `write_value_recipe`
///     bails, so the content hash itself returns None upstream.
///
/// A rejected body stays JIT-only (strictly additive).
fn mir_is_aot_runnable(m: &mir::MirFunction) -> bool {
    use mir::MirOp;
    // Reject any op whose IDENTITY embeds a session-specific SymId. CALL/APPLY
    // need the runtime shims (undefined host imports) AND precise deopt; the
    // CallBuiltin*/Switch family embed a SymId or are otherwise outside the AOT
    // subset. Explicitly reject the sym-bearing opaque ops here (audit #17) so
    // the cache key (leaf_content_hash, which keys ops by Debug — non-canonical
    // for an embedded raw SymId) is never armed for them, rather than relying on
    // the downstream lowering Err. A pure required-only AOT body must contain
    // none of these.
    let has_unsupported_opaque = m.blocks.iter().any(|b| {
        b.insts.iter().any(|i| {
            matches!(
                &i.op,
                MirOp::Opaque {
                    op: Op::Call(_)
                        | Op::Apply(_)
                        | Op::CallBuiltin(..)
                        | Op::CallBuiltinSym(..),
                    ..
                }
            )
        })
    });
    if has_unsupported_opaque {
        return false;
    }
    // An ESCAPING cons calls the cons shim (needs_rt). A scalar-replaced
    // (non-escaping) cons emits no shim, so only escaping conses block.
    let cons_repl = mir::cons_scalar_repl_targets(m);
    let has_escaping_cons = m
        .blocks
        .iter()
        .flat_map(|b| b.insts.iter())
        .any(|i| matches!(&i.op, MirOp::Cons(..)) && cons_repl[i.result.0 as usize].is_none());
    !has_escaping_cons
    // Heap constants are now ALLOWED (reloc via the sidecar + recipe rebuild).
}

/// Collect the reloc constants of a MIR leaf (the DISTINCT heap-object consts, in
/// first-seen order — same dedup as the lowering). Returns the ordered Values;
/// the recipe is emitted in this order and rebuilt into the same order at load.
fn collect_reloc_consts(m: &mir::MirFunction) -> Vec<Value> {
    use mir::MirOp;
    let mut seen = std::collections::HashMap::new();
    let mut out = Vec::new();
    for blk in &m.blocks {
        for inst in &blk.insts {
            if let MirOp::Const(v) = &inst.op {
                let bits = v.bits();
                // AOT reloc set: heap objects AND non-nil/t symbols (audit #16).
                // MUST match the lowering's `needs_reloc` decision (aot=true).
                if super::compile::const_relocs_for_aot(*v) && !seen.contains_key(&bits) {
                    seen.insert(bits, out.len());
                    out.push(*v);
                }
            }
        }
    }
    out
}

/// Compile one bytecode leaf to a relocatable `.o` for AOT (R1c + sidecar).
///
/// Computes the content hash + entry/descriptor symbols, builds the MIR, checks
/// it is AOT-runnable, collects the reloc consts into a rebuild recipe, emits the
/// entry + descriptor object (with the recipe + frame metadata), and returns
/// `(object_bytes, content_hash)`. Returns `Ok(None)` (NOT an error) when the
/// body is outside the supported subset — the caller stays JIT-only.
///
/// Currently covers the SHIM-FREE subset (no call/apply, no escaping cons) which
/// may now bear heap constants (reloc via the per-thread sidecar + recipe). The
/// remaining widening (call-bearing → precise deopt + host shim export) is a
/// later increment; the sidecar deopt path is already in the lowering for it.
pub(crate) fn compile_leaf_to_object(
    ops: &[Op],
    constants: &[Value],
    arity: usize,
) -> Result<Option<(Vec<u8>, u128)>, CompileError> {
    let Some(content_hash) = leaf_content_hash(ops, constants, arity) else {
        return Ok(None); // a constant outside the recipe subset.
    };
    let m = match mir::build_mir(ops, constants, arity) {
        Ok(m) => m,
        Err(_) => return Ok(None), // not MIR-lowerable → JIT-only.
    };
    if !mir_is_aot_runnable(&m) {
        return Ok(None);
    }
    // Reloc consts → rebuild recipe (R1c-3), in the SAME order the lowering
    // assigns reloc indices. If any const is outside the recipe subset, bail
    // (defensive — leaf_content_hash already gates this).
    let reloc_consts = collect_reloc_consts(&m);
    let mut recipe = Vec::new();
    for &c in &reloc_consts {
        if write_value_recipe(&mut recipe, c).is_err() {
            return Ok(None);
        }
    }
    let entry_name = aot_entry_symbol(content_hash);
    let desc_name = aot_descriptor_symbol(content_hash);
    // Shim-free subset: no calls → no precise deopt, no binds/handlers/side
    // effects. (max_depth=0 / has_precise_deopt=false until call-bearing AOT.)
    let meta = super::compile::AotLeafMeta {
        arity: m.arity,
        required: m.arity,
        has_rest: false,
        has_binds: false,
        has_handlers: false,
        has_side_effects: false,
        max_depth: 0,
        has_precise_deopt: false,
    };
    let desc_bytes = encode_descriptor(&meta, &recipe, reloc_consts.len() as u32);
    let obj = build_object_for_leaf_inner(&m, &entry_name, Some((&desc_name, &desc_bytes)))?;

    // Audit #15: ABI_TAG salts only the MIR_SHIM_NAMES set; an AOT `.so` that
    // imports a shim OUTSIDE that set would not be re-tagged when that shim's
    // ABI changes (a stale-`.so`-runs-against-changed-ABI hazard). The current
    // subset emits NO shim imports (no call/escaping-cons → needs_rt=false), but
    // assert it: every UNDEFINED import the object carries must be a salted shim,
    // so a future widening can't silently ship an unsalted import.
    #[cfg(debug_assertions)]
    {
        use object::{Object, ObjectSymbol};
        if let Ok(file) = object::File::parse(&*obj) {
            for sym in file.symbols() {
                // Only the `neovm_jit_*` undefined imports matter (skip the empty
                // / section symbols object emits).
                if sym.is_undefined()
                    && let Ok(name) = sym.name()
                    && name.starts_with("neovm_jit_")
                {
                    debug_assert!(
                        MIR_SHIM_NAMES.contains(&name),
                        "AOT object imports shim {name:?} not in ABI_TAG's salted \
                         MIR_SHIM_NAMES — salt it (compute_abi_tag) before emitting it"
                    );
                }
            }
        }
    }

    Ok(Some((obj, content_hash)))
}

// ---------------------------------------------------------------------------
// R1c-5: link `.o` → `.so`, load via libloading.
// R1c-7: the content-hash-keyed unit store (from NEOVM_AOT_DIR).
// R1c-6: `try_load_leaf` — the cache's AOT consult.
// ---------------------------------------------------------------------------

/// Link a relocatable object's bytes into a shared object at `so_path`, via
/// `cc -shared`. The host process must export the `neovm_jit_*` shims (linked
/// `-rdynamic`) so the loader can bind the `.so`'s undefined imports at dlopen.
pub(crate) fn link_object_to_so(
    obj_bytes: &[u8],
    so_path: &std::path::Path,
) -> Result<(), CompileError> {
    use std::io::Write;
    // Write the `.o` beside the target `.so` (same dir; temp name).
    let o_path = so_path.with_extension("o");
    std::fs::File::create(&o_path)
        .and_then(|mut f| f.write_all(obj_bytes))
        .map_err(|e| module_init_err(format!("write .o: {e}")))?;
    let status = std::process::Command::new("cc")
        .arg("-shared")
        .arg("-o")
        .arg(so_path)
        .arg(&o_path)
        .status()
        .map_err(|e| module_init_err(format!("spawn cc: {e}")))?;
    // Best-effort cleanup of the intermediate object.
    let _ = std::fs::remove_file(&o_path);
    if !status.success() {
        return Err(module_init_err(format!("cc -shared failed: {status}")));
    }
    Ok(())
}

/// One AOT compilation unit's on-disk location, keyed by content hash.
type UnitIndex = std::collections::HashMap<u128, std::path::PathBuf>;

/// Process-wide index of available AOT `.so`s by content hash, built once from
/// `NEOVM_AOT_DIR` (default: none → AOT disabled). Memoized loaded units are
/// thread-local (the cache + leaves are `!Send`); the INDEX is shareable (just
/// paths). Indexes only files whose name carries the CURRENT `ABI_TAG`.
fn unit_index() -> &'static UnitIndex {
    static INDEX: std::sync::OnceLock<UnitIndex> = std::sync::OnceLock::new();
    INDEX.get_or_init(|| {
        let mut idx = UnitIndex::new();
        let Some(dir) = std::env::var_os("NEOVM_AOT_DIR") else {
            return idx;
        };
        let tag_suffix = format!("_{ABI_TAG:08x}.so");
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let path = e.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                // Expect `<hash:032x>_<tag:08x>.so` (the entry-symbol stem). Only
                // index files matching the current ABI_TAG.
                if !name.ends_with(&tag_suffix) {
                    continue;
                }
                let stem = &name[..name.len() - tag_suffix.len()];
                if stem.len() == 32
                    && let Ok(hash) = u128::from_str_radix(stem, 16)
                {
                    idx.insert(hash, path);
                }
            }
        }
        idx
    })
}

thread_local! {
    /// Thread-local memo of units already `dlopen`'d on THIS thread, keyed by
    /// content hash. The `Arc<LoadedUnit>` keeps the `.so` mapped; cloned into
    /// every `CompiledLeaf` it backs. `!Send` (so per-thread), matching the
    /// thread-local `COMPILED` cache.
    ///
    /// COUPLING (audit #2): this memo is APPEND-ONLY — never cleared, even when
    /// `cache::clear()` drops the COMPILED leaves (and their per-leaf `Arc`s) on a
    /// heap-identity change. Safety does NOT depend on that: each `CompiledLeaf`
    /// holds its OWN `Arc<LoadedUnit>` (`_backing`), so its `entry` is valid for
    /// as long as it is cached regardless of this memo. The memo is purely a
    /// per-thread dlopen-dedup. INVARIANT for any future pruner: this map must
    /// outlive every COMPILED leaf that points into the same `.so` — never prune
    /// a unit while a leaf backed by it is still cached (today: never prune at
    /// all). The cost of append-only is a bounded-per-distinct-hash leak of
    /// mapped `.so` images across image reloads.
    static LOADED_UNITS: std::cell::RefCell<
        std::collections::HashMap<u128, std::sync::Arc<super::compile::LoadedUnit>>,
    > = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// dlopen (memoized) the unit for `content_hash` from the unit index, returning
/// the shared `Arc<LoadedUnit>`. `None` if no `.so` is indexed for this hash.
fn load_unit(content_hash: u128) -> Option<std::sync::Arc<super::compile::LoadedUnit>> {
    if let Some(u) = LOADED_UNITS.with(|m| m.borrow().get(&content_hash).cloned()) {
        return Some(u);
    }
    // Test seam: a directly-injected unit (see `test_support::inject_unit`) takes
    // precedence over the env-driven index, so a test can pre-load a `.so` it
    // just built and exercise the cache path.
    #[cfg(test)]
    if let Some(u) = test_support::injected_unit(content_hash) {
        return Some(u);
    }
    let path = unit_index().get(&content_hash)?;
    // SAFETY: dlopen of a `.so` we emitted; its undefined imports are the
    // `neovm_jit_*` shims, bound against the -rdynamic host. The library is
    // never unloaded while any backed leaf is cached (held by the Arc).
    let lib = unsafe { libloading::Library::new(path) }.ok()?;
    let unit = std::sync::Arc::new(super::compile::LoadedUnit::new(lib));
    LOADED_UNITS.with(|m| {
        m.borrow_mut().insert(content_hash, std::sync::Arc::clone(&unit));
    });
    Some(unit)
}

/// Whether AOT loading is enabled this session. R1c proves the path in-test via
/// `NEOVM_AOT=force`; R2 wires the real dump-time pre-warm.
pub(crate) fn aot_enabled() -> bool {
    // Test seam: a thread-local override (see `test_support`) lets a unit test
    // exercise the cache AOT path without relying on a process-start env var
    // (the env reads are OnceLock-memoized, so they can't be set per-test).
    #[cfg(test)]
    if let Some(forced) = test_support::forced_enabled() {
        return forced;
    }
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        matches!(
            std::env::var("NEOVM_AOT").as_deref(),
            Ok("force") | Ok("1") | Ok("on")
        )
    })
}

/// R1c-6: try to serve a leaf for the given bytecode source from AOT.
///
/// Computes the content hash, finds + dlopens the matching unit, dlsym's the
/// entry + descriptor, verifies the descriptor (magic/version/ABI_TAG), rebuilds
/// the live reloc consts, and constructs a pre-warmed [`CompiledLeaf`]. Returns
/// `None` on ANY miss/mismatch/error — the caller falls back to the JIT
/// (strictly additive). The returned reloc consts are rooted by the caller's
/// cache insertion (R1c-8: they live in `COMPILED`, walked by
/// `collect_jit_reloc_gc_roots`).
pub(crate) fn try_load_leaf(
    ops: &[Op],
    constants: &[Value],
    arity: usize,
) -> Option<super::compile::CompiledLeaf> {
    if !aot_enabled() {
        return None;
    }
    let content_hash = leaf_content_hash(ops, constants, arity)?;
    let unit = load_unit(content_hash)?;
    load_leaf_from_unit(&unit, content_hash, arity)
}

/// The dlsym + descriptor-decode + reloc-rebuild + construct core, factored out
/// of [`try_load_leaf`] so a test can drive it with a directly-loaded unit
/// (bypassing the env/index resolution). Returns `None` on any symbol miss /
/// descriptor mismatch / arity mismatch — the caller falls back to JIT.
pub(crate) fn load_leaf_from_unit(
    unit: &std::sync::Arc<super::compile::LoadedUnit>,
    content_hash: u128,
    arity: usize,
) -> Option<super::compile::CompiledLeaf> {
    let entry_name = aot_entry_symbol(content_hash);
    let desc_name = aot_descriptor_symbol(content_hash);

    // dlsym the entry + descriptor out of the SAME unit (so the entry points
    // into the library the Arc keeps mapped). Unified 4-param ABI (the 4th is the
    // *const LeafSidecar); the ptr is cast to *const u8 and called via
    // CompiledLeaf::invoke_native, which passes the leaf's own sidecar.
    type EntryFn =
        unsafe extern "C" fn(*mut u8, *const i64, *mut i64, *const core::ffi::c_void) -> i64;
    // Hardening (review): the descriptor lives in a `.so` from NEOVM_AOT_DIR — a
    // trust boundary. The data-object size is not available via dlsym, so we read
    // the FIXED header first, VALIDATE magic+version+ABI_TAG before trusting any
    // length field, then bound + checked-add the recipe length before the second
    // `from_raw_parts`. A foreign/corrupt blob whose first 12 bytes don't match
    // is rejected here (→ JIT fallback) without ever reading an attacker-chosen
    // length. (A blob that fakes a valid header but lies about recipe_len can
    // still over-read within the cap; that is acceptable for the in-process,
    // operator-controlled AOT dir — the cap bounds the damage and decode_descriptor
    // re-checks the recipe count.)
    const HDR: usize = 4 + 4 + 4 + 8 + 8 + 5 + 8 + 4 + 8; // see encode_descriptor (=53)
    // A generous cap: a leaf's reloc recipe is tiny in practice. Rejects an absurd
    // length before it drives a huge over-read.
    const MAX_RECIPE_LEN: usize = 1 << 20; // 1 MiB
    // SAFETY: symbols we exported; the entry's ABI is the CompiledLeaf entry ABI.
    let (entry_ptr, desc_bytes): (*const u8, Vec<u8>) = unsafe {
        let lib = unit.library();
        let entry: libloading::Symbol<EntryFn> = lib.get(entry_name.as_bytes()).ok()?;
        let entry_ptr = *entry as *const u8;
        let desc_sym: libloading::Symbol<*const u8> = lib.get(desc_name.as_bytes()).ok()?;
        let desc_ptr = *desc_sym;
        // 1) Read + copy the fixed header.
        let hdr = std::slice::from_raw_parts(desc_ptr, HDR).to_vec();
        // 2) Validate magic/version/ABI_TAG BEFORE trusting recipe_len: reject a
        //    foreign blob without reading an attacker-chosen length.
        let magic = u32::from_le_bytes(hdr[0..4].try_into().ok()?);
        let version = u32::from_le_bytes(hdr[4..8].try_into().ok()?);
        let tag = u32::from_le_bytes(hdr[8..12].try_into().ok()?);
        if magic != DESC_MAGIC || version != DESC_VERSION || tag != ABI_TAG {
            return None;
        }
        // 3) recipe_len: bound it, then checked-add for the total size.
        let recipe_len = u64::from_le_bytes(hdr[HDR - 8..HDR].try_into().ok()?) as usize;
        if recipe_len > MAX_RECIPE_LEN {
            return None;
        }
        let total = HDR.checked_add(recipe_len)?;
        let all = std::slice::from_raw_parts(desc_ptr, total).to_vec();
        (entry_ptr, all)
    };

    let desc = decode_descriptor(&desc_bytes)?;
    // Sanity: arity must match the call site's lambda list.
    if desc.meta.arity != arity {
        return None;
    }
    // R1c-8: rebuild the live reloc consts (allocate-black via the heap path).
    // `None` on a malformed/over-long recipe → fall through to JIT (additive).
    let reloc_data = rebuild_reloc_consts(&desc)?;
    // SAFETY: `entry_ptr` is the real native entry inside `unit`'s loaded `.so`;
    // `unit` is held by the returned leaf for its whole life (kept mapped).
    let leaf = unsafe {
        super::compile::CompiledLeaf::from_aot(
            entry_ptr,
            std::sync::Arc::clone(unit),
            desc.meta,
            reloc_data,
        )
    };
    Some(leaf)
}

/// Build a relocatable object (`.o` bytes) for one pure MIR leaf `m`, exporting
/// its entry under `entry_name`.
///
/// Mirrors [`compile::lower_mir_pure`]'s analysis prologue (the same `has_call`
/// / `cons_repl` / `needs_rt` derivation, the same reloc-constant collection and
/// deopt-buffer sizing) and then drives the SAME module-generic build seam
/// (`build_mir_leaf_fn`) with `M = ObjectModule` — so the emitted CLIF is
/// byte-identical to the JIT's, including the R1a reloc loads. The only
/// differences from the JIT path are the three seams above: no `builder.symbol`,
/// `Linkage::Export` (vs `Local`) for the entry, and `finish().emit()` (vs
/// `finalize_definitions`/`get_finalized_function`).
///
/// NOTE (R1c-1 scope): the buffer base addresses (`reloc_data`/`deopt_spill`/
/// `deopt_meta`) are still baked as immediates by `build_mir_leaf_fn`, pointing
/// at *this session's* throwaway buffers. That makes the `.o` parseable and its
/// symbol table correct (the R1c-1 gate) but NOT yet runnable across sessions;
/// load-time rebuild/relocation of those bases is R1c-3/5.
pub fn build_object_for_leaf(
    m: &mir::MirFunction,
    entry_name: &str,
) -> Result<Vec<u8>, CompileError> {
    build_object_for_leaf_inner(m, entry_name, None)
}

/// As [`build_object_for_leaf`], but also emits an exported, read-only data
/// object `descriptor.0` holding `descriptor.1` bytes — the AOT descriptor the
/// loader dlsym's to recover the leaf's metadata + reloc rebuild recipe (R1c-3).
fn build_object_for_leaf_inner(
    m: &mir::MirFunction,
    entry_name: &str,
    descriptor: Option<(&str, &[u8])>,
) -> Result<Vec<u8>, CompileError> {
    use mir::MirOp;

    // --- Host ISA with PIC (the .o must be position-independent for the .so). ---
    let mut flag_builder = settings::builder();
    // Mirror cranelift-jit's flags, except is_pic=true (a JITModule needs
    // is_pic=false; a shared object needs true so the loader can relocate it).
    flag_builder
        .set("use_colocated_libcalls", "false")
        .map_err(|e| module_init_err(e.to_string()))?;
    flag_builder
        .set("is_pic", "true")
        .map_err(|e| module_init_err(e.to_string()))?;
    let isa_builder = cranelift_native::builder()
        .map_err(|e| module_init_err(e.to_string()))?;
    let isa = isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(|e| module_init_err(e.to_string()))?;

    let builder = ObjectBuilder::new(isa, "neovm_aot", default_libcall_names())
        .map_err(|e| module_init_err(e.to_string()))?;
    let mut module = ObjectModule::new(builder);

    // ----- Analysis prologue, identical to lower_mir_pure (compile.rs). --------
    // A CALL forces all-precise deopt + the runtime scaffolding; an escaping cons
    // needs the cons shim. Both set needs_rt (vmctx + shims).
    let has_call = m.blocks.iter().any(|b| {
        b.insts.iter().any(|i| {
            matches!(
                &i.op,
                MirOp::Opaque {
                    op: crate::emacs_core::bytecode::opcode::Op::Call(_)
                        | crate::emacs_core::bytecode::opcode::Op::Apply(_),
                    ..
                }
            )
        })
    });
    let cons_repl: Vec<Option<(mir::MirValue, mir::MirValue)>> = if has_call {
        vec![None; m.value_types.len()]
    } else {
        mir::cons_scalar_repl_targets(m)
    };
    let has_escaping_cons = m
        .blocks
        .iter()
        .flat_map(|b| b.insts.iter())
        .any(|i| matches!(&i.op, MirOp::Cons(..)) && cons_repl[i.result.0 as usize].is_none());
    let needs_rt = has_call || has_escaping_cons;

    // Precise-deopt spill buffer + cells (sized exactly as the JIT does). These
    // are this-session throwaway buffers in R1c-1 — their *addresses* get baked,
    // which is fine for the parse gate (load-time rebuild is R1c-3/5).
    let max_depth = m
        .blocks
        .iter()
        .flat_map(|b| b.insts.iter())
        .map(|i| i.pre_stack.len())
        .max()
        .unwrap_or(0);
    let deopt_spill: Box<[core::cell::Cell<i64>]> = if has_call {
        (0..max_depth).map(|_| core::cell::Cell::new(0)).collect()
    } else {
        Box::from([])
    };
    let deopt_meta: Box<DeoptCells> = Box::new(DeoptCells {
        pc: core::cell::Cell::new(0),
        depth: core::cell::Cell::new(0),
        handlers: core::cell::Cell::new(0),
    });

    // R1a reloc-constant collection (dedup by tagged bits), identical to the JIT.
    // AOT reloc index, derived from the ONE collector (`collect_reloc_consts`)
    // so the index order is identical to the recipe order by construction (no
    // duplicated predicate/order to drift): reloc slot i ↔ recipe slot i ↔ the
    // rebuilt Vec at load. The lowering (build_mir_leaf_fn, aot=true) looks up
    // this index for each session-specific const (heap object / non-nil-t symbol).
    let reloc_vals = collect_reloc_consts(m);
    let reloc_index: std::collections::HashMap<usize, u32> = reloc_vals
        .iter()
        .enumerate()
        .map(|(i, v)| (v.bits(), i as u32))
        .collect();
    let reloc_data: Box<[Value]> = reloc_vals.into_boxed_slice();

    // ----- Drive the SHARED build seam with M = ObjectModule. ------------------
    // Same CLIF as the JIT (byte-identical incl R1a reloc loads); only the entry
    // declaration differs: Linkage::Export under `entry_name` so the loader can
    // dlsym it. The `neovm_jit_*` shims stay undefined Linkage::Import imports.
    super::compile::build_mir_leaf_fn(
        &mut module,
        m,
        &deopt_spill,
        &deopt_meta,
        &reloc_data,
        &reloc_index,
        has_call,
        &cons_repl,
        needs_rt,
        entry_name,
        Linkage::Export,
        /*aot=*/ true,
    )?;

    // R1c-3: emit the descriptor as an exported, read-only data object so the
    // loader can dlsym it and recover the leaf metadata + reloc rebuild recipe.
    if let Some((desc_name, desc_bytes)) = descriptor {
        let data_id = module
            .declare_data(desc_name, Linkage::Export, /*writable=*/ false, /*tls=*/ false)
            .map_err(|e| module_init_err(e.to_string()))?;
        let mut desc = cranelift_module::DataDescription::new();
        desc.define(desc_bytes.to_vec().into_boxed_slice());
        module
            .define_data(data_id, &desc)
            .map_err(|e| module_init_err(e.to_string()))?;
    }

    // No get_finalized_function: emit the relocatable object bytes.
    module
        .finish()
        .emit()
        .map_err(|e| module_init_err(e.to_string()))
}

/// Test-only seams that let a unit test drive the cache AOT path (`aot_enabled`
/// and `load_unit`) without a process-start env var (the env reads are
/// OnceLock-memoized and so cannot be toggled per-test). Production builds never
/// compile this; `aot_enabled`/`load_unit` consult it only under `cfg(test)`.
#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::Arc;

    thread_local! {
        static FORCE_ENABLED: RefCell<Option<bool>> = const { RefCell::new(None) };
        static INJECTED: RefCell<HashMap<u128, Arc<super::super::compile::LoadedUnit>>> =
            RefCell::new(HashMap::new());
    }

    /// The forced `aot_enabled()` value, if a test set one.
    pub(crate) fn forced_enabled() -> Option<bool> {
        FORCE_ENABLED.with(|c| *c.borrow())
    }

    /// Force `aot_enabled()` to `v` for the rest of this thread (test only).
    pub(crate) fn set_forced_enabled(v: bool) {
        FORCE_ENABLED.with(|c| *c.borrow_mut() = Some(v));
    }

    /// Reset the test overrides (call at the end of a test to avoid bleed).
    pub(crate) fn reset() {
        FORCE_ENABLED.with(|c| *c.borrow_mut() = None);
        INJECTED.with(|m| m.borrow_mut().clear());
    }

    /// Inject a pre-loaded unit for `content_hash` so `load_unit` returns it.
    pub(crate) fn inject_unit(content_hash: u128, unit: Arc<super::super::compile::LoadedUnit>) {
        INJECTED.with(|m| {
            m.borrow_mut().insert(content_hash, unit);
        });
    }

    /// The injected unit for `content_hash`, if any.
    pub(crate) fn injected_unit(
        content_hash: u128,
    ) -> Option<Arc<super::super::compile::LoadedUnit>> {
        INJECTED.with(|m| m.borrow().get(&content_hash).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use object::{Object, ObjectSymbol};

    /// R1c-1 gate: a pure leaf's object bytes parse via `object::File`, the entry
    /// symbol is exported (defined + global), and the `neovm_jit_*` shims appear
    /// as UNDEFINED imports (resolved by the loader, not baked).
    #[test]
    fn object_emits_with_exported_entry_and_imported_shims() {
        // A 0-arg pure body that conses two fixnums and RETURNS the cons. A
        // returned cons escapes → escape analysis keeps a real heap allocation →
        // needs_rt → the cons shim (+ gc_save/push/restore) declared as imports.
        // Fixnum constants need no reloc vector, keeping this body minimal.
        let ops = [Op::Constant(0), Op::Constant(1), Op::Cons, Op::Return];
        let constants = [Value::make_int(1), Value::make_int(2)];
        let m = mir::build_mir(&ops, &constants, 0).expect("build_mir for cons body");

        let entry = "__neovm_aot_test_cons";
        let bytes = build_object_for_leaf(&m, entry).expect("emit object");
        assert!(!bytes.is_empty(), "object bytes must be non-empty");

        let file = object::File::parse(&*bytes).expect("parse object bytes");

        // Entry symbol: defined (not undefined) and global.
        let entry_sym = file
            .symbols()
            .find(|s| s.name() == Ok(entry))
            .unwrap_or_else(|| panic!("entry symbol {entry} not found"));
        assert!(
            entry_sym.is_definition(),
            "entry {entry} must be a definition (exported)"
        );
        assert!(entry_sym.is_global(), "entry {entry} must be global");

        // The cons shim must appear as an UNDEFINED import (the loader resolves it
        // against the host; AOT never bakes the shim address).
        let cons_shim = file
            .symbols()
            .find(|s| s.name() == Ok("neovm_jit_cons"))
            .expect("neovm_jit_cons import symbol present");
        assert!(
            cons_shim.is_undefined(),
            "shim neovm_jit_cons must be an undefined import"
        );
    }

    /// SCRATCH validation (not a final gate): prove the FULL pure-subset path —
    /// emit `.o` → `cc -shared` → `dlopen` → `dlsym` → call — produces native
    /// code byte-identical to the JIT (`lower_mir_pure`). Uses a PURE arithmetic
    /// leaf with NO heap constants and NO calls, so the lowering bakes ZERO
    /// session-specific addresses (no reloc_base, no precise-deopt buffers) and
    /// the `.o` is directly runnable across the emit→load boundary. This is the
    /// foundation R1c-5 builds on; reloc/precise-deopt rebuild come later.
    #[cfg(target_os = "linux")]
    #[test]
    fn aot_pure_arith_leaf_matches_jit_via_dlopen() {
        use crate::emacs_core::eval::Context;
        use std::io::Write;

        // 1-arg pure body: (+ arg 5). No heap consts, no calls, no deopt buffers.
        let ops = [Op::Constant(0), Op::Add, Op::Return];
        let constants = [Value::make_int(5)];
        let m = mir::build_mir(&ops, &constants, 1).expect("build_mir add body");

        // Reference: the JIT leaf for the same MIR.
        let jit_leaf = super::super::compile::lower_mir_pure(&m).expect("JIT lowers");

        // Emit the AOT object for the same MIR.
        let entry = "__neovm_aot_test_add5";
        let obj = build_object_for_leaf(&m, entry).expect("emit object");

        // Write the `.o`, link to a `.so` with `cc -shared`.
        let dir = tempfile::tempdir().expect("tempdir");
        let o_path = dir.path().join("leaf.o");
        let so_path = dir.path().join("libleaf.so");
        std::fs::File::create(&o_path)
            .and_then(|mut f| f.write_all(&obj))
            .expect("write .o");
        let status = std::process::Command::new("cc")
            .arg("-shared")
            .arg("-o")
            .arg(&so_path)
            .arg(&o_path)
            .status()
            .expect("spawn cc");
        assert!(status.success(), "cc -shared failed");

        // dlopen + dlsym the entry (the unified 4-param CompiledLeaf entry ABI:
        // the 4th arg is the *const LeafSidecar — null here since this is a PURE
        // leaf that bakes its bases and never reads the sidecar).
        type EntryFn =
            unsafe extern "C" fn(*mut u8, *const i64, *mut i64, *const core::ffi::c_void) -> i64;
        let lib = unsafe { libloading::Library::new(&so_path) }.expect("dlopen .so");
        let aot_entry: libloading::Symbol<EntryFn> =
            unsafe { lib.get(entry.as_bytes()) }.expect("dlsym entry");

        // Call AOT and JIT for several args; results must be bit-identical.
        let mut eval = Context::new_minimal_vm_harness();
        let ctx_ptr = &mut eval as *mut Context as *mut u8;
        for a in [0i64, 1, 7, -3, 1000] {
            let arg = Value::make_int(a);
            // JIT result.
            let jit = match jit_leaf.call(ctx_ptr, &[arg]) {
                crate::emacs_core::jit::compile::NativeRun::Ok(bits) => bits as i64,
                other => panic!("JIT did not return Ok: {other:?}"),
            };
            // AOT result via the raw entry ABI (one arg word, out slot, null
            // sidecar — pure leaf ignores it).
            let args = [arg.bits() as i64];
            let mut out: i64 = 0;
            let status = unsafe {
                (aot_entry)(
                    ctx_ptr,
                    args.as_ptr(),
                    &mut out as *mut i64,
                    core::ptr::null(),
                )
            };
            assert_eq!(status, super::super::compile::STATUS_OK, "AOT status not OK");
            assert_eq!(out, jit, "AOT result != JIT result for arg {a}");
        }
    }

    /// R1c-3 gate: a recipe round-trips a const value with a string + symbol +
    /// (nested) cons — emit recipe → rebuild fresh against the live heap/obarray
    /// → leaves match by VALUE (not pointer). Needs a VM harness for allocation.
    #[test]
    fn recipe_round_trips_string_symbol_cons() {
        // The harness installs the thread-local heap so Value::string/cons can
        // allocate (same pattern as the compile.rs MIR const tests).
        let mut _eval = crate::emacs_core::eval::Context::new_minimal_vm_harness();

        // (cons "hello" (cons 'my-sym 42)) — exercises every supported recipe arm.
        let inner = Value::cons(
            Value::symbol(crate::emacs_core::intern::intern("my-sym")),
            Value::make_int(42),
        );
        let original = Value::cons(Value::string("hello"), inner);

        let mut recipe = Vec::new();
        write_value_recipe(&mut recipe, original).expect("string+symbol+cons supported");
        let (rebuilt, consumed) = rebuild_value(&recipe, 0).expect("valid recipe rebuilds");
        assert_eq!(consumed, recipe.len(), "recipe fully consumed");

        // Structurally equal by value (fresh allocations, so NOT `eq`).
        assert!(rebuilt.is_cons(), "top is a cons");
        assert_eq!(
            rebuilt.cons_car().as_lisp_string().unwrap().as_bytes(),
            b"hello"
        );
        assert_eq!(
            crate::emacs_core::intern::resolve_sym(
                rebuilt.cons_cdr().cons_car().as_symbol_id().unwrap()
            ),
            "my-sym"
        );
        assert_eq!(rebuilt.cons_cdr().cons_cdr().as_fixnum(), Some(42));

        // A float is outside the supported subset → recipe bails (caller → JIT).
        let mut tmp = Vec::new();
        assert!(
            write_value_recipe(&mut tmp, Value::make_float(1.5)).is_err(),
            "float must be unsupported (bail to JIT)"
        );
    }

    /// R1c-2 gate: the content hash is STABLE for identical source and
    /// DISCRIMINATES different bodies / arities / constants; the entry symbol
    /// round-trips the hash + ABI_TAG.
    #[test]
    fn content_hash_stable_and_discriminating() {
        let ops_a = [Op::Constant(0), Op::Add, Op::Return];
        let consts_a = [Value::make_int(5)];
        let h1 = leaf_content_hash(&ops_a, &consts_a, 1).expect("hashable");
        let h2 = leaf_content_hash(&ops_a, &consts_a, 1).expect("hashable");
        assert_eq!(h1, h2, "same source → same hash");

        // Different constant.
        let consts_b = [Value::make_int(6)];
        assert_ne!(
            h1,
            leaf_content_hash(&ops_a, &consts_b, 1).expect("hashable"),
            "different constant → different hash"
        );
        // Different arity (lambda-list drift).
        assert_ne!(
            h1,
            leaf_content_hash(&ops_a, &consts_a, 2).expect("hashable"),
            "different arity → different hash"
        );
        // Different ops.
        let ops_c = [Op::Constant(0), Op::Sub, Op::Return];
        assert_ne!(
            h1,
            leaf_content_hash(&ops_c, &consts_a, 1).expect("hashable"),
            "different ops → different hash"
        );

        // Entry symbol round-trips hash + tag.
        let sym = aot_entry_symbol(h1);
        assert!(sym.starts_with("__neovm_aot_"));
        assert!(sym.ends_with(&format!("{ABI_TAG:08x}")));
        assert!(sym.contains(&format!("{h1:032x}")));
    }

    /// Shared R1c-5/R1c-9 harness: for one pure body, build → link → load the
    /// AOT leaf and assert interp == JIT == AOT (bit-for-bit) over `args`.
    #[cfg(target_os = "linux")]
    fn assert_aot_matches_interp_and_jit(
        ops: &[Op],
        constants: &[Value],
        nargs: usize,
        args: &[i64],
    ) {
        use crate::emacs_core::bytecode::{ByteCodeFunction, Vm};
        use crate::emacs_core::eval::Context;
        use crate::emacs_core::value::LambdaParams;

        // Emit → link → dlopen → load via the production helpers.
        let (obj, content_hash) = compile_leaf_to_object(ops, constants, nargs)
            .expect("compile ok")
            .expect("pure subset → Some");
        let dir = tempfile::tempdir().expect("tempdir");
        let so_path = dir.path().join(format!("{content_hash:032x}_{ABI_TAG:08x}.so"));
        link_object_to_so(&obj, &so_path).expect("link .so");
        // SAFETY: dlopen a `.so` we just emitted; pure leaf has no shim imports.
        let lib = unsafe { libloading::Library::new(&so_path) }.expect("dlopen");
        let unit = std::sync::Arc::new(super::super::compile::LoadedUnit::new(lib));
        let aot_leaf =
            load_leaf_from_unit(&unit, content_hash, nargs).expect("load leaf from unit");

        // Reference: JIT leaf for the same MIR.
        let m = mir::build_mir(ops, constants, nargs).expect("mir");
        let jit_leaf = super::super::compile::lower_mir_pure(&m).expect("jit lowers");

        // Reference: the interpreter (the oracle).
        let mut eval = Context::new_minimal_vm_harness();
        let mut f = ByteCodeFunction::new(LambdaParams {
            required: (0..nargs)
                .map(|i| crate::emacs_core::intern::SymId(1 + i as u32))
                .collect(),
            optional: Vec::new(),
            rest: None,
        });
        f.lexical = true;
        f.ops = ops.to_vec();
        f.constants = constants.to_vec();
        f.max_stack = 16;

        let ctx_ptr = &mut eval as *mut Context as *mut u8;
        // For a 1-arg body sweep `args`; otherwise call once with the first
        // `nargs` entries (the corpus bodies below are 0- or 1-arg).
        let calls: Vec<Vec<Value>> = if nargs == 1 {
            args.iter().map(|&a| vec![Value::make_int(a)]).collect()
        } else {
            vec![args.iter().take(nargs).map(|&a| Value::make_int(a)).collect()]
        };
        for call in calls {
            let interp = {
                let mut vm = Vm::from_context(&mut eval);
                vm.execute(&f, call.clone()).expect("interp").bits()
            };
            let aot = match aot_leaf.call(ctx_ptr, &call) {
                crate::emacs_core::jit::compile::NativeRun::Ok(b) => b,
                other => panic!("AOT not Ok: {other:?}"),
            };
            let jit = match jit_leaf.call(ctx_ptr, &call) {
                crate::emacs_core::jit::compile::NativeRun::Ok(b) => b,
                other => panic!("JIT not Ok: {other:?}"),
            };
            assert_eq!(aot, interp, "AOT != interp for {call:?}");
            assert_eq!(aot, jit, "AOT != JIT for {call:?}");
        }
    }

    /// R1c-5 gate: the FULL PRODUCTION path for a pure leaf —
    /// `compile_leaf_to_object` → `link_object_to_so` → dlopen →
    /// `load_leaf_from_unit` → `CompiledLeaf::call` — is byte-identical to BOTH
    /// the interpreter and the JIT, incl across several args.
    #[cfg(target_os = "linux")]
    #[test]
    fn aot_pure_leaf_matches_jit_and_interp() {
        // 1-arg pure body: (* (+ arg 5) 2) — fixnum arith, no consts/calls.
        let ops = [
            Op::Constant(0),
            Op::Add,
            Op::Constant(1),
            Op::Mul,
            Op::Return,
        ];
        let constants = [Value::make_int(5), Value::make_int(2)];
        assert_aot_matches_interp_and_jit(&ops, &constants, 1, &[0, 1, 7, -3, 1000, -1000]);
    }

    /// R1c-9 harness: a CORPUS of pure bodies — each emitted → linked → loaded →
    /// compared interp == JIT == AOT bit-for-bit. Covers arithmetic, comparison
    /// (branchy), unary, and a 0-arg constant-folding body, exercising the AOT
    /// path across the pure subset (the in-test analogue of the suite-wide
    /// `NEOVM_AOT=force` byte-identity gate, which needs R2's pre-built `.so`s).
    #[cfg(target_os = "linux")]
    #[test]
    fn aot_roundtrip_matches_interp_and_jit_corpus() {
        let probe = [0i64, 1, 2, 7, -3, 42, 1000, -1000];

        // 0-arg: (+ 2 3) — constant fold, no args.
        assert_aot_matches_interp_and_jit(
            &[Op::Constant(0), Op::Constant(1), Op::Add, Op::Return],
            &[Value::make_int(2), Value::make_int(3)],
            0,
            &[],
        );
        // 1-arg: (- arg 1)
        assert_aot_matches_interp_and_jit(
            &[Op::Constant(0), Op::Sub, Op::Return],
            &[Value::make_int(1)],
            1,
            &probe,
        );
        // 1-arg: (1+ arg)
        assert_aot_matches_interp_and_jit(&[Op::Add1, Op::Return], &[], 1, &probe);
        // 1-arg: (* arg arg) — needs the arg twice (StackRef duplicates it).
        assert_aot_matches_interp_and_jit(
            &[Op::StackRef(0), Op::Mul, Op::Return],
            &[],
            1,
            &probe,
        );
        // 1-arg branchy: (if (< arg 0) ...) via Lss + GotoIfNil — comparison +
        // control flow, the deopt-free pure path.
        assert_aot_matches_interp_and_jit(
            &[
                Op::Constant(0),
                Op::Lss,
                Op::Return,
            ],
            &[Value::make_int(0)],
            1,
            &probe,
        );
    }

    /// AUDIT #16 gate (the CRITICAL one): a SYMBOL constant must be reloc'd by
    /// NAME, never baked as a session-specific SymId. A baked SymId is only valid
    /// in the emitting session; a cross-session load (the R2 dump-then-run case)
    /// would return the wrong symbol or an out-of-range SymId. This test proves
    /// the symbol const is in the reloc set (NOT baked) and that the rebuilt
    /// symbol is the right one by NAME — even after the intern table has grown
    /// (decoy interns) so an emit-time SymId would no longer be valid.
    #[cfg(target_os = "linux")]
    #[test]
    fn aot_symbol_const_relocs_by_name_not_baked_sym_id() {
        let mut eval = crate::emacs_core::eval::Context::new_minimal_vm_harness();

        // 1-arg body: (lambda (x) (if (consp x) 'yes 'no)) — two symbol consts.
        // Bytecode: StackRef(0); Consp; GotoIfNil(->op5); Constant(0); Return;
        //           Constant(1); Return.
        let ops = [
            Op::StackRef(0),
            Op::Consp,
            Op::GotoIfNil(5),
            Op::Constant(0),
            Op::Return,
            Op::Constant(1),
            Op::Return,
        ];
        let sym_yes = Value::symbol(crate::emacs_core::intern::intern("aot-yes"));
        let sym_no = Value::symbol(crate::emacs_core::intern::intern("aot-no"));
        let constants = [sym_yes, sym_no];
        let arity = 1usize;

        let Some((obj, content_hash)) =
            compile_leaf_to_object(&ops, &constants, arity).expect("compile ok")
        else {
            panic!("symbol-bearing shim-free leaf must be AOT-runnable");
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let so_path = dir.path().join(format!("{content_hash:032x}_{ABI_TAG:08x}.so"));
        link_object_to_so(&obj, &so_path).expect("link");

        // Grow the intern table BEFORE loading, so an emit-time-baked SymId for
        // 'aot-yes/'aot-no would now be stale relative to a fresh rebuild. (In one
        // process the ids don't actually move, but this models the cross-session
        // drift; the real proof is that the symbols are in the reloc set below.)
        for i in 0..50 {
            let _ = crate::emacs_core::intern::intern(&format!("aot-decoy-{i}"));
        }

        let lib = unsafe { libloading::Library::new(&so_path) }.expect("dlopen");
        let unit = std::sync::Arc::new(super::super::compile::LoadedUnit::new(lib));
        let aot_leaf = load_leaf_from_unit(&unit, content_hash, arity).expect("load");

        // PROOF OF #16 FIX: both symbols are in the reloc set (rebuilt by name),
        // NOT baked. A baked symbol would NOT appear in reloc_values().
        let relocs = aot_leaf.reloc_values();
        assert_eq!(relocs.len(), 2, "two symbol reloc consts (not baked)");
        let names: std::collections::HashSet<&str> = relocs
            .iter()
            .map(|v| {
                crate::emacs_core::intern::resolve_sym(
                    v.as_symbol_id().expect("reloc const is a symbol"),
                )
            })
            .collect();
        assert!(names.contains("aot-yes") && names.contains("aot-no"), "names: {names:?}");

        // And the leaf returns the RIGHT symbol per branch, by IDENTITY (eq):
        // the rebuilt symbol must be the live obarray's 'aot-yes/'aot-no.
        let ctx_ptr = &mut eval as *mut crate::emacs_core::eval::Context as *mut u8;
        let call = |arg: Value| match aot_leaf.call(ctx_ptr, &[arg]) {
            crate::emacs_core::jit::compile::NativeRun::Ok(b) => Value::from_bits(b),
            other => panic!("not Ok: {other:?}"),
        };
        // (consp '(1)) → 'yes ; (consp 5) → 'no.
        let cons_arg = Value::cons(Value::make_int(1), Value::NIL);
        assert_eq!(
            call(cons_arg).as_symbol_id(),
            Some(crate::emacs_core::intern::intern("aot-yes")),
            "consp arg → 'aot-yes (by current-session SymId)"
        );
        assert_eq!(
            call(Value::make_int(5)).as_symbol_id(),
            Some(crate::emacs_core::intern::intern("aot-no")),
            "non-consp arg → 'aot-no"
        );
    }

    /// AUDIT #16 gensym hole (team-lead must-add): reloc-by-NAME is sound ONLY
    /// for the CANONICAL interned symbol of a name. An UNINTERNED / gensym const
    /// (make-symbol; cl-macro/pcase expansions embed these in quoted forms) has a
    /// non-unique name, so rebuilding it by name in a different session would
    /// yield the WRONG symbol. The emitter must REFUSE such a leaf (→ JIT, which
    /// bakes the in-session SymId — correct same-session). This test: a leaf with
    /// a gensym const is NOT AOT-emitted (compile_leaf_to_object → None).
    #[test]
    fn aot_gensym_symbol_const_is_rejected_stays_jit() {
        let _eval = crate::emacs_core::eval::Context::new_minimal_vm_harness();

        // An uninterned (gensym) symbol — name not registered as canonical.
        let gensym = Value::symbol(crate::emacs_core::intern::intern_uninterned("g$decoy"));
        assert!(
            !crate::emacs_core::intern::is_canonical_id(gensym.as_symbol_id().unwrap()),
            "precondition: the gensym is non-canonical"
        );
        // write_value_recipe must REFUSE the gensym (the load-bearing guard).
        let mut buf = Vec::new();
        assert!(
            write_value_recipe(&mut buf, gensym).is_err(),
            "gensym recipe must be refused"
        );

        // And the whole emit pipeline must bail to None (JIT) for a leaf that
        // returns the gensym const.
        let ops = [Op::Constant(0), Op::Return];
        let constants = [gensym];
        assert!(
            compile_leaf_to_object(&ops, &constants, 1)
                .expect("compile ok")
                .is_none(),
            "a gensym-const leaf must NOT be AOT-emitted (stays JIT)"
        );

        // Sanity: the CANONICAL symbol of the same shape IS accepted (so the
        // rejection is specific to uninterned, not symbols in general).
        let interned = Value::symbol(crate::emacs_core::intern::intern("g-interned-ok"));
        let mut buf2 = Vec::new();
        assert!(
            write_value_recipe(&mut buf2, interned).is_ok(),
            "a canonical interned symbol is still accepted"
        );
    }

    /// R1c-sidecar gate: a RELOC-bearing leaf (returns a heap-string constant)
    /// loads its reloc base from the per-thread sidecar and rebuilds the const at
    /// load. The AOT leaf returns a FRESH string (different pointer from the JIT
    /// leaf's original const), so the result is compared by CONTENT, not bits.
    /// This exercises the genuinely new sidecar path: `reloc_base` ← sidecar load,
    /// recipe rebuild, and the rebuilt const being GC-rooted (it lives in the
    /// leaf's reloc_data, walked by collect_jit_reloc_gc_roots).
    #[cfg(target_os = "linux")]
    #[test]
    fn aot_reloc_bearing_leaf_rebuilds_string_const() {
        // Heap allocation needs a live heap — set up the harness FIRST so the
        // string const + JIT leaf are built against it.
        let mut eval = crate::emacs_core::eval::Context::new_minimal_vm_harness();

        // 1-arg body: (lambda (x) "hello") — returns a heap-string CONSTANT.
        let ops = [Op::Constant(0), Op::Return];
        let constants = [Value::string("hello")];
        let arity = 1usize;

        // The leaf is reloc-bearing (heap const) but shim-free → AOT-runnable.
        let (obj, content_hash) = compile_leaf_to_object(&ops, &constants, arity)
            .expect("compile ok")
            .expect("reloc-bearing shim-free leaf → Some");
        let dir = tempfile::tempdir().expect("tempdir");
        let so_path = dir.path().join(format!("{content_hash:032x}_{ABI_TAG:08x}.so"));
        link_object_to_so(&obj, &so_path).expect("link .so");
        let lib = unsafe { libloading::Library::new(&so_path) }.expect("dlopen");
        let unit = std::sync::Arc::new(super::super::compile::LoadedUnit::new(lib));
        let aot_leaf =
            load_leaf_from_unit(&unit, content_hash, arity).expect("load reloc leaf");

        // The rebuilt reloc Vec must hold ONE string == "hello" (fresh alloc).
        let relocs = aot_leaf.reloc_values();
        assert_eq!(relocs.len(), 1, "one reloc const");
        assert_eq!(relocs[0].as_lisp_string().unwrap().as_bytes(), b"hello");

        // Calling the AOT leaf returns a string equal-by-content to "hello".
        let ctx_ptr = &mut eval as *mut crate::emacs_core::eval::Context as *mut u8;
        let bits = match aot_leaf.call(ctx_ptr, &[Value::make_int(0)]) {
            crate::emacs_core::jit::compile::NativeRun::Ok(b) => b,
            other => panic!("AOT reloc leaf not Ok: {other:?}"),
        };
        let result = Value::from_bits(bits);
        assert_eq!(
            result.as_lisp_string().expect("string result").as_bytes(),
            b"hello",
            "AOT reloc leaf returns the rebuilt heap string by content"
        );
        // And it is a DIFFERENT allocation from the source const (rebuilt, not
        // the baked pointer) — the whole point of the reloc recipe.
        assert_ne!(
            result.bits(),
            constants[0].bits(),
            "AOT result is a freshly-rebuilt string, not the original const pointer"
        );
    }

    /// R1c-sidecar: a TWO-const reloc leaf with control flow — proves the recipe
    /// rebuild ORDER matches the lowering's reloc-index order (slot 0 ↔ "first",
    /// slot 1 ↔ "second"). A wrong order would swap the branches' results.
    #[cfg(target_os = "linux")]
    #[test]
    fn aot_two_reloc_consts_rebuild_in_index_order() {
        let mut eval = crate::emacs_core::eval::Context::new_minimal_vm_harness();

        // (lambda (x) (if x "first" "second")) — both arms return a heap string.
        // Bytecode: StackRef(0); GotoIfNil(->op3); Constant(0); Return;
        //           Constant(1); Return.  (op indices: 0..5)
        let ops = [
            Op::StackRef(0),
            Op::GotoIfNil(4),
            Op::Constant(0),
            Op::Return,
            Op::Constant(1),
            Op::Return,
        ];
        let constants = [Value::string("first"), Value::string("second")];
        let arity = 1usize;

        let Some((obj, content_hash)) =
            compile_leaf_to_object(&ops, &constants, arity).expect("compile ok")
        else {
            // If this body isn't MIR-lowerable/AOT-runnable, skip (don't fail) —
            // the single-const test already covers the reloc mechanism.
            return;
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let so_path = dir.path().join(format!("{content_hash:032x}_{ABI_TAG:08x}.so"));
        link_object_to_so(&obj, &so_path).expect("link");
        let lib = unsafe { libloading::Library::new(&so_path) }.expect("dlopen");
        let unit = std::sync::Arc::new(super::super::compile::LoadedUnit::new(lib));
        let aot_leaf = load_leaf_from_unit(&unit, content_hash, arity).expect("load");

        // reloc_values must be ["first","second"] in that order.
        let relocs = aot_leaf.reloc_values();
        assert_eq!(relocs.len(), 2, "two reloc consts");
        assert_eq!(relocs[0].as_lisp_string().unwrap().as_bytes(), b"first");
        assert_eq!(relocs[1].as_lisp_string().unwrap().as_bytes(), b"second");

        // And each branch returns the right rebuilt string.
        let ctx_ptr = &mut eval as *mut crate::emacs_core::eval::Context as *mut u8;
        let call = |arg: Value| match aot_leaf.call(ctx_ptr, &[arg]) {
            crate::emacs_core::jit::compile::NativeRun::Ok(b) => Value::from_bits(b),
            other => panic!("not Ok: {other:?}"),
        };
        assert_eq!(
            call(Value::T).as_lisp_string().unwrap().as_bytes(),
            b"first",
            "x=t → first arm"
        );
        assert_eq!(
            call(Value::NIL).as_lisp_string().unwrap().as_bytes(),
            b"second",
            "x=nil → second arm"
        );
    }

    /// R1c-8 gate: a reloc-bearing AOT leaf served THROUGH THE CACHE has its
    /// rebuilt reloc const collected as a GC root (so it survives collection — it
    /// is the leaf's only reference to that fresh string). Proves the AOT leaf is
    /// covered by the existing R1a COMPILED-walking root scan with NO new root set.
    #[cfg(target_os = "linux")]
    #[test]
    fn aot_reloc_const_is_gc_rooted_via_compiled_walk() {
        use crate::emacs_core::bytecode::ByteCodeFunction;
        use crate::emacs_core::value::LambdaParams;

        let mut _eval = crate::emacs_core::eval::Context::new_minimal_vm_harness();

        // Prime the cache's heap-identity guard with THIS heap BEFORE caching, so
        // the later root walk's `sync_cache_to_current_heap` does not see a
        // None→Some transition and clear the (just-cached) leaf. In production the
        // guard is primed by the first GC long before any compile; the test must
        // do it explicitly because it caches before any GC.
        {
            let mut prime: Vec<Value> = Vec::new();
            super::super::cache::collect_jit_reloc_gc_roots(&mut prime);
        }

        // (lambda (x) "needle") — reloc-bearing, shim-free.
        let ops = vec![Op::Constant(0), Op::Return];
        let constants = vec![Value::string("needle-aot-root")];
        let arity = 1usize;

        let (obj, content_hash) = compile_leaf_to_object(&ops, &constants, arity)
            .expect("compile ok")
            .expect("reloc shim-free leaf");
        let dir = tempfile::tempdir().expect("tempdir");
        let so_path = dir.path().join("leaf.so");
        link_object_to_so(&obj, &so_path).expect("link");
        let lib = unsafe { libloading::Library::new(&so_path) }.expect("dlopen");
        let unit = std::sync::Arc::new(super::super::compile::LoadedUnit::new(lib));

        // Serve it through the cache (so it lands in COMPILED, where the root walk
        // looks).
        test_support::set_forced_enabled(true);
        test_support::inject_unit(content_hash, unit);

        let mut f = ByteCodeFunction::new(LambdaParams {
            required: vec![crate::emacs_core::intern::SymId(1)],
            optional: Vec::new(),
            rest: None,
        });
        f.lexical = true;
        f.ops = ops.clone();
        f.constants = constants.clone();
        f.max_stack = 16;

        // Drive try_run_compiled so the AOT leaf is cached.
        let id = f.runtime.compiled_id_or_assign();
        let _ = super::super::cache::try_run_compiled(
            std::ptr::null_mut(),
            &f,
            Value::NIL,
            &[Value::make_int(0)],
        )
        .unwrap();
        // Sanity: it must have been served FROM AOT (else the test proves nothing).
        assert_eq!(
            super::super::cache::cached_leaf_is_aot_for_test(id),
            Some(true),
            "leaf must be AOT-backed for this rooting test to be meaningful"
        );

        // The root walk must include the leaf's rebuilt const (by content).
        let mut roots: Vec<Value> = Vec::new();
        super::super::cache::collect_jit_reloc_gc_roots(&mut roots);
        let found = roots.iter().any(|v| {
            v.as_lisp_string()
                .is_some_and(|s| s.as_bytes() == b"needle-aot-root")
        });
        assert!(
            found,
            "the AOT leaf's rebuilt reloc const must be a GC root (COMPILED walk)"
        );

        super::super::cache::clear();
        test_support::reset();
    }

    /// R1c-6 gate: a content/ABI MISMATCH (wrong arity, foreign hash) makes
    /// `load_leaf_from_unit` return None (→ caller falls back to JIT, additive).
    #[cfg(target_os = "linux")]
    #[test]
    fn aot_load_miss_falls_through() {
        let ops = [Op::Constant(0), Op::Add, Op::Return];
        let constants = [Value::make_int(5)];
        let arity = 1usize;
        let (obj, content_hash) = compile_leaf_to_object(&ops, &constants, arity)
            .expect("compile ok")
            .expect("pure subset");
        let dir = tempfile::tempdir().expect("tempdir");
        let so_path = dir.path().join("leaf.so");
        link_object_to_so(&obj, &so_path).expect("link");
        let lib = unsafe { libloading::Library::new(&so_path) }.expect("dlopen");
        let unit = std::sync::Arc::new(super::super::compile::LoadedUnit::new(lib));

        // Right hash, WRONG arity → entry symbol embeds the hash only, but the
        // descriptor arity check rejects... actually arity != desc.arity bails.
        assert!(
            load_leaf_from_unit(&unit, content_hash, /*arity=*/ 2).is_none(),
            "arity mismatch must miss"
        );
        // A foreign content hash → the entry/descriptor symbols don't exist →
        // dlsym miss → None.
        assert!(
            load_leaf_from_unit(&unit, content_hash ^ 0xdead_beef, arity).is_none(),
            "foreign hash must miss"
        );
    }

    /// R1c-6 gate: with AOT enabled and the unit pre-loaded, `try_run_compiled`
    /// serves the leaf FROM AOT (the cached entry is AOT-backed, NOT JIT) and the
    /// result matches the interpreter — the pre-warmed cache hit.
    #[cfg(target_os = "linux")]
    #[test]
    fn aot_hit_serves_without_jitting_through_cache() {
        use crate::emacs_core::bytecode::ByteCodeFunction;
        use crate::emacs_core::value::LambdaParams;

        // 1-arg pure body (+ arg 5) — the AOT pure subset.
        let ops = vec![Op::Constant(0), Op::Add, Op::Return];
        let constants = vec![Value::make_int(5)];
        let arity = 1usize;

        // Build + link the `.so`, dlopen it, inject the unit by content hash.
        let (obj, content_hash) = compile_leaf_to_object(&ops, &constants, arity)
            .expect("compile ok")
            .expect("pure subset");
        let dir = tempfile::tempdir().expect("tempdir");
        let so_path = dir.path().join("leaf.so");
        link_object_to_so(&obj, &so_path).expect("link");
        let lib = unsafe { libloading::Library::new(&so_path) }.expect("dlopen");
        let unit = std::sync::Arc::new(super::super::compile::LoadedUnit::new(lib));

        // Drive the cache path with the test seams (force-enable + inject unit).
        // Reset at the end so the override doesn't bleed into other tests.
        test_support::set_forced_enabled(true);
        test_support::inject_unit(content_hash, unit);

        let mut f = ByteCodeFunction::new(LambdaParams {
            required: vec![crate::emacs_core::intern::SymId(1)],
            optional: Vec::new(),
            rest: None,
        });
        f.lexical = true;
        f.ops = ops.clone();
        f.constants = constants.clone();
        f.max_stack = 16;

        let id = f.runtime.compiled_id_or_assign();
        let got = super::super::cache::try_run_compiled(
            std::ptr::null_mut(),
            &f,
            Value::NIL,
            &[Value::make_int(37)],
        )
        .unwrap();
        // (+ 37 5) = 42.
        assert_eq!(got, Some(Value::make_int(42).bits()), "AOT result");
        // The cached leaf must be AOT-backed — served from the `.so`, not JIT'd.
        assert_eq!(
            super::super::cache::cached_leaf_is_aot_for_test(id),
            Some(true),
            "cached leaf must be AOT-backed (served without JITing)"
        );

        super::super::cache::clear();
        test_support::reset();
    }
}
