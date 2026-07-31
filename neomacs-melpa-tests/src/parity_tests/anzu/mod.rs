use std::time::Duration;

use crate::{ANZU_MELPA_PIN, CachedMelpaOracle};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod practical;

const ANZU_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn anzu_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(ANZU_MELPA_PIN, "anzu.el")
        .expect("prepare pinned anzu source below ./tmp")
        .with_timeout(ANZU_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed anzu parity test").into()
}

pub(crate) fn assert_anzu_parity(elisp_form: &str, expected: Expect) {
    let name = current_test_name();
    let report = anzu_oracle()
        .run_value(&name, elisp_form)
        .unwrap_or_else(|error| panic!("anzu parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_anzu_parity` cases (2a).
pub(crate) fn assert_anzu_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        anzu_oracle(),
        &name,
        "anzu_parity",
        cases,
    );
}
