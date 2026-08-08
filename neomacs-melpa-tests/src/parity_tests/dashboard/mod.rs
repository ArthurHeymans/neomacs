use std::time::Duration;

use crate::{CachedMelpaOracle, DASHBOARD_MELPA_PIN};

use super::batch_support::{ParityBatchCase, assert_oracle_batch_cases};

mod workflows;

const DASHBOARD_TEST_TIMEOUT: Duration = Duration::from_secs(180);
const DASHBOARD_TEST_PRELUDE: &str = r####"
(require 'cl-lib)
(require 'dashboard)
"####;

fn dashboard_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(DASHBOARD_MELPA_PIN, "dashboard.el")
        .expect("prepare exact shallow dashboard source below ./tmp")
        .with_prelude(DASHBOARD_TEST_PRELUDE)
        .with_timeout(DASHBOARD_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    std::thread::current()
        .name()
        .unwrap_or("unnamed dashboard parity test")
        .into()
}

fn assert_dashboard_batch(cases: &[ParityBatchCase]) {
    assert_oracle_batch_cases(
        dashboard_oracle(),
        &current_test_name(),
        "dashboard_parity",
        cases,
    );
}

#[test]
fn dashboard_package_batch() {
    assert_dashboard_batch(&workflows::workflow_batch_cases());
}
