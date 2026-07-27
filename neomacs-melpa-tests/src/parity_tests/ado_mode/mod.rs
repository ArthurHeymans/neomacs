use std::time::Duration;

use crate::{ADO_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod clipboard;
mod editing;
mod font_lock;
mod mode;
mod registry;
mod stata;

const ADO_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn ado_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ADO_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned ado-mode source below ./tmp")
        .with_timeout(ADO_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ado-mode parity test")
        .into()
}

fn assert_ado_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ado_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ado-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ado_mode_parity(elisp_form: &str, expected: Expect) {
    assert_ado_mode_source_parity("ado-mode.el", elisp_form, expected);
}

pub(crate) fn assert_ado_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ado_mode_source_parity("ado-mode-autoloads.el", elisp_form, expected);
}
