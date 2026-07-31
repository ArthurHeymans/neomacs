use std::time::Duration;

use crate::{ASCII_TABLE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod commands;
mod formatting;
mod registry;
mod rendering;
mod workflows;

const ASCII_TABLE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ASCII_TABLE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun ascii-table-test-overlay-runs ()
  (let (runs)
    (dolist (overlay
             (sort
              (overlays-in
               (point-min)
               (point-max))
              (lambda (left right)
                (<
                 (overlay-start left)
                 (overlay-start right)))))
      (push
       (list
        (overlay-start overlay)
        (overlay-end overlay)
        (buffer-substring-no-properties
         (overlay-start overlay)
         (overlay-end overlay))
        (overlay-get overlay 'face))
       runs))
    (nreverse runs)))

(defun ascii-table-test-render
    (width base control escape)
  (let ((ascii-table-base base)
        (ascii-table-control control)
        (ascii-table-escape escape))
    (with-temp-buffer
      (cl-letf
          (((symbol-function 'ascii-table--width-limit)
            (lambda () width)))
        (ascii-table-mode))
      (list
       (buffer-string)
       (point)
       major-mode
       mode-name
       buffer-read-only
       revert-buffer-function
       (ascii-table-test-overlay-runs)))))
"##;

fn ascii_table_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ASCII_TABLE_MELPA_PIN, source_file)
        .expect("prepare pinned ascii-table source below ./tmp")
        .with_prelude(ASCII_TABLE_TEST_PRELUDE)
        .with_timeout(ASCII_TABLE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ascii-table parity test")
        .into()
}

fn assert_ascii_table_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ascii_table_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ascii-table parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ascii_table_parity(elisp_form: &str, expected: Expect) {
    assert_ascii_table_source_parity("ascii-table.el", elisp_form, expected);
}

pub(crate) fn assert_ascii_table_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ascii_table_source_parity("ascii-table-autoloads.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_ascii_table_autoload_parity` cases (2a).
pub(crate) fn assert_ascii_table_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        ascii_table_oracle("ascii-table-autoloads.el"),
        &name,
        "ascii_table_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_ascii_table_parity` cases (2a).
pub(crate) fn assert_ascii_table_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        ascii_table_oracle("ascii-table.el"),
        &name,
        "ascii_table_parity",
        cases,
    );
}
