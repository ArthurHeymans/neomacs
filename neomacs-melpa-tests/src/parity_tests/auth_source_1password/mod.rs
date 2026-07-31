use std::time::Duration;

use crate::{AUTH_SOURCE_1PASSWORD_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod backend;
mod reference;
mod registry;
mod search;

const AUTH_SOURCE_1PASSWORD_TEST_TIMEOUT: Duration = Duration::from_secs(120);
const AUTH_SOURCE_1PASSWORD_TEST_PRELUDE: &str = r##"
(require 'cl-lib)
(require 'seq)
(require 'subr-x)

(defun auth-source-1password-test-error-data (thunk)
  (condition-case error-data
      (list :ok
            (funcall thunk))
    (error
     (list :error
           (car error-data)
           (cdr error-data)))))

(defun auth-source-1password-test-backend-shape (backend)
  (list
   (eieio-object-p backend)
   (eieio-object-class-name backend)
   (slot-value backend 'type)
   (slot-value backend 'source)
   (slot-value backend 'host)
   (slot-value backend 'user)
   (slot-value backend 'port)
   (slot-value backend 'data)
   (slot-value backend 'create-function)
   (slot-value backend 'search-function)))

(defun auth-source-1password-test-read-file (path)
  (with-temp-buffer
    (insert-file-contents-literally path)
    (buffer-string)))
"##;

fn auth_source_1password_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AUTH_SOURCE_1PASSWORD_MELPA_PIN, source_file)
        .expect("prepare pinned auth-source-1password source below ./tmp")
        .with_prelude(AUTH_SOURCE_1PASSWORD_TEST_PRELUDE)
        .with_timeout(AUTH_SOURCE_1PASSWORD_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed auth-source-1password parity test")
        .into()
}

fn assert_auth_source_1password_source_parity(
    source_file: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = auth_source_1password_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("auth-source-1password parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_auth_source_1password_parity(elisp_form: &str, expected: Expect) {
    assert_auth_source_1password_source_parity("auth-source-1password.el", elisp_form, expected);
}

pub(crate) fn assert_auth_source_1password_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_auth_source_1password_source_parity(
        "auth-source-1password-autoloads.el",
        elisp_form,
        expected,
    );
}





/// Multi-probe batch for `assert_auth_source_1password_autoload_parity` cases (2a).
pub(crate) fn assert_auth_source_1password_autoload_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auth_source_1password_oracle("auth-source-1password-autoloads.el"),
        &name,
        "auth_source_1password_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_auth_source_1password_parity` cases (2a).
pub(crate) fn assert_auth_source_1password_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        auth_source_1password_oracle("auth-source-1password.el"),
        &name,
        "auth_source_1password_parity",
        cases,
    );
}
