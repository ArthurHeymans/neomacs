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

fn run_err(src: &str) -> bool {
    let artifact = execute_source(
        "test.el",
        &format!(";;; -*- lexical-binding: t; -*-\n{}", src),
        &[],
    );
    artifact.result.diagnostics.iter().any(|d| d.is_error())
}

// --- pcase `and` pattern ---

#[test]
fn pcase_and_two_predicates() {
    let result = run("(pcase 42 ((and (pred integerp) (pred natnump)) 'yes))");
    assert_eq!(result, "yes");
}

#[test]
fn pcase_and_first_fails() {
    let result = run("(pcase \"hello\" ((and (pred integerp) (pred stringp)) 'yes) (_ 'no))");
    assert_eq!(result, "no");
}

#[test]
fn pcase_and_binds_variables() {
    let result = run("(pcase '(1 2) ((and (pred consp) xs) (car xs)))");
    assert_eq!(result, "1");
}

#[test]
fn pcase_and_empty_matches() {
    let result = run("(pcase 5 ((and) 'yes))");
    assert_eq!(result, "yes");
}

// --- pcase `or` pattern ---

#[test]
fn pcase_or_literals() {
    let result = run("(pcase 3 ((or 1 2 3) 'small) (_ 'big))");
    assert_eq!(result, "small");
}

#[test]
fn pcase_or_no_match() {
    let result = run("(pcase 5 ((or 1 2 3) 'small) (_ 'big))");
    assert_eq!(result, "big");
}

#[test]
fn pcase_or_with_bindings() {
    let result = run("(pcase '(1 2) ((or (pred consp) (pred null)) 'got-it))");
    assert_eq!(result, "got-it");
}

#[test]
fn pcase_or_empty_no_match() {
    let result = run("(pcase 5 ((or) 'yes) (_ 'no))");
    assert_eq!(result, "no");
}

// --- pcase `not` pattern ---

#[test]
fn pcase_not_consp() {
    let result = run("(pcase 42 ((not (pred consp)) 'atom) (_ 'cons))");
    assert_eq!(result, "atom");
}

#[test]
fn pcase_not_fails_on_match() {
    let result = run("(pcase '(1 2) ((not (pred consp)) 'atom) (_ 'cons))");
    assert_eq!(result, "cons");
}

#[test]
fn pcase_not_nil() {
    let result = run("(pcase 42 ((not (pred null)) 'truthy) (_ 'falsy))");
    assert_eq!(result, "truthy");
}

#[test]
fn pcase_nil_literal_match() {
    let result = run("(pcase nil ('nil 'is-nil) (_ 'not-nil))");
    assert_eq!(result, "is-nil");
}

#[test]
fn pcase_nil_literal_no_match() {
    let result = run("(pcase 42 ('nil 'is-nil) (_ 'not-nil))");
    assert_eq!(result, "not-nil");
}

// --- pcase `app` pattern ---

#[test]
fn pcase_app_car() {
    let result = run("(pcase '(10 20 30) ((app car x) x))");
    assert_eq!(result, "10");
}

#[test]
fn pcase_app_length() {
    let result = run("(pcase '(a b c) ((app length n) n))");
    assert_eq!(result, "3");
}

#[test]
fn pcase_app_with_predicate() {
    let result = run("(pcase '(a b c) ((app length (pred natnump)) 'long-enough) (_ 'too-short))");
    assert_eq!(result, "long-enough");
}

#[test]
fn pcase_app_with_literal() {
    let result = run("(pcase '(a b c) ((app length 3) 'three) (_ 'other))");
    assert_eq!(result, "three");
}

// --- pcase combined patterns ---

#[test]
fn pcase_and_or_combined() {
    let result = run("(pcase 5 ((and (or 4 5 6) (pred integerp)) 'in-range))");
    assert_eq!(result, "in-range");
}

#[test]
fn pcase_guard_inside_and() {
    let result = run("(pcase 7 ((and x (guard (> x 5))) (format \"big: %d\" x)))");
    assert_eq!(result, "\"big: 7\"");
}

// --- cl-assert ---

#[test]
fn cl_assert_true_passes() {
    let result = run("(progn (cl-assert t) 'passed)");
    assert_eq!(result, "passed");
}

#[test]
fn cl_assert_nonnil_passes() {
    let result = run("(progn (cl-assert (> 5 3)) 'passed)");
    assert_eq!(result, "passed");
}

#[test]
fn cl_assert_nil_signals_error() {
    assert!(run_err("(progn (cl-assert nil) 'should-not-reach)"));
}

#[test]
fn cl_assert_false_condition_signals_error() {
    assert!(run_err("(progn (cl-assert (< 5 3)) 'should-not-reach)"));
}

#[test]
fn cl_assert_returns_value() {
    let result = run("(cl-assert 42)");
    assert_eq!(result, "42");
}

// --- cl-check-type ---

#[test]
fn cl_check_type_correct_type() {
    let result = run("(progn (cl-check-type 42 integer) 'ok)");
    assert_eq!(result, "ok");
}

#[test]
fn cl_check_type_wrong_type_signals_error() {
    assert!(run_err(
        "(progn (cl-check-type \"hello\" integer) 'should-not-reach)"
    ));
}

#[test]
fn cl_check_type_string_is_string() {
    let result = run("(progn (cl-check-type \"hello\" string) 'ok)");
    assert_eq!(result, "ok");
}

#[test]
fn cl_check_type_returns_value() {
    let result = run("(cl-check-type 42 integer)");
    assert_eq!(result, "42");
}

#[test]
fn cl_check_type_consp_check() {
    assert!(run_err("(progn (cl-check-type 42 cons) 'should-not-reach)"));
}
