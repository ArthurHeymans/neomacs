use std::time::Duration;

use crate::{ABYSS_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod faces;
mod lifecycle;
mod registry;

const ABYSS_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn abyss_theme_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ABYSS_THEME_MELPA_PIN, "abyss-theme.el")
        .expect("prepare pinned abyss-theme source below ./tmp")
        .with_timeout(ABYSS_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed abyss-theme parity test")
        .into()
}

pub(crate) fn assert_abyss_theme_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = abyss_theme_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("abyss-theme parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
