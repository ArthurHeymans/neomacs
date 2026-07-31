use std::time::Duration;

use crate::{ATOM_ONE_DARK_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod faces;
mod lifecycle;
mod palette;
mod practical;
mod registry;
mod remapping;
mod workflows;

const ATOM_ONE_DARK_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ATOM_ONE_DARK_THEME_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'face-remap)

(defun atom-one-dark-test-settings
    (kind)
  (nreverse
   (seq-filter
    (lambda (setting)
      (eq
       (car setting)
       kind))
    (copy-tree
     (get 'atom-one-dark 'theme-settings)))))

(defun atom-one-dark-test-face-settings ()
  (atom-one-dark-test-settings 'theme-face))

(defun atom-one-dark-test-value-settings ()
  (atom-one-dark-test-settings 'theme-value))

(defun atom-one-dark-test-face-chunk
    (start end)
  (mapcar
   (lambda (setting)
     (list
      (cadr setting)
      (nth 3 setting)))
   (seq-subseq
    (atom-one-dark-test-face-settings)
    start
    end)))

(defun atom-one-dark-test-face-specs
    (face)
  (mapcar
   (lambda (setting)
     (nth 3 setting))
   (seq-filter
    (lambda (setting)
      (eq
       (cadr setting)
       face))
    (atom-one-dark-test-face-settings))))

(defun atom-one-dark-test-error
    (thunk)
  (condition-case error
      (list
       :ok
       (funcall thunk))
    (error
     (list
      :signal
      (car error)
      (cdr error)))))

(defun atom-one-dark-test-face-attributes
    (face attributes)
  (mapcar
   (lambda (attribute)
     (list
      attribute
      (face-attribute
       face
       attribute
       nil
       nil)
      (face-attribute
       face
       attribute
       nil
       t)))
   attributes))
"##;

fn atom_one_dark_theme_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ATOM_ONE_DARK_THEME_MELPA_PIN, source_file)
        .expect("prepare pinned atom-one-dark-theme source below ./tmp")
        .with_prelude(ATOM_ONE_DARK_THEME_TEST_PRELUDE)
        .with_timeout(ATOM_ONE_DARK_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed atom-one-dark-theme parity test")
        .into()
}

fn assert_atom_one_dark_theme_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = atom_one_dark_theme_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("atom-one-dark-theme parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_atom_one_dark_theme_parity(elisp_form: &str, expected: Expect) {
    assert_atom_one_dark_theme_source_parity("atom-one-dark-theme.el", elisp_form, expected);
}

pub(crate) fn assert_atom_one_dark_theme_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_atom_one_dark_theme_source_parity(
        "atom-one-dark-theme-autoloads.el",
        elisp_form,
        expected,
    );
}



/// Multi-probe batch for `assert_atom_one_dark_theme_autoload_parity` cases (2a).
pub(crate) fn assert_atom_one_dark_theme_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        atom_one_dark_theme_oracle("atom-one-dark-theme-autoloads.el"),
        &name,
        "atom_one_dark_theme_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_atom_one_dark_theme_parity` cases (2a).
pub(crate) fn assert_atom_one_dark_theme_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        atom_one_dark_theme_oracle("atom-one-dark-theme.el"),
        &name,
        "atom_one_dark_theme_parity",
        cases,
    );
}
