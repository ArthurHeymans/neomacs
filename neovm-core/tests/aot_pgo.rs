//! R2 increment C — AOT PGO persistence (Linux-only integration test).
//!
//! Persist proven-hot JIT leaves to `NEOVM_AOT_DIR` at shutdown so the NEXT session
//! serves them native + speculative from call 1. These tests drive the REAL
//! production path: emit a spec-bearing leaf's `.so` via the unified producer the
//! drain calls (`compile_leaf_to_object` with the live obarray), place it in
//! `NEOVM_AOT_DIR`, then LOAD + serve it through `try_run_compiled` under
//! `NEOVM_AOT=force` against a FRESH obarray. They MUST be integration tests (not
//! lib unit tests): the round-1 spec shims (`neovm_jit_pred_spec` /
//! `neovm_jit_call_subr_spec`) are exported into the DYNAMIC symbol table only for
//! `-rdynamic` test binaries (see neovm-core/build.rs), so a spec-bearing `.so`'s
//! undefined shim imports resolve at `dlopen`.
//!
//! The scenario logic lives in `#[doc(hidden)] pub` crate-internal self-tests (they
//! need crate-private types: obarray, Vm, ByteCodeFunction internals + the spec
//! counters); each integration test just sets the env and calls one, so the whole
//! thing runs in the (shim-exporting) integration-test process. Each runs ALONE in
//! its own binary/process (nextest), so the process-global `SUBR_SPEC_*` counters,
//! the `NEOVM_AOT*` OnceLock gates, and the frozen unit index are uncontended.

#![cfg(all(feature = "jit", target_os = "linux"))]

/// STEP 1 (GO/NO-GO): a pred-class body emitted via `compile_leaf_to_object` (the
/// drain's exact producer) round-trips runtime-emit → next-session-load: it serves
/// AOT-backed, fires the pred FAST shim FROM CALL 1, and equals the interpreter.
#[test]
fn pgo_roundtrip_runtime_emit_next_session_serves_fast_from_call_1() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Set the AOT env BEFORE any AOT code runs (OnceLock-memoized gates).
    // SAFETY: single-threaded test setup before any AOT entry point reads these;
    // nextest isolates each test in its own process → no OnceLock cross-talk.
    unsafe {
        std::env::set_var("NEOVM_AOT", "force");
        std::env::set_var("NEOVM_AOT_DIR", dir.path());
    }
    let r = neovm_core::emacs_core::jit::aot::testkit_pgo_roundtrip_selftest(dir.path());
    unsafe {
        std::env::remove_var("NEOVM_AOT");
        std::env::remove_var("NEOVM_AOT_DIR");
    }
    if let Err(e) = r {
        panic!("AOT-PGO round-trip self-test failed: {e}");
    }
}
