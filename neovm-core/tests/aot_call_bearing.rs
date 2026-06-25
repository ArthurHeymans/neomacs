//! R1c call-bearing AOT — integration test (Linux-only).
//!
//! Drives the REAL production path: emit a call-bearing leaf's `.so`, place it in
//! `NEOVM_AOT_DIR`, and serve it through `try_run_compiled` under
//! `NEOVM_AOT=force`. This MUST be an integration test (not a lib unit test):
//! the host's `neovm_jit_*` shims are exported into the DYNAMIC symbol table only
//! for integration-test binaries (`-rdynamic` + `--export-dynamic-symbol`, see
//! neovm-core/build.rs), so a call/cons `.so`'s undefined shim imports resolve at
//! `dlopen`. The lib unit-test binary is NOT `-rdynamic`'d.
//!
//! The scenario logic lives in a `#[doc(hidden)] pub` crate-internal self-test
//! (it needs crate-private types: ByteCodeFunction internals, obarray, Vm); this
//! integration test just sets the env and calls it, so the whole thing runs in
//! the (shim-exporting) integration-test process.

#![cfg(all(feature = "jit", target_os = "linux"))]

#[test]
fn aot_call_bearing_deopt_across_call_side_effect_once_and_eq() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Set the AOT env BEFORE any AOT code runs (OnceLock-memoized gates).
    // SAFETY: single-threaded test setup before any AOT entry point reads these.
    unsafe {
        std::env::set_var("NEOVM_AOT", "force");
        std::env::set_var("NEOVM_AOT_DIR", dir.path());
    }
    // The crate-internal self-test does the full emit→place→serve-from-AOT→assert
    // (call-bearing serve + deopt-across-call side-effect-exactly-once + #A
    // eq-identity + #B non-UTF-8). Returns Err(reason) on any failure.
    let r = neovm_core::emacs_core::jit::aot::testkit_call_bearing_selftest(dir.path());
    unsafe {
        std::env::remove_var("NEOVM_AOT");
        std::env::remove_var("NEOVM_AOT_DIR");
    }
    if let Err(e) = r {
        panic!("call-bearing AOT self-test failed: {e}");
    }
}
