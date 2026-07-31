use std::time::Duration;

use crate::{AGE_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod workflows;

const AGE_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn age_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AGE_MELPA_PIN, "age.el")
        .expect("prepare pinned age source below ./tmp")
        .with_timeout(AGE_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed age parity test").into()
}

pub(crate) fn assert_age_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = age_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("age parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_age_parity` cases (2a).
pub(crate) fn assert_age_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        age_oracle(),
        &name,
        "age_parity",
        cases,
    );
}
