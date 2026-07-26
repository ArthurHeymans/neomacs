use std::time::Duration;

use crate::{ABL_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod configuration;
mod entities;
mod surface;

const ABL_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn abl_mode_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ABL_MODE_MELPA_PIN, "abl-mode.el")
        .expect("prepare pinned abl-mode source below ./tmp")
        .with_prelude(
            r##"(progn
                   (require 'cl-lib)
                   (require 'subr-x)
                   ;; The pinned package omits its runtime f/s
                   ;; dependencies from MELPA metadata. Upstream's test init
                   ;; installs them manually; these narrow equivalents isolate
                   ;; abl-mode's own behavior while f and s have independent
                   ;; comprehensive parity corpora.
                   (defun s-uppercase? (value)
                     (equal value (upcase value)))
                   (defun s-join (separator strings)
                     (mapconcat #'identity strings separator))
                   (defun s-trim-right (value)
                     (string-trim-right value))
                   (defun s-chop-prefix (prefix value)
                     (if (string-prefix-p prefix value)
                         (substring value (length prefix))
                       value))
                   (defun f-join (first &rest rest)
                     (let ((value first))
                       (dolist (part rest value)
                         (setq value
                               (concat
                                (file-name-as-directory value)
                                part)))))
                   (defun f-split (path)
                     (split-string
                      (directory-file-name path)
                      "/" t))
                   (defun f-no-ext (path)
                     (file-name-sans-extension path))
                   (provide 'f)
                   (provide 's))"##,
        )
        .with_timeout(ABL_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed abl-mode parity test")
        .into()
}

pub(crate) fn assert_abl_mode_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = abl_mode_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("abl-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_abl_mode_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = abl_mode_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("abl-mode signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
