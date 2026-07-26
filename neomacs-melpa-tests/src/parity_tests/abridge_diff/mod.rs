use std::time::Duration;

use crate::{ABRIDGE_DIFF_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod integration;
mod ranges;
mod visibility;

const ABRIDGE_DIFF_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn abridge_diff_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ABRIDGE_DIFF_MELPA_PIN, "abridge-diff.el")
        .expect("prepare pinned abridge-diff source below ./tmp")
        .with_timeout(ABRIDGE_DIFF_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed abridge-diff parity test")
        .into()
}

pub(crate) fn assert_abridge_diff_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = abridge_diff_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("abridge-diff parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_abridge_diff_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = abridge_diff_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| {
            panic!("abridge-diff signal parity case `{name}` failed:\n{error}")
        });
    expected.assert_eq(&report.gnu_emacs.to_string());
}
