use std::time::Duration;

use crate::{CachedMelpaOracle, ZERO_X_C_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod commands;
mod conversion;
mod inference;
mod live;

const ZERO_X_C_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn zero_x_c_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ZERO_X_C_MELPA_PIN, "0xc.el")
        .expect("prepare pinned 0xc source below ./tmp")
        .with_timeout(ZERO_X_C_TEST_TIMEOUT)
}

fn zero_x_c_live_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ZERO_X_C_MELPA_PIN, "0xc-live.el")
        .expect("prepare pinned 0xc-live source below ./tmp")
        .with_timeout(ZERO_X_C_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed 0xc parity test").into()
}

pub(crate) fn assert_zero_x_c_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = zero_x_c_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("0xc parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_zero_x_c_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = zero_x_c_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("0xc signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_zero_x_c_live_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = zero_x_c_live_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("0xc-live parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_zero_x_c_live_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = zero_x_c_live_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("0xc-live signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}





/// Multi-probe batch for `assert_zero_x_c_parity` cases (2a).
pub(crate) fn assert_zero_x_c_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        zero_x_c_oracle(),
        &name,
        "zero_x_c_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_zero_x_c_live_parity` cases (2a).
pub(crate) fn assert_zero_x_c_live_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        zero_x_c_live_oracle(),
        &name,
        "zero_x_c_live_parity",
        cases,
    );
}
