use std::time::Duration;

use crate::{AC_CAPF_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod candidates;
mod response;
mod setup;

const AC_CAPF_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_capf_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_CAPF_MELPA_PIN, "ac-capf.el")
        .expect("prepare pinned ac-capf source below ./tmp")
        .with_timeout(AC_CAPF_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-capf parity test")
        .into()
}

pub(crate) fn assert_ac_capf_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_capf_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-capf parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_capf_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_capf_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-capf signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
