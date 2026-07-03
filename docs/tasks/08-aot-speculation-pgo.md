# Task 08 — Speculation-carrying AOT artifacts -> PGO persistence + package native cache

Status: DESIGN QUESTION (unblocks two shelved features). Risk: MEDIUM-HIGH
(cross-session soundness — this subsystem's history says every widening
uncovers a session-baked-value class). Effort: design+critique 1 session,
implementation 1-2.

## 1. Where AOT stands (post-2026-07)

The AOT tier is ON MAIN, opt-in (`NEOVM_AOT`), correctness-complete and
quintuple-audited: Cranelift ObjectModule -> content-hashed `.so` (SHA-256 +
ABI_TAG), per-thread LeafSidecar 4-param entry ABI, symbols + op-SymIds
reloc'd BY NAME (gensym-guarded), eq-identity via live-constant reloc,
fail-closed everywhere (descriptor validation, RTLD_NOW prod loads, recipe
verification, manifest version/abi/fingerprint interlocks), dump-time loadup
preload (706 leaves native-from-call-1), insert-if-absent prepopulate, and —
after the fast-reject + manifest-v2 pre-key work — a prepopulate cost of
~6.4ms (was 33.9ms).

Measured value: ~4-6.5x on no-warmup PURE-FIXNUM compute from call 1;
~1x-1.35x on shim/call-dominated bodies; startup delta now ~+17ms-equivalent
halved twice (see task 12 for residual levers). Verdict recorded: useful
OPT-IN; not default.

## 2. The blocker this task removes

**AOT leaves are non-speculative by construction**: they compile at
`obarray=None`, so `find_spec_sites` yields nothing — no direct-subr calls,
no predicate fast shims, no bytecode spec calls, no inlining. Since the
2026-07 JIT work, speculation is where the JIT's big wins live (7.9x
predicates). Consequences:
- PGO persistence (cache the hot set's native code across sessions) is
  currently NET-NEGATIVE-LEANING: a persisted leaf would SERVE SLOWER CODE
  than letting the function re-JIT with speculation. This is why R1d/PGO was
  shelved.
- A package-wide native cache (the GNU `.eln` analogue) inherits the same
  ceiling.

## 3. The design question

Make AOT artifacts CARRY speculation in a session-independent form, with
load-time re-validation:

1. **What a spec site needs at runtime:** sym (SymId — session-dependent!),
   expected function-cell VALUE bits (session heap address — session-
   dependent!), a SpecSlot (arm/epoch state — runtime-only), and the kind
   (SubrGeneral/Pred*/EqInclProps/Bytecode).
2. **Session-independent encoding:** the by-NAME reloc discipline already
   solves symbols (recipe codec, gensym-guarded). Encode per spec site:
   callee NAME + kind + arity signature. At LOAD (prepopulate/try_load_leaf),
   re-resolve the name in the live obarray; if the function cell currently
   holds {a plain Builtin subr with matching fixed arity (Subr kinds) | a
   bytecode function whose CONTENT HASH matches a recorded callee-hash
   (Bytecode kind)} -> materialize the SpecSlot armed with the LIVE epoch +
   LIVE expected bits; else -> leave the site's slot DISARMED (epoch=never)
   so the generated code's shim takes the generic path forever (correctness
   identical, speed = generic).
3. **Codegen implication:** the JIT lowering bakes sym/expected/slot as
   iconsts. For AOT these become SIDE-CAR RELOCATIONS: extend the LeafSidecar
   (or the reloc_data vec) with per-site cells the loader fills — the
   4-param sidecar ABI exists precisely for per-session bases; spec cells are
   the same shape (the sidecar already carries reloc_base/spill/meta; add a
   spec_base pointing at a loader-owned SpecSlot array + a per-site constants
   table). ABI_TAG must be salted with the new sidecar shape + encoding
   version (the tag machinery enumerates explicitly — extend
   `compute_abi_tag`).
4. **Re-arm semantics:** the runtime shims re-validate + re-arm on epoch
   moves already; loader-armed slots join that lifecycle unchanged. The
   in-place-subr-rewrite hazard is covered by the fresh-entry-read discipline
   (never bake SubrFn pointers — the artifacts store NAMES).
5. **Bytecode-callee spec across sessions:** the callee's identity must be
   its CONTENT (hash), not its address; record callee content-hash at emit;
   at load, hash the live callee (cache by compiled_id) and compare. A
   redefined-but-identical callee arms; a changed one disarms. (Inlined
   callees are a HARDER case — the inlined BODY is baked into the artifact;
   encode inline_deps as name+content-hash pairs and REFUSE to serve the
   leaf at all on mismatch — miss->JIT, the standing additive contract.)

## 4. What it unlocks (build in this order)

1. **PGO persistence (R1d revived):** at runtime, when the JIT compiles a
   function (it just proved hot), ALSO emit the object into a user cache dir
   (content-hash keyed; the `unit_index`/`NEOVM_AOT_DIR` consult already
   loads such files). Emission must be OFF the hot path — without task 05's
   worker thread, emit at idle/exit (a kill-emacs hook draining a queue of
   {ops,constants,mir} snapshots is acceptable v1). Next session: hot
   functions are native from call 1 WITH speculation.
2. **Package native cache (`.eln` analogue):** extend the dump-time producer
   pattern to a per-package batch producer (post-install hook byte-compiles
   then AOT-emits a package's defuns into one `.so` + manifest). All the
   session-independence machinery is shared; the new surface is cache-dir
   layout + staleness (content-hash makes it self-keying) + a load hook at
   package activation.

## 5. Hazards for the critique round

- Every new artifact field is a new cross-session surface: the audits' recurring
  lesson is "anything session-specific must reloc by name or content-hash" —
  enumerate EVERY baked iconst in the spec-site lowering and classify.
- Loader-armed slots vs `compiler_function_overrides_active()` and load-order
  (prepopulate runs before user init finishes — a site armed against the
  loadup definition, later advised: epoch bump disarms correctly; a site
  DISARMED at load because the package loads later: needs a re-arm-on-miss
  path or stays generic — decide + document; simplest correct v1: stays
  generic).
- eq-identity: spec expected-bits must come from the LIVE cell at load (like
  live-constant reloc), never from the recipe.
- Emission-at-exit durability (partial writes -> the fail-closed descriptor
  validation already rejects torn files; write-to-temp+rename anyway).
- ABI_TAG/versioning discipline (old artifacts must be cleanly ignored).

## 6. Gates & measurement

The AOT test corpus (27+ tests incl. the cross-session decoy-growth pattern,
side-effect-once-across-deopt, prepopulate semantics) extends with: an
armed-spec-site cross-session test (emit, decoy-grow intern table, load,
assert armed + correct + counters show FAST path), a disarm-on-changed-callee
test, an inline_deps-refusal test. Measurement: the R2-D bench protocol
(startup N-boot medians; `aot_bench_compute_loop`; add a SPEC-dependent bench
— e.g. the `jit_bench_pred` body served from AOT must now hit ~7.9x from
call 1, which is the whole point). Success = pred-class AOT-served code
matches JIT-hot performance; startup delta unchanged.
