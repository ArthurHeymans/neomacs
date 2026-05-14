use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
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
