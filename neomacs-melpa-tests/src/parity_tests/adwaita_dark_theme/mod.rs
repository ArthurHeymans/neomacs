use std::time::Duration;

use crate::{ADWAITA_DARK_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod eldoc;
mod faces;
mod fringe;
mod neotree;
mod registry;

const ADWAITA_DARK_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn adwaita_dark_theme_oracle(source_file: &str, prelude: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ADWAITA_DARK_THEME_MELPA_PIN, source_file)
        .expect("prepare pinned adwaita-dark-theme source below ./tmp")
        .with_prelude(prelude)
        .with_timeout(ADWAITA_DARK_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed adwaita-dark-theme parity test")
        .into()
}

fn assert_adwaita_dark_theme_source_parity(
    source_file: &str,
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = adwaita_dark_theme_oracle(source_file, prelude)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("adwaita-dark-theme parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_adwaita_dark_theme_parity(elisp_form: &str, expected: Expect) {
    assert_adwaita_dark_theme_source_parity("adwaita-dark-theme.el", "", elisp_form, expected);
}

pub(crate) fn assert_adwaita_dark_theme_with_prelude_parity(
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    assert_adwaita_dark_theme_source_parity("adwaita-dark-theme.el", prelude, elisp_form, expected);
}

pub(crate) fn assert_adwaita_dark_theme_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_adwaita_dark_theme_source_parity(
        "adwaita-dark-theme-autoloads.el",
        "",
        elisp_form,
        expected,
    );
}
