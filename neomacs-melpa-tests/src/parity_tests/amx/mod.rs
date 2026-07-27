use std::time::Duration;

use crate::{AMX_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod backends;
mod bindings;
mod persistence;
mod ranking;
mod registry;

const AMX_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const DETERMINISTIC_PRELUDE: &str = r##"
(require 'cl-lib)

(defvar amx-test-timer-counter 0)
(defvar amx-test-timer-events nil)

(defun amx-test-run-with-idle-timer
    (seconds repeat function &rest arguments)
  (let ((timer
         (intern
          (format "amx-test-timer-%d"
                  (cl-incf amx-test-timer-counter)))))
    (push
     (list 'schedule timer seconds repeat function arguments)
     amx-test-timer-events)
    timer))

(defun amx-test-cancel-timer (timer)
  (push (list 'cancel timer) amx-test-timer-events)
  nil)

(fset 'run-with-idle-timer #'amx-test-run-with-idle-timer)
(fset 'cancel-timer #'amx-test-cancel-timer)

(defun amx-test-root (name)
  (let ((root
         (expand-file-name
          name
          (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
    (make-directory root t)
    (file-name-as-directory root)))

(defun amx-test-write (file contents)
  (make-directory (file-name-directory file) t)
  (write-region contents nil file nil 'silent)
  file)

(defun amx-test-read (file)
  (with-temp-buffer
    (insert-file-contents file)
    (buffer-string)))

(defun amx-test-alpha ()
  (interactive)
  'alpha-ran)

(defun amx-test-beta ()
  (interactive)
  'beta-ran)

(defun amx-test-gamma ()
  (interactive)
  'gamma-ran)

(defun amx-test-mouse (event)
  (interactive "e")
  event)

(defun amx-test-noncommand ()
  'not-interactive)
"##;

fn amx_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AMX_MELPA_PIN, source_file)
        .expect("prepare pinned amx source and dependencies below ./tmp")
        .with_prelude(DETERMINISTIC_PRELUDE)
        .with_timeout(AMX_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed amx parity test").into()
}

fn assert_amx_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = amx_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("amx parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_amx_parity(elisp_form: &str, expected: Expect) {
    assert_amx_source_parity("amx.el", elisp_form, expected);
}

pub(crate) fn assert_amx_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_amx_source_parity("amx-autoloads.el", elisp_form, expected);
}
