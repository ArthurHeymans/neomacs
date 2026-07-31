use std::time::Duration;

use crate::{AUTH_SOURCE_XOAUTH2_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod backend;
mod credentials;
mod enable;
mod password_store;
mod registry;
mod transport;
mod workflows;

const AUTH_SOURCE_XOAUTH2_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTH_SOURCE_XOAUTH2_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun auth-source-xoauth2-test-error-data
    (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun auth-source-xoauth2-test-file
    (name)
  (expand-file-name
   name
   (getenv "NEOMACS_TEST_SANDBOX_ROOT")))
"##;

fn auth_source_xoauth2_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTH_SOURCE_XOAUTH2_MELPA_PIN, source_file)
        .expect("prepare pinned auth-source-xoauth2 source below ./tmp")
        .with_prelude(AUTH_SOURCE_XOAUTH2_TEST_PRELUDE)
        .with_timeout(AUTH_SOURCE_XOAUTH2_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auth-source-xoauth2 parity test")
        .into()
}

fn assert_auth_source_xoauth2_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auth_source_xoauth2_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("auth-source-xoauth2 parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auth_source_xoauth2_parity(elisp_form: &str, expected: Expect) {
    assert_auth_source_xoauth2_source_parity("auth-source-xoauth2.el", elisp_form, expected);
}

pub(crate) fn assert_auth_source_xoauth2_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auth_source_xoauth2_source_parity(
        "auth-source-xoauth2-autoloads.el",
        elisp_form,
        expected,
    );
}



/// Multi-probe batch for `assert_auth_source_xoauth2_autoload_parity` cases (2a).
pub(crate) fn assert_auth_source_xoauth2_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auth_source_xoauth2_oracle("auth-source-xoauth2-autoloads.el"),
        &name,
        "auth_source_xoauth2_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auth_source_xoauth2_parity` cases (2a).
pub(crate) fn assert_auth_source_xoauth2_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        auth_source_xoauth2_oracle("auth-source-xoauth2.el"),
        &name,
        "auth_source_xoauth2_parity",
        cases,
    );
}
