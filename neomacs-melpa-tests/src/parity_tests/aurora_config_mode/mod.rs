use std::time::Duration;

use crate::{AURORA_CONFIG_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod commands;
mod jobpath;
mod keywords;
mod mode;
mod registry;
mod workflows;

const AURORA_CONFIG_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AURORA_CONFIG_MODE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun aurora-config-test-error-data (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun aurora-config-test-face-runs ()
  (let ((position
         (point-min))
        rows)
    (while
        (< position
           (point-max))
      (let* ((face
              (get-text-property
               position
               'face))
             (next
              (next-single-property-change
               position
               'face
               nil
               (point-max))))
        (when face
          (push
           (list
            (-
             position
             (point-min))
            (-
             next
             (point-min))
            (buffer-substring-no-properties
             position
             next)
            face)
           rows))
        (setq position next)))
    (nreverse rows)))

(defun aurora-config-test-buffer-state ()
  (list
   major-mode
   mode-name
   (derived-mode-p 'python-mode)
   (buffer-string)
   (buffer-modified-p)
   (and
    (local-variable-p
     'aurora-config-last-job-path)
    aurora-config-last-job-path)
   (local-variable-p
    'font-lock-defaults)
   (length
    (car font-lock-defaults))
   (lookup-key
    (current-local-map)
    (kbd "C-c a i"))
   (lookup-key
    (current-local-map)
    (kbd "C-c a d"))))
"##;

fn aurora_config_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AURORA_CONFIG_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned aurora-config-mode source below ./tmp")
        .with_prelude(AURORA_CONFIG_MODE_TEST_PRELUDE)
        .with_timeout(AURORA_CONFIG_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed aurora-config-mode parity test")
        .into()
}

fn assert_aurora_config_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = aurora_config_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("aurora-config-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_aurora_config_mode_parity(elisp_form: &str, expected: Expect) {
    assert_aurora_config_mode_source_parity("aurora-config-mode.el", elisp_form, expected);
}

pub(crate) fn assert_aurora_config_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_aurora_config_mode_source_parity(
        "aurora-config-mode-autoloads.el",
        elisp_form,
        expected,
    );
}



/// Multi-probe batch for `assert_aurora_config_mode_autoload_parity` cases (2a).
pub(crate) fn assert_aurora_config_mode_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        aurora_config_mode_oracle("aurora-config-mode-autoloads.el"),
        &name,
        "aurora_config_mode_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_aurora_config_mode_parity` cases (2a).
pub(crate) fn assert_aurora_config_mode_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        aurora_config_mode_oracle("aurora-config-mode.el"),
        &name,
        "aurora_config_mode_parity",
        cases,
    );
}
