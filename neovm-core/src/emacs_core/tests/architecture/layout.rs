use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ExprMethodCall, ImplItemFn, Item, ItemFn, ItemMod, Lit, Meta, Type};

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

fn cfg_meta_requires_test(meta: &Meta) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(list) => {
            let Ok(nested) = list.parse_args_with(
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
            ) else {
                return false;
            };
            if list.path.is_ident("all") {
                nested.iter().any(cfg_meta_requires_test)
            } else if list.path.is_ident("any") {
                !nested.is_empty() && nested.iter().all(cfg_meta_requires_test)
            } else {
                // `not(test)` and unknown cfg predicates do not establish that
                // an item is compiled only by the test configuration.
                false
            }
        }
        Meta::NameValue(_) => false,
    }
}

fn is_cfg_test(attribute: &Attribute) -> bool {
    attribute.path().is_ident("cfg")
        && attribute
            .parse_args::<Meta>()
            .is_ok_and(|meta| cfg_meta_requires_test(&meta))
}

fn is_test_attribute(attribute: &Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| matches!(segment.ident.to_string().as_str(), "test" | "rstest"))
}

fn path_attribute(module: &ItemMod) -> Option<PathBuf> {
    module.attrs.iter().find_map(|attribute| {
        if !attribute.path().is_ident("path") {
            return None;
        }
        let Meta::NameValue(name_value) = &attribute.meta else {
            return None;
        };
        let Expr::Lit(expression) = &name_value.value else {
            return None;
        };
        let Lit::Str(path) = &expression.lit else {
            return None;
        };
        Some(PathBuf::from(path.value()))
    })
}

fn has_misplaced_test_syntax(syntax: &syn::File) -> bool {
    if syntax.attrs.iter().any(is_cfg_test)
        || syntax.items.iter().any(
            |item| matches!(item, Item::Fn(function) if function.attrs.iter().any(is_test_attribute)),
        )
    {
        return true;
    }

    syntax.items.iter().any(|item| {
        let Item::Mod(module) = item else {
            return false;
        };
        if module.content.is_some() || !module.attrs.iter().any(is_cfg_test) {
            return false;
        }
        path_attribute(module).map_or(module.ident != "tests", |path| !is_test_source(&path))
    })
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
            let test_shaped_name =
                stem == "tests" || stem.ends_with("_test") || stem.ends_with("_tests");
            let syntax = parsed_rust_file(&path);
            (test_shaped_name || has_misplaced_test_syntax(&syntax)).then(|| relative.to_path_buf())
        })
        .collect::<Vec<_>>();
    misplaced.sort();

    assert!(
        misplaced.is_empty(),
        "out-of-line subsystem tests belong in <subsystem>/tests/: {misplaced:?}"
    );
}

#[test]
fn test_placement_guard_reads_rust_test_attributes_and_module_paths() {
    let top_level_test = syn::parse_file("#[test] fn behavior() {}").expect("parse test");
    assert!(has_misplaced_test_syntax(&top_level_test));

    let test_only_file = syn::parse_file("#![cfg(test)] fn helper() {}").expect("parse test");
    assert!(has_misplaced_test_syntax(&test_only_file));

    let non_test_file = syn::parse_file("#![cfg(not(test))] fn helper() {}").expect("parse test");
    assert!(!has_misplaced_test_syntax(&non_test_file));

    let mixed_cfg = syn::parse_file("#![cfg(any(test, feature = \"fuzzing\"))] fn helper() {}")
        .expect("parse test");
    assert!(!has_misplaced_test_syntax(&mixed_cfg));

    let test_conjunction =
        syn::parse_file("#![cfg(all(test, unix))] fn helper() {}").expect("parse test");
    assert!(has_misplaced_test_syntax(&test_conjunction));

    let external_test_module = syn::parse_file("#[cfg(test)] mod checks;").expect("parse test");
    assert!(has_misplaced_test_syntax(&external_test_module));

    let external_test_directory =
        syn::parse_file("#[cfg(test)] #[path = \"tests/checks.rs\"] mod checks;")
            .expect("parse test");
    assert!(!has_misplaced_test_syntax(&external_test_directory));

    let inline_white_box_tests = syn::parse_file(
        "fn implementation() {} #[cfg(test)] mod tests { #[test] fn behavior() {} }",
    )
    .expect("parse test");
    assert!(!has_misplaced_test_syntax(&inline_white_box_tests));
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
