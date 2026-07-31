use std::time::Duration;

use crate::{AH_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod cursor;
mod lifecycle;
mod quit;
mod theme;

const AH_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn ah_oracle(source_file: &str) -> CachedMelpaOracle {
    CachedMelpaOracle::new(AH_MELPA_PIN, source_file)
        .expect("prepare pinned ah source below ./tmp")
        .with_timeout(AH_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed ah parity test").into()
}

fn assert_ah_source_parity(source_file: &str, elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = ah_oracle(source_file)
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("ah parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_ah_parity(elisp_form: &str, expected: Expect) {
    assert_ah_source_parity("ah.el", elisp_form, expected);
}

pub(crate) fn assert_ah_autoload_parity(elisp_form: &str, expected: Expect) {
    assert_ah_source_parity("ah-autoloads.el", elisp_form, expected);
}

/// Multi-probe batch for `assert_ah_autoload_parity` cases (2a).
pub(crate) fn assert_ah_autoload_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        ah_oracle("ah-autoloads.el"),
        &name,
        "ah_autoload_parity",
        cases,
    );
}

/// Multi-probe batch for `assert_ah_parity` cases (2a).
pub(crate) fn assert_ah_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(ah_oracle("ah.el"), &name, "ah_parity", cases);
}
