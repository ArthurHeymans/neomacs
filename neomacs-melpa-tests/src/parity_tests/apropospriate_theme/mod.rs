use std::time::Duration;

use crate::{APROPOSPRIATE_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod contrast;
mod dark;
mod light;
mod registry;

const APROPOSPRIATE_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn apropospriate_theme_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(APROPOSPRIATE_THEME_MELPA_PIN, source_file)
        .expect("prepare pinned apropospriate-theme source below ./tmp")
        .with_timeout(APROPOSPRIATE_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed apropospriate-theme parity test")
        .into()
}

fn assert_apropospriate_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = apropospriate_theme_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("apropospriate-theme parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_apropospriate_theme_parity(elisp_form: &str, expected: Expect) {
    assert_apropospriate_source_parity("apropospriate-theme.el", elisp_form, expected);
}

pub(crate) fn assert_apropospriate_dark_parity(elisp_form: &str, expected: Expect) {
    assert_apropospriate_source_parity("apropospriate-dark-theme.el", elisp_form, expected);
}

pub(crate) fn assert_apropospriate_light_parity(elisp_form: &str, expected: Expect) {
    assert_apropospriate_source_parity("apropospriate-light-theme.el", elisp_form, expected);
}
