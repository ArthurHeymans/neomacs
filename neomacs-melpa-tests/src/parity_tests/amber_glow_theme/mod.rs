use std::time::Duration;

use crate::{AMBER_GLOW_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod lifecycle;
mod registry;
mod rendering;
mod specs;

const AMBER_GLOW_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn amber_glow_theme_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AMBER_GLOW_THEME_MELPA_PIN, source_file)
        .expect("prepare pinned amber-glow-theme source below ./tmp")
        .with_timeout(AMBER_GLOW_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed amber-glow-theme parity test")
        .into()
}

fn assert_amber_glow_theme_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = amber_glow_theme_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("amber-glow-theme parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_amber_glow_theme_parity(elisp_form: &str, expected: Expect) {
    assert_amber_glow_theme_source_parity("amber-glow-theme.el", elisp_form, expected);
}

pub(crate) fn assert_amber_glow_theme_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_amber_glow_theme_source_parity("amber-glow-theme-autoloads.el", elisp_form, expected);
}
