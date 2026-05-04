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

// --- fboundp for built-in primitives ---

#[test]
fn fboundp_car() {
    assert_eq!(run("(fboundp 'car)"), "t");
}

#[test]
fn fboundp_cdr() {
    assert_eq!(run("(fboundp 'cdr)"), "t");
}

#[test]
fn fboundp_cons() {
    assert_eq!(run("(fboundp 'cons)"), "t");
}

#[test]
fn fboundp_plus() {
    assert_eq!(run("(fboundp '+)"), "t");
}

#[test]
fn fboundp_length() {
    assert_eq!(run("(fboundp 'length)"), "t");
}

#[test]
fn fboundp_list() {
    assert_eq!(run("(fboundp 'list)"), "t");
}

#[test]
fn fboundp_unknown() {
    assert_eq!(run("(fboundp 'nonexistent-foo-bar)"), "nil");
}

#[test]
fn fboundp_defun() {
    assert_eq!(run("(defun my-test-fn (x) x) (fboundp 'my-test-fn)"), "t");
}

#[test]
fn fboundp_arithmetic_ops() {
    assert_eq!(
        run("(and (fboundp '+) (fboundp '-) (fboundp '*) (fboundp '/))"),
        "t"
    );
}

#[test]
fn fboundp_comparison_ops() {
    assert_eq!(
        run("(and (fboundp '=) (fboundp '<) (fboundp '>) (fboundp '<=) (fboundp '>=))"),
        "t"
    );
}

#[test]
fn fboundp_string_ops() {
    assert_eq!(
        run("(and (fboundp 'string=) (fboundp 'substring) (fboundp 'concat))"),
        "t"
    );
}

// --- functionp for builtins ---

#[test]
fn functionp_car() {
    assert_eq!(run("(functionp 'car)"), "t");
}

#[test]
fn functionp_lambda() {
    assert_eq!(run("(functionp (lambda (x) x))"), "t");
}
