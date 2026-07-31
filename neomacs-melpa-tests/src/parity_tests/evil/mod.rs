use std::time::Duration;

use crate::{CachedMelpaOracle, EVIL_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod core;
mod editing;
mod ex_search;
mod keymaps;
mod registers;
mod repeat_commands;
mod types;
mod utilities;

const EVIL_TEST_TIMEOUT: Duration = Duration::from_secs(180);

fn evil_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(EVIL_MELPA_PIN, "evil.el")
        .expect("prepare pinned Evil source and dependencies below ./tmp")
        .with_timeout(EVIL_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread.name().unwrap_or("unnamed Evil parity test").into()
}

pub(crate) fn assert_evil_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = evil_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("Evil parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_evil_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = evil_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("Evil signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_evil_parity` cases (2a).
pub(crate) fn assert_evil_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        evil_oracle(),
        &name,
        "evil_parity",
        cases,
    );
}
