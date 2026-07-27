use std::time::Duration;

use crate::{APIB_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod assets;
mod compilation;
mod mode;
mod refract;
mod registry;
mod upstream_workflows;

const APIB_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn apib_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(APIB_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned apib-mode source below ./tmp")
        .with_timeout(APIB_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed apib-mode parity test")
        .into()
}

fn assert_apib_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apib_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("apib-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apib_mode_parity(elisp_form: &str, expected: Expect) {
    assert_apib_mode_source_parity("apib-mode.el", elisp_form, expected);
}

pub(crate) fn assert_apib_mode_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apib_mode_oracle("apib-mode.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("apib-mode signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apib_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_apib_mode_source_parity("apib-mode-autoloads.el", elisp_form, expected);
}
