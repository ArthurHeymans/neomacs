use std::time::Duration;

use crate::{CachedMelpaOracle, WITH_EDITOR_MELPA_PIN};
use expect_test::Expect;

use super::batch_support::assert_oracle_batch;

mod environment;
mod lifecycle;
mod protocol;

const WITH_EDITOR_TEST_TIMEOUT: Duration = Duration::from_secs(120);

fn with_editor_oracle() -> CachedMelpaOracle {
    CachedMelpaOracle::new(WITH_EDITOR_MELPA_PIN, "with-editor.el")
        .expect("prepare pinned With-Editor source and dependencies below ./tmp")
        .with_timeout(WITH_EDITOR_TEST_TIMEOUT)
}

fn current_test_name() -> String {
    let thread = std::thread::current();
    thread
        .name()
        .unwrap_or("unnamed With-Editor parity test")
        .into()
}

pub(crate) fn assert_with_editor_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = with_editor_oracle()
        .run_value(&name, form)
        .unwrap_or_else(|error| panic!("With-Editor parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

pub(crate) fn assert_with_editor_signal_parity(form: &str, expected: Expect) {
    let name = current_test_name();
    let report = with_editor_oracle()
        .run_signal(&name, form)
        .unwrap_or_else(|error| panic!("With-Editor signal parity case `{name}` failed:\n{error}"));
    expected.assert_eq(&report.gnu_emacs.to_string());
}

/// Multi-probe batch for `assert_with_editor_parity` cases (2a).
pub(crate) fn assert_with_editor_batch(cases: &[(&str, &str, bool, Expect)]) {
    let name = current_test_name();
    assert_oracle_batch(
        with_editor_oracle(),
        &name,
        "with_editor_parity",
        cases,
    );
}
