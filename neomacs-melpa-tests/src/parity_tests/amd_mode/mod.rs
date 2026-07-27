use std::time::Duration;

use crate::{AMD_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod editing;
mod paths;
mod references;
mod registry;

const AMD_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const DETERMINISTIC_PRELUDE: &str = r##"
(require 'cl-lib)

(setq js2-mode-show-parse-errors nil
      js2-mode-show-strict-warnings nil)

(defun amd-test-project (name)
  (let ((root
         (expand-file-name
          name
          (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
    (make-directory root t)
    (write-region "" nil
                  (expand-file-name ".projectile" root)
                  nil 'silent)
    (file-name-as-directory root)))

(defun amd-test-write (root relative contents)
  (let ((file (expand-file-name relative root)))
    (make-directory (file-name-directory file) t)
    (write-region contents nil file nil 'silent)
    file))

(defun amd-test-read (file)
  (with-temp-buffer
    (insert-file-contents file)
    (buffer-string)))

(defun amd-test-parse ()
  (js2-mode)
  (amd-mode 1)
  (js2-parse)
  (current-buffer))

(defun amd-test-open (root relative contents)
  (let ((file (amd-test-write root relative contents)))
    (find-file-noselect file)))

(defun amd-test-configure-ag (root output)
  (let ((program (expand-file-name "ag" root))
        (log-file (expand-file-name "ag.log" root)))
    (write-region
     "#!/bin/sh\nprintf '<%s>\\n' \"$@\" > \"$AMD_TEST_AG_LOG\"\nprintf '%s' \"$AMD_TEST_AG_OUTPUT\"\n"
     nil program nil 'silent)
    (set-file-modes program #o755)
    (setenv "AMD_TEST_AG_LOG" log-file)
    (setenv "AMD_TEST_AG_OUTPUT" output)
    (setq exec-path (cons root exec-path))
    (setenv "PATH"
            (concat root path-separator (getenv "PATH")))
    log-file))
"##;

fn amd_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AMD_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned amd-mode source and dependencies below ./tmp")
        .with_prelude(DETERMINISTIC_PRELUDE)
        .with_timeout(AMD_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed amd-mode parity test")
        .into()
}

fn assert_amd_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = amd_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("amd-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_amd_mode_parity(elisp_form: &str, expected: Expect) {
    assert_amd_mode_source_parity("amd-mode.el", elisp_form, expected);
}

pub(crate) fn assert_amd_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_amd_mode_source_parity("amd-mode-autoloads.el", elisp_form, expected);
}
