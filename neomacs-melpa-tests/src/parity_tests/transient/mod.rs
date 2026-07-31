use std::time::Duration;

use crate::{CachedMelpaOracle, TRANSIENT_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod layout;
mod state;

const TRANSIENT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn transient_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(TRANSIENT_MELPA_PIN, "transient.el")
        .expect("prepare pinned Transient source and dependencies below ./tmp")
        .with_prelude("(setq transient-error-on-insert-failure t)")
        .with_timeout(TRANSIENT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed Transient parity test")
        .into()
}

pub(crate) fn assert_transient_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = transient_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("Transient parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_transient_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = transient_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("Transient signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_transient_parity` cases (2a).
pub(crate) fn assert_transient_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(transient_oracle(), &name, "transient_parity", cases);
}
