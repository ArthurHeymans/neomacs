use std::time::Duration;

use crate::{APACHE_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod font_lock;
mod indentation;
mod mode;
mod navigation;
mod registry;

const APACHE_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const APACHE_MODE_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defvar apache-mode-test-events nil)

(defun apache-mode-test-face-at
    (needle &optional occurrence)
  (save-excursion
    (let ((case-fold-search nil))
      (goto-char (point-min))
      (dotimes (_ (or occurrence 1))
        (search-forward needle))
      (get-text-property
       (- (point) (length needle))
       'face))))

(defun apache-mode-test-point-at
    (needle &optional offset)
  (let ((case-fold-search nil))
    (goto-char (point-min))
    (search-forward needle)
    (+ (- (point) (length needle))
       (or offset 0))))

(defun apache-mode-test-line-indents ()
  (save-excursion
    (goto-char (point-min))
    (let (result)
      (while (not (eobp))
        (setq result
              (append
               result
               (list
                (list
                 (buffer-substring-no-properties
                  (line-beginning-position)
                  (line-end-position))
                 (current-indentation)))))
        (forward-line 1))
      result)))
"##;

fn apache_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(APACHE_MODE_MELPA_PIN, source_file)
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

fn assert_apache_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apache_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("apache-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apache_mode_parity(elisp_form: &str, expected: Expect) {
    assert_apache_mode_source_parity("apache-mode.el", elisp_form, expected);
}

pub(crate) fn assert_apache_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_apache_mode_source_parity("apache-mode-autoloads.el", elisp_form, expected);
}
