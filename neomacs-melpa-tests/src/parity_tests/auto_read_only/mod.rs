use std::time::Duration;

use crate::{AUTO_READ_ONLY_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod hook;
mod matching;
mod mode;
mod registry;
mod workflows;

const AUTO_READ_ONLY_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTO_READ_ONLY_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'project)
(require 'view)

(defun auto-read-only-test-error-data (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun auto-read-only-test-hook-count (function hook)
  (length
   (seq-filter
    (lambda (candidate)
      (eq candidate function))
    (symbol-value hook))))

(defun auto-read-only-test-buffer-state ()
  (list
   (buffer-name)
   buffer-file-name
   (buffer-string)
   (point)
   buffer-read-only
   (bound-and-true-p view-mode)
   (buffer-modified-p)))
"##;

fn auto_read_only_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_READ_ONLY_MELPA_PIN, source_file)
        .expect("prepare pinned auto-read-only source below ./tmp")
        .with_prelude(AUTO_READ_ONLY_TEST_PRELUDE)
        .with_timeout(AUTO_READ_ONLY_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-read-only parity test")
        .into()
}

fn assert_auto_read_only_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auto_read_only_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("auto-read-only parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_read_only_parity(elisp_form: &str, expected: Expect) {
    assert_auto_read_only_source_parity("auto-read-only.el", elisp_form, expected);
}

pub(crate) fn assert_auto_read_only_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_read_only_source_parity("auto-read-only-autoloads.el", elisp_form, expected);
}





/// Multi-probe batch for `assert_auto_read_only_autoload_parity` cases (2a).
pub(crate) fn assert_auto_read_only_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auto_read_only_oracle("auto-read-only-autoloads.el"),
        &name,
        "auto_read_only_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_read_only_parity` cases (2a).
pub(crate) fn assert_auto_read_only_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auto_read_only_oracle("auto-read-only.el"),
        &name,
        "auto_read_only_parity",
        cases,
    );
}
