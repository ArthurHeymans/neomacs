use std::time::Duration;

use crate::{ANSI_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod colors;
mod csi;
mod dsl;
mod helpers;
mod inhibit;
mod registry;
mod styles;

const ANSI_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ansi_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANSI_MELPA_PIN, source_file)
        .expect("prepare pinned ansi source below ./tmp")
        .with_timeout(ANSI_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ansi parity test").into()
}

fn assert_ansi_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ansi_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ansi parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ansi_parity(elisp_form: &str, expected: Expect) {
    assert_ansi_source_parity("ansi.el", elisp_form, expected);
}

pub(crate) fn assert_ansi_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ansi_oracle("ansi.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ansi signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ansi_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ansi_source_parity("ansi-autoloads.el", elisp_form, expected);
}
