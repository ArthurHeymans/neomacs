use std::time::Duration;

use crate::{CachedPackageOracle, USE_PACKAGE_GNU_ELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod activation;
mod core;
mod integrations;

const USE_PACKAGE_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn use_package_oracle() -> CachedPackageOracle {
    CachedPackageOracle::new_from_gnu_elpa(USE_PACKAGE_GNU_ELPA_PIN, "use-package.el")
        .expect("prepare pinned Use-Package source and dependencies below ./tmp")
        .with_timeout(USE_PACKAGE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed Use-Package parity test")
        .into()
}

pub(crate) fn assert_use_package_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = use_package_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("Use-Package parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_use_package_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = use_package_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("Use-Package signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_use_package_parity` cases (2a).
pub(crate) fn assert_use_package_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        use_package_oracle(),
        &name,
        "use_package_parity",
        cases,
    );
}
