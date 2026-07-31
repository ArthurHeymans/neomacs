use std::time::Duration;

use crate::{
    AUTO_COMPLETE_CHUNK_MELPA_PIN, AUTO_COMPLETE_MELPA_PIN, CachedMelpaOracle, POPUP_MELPA_PIN,
};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod boundaries;
mod candidates;
mod registry;
mod sources;
mod workflows;

const AUTO_COMPLETE_CHUNK_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTO_COMPLETE_CHUNK_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defvar auto-complete-chunk-test-events nil)

(defun auto-complete-chunk-test-error (thunk)
  (condition-case error-data
      (list :value
            (funcall thunk))
    (error
     (list :signal
           (car error-data)
           (cdr error-data)))))

(defun auto-complete-chunk-test-beginning (mode text &optional position)
  (with-temp-buffer
    (funcall mode)
    (insert text)
    (goto-char
     (or position
         (point-max)))
    (let ((beginning
           (ac-chunk-beginning)))
      (list
       mode
       text
       (point)
       beginning
       (and beginning
            (buffer-substring-no-properties
             beginning
             (point)))))))

;; popup.el normally obtains these coordinates from the display engine.
;; Deterministic batch coordinates preserve the real completion lifecycle.
(defun auto-complete-chunk-test-posn-at-point (&rest _arguments)
  'auto-complete-chunk-test-position)

(defun auto-complete-chunk-test-posn-col-row (_position)
  (cons
   (current-column)
   (line-number-at-pos
    (point))))

(fset
 'posn-at-point
 #'auto-complete-chunk-test-posn-at-point)
(fset
 'posn-col-row
 #'auto-complete-chunk-test-posn-col-row)
"##;

fn auto_complete_chunk_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_COMPLETE_CHUNK_MELPA_PIN, source_file)
        .expect("prepare pinned auto-complete-chunk source below ./tmp")
        .with_melpa_dependency(AUTO_COMPLETE_MELPA_PIN)
        .expect("prepare pinned auto-complete dependency below ./tmp")
        .with_melpa_dependency(POPUP_MELPA_PIN)
        .expect("prepare pinned popup transitive dependency below ./tmp")
        .with_prelude(AUTO_COMPLETE_CHUNK_TEST_PRELUDE)
        .with_timeout(AUTO_COMPLETE_CHUNK_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-complete-chunk parity test")
        .into()
}

fn assert_auto_complete_chunk_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auto_complete_chunk_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("auto-complete-chunk parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_complete_chunk_parity(elisp_form: &str, expected: Expect) {
    assert_auto_complete_chunk_source_parity("auto-complete-chunk.el", elisp_form, expected);
}

pub(crate) fn assert_auto_complete_chunk_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_complete_chunk_source_parity(
        "auto-complete-chunk-autoloads.el",
        elisp_form,
        expected,
    );
}



/// Multi-probe batch for `assert_auto_complete_chunk_autoload_parity` cases (2a).
pub(crate) fn assert_auto_complete_chunk_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auto_complete_chunk_oracle("auto-complete-chunk-autoloads.el"),
        &name,
        "auto_complete_chunk_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_complete_chunk_parity` cases (2a).
pub(crate) fn assert_auto_complete_chunk_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auto_complete_chunk_oracle("auto-complete-chunk.el"),
        &name,
        "auto_complete_chunk_parity",
        cases,
    );
}
