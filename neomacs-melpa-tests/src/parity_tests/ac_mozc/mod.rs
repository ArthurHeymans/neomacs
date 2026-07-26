use std::time::Duration;

use crate::{AC_MOZC_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod actions;
mod prefix;
mod session;
mod surface;
mod words;

const AC_MOZC_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_mozc_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_MOZC_MELPA_PIN, "ac-mozc.el")
        .expect("prepare pinned ac-mozc source below ./tmp")
        .with_timeout(AC_MOZC_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-mozc parity test")
        .into()
}

pub(crate) fn assert_ac_mozc_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_mozc_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-mozc parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_mozc_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_mozc_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-mozc signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
