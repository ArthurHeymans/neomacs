use std::time::Duration;

use crate::{ASILEA_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod acceptance;
mod configuration;
mod engine;
mod options;
mod process;
mod registry;
mod workflows;

const ASILEA_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ASILEA_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defvar asilea-test-events nil)
(defvar asilea-test-process-specs nil)
(defvar asilea-test-pending nil)
(defvar asilea-test-next-process-id 0)
(defvar asilea-test-buffers nil)

(defun asilea-test-reset
    (specs)
  (setq asilea-test-events nil
        asilea-test-process-specs
        (copy-tree specs)
        asilea-test-pending nil
        asilea-test-next-process-id 0
        asilea-test-buffers nil))

(defun asilea-test-start-process
    (program state options)
  (let* ((spec
          (or
           (pop asilea-test-process-specs)
           '("finished\n" "0")))
         (id
          (cl-incf
           asilea-test-next-process-id))
         (buffer
          (generate-new-buffer
           (format
            " *asilea-test-process-%d*"
            id)))
         (process
          (vector
           id
           program
           (copy-sequence state)
           (asilea--state-to-option-list
            state
            options)
           (nth 0 spec)
           (nth 1 spec)
           buffer)))
    (with-current-buffer buffer
      (insert
       (nth 1 spec)))
    (push buffer asilea-test-buffers)
    (push
     (list
      :start
      id
      program
      (append state nil)
      (aref process 3)
      (nth 0 spec)
      (nth 1 spec))
     asilea-test-events)
    process))

(defun asilea-test-process-buffer
    (process)
  (aref process 6))

(defun asilea-test-set-process-sentinel
    (process sentinel)
  (setq asilea-test-pending
        (nconc
         asilea-test-pending
         (list
          (cons process sentinel))))
  (push
   (list
    :sentinel
    (aref process 0))
   asilea-test-events)
  sentinel)

(defun asilea-test-tick ()
  (when asilea-test-pending
    (let* ((entry
            (pop asilea-test-pending))
           (process
            (car entry))
           (sentinel
            (cdr entry)))
      (push
       (list
        :complete
        (aref process 0)
        (aref process 4))
       asilea-test-events)
      (funcall
       sentinel
       process
       (aref process 4))
      t)))

(defun asilea-test-drain ()
  (while
      (asilea-test-tick))
  (nreverse
   asilea-test-events))

(defun asilea-test-cleanup ()
  (dolist (buffer asilea-test-buffers)
    (when
        (buffer-live-p buffer)
      (kill-buffer buffer)))
  (setq asilea-test-buffers nil))
"##;

fn asilea_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ASILEA_MELPA_PIN, source_file)
        .expect("prepare pinned asilea source below ./tmp")
        .with_prelude(ASILEA_TEST_PRELUDE)
        .with_timeout(ASILEA_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed asilea parity test").into()
}

fn assert_asilea_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = asilea_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("asilea parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_asilea_parity(elisp_form: &str, expected: Expect) {
    assert_asilea_source_parity("asilea.el", elisp_form, expected);
}

pub(crate) fn assert_asilea_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_asilea_source_parity("asilea-autoloads.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_asilea_autoload_parity` cases (2a).
pub(crate) fn assert_asilea_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        asilea_oracle("asilea-autoloads.el"),
        &name,
        "asilea_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_asilea_parity` cases (2a).
pub(crate) fn assert_asilea_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(asilea_oracle("asilea.el"), &name, "asilea_parity", cases);
}
