use std::time::Duration;

use crate::{AFTERNOON_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod faces;
mod lifecycle;
mod registry;
mod variables;

const AFTERNOON_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn afternoon_theme_oracle(source_file: &str, prelude: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AFTERNOON_THEME_MELPA_PIN, source_file)
        .expect("prepare pinned afternoon-theme source below ./tmp")
        .with_prelude(prelude)
        .with_timeout(AFTERNOON_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed afternoon-theme parity test")
        .into()
}

fn assert_afternoon_theme_source_parity(
    source_file: &str,
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = afternoon_theme_oracle(source_file, prelude)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("afternoon-theme parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_afternoon_theme_parity(elisp_form: &str, expected: Expect) {
    assert_afternoon_theme_source_parity("afternoon-theme.el", "", elisp_form, expected);
}

pub(crate) fn assert_afternoon_theme_with_prelude_parity(
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    assert_afternoon_theme_source_parity("afternoon-theme.el", prelude, elisp_form, expected);
}

pub(crate) fn assert_afternoon_theme_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_afternoon_theme_source_parity("afternoon-theme-autoloads.el", "", elisp_form, expected);
}
