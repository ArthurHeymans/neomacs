use std::time::Duration;

use crate::{AUTO_DARK_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod detection;
mod lifecycle;
mod listeners;
mod registry;
mod themes;
mod workflows;

const AUTO_DARK_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const AUTO_DARK_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'subr-x)

(defvar auto-dark-test-ns-output nil)
(defvar auto-dark-test-mac-output nil)
(defvar auto-dark-test-dark nil)
(defvar auto-dark-test-osascript-output nil)
(defvar auto-dark-test-termux-output nil)
(defvar auto-dark-test-powershell-output nil)
(defvar auto-dark-test-registry-value nil)
(defvar auto-dark-test-dbus-result nil)
(defvar auto-dark-test-appearance nil)
(defvar auto-dark-test-dbus-names nil)
(defvar auto-dark-test-shell-output nil)
(defvar auto-dark-test-use-ns nil)
(defvar auto-dark-test-use-mac nil)
(defvar auto-dark-test-use-dbus nil)

(defun auto-dark-test-error-data (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun auto-dark-test-warning-data (thunk)
  (let (warnings)
    (cl-letf
        (((symbol-function 'display-warning)
          (lambda (type message &optional level buffer-name)
            (push
             (list type message level buffer-name)
             warnings))))
      (list
       (funcall thunk)
       (nreverse warnings)))))

(defun auto-dark-test-theme-state ()
  (list
   (copy-sequence custom-enabled-themes)
   (mapcar
    (lambda (theme)
      (list
       theme
       (and
        (custom-theme-p theme)
        t)
       (and
        (get theme 'theme-settings)
        t)
       (and
        (memq theme custom-enabled-themes)
        t)))
    '(tango-dark tango tsdh-dark tsdh-light wombat leuven))))
"##;

fn auto_dark_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_DARK_MELPA_PIN, source_file)
        .expect("prepare pinned auto-dark source below ./tmp")
        .with_prelude(AUTO_DARK_TEST_PRELUDE)
        .with_timeout(AUTO_DARK_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-dark parity test")
        .into()
}

fn assert_auto_dark_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auto_dark_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("auto-dark parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_dark_parity(elisp_form: &str, expected: Expect) {
    assert_auto_dark_source_parity("auto-dark.el", elisp_form, expected);
}

pub(crate) fn assert_auto_dark_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_dark_source_parity("auto-dark-autoloads.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_auto_dark_autoload_parity` cases (2a).
pub(crate) fn assert_auto_dark_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_dark_oracle("auto-dark-autoloads.el"),
        &name,
        "auto_dark_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_dark_parity` cases (2a).
pub(crate) fn assert_auto_dark_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_dark_oracle("auto-dark.el"),
        &name,
        "auto_dark_parity",
        cases,
    );
}
