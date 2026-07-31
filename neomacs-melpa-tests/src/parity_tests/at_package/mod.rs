use std::time::Duration;

use crate::{AT_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod core;
mod mixins;
mod reflection;

const AT_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn at_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(AT_MELPA_PIN, "@-mixins.el")
        .expect("prepare pinned @ and @-mixins sources below ./tmp")
        .with_timeout(AT_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed @ parity test").into()
}

pub(crate) fn assert_at_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = at_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("@ parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_at_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = at_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("@ signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_at_parity` cases (2a).
pub(crate) fn assert_at_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        at_oracle(),
        &name,
        "at_parity",
        cases,
    );
}
