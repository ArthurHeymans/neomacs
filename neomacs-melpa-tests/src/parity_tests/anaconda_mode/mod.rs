use std::time::Duration;

use crate::{ANACONDA_MODE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod lifecycle;
mod protocol;
mod registry;
mod server;
mod ui;

const ANACONDA_MODE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn anaconda_mode_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANACONDA_MODE_MELPA_PIN, source_file)
        .expect("prepare pinned anaconda-mode source below ./tmp")
        .with_timeout(ANACONDA_MODE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anaconda-mode parity test")
        .into()
}

fn assert_anaconda_mode_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anaconda_mode_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anaconda-mode parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anaconda_mode_parity(elisp_form: &str, expected: Expect) {
    assert_anaconda_mode_source_parity("anaconda-mode.el", elisp_form, expected);
}

pub(crate) fn assert_anaconda_mode_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anaconda_mode_oracle("anaconda-mode.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("anaconda-mode signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_anaconda_mode_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_anaconda_mode_source_parity("anaconda-mode-autoloads.el", elisp_form, expected);
}
