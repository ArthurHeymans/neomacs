use std::time::Duration;

use crate::{APPLESCRIPT_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const APPLESCRIPT_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const APPLESCRIPT_MODE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defun applescript-test-face-at
    (token)
  (goto-char (point-min))
  (search-forward token)
  (or
   (get-text-property
     (match-beginning 0)
     'face)
   (get-text-property
    (match-beginning 0)
    'font-lock-face)))

(defun applescript-test-kill-buffers
    (regexp)
  (dolist (buffer (buffer-list))
    (when (string-match-p
           regexp
           (buffer-name buffer))
      (set-buffer-modified-p nil)
      (kill-buffer buffer))))
"##;

fn applescript_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(APPLESCRIPT_MODE_MELPA_PIN, "applescript-mode.el")
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

pub(crate) fn assert_applescript_mode_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = applescript_mode_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("applescript-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_applescript_mode_parity` cases (2a).
pub(crate) fn assert_applescript_mode_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        applescript_mode_oracle(),
        &name,
        "applescript_mode_parity",
        cases,
    );
}
