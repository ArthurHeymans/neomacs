use std::time::Duration;

use crate::{CachedMelpaOracle, GENERAL_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod bindings;
mod configuration;
mod definers;
mod dispatch;
mod integrations;

const GENERAL_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn general_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(GENERAL_MELPA_PIN, "general.el")
        .expect("prepare pinned General source and dependencies below ./tmp")
        .with_timeout(GENERAL_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed General parity test")
        .into()
}

pub(crate) fn assert_general_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = general_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("General parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_general_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = general_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("General signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_general_parity` cases (2a).
pub(crate) fn assert_general_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        general_oracle(),
        &name,
        "general_parity",
        cases,
    );
}
