use std::time::Duration;

use crate::{AUTH_SOURCE_GOPASS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod backend;
mod paths;
mod registry;
mod search;
mod workflows;

const AUTH_SOURCE_GOPASS_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTH_SOURCE_GOPASS_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)

(defun auth-source-gopass-test-error-data
    (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))
"##;

fn auth_source_gopass_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTH_SOURCE_GOPASS_MELPA_PIN, source_file)
        .expect("prepare pinned auth-source-gopass source below ./tmp")
        .with_prelude(AUTH_SOURCE_GOPASS_TEST_PRELUDE)
        .with_timeout(AUTH_SOURCE_GOPASS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auth-source-gopass parity test")
        .into()
}

fn assert_auth_source_gopass_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = auth_source_gopass_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("auth-source-gopass parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auth_source_gopass_parity(elisp_form: &str, expected: Expect) {
    assert_auth_source_gopass_source_parity("auth-source-gopass.el", elisp_form, expected);
}

pub(crate) fn assert_auth_source_gopass_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auth_source_gopass_source_parity(
        "auth-source-gopass-autoloads.el",
        elisp_form,
        expected,
    );
}



/// Multi-probe batch for `assert_auth_source_gopass_autoload_parity` cases (2a).
pub(crate) fn assert_auth_source_gopass_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auth_source_gopass_oracle("auth-source-gopass-autoloads.el"),
        &name,
        "auth_source_gopass_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auth_source_gopass_parity` cases (2a).
pub(crate) fn assert_auth_source_gopass_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auth_source_gopass_oracle("auth-source-gopass.el"),
        &name,
        "auth_source_gopass_parity",
        cases,
    );
}
