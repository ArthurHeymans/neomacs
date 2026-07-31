use std::time::Duration;

use crate::{A_MELPA_PIN, CachedMelpaOracle, OracleBatchCase};
use expect_test::Expect;

mod workflows;

const A_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn a_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(A_MELPA_PIN, "a.el")
        .expect("prepare pinned a source below ./tmp")
        .with_timeout(A_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed a parity test").into()
}

/// Single-probe helper retained for ad-hoc cases that should not share a process.
#[allow(dead_code)]
pub(crate) fn assert_a_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = a_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("a parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Single-probe signal helper retained for ad-hoc cases outside a batch.
#[allow(dead_code)]
pub(crate) fn assert_a_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = a_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("a signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch: one process pair, many named cases (2a).
pub(crate) fn assert_a_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    let batch: Vec<OracleBatchCase<'_>> = cases
        .iter()
        .map(|(id, probe, expect_value, _)| OracleBatchCase {
            id,
            probe,
            expect_value: *expect_value,
        })
        .collect();
    let reports = a_oracle()
        .run_batch(&name, &batch)
        .unwrap_or_else(|error| panic!("a batch `{name}` failed:\n{error}"));
    assert_eq!(
        reports.len(),
        cases.len(),
        "a batch `{name}` returned {} reports for {} cases",
        reports.len(),
        cases.len()
    );
    for (report, (id, _, _, expected)) in reports.iter().zip(cases.iter()) {
        assert_eq!(report.id, *id);
        expected.assert_eq(&report.gnu_emacs.to_string());
    }
}
