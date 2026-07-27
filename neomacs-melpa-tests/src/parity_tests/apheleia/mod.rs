use std::time::Duration;

use crate::{APHELEIA_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod dp_rcs;
mod formatting;
mod logging;
mod mode;
mod registry;
mod selection;
mod utils;

const APHELEIA_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const APHELEIA_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defvar apheleia-test-callback-result :not-called)
(defvar apheleia-test-hook-events nil)

(defun apheleia-test-await-callback ()
  (let ((attempts 0))
    (while (and
            (eq apheleia-test-callback-result :not-called)
            (< attempts 1000))
      (setq attempts (1+ attempts))
      (accept-process-output nil 0.01))
    (when (eq apheleia-test-callback-result :not-called)
      (error "Apheleia callback did not run"))
    apheleia-test-callback-result))

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

(defun apheleia-test-write-file
    (relative content)
  (let ((path
         (expand-file-name
          relative
          default-directory)))
    (make-directory
     (file-name-directory path)
     t)
    (with-temp-file path
      (insert content))
    path))

(defun apheleia-test-read-file
    (path)
  (with-temp-buffer
    (insert-file-contents-literally path)
    (buffer-string)))

(defun apheleia-test-table
    (before after)
  (let ((table
         (apheleia--edit-distance-table
          before
          after)))
    (mapcar
     (lambda (row)
       (mapcar
        (lambda (column)
          (gethash
           (cons column row)
           table))
        (number-sequence
         0
         (length before))))
     (number-sequence
      0
      (length after)))))

(defun apheleia-test-kill-buffers
    (regexp)
  (dolist (buffer (buffer-list))
    (when (string-match-p
           regexp
           (buffer-name buffer))
      (kill-buffer buffer))))
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

pub(crate) fn assert_apheleia_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_apheleia_source_parity("apheleia-autoloads.el", elisp_form, expected);
}
