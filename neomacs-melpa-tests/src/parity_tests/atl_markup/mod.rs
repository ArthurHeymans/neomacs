use std::time::Duration;

use crate::{ATL_MARKUP_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod lifecycle;
mod predicates;
mod registry;
mod timers;
mod truncation;
mod utilities;

const ATL_MARKUP_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ATL_MARKUP_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun atl-markup-test-error-data (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun atl-markup-test-place-marker (contents)
  (insert contents)
  (goto-char (point-min))
  (unless (search-forward "|" nil t)
    (error "Fixture has no point marker: %S" contents))
  (delete-char -1)
  (point))

(defun atl-markup-test-at-marker
    (contents mode function)
  (with-temp-buffer
    (atl-markup-test-place-marker contents)
    (when mode
      (funcall mode))
    (set-buffer-modified-p nil)
    (let ((before-point
           (point))
          (before-text
           (buffer-string)))
      (list
       (funcall function)
       (=
        before-point
        (point))
       (equal
        before-text
        (buffer-string))
       (buffer-modified-p)))))

(defun atl-markup-test-root ()
  (let ((root
         (file-name-as-directory
          (expand-file-name
           "atl-markup-case"
           (getenv "TMPDIR")))))
    (when (file-exists-p root)
      (delete-directory root t))
    (make-directory root t)
    root))

(defun atl-markup-test-read-file (file)
  (with-temp-buffer
    (insert-file-contents-literally file)
    (buffer-string)))
"##;

fn atl_markup_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ATL_MARKUP_MELPA_PIN, source_file)
        .expect("prepare pinned atl-markup source below ./tmp")
        .with_prelude(ATL_MARKUP_TEST_PRELUDE)
        .with_timeout(ATL_MARKUP_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed atl-markup parity test")
        .into()
}

fn assert_atl_markup_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = atl_markup_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("atl-markup parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_atl_markup_parity(elisp_form: &str, expected: Expect) {
    assert_atl_markup_source_parity("atl-markup.el", elisp_form, expected);
}

pub(crate) fn assert_atl_markup_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_atl_markup_source_parity("atl-markup-autoloads.el", elisp_form, expected);
}





/// Multi-probe batch for `assert_atl_markup_autoload_parity` cases (2a).
pub(crate) fn assert_atl_markup_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        atl_markup_oracle("atl-markup-autoloads.el"),
        &name,
        "atl_markup_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_atl_markup_parity` cases (2a).
pub(crate) fn assert_atl_markup_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        atl_markup_oracle("atl-markup.el"),
        &name,
        "atl_markup_parity",
        cases,
    );
}
