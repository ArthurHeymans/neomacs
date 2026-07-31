use std::time::Duration;

use crate::{CachedMelpaOracle, ZERO_B_LAYOUT_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod keybindings;
mod layouts;
mod state;

const ZERO_B_LAYOUT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn zero_b_layout_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ZERO_B_LAYOUT_MELPA_PIN, "0blayout.el")
        .expect("prepare pinned 0blayout source below ./tmp")
        .with_timeout(ZERO_B_LAYOUT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed 0blayout parity test")
        .into()
}

pub(crate) fn assert_zero_b_layout_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = zero_b_layout_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("0blayout parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_zero_b_layout_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = zero_b_layout_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("0blayout signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_zero_b_layout_parity` cases (2a).
pub(crate) fn assert_zero_b_layout_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        zero_b_layout_oracle(),
        &name,
        "zero_b_layout_parity",
        cases,
    );
}
