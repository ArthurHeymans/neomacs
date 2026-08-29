use std::path::PathBuf;

const DOMAINS: &[&str] = &[
    "commands", "display", "editing", "lisp", "runtime", "system", "tests", "text",
];

fn emacs_core_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/emacs_core")
}

#[test]
fn emacs_core_root_is_a_facade_over_domain_directories() {
    let root = emacs_core_root();
    let mut root_files = std::fs::read_dir(&root)
        .expect("read emacs_core root")
        .filter_map(Result::ok)
        .filter(|entry| {
            let path = entry.path();
            path.is_file() && path.extension().is_some_and(|ext| ext == "rs")
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    root_files.sort();

    assert_eq!(
        root_files,
        ["mod.rs"],
        "emacs_core root is a stable facade; put subsystem files in their owning directory"
    );

    for domain in DOMAINS {
        assert!(
            root.join(domain).is_dir(),
            "emacs_core domain directory is missing: {domain}"
        );
    }
}

#[test]
fn production_domains_contain_subsystem_directories_not_loose_rust_files() {
    let root = emacs_core_root();

    for domain in DOMAINS.iter().copied().filter(|domain| *domain != "tests") {
        let loose_rust_files = std::fs::read_dir(root.join(domain))
            .unwrap_or_else(|error| panic!("read emacs_core/{domain}: {error}"))
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "rs"))
            .collect::<Vec<_>>();

        assert!(
            loose_rust_files.is_empty(),
            "emacs_core/{domain} contains loose Rust files: {loose_rust_files:?}; every subsystem owns a directory"
        );
    }
}
