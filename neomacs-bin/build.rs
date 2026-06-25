use std::env;

/// The full `neovm_jit_*` runtime-shim set an AOT preload `.so` imports
/// (`#[unsafe(no_mangle)] pub` in neovm-core). MUST stay in sync with neovm-core's
/// `MIR_SHIM_NAMES` / `JIT_SHIM_ANCHOR` / build.rs (jit/aot.rs + jit/compile.rs).
/// R2-B5: the PRODUCTION `neomacs` binary exports these into its DYNAMIC symbol
/// table so the dump-time `libneomacs-preload.so` (R2) binds its undefined shim
/// imports at `dlopen`. Under the workspace linker `wild`, plain `-rdynamic` does
/// NOT promote these address-only-referenced fns — each must be named with
/// `--export-dynamic-symbol` (the R1c carry-forward; without it the preload `.so`
/// aborts on first shim call).
const NEOVM_JIT_SHIMS: &[&str] = &[
    "neovm_jit_apply",
    "neovm_jit_backedge",
    "neovm_jit_builtin1",
    "neovm_jit_builtin2",
    "neovm_jit_builtin3",
    "neovm_jit_builtin_slice",
    "neovm_jit_call",
    "neovm_jit_call_spec",
    "neovm_jit_cons",
    "neovm_jit_eq_slow",
    "neovm_jit_gc_push",
    "neovm_jit_gc_restore",
    "neovm_jit_gc_save",
    "neovm_jit_integerp_slow",
    "neovm_jit_list",
    "neovm_jit_match_handler",
    "neovm_jit_named_builtin",
    "neovm_jit_numberp_slow",
    "neovm_jit_pop_handler",
    "neovm_jit_push_catch",
    "neovm_jit_push_cc",
    "neovm_jit_push_cc_raw",
    "neovm_jit_save_current_buffer",
    "neovm_jit_save_excursion",
    "neovm_jit_save_restriction",
    "neovm_jit_save_window_excursion",
    "neovm_jit_switch",
    "neovm_jit_switch_stale",
    "neovm_jit_symbolp_slow",
    "neovm_jit_throw",
    "neovm_jit_unbind",
    "neovm_jit_unwind_protect",
    "neovm_jit_varbind",
    "neovm_jit_varref",
    "neovm_jit_varset",
];

/// R2-B5: export the `neovm_jit_*` shims into the `neomacs` binary's dynamic
/// symbol table (Linux + `jit` only) so the AOT preload `.so` resolves them at
/// dlopen. Targets ONLY the `neomacs` bin (`rustc-link-arg-bin=neomacs=`), not
/// `mock-display` or any test.
fn export_jit_shims_for_aot(target_os: &str) {
    if target_os != "linux" || env::var_os("CARGO_FEATURE_JIT").is_none() {
        return;
    }
    println!("cargo:rustc-link-arg-bin=neomacs=-rdynamic");
    for shim in NEOVM_JIT_SHIMS {
        println!("cargo:rustc-link-arg-bin=neomacs=-Wl,--export-dynamic-symbol={shim}");
    }
}

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    export_jit_shims_for_aot(&target_os);
    if target_os == "windows" {
        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        match target_env.as_str() {
            "msvc" => println!("cargo:rustc-link-arg-bin=neomacs=/STACK:134217728"),
            "gnu" => println!("cargo:rustc-link-arg-bin=neomacs=-Wl,--stack,134217728"),
            _ => {}
        }
        return;
    }

    let candidates: &[&str] = match target_os.as_str() {
        "linux" => &["ncursesw", "ncurses"],
        "macos" => &["ncurses", "ncursesw"],
        _ => return,
    };

    for name in candidates {
        if let Ok(library) = pkg_config::Config::new().probe(name) {
            for path in library.link_paths {
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", path.display());
            }
            return;
        }
    }

    println!("cargo:rustc-link-lib={}", candidates[0]);
}
