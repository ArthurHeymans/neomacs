use std::time::Duration;

use crate::{AUTO_DIM_OTHER_BUFFERS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod focus;
mod lifecycle;
mod registry;
mod remapping;
mod windows;
mod workflows;

const AUTO_DIM_OTHER_BUFFERS_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const AUTO_DIM_OTHER_BUFFERS_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'face-remap)

(defvar adob-test-never-dim-names nil)
(defvar adob-test-focus-state t)
(defvar adob-test-events nil)

(defun adob-test-error-data (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun adob-test-never-dim-by-name (buffer)
  (member
   (buffer-name buffer)
   adob-test-never-dim-names))

(defun adob-test-hook-count (function hook)
  (length
   (seq-filter
    (lambda (candidate)
      (eq candidate function))
    (symbol-value hook))))

(defun adob-test-focus-advice-installed-p ()
  (not
   (eq
    after-focus-change-function
    #'ignore)))

(defun adob-test-remap-summary (buffer)
  (with-current-buffer buffer
    (list
     (local-variable-p
      'adob--face-mode-remapping)
     (length adob--face-mode-remapping)
     (mapcar #'car
             adob--face-mode-remapping)
     (mapcar
      (lambda (entry)
        (list
         (car entry)
         (seq-filter
          (lambda (spec)
            (eq
             (car-safe spec)
             :filtered))
          (cdr entry))))
      face-remapping-alist))))

(defun adob-test-window-summary ()
  (let ((selected
         (selected-window)))
    (mapcar
     (lambda (window)
       (list
        (eq window selected)
        (buffer-name
         (window-buffer window))
        (window-parameter
         window
         'adob--dim)))
     (window-list nil 'n))))
"##;

fn auto_dim_other_buffers_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_DIM_OTHER_BUFFERS_MELPA_PIN, source_file)
        .expect("prepare pinned auto-dim-other-buffers source below ./tmp")
        .with_prelude(AUTO_DIM_OTHER_BUFFERS_TEST_PRELUDE)
        .with_timeout(AUTO_DIM_OTHER_BUFFERS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-dim-other-buffers parity test")
        .into()
}

fn assert_auto_dim_other_buffers_source_parity(
    source_file: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = auto_dim_other_buffers_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("auto-dim-other-buffers parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_dim_other_buffers_parity(elisp_form: &str, expected: Expect) {
    assert_auto_dim_other_buffers_source_parity("auto-dim-other-buffers.el", elisp_form, expected);
}

pub(crate) fn assert_auto_dim_other_buffers_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_dim_other_buffers_source_parity(
        "auto-dim-other-buffers-autoloads.el",
        elisp_form,
        expected,
    );
}





/// Multi-probe batch for `assert_auto_dim_other_buffers_autoload_parity` cases (2a).
pub(crate) fn assert_auto_dim_other_buffers_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auto_dim_other_buffers_oracle("auto-dim-other-buffers-autoloads.el"),
        &name,
        "auto_dim_other_buffers_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_dim_other_buffers_parity` cases (2a).
pub(crate) fn assert_auto_dim_other_buffers_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auto_dim_other_buffers_oracle("auto-dim-other-buffers.el"),
        &name,
        "auto_dim_other_buffers_parity",
        cases,
    );
}
