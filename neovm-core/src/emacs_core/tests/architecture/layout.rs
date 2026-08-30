use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};
use syn::{Expr, ExprMethodCall, ImplItemFn, Item, ItemFn, Type};

const DOMAINS: &[&str] = &[
    "commands", "display", "editing", "lisp", "runtime", "system", "tests", "text",
];

fn emacs_core_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/emacs_core")
}

fn rust_files_below(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("read emacs_core source directory") {
        let path = entry.expect("read emacs_core source entry").path();
        if path.is_dir() {
            rust_files_below(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn parsed_rust_file(path: &Path) -> syn::File {
    let source = std::fs::read_to_string(path).expect("read emacs_core Rust source");
    syn::parse_file(&source).unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

fn is_test_source(relative: &Path) -> bool {
    relative
        .components()
        .any(|component| component.as_os_str() == "tests")
}

fn is_subrs_file(relative: &Path) -> bool {
    relative.file_name().is_some_and(|name| name == "subrs.rs")
        || relative.ends_with("subrs/mod.rs")
}

fn is_subr_spec_slice(ty: &Type) -> bool {
    let Type::Reference(reference) = ty else {
        return false;
    };
    let Type::Slice(slice) = reference.elem.as_ref() else {
        return false;
    };
    let Type::Path(path) = slice.elem.as_ref() else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "SubrSpec")
}

fn is_subrs_path(expr: &Expr) -> bool {
    let Expr::Path(path) = expr else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "SUBRS")
}

#[derive(Default)]
struct RegistrationVisitor {
    has_registration: bool,
    registers_subrs_slice: bool,
}

impl<'ast> Visit<'ast> for RegistrationVisitor {
    fn visit_item_fn(&mut self, function: &'ast ItemFn) {
        if function.sig.ident == "register_subrs" {
            self.has_registration = true;
        }
        visit::visit_item_fn(self, function);
    }

    fn visit_impl_item_fn(&mut self, function: &'ast ImplItemFn) {
        if function.sig.ident == "register_subrs" {
            self.has_registration = true;
        }
        visit::visit_impl_item_fn(self, function);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if call.method == "register_subr" || call.method == "register_subrs" {
            self.has_registration = true;
        }
        if call.method == "register_subrs"
            && call.args.len() == 1
            && call.args.first().is_some_and(is_subrs_path)
        {
            self.registers_subrs_slice = true;
        }
        visit::visit_expr_method_call(self, call);
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
fn out_of_line_subsystem_tests_live_in_tests_directories() {
    let root = emacs_core_root();
    let mut rust_files = Vec::new();
    rust_files_below(&root, &mut rust_files);

    let mut misplaced = rust_files
        .into_iter()
        .filter_map(|path| {
            let relative = path.strip_prefix(&root).expect("path below emacs_core");
            if is_test_source(relative) {
                return None;
            }
            let stem = relative.file_stem()?.to_string_lossy();
            (stem == "tests" || stem.ends_with("_test") || stem.ends_with("_tests"))
                .then(|| relative.to_path_buf())
        })
        .collect::<Vec<_>>();
    misplaced.sort();

    assert!(
        misplaced.is_empty(),
        "out-of-line subsystem tests belong in <subsystem>/tests/: {misplaced:?}"
    );
}

#[test]
fn production_subr_registration_lives_in_subrs_files() {
    let root = emacs_core_root();
    let mut rust_files = Vec::new();
    rust_files_below(&root, &mut rust_files);

    let mut misplaced = Vec::new();
    let mut malformed_tables = Vec::new();
    let mut implementation_leaks = Vec::new();
    for path in rust_files {
        let relative = path
            .strip_prefix(&root)
            .expect("emacs_core Rust file must be below emacs_core root");
        if is_test_source(relative) {
            continue;
        }

        let syntax = parsed_rust_file(&path);
        let mut registration = RegistrationVisitor::default();
        registration.visit_file(&syntax);
        if registration.has_registration && !is_subrs_file(relative) {
            misplaced.push(relative.to_path_buf());
            continue;
        }
        if !is_subrs_file(relative) {
            continue;
        }

        if relative.file_name().is_some_and(|name| name == "subrs.rs") {
            let has_typed_table = syntax.items.iter().any(|item| {
                matches!(item, Item::Const(item) if item.ident == "SUBRS" && is_subr_spec_slice(&item.ty))
            });
            if !has_typed_table || !registration.registers_subrs_slice {
                malformed_tables.push(relative.to_path_buf());
            }
        }

        let evaluator_exception = relative == Path::new("runtime/eval/subrs.rs");
        if !evaluator_exception {
            let extra_functions = syntax
                .items
                .iter()
                .filter_map(|item| match item {
                    Item::Fn(function) if function.sig.ident != "register_subrs" => {
                        Some(function.sig.ident.to_string())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            let owns_types_or_impls = syntax.items.iter().any(|item| {
                matches!(
                    item,
                    Item::Enum(_)
                        | Item::Impl(_)
                        | Item::Struct(_)
                        | Item::Trait(_)
                        | Item::Type(_)
                )
            });
            if owns_types_or_impls || !extra_functions.is_empty() {
                implementation_leaks.push((relative.to_path_buf(), extra_functions));
            }
        }
    }

    misplaced.sort();
    malformed_tables.sort();
    implementation_leaks.sort_by(|left, right| left.0.cmp(&right.0));
    assert!(
        misplaced.is_empty(),
        "native Lisp registration belongs in subsystem-owned subrs.rs files: {misplaced:?}"
    );
    assert!(
        malformed_tables.is_empty(),
        "subsystem subrs.rs files use const SUBRS: &[SubrSpec] and register it as a slice: {malformed_tables:?}"
    );
    assert!(
        implementation_leaks.is_empty(),
        "subrs.rs owns declarations only; move implementations and domain types to mod.rs: {implementation_leaks:?}"
    );
}
