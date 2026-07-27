use std::time::Duration;

use crate::{APP_MONOCHROME_THEMES_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod contrast;
mod dark;
mod light;
mod registry;

const APP_MONOCHROME_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn app_monochrome_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(APP_MONOCHROME_THEMES_MELPA_PIN, source_file)
        .expect("prepare pinned app-monochrome-themes source below ./tmp")
        .with_timeout(APP_MONOCHROME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed app-monochrome-themes parity test")
        .into()
}

fn assert_app_monochrome_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = app_monochrome_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("app-monochrome-themes parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_app_monochrome_parity(elisp_form: &str, expected: Expect) {
    assert_app_monochrome_source_parity("app-monochrome-themes.el", elisp_form, expected);
}

pub(crate) fn assert_app_monochrome_dark_parity(elisp_form: &str, expected: Expect) {
    assert_app_monochrome_source_parity(
        "app-monochrome-themes-dark-theme-theme.el",
        elisp_form,
        expected,
    );
}

pub(crate) fn assert_app_monochrome_light_parity(elisp_form: &str, expected: Expect) {
    assert_app_monochrome_source_parity(
        "app-monochrome-themes-light-theme-theme.el",
        elisp_form,
        expected,
    );
}
