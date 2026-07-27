use std::time::Duration;

use crate::{ANGULAR_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod html;
mod javascript;
mod lifecycle;
mod registry;

const ANGULAR_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn angular_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANGULAR_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned angular-mode source below ./tmp")
        .with_timeout(ANGULAR_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed angular-mode parity test")
        .into()
}

fn assert_angular_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = angular_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("angular-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_angular_mode_parity(elisp_form: &str, expected: Expect) {
    assert_angular_mode_source_parity("angular-mode.el", elisp_form, expected);
}

pub(crate) fn assert_angular_html_mode_parity(elisp_form: &str, expected: Expect) {
    assert_angular_mode_source_parity("angular-html-mode.el", elisp_form, expected);
}

pub(crate) fn assert_angular_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_angular_mode_source_parity("angular-mode-autoloads.el", elisp_form, expected);
}
