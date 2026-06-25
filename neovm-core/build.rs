use std::fs;
use std::path::{Path, PathBuf};

#[path = "build_support/unicode_gen.rs"]
mod unicode_gen;

// Single source of truth (R2-C2): the `neovm_jit_*` shim names, shared with
// jit/aot.rs (MIR_SHIM_NAMES) + neomacs-bin/build.rs via `include!` so the
// emit/salt set and both export sets can never drift.
include!("src/emacs_core/jit/shim_names.rs");

fn main() {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let project_root = manifest_dir.parent().expect("workspace root");

    unicode_gen::ensure_generated_unicode_lisp(&manifest_dir, project_root);
    generate_x11_color_table(project_root, &manifest_dir);

    // R1c call-bearing AOT: export the host's `neovm_jit_*` shims
    // (`#[unsafe(no_mangle)] pub`, anchored by `JIT_SHIM_ANCHOR`) into the TEST
    // binaries' DYNAMIC symbol table, where a `dlopen`'d call/cons AOT `.so` binds
    // its undefined imports against them. `-rdynamic` alone is insufficient under
    // the workspace linker (it doesn't promote these otherwise-unreferenced fns
    // to the dynamic table), so we additionally name each shim with
    // `--export-dynamic-symbol`. `rustc-link-arg-tests` applies ONLY to test
    // binaries — the lib + production binaries are untouched (production export is
    // R2's job). Gated on the `jit` feature (the only config that emits AOT).
    //
    // R2 CARRY-FORWARD (R2-B5): the PRODUCTION binary (neomacs-bin) that loads the
    // dump-time preload `.so` MUST replicate BOTH the `#[unsafe(no_mangle)] pub`
    // shims AND this per-shim `--export-dynamic-symbol` export in ITS build.rs —
    // under the `wild` linker, plain `-rdynamic` does NOT promote these
    // address-only-referenced fns to the dynamic table, so the preload `.so`'s
    // `neovm_jit_*` imports would otherwise fail to resolve at dlopen and abort on
    // first shim call. Do not assume `-rdynamic` alone suffices.
    if std::env::var_os("CARGO_FEATURE_JIT").is_some() && cfg!(target_os = "linux") {
        println!("cargo:rustc-link-arg-tests=-rdynamic");
        // NEOVM_JIT_SHIM_NAMES is `include!`-ed at module scope (above) from the
        // single-source shim_names.rs — same set aot.rs salts/exports.
        for shim in NEOVM_JIT_SHIM_NAMES {
            println!("cargo:rustc-link-arg-tests=-Wl,--export-dynamic-symbol={shim}");
        }
    }
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("unicode-data").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        project_root.join("etc/rgb.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("build_support/unicode_gen.rs").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir
            .join("src/emacs_core/jit/shim_names.rs")
            .display()
    );
}

/// Parse etc/rgb.txt and generate a Rust source file with a static
/// color lookup function. This gives us the full X11 color database
/// (788 colors including grey0-grey100, DarkGoldenrod, etc.) with
/// zero runtime file I/O — the table is compiled into the binary.
fn generate_x11_color_table(project_root: &Path, manifest_dir: &Path) {
    let rgb_path = project_root.join("etc/rgb.txt");
    println!("cargo:rerun-if-changed={}", rgb_path.display());

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let out_path = out_dir.join("x11_colors.rs");

    let content = fs::read_to_string(&rgb_path).unwrap_or_else(|e| {
        eprintln!("cargo:warning=Cannot read {}: {}", rgb_path.display(), e);
        String::new()
    });

    // Parse rgb.txt: "R G B\t\tColorName"
    // Collect unique (lowercase_name -> (r, g, b)), also add no-space variants.
    let mut colors: std::collections::BTreeMap<String, (u8, u8, u8)> =
        std::collections::BTreeMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let r = parts.next().and_then(|s| s.parse::<u8>().ok());
        let g = parts.next().and_then(|s| s.parse::<u8>().ok());
        let b = parts.next().and_then(|s| s.parse::<u8>().ok());
        // Remaining is the color name (may contain spaces)
        let name: String = parts.collect::<Vec<_>>().join(" ");

        if let (Some(r), Some(g), Some(b)) = (r, g, b) {
            if !name.is_empty() {
                let lower = name.to_lowercase();
                let no_spaces = lower.replace(' ', "");
                colors.entry(lower.clone()).or_insert((r, g, b));
                if no_spaces != lower {
                    colors.entry(no_spaces).or_insert((r, g, b));
                }
            }
        }
    }

    // Generate Rust source: a function with a match statement.
    let mut code = String::new();
    code.push_str("/// Auto-generated from etc/rgb.txt — do not edit.\n");
    code.push_str("/// X11 color name lookup (case-insensitive).\n");
    code.push_str("pub fn x11_color_lookup(name: &str) -> Option<(u8, u8, u8)> {\n");
    code.push_str("    match name.to_lowercase().as_str() {\n");
    for (name, (r, g, b)) in &colors {
        code.push_str(&format!(
            "        {:?} => Some(({}, {}, {})),\n",
            name, r, g, b
        ));
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    fs::write(&out_path, &code).expect("Failed to write x11_colors.rs");
    eprintln!(
        "cargo:warning=Generated X11 color table: {} entries from {}",
        colors.len(),
        rgb_path.display()
    );
}
