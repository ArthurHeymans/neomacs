use std::path::PathBuf;

const DOMAINS: &[&str] = &[
    "commands", "display", "editing", "lisp", "runtime", "system", "tests", "text",
];

fn emacs_core_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/emacs_core")
}

fn rust_files_below(directory: &std::path::Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("read emacs_core source directory") {
        let path = entry.expect("read emacs_core source entry").path();
        if path.is_dir() {
            rust_files_below(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
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

#[test]
fn production_subr_registration_lives_in_subrs_files() {
    let root = emacs_core_root();
    let mut rust_files = Vec::new();
    rust_files_below(&root, &mut rust_files);

    let mut misplaced = Vec::new();
    let mut missing_tables = Vec::new();
    for path in rust_files {
        let relative = path
            .strip_prefix(&root)
            .expect("emacs_core Rust file must be below emacs_core root");
        if relative
            .components()
            .any(|component| component.as_os_str() == "tests")
            || relative.file_stem().is_some_and(|stem| stem == "tests")
            || relative
                .file_stem()
                .is_some_and(|stem| stem.to_string_lossy().ends_with("_test"))
        {
            continue;
        }

        let source = std::fs::read_to_string(&path).expect("read emacs_core Rust source");
        let owns_registration = source.contains("fn register_subrs(")
            || source.contains(".register_subr(")
            || source.contains(".register_subrs(");
        if !owns_registration {
            continue;
        }

        let is_subrs_file = relative.file_name().is_some_and(|name| name == "subrs.rs")
            || relative.ends_with("subrs/mod.rs");
        if !is_subrs_file {
            misplaced.push(relative.to_path_buf());
        } else if relative.file_name().is_some_and(|name| name == "subrs.rs")
            && (!source.contains("const SUBRS: &[SubrSpec]")
                || !source.contains("register_subrs(SUBRS)"))
        {
            missing_tables.push(relative.to_path_buf());
        }
    }

    misplaced.sort();
    missing_tables.sort();
    assert!(
        misplaced.is_empty(),
        "native Lisp registration belongs in subsystem-owned subrs.rs files: {misplaced:?}"
    );
    assert!(
        missing_tables.is_empty(),
        "subsystem subrs.rs files use const SUBRS: &[SubrSpec] and register it as a slice: {missing_tables:?}"
    );
}
