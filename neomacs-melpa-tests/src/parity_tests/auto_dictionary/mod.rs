use std::time::Duration;

use crate::{AUTO_DICTIONARY_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod conditional;
mod detection;
mod mode;
mod registry;
mod workflows;

const AUTO_DICTIONARY_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTO_DICTIONARY_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'flyspell)
(require 'ispell)

(defvar adict-test-valid-dictionaries
  '("en" "de" "fr" "es" "sv" "sl" "hu" "ro" "pt"
    "nb" "da" "grc" "el" "hi" "nn" "ca" "eo" "sk"))

(defun adict-test-valid-dictionary-list ()
  adict-test-valid-dictionaries)

(advice-add
 'ispell-valid-dictionary-list
 :override
 #'adict-test-valid-dictionary-list)

(defun adict-test-error (thunk)
  (condition-case error-data
      (list :ok (funcall thunk))
    (error
     (list :signal
           (car error-data)
           (cdr error-data)))))

(defun adict-test-overlay-state (overlay)
  (list
   (overlay-start overlay)
   (overlay-end overlay)
   (overlay-get overlay 'evaporate)
   (overlay-get overlay 'face)
   (overlay-get overlay
                'adict-conditional-list)
   (overlay-get overlay
                'modification-hooks)))
"##;

fn auto_dictionary_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTO_DICTIONARY_MELPA_PIN, source_file)
        .expect("prepare pinned auto-dictionary source below ./tmp")
        .with_prelude(AUTO_DICTIONARY_TEST_PRELUDE)
        .with_timeout(AUTO_DICTIONARY_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auto-dictionary parity test")
        .into()
}

fn assert_auto_dictionary_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auto_dictionary_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("auto-dictionary parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auto_dictionary_parity(elisp_form: &str, expected: Expect) {
    assert_auto_dictionary_source_parity("auto-dictionary.el", elisp_form, expected);
}

pub(crate) fn assert_auto_dictionary_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auto_dictionary_source_parity("auto-dictionary-autoloads.el", elisp_form, expected);
}



/// Multi-probe batch for `assert_auto_dictionary_autoload_parity` cases (2a).
pub(crate) fn assert_auto_dictionary_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_dictionary_oracle("auto-dictionary-autoloads.el"),
        &name,
        "auto_dictionary_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auto_dictionary_parity` cases (2a).
pub(crate) fn assert_auto_dictionary_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auto_dictionary_oracle("auto-dictionary.el"),
        &name,
        "auto_dictionary_parity",
        cases,
    );
}
