//! Divergence tests: subr argument introspection, backtrace, profiling stubs.

use super::common::assert_oracle_parity_with_bootstrap;
use super::common::return_if_neovm_enable_oracle_proptest_not_set;

#[test]
fn divergence_subr_arity() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (subr-arity (symbol-function 'car))
  (subr-arity (symbol-function 'list))
  (subr-arity (symbol-function 'format))
  (subr-arity (symbol-function '+)))"#,
    );
}

#[test]
fn divergence_subr_type() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (subrp (symbol-function 'car))
  (subrp (symbol-function 'list))
  (subrp (lambda (x) x))
  (byte-code-function-p (symbol-function 'car))
  (subr-name (symbol-function 'car)))"#,
    );
}

#[test]
fn divergence_interactive_spec() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (commandp 'forward-char)
  (commandp 'car)
  (interactive-form 'forward-char)
  (interactive-form 'car))"#,
    );
}

#[test]
fn divergence_function_documentation() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (documentation 'car)
  (documentation 'list)
  (documentation 'not-a-real-function-xyz))"#,
    );
}

#[test]
fn divergence_backtrace_on_error() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((backtrace-on-error-interactive t))
  (list backtrace-on-error-interactive
        (booleanp backtrace-on-error-interactive)))"#,
    );
}

#[test]
fn divergence_profiler_supported() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list (featurep 'profiler)
              (functionp 'profiler-start))"#,
    );
}

#[test]
fn divergence_core_emacs_version() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (stringp emacs-version)
  (stringp (emacs-version))
  (>= emacs-major-version 28)
  (integerp emacs-major-version)
  (integerp emacs-minor-version))"#,
    );
}

#[test]
fn divergence_system_info() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (stringp system-type)
  (stringp system-name)
  (stringp user-login-name)
  (stringp user-full-name)
  (stringp user-emacs-directory)
  (file-name-absolute-p user-emacs-directory))"#,
    );
}

#[test]
fn divergence_caret_identity() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(let ((x (list 1 2 3)))
  (list (eq x x)
        (eq (car x) (car x))
        (eql 1.0 1.0)
        (equal '(1 2 3) '(1 2 3))
        (eq '(1 2 3) '(1 2 3))))"#,
    );
}

#[test]
fn divergence_noreorder() {
    return_if_neovm_enable_oracle_proptest_not_set!();

    assert_oracle_parity_with_bootstrap(
        r#"(list
  (number-or-marker-p 42)
  (number-or-marker-p (point-marker))
  (number-or-marker-p "string")
  (booleanp t)
  (booleanp nil)
  (booleanp 0))"#,
    );
}
