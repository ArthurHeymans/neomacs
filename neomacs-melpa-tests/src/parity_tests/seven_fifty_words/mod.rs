use std::time::Duration;

use crate::{CachedMelpaOracle, SEVEN_FIFTY_WORDS_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod authentication;
mod posting;

const SEVEN_FIFTY_WORDS_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn seven_fifty_words_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(SEVEN_FIFTY_WORDS_MELPA_PIN, "750words.el")
        .expect("prepare pinned 750words source below ./tmp")
        .with_timeout(SEVEN_FIFTY_WORDS_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed 750words parity test")
        .into()
}

pub(crate) fn assert_seven_fifty_words_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = seven_fifty_words_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("750words parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_seven_fifty_words_parity` cases (2a).
pub(crate) fn assert_seven_fifty_words_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        seven_fifty_words_oracle(),
        &name,
        "seven_fifty_words_parity",
        cases,
    );
}
