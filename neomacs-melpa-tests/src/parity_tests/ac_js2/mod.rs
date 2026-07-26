use std::time::Duration;

use crate::{AC_JS2_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod ast;
mod candidates;
mod completion;
mod mode;
mod navigation;
mod skewer;
mod surface;

const AC_JS2_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_js2_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_JS2_MELPA_PIN, "ac-js2.el")
        .expect("prepare pinned ac-js2 source below ./tmp")
        .with_timeout(AC_JS2_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ac-js2 parity test").into()
}

pub(crate) fn assert_ac_js2_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_js2_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-js2 parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_js2_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_js2_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-js2 signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
