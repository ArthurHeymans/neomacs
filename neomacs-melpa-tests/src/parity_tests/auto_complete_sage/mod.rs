use std::time::Duration;

use crate::{
    AUTO_COMPLETE_MELPA_PIN, AUTO_COMPLETE_SAGE_MELPA_PIN, CachedMelpaOracle, DEFERRED_MELPA_PIN,
    LET_ALIST_GNU_ELPA_PIN, POPUP_MELPA_PIN, SAGE_SHELL_MODE_MELPA_PIN,
};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod cache_docs;
mod edit;
mod registry;
mod repl;
mod workflows;

const AUTO_COMPLETE_SAGE_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const AUTO_COMPLETE_SAGE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun acsage-test-error (thunk)
  (condition-case error-data
      (list :value
            (funcall thunk))
    (error
     (list :signal
           (car error-data)
           (cdr error-data)))))

(defun acsage-test-posn-at-point (&rest _arguments)
  'acsage-test-position)

(defun acsage-test-posn-col-row (_position)
  (cons
   (current-column)
   (line-number-at-pos
    (point))))

(fset 'posn-at-point
      #'acsage-test-posn-at-point)
(fset 'posn-col-row
      #'acsage-test-posn-col-row)
"##;

fn auto_complete_sage_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_COMPLETE_SAGE_MELPA_PIN, source_file)
        .expect("prepare pinned auto-complete-sage source below ./tmp")
        .with_melpa_dependency(POPUP_MELPA_PIN)
        .expect("prepare pinned popup dependency below ./tmp")
        .with_melpa_dependency(AUTO_COMPLETE_MELPA_PIN)
        .expect("prepare pinned auto-complete dependency below ./tmp")
        .with_melpa_dependency(DEFERRED_MELPA_PIN)
        .expect("prepare pinned deferred dependency below ./tmp")
        .with_gnu_elpa_dependency(LET_ALIST_GNU_ELPA_PIN)
        .expect("prepare pinned let-alist dependency below ./tmp")
        .with_melpa_dependency(SAGE_SHELL_MODE_MELPA_PIN)
        .expect("prepare pinned sage-shell-mode dependency below ./tmp")
        .with_prelude(AUTO_COMPLETE_SAGE_TEST_PRELUDE)
        .with_timeout(AUTO_COMPLETE_SAGE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-complete-sage parity test")
        .into()
}

fn assert_auto_complete_sage_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auto_complete_sage_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("auto-complete-sage parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_complete_sage_parity(elisp_form: &str, expected: Expect) {
    assert_auto_complete_sage_source_parity("auto-complete-sage.el", elisp_form, expected);
}

pub(crate) fn assert_auto_complete_sage_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_complete_sage_source_parity(
        "auto-complete-sage-autoloads.el",
        elisp_form,
        expected,
    );
}

/// Multi-probe batch for `assert_auto_complete_sage_autoload_parity` cases (2a).
pub(crate) fn assert_auto_complete_sage_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_complete_sage_oracle("auto-complete-sage-autoloads.el"),
        &name,
        "auto_complete_sage_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_complete_sage_parity` cases (2a).
pub(crate) fn assert_auto_complete_sage_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_complete_sage_oracle("auto-complete-sage.el"),
        &name,
        "auto_complete_sage_parity",
        cases,
    );
}
