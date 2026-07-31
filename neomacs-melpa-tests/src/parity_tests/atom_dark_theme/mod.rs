use std::time::Duration;

use crate::{ATOM_DARK_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod faces;
mod lifecycle;
mod registry;
mod remapping;

const ATOM_DARK_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const ATOM_DARK_THEME_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'face-remap)

(defun atom-dark-test-theme-spec
    (face)
  (catch 'found
    (dolist
        (setting
         (get 'atom-dark 'theme-settings))
      (when
          (and
           (eq
            (car setting)
            'theme-face)
           (eq
            (cadr setting)
            face))
        (throw
         'found
         (copy-tree
          (nth 3 setting)))))))

(defun atom-dark-test-theme-specs
    (faces)
  (mapcar
   (lambda (face)
     (list
      face
      (atom-dark-test-theme-spec face)))
   faces))

(defun atom-dark-test-error
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

(defun atom-dark-test-face-attributes
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

fn atom_dark_theme_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ATOM_DARK_THEME_MELPA_PIN, source_file)
        .expect("prepare revision-pinned atom-dark-theme source below ./tmp")
        .with_prelude(ATOM_DARK_THEME_TEST_PRELUDE)
        .with_timeout(ATOM_DARK_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed atom-dark-theme parity test")
        .into()
}

fn assert_atom_dark_theme_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = atom_dark_theme_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("atom-dark-theme parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_atom_dark_theme_parity(elisp_form: &str, expected: Expect) {
    assert_atom_dark_theme_source_parity("atom-dark-theme.el", elisp_form, expected);
}

pub(crate) fn assert_atom_dark_theme_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_atom_dark_theme_source_parity("atom-dark-theme-autoloads.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_atom_dark_theme_autoload_parity` cases (2a).
pub(crate) fn assert_atom_dark_theme_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        atom_dark_theme_oracle("atom-dark-theme-autoloads.el"),
        &name,
        "atom_dark_theme_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_atom_dark_theme_parity` cases (2a).
pub(crate) fn assert_atom_dark_theme_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        atom_dark_theme_oracle("atom-dark-theme.el"),
        &name,
        "atom_dark_theme_parity",
        cases,
    );
}
