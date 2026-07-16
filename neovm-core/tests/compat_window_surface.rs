mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use common::{
    collect_defsubr_targets, gnu_window_c_path, oracle_enabled, run_neovm_eval, run_oracle_eval,
};
use regex::Regex;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ImplBucket {
    WindowCmds,
    LocalBuiltins,
    Stubs,
    OtherModule,
}

impl ImplBucket {
    fn label(self) -> &'static str {
        match self {
            Self::WindowCmds => "window_cmds",
            Self::LocalBuiltins => "builtins_mod_local",
            Self::Stubs => "stubs",
            Self::OtherModule => "other_module",
        }
    }
}

fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn parse_gnu_window_defuns(path: &Path) -> BTreeSet<String> {
    let source = fs::read_to_string(path).expect("read GNU window.c");
    let re = Regex::new(r#"DEFUN \("([^"]+)","#).expect("window.c DEFUN regex");
    re.captures_iter(&source)
        .map(|caps| caps[1].to_string())
        .collect()
}

fn builtin_ident_in_target(target: &str) -> Option<String> {
    let re = Regex::new(r#"(builtin_[A-Za-z0-9_]+)"#).expect("builtin ident regex");
    re.captures(target).map(|caps| caps[1].to_string())
}

fn classify_target(target: &str, stubs_source: &str) -> ImplBucket {
    if target.contains("super::window_cmds::") {
        return ImplBucket::WindowCmds;
    }
    if target.contains("super::") {
        return ImplBucket::OtherModule;
    }
    if let Some(ident) = builtin_ident_in_target(target) {
        if stubs_source.contains(&format!("fn {ident}("))
            || stubs_source.contains(&format!("fn {ident} ("))
        {
            return ImplBucket::Stubs;
        }
    }
    ImplBucket::LocalBuiltins
}

fn symbol_list_literal(names: &BTreeSet<String>) -> String {
    let body = names.iter().cloned().collect::<Vec<_>>().join(" ");
    format!("'({body})")
}

#[test]
fn compat_window_surface_matches_gnu_emacs() {
    if !oracle_enabled() {
        eprintln!(
            "skipping window surface audit: set NEOVM_FORCE_ORACLE_PATH or place GNU Emacs mirror alongside the repo"
        );
        return;
    }

    let Some(gnu_window_c) = gnu_window_c_path() else {
        eprintln!("skipping window surface audit: GNU window.c not found");
        return;
    };
    let stubs_rs = crate_root().join("src/emacs_core/builtins/stubs.rs");

    let gnu_window_defuns = parse_gnu_window_defuns(&gnu_window_c);
    let defsubr_targets = collect_defsubr_targets(&crate_root().join("src"));
    let stubs_source = fs::read_to_string(&stubs_rs).expect("read builtins/stubs.rs");

    let missing_registrations = gnu_window_defuns
        .iter()
        .filter(|name| !defsubr_targets.contains_key(*name))
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        missing_registrations.is_empty(),
        "GNU window.c DEFUNs missing NeoVM defsubr registrations: {}",
        missing_registrations.join(", ")
    );

    let mut by_bucket: BTreeMap<ImplBucket, Vec<String>> = BTreeMap::new();
    for name in &gnu_window_defuns {
        let target = defsubr_targets
            .get(name)
            .expect("window primitive target should exist");
        by_bucket
            .entry(classify_target(target, &stubs_source))
            .or_default()
            .push(name.clone());
    }

    for bucket in [
        ImplBucket::WindowCmds,
        ImplBucket::LocalBuiltins,
        ImplBucket::Stubs,
        ImplBucket::OtherModule,
    ] {
        let names = by_bucket.get(&bucket).cloned().unwrap_or_default();
        println!(
            "window.c audit bucket {} count={} names={}",
            bucket.label(),
            names.len(),
            names.join(", ")
        );
    }

    let symbol_list = symbol_list_literal(&gnu_window_defuns);
    let form = format!(
        r#"(mapcar (lambda (name) (cons name (subrp (symbol-function name)))) {symbol_list})"#
    );

    let gnu = run_oracle_eval(&form).expect("GNU Emacs window surface evaluation");
    let neovm = run_neovm_eval(&form).expect("NeoVM window surface evaluation");
    assert_eq!(
        neovm, gnu,
        "window subr surface mismatch:\nGNU: {}\nNeoVM: {}",
        gnu, neovm
    );
}
