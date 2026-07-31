use std::time::Duration;

use crate::{
    AUTO_COMPLETE_C_HEADERS_MELPA_PIN, AUTO_COMPLETE_MELPA_PIN, CachedMelpaOracle, POPUP_MELPA_PIN,
};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod candidates;
mod filesystem;
mod options;
mod registry;
mod workflows;

const AUTO_COMPLETE_C_HEADERS_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTO_COMPLETE_C_HEADERS_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defun achead-test-error (thunk)
  (condition-case error-data
      (list :value (funcall thunk))
    (error
     (list :signal
           (car error-data)
           (cdr error-data)))))

(defun achead-test-reset-directory (directory)
  (when (file-exists-p directory)
    (delete-directory directory t))
  (make-directory directory t)
  directory)

(defun achead-test-write-file (root relative content)
  (let ((file (expand-file-name relative root)))
    (make-directory (file-name-directory file) t)
    (with-temp-file file
      (insert content))
    file))

(defun achead-test-relative-results (results root)
  (mapcar
   (lambda (entry)
     (cons
      (car entry)
      (file-relative-name (cdr entry) root)))
   results))
"##;

fn auto_complete_c_headers_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_COMPLETE_C_HEADERS_MELPA_PIN, source_file)
        .expect("prepare pinned auto-complete-c-headers source below ./tmp")
        .with_melpa_dependency(AUTO_COMPLETE_MELPA_PIN)
        .expect("prepare pinned auto-complete dependency below ./tmp")
        .with_melpa_dependency(POPUP_MELPA_PIN)
        .expect("prepare pinned popup dependency below ./tmp")
        .with_prelude(AUTO_COMPLETE_C_HEADERS_TEST_PRELUDE)
        .with_timeout(AUTO_COMPLETE_C_HEADERS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-complete-c-headers parity test")
        .into()
}

fn assert_auto_complete_c_headers_source_parity(
    source_file: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = auto_complete_c_headers_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("auto-complete-c-headers parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_complete_c_headers_parity(elisp_form: &str, expected: Expect) {
    assert_auto_complete_c_headers_source_parity(
        "auto-complete-c-headers.el",
        elisp_form,
        expected,
    );
}

pub(crate) fn assert_auto_complete_c_headers_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_complete_c_headers_source_parity(
        "auto-complete-c-headers-autoloads.el",
        elisp_form,
        expected,
    );
}

/// Multi-probe batch for `assert_auto_complete_c_headers_autoload_parity` cases (2a).
pub(crate) fn assert_auto_complete_c_headers_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_complete_c_headers_oracle("auto-complete-c-headers-autoloads.el"),
        &name,
        "auto_complete_c_headers_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_complete_c_headers_parity` cases (2a).
pub(crate) fn assert_auto_complete_c_headers_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_complete_c_headers_oracle("auto-complete-c-headers.el"),
        &name,
        "auto_complete_c_headers_parity",
        cases,
    );
}
