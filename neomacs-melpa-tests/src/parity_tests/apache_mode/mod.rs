use std::time::Duration;

use crate::{APACHE_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod workflows;

const APACHE_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const APACHE_MODE_TEST_PRELUDE: &str = r##"
(defun neomacs-apache-test-face-at (needle)
  (save-excursion
    (let ((case-fold-search nil))
      (goto-char (point-min))
      (search-forward needle)
      (get-text-property (match-beginning 0) 'face))))

(defun neomacs-apache-test-lines ()
  (save-excursion
    (goto-char (point-min))
    (let (lines)
      (while
          (< (point) (point-max))
        (push
         (list
          (line-number-at-pos)
          (current-indentation)
          (buffer-substring-no-properties
           (line-beginning-position)
           (line-end-position)))
         lines)
        (forward-line 1))
      (nreverse lines))))

(defun neomacs-apache-test-file-string (file)
  (with-temp-buffer
    (insert-file-contents-literally file)
    (buffer-string)))

(defun neomacs-apache-test-cleanup (root)
  (dolist (buffer (buffer-list))
    (let ((file (buffer-file-name buffer)))
      (when
          (and file (string-prefix-p root file))
        (with-current-buffer buffer
          (set-buffer-modified-p nil))
        (kill-buffer buffer))))
  (when
      (file-exists-p root)
    (delete-directory root t)))
"##;

fn apache_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(APACHE_MODE_MELPA_PIN, "apache-mode.el")
        .expect("prepare pinned apache-mode source below ./tmp")
        .with_prelude(APACHE_MODE_TEST_PRELUDE)
        .with_timeout(APACHE_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed apache-mode parity test")
        .into()
}

pub(crate) fn assert_apache_mode_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apache_mode_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("apache-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_apache_mode_parity` cases (2a).
pub(crate) fn assert_apache_mode_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(apache_mode_oracle(), &name, "apache_mode_parity", cases);
}
