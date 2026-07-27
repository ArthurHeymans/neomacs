use std::time::Duration;

use crate::{ALMOST_MONO_THEMES_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod lifecycle;
mod registry;
mod rendering;
mod themes;

const ALMOST_MONO_THEMES_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn almost_mono_themes_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ALMOST_MONO_THEMES_MELPA_PIN, "almost-mono-themes.el")
        .expect("prepare pinned almost-mono-themes source below ./tmp")
        .with_timeout(ALMOST_MONO_THEMES_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed almost-mono-themes parity test")
        .into()
}

pub(crate) fn assert_almost_mono_themes_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = almost_mono_themes_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("almost-mono-themes parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_almost_mono_themes_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = almost_mono_themes_oracle()
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("almost-mono-themes signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}
