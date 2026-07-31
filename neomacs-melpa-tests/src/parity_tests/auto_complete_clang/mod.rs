use std::time::Duration;

use crate::{
    AUTO_COMPLETE_CLANG_MELPA_PIN, AUTO_COMPLETE_MELPA_PIN, CachedMelpaOracle, POPUP_MELPA_PIN,
};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod actions;
mod arguments;
mod candidates;
mod parsing;
mod registry;
mod templates;
mod workflows;

const AUTO_COMPLETE_CLANG_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTO_COMPLETE_CLANG_TEST_PRELUDE: &str = r##"
(require 'cl-lib)

(defun ac-clang-test-error (thunk)
  (condition-case error-data
      (list :value (funcall thunk))
    (error
     (list :signal
           (car error-data)
           (cdr error-data)))))

(defun ac-clang-test-candidate-state (candidate)
  (list
   (substring-no-properties candidate)
   (get-text-property
    0 'ac-clang-help candidate)
   (get-text-property
    0 'raw-args candidate)))

(defun ac-clang-test-reset-file (file content)
  (make-directory
   (file-name-directory file)
   t)
  (with-temp-file file
    (insert content))
  file)
"##;

fn auto_complete_clang_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_COMPLETE_CLANG_MELPA_PIN, source_file)
        .expect("prepare pinned auto-complete-clang source below ./tmp")
        .with_melpa_dependency(AUTO_COMPLETE_MELPA_PIN)
        .expect("prepare pinned auto-complete dependency below ./tmp")
        .with_melpa_dependency(POPUP_MELPA_PIN)
        .expect("prepare pinned popup dependency below ./tmp")
        .with_prelude(AUTO_COMPLETE_CLANG_TEST_PRELUDE)
        .with_timeout(AUTO_COMPLETE_CLANG_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-complete-clang parity test")
        .into()
}

fn assert_auto_complete_clang_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auto_complete_clang_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("auto-complete-clang parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_complete_clang_parity(elisp_form: &str, expected: Expect) {
    assert_auto_complete_clang_source_parity("auto-complete-clang.el", elisp_form, expected);
}

pub(crate) fn assert_auto_complete_clang_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_complete_clang_source_parity(
        "auto-complete-clang-autoloads.el",
        elisp_form,
        expected,
    );
}



/// Multi-probe batch for `assert_auto_complete_clang_autoload_parity` cases (2a).
pub(crate) fn assert_auto_complete_clang_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auto_complete_clang_oracle("auto-complete-clang-autoloads.el"),
        &name,
        "auto_complete_clang_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_complete_clang_parity` cases (2a).
pub(crate) fn assert_auto_complete_clang_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auto_complete_clang_oracle("auto-complete-clang.el"),
        &name,
        "auto_complete_clang_parity",
        cases,
    );
}
