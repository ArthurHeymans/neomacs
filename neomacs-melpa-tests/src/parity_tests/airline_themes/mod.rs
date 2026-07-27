use std::time::Duration;

use crate::{AIRLINE_THEMES_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod filesystem;
mod lifecycle;
mod modeline;
mod palettes;
mod registry;

const AIRLINE_THEMES_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn airline_themes_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AIRLINE_THEMES_MELPA_PIN, source_file)
        .expect("prepare pinned airline-themes source below ./tmp")
        .with_timeout(AIRLINE_THEMES_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed airline-themes parity test")
        .into()
}

fn assert_airline_themes_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = airline_themes_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("airline-themes parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_airline_themes_parity(elisp_form: &str, expected: Expect) {
    assert_airline_themes_source_parity("airline-themes.el", elisp_form, expected);
}

pub(crate) fn assert_airline_themes_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_airline_themes_source_parity("airline-themes-autoloads.el", elisp_form, expected);
}
