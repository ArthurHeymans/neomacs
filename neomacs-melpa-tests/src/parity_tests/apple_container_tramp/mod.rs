use std::time::Duration;

use crate::{APPLE_CONTAINER_TRAMP_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod cleanup;
mod discovery;
mod method;
mod registry;

const APPLE_CONTAINER_TRAMP_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn apple_container_tramp_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(APPLE_CONTAINER_TRAMP_MELPA_PIN, source_file)
        .expect("prepare pinned apple-container-tramp source below ./tmp")
        .with_timeout(APPLE_CONTAINER_TRAMP_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed apple-container-tramp parity test")
        .into()
}

fn assert_apple_container_tramp_source_parity(
    source_file: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = apple_container_tramp_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("apple-container-tramp parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apple_container_tramp_parity(elisp_form: &str, expected: Expect) {
    assert_apple_container_tramp_source_parity("apple-container-tramp.el", elisp_form, expected);
}

pub(crate) fn assert_apple_container_tramp_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apple_container_tramp_oracle("apple-container-tramp.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("apple-container-tramp signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apple_container_tramp_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_apple_container_tramp_source_parity(
        "apple-container-tramp-autoloads.el",
        elisp_form,
        expected,
    );
}
