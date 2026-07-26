use std::time::Duration;

use crate::{AC_ETAGS_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod cache;
mod integration;
mod setup;
mod surface;

const AC_ETAGS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_etags_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_ETAGS_MELPA_PIN, "ac-etags.el")
        .expect("prepare pinned ac-etags source below ./tmp")
        .with_timeout(AC_ETAGS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-etags parity test")
        .into()
}

pub(crate) fn assert_ac_etags_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_etags_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-etags parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_etags_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_etags_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-etags signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
