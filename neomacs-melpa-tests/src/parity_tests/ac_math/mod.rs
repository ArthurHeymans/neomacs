use std::time::Duration;

use crate::{AC_MATH_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

mod actions;
mod candidates;
mod context;
mod surface;

const AC_MATH_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ac_math_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AC_MATH_MELPA_PIN, "ac-math.el")
        .expect("prepare pinned ac-math source below ./tmp")
        .with_timeout(AC_MATH_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed ac-math parity test")
        .into()
}

pub(crate) fn assert_ac_math_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_math_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("ac-math parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ac_math_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ac_math_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("ac-math signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}
