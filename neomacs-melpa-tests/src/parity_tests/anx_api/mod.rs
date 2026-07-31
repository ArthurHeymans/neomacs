use std::time::Duration;

use crate::{ANX_API_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod practical;

const ANX_API_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn anx_api_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANX_API_MELPA_PIN, "anx-api.el")
        .expect("prepare pinned anx-api source below ./tmp")
        .with_timeout(ANX_API_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed anx-api parity test")
        .into()
}

pub(crate) fn assert_anx_api_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anx_api_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anx-api parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_anx_api_parity` cases (2a).
pub(crate) fn assert_anx_api_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(anx_api_oracle(), &name, "anx_api_parity", cases);
}
