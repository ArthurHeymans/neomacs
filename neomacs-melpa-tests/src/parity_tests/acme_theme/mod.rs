use std::time::Duration;

use crate::{ACME_THEME_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod faces;
mod lifecycle;
mod registry;

const ACME_THEME_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn acme_theme_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACME_THEME_MELPA_PIN, source_file)
        .expect("prepare pinned acme-theme source below ./tmp")
        .with_timeout(ACME_THEME_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed acme-theme parity test")
        .into()
}

pub(crate) fn assert_acme_theme_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = acme_theme_oracle("acme-theme.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("acme-theme parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_acme_theme_with_prelude_parity(
    prelude: &str,
    elisp_form: &str,
    expected: Expect,
) {
    let name = current_test_name();
    let report = acme_theme_oracle("acme-theme.el")
        .with_prelude(prelude)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("acme-theme pre-load parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_acme_theme_autoload_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = acme_theme_oracle("acme-theme-autoloads.el")
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("acme-theme autoload parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}
