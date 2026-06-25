use std::env;

/// The full `neovm_jit_*` runtime-shim set an AOT preload `.so` imports
/// (`#[unsafe(no_mangle)] pub` in neovm-core). SINGLE SOURCE OF TRUTH (R2-C2):
/// the list lives in `neovm-core/src/emacs_core/jit/shim_names.rs` and is
/// `include!`-ed here (as `NEOVM_JIT_SHIM_NAMES`) so this production export set
/// can never drift from neovm-core's `MIR_SHIM_NAMES` or its lib build.rs export
/// set. Still MUST match the shim DEFINITIONS in jit/compile.rs + `JIT_SHIM_ANCHOR`.
/// R2-B5: the PRODUCTION `neomacs` binary exports these into its DYNAMIC symbol
/// table so the dump-time `libneomacs-preload.so` (R2) binds its undefined shim
/// imports at `dlopen`. Under the workspace linker `wild`, plain `-rdynamic` does
/// NOT promote these address-only-referenced fns — each must be named with
/// `--export-dynamic-symbol` (the R1c carry-forward; without it the preload `.so`
/// aborts on first shim call).
include!("../neovm-core/src/emacs_core/jit/shim_names.rs");

/// R2-B5: export the `neovm_jit_*` shims into the `neomacs` binary's dynamic
/// symbol table (Linux + `jit` only) so the AOT preload `.so` resolves them at
/// dlopen. Targets ONLY the `neomacs` bin (`rustc-link-arg-bin=neomacs=`), not
/// `mock-display` or any test.
fn export_jit_shims_for_aot(target_os: &str) {
    if target_os != "linux" || env::var_os("CARGO_FEATURE_JIT").is_none() {
        return;
    }
    println!("cargo:rustc-link-arg-bin=neomacs=-rdynamic");
    for shim in NEOVM_JIT_SHIM_NAMES {
        println!("cargo:rustc-link-arg-bin=neomacs=-Wl,--export-dynamic-symbol={shim}");
    }
}

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    println!("cargo:rerun-if-changed=../neovm-core/src/emacs_core/jit/shim_names.rs");
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
