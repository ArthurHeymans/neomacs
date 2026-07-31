use std::time::Duration;

use crate::{APHELEIA_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const APHELEIA_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const APHELEIA_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defvar apheleia-test-callback-result :not-called)
(defvar apheleia-test-hook-events nil)

(defun apheleia-test-root (name)
  (file-name-as-directory
   (expand-file-name
    name
    (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))

(defun apheleia-test-cleanup (root)
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

(defun apheleia-test-await (predicate description)
  (let ((attempts 0))
    (while
        (and
         (not (funcall predicate))
         (< attempts 1000))
      (setq attempts (1+ attempts))
      (accept-process-output nil 0.01))
    (unless
        (funcall predicate)
      (error "Timed out waiting for %s" description))))

(defun apheleia-test-await-callback ()
  (apheleia-test-await
   (lambda ()
     (not
      (eq apheleia-test-callback-result :not-called)))
   "Apheleia callback")
  apheleia-test-callback-result)

(defun apheleia-test-format-buffer
    (formatter)
  (setq apheleia-test-callback-result :not-called)
  (apheleia-format-buffer
   formatter
   nil
   :callback
   (lambda (&rest properties)
     (setq apheleia-test-callback-result properties)))
  (apheleia-test-await-callback))

(defun apheleia-test-read-file
    (path)
  (with-temp-buffer
    (insert-file-contents-literally path)
    (buffer-string)))

"##;

fn apheleia_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(APHELEIA_MELPA_PIN, source_file)
        .expect("prepare pinned Apheleia source below ./tmp")
        .with_prelude(APHELEIA_TEST_PRELUDE)
        .with_timeout(APHELEIA_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed Apheleia parity test")
        .into()
}

fn assert_apheleia_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apheleia_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("Apheleia parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apheleia_parity(elisp_form: &str, expected: Expect) {
    assert_apheleia_source_parity("apheleia.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_apheleia_parity` cases (2a).
pub(crate) fn assert_apheleia_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        apheleia_oracle("apheleia.el"),
        &name,
        "apheleia_parity",
        cases,
    );
}
