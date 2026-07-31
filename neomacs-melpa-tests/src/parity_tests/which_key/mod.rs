use std::time::Duration;

use crate::{CachedMelpaOracle, WHICH_KEY_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch_cases;

/// Case constructors in child modules use this via `super::ParityBatchCase`.
pub(crate) use super::batch_support::ParityBatchCase;

mod defaults;
mod keymaps;
mod layout;
mod replacements;
mod sorting;

const WHICH_KEY_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn which_key_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(WHICH_KEY_MELPA_PIN, "which-key.el")
        .expect("prepare pinned Which-Key source and dependencies below ./tmp")
        .with_timeout(WHICH_KEY_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed Which-Key parity test")
        .into()
}

pub(crate) fn assert_which_key_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = which_key_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("Which-Key parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_which_key_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = which_key_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("Which-Key signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_which_key_parity` cases (2a).
pub(crate) fn assert_which_key_batch(cases: &[ParityBatchCase]) {
    let name = current_test_name();
    assert_oracle_batch_cases(
        which_key_oracle(),
        &name,
        "which_key_parity",
        cases,
    );
}
