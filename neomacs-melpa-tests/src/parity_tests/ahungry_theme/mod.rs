use std::time::Duration;

use crate::{AHUNGRY_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod lifecycle;
mod rendering;

const AHUNGRY_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ahungry_theme_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AHUNGRY_THEME_MELPA_PIN, "ahungry-theme.el")
        .expect("prepare pinned ahungry-theme source below ./tmp")
        .with_timeout(AHUNGRY_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ahungry-theme parity test")
        .into()
}

pub(crate) fn assert_ahungry_theme_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ahungry_theme_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ahungry-theme parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
