use super::{Diagnostic, Executor, LispValue, execute_source};

fn run(src: &str) -> String {
    let artifact = execute_source(
        "test.el",
        &format!(";;; -*- lexical-binding: t; -*-\n{}", src),
        &[],
    );
    assert_eq!(artifact.result.diagnostics, Vec::new());
    artifact
        .result
        .value
        .map(|v| artifact.runtime.format_value(v))
        .unwrap_or("nil".to_string())
}

fn run_with_args(src: &str, args: &[i64]) -> String {
    let artifact = execute_source(
        "test.el",
        &format!(";;; -*- lexical-binding: t; -*-\n{}", src),
        args,
    );
    assert_eq!(artifact.result.diagnostics, Vec::new());
    artifact
        .result
        .value
        .map(|v| artifact.runtime.format_value(v))
        .unwrap_or("nil".to_string())
}

fn run_err(src: &str) -> bool {
    let artifact = execute_source(
        "test.el",
        &format!(";;; -*- lexical-binding: t; -*-\n{}", src),
        &[],
    );
    artifact.result.diagnostics.iter().any(|d| d.is_error())
}

// --- &aux ---

#[test]
fn cl_defun_aux_simple() {
    let result = run("(cl-defun foo (x &aux (y (+ x 1))) y) (foo 5)");
    assert_eq!(result, "6");
}

#[test]
fn cl_defun_aux_nil_default() {
    let result = run("(cl-defun foo (x &aux y) (list x y)) (foo 5)");
    assert_eq!(result, "(5 nil)");
}

#[test]
fn cl_defun_aux_multiple() {
    let result = run("(cl-defun foo (x &aux (y (* x 2)) (z (+ y 1))) (list x y z)) (foo 3)");
    assert_eq!(result, "(3 6 7)");
}

// --- &key ---

#[test]
fn cl_defun_key_basic() {
    let result = run("(cl-defun foo (&key x y) (list x y)) (foo :x 1 :y 2)");
    assert_eq!(result, "(1 2)");
}

#[test]
fn cl_defun_key_default_nil() {
    let result = run("(cl-defun foo (&key x) x) (foo)");
    assert_eq!(result, "nil");
}

#[test]
fn cl_defun_key_with_default() {
    let result = run("(cl-defun foo (&key (x 10)) x) (foo)");
    assert_eq!(result, "10");
}

#[test]
fn cl_defun_key_override_default() {
    let result = run("(cl-defun foo (&key (x 10)) x) (foo :x 42)");
    assert_eq!(result, "42");
}

#[test]
fn cl_defun_key_reversed_order() {
    let result = run("(cl-defun foo (&key x y) (list x y)) (foo :y 2 :x 1)");
    assert_eq!(result, "(1 2)");
}

#[test]
fn cl_defun_required_and_key() {
    let result = run("(cl-defun foo (a &key x) (list a x)) (foo 1 :x 2)");
    assert_eq!(result, "(1 2)");
}

#[test]
fn cl_defun_required_and_key_no_key_arg() {
    let result = run("(cl-defun foo (a &key x) (list a x)) (foo 1)");
    assert_eq!(result, "(1 nil)");
}

#[test]
fn cl_defun_optional_and_key() {
    let result = run("(cl-defun foo (a &optional b &key x) (list a b x)) (foo 1 2 :x 3)");
    assert_eq!(result, "(1 2 3)");
}

// --- &key and &aux combined ---

#[test]
fn cl_defun_key_and_aux() {
    let result = run("(cl-defun foo (&key x &aux (y (or x 99))) y) (foo)");
    assert_eq!(result, "99");
}

#[test]
fn cl_defun_key_and_aux_with_value() {
    let result = run("(cl-defun foo (&key x &aux (y (or x 99))) y) (foo :x 42)");
    assert_eq!(result, "42");
}

// --- &rest and &key ---

#[test]
fn cl_defun_rest_and_key() {
    let result = run("(cl-defun foo (&rest args &key x) (list args x)) (foo :x 1)");
    assert_eq!(result, "((:x 1) 1)");
}
