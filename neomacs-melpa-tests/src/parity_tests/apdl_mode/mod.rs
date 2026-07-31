use std::time::Duration;

use crate::{APDL_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const APDL_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(240);
const APDL_MODE_TEST_PRELUDE: &str = r####"
(require 'cl-lib)

;; The workflows configure a deterministic solver when they need one.
;; Normal mode activation must not discover a host Ansys installation.
(setq apdl-initialised-flag t)

(defun neomacs-apdl-test-face-at (needle &optional occurrence)
  (save-excursion
    (let ((case-fold-search nil))
      (goto-char (point-min))
      (dotimes (_ (or occurrence 1))
        (search-forward needle))
      (get-text-property (match-beginning 0) 'face))))

(defun neomacs-apdl-test-lines ()
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

(defun neomacs-apdl-test-file-string (file)
  (with-temp-buffer
    (insert-file-contents-literally file)
    (buffer-string)))

(defun neomacs-apdl-test-cleanup (root)
  (when
      (and (boundp 'apdl-timer)
           (timerp apdl-timer))
    (cancel-timer apdl-timer)
    (setq apdl-timer nil))
  (dolist (name '("MAPDL-Batch" "MAPDL" "Classics"))
    (let ((process (get-process name)))
      (when process
        (delete-process process))))
  (dolist (buffer (buffer-list))
    (let ((file (buffer-file-name buffer))
          (name (buffer-name buffer)))
      (when
          (or
           (and file (string-prefix-p root file))
           (string-prefix-p "*APDL" name)
           (member
            name
            '("*MAPDL-Batch*"
              "*MAPDL*"
              "*Classics*"
              "*User-licenses*")))
        (with-current-buffer buffer
          (set-buffer-modified-p nil))
        (kill-buffer buffer))))
  (when
      (file-exists-p root)
    (delete-directory root t)))
"####;

fn apdl_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(APDL_MODE_MELPA_PIN, "apdl-mode.el")
        .expect("prepare pinned apdl-mode source below ./tmp")
        .with_prelude(APDL_MODE_TEST_PRELUDE)
        .with_timeout(APDL_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed apdl-mode parity test")
        .into()
}

pub(crate) fn assert_apdl_mode_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apdl_mode_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("apdl-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_apdl_mode_parity` cases (2a).
pub(crate) fn assert_apdl_mode_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        apdl_mode_oracle(),
        &name,
        "apdl_mode_parity",
        cases,
    );
}
