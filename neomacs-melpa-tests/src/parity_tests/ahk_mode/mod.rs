use std::time::Duration;

use crate::{AHK_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod commands;
mod completion;
mod font_lock;
mod indentation;
mod registry;

const AHK_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ahk_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AHK_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned ahk-mode source below ./tmp")
        .with_timeout(AHK_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ahk-mode parity test")
        .into()
}

fn assert_ahk_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ahk_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ahk-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ahk_mode_parity(elisp_form: &str, expected: Expect) {
    assert_ahk_mode_source_parity("ahk-mode.el", elisp_form, expected);
}

pub(crate) fn assert_ahk_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ahk_mode_source_parity("ahk-mode-autoloads.el", elisp_form, expected);
}
