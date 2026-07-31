use std::time::Duration;

use crate::{ARDUINO_MODE_FLYCHECK_MELPA_PIN, ARDUINO_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod ede_makefile;
mod ede_preferences;
mod ede_projects;
mod editing;
mod flycheck;
mod org_babel;
mod processes;
mod surface;
mod workflows;

const ARDUINO_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const ARDUINO_MODE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defun neomacs-arduino-mode-test-write-file (path content)
  (make-directory (file-name-directory path) t)
  (with-temp-buffer
    (insert content)
    (write-region (point-min) (point-max) path nil 'silent))
  path)

(defun neomacs-arduino-mode-test-read-file (path)
  (with-temp-buffer
    (insert-file-contents-literally path)
    (buffer-string)))

(defun neomacs-arduino-mode-test-fixture ()
  (let* ((root
          (file-name-as-directory
           (expand-file-name
            "customer firmware"
            (getenv "NEOMACS_TEST_SANDBOX_ROOT"))))
         (sketch
          (expand-file-name
           "greenhouse monitor/greenhouse_monitor.ino"
           root))
         (executable
          (expand-file-name "bin/arduino" root))
         (call-log
          (expand-file-name "arduino-call.log" root))
         (gate
          (expand-file-name "continue-process" root))
         (previous-call-log
          (getenv "NEOMACS_ARDUINO_MODE_CALL_LOG"))
         (previous-gate
          (getenv "NEOMACS_ARDUINO_MODE_GATE")))
    (neomacs-arduino-mode-test-write-file
     sketch
     (concat
      "const int sensorPin = A0;\n"
      "void setup() { Serial.begin(115200); }\n"
      "void loop() { Serial.println(analogRead(sensorPin)); }\n"))
    (neomacs-arduino-mode-test-write-file
     executable
     (concat
      "#!/bin/sh\n"
      "set -eu\n"
      "{\n"
      "  printf 'cwd=%s\\n' \"$PWD\"\n"
      "  for argument in \"$@\"; do\n"
      "    printf 'arg=%s\\n' \"$argument\"\n"
      "  done\n"
      "} > \"${NEOMACS_ARDUINO_MODE_CALL_LOG:?}\"\n"
      "while [ ! -e \"${NEOMACS_ARDUINO_MODE_GATE:?}\" ]; do\n"
      "  sleep 0.01\n"
      "done\n"
      "case \"${1-}\" in\n"
      "  --upload)\n"
      "    IFS= read -r first_line < \"$2\"\n"
      "    printf 'Sketch uses 924 bytes (2%%) of program storage space.\\n'\n"
      "    printf 'Uploaded %s\\n' \"$2\"\n"
      "    printf 'CLI read: %s\\n' \"$first_line\"\n"
      "    ;;\n"
      "  --verify)\n"
      "    printf 'Verifying %s\\n' \"$2\" >&2\n"
      "    printf '%s:7:3: error: sensorPin was not declared in this scope\\n' \"$2\" >&2\n"
      "    exit 17\n"
      "    ;;\n"
      "  *)\n"
      "    printf 'unexpected invocation\\n' >&2\n"
      "    exit 64\n"
      "    ;;\n"
      "esac\n"))
    (set-file-modes executable #o755)
    (setenv "NEOMACS_ARDUINO_MODE_CALL_LOG" call-log)
    (setenv "NEOMACS_ARDUINO_MODE_GATE" gate)
    (list
     :root root
     :sketch sketch
     :executable executable
     :call-log call-log
     :gate gate
     :previous-call-log previous-call-log
     :previous-gate previous-gate)))

(defun neomacs-arduino-mode-test-sentinel (process event)
  (let ((original
         (process-get
          process
          'neomacs-arduino-mode-test-original-sentinel)))
    (when original
      (funcall original process event)))
  ;; Recording after the package sentinel returns guarantees that every
  ;; process-buffer, mode-line, spinner, display, and message side effect is
  ;; observable when the wait helper completes.
  (process-put
   process
   'neomacs-arduino-mode-test-completion-event
   event))

(defun neomacs-arduino-mode-test-observe (process)
  (process-put
   process
   'neomacs-arduino-mode-test-original-sentinel
   (process-sentinel process))
  (process-put
   process
   'neomacs-arduino-mode-test-completion-event
   nil)
  (set-process-sentinel
   process
   #'neomacs-arduino-mode-test-sentinel)
  process)

(defun neomacs-arduino-mode-test-wait (process)
  (let ((remaining 400))
    (while
        (and
         (> remaining 0)
         (not
          (process-get
           process
           'neomacs-arduino-mode-test-completion-event)))
      (setq remaining (1- remaining))
      (accept-process-output process 0.05))
    (unless
        (process-get
         process
         'neomacs-arduino-mode-test-completion-event)
      (error
       "Timed out waiting for Arduino process sentinel: %s"
       (process-name process)))
    (list
     t
     (process-status process)
     (process-exit-status process)
     (process-get
      process
      'neomacs-arduino-mode-test-completion-event))))

(defun neomacs-arduino-mode-test-spinner-active-p ()
  (and
   (boundp 'spinner-current)
   spinner-current
   (spinner--active-p spinner-current)))
"##;

fn arduino_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    let oracle = CachedMelpaOracle::new(ARDUINO_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned arduino-mode source below ./tmp")
        .with_timeout(ARDUINO_MODE_TEST_TIMEOUT);
    let oracle = if source_file == "flycheck-arduino.el" {
        oracle
            .with_melpa_dependency(ARDUINO_MODE_FLYCHECK_MELPA_PIN)
            .expect("prepare pinned Flycheck dependency below ./tmp")
    } else {
        oracle
    };
    oracle.with_prelude(ARDUINO_MODE_TEST_PRELUDE)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed arduino-mode parity test")
        .into()
}

fn assert_arduino_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arduino_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("arduino-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

fn assert_arduino_source_signal_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = arduino_mode_oracle(source_file)
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("arduino-mode signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_arduino_mode_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_parity("arduino-mode.el", elisp_form, expected);
}

pub(crate) fn assert_arduino_init_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_parity("arduino-mode-init.el", elisp_form, expected);
}

pub(crate) fn assert_ede_arduino_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_parity("ede-arduino.el", elisp_form, expected);
}

pub(crate) fn assert_ede_arduino_signal_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_signal_parity("ede-arduino.el", elisp_form, expected);
}

pub(crate) fn assert_flycheck_arduino_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_parity("flycheck-arduino.el", elisp_form, expected);
}

pub(crate) fn assert_ob_arduino_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_parity("ob-arduino.el", elisp_form, expected);
}

pub(crate) fn assert_ob_arduino_signal_parity(elisp_form: &str, expected: Expect) {
    assert_arduino_source_signal_parity("ob-arduino.el", elisp_form, expected);
}











/// Multi-probe batch for `assert_arduino_init_parity` cases (2a).
pub(crate) fn assert_arduino_init_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        arduino_mode_oracle("arduino-mode-init.el"),
        &name,
        "arduino_init_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_arduino_mode_parity` cases (2a).
pub(crate) fn assert_arduino_mode_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        arduino_mode_oracle("arduino-mode.el"),
        &name,
        "arduino_mode_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_ede_arduino_parity` cases (2a).
pub(crate) fn assert_ede_arduino_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        arduino_mode_oracle("ede-arduino.el"),
        &name,
        "ede_arduino_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_flycheck_arduino_parity` cases (2a).
pub(crate) fn assert_flycheck_arduino_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        arduino_mode_oracle("flycheck-arduino.el"),
        &name,
        "flycheck_arduino_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_ob_arduino_parity` cases (2a).
pub(crate) fn assert_ob_arduino_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        arduino_mode_oracle("ob-arduino.el"),
        &name,
        "ob_arduino_parity",
        cases,
    );
}
