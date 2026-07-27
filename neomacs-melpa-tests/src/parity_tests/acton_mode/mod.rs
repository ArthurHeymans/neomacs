use std::time::Duration;

use crate::{ACTON_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod colon;
mod indentation;
mod registry;
mod syntax_and_font_lock;

const ACTON_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn acton_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACTON_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned acton-mode source below ./tmp")
        .with_timeout(ACTON_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed acton-mode parity test")
        .into()
}

fn assert_acton_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = acton_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("acton-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_acton_mode_parity(elisp_form: &str, expected: Expect) {
    assert_acton_mode_source_parity("acton-mode.el", elisp_form, expected);
}

pub(crate) fn assert_acton_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_acton_mode_source_parity("acton-mode-autoloads.el", elisp_form, expected);
}
