use std::time::Duration;

use crate::{APPLESCRIPT_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod execution;
mod font_lock;
mod mode;
mod parsing;
mod registry;
mod utilities;

const APPLESCRIPT_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const APPLESCRIPT_MODE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defvar applescript-test-events nil)

(defun applescript-test-face-at
    (token &optional occurrence)
  (goto-char (point-min))
  (let ((count (or occurrence 1)))
    (dotimes (_ count)
      (search-forward token))
    (get-text-property
     (match-beginning 0)
     'face)))

(defun applescript-test-syntax-at
    (token)
  (goto-char (point-min))
  (search-forward token)
  (let ((state
         (syntax-ppss
          (match-beginning 0))))
    (list
     (nth 3 state)
     (nth 4 state)
     (nth 8 state))))

(defun applescript-test-kill-buffers
    (regexp)
  (dolist (buffer (buffer-list))
    (when (string-match-p
           regexp
           (buffer-name buffer))
      (set-buffer-modified-p nil)
      (kill-buffer buffer))))
"##;

fn applescript_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(APPLESCRIPT_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned applescript-mode source below ./tmp")
        .with_prelude(APPLESCRIPT_MODE_TEST_PRELUDE)
        .with_timeout(APPLESCRIPT_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed applescript-mode parity test")
        .into()
}

fn assert_applescript_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = applescript_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("applescript-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_applescript_mode_parity(elisp_form: &str, expected: Expect) {
    assert_applescript_mode_source_parity("applescript-mode.el", elisp_form, expected);
}

pub(crate) fn assert_applescript_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_applescript_mode_source_parity("applescript-mode-autoloads.el", elisp_form, expected);
}
