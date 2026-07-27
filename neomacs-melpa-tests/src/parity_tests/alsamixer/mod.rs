use std::time::Duration;

use crate::{ALSAMIXER_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod command;
mod controls;
mod registry;
mod volume;

const ALSAMIXER_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const DETERMINISTIC_PRELUDE: &str = r##"
(require 'cl-lib)

(defvar alsamixer-test-program nil)
(defvar alsamixer-test-log-file nil)

(defun alsamixer-test-set-output (output &optional status)
  (setenv "ALSAMIXER_TEST_STDOUT" output)
  (setenv "ALSAMIXER_TEST_STATUS"
          (number-to-string (or status 0))))

(defun alsamixer-test-configure (output &optional status)
  (let* ((root (getenv "NEOMACS_TEST_SANDBOX_ROOT"))
         (program (expand-file-name "fake-amixer" root))
         (log-file (expand-file-name "fake-amixer.log" root)))
    (write-region
     "#!/bin/sh\nprintf '<%s>\\n' \"$@\" >> \"$ALSAMIXER_TEST_LOG\"\nprintf '%s' \"$ALSAMIXER_TEST_STDOUT\"\nexit \"$ALSAMIXER_TEST_STATUS\"\n"
     nil program nil 'silent)
    (set-file-modes program #o755)
    (when (file-exists-p log-file)
      (delete-file log-file))
    (setenv "ALSAMIXER_TEST_LOG" log-file)
    (alsamixer-test-set-output output status)
    (setq alsamixer-test-program program
          alsamixer-test-log-file log-file
          alsamixer-amixer-command program)
    nil))

(defun alsamixer-test-log ()
  (if (file-exists-p alsamixer-test-log-file)
      (with-temp-buffer
        (insert-file-contents alsamixer-test-log-file)
        (buffer-string))
    ""))
"##;

fn alsamixer_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALSAMIXER_MELPA_PIN, source_file)
        .expect("prepare pinned alsamixer source below ./tmp")
        .with_prelude(DETERMINISTIC_PRELUDE)
        .with_timeout(ALSAMIXER_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed alsamixer parity test")
        .into()
}

fn assert_alsamixer_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = alsamixer_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("alsamixer parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_alsamixer_parity(elisp_form: &str, expected: Expect) {
    assert_alsamixer_source_parity("alsamixer.el", elisp_form, expected);
}

pub(crate) fn assert_alsamixer_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_alsamixer_source_parity("alsamixer-autoloads.el", elisp_form, expected);
}
