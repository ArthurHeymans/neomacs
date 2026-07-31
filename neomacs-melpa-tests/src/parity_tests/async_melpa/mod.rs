use std::time::Duration;

use crate::{ASYNC_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod bytecomp;
mod core;
mod dired;
mod package;
mod registry;
mod smtpmail;

const ASYNC_MELPA_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const ASYNC_MELPA_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun async-melpa-test-path (filename)
  (expand-file-name
   filename
   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))

(defun async-melpa-test-read-file (filename)
  (with-temp-buffer
    (insert-file-contents-literally filename)
    (buffer-string)))

(defun async-melpa-test-write-file
    (filename content)
  (make-directory
   (file-name-directory filename)
   t)
  (with-temp-file filename
    (insert content))
  filename)

(defun async-melpa-test-wait-until
    (predicate)
  (let ((deadline
         (+ (float-time) 20)))
    (while
        (and
         (not
          (funcall predicate))
         (< (float-time)
            deadline))
      (accept-process-output
       nil 0.02))
    (unless
        (funcall predicate)
      (error
       "Timed out waiting for async fixture"))
    t))

(defun async-melpa-test-kill-buffers
    (&rest names)
  (dolist (name names)
    (when-let ((buffer
                (get-buffer name)))
      (with-current-buffer buffer
        (set-buffer-modified-p nil))
      (kill-buffer buffer))))

(defun async-melpa-test-outcome
    (thunk)
  (condition-case error-data
      (list
       :value
       (funcall thunk))
    (error
     (list
      :signal
      error-data))))
"##;

fn async_melpa_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ASYNC_MELPA_PIN, source_file)
        .expect("prepare pinned current MELPA Async source below ./tmp")
        .with_prelude(ASYNC_MELPA_TEST_PRELUDE)
        .with_timeout(ASYNC_MELPA_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed current MELPA Async parity test")
        .into()
}

fn assert_async_melpa_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = async_melpa_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("current MELPA Async case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_async_melpa_source_signal_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = async_melpa_oracle(source_file)
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("current MELPA Async signal case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_async_melpa_parity(elisp_form: &str, expected: Expect) {
    assert_async_melpa_source_parity("async.el", elisp_form, expected);
}

pub(crate) fn assert_async_melpa_signal_parity(elisp_form: &str, expected: Expect) {
    assert_async_melpa_source_signal_parity("async.el", elisp_form, expected);
}

pub(crate) fn assert_async_melpa_bytecomp_parity(elisp_form: &str, expected: Expect) {
    assert_async_melpa_source_parity("async-bytecomp.el", elisp_form, expected);
}

pub(crate) fn assert_async_melpa_dired_parity(elisp_form: &str, expected: Expect) {
    assert_async_melpa_source_parity("dired-async.el", elisp_form, expected);
}

pub(crate) fn assert_async_melpa_package_parity(elisp_form: &str, expected: Expect) {
    assert_async_melpa_source_parity("async-package.el", elisp_form, expected);
}

pub(crate) fn assert_async_melpa_smtpmail_parity(elisp_form: &str, expected: Expect) {
    assert_async_melpa_source_parity("smtpmail-async.el", elisp_form, expected);
}

pub(crate) fn assert_async_melpa_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_async_melpa_source_parity("async-autoloads.el", elisp_form, expected);
}













/// Multi-probe batch for `assert_async_melpa_autoload_parity` cases (2a).
pub(crate) fn assert_async_melpa_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        async_melpa_oracle("async-autoloads.el"),
        &name,
        "async_melpa_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_async_melpa_parity` cases (2a).
pub(crate) fn assert_async_melpa_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        async_melpa_oracle("async.el"),
        &name,
        "async_melpa_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_async_melpa_bytecomp_parity` cases (2a).
pub(crate) fn assert_async_melpa_bytecomp_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        async_melpa_oracle("async-bytecomp.el"),
        &name,
        "async_melpa_bytecomp_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_async_melpa_dired_parity` cases (2a).
pub(crate) fn assert_async_melpa_dired_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        async_melpa_oracle("dired-async.el"),
        &name,
        "async_melpa_dired_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_async_melpa_package_parity` cases (2a).
pub(crate) fn assert_async_melpa_package_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        async_melpa_oracle("async-package.el"),
        &name,
        "async_melpa_package_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_async_melpa_smtpmail_parity` cases (2a).
pub(crate) fn assert_async_melpa_smtpmail_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        async_melpa_oracle("smtpmail-async.el"),
        &name,
        "async_melpa_smtpmail_parity",
        cases,
    );
}
