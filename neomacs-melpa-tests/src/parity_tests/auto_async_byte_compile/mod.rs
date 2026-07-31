use std::time::Duration;

use crate::{AUTO_ASYNC_BYTE_COMPILE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod mode;
mod process;
mod registry;
mod report;
mod status;

const AUTO_ASYNC_BYTE_COMPILE_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const AUTO_ASYNC_BYTE_COMPILE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'subr-x)

(defun auto-async-byte-compile-test-error-data (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun auto-async-byte-compile-test-read-file (path)
  (with-temp-buffer
    (insert-file-contents-literally path)
    (buffer-string)))

(defun auto-async-byte-compile-test-wait (process)
  (let ((remaining 600))
    (while (and
            (> remaining 0)
            (process-live-p process))
      (setq remaining
            (1- remaining))
      (accept-process-output process 0.05))
    (accept-process-output process 0.05)
    (list
     (> remaining 0)
     (process-status process)
     (process-exit-status process))))
"##;

fn auto_async_byte_compile_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_ASYNC_BYTE_COMPILE_MELPA_PIN, source_file)
        .expect("prepare pinned auto-async-byte-compile source below ./tmp")
        .with_prelude(AUTO_ASYNC_BYTE_COMPILE_TEST_PRELUDE)
        .with_timeout(AUTO_ASYNC_BYTE_COMPILE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-async-byte-compile parity test")
        .into()
}

fn assert_auto_async_byte_compile_source_parity(
    source_file: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = auto_async_byte_compile_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("auto-async-byte-compile parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_async_byte_compile_parity(elisp_form: &str, expected: Expect) {
    assert_auto_async_byte_compile_source_parity(
        "auto-async-byte-compile.el",
        elisp_form,
        expected,
    );
}

pub(crate) fn assert_auto_async_byte_compile_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_async_byte_compile_source_parity(
        "auto-async-byte-compile-autoloads.el",
        elisp_form,
        expected,
    );
}

/// Multi-probe batch for `assert_auto_async_byte_compile_autoload_parity` cases (2a).
pub(crate) fn assert_auto_async_byte_compile_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_async_byte_compile_oracle("auto-async-byte-compile-autoloads.el"),
        &name,
        "auto_async_byte_compile_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_async_byte_compile_parity` cases (2a).
pub(crate) fn assert_auto_async_byte_compile_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_async_byte_compile_oracle("auto-async-byte-compile.el"),
        &name,
        "auto_async_byte_compile_parity",
        cases,
    );
}
