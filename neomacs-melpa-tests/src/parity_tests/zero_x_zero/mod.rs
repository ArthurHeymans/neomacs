use std::time::Duration;

use crate::{CachedMelpaOracle, ZERO_X_ZERO_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod commands;
mod configuration;
mod transport;

const ZERO_X_ZERO_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn zero_x_zero_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ZERO_X_ZERO_MELPA_PIN, "0x0.el")
        .expect("prepare pinned 0x0 source below ./tmp")
        .with_timeout(ZERO_X_ZERO_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed 0x0 parity test").into()
}

pub(crate) fn assert_zero_x_zero_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = zero_x_zero_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("0x0 parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_zero_x_zero_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = zero_x_zero_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("0x0 signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_zero_x_zero_parity` cases (2a).
pub(crate) fn assert_zero_x_zero_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        zero_x_zero_oracle(),
        &name,
        "zero_x_zero_parity",
        cases,
    );
}
