//! Integration test for dynamic module loading and calling.
//!
//! Builds a Rust cdylib test module, loads it via `module-load`,
//! and exercises the key module API functions.

use std::path::PathBuf;
use std::process::Command;

use neovm_core::emacs_core::eval::Context;
use neovm_core::emacs_core::format_eval_result_with_eval;
use neovm_core::emacs_core::load::{
    apply_runtime_startup_state, create_bootstrap_evaluator_cached,
};

mod common;

/// Build the test cdylib, returning the path to the .so file.
fn build_test_module() -> PathBuf {
    let module_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test-module");
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&module_dir)
        .status()
        .expect("failed to build test module");

    assert!(status.success(), "test module build failed");

    let target_dir = module_dir.join("target/release");
    for entry in std::fs::read_dir(&target_dir).expect("read target dir") {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name_str = name.to_str().unwrap();
        if name_str.starts_with("libneovm_test_module")
            && (name_str.ends_with(".so") || name_str.ends_with(".dylib"))
        {
            return entry.path();
        }
    }
    panic!("test module .so file not found in {:?}", target_dir);
}

fn setup_eval() -> Context {
    let mut eval = create_bootstrap_evaluator_cached().expect("bootstrap");
    apply_runtime_startup_state(&mut eval).expect("startup");
    eval.set_lexical_binding(true);
    eval
}

fn eval_str(eval: &mut Context, form: &str) -> String {
    let result = eval.eval_str(form);
    format_eval_result_with_eval(eval, &result)
}

#[test]
fn test_module_load_and_sum() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();

    let mut eval = setup_eval();

    // Load the module.
    let result = eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    assert_eq!(result, "OK t");

    // Test mod-test-sum.
    let result = eval_str(&mut eval, "(mod-test-sum 3 4)");
    assert_eq!(result, "OK 7");
}

#[test]
fn test_module_return_t() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();
    let mut eval = setup_eval();
    eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    let result = eval_str(&mut eval, "(mod-test-return-t)");
    assert_eq!(result, "OK t");
}

#[test]
fn test_module_make_string() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();
    let mut eval = setup_eval();
    eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    let result = eval_str(&mut eval, "(mod-test-make-string \"hello\")");
    assert_eq!(result, "OK \"hello\"");
}

#[test]
fn test_module_string_a_to_b() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();
    let mut eval = setup_eval();
    eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    let result = eval_str(&mut eval, "(mod-test-string-a-to-b \"abracadabra\")");
    assert_eq!(result, "OK \"bbrbcbdbbrb\"");
}

#[test]
fn test_module_userptr_make_and_get() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();
    let mut eval = setup_eval();
    eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    let result = eval_str(&mut eval, "(mod-test-userptr-get (mod-test-userptr-make))");
    assert_eq!(result, "OK 42");
}

#[test]
fn test_module_vector_fill() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();
    let mut eval = setup_eval();
    eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    let result = eval_str(&mut eval, "(mod-test-vector-fill (vector 1 2 3) 42)");
    assert_eq!(result, "OK [42 42 42]");
}

#[test]
fn test_module_vector_eq() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();
    let mut eval = setup_eval();
    eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    let result = eval_str(&mut eval, "(mod-test-vector-eq [1 2 3] [1 2 3])");
    assert_eq!(result, "OK t");
    let result = eval_str(&mut eval, "(mod-test-vector-eq [1 2 3] [4 5 6])");
    assert_eq!(result, "OK nil");
}

#[test]
fn test_module_global_ref() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();
    let mut eval = setup_eval();
    eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    let result = eval_str(&mut eval, "(mod-test-globref-make)");
    // Just verify it returns something without crashing.
    assert!(!result.is_empty());
}

#[test]
fn test_module_function_p() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();
    let mut eval = setup_eval();
    eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    let result = eval_str(
        &mut eval,
        "(module-function-p (symbol-function 'mod-test-sum))",
    );
    assert_eq!(result, "OK t");
    let result = eval_str(&mut eval, "(module-function-p (symbol-function '+))");
    assert_eq!(result, "OK nil");
}

#[test]
fn test_module_user_ptrp() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();
    let mut eval = setup_eval();
    eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    let result = eval_str(&mut eval, "(user-ptrp (mod-test-userptr-make))");
    assert_eq!(result, "OK t");
    let result = eval_str(&mut eval, "(user-ptrp 42)");
    assert_eq!(result, "OK nil");
}

#[test]
fn test_module_double_load_is_noop() {
    let so_path = build_test_module();
    let so_path_str = so_path.to_str().unwrap();
    let mut eval = setup_eval();
    let result = eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    assert_eq!(result, "OK t");
    let result = eval_str(&mut eval, &format!("(module-load \"{}\")", so_path_str));
    assert_eq!(result, "OK t");
}

#[test]
fn test_module_file_suffix() {
    let mut eval = setup_eval();
    let result = eval_str(&mut eval, "module-file-suffix");
    assert!(!result.is_empty());
    assert!(result != "ERR");
}
