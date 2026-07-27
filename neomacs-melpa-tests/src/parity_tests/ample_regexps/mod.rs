use std::time::Duration;

use crate::{AMPLE_REGEXPS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod definitions;
mod helpers;
mod lifecycle;
mod matching;
mod registry;

const AMPLE_REGEXPS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ample_regexps_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AMPLE_REGEXPS_MELPA_PIN, source_file)
        .expect("prepare pinned ample-regexps source below ./tmp")
        .with_timeout(AMPLE_REGEXPS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ample-regexps parity test")
        .into()
}

fn assert_ample_regexps_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ample_regexps_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ample-regexps parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ample_regexps_parity(elisp_form: &str, expected: Expect) {
    assert_ample_regexps_source_parity("ample-regexps.el", elisp_form, expected);
}

pub(crate) fn assert_ample_regexps_signal_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ample_regexps_oracle("ample-regexps.el")
        .run_signal(&name, elisp_form)
        .unwrap_or_else(|error| {
            panic!("ample-regexps signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ample_regexps_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ample_regexps_source_parity("ample-regexps-autoloads.el", elisp_form, expected);
}
