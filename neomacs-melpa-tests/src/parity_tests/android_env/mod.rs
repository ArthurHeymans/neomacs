use std::time::Duration;

use crate::{ANDROID_ENV_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod android;
mod gradle;
mod logcat;
mod refactor;
mod registry;

const ANDROID_ENV_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn android_env_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANDROID_ENV_MELPA_PIN, source_file)
        .expect("prepare pinned android-env source below ./tmp")
        .with_timeout(ANDROID_ENV_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed android-env parity test")
        .into()
}

fn assert_android_env_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = android_env_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("android-env parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_android_env_parity(elisp_form: &str, expected: Expect) {
    assert_android_env_source_parity("android-env.el", elisp_form, expected);
}

pub(crate) fn assert_android_env_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_android_env_source_parity("android-env-autoloads.el", elisp_form, expected);
}

pub(crate) fn assert_android_env_hydra_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let prelude = r##"(progn
  (defvar android-env-test-hydra-definition nil)
  (defmacro defhydra (name properties &rest body)
    `(setq android-env-test-hydra-definition
           ',(list name properties body)))
  (provide 'hydra))"##;
    let report = android_env_oracle("android-env.el")
        .with_prelude(prelude)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("android-env hydra parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
