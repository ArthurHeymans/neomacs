use std::time::Duration;

use crate::{AUTH_SOURCE_KEYTAR_MELPA_PIN, CachedMelpaOracle, KEYTAR_MELPA_PIN, S_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod backend;
mod enable;
mod parsing;
mod registry;
mod search;
mod workflows;

const AUTH_SOURCE_KEYTAR_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTH_SOURCE_KEYTAR_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun auth-source-keytar-test-error-data (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun auth-source-keytar-test-backend-data (backend)
  (when backend
    (list
     (slot-value backend 'source)
     (slot-value backend 'type)
     (slot-value backend 'search-function))))
"##;

fn auth_source_keytar_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTH_SOURCE_KEYTAR_MELPA_PIN, source_file)
        .expect("prepare pinned auth-source-keytar source below ./tmp")
        .with_melpa_dependency(KEYTAR_MELPA_PIN)
        .expect("prepare pinned Keytar dependency below ./tmp")
        .with_melpa_dependency(S_MELPA_PIN)
        .expect("prepare pinned s dependency below ./tmp")
        .with_prelude(AUTH_SOURCE_KEYTAR_TEST_PRELUDE)
        .with_timeout(AUTH_SOURCE_KEYTAR_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auth-source-keytar parity test")
        .into()
}

fn assert_auth_source_keytar_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auth_source_keytar_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("auth-source-keytar parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auth_source_keytar_parity(elisp_form: &str, expected: Expect) {
    assert_auth_source_keytar_source_parity("auth-source-keytar.el", elisp_form, expected);
}

pub(crate) fn assert_auth_source_keytar_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auth_source_keytar_source_parity(
        "auth-source-keytar-autoloads.el",
        elisp_form,
        expected,
    );
}



/// Multi-probe batch for `assert_auth_source_keytar_autoload_parity` cases (2a).
pub(crate) fn assert_auth_source_keytar_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auth_source_keytar_oracle("auth-source-keytar-autoloads.el"),
        &name,
        "auth_source_keytar_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auth_source_keytar_parity` cases (2a).
pub(crate) fn assert_auth_source_keytar_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auth_source_keytar_oracle("auth-source-keytar.el"),
        &name,
        "auth_source_keytar_parity",
        cases,
    );
}
