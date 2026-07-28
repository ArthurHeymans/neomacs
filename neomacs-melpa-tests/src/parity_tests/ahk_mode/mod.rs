use std::time::Duration;

use crate::{AHK_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod editing;
mod language;
mod navigation;

const AHK_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AHK_MODE_TEST_PRELUDE: &str = r##"
(defun neomacs-ahk-test-write-file (file content)
  (make-directory (file-name-directory file) t)
  (with-temp-file file
    (insert content)))

(defun neomacs-ahk-test-file-string (file)
  (with-temp-buffer
    (insert-file-contents file)
    (buffer-string)))

(defun neomacs-ahk-test-cleanup (root)
  (dolist (buffer (buffer-list))
    (let ((file (buffer-file-name buffer)))
      (when
          (and file
               (string-prefix-p root file))
        (with-current-buffer buffer
          (set-buffer-modified-p nil))
        (kill-buffer buffer))))
  (when
      (file-exists-p root)
    (delete-directory root t)))
"##;

fn ahk_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AHK_MODE_MELPA_PIN, "ahk-mode.el")
        .expect("prepare pinned ahk-mode source below ./tmp")
        .with_prelude(AHK_MODE_TEST_PRELUDE)
        .with_timeout(AHK_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ahk-mode parity test")
        .into()
}

pub(crate) fn assert_ahk_mode_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ahk_mode_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ahk-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
