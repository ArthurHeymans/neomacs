//! Shared multi-probe batch assertion helper for package parity suites.

use crate::{CachedMelpaOracle, OracleBatchCase};
use expect_test::Expect;

/// One named probe: Elisp form + whether it must succeed + expect-test snapshot.
///
/// Prefer building these from small case constructors (`fn opens_…() -> Self`)
/// so a package batch test stays a short list of names rather than a wall of
/// inline raw strings.
pub(crate) struct ParityBatchCase {
    pub id: &'static str,
    pub probe: &'static str,
    pub expect_value: bool,
    pub expected: Expect,
}

impl ParityBatchCase {
    pub(crate) fn new(
        id: &'static str,
        probe: &'static str,
        expect_value: bool,
        expected: Expect,
    ) -> Self {
        Self {
            id,
            probe,
            expect_value,
            expected,
        }
    }
}

/// Run many named probes in one dual-editor process pair and check expect-test
/// snapshots. `expect_value` is true for OK outcomes and false for ERR signals.
pub(crate) fn assert_oracle_batch(
    oracle: CachedMelpaOracle,
    batch_name: &str,
    package_label: &str,
    cases: &[(&str, &str, bool, Expect)],
) {
    let batch: Vec<OracleBatchCase<'_>> = cases
        .iter()
        .map(|(id, probe, expect_value, _)| OracleBatchCase {
            id,
            probe,
            expect_value: *expect_value,
        })
        .collect();
    let reports = oracle
        .run_batch(batch_name, &batch)
        .unwrap_or_else(|error| {
            panic!("{package_label} batch `{batch_name}` failed:\n{error}");
        });
    assert_eq!(
        reports.len(),
        cases.len(),
        "{package_label} batch `{batch_name}` returned {} reports for {} cases",
        reports.len(),
        cases.len()
    );
    for (report, (id, _, _, expected)) in reports.iter().zip(cases.iter()) {
        assert_eq!(report.id, *id, "{package_label} batch case order mismatch");
        expected.assert_eq(&report.gnu_emacs.to_string());
    }
}

/// Same as [`assert_oracle_batch`], but takes structured [`ParityBatchCase`] values
/// (from per-case constructor functions).
pub(crate) fn assert_oracle_batch_cases(
    oracle: CachedMelpaOracle,
    batch_name: &str,
    package_label: &str,
    cases: &[ParityBatchCase],
) {
    let batch: Vec<OracleBatchCase<'_>> = cases
        .iter()
        .map(|case| OracleBatchCase {
            id: case.id,
            probe: case.probe,
            expect_value: case.expect_value,
        })
        .collect();
    let reports = oracle
        .run_batch(batch_name, &batch)
        .unwrap_or_else(|error| {
            panic!("{package_label} batch `{batch_name}` failed:\n{error}");
        });
    assert_eq!(
        reports.len(),
        cases.len(),
        "{package_label} batch `{batch_name}` returned {} reports for {} cases",
        reports.len(),
        cases.len()
    );
    for (report, case) in reports.iter().zip(cases.iter()) {
        assert_eq!(
            report.id, case.id,
            "{package_label} batch case order mismatch"
        );
        case.expected.assert_eq(&report.gnu_emacs.to_string());
    }
}
