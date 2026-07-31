use std::time::Duration;

use crate::{AUTO_INDENT_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod editing;
mod hooks;
mod kill;
mod lifecycle;
mod registry;
mod repository;
mod workflows;

const AUTO_INDENT_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTO_INDENT_MODE_TEST_PRELUDE: &str = r##"
(require 'cl)
(require 'cl-lib)

(defun auto-indent-test-error (thunk)
  (condition-case error-data
      (list :value (funcall thunk))
    (error
     (list :signal (car error-data) (cdr error-data)))))

(defun auto-indent-test-relative-or-value (value root)
  (if (stringp value)
      (file-relative-name value root)
    value))

(defun auto-indent-test-advice-state (function)
  (list
   function
   (not
    (null
     (ad-find-advice
      function 'around 'auto-indent-mode-advice)))
   (ad-is-active function)))
"##;

fn auto_indent_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_INDENT_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned auto-indent-mode source below ./tmp")
        .with_prelude(AUTO_INDENT_MODE_TEST_PRELUDE)
        .with_timeout(AUTO_INDENT_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-indent-mode parity test")
        .into()
}

fn assert_auto_indent_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auto_indent_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("auto-indent-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_indent_mode_parity(elisp_form: &str, expected: Expect) {
    assert_auto_indent_mode_source_parity("auto-indent-mode.el", elisp_form, expected);
}

pub(crate) fn assert_auto_indent_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_indent_mode_source_parity("auto-indent-mode-autoloads.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_auto_indent_mode_autoload_parity` cases (2a).
pub(crate) fn assert_auto_indent_mode_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_indent_mode_oracle("auto-indent-mode-autoloads.el"),
        &name,
        "auto_indent_mode_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_indent_mode_parity` cases (2a).
pub(crate) fn assert_auto_indent_mode_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_indent_mode_oracle("auto-indent-mode.el"),
        &name,
        "auto_indent_mode_parity",
        cases,
    );
}
