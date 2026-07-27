use std::time::Duration;

use crate::{ACTIONSCRIPT_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod indentation;
mod mode;
mod navigation;
mod regex;
mod registry;

const ACTIONSCRIPT_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn actionscript_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ACTIONSCRIPT_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned actionscript-mode source below ./tmp")
        .with_timeout(ACTIONSCRIPT_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed actionscript-mode parity test")
        .into()
}

fn assert_actionscript_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = actionscript_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("actionscript-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_actionscript_mode_parity(elisp_form: &str, expected: Expect) {
    assert_actionscript_mode_source_parity("actionscript-mode.el", elisp_form, expected);
}

pub(crate) fn assert_actionscript_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_actionscript_mode_source_parity("actionscript-mode-autoloads.el", elisp_form, expected);
}
