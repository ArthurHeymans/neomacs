use std::time::Duration;

use crate::{ASYNC_GNU_ELPA_PIN, CachedPackageOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod futures;
mod processes;
mod serialization;

const ASYNC_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn async_oracle() -> CachedPackageOracle {
    CachedPackageOracle::new_from_gnu_elpa(ASYNC_GNU_ELPA_PIN, "async.el")
        .expect("prepare pinned Async source and dependencies below ./tmp")
        .with_timeout(ASYNC_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed Async parity test").into()
}

pub(crate) fn assert_async_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = async_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("Async parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_async_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = async_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("Async signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_async_parity` cases (2a).
pub(crate) fn assert_async_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(async_oracle(), &name, "async_parity", cases);
}
