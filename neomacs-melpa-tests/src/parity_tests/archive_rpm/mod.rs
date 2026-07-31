use std::time::Duration;

use crate::{ARCHIVE_RPM_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod workflows;

const ARCHIVE_RPM_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn archive_rpm_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ARCHIVE_RPM_MELPA_PIN, "archive-rpm.el")
        .expect("prepare pinned archive-rpm source below ./tmp")
        .with_timeout(ARCHIVE_RPM_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed archive-rpm parity test")
        .into()
}

pub(crate) fn assert_archive_rpm_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = archive_rpm_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("archive-rpm parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_archive_rpm_parity` cases (2a).
pub(crate) fn assert_archive_rpm_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        archive_rpm_oracle(),
        &name,
        "archive_rpm_parity",
        cases,
    );
}
