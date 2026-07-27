use std::time::Duration;

use crate::{AMPLE_ZEN_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod lifecycle;
mod registry;
mod rendering;
mod themes;

const AMPLE_ZEN_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn ample_zen_theme_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AMPLE_ZEN_THEME_MELPA_PIN, source_file)
        .expect("prepare pinned ample-zen-theme source below ./tmp")
        .with_timeout(AMPLE_ZEN_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ample-zen-theme parity test")
        .into()
}

fn assert_ample_zen_theme_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ample_zen_theme_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ample-zen-theme parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ample_zen_theme_parity(elisp_form: &str, expected: Expect) {
    assert_ample_zen_theme_source_parity("ample-zen-theme.el", elisp_form, expected);
}

pub(crate) fn assert_ample_zen_theme_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ample_zen_theme_source_parity("ample-zen-theme-autoloads.el", elisp_form, expected);
}
