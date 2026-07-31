use std::time::Duration;

use crate::{APEL_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod workflows;

const APEL_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const APEL_TEST_PRELUDE: &str = r####"
(require 'cl-lib)

(defun neomacs-apel-test-file-string (file)
  (with-temp-buffer
    (insert-file-contents-literally file)
    (buffer-string)))

(defun neomacs-apel-test-file-bytes (file)
  (with-temp-buffer
    (set-buffer-multibyte nil)
    (insert-file-contents-literally file)
    (string-to-list (buffer-string))))

(defun neomacs-apel-test-cleanup (root)
  (dolist (buffer (buffer-list))
    (let ((file (buffer-file-name buffer)))
      (when
          (and file (string-prefix-p root file))
        (with-current-buffer buffer
          (set-buffer-modified-p nil))
        (kill-buffer buffer))))
  (when
      (file-exists-p root)
    (delete-directory root t)))
"####;

fn apel_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(APEL_MELPA_PIN, source_file)
        .expect("prepare pinned APEL source below ./tmp")
        .with_prelude(APEL_TEST_PRELUDE)
        .with_timeout(APEL_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed APEL parity test").into()
}

pub(crate) fn assert_apel_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apel_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("APEL parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch loading one source file (2a).
pub(crate) fn assert_apel_source_batch(source_file: &str, cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        apel_oracle(source_file),
        &name,
        "apel_source_batch",
        cases,
    );
}
