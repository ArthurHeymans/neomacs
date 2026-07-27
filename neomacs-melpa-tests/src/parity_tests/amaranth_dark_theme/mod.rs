use std::time::Duration;

use crate::{AMARANTH_DARK_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod faces;
mod lifecycle;
mod registry;
mod rendering;

const AMARANTH_DARK_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn amaranth_dark_theme_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AMARANTH_DARK_THEME_MELPA_PIN, source_file)
        .expect("prepare pinned amaranth-dark-theme source below ./tmp")
        .with_timeout(AMARANTH_DARK_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed amaranth-dark-theme parity test")
        .into()
}

fn assert_amaranth_dark_theme_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = amaranth_dark_theme_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("amaranth-dark-theme parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_amaranth_dark_theme_parity(elisp_form: &str, expected: Expect) {
    assert_amaranth_dark_theme_source_parity("amaranth-dark-theme.el", elisp_form, expected);
}

pub(crate) fn assert_amaranth_dark_theme_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_amaranth_dark_theme_source_parity(
        "amaranth-dark-theme-autoloads.el",
        elisp_form,
        expected,
    );
}
